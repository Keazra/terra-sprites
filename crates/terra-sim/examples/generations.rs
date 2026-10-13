//! Whether the default world keeps going, generation after generation (M2
//! design §10, Slice 23's bar): runs it from seeds 1–10, left alone, and
//! reports for each seed the sprites alive at the end, the most at once,
//! the eggs laid with a partner and alone, the hatchings, the sprites that
//! turned female, the deepest generation reached and how many hatchlings
//! grew up; then how long its sprites lived and what they died of; then how
//! far the species' genes moved from the starter genome's (M2 design §12,
//! G4). The bar: sprites alive at the end in at least 8 of 10 seeds, and
//! never more than the world's limit.
//!
//! `cargo run --release -p terra-sim --example generations`
//!
//! `--ticks N` runs each world N ticks (100,000 by default), `--seeds A-B`
//! the seeds A to B (1-10 by default), and `--jobs N` N worlds at a time (4
//! by default). Each world is its own, so the results don't depend on it.

#[path = "common/args.rs"]
mod args;

use std::collections::BTreeMap;
use std::process::ExitCode;
use std::sync::Mutex;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, GeneView, Genome, Outcome, Verb, World, WorldConfig,
};

/// Ticks in an hour at 1×, 1.25 ticks a second (design §6.6).
const HOUR: f64 = 4_500.0;

struct Options {
    ticks: u64,
    seeds: (u64, u64),
    jobs: u64,
}

/// What one seed's world came to.
#[derive(Debug, Default)]
struct Run {
    /// Sprites alive at the end, eggs not counted, and eggs.
    alive: usize,
    eggs: usize,
    /// The most sprites at once, eggs included.
    most: usize,
    /// Eggs laid with a partner, and alone.
    mated: u64,
    alone: u64,
    /// Tries to mate that came to nothing.
    fruitless: u64,
    hatched: u64,
    turned_female: u64,
    /// Hatchlings that grew up.
    grew_up: u64,
    /// The deepest generation reached: the first population is 0.
    generation: u32,
    /// The tick the last sprite died, if they all did.
    died_out: Option<u64>,
    /// The ages at which hatchlings and the first sprites died of old age.
    lives: Vec<u64>,
    first_lives: Vec<u64>,
    /// The ages at which hatchlings died of old age in the run's second
    /// half, once lives have had generations to move (M2 design §12).
    late_lives: Vec<u64>,
    /// Each gene number's middle value among the sprites alive at the end
    /// whose genes line up with the starter's, in the starter's order; empty
    /// if none do.
    numbers: Vec<f32>,
    /// Deaths after tick 10,000, once the first sprites' ages have spread
    /// out (A4's window starts there): of old age, hunger, thirst, and hurt.
    old_age: u64,
    hunger: u64,
    thirst: u64,
    hurt: u64,
}

fn main() -> ExitCode {
    let usage = "generations [--ticks N] [--seeds A-B] [--jobs N]";
    let Some(options) = args::parse(usage, parse) else {
        return ExitCode::FAILURE;
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release, or it will take a long time");
    }
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let config = WorldConfig::builtin(&data);
    let limit = config.most_sprites();
    let (first, last) = options.seeds;
    println!(
        "Generations: the default world, left alone, seeds {first}–{last}, {} ticks each, \
         at most {limit} sprites.",
        options.ticks
    );
    println!(
        "\n{:>5} {:>6} {:>5} {:>5} {:>6} {:>9} {:>6} {:>7} {:>7} {:>7} {:>5} {:>9}",
        "Seed",
        "Alive",
        "Eggs",
        "Most",
        "Mated",
        "Fruitless",
        "Alone",
        "Hatched",
        "Female",
        "GrewUp",
        "Gen",
        "Died out"
    );
    let (mut going, mut over) = (0, 0);
    let starter = starter(&data);
    let mut runs = Vec::new();
    for (seed, run) in in_parallel(&data, &config, &starter, &options) {
        going += usize::from(run.alive + run.eggs > 0);
        over += usize::from(run.most > limit);
        let died_out = run
            .died_out
            .map_or("-".to_string(), |tick| tick.to_string());
        println!(
            "{seed:>5} {:>6} {:>5} {:>5} {:>6} {:>9} {:>6} {:>7} {:>7} {:>7} {:>5} {died_out:>9}",
            run.alive,
            run.eggs,
            run.most,
            run.mated,
            run.fruitless,
            run.alone,
            run.hatched,
            run.turned_female,
            run.grew_up,
            run.generation
        );
        runs.push((seed, run));
    }
    let seeds = last - first + 1;
    println!("\nStill going in {going} of {seeds} seeds; over the limit in {over}.");
    lives(&runs);
    evolution(&starter, &runs);
    ExitCode::SUCCESS
}

/// Runs the seeds' worlds, `options.jobs` at a time, and returns each
/// seed's in order.
fn in_parallel(
    data: &DataPack,
    config: &WorldConfig,
    starter: &[(String, f32)],
    options: &Options,
) -> Vec<(u64, Run)> {
    let (first, last) = options.seeds;
    let next = Mutex::new(first);
    let done = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..options.jobs {
            scope.spawn(|| {
                loop {
                    let seed = {
                        let mut next = next.lock().expect("no thread panicked");
                        let seed = *next;
                        *next += 1;
                        seed
                    };
                    if seed > last {
                        break;
                    }
                    let run = run(data, config, starter, seed, options.ticks);
                    done.lock().expect("no thread panicked").push((seed, run));
                }
            });
        }
    });
    let mut done = done.into_inner().expect("no thread panicked");
    done.sort_by_key(|&(seed, _)| seed);
    done
}

/// The starter genome's numbers, named, in genome order.
fn starter(data: &DataPack) -> Vec<(String, f32)> {
    let text = DataPack::builtin_sources()
        .iter()
        .find(|(path, _)| *path == "genomes/starter.ron")
        .map(|(_, text)| text)
        .expect("the built-in pack has a starter genome");
    let genome = Genome::from_ron(text, data).expect("the starter genome is valid");
    numbers(&genome.genes(data))
}

/// Each number in `genes`, named, in order (design §4.3's value fields).
fn numbers(genes: &[GeneView]) -> Vec<(String, f32)> {
    let mut numbers = Vec::new();
    for gene in genes {
        let mut push = |field: &str, name: String, value: f32| {
            numbers.push((format!("{name}: {field}"), value));
        };
        match gene {
            GeneView::HalfLife { chem, ticks } => {
                push("half-life", chem.to_string(), *ticks as f32)
            }
            GeneView::Reaction {
                reactants,
                products,
                rate,
            } => {
                let side = |terms: &[(&str, u8)]| {
                    let names: Vec<&str> = terms.iter().map(|(chem, _)| *chem).collect();
                    names.join(" + ")
                };
                push(
                    "rate",
                    format!("{} → {}", side(reactants), side(products)),
                    *rate,
                );
            }
            GeneView::Emitter {
                locus,
                mode,
                invert,
                threshold,
                gain,
                chem,
            } => {
                let low = if *invert { "low " } else { "" };
                let name = format!("{low}{locus} {mode:?} → {chem}");
                push("threshold", name.clone(), *threshold);
                push("gain", name, *gain);
            }
            GeneView::Receptor {
                chem,
                threshold,
                gain,
                target,
            } => {
                let name = format!("{chem} → {target}");
                push("threshold", name.clone(), *threshold);
                push("gain", name, *gain);
            }
            GeneView::InitialConcentration { chem, value } => {
                push("at birth", chem.to_string(), *value)
            }
            GeneView::Trait { which, value } => push("trait", format!("{which:?}"), *value),
            GeneView::BrainParam { param, value } => push("brain", param.to_string(), *value),
            GeneView::Instinct {
                inputs,
                verb,
                weight,
            } => {
                let inputs: Vec<String> = inputs
                    .iter()
                    .map(|(input, negated)| {
                        if *negated {
                            format!("not {input}")
                        } else {
                            input.to_string()
                        }
                    })
                    .collect();
                push(
                    "weight",
                    format!("{} → {verb:?}", inputs.join(" & ")),
                    *weight,
                );
            }
            GeneView::AttentionInstinct {
                input,
                category,
                weight,
            } => push(
                "weight",
                format!("{input} → attends to {category}"),
                *weight,
            ),
            GeneView::Unknown { .. } => {}
        }
    }
    numbers
}

/// How far the species' genes moved from the starter's (M2 design §12,
/// G4): each number whose middle value among the living moved by at least
/// a tenth the same way in every seed. Chance moves a number up in some
/// worlds and down in others, and the same way in all ten about 1 time in
/// 500. A number at the edge of its range can only move one way, so check
/// the list for one.
fn evolution(starter: &[(String, f32)], runs: &[(u64, Run)]) {
    let measured: Vec<&(u64, Run)> = runs
        .iter()
        .filter(|(_, run)| !run.numbers.is_empty())
        .collect();
    println!(
        "\nEvolution: each gene number's middle value among the living at the end, against the \
         starter's, in {} seeds.",
        measured.len()
    );
    if measured.is_empty() {
        return;
    }
    // Each number's ratio to the starter's, seed by seed.
    let mut moved = Vec::new();
    for (index, (name, value)) in starter.iter().enumerate() {
        if *value == 0.0 {
            continue;
        }
        let ratios: Vec<f32> = measured
            .iter()
            .map(|(_, run)| run.numbers[index] / value)
            .collect();
        let up = ratios.iter().filter(|&&r| r >= 1.1).count();
        let down = ratios.iter().filter(|&&r| r <= 0.9).count();
        let mut sorted = ratios.clone();
        sorted.sort_by(f32::total_cmp);
        let middle = sorted[sorted.len() / 2];
        moved.push((
            name,
            *value,
            sorted[0],
            middle,
            sorted[sorted.len() - 1],
            up,
            down,
        ));
    }
    let seeds = measured.len();
    let all_way: Vec<_> = moved
        .iter()
        .filter(|m| m.5 == seeds || m.6 == seeds)
        .collect();
    println!(
        "\n{} of {} numbers moved by at least a tenth the same way in all {seeds} seeds.",
        all_way.len(),
        moved.len()
    );
    println!(
        "\n{:<52} {:>10} {:>6} {:>6} {:>6} {:>4} {:>4}",
        "Number", "Starter", "Least", "Middle", "Most", "Up", "Down"
    );
    let mut by_size = moved.clone();
    by_size.sort_by(|a, b| {
        let size = |m: &(&String, f32, f32, f32, f32, usize, usize)| (distance(m.3)).abs();
        size(b).total_cmp(&size(a))
    });
    for (name, value, least, middle, most, up, down) in by_size.iter().take(25) {
        let mark = if *up == seeds || *down == seeds {
            "*"
        } else {
            " "
        };
        println!(
            "{mark}{name:<51} {value:>10.4} {least:>6.2} {middle:>6.2} {most:>6.2} {up:>4} {down:>4}"
        );
    }
    println!(
        "\n* moved the same way in every seed. Least, middle and most are seeds' ratios to the starter."
    );
}

/// How far a ratio is from 1, either way: halving and doubling come out
/// alike.
fn distance(ratio: f32) -> f32 {
    if ratio >= 1.0 {
        ratio - 1.0
    } else {
        1.0 / ratio - 1.0
    }
}

/// How long the sprites lived, in hours at 1×, and what they died of after
/// tick 10,000.
fn lives(runs: &[(u64, Run)]) {
    println!(
        "\nLives ended by old age, in hours at 1x: hatchlings' shortest, median and longest, \
         the median of those that died in the run's second half, then the first sprites'; and \
         deaths after tick 10,000."
    );
    println!(
        "\n{:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>6} {:>6} {:>5}",
        "Seed",
        "Short",
        "Median",
        "Long",
        "Late",
        "First",
        "to",
        "Old",
        "Hunger",
        "Thirst",
        "Hurt",
        "Need"
    );
    let hours = |ticks: u64| ticks as f64 / HOUR;
    let (mut all, mut needs, mut deaths) = (Vec::new(), 0, 0);
    let mut late_within = 0;
    for (seed, run) in runs {
        let mut late = run.late_lives.clone();
        late.sort_unstable();
        let late = late.get(late.len().saturating_sub(1) / 2).copied();
        late_within += usize::from(late.is_some_and(|age| (5.0..=7.0).contains(&hours(age))));
        let mut lives = run.lives.clone();
        lives.sort_unstable();
        all.extend(&lives);
        let at = |share: f64| lives.get(((lives.len() - 1) as f64 * share) as usize);
        let shown =
            |age: Option<&u64>| age.map_or("-".to_string(), |&a| format!("{:.1}", hours(a)));
        let first = |pick: fn(&u64, &u64) -> bool| {
            run.first_lives
                .iter()
                .copied()
                .reduce(|a, b| if pick(&a, &b) { a } else { b })
        };
        let need = run.hunger + run.thirst;
        let died = need + run.old_age + run.hurt;
        needs += need;
        deaths += died;
        println!(
            "{seed:>5} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>8} {:>6} {:>6} {:>4.1}%",
            shown(at(0.0)),
            shown(at(0.5)),
            shown(at(1.0)),
            shown(late.as_ref()),
            shown(first(|a, b| a < b).as_ref()),
            shown(first(|a, b| a > b).as_ref()),
            run.old_age,
            run.hunger,
            run.thirst,
            run.hurt,
            100.0 * need as f64 / died.max(1) as f64
        );
    }
    all.sort_unstable();
    if !all.is_empty() {
        let at = |share: f64| hours(all[((all.len() - 1) as f64 * share) as usize]);
        println!(
            "\nHatchlings' lives, all seeds: {} ended by old age, a tenth by {:.1} hours, half by \
             {:.1}, nine tenths by {:.1}.",
            all.len(),
            at(0.1),
            at(0.5),
            at(0.9)
        );
    }
    println!(
        "Hunger and thirst: {needs} of {deaths} deaths after tick 10,000 ({:.1}%).",
        100.0 * needs as f64 / deaths.max(1) as f64
    );
    println!(
        "Second half's median life between 5 and 7 hours in {late_within} of {} seeds.",
        runs.len()
    );
}

/// Runs the default world from `seed` for `ticks` ticks.
fn run(
    data: &DataPack,
    config: &WorldConfig,
    starter: &[(String, f32)],
    seed: u64,
    ticks: u64,
) -> Run {
    let mut world = World::new(config.clone(), data.clone(), seed);
    let mut run = Run::default();
    let mut generation: BTreeMap<EntityId, u32> =
        world.sprites().map(|sprite| (sprite.id(), 0)).collect();
    let mut hatchlings = Vec::new();
    for _ in 0..ticks {
        for event in world.step() {
            match event.kind {
                EventKind::Laid { egg, by, with } => {
                    let parents = [Some(by), with];
                    let deepest = parents
                        .iter()
                        .flatten()
                        .filter_map(|parent| generation.get(parent))
                        .max()
                        .copied()
                        .unwrap_or(0);
                    generation.insert(egg, deepest + 1);
                    run.generation = run.generation.max(deepest + 1);
                    if with.is_some() {
                        run.mated += 1;
                    } else {
                        run.alone += 1;
                    }
                }
                EventKind::Hatched { id, .. } => {
                    run.hatched += 1;
                    hatchlings.push(id);
                }
                EventKind::GrewUp { id } if hatchlings.contains(&id) => run.grew_up += 1,
                EventKind::Died { id, cause, age, .. } => {
                    if cause == DeathCause::OldAge {
                        if hatchlings.contains(&id) {
                            run.lives.push(age);
                            if world.tick() > ticks / 2 {
                                run.late_lives.push(age);
                            }
                        } else {
                            run.first_lives.push(age);
                        }
                    }
                    if world.tick() > 10_000 {
                        *match cause {
                            DeathCause::OldAge => &mut run.old_age,
                            DeathCause::Starvation => &mut run.hunger,
                            DeathCause::Dehydration => &mut run.thirst,
                            DeathCause::HurtBy(_) => &mut run.hurt,
                        } += 1;
                    }
                }
                EventKind::TurnedFemale { .. } => run.turned_female += 1,
                EventKind::ActionEnded {
                    verb: Verb::Mate,
                    outcome: Outcome::Failed,
                    ..
                } => run.fruitless += 1,
                _ => {}
            }
        }
        let count = world.sprites().count();
        run.most = run.most.max(count);
        if count == 0 && run.died_out.is_none() {
            run.died_out = Some(world.tick());
        }
    }
    run.eggs = world.sprites().filter(|sprite| sprite.is_egg()).count();
    run.alive = world.sprites().count() - run.eggs;
    run.numbers = middles(&world, starter);
    run
}

/// Each of the starter's numbers' middle value among the hatched sprites
/// alive in `world` whose genes line up with the starter's; empty if none
/// do.
fn middles(world: &World, starter: &[(String, f32)]) -> Vec<f32> {
    let mut columns: Vec<Vec<f32>> = vec![Vec::new(); starter.len()];
    for sprite in world.sprites().filter(|sprite| !sprite.is_egg()) {
        let genes: Vec<GeneView> = sprite.genes().into_iter().map(|(gene, _)| gene).collect();
        let numbers = numbers(&genes);
        let lined_up = numbers.len() == starter.len()
            && numbers.iter().zip(starter).all(|((a, _), (b, _))| a == b);
        if !lined_up {
            continue;
        }
        for (column, (_, value)) in columns.iter_mut().zip(numbers) {
            column.push(value);
        }
    }
    if columns.first().is_none_or(Vec::is_empty) {
        return Vec::new();
    }
    columns
        .into_iter()
        .map(|mut column| {
            column.sort_by(f32::total_cmp);
            column[column.len() / 2]
        })
        .collect()
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        ticks: 100_000,
        seeds: (1, 10),
        jobs: 4,
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--ticks" => options.ticks = args::positive("--ticks", &mut args)?,
            "--seeds" => options.seeds = args::seeds("--seeds", &mut args)?,
            "--jobs" => options.jobs = args::positive("--jobs", &mut args)?,
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok(options)
}
