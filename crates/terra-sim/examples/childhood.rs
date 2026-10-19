//! The harsher world's long run (M2 design §19.6): runs the default world,
//! left alone, from the given seeds for 220 hours of play each, and
//! reports every 10 hours how its sprites stand, then each seed against
//! Slice 29's bars, then lone newborns given the genes of the sprites alive
//! at the end.
//!
//! Each row: the hour; sprites, the young among them, and eggs; berry
//! bushes; and since the last row, the hatched young that grew up and that
//! died young, the grown-up deaths and how many of them were adults dying
//! of hunger or thirst, the share of bites tried by a sprite too full to
//! eat, the share of adults' time on the move, and how many of the eleven
//! needs genes (M2 design §19.6) more than half of the hatched sprites
//! alive lack.
//!
//! The bars, for each seed:
//!
//! - **The world lasts** the whole run.
//! - **At least 60% of the young hatched grow up.** Those still growing at
//!   the end aren't counted.
//! - **Adults dying of hunger or thirst are under 1 in 20 of grown-up
//!   deaths.** Old sprites' are reported apart.
//! - **At most 1 in 10 bites are tried by a sprite too full to eat,** over
//!   the run and over its last 50 hours.
//! - **The bushes hold:** over the last 50 hours they average at least half
//!   of what they did over hours 50–100, in at least 9 seeds of 10.
//!
//! Reported with no bar: the fewest sprites after hour 10; adults' time on
//! the move over the last 50 hours, which #318 asks to be a quarter; the
//! share of the hatched sprites alive at the end lacking each needs gene;
//! and lone newborns. For each seed whose world lasted, the unattended run
//! (`scenarios/unattended.ron`) on the same seed, its newborns taking after
//! the hatched sprites alive at the end, until each grows up or dies.
//! 10%–30% growing up says the species hasn't fallen behind. Careful
//! copying would switch on for a seed that fails a bar while more than half
//! its living lack a needs gene (M2 design §19.6).
//!
//! `cargo run --release -p terra-sim --example childhood`
//!
//! `--seeds` runs the seeds listed, `5,9`, or a range, `1-10` (1-10 by
//! default), `--hours N` each for N hours of play at 1× (220 by default),
//! `--every N` reports every N hours (10 by default), and `--jobs N` runs N
//! worlds at a time (4 by default). Each world is its own, so the results
//! don't depend on it. Each seed's table is printed as it finishes, and the
//! bars at the end.

#[path = "common/args.rs"]
mod args;

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::sync::Mutex;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, GeneView, LifeStage, Outcome, Pos, Switch, Verb,
    World, WorldConfig,
};

/// Ticks in an hour at 1×, 1.25 ticks a second (design §6.6).
const HOUR: u64 = 4_500;

/// The hours at the end the late bars are judged on.
const LATE_HOURS: u64 = 50;

/// The bushes' hours to compare the last 50 hours with.
const EARLY_BUSHES: (u64, u64) = (50, 100);

/// The longest the lone newborns run, in ticks: a sprite whose genes take
/// longer than this to grow up is counted as neither.
const NEWBORN_TICKS: u64 = 20_000;

/// The eleven needs genes (M2 design §19.6), by what each does, as the
/// starter genome has them: hunger's, thirst's and tiredness's.
const NEEDS: [&str; 11] = [
    "low energy Level → hunger",
    "ate Level → hunger",
    "hunger",
    "hunger → Eat",
    "low hydration Level → thirst",
    "drank Level → thirst",
    "thirst",
    "thirst → Drink",
    "low stamina Level → tiredness",
    "stamina Level → tiredness",
    "tiredness → Rest",
];

struct Options {
    seeds: Vec<u64>,
    hours: u64,
    every: u64,
    jobs: u64,
}

/// Counts over some ticks of a run: a row's, or the whole run's.
#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    /// Hatched young that grew up, and that died young.
    grew_up: u64,
    died_young: u64,
    /// Grown-up deaths, and those of adults and of old sprites by hunger
    /// or thirst.
    grown_deaths: u64,
    adult_hunger_thirst: u64,
    old_hunger_thirst: u64,
    /// Bites tried, and those by a sprite too full to eat.
    bites: u64,
    too_full: u64,
    /// Adults' ticks, and those on the move: on another tile than the tick
    /// before.
    adult_ticks: u64,
    moving: u64,
}

impl Counts {
    fn add(&mut self, other: &Counts) {
        self.grew_up += other.grew_up;
        self.died_young += other.died_young;
        self.grown_deaths += other.grown_deaths;
        self.adult_hunger_thirst += other.adult_hunger_thirst;
        self.old_hunger_thirst += other.old_hunger_thirst;
        self.bites += other.bites;
        self.too_full += other.too_full;
        self.adult_ticks += other.adult_ticks;
        self.moving += other.moving;
    }
}

/// One row: the moment, and the counts since the row before.
#[derive(Debug)]
struct Row {
    hour: u64,
    sprites: usize,
    young: usize,
    eggs: usize,
    bushes: usize,
    counts: Counts,
    /// Needs genes more than half the hatched sprites alive lack.
    needs_lacked: usize,
}

/// What one seed's world came to.
#[derive(Debug)]
struct Run {
    seed: u64,
    rows: Vec<Row>,
    whole: Counts,
    late: Counts,
    /// What killed the hatched young, by cause.
    died_young_of: BTreeMap<String, u64>,
    /// The bushes at the end of each hour.
    bushes: Vec<usize>,
    /// The fewest sprites at the end of an hour after the first 10.
    fewest: Option<usize>,
    /// The tick the world died out on, if it did.
    died_out: Option<u64>,
    /// Each needs gene, and the share of the hatched sprites alive at the
    /// end that lack it; empty if none is alive.
    lacking: Vec<(&'static str, f64)>,
    /// The lone newborns: how many grew up and how many died, if the
    /// world lasted with hatched sprites alive.
    newborns: Option<(u64, u64)>,
}

fn main() -> ExitCode {
    let usage = "childhood [--seeds A-B | a,b,c] [--hours N] [--every N] [--jobs N]";
    let Some(options) = args::parse(usage, parse) else {
        return ExitCode::FAILURE;
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release, or it will take a long time");
    }
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let starter = starter_needs(&data);
    let seeds: Vec<String> = options.seeds.iter().map(u64::to_string).collect();
    println!(
        "The harsher world's long run: the default world, left alone, seeds {}, {} hours each \
         ({} ticks).",
        seeds.join(", "),
        options.hours,
        options.hours * HOUR
    );
    let runs = in_parallel(&data, &starter, &options);
    bars(&runs, &options);
    ExitCode::SUCCESS
}

/// Runs the seeds' worlds, `options.jobs` at a time, printing each one's
/// table as it finishes, and returns them in seed order.
fn in_parallel(data: &DataPack, starter: &BTreeMap<String, usize>, options: &Options) -> Vec<Run> {
    let todo = Mutex::new(options.seeds.clone().into_iter());
    let done = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..options.jobs {
            scope.spawn(|| {
                loop {
                    let next = todo.lock().expect("no thread panicked").next();
                    let Some(seed) = next else {
                        break;
                    };
                    let run = run(data, starter, seed, options);
                    let mut done = done.lock().expect("no thread panicked");
                    print_run(&run);
                    done.push(run);
                }
            });
        }
    });
    let mut done = done.into_inner().expect("no thread panicked");
    done.sort_by_key(|run| run.seed);
    done
}

/// Runs the default world from `seed` for `options.hours`, then the lone
/// newborns.
fn run(data: &DataPack, starter: &BTreeMap<String, usize>, seed: u64, options: &Options) -> Run {
    let config = WorldConfig::builtin(data);
    let mut world = World::new(config.clone(), data.clone(), seed);
    let mut stages: BTreeMap<EntityId, LifeStage> = world
        .sprites()
        .map(|sprite| (sprite.id(), sprite.life_stage()))
        .collect();
    let mut hatched: BTreeSet<EntityId> = BTreeSet::new();
    let mut last: BTreeMap<EntityId, Pos> = BTreeMap::new();
    let late_from = options.hours.saturating_sub(LATE_HOURS) * HOUR;
    let mut out = Run {
        seed,
        rows: Vec::new(),
        whole: Counts::default(),
        late: Counts::default(),
        died_young_of: BTreeMap::new(),
        bushes: Vec::new(),
        fewest: None,
        died_out: None,
        lacking: Vec::new(),
        newborns: None,
    };
    let mut row = Counts::default();
    for _ in 0..options.hours * HOUR {
        let tick = world.tick();
        let mut now = Counts::default();
        for event in world.step() {
            match event.kind {
                EventKind::Hatched { id, .. } => {
                    stages.insert(id, LifeStage::Young);
                    hatched.insert(id);
                }
                EventKind::Spawned { id, .. } => {
                    stages.insert(id, LifeStage::Young);
                }
                EventKind::GrewUp { id } => {
                    stages.insert(id, LifeStage::Adult);
                    if hatched.remove(&id) {
                        now.grew_up += 1;
                    }
                }
                EventKind::GrewOld { id } => {
                    stages.insert(id, LifeStage::Old);
                }
                EventKind::Died { id, cause, .. } => {
                    let stage = stages.remove(&id).unwrap_or_default();
                    let need = matches!(cause, DeathCause::Starvation | DeathCause::Dehydration);
                    match stage {
                        LifeStage::Young => {
                            if hatched.remove(&id) {
                                now.died_young += 1;
                                *out.died_young_of.entry(cause_name(cause)).or_insert(0) += 1;
                            }
                        }
                        LifeStage::Adult => {
                            now.grown_deaths += 1;
                            now.adult_hunger_thirst += u64::from(need);
                        }
                        LifeStage::Old => {
                            now.grown_deaths += 1;
                            now.old_hunger_thirst += u64::from(need);
                        }
                    }
                }
                EventKind::ActionEnded {
                    verb: Verb::Eat,
                    outcome,
                    ..
                } => {
                    now.bites += 1;
                    now.too_full += u64::from(outcome == Outcome::TooFull);
                }
                _ => {}
            }
        }
        let mut here = BTreeMap::new();
        for sprite in world.sprites() {
            if sprite.life_stage() == LifeStage::Adult {
                now.adult_ticks += 1;
                let moved = last.get(&sprite.id()).is_some_and(|&at| at != sprite.pos());
                now.moving += u64::from(moved);
            }
            here.insert(sprite.id(), sprite.pos());
        }
        last = here;
        row.add(&now);
        out.whole.add(&now);
        if tick >= late_from {
            out.late.add(&now);
        }
        let ticks = tick + 1;
        if ticks % HOUR == 0 {
            let hour = ticks / HOUR;
            let bushes = bush_count(&world);
            out.bushes.push(bushes);
            if hour > 10 {
                let sprites = world.sprites().count();
                out.fewest = Some(out.fewest.map_or(sprites, |fewest| fewest.min(sprites)));
            }
            if hour % options.every == 0 {
                let lacking = lacking(&world, starter);
                out.rows.push(Row {
                    hour,
                    sprites: world.sprites().count(),
                    young: world
                        .sprites()
                        .filter(|sprite| sprite.life_stage() == LifeStage::Young)
                        .count(),
                    eggs: world.eggs().count(),
                    bushes,
                    counts: std::mem::take(&mut row),
                    needs_lacked: lacking.iter().filter(|(_, share)| *share > 0.5).count(),
                });
            }
        }
        if world.sprites().count() + world.eggs().count() == 0 {
            out.died_out = Some(ticks);
            break;
        }
    }
    if out.died_out.is_none() {
        out.lacking = lacking(&world, starter);
        out.newborns = World::unattended_like(config, data.clone(), seed, &world).map(newborns);
    }
    out
}

/// Runs the lone newborns' world until each of its first sprites has
/// grown up or died, or for `NEWBORN_TICKS`: how many grew up, and how
/// many died.
fn newborns(mut world: World) -> (u64, u64) {
    let mut growing: BTreeSet<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
    let (mut grew_up, mut died) = (0, 0);
    while !growing.is_empty() && world.tick() < NEWBORN_TICKS {
        for event in world.step() {
            match event.kind {
                EventKind::GrewUp { id } if growing.remove(&id) => grew_up += 1,
                EventKind::Died { id, .. } if growing.remove(&id) => died += 1,
                _ => {}
            }
        }
    }
    (grew_up, died)
}

/// The berry bushes in `world`, seedlings too.
fn bush_count(world: &World) -> usize {
    world
        .objects()
        .filter(|object| object.type_name() == "berry_bush")
        .count()
}

/// What a death was of, by name.
fn cause_name(cause: DeathCause) -> String {
    match cause {
        DeathCause::Starvation => "hunger".into(),
        DeathCause::Dehydration => "thirst".into(),
        DeathCause::OldAge => "old age".into(),
        DeathCause::HurtBy(id) => format!("hurt by object type {id}"),
    }
}

/// How many the starter has of each needs gene: at least one, or `NEEDS`
/// has gone out of step with it.
fn starter_needs(data: &DataPack) -> BTreeMap<String, usize> {
    let genome = data.starter();
    let counts = counted(&genome.genes(data), &genome.switches());
    NEEDS
        .iter()
        .map(|&need| {
            let count = counts.get(need).copied();
            let count = count.unwrap_or_else(|| panic!("the starter genome has no {need} gene"));
            (need.to_string(), count)
        })
        .collect()
}

/// Each needs gene, and the share of the hatched sprites alive in `world`
/// with fewer copies of it than the starter: lost, or changed into
/// something else (M2 design §13). Empty if none is alive.
fn lacking(world: &World, starter: &BTreeMap<String, usize>) -> Vec<(&'static str, f64)> {
    let mut lack = [0u64; NEEDS.len()];
    let mut sprites = 0u64;
    for sprite in world
        .sprites()
        .filter(|sprite| !sprite.parents().is_empty())
    {
        sprites += 1;
        let (genes, _): (Vec<GeneView>, Vec<_>) = sprite.genes().into_iter().unzip();
        let has = counted(&genes, &sprite.switches());
        for (n, need) in NEEDS.iter().enumerate() {
            let its = has.get(*need).copied().unwrap_or(0);
            lack[n] += u64::from(its < starter[*need]);
        }
    }
    if sprites == 0 {
        return Vec::new();
    }
    NEEDS
        .iter()
        .zip(lack)
        .map(|(&need, lacked)| (need, lacked as f64 / sprites as f64))
        .collect()
}

/// How many of `genes` there are of each, by what each does, of those that
/// work for all of a sprite's life, `switches` saying when each works.
/// Genes this build can't read aren't counted.
fn counted(genes: &[GeneView], switches: &[Switch]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for (gene, switch) in genes.iter().zip(switches) {
        if switch.from != LifeStage::Young || switch.until.is_some() {
            continue;
        }
        if let Some((name, _)) = gene.named() {
            *counts.entry(name).or_insert(0) += 1;
        }
    }
    counts
}

/// `part` of `whole` as a percentage, or a dash if `whole` is none.
fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        "-".into()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}

fn print_run(run: &Run) {
    let ended = match run.died_out {
        Some(tick) => format!(", died out at hour {:.1}", tick as f64 / HOUR as f64),
        None => String::new(),
    };
    println!("\nSeed {}{ended}", run.seed);
    println!(
        "{:>5} {:>7} {:>6} {:>5} {:>7} {:>7} {:>10} {:>6} {:>8} {:>8} {:>7} {:>6}",
        "Hour",
        "Sprites",
        "Young",
        "Eggs",
        "Bushes",
        "GrewUp",
        "DiedYoung",
        "Grown",
        "AdultH/T",
        "TooFull",
        "Moving",
        "Needs"
    );
    for row in &run.rows {
        let c = row.counts;
        println!(
            "{:>5} {:>7} {:>6} {:>5} {:>7} {:>7} {:>10} {:>6} {:>8} {:>8} {:>7} {:>6}",
            row.hour,
            row.sprites,
            row.young,
            row.eggs,
            row.bushes,
            c.grew_up,
            c.died_young,
            c.grown_deaths,
            c.adult_hunger_thirst,
            percent(c.too_full, c.bites),
            percent(c.moving, c.adult_ticks),
            row.needs_lacked
        );
    }
}

/// The mean of the bushes counted at the ends of hours `from` + 1 to `to`;
/// none if the run didn't reach them.
fn mean_bushes(bushes: &[usize], from: u64, to: u64) -> Option<f64> {
    let hours = bushes.get(from as usize..to as usize)?;
    Some(hours.iter().sum::<usize>() as f64 / hours.len() as f64)
}

/// Whether the bushes held in `run`: over the last 50 hours, at least half
/// of hours 50–100. None if the run was too short to tell or died out.
fn bushes_held(run: &Run, hours: u64) -> Option<bool> {
    if run.died_out.is_some() || hours < EARLY_BUSHES.1 + LATE_HOURS {
        return None;
    }
    let early = mean_bushes(&run.bushes, EARLY_BUSHES.0, EARLY_BUSHES.1)?;
    let late = mean_bushes(&run.bushes, hours - LATE_HOURS, hours)?;
    Some(late >= early / 2.0)
}

/// The middle of `shares`, or the mean of the middle two; none if empty.
fn middle(mut shares: Vec<f64>) -> Option<f64> {
    shares.sort_by(f64::total_cmp);
    let half = shares.len() / 2;
    match shares.len() {
        0 => None,
        n if n % 2 == 1 => Some(shares[half]),
        _ => Some((shares[half - 1] + shares[half]) / 2.0),
    }
}

/// Each seed against the bars, all the seeds together, and the needs genes
/// lacked at the end.
fn bars(runs: &[Run], options: &Options) {
    println!("\nEach seed against the bars");
    println!(
        "{:>5} {:>7} {:>7} {:>15} {:>12} {:>8} {:>17} {:>11} {:>8} {:>13}",
        "Seed",
        "Lasted",
        "Fewest",
        "Young grew up",
        "Adult H/T",
        "Old H/T",
        "TooFull run/late",
        "Bushes held",
        "Moving",
        "Lone newborns"
    );
    let mut pooled = Counts::default();
    let mut late_moving = (0, 0);
    let mut shares = Vec::new();
    let mut failed: Vec<(u64, Vec<&str>)> = Vec::new();
    for run in runs {
        let (whole, late) = (run.whole, run.late);
        pooled.add(&whole);
        late_moving = (
            late_moving.0 + late.moving,
            late_moving.1 + late.adult_ticks,
        );
        let lasted = run.died_out.is_none();
        let raised = whole.grew_up + whole.died_young;
        let young_ok = 100 * whole.grew_up >= 60 * raised;
        let needs_ok = 20 * whole.adult_hunger_thirst < whole.grown_deaths.max(1);
        let full_ok = 10 * whole.too_full <= whole.bites && 10 * late.too_full <= late.bites;
        let held = bushes_held(run, options.hours);
        let newborns = match run.newborns {
            Some((grew_up, died)) => {
                if grew_up + died > 0 {
                    shares.push(grew_up as f64 / (grew_up + died) as f64);
                }
                format!("{grew_up} of {}", grew_up + died)
            }
            None => "-".into(),
        };
        println!(
            "{:>5} {:>7} {:>7} {:>15} {:>12} {:>8} {:>17} {:>11} {:>8} {:>13}",
            run.seed,
            if lasted { "yes" } else { "no" },
            run.fewest.map_or("-".into(), |fewest| fewest.to_string()),
            format!("{} of {raised}", whole.grew_up),
            format!("{} of {}", whole.adult_hunger_thirst, whole.grown_deaths),
            whole.old_hunger_thirst,
            format!(
                "{}/{}",
                percent(whole.too_full, whole.bites),
                percent(late.too_full, late.bites)
            ),
            held.map_or("-", |held| if held { "yes" } else { "no" }),
            percent(late.moving, late.adult_ticks),
            newborns
        );
        let fails: Vec<&str> = [
            (!lasted, "died out"),
            (!young_ok, "young grew up"),
            (!needs_ok, "adults' hunger and thirst"),
            (!full_ok, "too full"),
            (held == Some(false), "bushes"),
        ]
        .into_iter()
        .filter_map(|(fails, bar)| fails.then_some(bar))
        .collect();
        if !fails.is_empty() {
            failed.push((run.seed, fails));
        }
    }
    let held = runs
        .iter()
        .filter(|run| bushes_held(run, options.hours) == Some(true))
        .count();
    println!(
        "\nAll seeds: young grew up {} ({} of {}); adults dying of hunger or thirst {} of grown-up \
         deaths ({} of {}), and old sprites {}; bites tried too full {}; adults on the move over \
         the last {LATE_HOURS} hours {} (#318 asks a quarter).",
        percent(pooled.grew_up, pooled.grew_up + pooled.died_young),
        pooled.grew_up,
        pooled.grew_up + pooled.died_young,
        percent(pooled.adult_hunger_thirst, pooled.grown_deaths),
        pooled.adult_hunger_thirst,
        pooled.grown_deaths,
        pooled.old_hunger_thirst,
        percent(pooled.too_full, pooled.bites),
        percent(late_moving.0, late_moving.1),
    );
    println!(
        "Bushes held in {held} of {} seeds (bar: 9 of 10).",
        runs.len()
    );
    if let Some(middle) = middle(shares) {
        println!(
            "Lone newborns given the end's genes: {:.0}% grew up in the middle seed (10%-30% says \
             the species hasn't fallen behind).",
            100.0 * middle
        );
    }
    let mut died_young: BTreeMap<&str, u64> = BTreeMap::new();
    for run in runs {
        for (cause, n) in &run.died_young_of {
            *died_young.entry(cause).or_insert(0) += n;
        }
    }
    let causes: Vec<String> = died_young
        .iter()
        .map(|(cause, n)| format!("{cause} {n}"))
        .collect();
    println!("What killed the hatched young: {}.", causes.join(", "));
    println!("\nNeeds genes lacked by the hatched sprites alive at the end");
    print!("{:<30}", "Gene");
    for run in runs {
        print!("{:>9}", format!("seed {}", run.seed));
    }
    println!();
    for (n, need) in NEEDS.iter().enumerate() {
        print!("{need:<30}");
        for run in runs {
            let share = run.lacking.get(n).map(|&(_, share)| share);
            print!(
                "{:>9}",
                share.map_or("-".into(), |share| format!("{:.0}%", 100.0 * share))
            );
        }
        println!();
    }
    if failed.is_empty() {
        println!("\nEvery seed met the bars.");
        return;
    }
    let lines: Vec<String> = failed
        .iter()
        .map(|(seed, fails)| format!("seed {seed}: {}", fails.join(", ")))
        .collect();
    println!("\nBars failed: {}.", lines.join("; "));
    let trips: Vec<String> = runs
        .iter()
        .filter(|run| failed.iter().any(|(seed, _)| *seed == run.seed))
        .filter(|run| run.lacking.iter().any(|(_, share)| *share > 0.5))
        .map(|run| run.seed.to_string())
        .collect();
    if !trips.is_empty() {
        println!(
            "Careful copying would switch on: seeds {} failed a bar with more than half their \
             living lacking a needs gene.",
            trips.join(", ")
        );
    }
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        seeds: (1..=10).collect(),
        hours: 220,
        every: 10,
        jobs: 4,
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--seeds" => options.seeds = args::seed_list("--seeds", &mut args)?,
            "--hours" => options.hours = args::positive("--hours", &mut args)?,
            "--every" => options.every = args::positive("--every", &mut args)?,
            "--jobs" => options.jobs = args::positive("--jobs", &mut args)?,
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok(options)
}
