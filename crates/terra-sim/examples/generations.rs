//! Whether the default world keeps going, generation after generation (M2
//! design §10, Slice 23's bar): runs it from seeds 1–10, left alone, and
//! reports for each seed the sprites alive at the end, the most at once,
//! the eggs laid with a partner and alone, the hatchings, the sprites that
//! turned female, the deepest generation reached and how many hatchlings
//! grew up; then how long its sprites lived and what they died of; then the
//! hidden flaws its sprites carried (M2 design §12), and how long young
//! from lone eggs lived against young from pairs; then how far the species'
//! genes moved from the starter genome's (§12, G4); then the genes the
//! starter lacks that the living carry, and in how many each works, and
//! the genes they lost and doubled (§13). The bars: sprites alive at the end in at least 8 of 10
//! seeds, and never more than the world's limit; at least 3 gene numbers
//! moved by a tenth the same way in every seed (§12.7); and in at least 5
//! of 10 seeds, a gene the starter lacks carried by at least a tenth of the
//! living (§13).
//!
//! `cargo run --release -p terra-sim --example generations`
//!
//! `--ticks N` runs each world N ticks (100,000 by default), `--seeds A-B`
//! the seeds A to B (1-10 by default), and `--jobs N` N worlds at a time (4
//! by default). Each world is its own, so the results don't depend on it.

#[path = "common/args.rs"]
mod args;

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::sync::Mutex;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, Expression, GeneView, Genome, LifeStage, Outcome,
    Switch, Verb, World, WorldConfig, gene_values, middle_values,
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
    /// The ages at which young from lone eggs and from pairs died of old
    /// age (M2 design §12).
    lone_lives: Vec<u64>,
    pair_lives: Vec<u64>,
    /// The flaws among the hatched sprites alive halfway through and at
    /// the end (M2 design §12).
    flaws_halfway: Option<FlawLoad>,
    flaws_at_end: Option<FlawLoad>,
    /// Each gene number's middle value among the sprites alive at the end,
    /// lined up with the starter's, in the starter's order; empty if none
    /// is alive, and not a number where none has the gene.
    numbers: Vec<f32>,
    /// The genes of the hatched sprites alive at the end (M2 design §13).
    genes: Option<GenePool>,
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
    let (starter, genes) = (starter(&data), starter_genes(&data));
    let mut runs = Vec::new();
    for (seed, run) in in_parallel(&data, &config, &starter, &genes, &options) {
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
    flaws(&runs);
    evolution(&starter, &runs);
    copy_and_change(&runs);
    ExitCode::SUCCESS
}

/// Runs the seeds' worlds, `options.jobs` at a time, and returns each
/// seed's in order.
fn in_parallel(
    data: &DataPack,
    config: &WorldConfig,
    starter: &[(String, f32)],
    genes: &BTreeMap<String, usize>,
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
                    let run = run(data, config, starter, genes, seed, options.ticks);
                    done.lock().expect("no thread panicked").push((seed, run));
                }
            });
        }
    });
    let mut done = done.into_inner().expect("no thread panicked");
    done.sort_by_key(|&(seed, _)| seed);
    done
}

/// The starter genome.
fn starter_genome(data: &DataPack) -> Genome {
    let text = DataPack::builtin_sources()
        .iter()
        .find(|(path, _)| *path == "genomes/starter.ron")
        .map(|(_, text)| text)
        .expect("the built-in pack has a starter genome");
    Genome::from_ron(text, data).expect("the starter genome is valid")
}

/// The starter genome's numbers, named, in genome order.
fn starter(data: &DataPack) -> Vec<(String, f32)> {
    gene_values(&starter_genome(data).genes(data))
}

/// The starter's genes, by what each does and when, with how many it has
/// of each (M2 design §13).
fn starter_genes(data: &DataPack) -> BTreeMap<String, usize> {
    let genome = starter_genome(data);
    counted(&genome.genes(data), &genome.switches())
}

/// How many of `genes` there are of each, by what each does and when it
/// works, `switches` saying when: `thirst → Drink`, `always → Wander from
/// Adult`. Genes this build can't read aren't counted.
fn counted(genes: &[GeneView], switches: &[Switch]) -> BTreeMap<String, usize> {
    counted_where(genes, switches, |_| true)
}

/// How many of `genes` there are of each, as `counted` says, of those at
/// the indices `keep` keeps.
fn counted_where(
    genes: &[GeneView],
    switches: &[Switch],
    keep: impl Fn(usize) -> bool,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for (index, (gene, switch)) in genes.iter().zip(switches).enumerate() {
        if !keep(index) {
            continue;
        }
        if let Some((name, _)) = gene.named() {
            *counts
                .entry(format!("{name}{}", when(*switch)))
                .or_insert(0) += 1;
        }
    }
    counts
}

/// When a gene works, after what it does: nothing for all its life.
fn when(switch: Switch) -> String {
    let mut text = String::new();
    if switch.from != LifeStage::Young {
        text += &format!(" from {:?}", switch.from);
    }
    if let Some(until) = switch.until {
        text += &format!(" until {until:?}");
    }
    text
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
        moved.push(Moved {
            name,
            starter: *value,
            least: sorted[0],
            middle: sorted[sorted.len() / 2],
            most: sorted[sorted.len() - 1],
            up,
            down,
        });
    }
    let seeds = measured.len();
    let all_way: Vec<_> = moved.iter().filter(|m| m.every_seed(seeds)).collect();
    // G4 asks for every seed, so a world that died out fails it.
    let died_out = runs.len() - seeds;
    let of_all = if died_out == 0 {
        String::new()
    } else {
        format!(
            " alive at the end, but {died_out} of {} died out",
            runs.len()
        )
    };
    println!(
        "\n{} of {} numbers moved by at least a tenth the same way in all {seeds} seeds{of_all}.",
        all_way.len(),
        moved.len()
    );
    println!(
        "\n{:<52} {:>10} {:>6} {:>6} {:>6} {:>4} {:>4}",
        "Number", "Starter", "Least", "Middle", "Most", "Up", "Down"
    );
    let mut by_size = moved.clone();
    by_size.sort_by(|a, b| distance(b.middle).total_cmp(&distance(a.middle)));
    for m in by_size.iter().take(25) {
        let mark = if m.every_seed(seeds) { "*" } else { " " };
        println!(
            "{mark}{:<51} {:>10.4} {:>6.2} {:>6.2} {:>6.2} {:>4} {:>4}",
            m.name, m.starter, m.least, m.middle, m.most, m.up, m.down
        );
    }
    println!(
        "\n* moved the same way in every seed. Least, middle and most are seeds' ratios to the starter."
    );
}

/// How far one gene number moved from the starter's, across seeds: its
/// least, middle and most ratio to the starter's value, and in how many
/// seeds it moved a tenth up and a tenth down.
#[derive(Debug, Clone)]
struct Moved<'a> {
    name: &'a str,
    starter: f32,
    least: f32,
    middle: f32,
    most: f32,
    up: usize,
    down: usize,
}

impl Moved<'_> {
    /// Whether it moved a tenth the same way in every one of `seeds`.
    fn every_seed(&self, seeds: usize) -> bool {
        self.up == seeds || self.down == seeds
    }
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
    let mut lates = Vec::new();
    for (seed, run) in runs {
        let mut late = run.late_lives.clone();
        late.sort_unstable();
        let late = late.get(late.len().saturating_sub(1) / 2).copied();
        lates.extend(late);
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
    lates.sort_unstable();
    if let (Some(&shortest), Some(&longest)) = (lates.first(), lates.last()) {
        println!(
            "Second half's median life: {:.1} to {:.1} hours a seed.",
            hours(shortest),
            hours(longest)
        );
    }
}

/// The hidden flaws the sprites carried (M2 design §12), and how long young
/// from lone eggs lived against young from pairs.
fn flaws(runs: &[(u64, Run)]) {
    println!(
        "\nHidden flaws: genes with one flawed copy (carried) and with two (broken), on average \
         among the hatched sprites alive halfway and at the end; then the median life, in hours, \
         of young from lone eggs and from pairs, of those old age killed, and how many from lone \
         eggs it killed."
    );
    println!(
        "\n{:>5} {:>8} {:>7} {:>8} {:>7} {:>6} {:>6} {:>5}",
        "Seed", "Carried", "Broken", "Carried", "Broken", "Lone", "Pair", "Lone"
    );
    println!("{:>5} {:>16} {:>16}", "", "halfway", "at the end");
    let hours = |ticks: u64| ticks as f64 / HOUR;
    let median = |ages: &[u64]| {
        let mut ages = ages.to_vec();
        ages.sort_unstable();
        ages.get(ages.len().saturating_sub(1) / 2)
            .map_or("-".to_string(), |&age| format!("{:.1}", hours(age)))
    };
    for (seed, run) in runs {
        let load = |at: Option<FlawLoad>| {
            at.map_or(("-".to_string(), "-".to_string()), |load| {
                (
                    format!("{:.1}", load.carried),
                    format!("{:.2}", load.broken),
                )
            })
        };
        let ((carried_half, broken_half), (carried, broken)) =
            (load(run.flaws_halfway), load(run.flaws_at_end));
        println!(
            "{seed:>5} {carried_half:>8} {broken_half:>7} {carried:>8} {broken:>7} {:>6} {:>6} {:>5}",
            median(&run.lone_lives),
            median(&run.pair_lives),
            run.lone_lives.len()
        );
    }
}

/// The hidden flaws carried and the genes broken, each on average, among
/// the hatched sprites alive at a moment (M2 design §12).
#[derive(Debug, Clone, Copy)]
struct FlawLoad {
    carried: f32,
    broken: f32,
}

/// The flaws among the hatched sprites alive in `world`; `None` if there
/// are none.
fn flaw_load(world: &World) -> Option<FlawLoad> {
    let (mut carried, mut broken, mut sprites) = (0, 0, 0);
    for sprite in world
        .sprites()
        .filter(|sprite| !sprite.parents().is_empty())
    {
        sprites += 1;
        for flawed in sprite.flaws() {
            carried += usize::from(flawed == 1);
            broken += usize::from(flawed == 2);
        }
    }
    (sprites > 0).then(|| {
        let each = |count: usize| count as f32 / sprites as f32;
        FlawLoad {
            carried: each(carried),
            broken: each(broken),
        }
    })
}

/// Runs the default world from `seed` for `ticks` ticks.
fn run(
    data: &DataPack,
    config: &WorldConfig,
    starter: &[(String, f32)],
    starter_genes: &BTreeMap<String, usize>,
    seed: u64,
    ticks: u64,
) -> Run {
    let mut world = World::new(config.clone(), data.clone(), seed);
    let mut run = Run::default();
    let mut generation: BTreeMap<EntityId, u32> =
        world.sprites().map(|sprite| (sprite.id(), 0)).collect();
    let mut hatchlings = Vec::new();
    let mut lone_eggs = BTreeSet::new();
    for _ in 0..ticks {
        for event in world.step() {
            match event.kind {
                EventKind::Laid { egg, by, with } => {
                    if with.is_none() {
                        lone_eggs.insert(egg);
                    }
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
                            if lone_eggs.contains(&id) {
                                run.lone_lives.push(age);
                            } else {
                                run.pair_lives.push(age);
                            }
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
        if world.tick() == ticks / 2 {
            run.flaws_halfway = flaw_load(&world);
        }
        // Eggs count, as the world's limit counts them (M2 design §10.4).
        let count = world.sprites().count() + world.eggs().count();
        run.most = run.most.max(count);
        if count == 0 && run.died_out.is_none() {
            run.died_out = Some(world.tick());
        }
    }
    run.eggs = world.eggs().count();
    run.alive = world.sprites().count();
    run.numbers = middles(&world, starter);
    run.flaws_at_end = flaw_load(&world);
    run.genes = gene_pool(&world, starter_genes);
    run
}

/// The genes of the hatched sprites alive in `world`, against the
/// starter's (M2 design §13); `None` if none is alive.
fn gene_pool(world: &World, starter: &BTreeMap<String, usize>) -> Option<GenePool> {
    let mut pool = GenePool::default();
    let mut lengths = Vec::new();
    for sprite in world
        .sprites()
        .filter(|sprite| !sprite.parents().is_empty())
    {
        let (genes, how): (Vec<GeneView>, Vec<Expression>) = sprite.genes().into_iter().unzip();
        lengths.push(genes.len());
        let switches = sprite.switches();
        let has = counted(&genes, &switches);
        for (gene, &count) in starter {
            let its = has.get(gene).copied().unwrap_or(0);
            pool.lost += count.saturating_sub(its);
            pool.doubled += its.saturating_sub(count);
            if its < count {
                *pool.lacking.entry(gene.clone()).or_default() += 1;
            }
        }
        // A new gene works in a sprite if a copy of it is expressed now, at
        // its life stage: a spare that an earlier gene overrides doesn't.
        let working = counted_where(&genes, &switches, |index| {
            how[index] == Expression::Expressed
        });
        for gene in has.keys().filter(|gene| !starter.contains_key(*gene)) {
            let carried = pool.new.entry(gene.clone()).or_default();
            carried.0 += 1;
            carried.1 += usize::from(working.contains_key(gene));
        }
    }
    lengths.sort_unstable();
    pool.sprites = lengths.len();
    pool.lengths = (
        *lengths.first()?,
        lengths[(lengths.len() - 1) / 2],
        *lengths.last()?,
    );
    Some(pool)
}

/// The genes of the hatched sprites alive at a moment, against the
/// starter's (M2 design §13).
#[derive(Debug, Default)]
struct GenePool {
    /// The sprites counted.
    sprites: usize,
    /// Each gene the starter lacks, by what it does and when, with how
    /// many of them carry it, and in how many it works.
    new: BTreeMap<String, (usize, usize)>,
    /// Each of the starter's genes, by what it does and when, with how
    /// many of them have fewer copies of it than the starter: lost, or
    /// changed in its place.
    lacking: BTreeMap<String, usize>,
    /// The starter's genes they lack, and their copies of its genes beyond
    /// the starter's, all told.
    lost: usize,
    doubled: usize,
    /// Their genomes' shortest, median and longest length.
    lengths: (usize, usize, usize),
}

impl GenePool {
    /// The genes the starter lacks carried by at least a tenth of them,
    /// most widely carried first, each with the shares that carry it and
    /// in which it works.
    fn taken_hold(&self) -> Vec<(&str, f32, f32)> {
        let share = |count: usize| count as f32 / self.sprites as f32;
        let mut held: Vec<(&str, f32, f32)> = self
            .new
            .iter()
            .map(|(gene, &(carriers, working))| (gene.as_str(), share(carriers), share(working)))
            .filter(|&(_, carried, _)| carried >= 0.1)
            .collect();
        held.sort_by(|a, b| b.1.total_cmp(&a.1));
        held
    }
}

/// The genes the starter lacks that the living carry, and the genes they
/// lost and doubled (M2 design §13); the bar is a gene the starter lacks
/// carried by at least a tenth of the living in at least 5 of 10 seeds.
/// Whether the gene works in them is reported beside it: a spare copy
/// changed to work only at another life stage is carried but may do
/// nothing yet. Then the starter's genes a tenth or more of the living
/// lack.
fn copy_and_change(runs: &[(u64, Run)]) {
    println!(
        "\nCopy and change: among the hatched sprites alive at the end, the genes the starter \
         lacks (by what they do and when) carried by at least a tenth, and of those how many \
         work in at least a tenth; the most widely carried, with the shares that carry it and \
         in which it works; starter genes lost and extra copies, on average; and genome lengths."
    );
    println!(
        "\n{:>5} {:>7} {:>6} {:>7} {:>5} {:>5} {:>5} {:>6} {:>5}  Most widely carried new gene",
        "Seed", "Sprites", "New10%", "Works10", "Lost", "Extra", "Short", "Median", "Long",
    );
    let mut took_hold = 0;
    let mut working = 0;
    let mut spread: BTreeMap<&str, (usize, f32, f32)> = BTreeMap::new();
    let mut lacked: BTreeMap<&str, (usize, f32)> = BTreeMap::new();
    for (seed, run) in runs {
        let Some(pool) = &run.genes else {
            println!("{seed:>5} {:>7}", 0);
            continue;
        };
        let held = pool.taken_hold();
        let works = held.iter().filter(|&&(_, _, works)| works >= 0.1).count();
        took_hold += usize::from(!held.is_empty());
        working += usize::from(works > 0);
        for &(gene, carried, works) in &held {
            let seen = spread.entry(gene).or_insert((0, 0.0, 0.0));
            *seen = (seen.0 + 1, seen.1.max(carried), seen.2.max(works));
        }
        let each = |count: usize| count as f32 / pool.sprites as f32;
        for (gene, &lacking) in &pool.lacking {
            if each(lacking) >= 0.1 {
                let seen = lacked.entry(gene).or_insert((0, 0.0));
                *seen = (seen.0 + 1, seen.1.max(each(lacking)));
            }
        }
        let widest = pool
            .new
            .iter()
            .max_by_key(|&(_, &(carriers, _))| carriers)
            .map_or("-".to_string(), |(gene, &(carriers, works))| {
                format!(
                    "{gene} ({:.0}%, works in {:.0}%)",
                    100.0 * each(carriers),
                    100.0 * each(works)
                )
            });
        let (short, median, long) = pool.lengths;
        println!(
            "{seed:>5} {:>7} {:>6} {works:>7} {:>5.2} {:>5.2} {short:>5} {median:>6} {long:>5}  {widest}",
            pool.sprites,
            held.len(),
            each(pool.lost),
            each(pool.doubled)
        );
    }
    println!(
        "\nA gene the starter lacks was carried by a tenth or more of the living in {took_hold} of \
         {} seeds, and worked in a tenth or more in {working}.",
        runs.len()
    );
    if !spread.is_empty() {
        println!(
            "\n{:<60} {:>5} {:>8} {:>8}",
            "New gene carried by a tenth or more", "Seeds", "Widest", "Works"
        );
        let mut spread: Vec<_> = spread.into_iter().collect();
        spread.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.total_cmp(&a.1.1)));
        for (gene, (seeds, widest, works)) in spread {
            println!(
                "{gene:<60} {seeds:>5} {:>7.0}% {:>7.0}%",
                100.0 * widest,
                100.0 * works
            );
        }
    }
    if !lacked.is_empty() {
        println!(
            "\n{:<60} {:>5} {:>8}",
            "Starter gene lacked by a tenth or more (lost or changed)", "Seeds", "Widest"
        );
        let mut lacked: Vec<_> = lacked.into_iter().collect();
        lacked.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.total_cmp(&a.1.1)));
        for (gene, (seeds, widest)) in lacked {
            println!("{gene:<60} {seeds:>5} {:>7.0}%", 100.0 * widest);
        }
    }
}

/// Each of the starter's numbers' middle value among the sprites alive in
/// `world`, eggs aside; empty if none is alive, and not a number where none
/// has it (M2 design §13). By the end of a long run, all were hatched.
fn middles(world: &World, starter: &[(String, f32)]) -> Vec<f32> {
    middle_values(world, starter)
        .map(|middles| {
            middles
                .into_iter()
                .map(|middle| middle.unwrap_or(f32::NAN))
                .collect()
        })
        .unwrap_or_default()
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
