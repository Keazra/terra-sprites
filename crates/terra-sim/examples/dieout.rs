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
//! default):
//!
//! - `as-is`: the built-in data pack.
//! - `no-flaws`: no hidden flaws, in the first sprites or as genes are copied
//!   (M2 design §12): `physiology.ron`'s `flaws` `founders` and `arise` at 0.
//! - `no-changes`: genes copied without copying errors or copy and change
//!   (M2 design §12, §13): `mutation`'s `chance` and `copy_and_change`'s rates
//!   at 0. Hidden flaws stay.
//! - `unchanging`: both, so a hatchling's genes are only its parents' mixed.
//! - `evergreen`: berry bushes that never die once grown (`objects.ron`), so
//!   the food can't wear out. The genes are as they are.
//!
//! Each row: the hour; sprites and eggs; bushes, the seedlings among them,
//! the bushes with fruit, the fruit they hold and the berries lying on the
//! ground; the bushes that grew and died since the last row; how often
//! sprites ate, and how far on average a sprite was from the nearest fruit
//! or berry; the hatchings and the deaths by hunger, thirst and old age
//! since the last row; and among the hatched sprites alive, the broken
//! genes on average, how many of the starter's gene values have a middle
//! moved by a tenth either way, and the middle of `ate → hunger: gain`
//! against the starter's (how much eating eases hunger).

#[path = "common/args.rs"]
mod args;

use std::collections::BTreeSet;
use std::process::ExitCode;
use std::sync::Mutex;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, Outcome, Verb, World, WorldConfig, drift,
    gene_values, middle_values,
};

/// Ticks in an hour at 1×, 1.25 ticks a second (design §6.6).
const HOUR: u64 = 4_500;

/// The gene value the rows follow by itself: how much eating eases hunger.
const ATE_HUNGER: &str = "ate Level → hunger: gain";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Way {
    AsIs,
    NoFlaws,
    NoChanges,
    Unchanging,
    Evergreen,
}

impl Way {
    const ALL: [Way; 5] = [
        Way::AsIs,
        Way::NoFlaws,
        Way::NoChanges,
        Way::Unchanging,
        Way::Evergreen,
    ];

    fn name(self) -> &'static str {
        match self {
            Way::AsIs => "as-is",
            Way::NoFlaws => "no-flaws",
            Way::NoChanges => "no-changes",
            Way::Unchanging => "unchanging",
            Way::Evergreen => "evergreen",
        }
    }

    /// What the way changes in the built-in pack: `(file, [(text, new text)])`.
    fn changes(self) -> Vec<(&'static str, &'static str, &'static str)> {
        const NO_FLAWS: (&str, &str, &str) = (
            "physiology.ron",
            "flaws: (arise: 0.004, founders: 0.035,",
            "flaws: (arise: 0.0, founders: 0.0,",
        );
        const NO_ERRORS: (&str, &str, &str) = (
            "physiology.ron",
            "mutation: (chance: 0.01,",
            "mutation: (chance: 0.0,",
        );
        const NO_COPY_AND_CHANGE: (&str, &str, &str) = (
            "physiology.ron",
            "copy_and_change: (doubled: 0.00029, lost: 0.00029, changed: 0.00029,",
            "copy_and_change: (doubled: 0.0, lost: 0.0, changed: 0.0,",
        );
        const EVERGREEN: (&str, &str, &str) = (
            "objects.ron",
            r#"(name: "mature",   ticks: (20000, 30000), next: Expire),"#,
            r#"(name: "mature",   ticks: (4000000, 4000000), next: Expire),"#,
        );
        match self {
            Way::AsIs => vec![],
            Way::NoFlaws => vec![NO_FLAWS],
            Way::NoChanges => vec![NO_ERRORS, NO_COPY_AND_CHANGE],
            Way::Unchanging => vec![NO_FLAWS, NO_ERRORS, NO_COPY_AND_CHANGE],
            Way::Evergreen => vec![EVERGREEN],
        }
    }

    /// The built-in pack, changed the way says.
    fn data(self) -> DataPack {
        let mut sources: Vec<(String, String)> = DataPack::builtin_sources()
            .iter()
            .map(|&(path, text)| (path.to_string(), text.to_string()))
            .collect();
        for (file, from, to) in self.changes() {
            let (_, text) = sources
                .iter_mut()
                .find(|(path, _)| path == file)
                .expect("the built-in pack has the file");
            assert!(
                text.matches(from).count() == 1,
                "{} expects {file} to hold `{from}` once",
                self.name()
            );
            *text = text.replace(from, to);
        }
        let sources: Vec<(&str, &str)> = sources
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        DataPack::from_sources(&sources).expect("the changed pack is valid")
    }
}

struct Options {
    seeds: Vec<u64>,
    hours: u64,
    every: u64,
    jobs: u64,
    ways: Vec<Way>,
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
    grew: usize,
    died: usize,
    /// Eats since the last row, of fruit on a bush and of berries.
    ate_bush: u64,
    ate_berry: u64,
    /// The mean distance in tiles from a sprite to the nearest fruit on a
    /// bush or berry on the ground; `None` with no sprites or no food.
    food_distance: Option<f64>,
    hatched: u64,
    hunger: u64,
    thirst: u64,
    old_age: u64,
    hurt: u64,
    /// Among the hatched sprites alive: broken genes on average, values
    /// moved by a tenth, and `ate → hunger: gain` against the starter's.
    broken: Option<f64>,
    moved: Option<usize>,
    ate_gain: Option<f32>,
}

/// What one seed's world came to, run one way.
struct Run {
    seed: u64,
    way: Way,
    rows: Vec<Row>,
    died_out: Option<u64>,
    /// Totals over the run.
    hatched: u64,
    hunger: u64,
    thirst: u64,
    old_age: u64,
    hurt: u64,
}

fn main() -> ExitCode {
    let usage = "dieout [--seeds A-B | a,b,c] [--hours N] [--every N] [--jobs N] \
                 [--ways as-is,no-flaws,no-changes,unchanging,evergreen]";
    let Some(options) = args::parse(usage, parse) else {
        return ExitCode::FAILURE;
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release, or it will take a long time");
    }
    let ways: Vec<&str> = options.ways.iter().map(|way| way.name()).collect();
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
        println!(
            "{:<11} {:>5} {died_out:>9} {:>8} {:>7} {:>7} {:>7} {:>6}",
            run.way.name(),
            run.seed,
            run.hatched,
            run.hunger,
            run.thirst,
            run.old_age,
            run.hurt
        );
    }
    ExitCode::SUCCESS
}

/// Runs every seed every way, `options.jobs` at a time, and returns the
/// runs by way, then seed.
fn in_parallel(options: &Options) -> Vec<Run> {
    let mut todo: Vec<(Way, u64)> = Vec::new();
    for &way in &options.ways {
        for &seed in &options.seeds {
            todo.push((way, seed));
        }
    }
    let todo = Mutex::new(todo.into_iter());
    let done = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..options.jobs {
            scope.spawn(|| {
                loop {
                    let next = todo.lock().expect("no thread panicked").next();
                    let Some((way, seed)) = next else {
                        break;
                    };
                    let run = run(way, seed, options);
                    let mut done = done.lock().expect("no thread panicked");
                    print_run(&run);
                    done.push(run);
                }
            });
        }
    });
    let mut done = done.into_inner().expect("no thread panicked");
    done.sort_by_key(|run| (run.way, run.seed));
    done
}

/// Runs the default world from `seed`, changed as `way` says.
fn run(way: Way, seed: u64, options: &Options) -> Run {
    let data = way.data();
    let config = WorldConfig::builtin(&data);
    let starter = gene_values(&data.starter().genes(&data));
    let ate_gain = starter.iter().position(|(name, _)| name == ATE_HUNGER);
    let (bush, berry) = (
        data.object_type_id("berry_bush")
            .expect("a berry bush type"),
        data.object_type_id("berry").expect("a berry type"),
    );
    let mut world = World::new(config, data.clone(), seed);
    let mut run = Run {
        seed,
        way,
        rows: Vec::new(),
        died_out: None,
        hatched: 0,
        hunger: 0,
        thirst: 0,
        old_age: 0,
        hurt: 0,
    };
    let mut row = Row::default();
    let mut bushes_before: BTreeSet<EntityId> = BTreeSet::new();
    take_row(
        &world,
        &starter,
        ate_gain,
        &mut row,
        &mut bushes_before,
        true,
    );
    run.rows.push(std::mem::take(&mut row));
    let ticks = options.hours * HOUR;
    let every = options.every * HOUR;
    for _ in 0..ticks {
        for event in world.step() {
            match event.kind {
                EventKind::Hatched { .. } => row.hatched += 1,
                EventKind::Died { cause, .. } => {
                    *match cause {
                        DeathCause::OldAge => &mut row.old_age,
                        DeathCause::Starvation => &mut row.hunger,
                        DeathCause::Dehydration => &mut row.thirst,
                        DeathCause::HurtBy(_) => &mut row.hurt,
                    } += 1;
                }
                EventKind::ActionEnded {
                    verb: Verb::Eat,
                    outcome: Outcome::Applied,
                    ref action,
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
            take_row(
                &world,
                &starter,
                ate_gain,
                &mut row,
                &mut bushes_before,
                false,
            );
            run.hatched += row.hatched;
            run.hunger += row.hunger;
            run.thirst += row.thirst;
            run.old_age += row.old_age;
            run.hurt += row.hurt;
            eprintln!(
                "{} seed {seed}: hour {:.0}, {} sprites, {} bushes",
                way.name(),
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
/// are already in it. `before` holds the bushes at the last row, and then
/// those now.
fn take_row(
    world: &World,
    starter: &[(String, f32)],
    ate_gain: Option<usize>,
    row: &mut Row,
    before: &mut BTreeSet<EntityId>,
    first: bool,
) {
    row.tick = world.tick();
    row.sprites = world.sprites().count();
    row.eggs = world.eggs().count();
    let mut bushes = BTreeSet::new();
    let mut food = Vec::new();
    for object in world.objects() {
        match object.type_name() {
            "berry_bush" => {
                bushes.insert(object.id());
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
            "berry" => {
                row.berries += 1;
                food.push(object.pos());
            }
            _ => {}
        }
    }
    row.bushes = bushes.len();
    if !first {
        row.grew = bushes.difference(before).count();
        row.died = before.difference(&bushes).count();
    }
    *before = bushes;
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
    if hatched > 0
        && let Some(middles) = middle_values(world, starter)
    {
        let ratio = |index: usize| middles[index].map(|middle| middle / starter[index].1);
        row.moved = Some(
            (0..starter.len())
                .filter(|&index| starter[index].1 != 0.0)
                .filter(|&index| ratio(index).is_some_and(|r| drift(r).is_some()))
                .count(),
        );
        row.ate_gain = ate_gain.and_then(ratio);
    }
}

fn hours(tick: u64) -> f64 {
    tick as f64 / HOUR as f64
}

fn print_run(run: &Run) {
    println!("\n{} seed {}", run.way.name(), run.seed);
    println!(
        "{:>5} {:>7} {:>4} {:>6} {:>5} {:>8} {:>5} {:>7} {:>4} {:>4} {:>6} {:>6} {:>6} \
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
        "Died",
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
            "{:>5.1} {:>7} {:>4} {:>6} {:>5} {:>8} {:>5} {:>7} {:>4} {:>4} {:>6} {:>6} {:>6} \
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
            row.died,
            row.ate_bush,
            row.ate_berry,
            or_dash(row.food_distance.map(|d| format!("{d:.1}"))),
            row.hatched,
            row.hunger,
            row.thirst,
            row.old_age,
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
        ways: vec![Way::AsIs],
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--seeds" => options.seeds = seeds(&mut args)?,
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
                        Way::ALL
                            .into_iter()
                            .find(|way| way.name() == name)
                            .ok_or(format!("unknown way {name}"))
                    })
                    .collect::<Result<_, _>>()?;
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok(options)
}

/// The seeds `--seeds` lists, `5,9`, or ranges over, `1-10`.
fn seeds(args: &mut std::slice::Iter<'_, String>) -> Result<Vec<u64>, String> {
    let value = args
        .next()
        .ok_or("--seeds needs seeds, such as 5,9 or 1-10")?;
    let bad = || format!("--seeds needs seeds, such as 5,9 or 1-10, not {value}");
    if let Some((a, b)) = value.split_once('-') {
        let (a, b): (u64, u64) = (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
        return if a <= b {
            Ok((a..=b).collect())
        } else {
            Err(bad())
        };
    }
    value
        .split(',')
        .map(|seed| seed.parse().map_err(|_| bad()))
        .collect()
}
