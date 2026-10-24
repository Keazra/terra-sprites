//! Why the default world dies out (#221, from #212): runs the default world,
//! left alone, from the given seeds, each of the given ways, and reports
//! every few hours of play how the population and its food stand. So a
//! die-out can be told apart: the genes wearing out (broken genes and gene
//! values moving, while food stays) or the berry bushes eaten away (bushes
//! and fallen berries falling, food further off, while the genes hold).
//!
//! `cargo run --release -p terra-sim --example dieout -- --seeds 5,9 --hours 160`
//!
//! `--seeds` runs the seeds listed, `5,9`, or a range, `1-10` (9 by default),
//! `--hours N` each for N hours of play at 1× (160 by default), `--every N`
//! reports every N hours (5 by default), and `--jobs N` runs N worlds at a
//! time (4 by default). Each run's table is printed as it finishes, and the
//! summary at the end.
//! `--ways` names the ways to run each seed, separated by commas (`as-is` by
//! default). Each way but `as-is` is a control run for it, without one thing:
//!
//! - `as-is`: the built-in data pack.
//! - `no-flaws`: no hidden flaws, in the first sprites or as genes are copied
//!   (M2 design §12.3): `physiology.ron`'s `flaws` `founders` and `arise` at 0.
//! - `no-changes`: genes copied without copying errors or copy and change
//!   (M2 design §12.1, §13): `mutation`'s `chance` and `copy_and_change`'s
//!   rates at 0. Hidden flaws stay.
//! - `unchanging`: both, so a hatchling's genes are only its parents' mixed.
//! - `evergreen`: berry bushes that never expire once grown, as their last
//!   stage starts again when it ends (`objects.ron`), so the food can't wear
//!   out. The genes are as they are.
//!
//! A way sets values by their names, so retuning the data leaves it working.
//! A rate of 0 draws nothing (`mutation.rs`), so a way's world parts from
//! the as-is world of the same seed early: `no-flaws` and `unchanging` from
//! the first sprites, `no-changes` from the first egg, `evergreen` when the
//! first bush would have expired. Compare ways over several seeds, not seed
//! by seed.
//!
//! Each row: the hour; sprites and eggs; bushes, the seedlings among them,
//! the bushes with fruit, the fruit they hold and the berries lying on the
//! ground; the bushes that grew and expired since the last row; how often
//! sprites ate, and how far on average a sprite was, as the crow flies,
//! from the nearest fruit or berry; the hatchings and the deaths by hunger,
//! thirst and old age since the last row; the broken genes (M2 design
//! §12.4) on average among the hatched sprites alive; and among all the
//! sprites alive, the first sprites too while they live, how many of the
//! starter's gene values have a middle moved by a tenth either way (G4,
//! §12.7; a value no sprite has any more counts as not moved), and the
//! middle of `ate Level → hunger: gain` against the starter's (how much
//! eating eases hunger).

#[path = "common/args.rs"]
mod args;
#[path = "common/parallel.rs"]
mod parallel;

use std::process::ExitCode;

use terra_sim::{
    DataPack, DeathCause, EventKind, Outcome, Removal, Verb, World, WorldConfig, drift,
    gene_values, middle_values,
};

/// Ticks in an hour at 1×, 1.25 ticks a second (design §6.6).
const HOUR: u64 = 4_500;

/// The object types the rows count.
const BUSH: &str = "berry_bush";
const BERRY: &str = "berry";

/// The gene value the rows follow by itself: how much eating eases hunger.
const ATE_HUNGER_GAIN: &str = "ate Level → hunger: gain";

/// A value a way sets in a built-in data file. `at` finds it: the first
/// text must be in the file once, and each after it is the first of it
/// after the one before. The value right after the last, a word, a number
/// or a bracketed group, becomes `value`.
struct Change {
    file: &'static str,
    at: &'static [&'static str],
    value: &'static str,
}

const NO_FLAWS: [Change; 2] = [
    Change {
        file: "physiology.ron",
        at: &["flaws: (", "arise: "],
        value: "0.0",
    },
    Change {
        file: "physiology.ron",
        at: &["flaws: (", "founders: "],
        value: "0.0",
    },
];

const NO_CHANGES: [Change; 4] = [
    Change {
        file: "physiology.ron",
        at: &["mutation: (", "chance: "],
        value: "0.0",
    },
    Change {
        file: "physiology.ron",
        at: &["copy_and_change: (", "doubled: "],
        value: "0.0",
    },
    Change {
        file: "physiology.ron",
        at: &["copy_and_change: (", "lost: "],
        value: "0.0",
    },
    Change {
        file: "physiology.ron",
        at: &["copy_and_change: (", "changed: "],
        value: "0.0",
    },
];

const EVERGREEN: [Change; 1] = [Change {
    file: "objects.ron",
    at: &[r#"name: "berry_bush""#, r#"name: "mature""#, "next: "],
    value: r#"Stage("mature")"#,
}];

/// One way to run a seed: the built-in data pack with some values set.
struct Way {
    name: &'static str,
    changes: &'static [&'static [Change]],
}

static WAYS: [Way; 5] = [
    Way {
        name: "as-is",
        changes: &[],
    },
    Way {
        name: "no-flaws",
        changes: &[&NO_FLAWS],
    },
    Way {
        name: "no-changes",
        changes: &[&NO_CHANGES],
    },
    Way {
        name: "unchanging",
        changes: &[&NO_FLAWS, &NO_CHANGES],
    },
    Way {
        name: "evergreen",
        changes: &[&EVERGREEN],
    },
];

impl Way {
    /// The built-in pack, with the way's values set.
    fn data(&self) -> DataPack {
        let mut sources: Vec<(&str, String)> = DataPack::builtin_sources()
            .iter()
            .map(|&(path, text)| (path, text.to_string()))
            .collect();
        for change in self.changes.iter().copied().flatten() {
            let (_, text) = sources
                .iter_mut()
                .find(|(path, _)| *path == change.file)
                .expect("the built-in pack has the file");
            change.make(text, self.name);
        }
        let sources: Vec<(&str, &str)> = sources
            .iter()
            .map(|(path, text)| (*path, text.as_str()))
            .collect();
        DataPack::from_sources(&sources).expect("the changed pack is valid")
    }
}

impl Change {
    /// Sets the value in `text`, the file's text, for the way `way`.
    fn make(&self, text: &mut String, way: &str) {
        let Change { file, at, value } = self;
        assert!(
            text.matches(at[0]).count() == 1,
            "{way} expects {file} to hold `{}` once",
            at[0]
        );
        let mut start = 0;
        for anchor in *at {
            let found = text[start..]
                .find(anchor)
                .unwrap_or_else(|| panic!("{way} expects `{anchor}` in {file}, after {at:?}"));
            start += found + anchor.len();
        }
        let mut depth = 0;
        let length = text[start..]
            .find(|c: char| match c {
                '(' => {
                    depth += 1;
                    false
                }
                ')' if depth > 0 => {
                    depth -= 1;
                    false
                }
                ',' | ')' => depth == 0,
                _ => false,
            })
            .unwrap_or_else(|| panic!("{way} expects a value after {at:?} in {file}"));
        text.replace_range(start..start + length, value);
    }
}

struct Options {
    seeds: Vec<u64>,
    hours: u64,
    every: u64,
    jobs: u64,
    ways: Vec<&'static Way>,
}

/// Hatchings, and deaths by cause.
#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    hatched: u64,
    hunger: u64,
    thirst: u64,
    old_age: u64,
    hurt: u64,
}

impl Tally {
    fn died(&mut self, cause: DeathCause) {
        *match cause {
            DeathCause::OldAge => &mut self.old_age,
            DeathCause::Starvation => &mut self.hunger,
            DeathCause::Dehydration => &mut self.thirst,
            DeathCause::HurtBy(_) => &mut self.hurt,
        } += 1;
    }

    fn add(&mut self, other: Tally) {
        self.hatched += other.hatched;
        self.hunger += other.hunger;
        self.thirst += other.thirst;
        self.old_age += other.old_age;
        self.hurt += other.hurt;
    }
}

/// One report row.
#[derive(Debug, Default, Clone)]
struct Row {
    tick: u64,
    sprites: usize,
    eggs: usize,
    bushes: usize,
    seedlings: usize,
    fruiting: usize,
    fruit: u64,
    berries: usize,
    /// Bushes that grew and expired since the last row.
    grew: u64,
    expired: u64,
    /// Eats since the last row, of fruit on a bush and of berries.
    ate_bush: u64,
    ate_berry: u64,
    /// The mean distance in tiles from a sprite to the nearest fruit on a
    /// bush or berry on the ground; `None` with no sprites or no food.
    food_distance: Option<f64>,
    /// Since the last row.
    tally: Tally,
    /// Broken genes on average among the hatched sprites alive.
    broken: Option<f64>,
    /// Among the sprites alive: the starter's values moved by a tenth, and
    /// `ate Level → hunger: gain` against the starter's.
    moved: Option<usize>,
    ate_gain: Option<f32>,
}

/// What one seed's world came to, run one way.
struct Run {
    seed: u64,
    way: &'static Way,
    rows: Vec<Row>,
    died_out: Option<u64>,
    total: Tally,
}

/// The starter genome's gene values, which the rows measure against.
struct Starter {
    values: Vec<(String, f32)>,
    /// Where `ate Level → hunger: gain` is among them.
    ate_gain: Option<usize>,
}

fn main() -> ExitCode {
    let usage = "dieout [--seeds A-B | a,b,c] [--hours N] [--every N] [--jobs N] \
                 [--ways as-is,no-flaws,no-changes,unchanging,evergreen]";
    let Some(options) = args::parse(usage, parse) else {
        return ExitCode::FAILURE;
    };
    args::warn_if_debug(", or it will take a long time");
    let ways: Vec<&str> = options.ways.iter().map(|way| way.name).collect();
    let seeds: Vec<String> = options.seeds.iter().map(u64::to_string).collect();
    println!(
        "Die-out: the default world, left alone, seeds {}, {} hours each ({} ticks), ways: {}.",
        seeds.join(", "),
        options.hours,
        options.hours * HOUR,
        ways.join(", ")
    );
    let runs = in_parallel(&options);
    println!("\nSummary");
    println!(
        "{:<11} {:>5} {:>9} {:>8} {:>7} {:>7} {:>7} {:>6}",
        "Way", "Seed", "Died out", "Hatched", "Hunger", "Thirst", "OldAge", "Hurt"
    );
    for run in &runs {
        let died_out = run
            .died_out
            .map_or("-".to_string(), |tick| format!("{:.1} h", hours(tick)));
        let total = run.total;
        println!(
            "{:<11} {:>5} {died_out:>9} {:>8} {:>7} {:>7} {:>7} {:>6}",
            run.way.name,
            run.seed,
            total.hatched,
            total.hunger,
            total.thirst,
            total.old_age,
            total.hurt
        );
    }
    ExitCode::SUCCESS
}

/// Runs every seed every way, `options.jobs` at a time, and returns the
/// runs by way, in the order given, then seed.
fn in_parallel(options: &Options) -> Vec<Run> {
    let mut todo: Vec<(&'static Way, u64)> = Vec::new();
    for &way in &options.ways {
        for &seed in &options.seeds {
            todo.push((way, seed));
        }
    }
    let mut done = parallel::in_parallel(todo, options.jobs, |(way, seed)| {
        let run = run(way, seed, options);
        // Its table printed whole, before another run's.
        let _whole = std::io::stdout().lock();
        print_run(&run);
        run
    });
    let order = |way: &Way| options.ways.iter().position(|&w| w.name == way.name);
    done.sort_by_key(|run| (order(run.way), run.seed));
    done
}

/// Runs the default world from `seed`, changed as `way` says.
fn run(way: &'static Way, seed: u64, options: &Options) -> Run {
    let data = way.data();
    let config = WorldConfig::builtin(&data);
    let values = gene_values(&data.starter().genes(&data), &data.starter().switches());
    let starter = Starter {
        ate_gain: values.iter().position(|(name, _)| name == ATE_HUNGER_GAIN),
        values,
    };
    let (bush, berry) = (
        data.object_type_id(BUSH).expect("a berry bush type"),
        data.object_type_id(BERRY).expect("a berry type"),
    );
    let mut world = World::new(config, data.clone(), seed);
    let mut run = Run {
        seed,
        way,
        rows: Vec::new(),
        died_out: None,
        total: Tally::default(),
    };
    let mut row = Row::default();
    fill_in(&mut row, &world, &starter);
    run.rows.push(std::mem::take(&mut row));
    let ticks = options.hours * HOUR;
    let every = options.every * HOUR;
    for _ in 0..ticks {
        for event in world.step() {
            match &event.kind {
                EventKind::Hatched { .. } => row.tally.hatched += 1,
                EventKind::Died { cause, .. } => row.tally.died(*cause),
                EventKind::ObjectSpawned { object_type, .. } if object_type == BUSH => {
                    row.grew += 1;
                }
                EventKind::ObjectRemoved {
                    object_type,
                    reason: Removal::Expired,
                    ..
                } if object_type == BUSH => row.expired += 1,
                EventKind::ActionEnded {
                    verb: Verb::Eat,
                    outcome: Outcome::Applied,
                    action,
                    ..
                } => {
                    if action.target_type == Some(bush) {
                        row.ate_bush += 1;
                    } else if action.target_type == Some(berry) {
                        row.ate_berry += 1;
                    }
                }
                _ => {}
            }
        }
        if run.died_out.is_none() && world.sprites().count() + world.eggs().count() == 0 {
            run.died_out = Some(world.tick());
        }
        if world.tick().is_multiple_of(every) || world.tick() == ticks {
            fill_in(&mut row, &world, &starter);
            run.total.add(row.tally);
            eprintln!(
                "{} seed {seed}: hour {:.0}, {} sprites, {} bushes",
                way.name,
                hours(row.tick),
                row.sprites,
                row.bushes
            );
            run.rows.push(std::mem::take(&mut row));
            if run.died_out.is_some() {
                break;
            }
        }
    }
    run
}

/// Fills in `row` from `world` as it stands; the counts since the last row
/// are already in it.
fn fill_in(row: &mut Row, world: &World, starter: &Starter) {
    row.tick = world.tick();
    row.sprites = world.sprites().count();
    row.eggs = world.eggs().count();
    let mut food = Vec::new();
    for object in world.objects() {
        match object.type_name() {
            BUSH => {
                row.bushes += 1;
                if object.stage() == Some("seedling") {
                    row.seedlings += 1;
                }
                let fruit = object.counter("fruit").unwrap_or(0);
                if fruit > 0 {
                    row.fruiting += 1;
                    food.push(object.pos());
                }
                row.fruit += u64::from(fruit);
            }
            BERRY => {
                row.berries += 1;
                food.push(object.pos());
            }
            _ => {}
        }
    }
    let distances: Vec<f64> = world
        .sprites()
        .filter_map(|sprite| {
            let at = sprite.pos();
            food.iter()
                .map(|pos| {
                    let dx = f64::from(pos.x) - f64::from(at.x);
                    let dy = f64::from(pos.y) - f64::from(at.y);
                    (dx * dx + dy * dy).sqrt()
                })
                .min_by(f64::total_cmp)
        })
        .collect();
    row.food_distance =
        (!distances.is_empty()).then(|| distances.iter().sum::<f64>() / distances.len() as f64);
    // Broken genes (M2 design §12.4), counted as the generations example does.
    let (mut broken, mut hatched) = (0, 0);
    for sprite in world.sprites().filter(|s| !s.parents().is_empty()) {
        hatched += 1;
        broken += sprite
            .flaws()
            .into_iter()
            .filter(|&flawed| flawed == 2)
            .count();
    }
    row.broken = (hatched > 0).then(|| broken as f64 / f64::from(hatched));
    // Values moved by a tenth, as G4 counts them (M2 design §12.7).
    let values = &starter.values;
    if let Some(middles) = middle_values(world, values) {
        let ratio = |index: usize| middles[index].map(|middle| middle / values[index].1);
        row.moved = Some(
            (0..values.len())
                .filter(|&index| values[index].1 != 0.0)
                .filter(|&index| ratio(index).is_some_and(|r| drift(r).is_some()))
                .count(),
        );
        row.ate_gain = starter.ate_gain.and_then(ratio);
    }
}

fn hours(tick: u64) -> f64 {
    tick as f64 / HOUR as f64
}

fn print_run(run: &Run) {
    println!("\n{} seed {}", run.way.name, run.seed);
    println!(
        "{:>5} {:>7} {:>4} {:>6} {:>5} {:>8} {:>5} {:>7} {:>4} {:>7} {:>6} {:>6} {:>6} \
         {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>5}",
        "Hour",
        "Sprites",
        "Eggs",
        "Bushes",
        "Seedl",
        "Fruiting",
        "Fruit",
        "Berries",
        "Grew",
        "Expired",
        "AteB",
        "AteBy",
        "FoodD",
        "Hatched",
        "Hunger",
        "Thirst",
        "OldAge",
        "Broken",
        "Moved",
        "AteG"
    );
    let or_dash = |value: Option<String>| value.unwrap_or_else(|| "-".to_string());
    for row in &run.rows {
        println!(
            "{:>5.1} {:>7} {:>4} {:>6} {:>5} {:>8} {:>5} {:>7} {:>4} {:>7} {:>6} {:>6} {:>6} \
             {:>7} {:>6} {:>6} {:>6} {:>6} {:>6} {:>5}",
            hours(row.tick),
            row.sprites,
            row.eggs,
            row.bushes,
            row.seedlings,
            row.fruiting,
            row.fruit,
            row.berries,
            row.grew,
            row.expired,
            row.ate_bush,
            row.ate_berry,
            or_dash(row.food_distance.map(|d| format!("{d:.1}"))),
            row.tally.hatched,
            row.tally.hunger,
            row.tally.thirst,
            row.tally.old_age,
            or_dash(row.broken.map(|b| format!("{b:.2}"))),
            or_dash(row.moved.map(|m| m.to_string())),
            or_dash(row.ate_gain.map(|g| format!("{g:.2}"))),
        );
    }
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        seeds: vec![9],
        hours: 160,
        every: 5,
        jobs: 4,
        ways: vec![&WAYS[0]],
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--seeds" => options.seeds = args::seed_list("--seeds", &mut args)?,
            "--hours" => options.hours = args::positive("--hours", &mut args)?,
            "--every" => options.every = args::positive("--every", &mut args)?,
            "--jobs" => options.jobs = args::positive("--jobs", &mut args)?,
            "--ways" => {
                let names = args
                    .next()
                    .ok_or("--ways needs a list, such as as-is,no-flaws")?;
                options.ways = names
                    .split(',')
                    .map(|name| {
                        WAYS.iter()
                            .find(|way| way.name == name)
                            .ok_or(format!("unknown way {name}"))
                    })
                    .collect::<Result<_, _>>()?;
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok(options)
}
