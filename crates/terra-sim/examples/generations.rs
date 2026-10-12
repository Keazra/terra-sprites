//! Whether the default world keeps going, generation after generation (M2
//! design §10, Slice 23's bar): runs it from seeds 1–10, left alone, and
//! reports for each seed the sprites alive at the end, the most at once,
//! the eggs laid with a partner and alone, the hatchings, the sprites that
//! turned female, the deepest generation reached and how many hatchlings
//! grew up. The bar: sprites alive at the end in at least 8 of 10 seeds,
//! and never more than the world's limit.
//!
//! `cargo run --release -p terra-sim --example generations`
//!
//! `--ticks N` runs each world N ticks (100,000 by default), and `--seeds
//! A-B` the seeds A to B (1-10 by default).

#[path = "common/args.rs"]
mod args;

use std::collections::BTreeMap;
use std::process::ExitCode;

use terra_sim::{DataPack, EntityId, EventKind, Outcome, Verb, World, WorldConfig};

struct Options {
    ticks: u64,
    seeds: (u64, u64),
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
}

fn main() -> ExitCode {
    let usage = "generations [--ticks N] [--seeds A-B]";
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
    for seed in first..=last {
        let run = run(&data, &config, seed, options.ticks);
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
    }
    let seeds = last - first + 1;
    println!("\nStill going in {going} of {seeds} seeds; over the limit in {over}.");
    ExitCode::SUCCESS
}

/// Runs the default world from `seed` for `ticks` ticks.
fn run(data: &DataPack, config: &WorldConfig, seed: u64, ticks: u64) -> Run {
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
    run
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        ticks: 100_000,
        seeds: (1, 10),
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--ticks" => options.ticks = args::positive("--ticks", &mut args)?,
            "--seeds" => options.seeds = args::seeds("--seeds", &mut args)?,
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok(options)
}
