//! How often sprites rest, and how long they take to recover, as they age
//! (#130): runs the default world from seeds 1–10 and, for each stretch of
//! a sprite's life, reports the share of its ticks spent resting, how many
//! rests it starts, how long it takes from a rest starting until its
//! tiredness is back under 0.1, and how far it walks. It only measures:
//! the numbers feed tiredness tuning (#125) and ageing (#49).
//!
//! `cargo run --release -p terra-sim --example rest`
//!
//! `--ticks N` runs each world N ticks (80,000 by default), `--seeds A-B`
//! the seeds A to B (1-10 by default), and `--sprites` adds a line for
//! each sprite.
//!
//! A sprite's life is cut into quarters of its own lifespan, and a stretch
//! past it, when old age injures it (design §4.8, §4.10). A rest counts toward the
//! stretch it starts in, and so does its recovery.

use std::collections::BTreeMap;
use std::process::ExitCode;

use terra_sim::{DataPack, EntityId, EventKind, Verb, World, WorldConfig, median};

/// Tiredness under this counts as recovered.
const RECOVERED: f32 = 0.1;

/// The stretches of a life, by the share of its lifespan lived.
const STRETCHES: [&str; 5] = ["0–25%", "25–50%", "50–75%", "75–100%", "past it"];

/// What was counted in one stretch of life, for one sprite or for many.
/// A rest still going when the run ends, or the sprite dies, has no
/// recovery.
#[derive(Debug, Default, Clone)]
struct Tally {
    /// Ticks lived in it.
    ticks: u64,
    /// Ticks spent resting.
    resting: u64,
    /// Rests started.
    rests: u64,
    /// Ticks from each rest starting until tiredness was back under 0.1.
    recoveries: Vec<u64>,
    /// Tiles moved, as Chebyshev distance tick to tick.
    tiles: u64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.ticks += other.ticks;
        self.resting += other.resting;
        self.rests += other.rests;
        self.recoveries.extend(&other.recoveries);
        self.tiles += other.tiles;
    }

    /// The tally as a table row.
    fn row(&self, label: &str) -> String {
        let per = |n: u64| {
            if self.ticks == 0 {
                0.0
            } else {
                n as f64 / self.ticks as f64
            }
        };
        let recovery = if self.recoveries.is_empty() {
            "-".to_string()
        } else {
            format!("{:.0}", median(&self.recoveries))
        };
        format!(
            "{label:<22} {:>9} {:>8.1}% {:>8.2} {:>10} {:>10.0}",
            self.ticks,
            100.0 * per(self.resting),
            1000.0 * per(self.rests),
            recovery,
            1000.0 * per(self.tiles),
        )
    }
}

/// One sprite as followed through a run.
#[derive(Debug, Default)]
struct Life {
    /// Its tally for each stretch of life.
    stretches: [Tally; 5],
    /// Where it stood after the last tick.
    last: Option<terra_sim::Pos>,
    /// A rest started on this tick, in this stretch, with tiredness not yet
    /// back under 0.1.
    recovering: Option<(u64, usize)>,
}

/// The stretch of life a sprite of `age` with `lifespan` is in.
fn stretch(age: u64, lifespan: f32) -> usize {
    let lived = age as f64 / f64::from(lifespan.max(1.0));
    if lived >= 1.0 {
        4
    } else {
        (lived * 4.0) as usize
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}\nusage: rest [--ticks N] [--seeds A-B] [--sprites]");
            return ExitCode::FAILURE;
        }
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release, or it will take a long time");
    }
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let (first, last) = options.seeds;
    println!(
        "Resting by age: the default world, seeds {first}–{last}, {} ticks each.",
        options.ticks
    );
    println!(
        "Rests a 1,000 ticks and tiles a 1,000 ticks count the ticks lived in the stretch; \
         recovery is the median ticks from a rest starting until tiredness is under {RECOVERED}."
    );
    let mut total: [Tally; 5] = Default::default();
    let mut lines = Vec::new();
    for seed in first..=last {
        let lives = run(&data, seed, options.ticks);
        for (id, life) in &lives {
            for (all, one) in total.iter_mut().zip(&life.stretches) {
                all.add(one);
            }
            if options.per_sprite {
                for (name, tally) in STRETCHES.iter().zip(&life.stretches) {
                    if tally.ticks > 0 {
                        lines.push(tally.row(&format!("seed {seed} #{} {name}", id.0)));
                    }
                }
            }
        }
    }
    let header = format!(
        "{:<22} {:>9} {:>9} {:>8} {:>10} {:>10}",
        "Stretch of life", "Ticks", "Resting", "Rests/1k", "Recovery", "Tiles/1k"
    );
    println!("\n{header}");
    for (name, tally) in STRETCHES.iter().zip(&total) {
        println!("{}", tally.row(name));
    }
    if options.per_sprite {
        println!("\n{header}");
        for line in lines {
            println!("{line}");
        }
    }
    ExitCode::SUCCESS
}

/// Runs the default world from `seed` for `ticks` ticks, following every
/// sprite.
fn run(data: &DataPack, seed: u64, ticks: u64) -> BTreeMap<EntityId, Life> {
    let mut world = World::new(WorldConfig::builtin(data), data.clone(), seed);
    let mut lives: BTreeMap<EntityId, Life> = BTreeMap::new();
    for _ in 0..ticks {
        let events = world.step();
        let tick = world.tick();
        for event in &events {
            if let EventKind::ActionStarted {
                id,
                verb: Verb::Rest,
            } = event.kind
                && let Some(sprite) = world.sprite(id)
            {
                let at = stretch(sprite.age(), sprite.traits().lifespan);
                let life = lives.entry(id).or_default();
                life.stretches[at].rests += 1;
                // A rest begun rested has nothing to recover from.
                if sprite.chemical("tiredness").unwrap_or(0.0) >= RECOVERED {
                    life.recovering.get_or_insert((tick, at));
                }
            }
        }
        observe(&world, &mut lives);
    }
    lives
}

/// Counts this tick for every sprite alive in `world`.
fn observe(world: &World, lives: &mut BTreeMap<EntityId, Life>) {
    let tick = world.tick();
    for sprite in world.sprites() {
        let life = lives.entry(sprite.id()).or_default();
        let tally = &mut life.stretches[stretch(sprite.age(), sprite.traits().lifespan)];
        tally.ticks += 1;
        if sprite
            .action()
            .is_some_and(|action| action.verb == Verb::Rest)
        {
            tally.resting += 1;
        }
        let pos = sprite.pos();
        if let Some(last) = life.last {
            tally.tiles += u64::from(last.x.abs_diff(pos.x).max(last.y.abs_diff(pos.y)));
        }
        life.last = Some(pos);
        let tired = sprite.chemical("tiredness").unwrap_or(0.0);
        if tired < RECOVERED
            && let Some((started, at)) = life.recovering.take()
        {
            life.stretches[at].recoveries.push(tick - started);
        }
    }
}

/// What the arguments ask for.
struct Options {
    ticks: u64,
    seeds: (u64, u64),
    per_sprite: bool,
}

/// The options, from the arguments.
fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        ticks: 80_000,
        seeds: (1, 10),
        per_sprite: false,
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--sprites" => options.per_sprite = true,
            "--ticks" => {
                let value = args.next().ok_or("--ticks needs a number")?;
                options.ticks = value
                    .parse()
                    .map_err(|_| format!("--ticks needs a number, not {value}"))?;
            }
            "--seeds" => {
                let value = args.next().ok_or("--seeds needs a range, such as 1-10")?;
                let bad = || format!("--seeds needs a range, such as 1-10, not {value}");
                let (a, b) = value.split_once('-').ok_or_else(bad)?;
                let (a, b) = (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
                if a > b {
                    return Err(bad());
                }
                options.seeds = (a, b);
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    if options.ticks == 0 {
        return Err("--ticks needs a number above 0".to_string());
    }
    Ok(options)
}
