//! The harsher world's long run (M2 design §19.6): runs the default world,
//! left alone, from the given seeds for 220 hours of play each, and
//! reports every 10 hours how its sprites stand, then each seed against
//! the bars and the bars judged across seeds, lone newborns given the
//! genes of the sprites alive at the end, the race against the starter's
//! genes and evolution (G4), and last whether every bar was met.
//!
//! Each row: the hour; sprites, the young among them, and eggs; berry
//! bushes; and since the last row, the hatched young that grew up and that
//! died young, the grown-up deaths and how many of them were adults dying
//! of hunger or thirst, the share of bites tried by a young sprite or an
//! adult too full to eat and the same share of old sprites' bites, the
//! share of adults' time on the move and of the young's; then how many of
//! the adults alive have their wandering switched off, of how many adults
//! (#318); and how many of the twelve needs genes (M2 design §16.6) more
//! than half of the hatched sprites alive lack. A bite tried is an Eat
//! that got to its food and made its try (design §5.5), a Too full one
//! too; an Eat that ended on its way isn't one. An adult's wandering is
//! switched off when its `always → Wander` instinct, as its brain takes it,
//! the expressed one's weight times its gene's health (M2 design §9.4),
//! pulls less than its act threshold (§19.2): it never wanders on its own.
//! It comes from a copying error that weakened the gene, two hidden flaws
//! that broke it, or copy and change losing it (§10.9, #318).
//!
//! The bar for each seed that lasts, since Slice 29b (#331):
//!
//! - **Adults are on the move at least a quarter of the time** over the
//!   last 50 hours (#237, #318): the share of their ticks on another tile
//!   than the tick before.
//!
//! Held over to #332 by the owner's call (2026-10-24), each reported for
//! each seed but not must-pass, beside how many seeds missed it in Slice
//! 29's run on the owner's PC (`SLICE_29_MISSES`). If one misses in more
//! seeds than that, it's clearly worse, and goes back to the owner before
//! the PR merges:
//!
//! - **Adults dying of hunger or thirst are under 1 in 20 of grown-up
//!   deaths,** over the run and over its last 50 hours, in each seed that
//!   lasts (#226). Old sprites' are reported apart.
//! - **At most 1 in 10 bites tried by young sprites and adults are by a
//!   sprite too full to eat,** over the run and over its last 50 hours
//!   (#224). Old sprites' bites are reported apart (#225 round 1).
//!
//! The bars across seeds, each met when at most one seed in ten misses it,
//! so in at least 9 seeds of 10:
//!
//! - **The world lasts** (#222): at the end it has sprites or eggs; it
//!   never had fewer than 10 sprites, eggs not counted, at any count of its
//!   population record; and its sprites over the last 50 hours average at
//!   least half of what they did over hours 50–100.
//! - **The bushes hold** (#224): over the last 50 hours they average at
//!   least half of what they did over hours 50–100.
//!
//! The 50-hour averages need a run of 150 hours or more, and a shorter one
//! says they weren't judged. A seed that misses a bar across seeds has
//! failed it only if the bar isn't met.
//!
//! **The race** against the starter's genes (#226 Q7, #237 rules 1 and 8),
//! run every time: for each seed that didn't die out, with hatched sprites
//! alive at the end, the same fresh world as the viability run's
//! (`scenarios/viability.ron`), run as it is for 50,000 ticks twice: once
//! as it generates, its 30 first sprites wild, from the starter's genes;
//! once with them taking after the hatched sprites alive at the end, in
//! turn, their genomes and hidden flaws, each on the tile the starter's
//! sprite in its place has (`World::new_like`). A seed is won when the
//! evolved side's hatched young grow up more often, counted as the viability
//! run counts them: those still growing at the end aren't. The race is won
//! when the evolved side wins at least 8 of the seeds that lasted and were
//! raced; one with no young counted on a side isn't judged. Lost, the
//! species has **fallen behind** the starter's, and each seed raced that
//! wasn't won has failed it, as with the bars across seeds.
//!
//! Then evolution (M2 design §12.7, G4), judged on this run by #226: at
//! least 3 gene numbers whose middle value among the living at the end
//! moved by a tenth the same way in every seed that lasted. A seed that
//! didn't last, or had no sprite alive at the end, drops out of it. Each
//! number is named by what its gene does and when it works, so the young's
//! and grown sprites' Wander weights are two numbers. It's G4's first test,
//! so it fails no seed: if it isn't met, the race is its second (#226), and
//! G4 is met if the race is won.
//!
//! Reported with no bar: the young hatched that grew up, whose bar is the
//! viability run's (#226); the young's time on the move (#237 rule 5);
//! the adults with their wandering switched off, hour by hour (#318); what
//! killed the hatched young, and how those that saw a parent for most of
//! their childhood fared against those that didn't; the share of the
//! hatched sprites alive at the end lacking each needs gene, and each other
//! starter gene that a tenth or more of some seed's living lack (#237 rule
//! 4), a gene with a life-stage switch listed by it, so the young's and
//! grown sprites' Wander genes show apart; and lone newborns. For each seed
//! that didn't die out, with hatched sprites alive at the end, the
//! unattended run (`scenarios/unattended.ron`) on the same seed, its
//! newborns taking after those sprites, until each grows up or dies, set
//! beside the unattended band of 10%–30%; whether the species has fallen
//! behind is the race's to say.
//!
//! Careful copying would switch on for a seed that failed a bar, the race
//! among them when it's lost overall, while more than half its living lack
//! a needs gene (#237 rule 4, M2 design §19.6).
//! A held-over bar doesn't switch it on (the owner, 2026-10-24): the seeds
//! a held-over bar would have switched it on for are named apart, for
//! #332. A seed that died out is judged on its living at the last hour any
//! hatched sprite was alive.
//!
//! `cargo run --release -p terra-sim --example childhood`
//!
//! `--seeds` runs the seeds listed, `5,9`, or a range, `1-10` (1-10 by
//! default), `--hours N` each for N hours of play at 1× (220 by default),
//! `--every N` reports every N hours (10 by default), and `--jobs N` runs N
//! worlds at a time (4 by default). Each world is its own, so the results
//! don't depend on it. Each seed's table is printed as it finishes, and the
//! bars, the race, evolution and the verdict at the end.

#[path = "common/args.rs"]
mod args;
#[path = "common/evolution.rs"]
mod evolution;
#[path = "common/genes.rs"]
mod genes;
#[path = "common/lasting.rs"]
mod lasting;
#[path = "common/parallel.rs"]
mod parallel;

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use lasting::{Census, HOUR, LATE_HOURS};
use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, Expression, GeneView, LifeStage, Outcome, Pos,
    SpriteView, Verb, World, WorldConfig, cause_name,
};

/// The longest the lone newborns run, in ticks: a sprite whose genes take
/// longer than this to grow up is counted as neither.
const NEWBORN_TICKS: u64 = 20_000;

/// Each side of the race runs as long as the viability run
/// (`scenarios/viability.ron`), in ticks (#226 Q7).
const RACE_TICKS: u64 = 50_000;

/// The seeds the evolved side must win, of those that lasted and were
/// raced, for the race to be won (#226 Q7): by chance alone, about 1 time
/// in 18 with ten seeds.
const RACE_WINS: usize = 8;

/// The bar a seed fails when the race is lost overall (#237 rule 1).
const RACE: &str = "the race";

/// The twelve needs genes (M2 design §16.6, §19.6), by what each does, as
/// the starter genome has them: hunger's, thirst's and tiredness's.
const NEEDS: [&str; 12] = [
    "low energy Level → hunger",
    "ate Level → hunger",
    "fullness Level → hunger",
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

/// The share of a seed's living lacking a starter gene from which the gene
/// table lists it, as `generations` does; the needs genes are always
/// listed (#237 rule 4).
const LISTED_FROM: f64 = 0.1;

/// The held-over bar on adults dying of hunger or thirst (M2 design §19.6).
const HUNGER_AND_THIRST: &str = "adults' hunger and thirst";

/// The held-over bar on bites tried too full (M2 design §19.6).
const TOO_FULL: &str = "too full";

/// The held-over bars (the owner, 2026-10-24), each with how many seeds
/// missed it in Slice 29's run on the owner's PC, at 9bc50de: more than
/// that is clearly worse (M2 design §19.6).
const SLICE_29_MISSES: [(&str, usize); 2] = [(HUNGER_AND_THIRST, 2), (TOO_FULL, 1)];

/// The roaming bar, for each seed that lasts (M2 design §10.9, §19.6).
const ON_THE_MOVE: &str = "on the move";

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
    /// Bites tried by young sprites and adults, and those by a sprite too
    /// full to eat (#224).
    bites: u64,
    too_full: u64,
    /// Old sprites' bites tried, and those too full, reported apart from
    /// the bar (#225 round 1).
    old_bites: u64,
    old_too_full: u64,
    /// Adults' ticks, and those on the move: on another tile than the tick
    /// before. Then the young's.
    adult_ticks: u64,
    moving: u64,
    young_ticks: u64,
    young_moving: u64,
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
        self.old_bites += other.old_bites;
        self.old_too_full += other.old_too_full;
        self.adult_ticks += other.adult_ticks;
        self.moving += other.moving;
        self.young_ticks += other.young_ticks;
        self.young_moving += other.young_moving;
    }
}

/// How many young grew up, and how many died young.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Raised {
    grew_up: u64,
    died_young: u64,
}

impl Raised {
    /// The young counted: those that grew up or died young.
    fn counted(&self) -> u64 {
        self.grew_up + self.died_young
    }
}

/// One seed's race (#226 Q7): its fresh world's hatched young, run as the
/// viability run, from the starter's genes and from the end's.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Race {
    starter: Raised,
    evolved: Raised,
}

impl Race {
    /// Whether the evolved side's young grew up more often than the
    /// starter's, a share against a share; a tie isn't. None if either
    /// side had no young to count.
    fn won(&self) -> Option<bool> {
        let (starter, evolved) = (self.starter, self.evolved);
        if starter.counted() == 0 || evolved.counted() == 0 {
            return None;
        }
        Some(evolved.grew_up * starter.counted() > starter.grew_up * evolved.counted())
    }
}

/// What a run follows from event to event: each sprite's life stage, the
/// hatched young growing up, and what became of them.
#[derive(Debug, Default)]
struct Tally {
    stages: BTreeMap<EntityId, LifeStage>,
    /// The hatched young growing up, each with its ticks so far and those
    /// with a parent in sight.
    hatched: BTreeMap<EntityId, (u64, u64)>,
    /// What killed the hatched young, by cause.
    died_young_of: BTreeMap<DeathCause, u64>,
    /// The hatched young that grew up and that died young: those that saw
    /// a parent for at least half their childhood, then the rest.
    by_parent: [Raised; 2],
}

impl Tally {
    /// Starts with the sprites `world` has.
    fn new(world: &World) -> Tally {
        Tally {
            stages: world
                .sprites()
                .map(|sprite| (sprite.id(), sprite.life_stage()))
                .collect(),
            ..Tally::default()
        }
    }

    /// Counts `event` into `now`.
    fn count(&mut self, event: &EventKind, now: &mut Counts) {
        match *event {
            EventKind::Hatched { id, .. } => {
                self.stages.insert(id, LifeStage::Young);
                self.hatched.insert(id, (0, 0));
            }
            EventKind::Spawned { id, .. } => {
                self.stages.insert(id, LifeStage::Young);
            }
            EventKind::GrewUp { id } => {
                self.stages.insert(id, LifeStage::Adult);
                if let Some(sight) = self.hatched.remove(&id) {
                    now.grew_up += 1;
                    self.by_parent[apart(sight)].grew_up += 1;
                }
            }
            EventKind::GrewOld { id } => {
                self.stages.insert(id, LifeStage::Old);
            }
            EventKind::Died { id, cause, .. } => {
                let stage = self.stages.remove(&id).unwrap_or_default();
                let need = matches!(cause, DeathCause::Starvation | DeathCause::Dehydration);
                match stage {
                    LifeStage::Young => {
                        if let Some(sight) = self.hatched.remove(&id) {
                            now.died_young += 1;
                            self.by_parent[apart(sight)].died_young += 1;
                            *self.died_young_of.entry(cause).or_insert(0) += 1;
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
            // A bite tried: an Eat that got to its food and made its try
            // (design §5.5). Only the try finds the gut full (M2 design
            // §19.4), so an Eat that ended on its way is none.
            EventKind::ActionEnded {
                id,
                verb: Verb::Eat,
                outcome,
                action,
            } if action.attempted => {
                let full = u64::from(outcome == Outcome::TooFull);
                if self.stages.get(&id) == Some(&LifeStage::Old) {
                    now.old_bites += 1;
                    now.old_too_full += full;
                } else {
                    now.bites += 1;
                    now.too_full += full;
                }
            }
            _ => {}
        }
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
    /// The adults alive with their wandering switched off, and the adults
    /// alive (#318).
    wander_off: (usize, usize),
    /// Needs genes more than half the hatched sprites alive lack.
    needs_lacked: usize,
}

/// What one seed's world came to.
#[derive(Debug, Default)]
struct Run {
    seed: u64,
    rows: Vec<Row>,
    whole: Counts,
    late: Counts,
    /// What killed the hatched young, by cause.
    died_young_of: BTreeMap<String, u64>,
    /// The hatched young that grew up and that died young: those that saw
    /// a parent for at least half their childhood, then the rest.
    by_parent: [Raised; 2],
    /// The bushes at the end of each hour.
    bushes: Vec<usize>,
    /// The sprites: the fewest at a count of the population record, and
    /// at the end of each hour (#222).
    census: Census,
    /// The tick the world died out on, if it did.
    died_out: Option<u64>,
    /// Each of the starter's numbers' middle value among the sprites alive
    /// at the end, in the starter's order; empty if none is (G4).
    numbers: Vec<f32>,
    /// Each starter gene, by what it does and when, and the share of the
    /// hatched sprites alive at the end of hour `lacking_hour` that lack
    /// it: the last hour any was alive, so the end for a world that didn't
    /// die out and still had them, and the last before it died out for one
    /// that did. Empty if none ever was.
    lacking: BTreeMap<String, f64>,
    lacking_hour: u64,
    /// The lone newborns: how many grew up and how many died, if the
    /// world didn't die out and had hatched sprites alive at the end.
    newborns: Option<(u64, u64)>,
    /// The race, if the world didn't die out and had hatched sprites alive
    /// at the end.
    race: Option<Race>,
}

impl Run {
    /// Keeps `lacking`, measured at the end of `hour`, unless no hatched
    /// sprite was alive then, so a world that dies out keeps its living's
    /// genes from the last hour before it did (#237 rule 4).
    fn measured(&mut self, lacking: BTreeMap<String, f64>, hour: u64) {
        if !lacking.is_empty() {
            self.lacking = lacking;
            self.lacking_hour = hour;
        }
    }
}

fn main() -> ExitCode {
    let usage = "childhood [--seeds A-B | a,b,c] [--hours N] [--every N] [--jobs N]";
    let Some(options) = args::parse(usage, parse) else {
        return ExitCode::FAILURE;
    };
    args::warn_if_debug(", or it will take a long time");
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let starter = Starter {
        genes: starter_genes(&data),
        numbers: evolution::starter_numbers(&data),
    };
    let seeds: Vec<String> = options.seeds.iter().map(u64::to_string).collect();
    println!(
        "The harsher world's long run: the default world, left alone, seeds {}, {} hours each \
         ({} ticks).",
        seeds.join(", "),
        options.hours,
        options.hours * HOUR
    );
    let runs = in_parallel(&data, &starter, &options);
    bars(&runs, &starter.numbers, &options);
    ExitCode::SUCCESS
}

/// The seeds G4 is judged over, on this run (#226): each that lasted
/// (#222), with sprites alive at the end, and its numbers; then the seeds
/// that drop out, as they didn't last or had no sprite alive to measure.
fn evolution_seeds<'a>(
    runs: &'a [Run],
    not_lasted: &[(u64, Vec<&str>)],
) -> (Vec<(u64, &'a [f32])>, Vec<u64>) {
    let (kept, dropped): (Vec<&Run>, Vec<&Run>) = runs.iter().partition(|run| {
        !not_lasted.iter().any(|(seed, _)| *seed == run.seed) && !run.numbers.is_empty()
    });
    (
        kept.iter()
            .map(|run| (run.seed, &run.numbers[..]))
            .collect(),
        dropped.iter().map(|run| run.seed).collect(),
    )
}

/// The starter genome as the run sets the living against it: its genes by
/// what each does and when, with how many it has of each, and its numbers,
/// named the same way (G4).
struct Starter {
    genes: BTreeMap<String, usize>,
    numbers: Vec<(String, f32)>,
}

/// Runs the seeds' worlds, `options.jobs` at a time, printing each one's
/// table as it finishes, and returns them in seed order.
fn in_parallel(data: &DataPack, starter: &Starter, options: &Options) -> Vec<Run> {
    let mut done = parallel::in_parallel(options.seeds.clone(), options.jobs, |seed| {
        let run = run(data, starter, seed, options);
        // Its table printed whole, before another run's.
        let _whole = std::io::stdout().lock();
        print_run(&run);
        run
    });
    done.sort_by_key(|run| run.seed);
    done
}

/// Runs the default world from `seed` for `options.hours`, then the lone
/// newborns and the race.
fn run(data: &DataPack, starter: &Starter, seed: u64, options: &Options) -> Run {
    let config = WorldConfig::builtin(data);
    let mut world = World::new(config.clone(), data.clone(), seed);
    let mut tally = Tally::new(&world);
    let mut last: BTreeMap<EntityId, Pos> = BTreeMap::new();
    let late_from = options.hours.saturating_sub(LATE_HOURS) * HOUR;
    let mut out = Run {
        seed,
        census: Census::new(&world),
        ..Run::default()
    };
    let mut row = Counts::default();
    for _ in 0..options.hours * HOUR {
        let tick = world.tick();
        let every = world.population_every();
        let mut now = Counts::default();
        for event in world.step() {
            tally.count(&event.kind, &mut now);
        }
        out.census.count(&world, every);
        let mut here = BTreeMap::new();
        for sprite in world.sprites() {
            let moved = last.get(&sprite.id()).is_some_and(|&at| at != sprite.pos());
            match sprite.life_stage() {
                LifeStage::Young => {
                    now.young_ticks += 1;
                    now.young_moving += u64::from(moved);
                }
                LifeStage::Adult => {
                    now.adult_ticks += 1;
                    now.moving += u64::from(moved);
                }
                LifeStage::Old => {}
            }
            here.insert(sprite.id(), sprite.pos());
            if let Some(sight) = tally.hatched.get_mut(&sprite.id()) {
                let mut parents = sprite.parents().iter().filter_map(|&p| world.sprite(p));
                let seen = parents.any(|parent| sprite.sees(parent.pos()));
                *sight = (sight.0 + 1, sight.1 + u64::from(seen));
            }
        }
        last = here;
        row.add(&now);
        out.whole.add(&now);
        if tick >= late_from {
            out.late.add(&now);
        }
        let ticks = tick + 1;
        if ticks.is_multiple_of(HOUR) {
            let hour = ticks / HOUR;
            let bushes = bush_count(&world);
            out.bushes.push(bushes);
            // Kept each hour, so a world that dies out keeps its living's
            // genes from the last hour before it did.
            let lacking = lacking(&world, &starter.genes);
            let needs_lacked = needs_lacked(&lacking).len();
            out.measured(lacking, hour);
            if hour.is_multiple_of(options.every) {
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
                    wander_off: wander_off(&world),
                    needs_lacked,
                });
            }
        }
        if world.sprites().count() + world.eggs().count() == 0 {
            out.died_out = Some(ticks);
            break;
        }
    }
    out.died_young_of = (tally.died_young_of.into_iter())
        .map(|(cause, n)| (cause_name(cause, data), n))
        .collect();
    out.by_parent = tally.by_parent;
    out.numbers = evolution::middles(&world, &starter.numbers);
    if out.died_out.is_none() {
        out.newborns =
            World::unattended_like(config.clone(), data.clone(), seed, &world).map(newborns);
        out.race =
            World::new_like(config.clone(), data.clone(), seed, &world).map(|evolved| Race {
                starter: raised(World::new(config, data.clone(), seed)),
                evolved: raised(evolved),
            });
    }
    out
}

/// Runs `world` for the race's ticks: how many of its hatched young grew
/// up, and how many died young (#226 Q7).
fn raised(mut world: World) -> Raised {
    let mut tally = Tally::new(&world);
    let mut counts = Counts::default();
    for _ in 0..RACE_TICKS {
        for event in world.step() {
            tally.count(&event.kind, &mut counts);
        }
    }
    Raised {
        grew_up: counts.grew_up,
        died_young: counts.died_young,
    }
}

/// The adults alive in `world` with their wandering switched off, and the
/// adults alive (M2 design §19.6, #318).
fn wander_off(world: &World) -> (usize, usize) {
    let adults: Vec<SpriteView> = world
        .sprites()
        .filter(|sprite| sprite.life_stage() == LifeStage::Adult)
        .collect();
    let off = adults.iter().filter(|sprite| switched_off(sprite)).count();
    (off, adults.len())
}

/// Whether `sprite`'s wandering is switched off (M2 design §19.6, #318):
/// its `always → Wander` instinct, as its brain takes it, pulls less than
/// its act threshold (§19.2), so it never wanders on its own. The brain
/// takes the expressed instinct for those inputs and verb, the one gene of
/// them that expression left expressed (design §4.3), its weight times its
/// gene's health (M2 design §9.4).
fn switched_off(sprite: &SpriteView) -> bool {
    let health = sprite.gene_health();
    let wander = sprite
        .genes()
        .into_iter()
        .enumerate()
        .find_map(|(index, (gene, expression))| match gene {
            GeneView::Instinct {
                inputs,
                verb: Verb::Wander,
                weight,
            } if expression == Expression::Expressed && inputs == [("always", false)] => {
                Some(weight * health.get(index).copied().unwrap_or(1.0))
            }
            _ => None,
        })
        .unwrap_or(0.0);
    wander < sprite.act_threshold()
}

/// Which of `Run::by_parent` a young sprite counts in, by its ticks and
/// those with a parent in sight: 0 if it saw one for at least half of them.
fn apart((ticks, seen): (u64, u64)) -> usize {
    usize::from(2 * seen < ticks)
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

/// The starter's genes, by what each does and when it works (M2 design
/// §13), with how many it has of each: every needs gene among them, or
/// `NEEDS` has gone out of step with it.
fn starter_genes(data: &DataPack) -> BTreeMap<String, usize> {
    let genome = data.starter();
    let counts = genes::counted(&genome.genes(data), &genome.switches());
    for need in NEEDS {
        assert!(
            counts.contains_key(need),
            "the starter genome has no {need} gene"
        );
    }
    counts
}

/// Each of the `starter`'s genes, and the share of the hatched sprites
/// alive in `world` with fewer copies of it than the starter: lost, or
/// changed into something else (M2 design §13). Empty if none is alive.
fn lacking(world: &World, starter: &BTreeMap<String, usize>) -> BTreeMap<String, f64> {
    let mut lack = vec![0u64; starter.len()];
    let mut sprites = 0u64;
    for sprite in world
        .sprites()
        .filter(|sprite| !sprite.parents().is_empty())
    {
        sprites += 1;
        let (genes, _): (Vec<GeneView>, Vec<_>) = sprite.genes().into_iter().unzip();
        let has = genes::counted(&genes, &sprite.switches());
        for (lacked, (gene, &count)) in lack.iter_mut().zip(starter) {
            *lacked += u64::from(has.get(gene).copied().unwrap_or(0) < count);
        }
    }
    if sprites == 0 {
        return BTreeMap::new();
    }
    starter
        .keys()
        .zip(lack)
        .map(|(gene, lacked)| (gene.clone(), lacked as f64 / sprites as f64))
        .collect()
}

/// The needs genes more than half of the living lack, by `lacking`.
fn needs_lacked(lacking: &BTreeMap<String, f64>) -> Vec<&'static str> {
    NEEDS
        .into_iter()
        .filter(|need| lacking.get(*need).is_some_and(|&share| share > 0.5))
        .collect()
}

/// `part` of `whole` as a percentage, or a dash if `whole` is none.
fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        "-".into()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}

/// Yes, no, or a dash if it wasn't judged.
fn yes_no(judged: Option<bool>) -> &'static str {
    judged.map_or("-", |yes| if yes { "yes" } else { "no" })
}

fn print_run(run: &Run) {
    let ended = match run.died_out {
        Some(tick) => format!(", died out at hour {:.1}", tick as f64 / HOUR as f64),
        None => String::new(),
    };
    println!("\nSeed {}{ended}", run.seed);
    println!(
        "{:>5} {:>7} {:>6} {:>5} {:>7} {:>7} {:>10} {:>6} {:>8} {:>8} {:>8} {:>7} {:>8} {:>10} \
         {:>6}",
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
        "OldFull",
        "Moving",
        "YoungMv",
        "WanderOff",
        "Needs"
    );
    for row in &run.rows {
        let c = row.counts;
        let (off, adults) = row.wander_off;
        println!(
            "{:>5} {:>7} {:>6} {:>5} {:>7} {:>7} {:>10} {:>6} {:>8} {:>8} {:>8} {:>7} {:>8} \
             {:>10} {:>6}",
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
            percent(c.old_too_full, c.old_bites),
            percent(c.moving, c.adult_ticks),
            percent(c.young_moving, c.young_ticks),
            format!("{off}/{adults}"),
            row.needs_lacked
        );
    }
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

/// The long run's verdicts on its seeds (M2 design §19.6).
#[derive(Debug)]
struct Verdicts {
    /// The seeds that didn't last, each with why (#222).
    not_lasted: Vec<(u64, Vec<&'static str>)>,
    /// Whether enough seeds lasted.
    lasted_met: bool,
    /// The seeds whose bushes didn't hold, and how many seeds' were
    /// judged: those that ran the whole run (#224). None if the run was
    /// too short to judge them.
    bushes: Option<(Vec<u64>, usize)>,
    /// Whether enough seeds' bushes held: met if they weren't judged.
    bushes_met: bool,
    /// Each seed that failed a bar, with the bars it failed: one judged in
    /// each seed that lasted, or one judged across seeds that wasn't met
    /// and that it missed.
    failed: Vec<(u64, Vec<&'static str>)>,
    /// Each seed that missed a held-over bar, with the bars it missed
    /// (#332): reported, not failed.
    held_over: Vec<(u64, Vec<&'static str>)>,
    /// Each seed that lasted and was raced, with whether the evolved side
    /// won it, and whether the race was won (#226 Q7).
    raced: Vec<(u64, bool)>,
    race_won: bool,
    /// The seeds that failed a bar with more than half their living lacking
    /// a needs gene, at the end or the last hour before dying out, for
    /// which careful copying would switch on (#237 rule 4).
    trips: Vec<u64>,
    /// The seeds that missed only a held-over bar with more than half their
    /// living lacking a needs gene: careful copying would have switched on
    /// for them, had held-over bars counted (#332).
    held_over_trips: Vec<u64>,
}

impl Verdicts {
    /// Each held-over bar, with how many seeds missed it now and in Slice
    /// 29's run (M2 design §19.6).
    fn held_over_misses(&self) -> Vec<(&'static str, usize, usize)> {
        SLICE_29_MISSES
            .into_iter()
            .map(|(bar, then)| {
                let now = self
                    .held_over
                    .iter()
                    .filter(|(_, bars)| bars.contains(&bar));
                (bar, now.count(), then)
            })
            .collect()
    }

    /// The held-over bars that missed in more seeds than in Slice 29's run,
    /// each with how many seeds now and then: clearly worse, for the owner
    /// before the PR merges (M2 design §19.6).
    fn clearly_worse(&self) -> Vec<(&'static str, usize, usize)> {
        let misses = self.held_over_misses().into_iter();
        misses.filter(|&(_, now, then)| now > then).collect()
    }
}

/// Judges `runs`, each `hours` long, against the bars (M2 design §19.6),
/// and the held-over bars apart (#331). The young's growing up is reported,
/// not judged, on the long run (#226).
fn judge(runs: &[Run], hours: u64) -> Verdicts {
    let not_lasted: Vec<(u64, Vec<&str>)> = runs
        .iter()
        .map(|run| {
            let why = lasting::not_lasted(run.died_out.is_some(), &run.census, hours);
            (run.seed, why)
        })
        .filter(|(_, why)| !why.is_empty())
        .collect();
    let lasted_met = lasting::pooled_met(not_lasted.len(), runs.len());
    let bushes: Option<(Vec<u64>, usize)> = (hours >= lasting::JUDGED_FROM).then(|| {
        let judged: Vec<(u64, bool)> = runs
            .iter()
            .filter_map(|run| Some((run.seed, lasting::held(&run.bushes, hours)?)))
            .collect();
        let missed = judged.iter().filter(|(_, held)| !held);
        (missed.map(|&(seed, _)| seed).collect(), judged.len())
    });
    let bushes_met = bushes
        .as_ref()
        .is_none_or(|(missed, judged)| lasting::pooled_met(missed.len(), *judged));
    let raced: Vec<(u64, bool)> = runs
        .iter()
        .filter(|run| !not_lasted.iter().any(|(seed, _)| *seed == run.seed))
        .filter_map(|run| Some((run.seed, run.race?.won()?)))
        .collect();
    let race_won = raced.iter().filter(|(_, won)| *won).count() >= RACE_WINS;
    let (mut failed, mut held_over) = (Vec::new(), Vec::new());
    for run in runs {
        let (mut fails, mut misses) = (Vec::new(), Vec::new());
        let why = not_lasted.iter().find(|(s, _)| *s == run.seed);
        if !lasted_met && let Some((_, why)) = why {
            fails.extend(why);
        }
        let (whole, late) = (run.whole, run.late);
        // A world dying out starves its adults, and stills them: #226
        // judges their hunger and thirst only in the seeds that last, and
        // #331 their moving.
        if why.is_none()
            && ![whole, late]
                .iter()
                .all(|c| 20 * c.adult_hunger_thirst < c.grown_deaths.max(1))
        {
            misses.push(HUNGER_AND_THIRST);
        }
        if ![whole, late].iter().all(|c| 10 * c.too_full <= c.bites) {
            misses.push(TOO_FULL);
        }
        if why.is_none() && 4 * late.moving < late.adult_ticks {
            fails.push(ON_THE_MOVE);
        }
        if !bushes_met
            && let Some((missed, _)) = &bushes
            && missed.contains(&run.seed)
        {
            fails.push("bushes");
        }
        if !race_won && raced.contains(&(run.seed, false)) {
            fails.push(RACE);
        }
        if !fails.is_empty() {
            failed.push((run.seed, fails));
        }
        if !misses.is_empty() {
            held_over.push((run.seed, misses));
        }
    }
    let lacks_needs = |run: &&Run| !needs_lacked(&run.lacking).is_empty();
    let in_list = |list: &[(u64, Vec<&str>)], seed: u64| list.iter().any(|(s, _)| *s == seed);
    let trips = runs
        .iter()
        .filter(|run| in_list(&failed, run.seed))
        .filter(lacks_needs)
        .map(|run| run.seed)
        .collect();
    let held_over_trips = runs
        .iter()
        .filter(|run| in_list(&held_over, run.seed) && !in_list(&failed, run.seed))
        .filter(lacks_needs)
        .map(|run| run.seed)
        .collect();
    Verdicts {
        not_lasted,
        lasted_met,
        bushes,
        bushes_met,
        failed,
        held_over,
        raced,
        race_won,
        trips,
        held_over_trips,
    }
}

/// Each seed against the bars, all the seeds together, the starter genes
/// lacked at the end, evolution (G4, over the seeds that lasted, as
/// `starter`'s numbers moved) and the verdict.
fn bars(runs: &[Run], starter: &[(String, f32)], options: &Options) {
    let hours = options.hours;
    let verdicts = judge(runs, hours);
    println!("\nEach seed against the bars");
    println!(
        "{:>5} {:>7} {:>7} {:>8} {:>15} {:>17} {:>8} {:>17} {:>9} {:>7} {:>8} {:>8} {:>13}",
        "Seed",
        "Lasted",
        "Fewest",
        "Spr held",
        "Young grew up",
        "AdultH/T run/late",
        "Old H/T",
        "TooFull run/late",
        "Old full",
        "Bushes",
        "Moving",
        "YoungMv",
        "Lone newborns"
    );
    let mut pooled = Counts::default();
    let mut pooled_late = Counts::default();
    let mut shares = Vec::new();
    for run in runs {
        let (whole, late) = (run.whole, run.late);
        pooled.add(&whole);
        pooled_late.add(&late);
        let lasted = !verdicts
            .not_lasted
            .iter()
            .any(|(seed, _)| *seed == run.seed);
        let raised = whole.grew_up + whole.died_young;
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
            "{:>5} {:>7} {:>7} {:>8} {:>15} {:>17} {:>8} {:>17} {:>9} {:>7} {:>8} {:>8} {:>13}",
            run.seed,
            if lasted { "yes" } else { "no" },
            run.census.fewest,
            yes_no(lasting::held(&run.census.hourly, hours)),
            format!("{} of {raised}", whole.grew_up),
            format!(
                "{}/{}",
                percent(whole.adult_hunger_thirst, whole.grown_deaths),
                percent(late.adult_hunger_thirst, late.grown_deaths)
            ),
            whole.old_hunger_thirst,
            format!(
                "{}/{}",
                percent(whole.too_full, whole.bites),
                percent(late.too_full, late.bites)
            ),
            percent(whole.old_too_full, whole.old_bites),
            yes_no(lasting::held(&run.bushes, hours)),
            percent(late.moving, late.adult_ticks),
            percent(late.young_moving, late.young_ticks),
            newborns
        );
    }
    println!(
        "\nFewest: sprites at a count of the population record. Spr held and Bushes: over the \
         last {LATE_HOURS} hours, at least half of hours 50-100. Old full: old sprites' bites \
         tried too full, reported apart from the bar."
    );
    println!(
        "\nAll seeds: young grew up {} ({} of {}), with no bar here (#226); adults dying of \
         hunger or thirst {} of grown-up deaths ({} of {}), and old sprites {}; bites tried too \
         full {} ({} of {}), and old sprites' {} ({} of {}) apart from the bar; over the last \
         {LATE_HOURS} hours, adults on the move {} (the bar: a quarter, in each seed that lasts, \
         #331) and the young {}.",
        percent(pooled.grew_up, pooled.grew_up + pooled.died_young),
        pooled.grew_up,
        pooled.grew_up + pooled.died_young,
        percent(pooled.adult_hunger_thirst, pooled.grown_deaths),
        pooled.adult_hunger_thirst,
        pooled.grown_deaths,
        pooled.old_hunger_thirst,
        percent(pooled.too_full, pooled.bites),
        pooled.too_full,
        pooled.bites,
        percent(pooled.old_too_full, pooled.old_bites),
        pooled.old_too_full,
        pooled.old_bites,
        percent(pooled_late.moving, pooled_late.adult_ticks),
        percent(pooled_late.young_moving, pooled_late.young_ticks),
    );
    let met = |met: bool| if met { "met" } else { "not met" };
    let short = hours < lasting::JUDGED_FROM;
    let why: Vec<String> = verdicts
        .not_lasted
        .iter()
        .map(|(seed, why)| format!("seed {seed} ({})", why.join(", ")))
        .collect();
    println!(
        "\nThe world lasted in {} of {} seeds: {} (bar: at least 9 of 10).{}{}",
        runs.len() - verdicts.not_lasted.len(),
        runs.len(),
        met(verdicts.lasted_met),
        if why.is_empty() {
            String::new()
        } else {
            format!(" Didn't last: {}.", why.join("; "))
        },
        if short {
            " The sprites' 50-hour averages weren't judged: they need 150 hours or more."
        } else {
            ""
        }
    );
    match &verdicts.bushes {
        Some((missed, judged)) => println!(
            "The bushes held in {} of {judged} seeds that ran the whole run: {} (bar: at least 9 \
             of 10).",
            judged - missed.len(),
            met(verdicts.bushes_met)
        ),
        None => println!("The bushes weren't judged: they need a run of 150 hours or more."),
    }
    if let Some(middle) = middle(shares) {
        println!(
            "Lone newborns given the end's genes: {:.0}% grew up in the middle seed (the \
             unattended band is 10%-30%; whether the species has fallen behind is Slice 30's \
             race against the starter's genes).",
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
    let [with, without] = [0, 1].map(|n| {
        let raised = runs.iter().map(|run| run.by_parent[n]);
        raised.fold(Raised::default(), |sum, r| Raised {
            grew_up: sum.grew_up + r.grew_up,
            died_young: sum.died_young + r.died_young,
        })
    });
    println!(
        "Of the hatched young that saw a parent for most of their childhood, {} grew up ({} of \
         {}); of the rest, {} ({} of {}).",
        percent(with.grew_up, with.grew_up + with.died_young),
        with.grew_up,
        with.grew_up + with.died_young,
        percent(without.grew_up, without.grew_up + without.died_young),
        without.grew_up,
        without.grew_up + without.died_young,
    );
    genes_lacked(runs, hours);
    race(runs, &verdicts);
    let (lasted, dropped) = evolution_seeds(runs, &verdicts.not_lasted);
    let by_numbers = evolution::evolution(starter, &lasted, &dropped);
    // The numbers unmet fail no seed: the race is G4's second test (#226).
    let g4 = by_numbers || verdicts.race_won;
    if verdicts.failed.is_empty() && g4 && verdicts.race_won {
        let every = if short {
            "Every bar judged"
        } else {
            "Every bar"
        };
        println!("\n{every} was met.");
    } else {
        let mut lines = listed(&verdicts.failed);
        if !verdicts.race_won {
            lines.push("the race, across seeds: the species has fallen behind".into());
        }
        if !g4 {
            lines.push("G4, across seeds".into());
        }
        println!("\nBars failed: {}.", lines.join("; "));
    }
    if g4 && !by_numbers {
        println!(
            "G4 was met by the race, its second test: the numbers, its first, fell short (#226)."
        );
    } else if !g4 {
        println!(
            "G4 wasn't met by the numbers, its first test, nor by the race, its second (#226)."
        );
    }
    held_over(&verdicts);
    if !verdicts.trips.is_empty() {
        println!(
            "Careful copying would switch on: seeds {} failed a bar with more than half their \
             living lacking a needs gene{}.",
            seeds(&verdicts.trips),
            before_dying_out(runs, &verdicts.trips)
        );
    }
    if !verdicts.held_over_trips.is_empty() {
        println!(
            "A held-over bar doesn't switch careful copying on, but would have for seeds {}, \
             with more than half their living lacking a needs gene{}: for #332.",
            seeds(&verdicts.held_over_trips),
            before_dying_out(runs, &verdicts.held_over_trips)
        );
    }
}

/// The race against the starter's genes, seed by seed, and whether it was
/// won (#226 Q7, #237 rules 1 and 8).
fn race(runs: &[Run], verdicts: &Verdicts) {
    println!(
        "\nThe race: each seed's fresh world run as the viability run, {RACE_TICKS} ticks, with \
         the starter's genes and with the genes of the hatched sprites alive at the end"
    );
    println!(
        "{:>5} {:>22} {:>22} {:>5}",
        "Seed", "Starter: young grew up", "Evolved: young grew up", "Won"
    );
    let side = |raised: Raised| {
        format!(
            "{} of {} ({})",
            raised.grew_up,
            raised.counted(),
            percent(raised.grew_up, raised.counted())
        )
    };
    for run in runs {
        let Some(race) = run.race else {
            println!("{:>5} {:>22} {:>22} {:>5}", run.seed, "-", "-", "-");
            continue;
        };
        let judged = verdicts.raced.iter().find(|(seed, _)| *seed == run.seed);
        println!(
            "{:>5} {:>22} {:>22} {:>5}",
            run.seed,
            side(race.starter),
            side(race.evolved),
            yes_no(judged.map(|&(_, won)| won))
        );
    }
    let won = verdicts.raced.iter().filter(|(_, won)| *won).count();
    println!(
        "\nThe evolved side's young grew up more often in {won} of the {} seeds that lasted and \
         were raced: {} (bar: at least {RACE_WINS}).{}",
        verdicts.raced.len(),
        if verdicts.race_won { "won" } else { "lost" },
        if verdicts.race_won {
            ""
        } else {
            " The species has fallen behind the starter's genes."
        }
    );
}

/// Each seed with the bars it failed or missed: `seed 8: too full`.
fn listed(seeds: &[(u64, Vec<&str>)]) -> Vec<String> {
    seeds
        .iter()
        .map(|(seed, bars)| format!("seed {seed}: {}", bars.join(", ")))
        .collect()
}

/// `seeds`, as `4, 9`.
fn seeds(seeds: &[u64]) -> String {
    let seeds: Vec<String> = seeds.iter().map(u64::to_string).collect();
    seeds.join(", ")
}

/// Which of `seeds` died out, judged on their living before it:
/// ` (seed 4's at hour 170, before it died out)`, or nothing.
fn before_dying_out(runs: &[Run], seeds: &[u64]) -> String {
    let before: Vec<String> = runs
        .iter()
        .filter(|run| run.died_out.is_some() && seeds.contains(&run.seed))
        .map(|run| format!("seed {}'s at hour {}", run.seed, run.lacking_hour))
        .collect();
    if before.is_empty() {
        String::new()
    } else {
        format!(" ({}, before it died out)", before.join("; "))
    }
}

/// The held-over bars (the owner, 2026-10-24): the seeds that missed them,
/// and whether that's clearly worse than Slice 29's run (#331).
fn held_over(verdicts: &Verdicts) {
    if verdicts.held_over.is_empty() {
        println!("The bars held over to #332 were met in every seed.");
        return;
    }
    println!(
        "Held over to #332, reported but not must-pass (the owner, 2026-10-24): {}.",
        listed(&verdicts.held_over).join("; ")
    );
    let missed: Vec<String> = verdicts
        .held_over_misses()
        .into_iter()
        .map(|(bar, now, then)| format!("{bar} in {now} (Slice 29: {then})"))
        .collect();
    let worse: Vec<&str> = verdicts
        .clearly_worse()
        .into_iter()
        .map(|(bar, _, _)| bar)
        .collect();
    if worse.is_empty() {
        println!(
            "Seeds missing them: {}, no more than in Slice 29's run.",
            missed.join(", ")
        );
    } else {
        println!(
            "Seeds missing them: {}. Clearly worse than Slice 29's run in {}, so it goes back \
             to the owner before the PR merges.",
            missed.join(", "),
            worse.join(" and ")
        );
    }
}

/// The starter genes the hatched sprites alive at the end lack, seed by
/// seed: the needs genes, then each other gene a tenth or more of some
/// seed's living lack (#237 rule 4). A seed whose last hour with hatched
/// sprites alive came before the end is measured then.
fn genes_lacked(runs: &[Run], hours: u64) {
    let others: BTreeSet<&str> = runs
        .iter()
        .flat_map(|run| &run.lacking)
        .filter(|&(gene, &share)| share >= LISTED_FROM && !NEEDS.contains(&gene.as_str()))
        .map(|(gene, _)| gene.as_str())
        .collect();
    let shown: Vec<&str> = NEEDS.into_iter().chain(others).collect();
    let width = 2 + shown
        .iter()
        .map(|gene| gene.chars().count())
        .max()
        .unwrap_or(0);
    println!(
        "\nStarter genes lacked by the hatched sprites alive at the end: * a needs gene, and the \
         others a tenth or more of some seed's living lack"
    );
    print!("{:<width$}", "Gene");
    for run in runs {
        print!("{:>9}", format!("seed {}", run.seed));
    }
    println!();
    for gene in shown {
        let mark = if NEEDS.contains(&gene) { "*" } else { " " };
        let pad = width - 1;
        print!("{mark}{gene:<pad$}");
        for run in runs {
            let share = run.lacking.get(gene);
            print!(
                "{:>9}",
                share.map_or("-".into(), |share| format!("{:.0}%", 100.0 * share))
            );
        }
        println!();
    }
    for run in runs.iter().filter(|run| !run.lacking.is_empty()) {
        if run.died_out.is_some() {
            println!(
                "Seed {}: at hour {}, before it died out.",
                run.seed, run.lacking_hour
            );
        } else if run.lacking_hour < hours {
            println!(
                "Seed {}: at hour {}, the last with hatched sprites alive.",
                run.seed, run.lacking_hour
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use terra_sim::{ActionView, Genome, Hurt, Map, Progress, Scenario};

    /// An Eat by sprite `id` that ended `outcome`, having made its try or
    /// not.
    fn eat(id: u64, outcome: Outcome, attempted: bool) -> EventKind {
        EventKind::ActionEnded {
            id: EntityId(id),
            verb: Verb::Eat,
            outcome,
            action: ActionView {
                verb: Verb::Eat,
                destination: None,
                target: None,
                target_type: None,
                attempted,
                target_gone: false,
                hurt: Hurt::default(),
                progress: Progress::Ended(outcome),
                remembered: false,
                heard: None,
                egg_at: None,
            },
        }
    }

    /// A tally of sprite 1, an adult, and sprite 2, old.
    fn tally() -> Tally {
        Tally {
            stages: BTreeMap::from([
                (EntityId(1), LifeStage::Adult),
                (EntityId(2), LifeStage::Old),
            ]),
            ..Tally::default()
        }
    }

    /// Seed `seed`'s run of `hours` hours, meeting every bar: 30 sprites
    /// and 1,000 bushes all along, 100 bites with none too full, and 100
    /// grown-up deaths with none of adults by hunger or thirst, over the
    /// whole run and the last 50 hours.
    fn lasted(seed: u64, hours: u64) -> Run {
        let counts = Counts {
            bites: 100,
            grown_deaths: 100,
            ..Counts::default()
        };
        Run {
            seed,
            whole: counts,
            late: counts,
            race: Some(race(90, 91)),
            bushes: vec![1_000; hours as usize],
            census: Census {
                fewest: 30,
                hourly: vec![30; hours as usize],
            },
            lacking_hour: hours,
            ..Run::default()
        }
    }

    /// A race in which the starter's side raised 90 young of 100 and the
    /// evolved side `grew_up` of `of`.
    fn race(grew_up: u64, of: u64) -> Race {
        Race {
            starter: Raised {
                grew_up: 90,
                died_young: 10,
            },
            evolved: Raised {
                grew_up,
                died_young: of - grew_up,
            },
        }
    }

    /// Ten seeds' runs of 220 hours, meeting every bar.
    fn ten() -> Vec<Run> {
        (1..=10).map(|seed| lasted(seed, 220)).collect()
    }

    /// The share of the living lacking `drank → thirst`, a needs gene.
    fn lacking_thirst(share: f64) -> BTreeMap<String, f64> {
        BTreeMap::from([("drank Level → thirst".to_string(), share)])
    }

    #[test]
    fn only_an_eat_that_made_its_try_is_a_bite_tried() {
        // #224's bar counts bites tried (M2 design §19.6): an Eat that ended
        // on its way, its food gone or its attention moved, made none.
        let (mut tally, mut now) = (tally(), Counts::default());
        for outcome in [
            Outcome::Interrupted,
            Outcome::Failed,
            Outcome::Blocked,
            Outcome::TimedOut,
        ] {
            tally.count(&eat(1, outcome, false), &mut now);
        }
        assert_eq!((now.bites, now.too_full), (0, 0));
        tally.count(&eat(1, Outcome::Applied, true), &mut now);
        tally.count(&eat(1, Outcome::Failed, true), &mut now);
        tally.count(&eat(1, Outcome::TooFull, true), &mut now);
        assert_eq!((now.bites, now.too_full), (3, 1));
    }

    #[test]
    fn old_sprites_bites_are_counted_apart_from_the_bar() {
        // #225 round 1: old sprites' tries are reported apart from the bar.
        let (mut tally, mut now) = (tally(), Counts::default());
        tally.count(&eat(2, Outcome::TooFull, true), &mut now);
        tally.count(&eat(2, Outcome::Applied, true), &mut now);
        assert_eq!((now.bites, now.too_full), (0, 0));
        assert_eq!((now.old_bites, now.old_too_full), (2, 1));
        // An adult's bites count apart once it has grown old.
        tally.count(&eat(1, Outcome::TooFull, true), &mut now);
        tally.count(&EventKind::GrewOld { id: EntityId(1) }, &mut now);
        tally.count(&eat(1, Outcome::TooFull, true), &mut now);
        assert_eq!((now.bites, now.too_full), (1, 1));
        assert_eq!((now.old_bites, now.old_too_full), (3, 2));
    }

    #[test]
    fn the_census_counts_sprites_at_each_count_of_the_population_record() {
        // #222 judges the fewest sprites at any count of the record (M2
        // design §14): its first, before the first step, then every 100
        // ticks until it squeezes.
        let data = DataPack::builtin().expect("the built-in data pack is valid");
        let sprites: Vec<(Pos, Option<Genome>)> = [0, 2, 4].map(|x| (Pos { x, y: 0 }, None)).into();
        let world = on_a_row(data, &sprites);
        let mut census = Census::new(&world);
        census.note(100, 30, 100);
        assert_eq!(census.fewest, 3);
        assert_eq!(
            lasting::not_lasted(false, &census, 140),
            ["under 10 sprites"]
        );
        let mut census = Census {
            fewest: 30,
            hourly: Vec::new(),
        };
        census.note(200, 12, 100);
        census.note(250, 5, 100);
        census.note(HOUR, 20, 100);
        assert_eq!((census.fewest, &census.hourly[..]), (12, &[20][..]));
        // The record takes its 512th count at tick 51,100, then squeezes to
        // count every 200 ticks, so that step's `every` from before it is
        // still 100.
        census.note(51_100, 11, 100);
        census.note(51_300, 4, 200);
        assert_eq!(census.fewest, 11);
    }

    #[test]
    fn a_seed_lasts_never_under_10_sprites_and_holding_half_its_sprites() {
        // #222: never fewer than 10 sprites at any count of its population
        // record, and its sprites over the last 50 hours averaging at least
        // half of hours 50–100.
        let census = |fewest, late, hours: usize| {
            let mut hourly = vec![100; hours];
            hourly[hours - 50..].fill(late);
            Census { fewest, hourly }
        };
        assert!(lasting::not_lasted(false, &census(10, 50, 220), 220).is_empty());
        assert_eq!(
            lasting::not_lasted(false, &census(9, 50, 220), 220),
            ["under 10 sprites"]
        );
        assert_eq!(
            lasting::not_lasted(false, &census(10, 49, 220), 220),
            ["sprites fell by half"]
        );
        assert_eq!(
            lasting::not_lasted(true, &census(0, 50, 180), 220),
            ["died out", "under 10 sprites"]
        );
        // A run under 150 hours can't judge the averages.
        assert!(lasting::not_lasted(false, &census(10, 10, 140), 140).is_empty());
    }

    #[test]
    fn the_world_lasting_is_judged_across_seeds() {
        // #222: at least 9 of 10 seeds must last, so one that doesn't fails
        // no bar on its own.
        let mut runs = ten();
        runs[3].census.fewest = 9;
        let one = judge(&runs, 220);
        assert_eq!(one.not_lasted, [(4, vec!["under 10 sprites"])]);
        assert!(one.lasted_met);
        assert!(one.failed.is_empty(), "{:?}", one.failed);
        runs[6].census.hourly[170..].fill(14);
        let two = judge(&runs, 220);
        assert!(!two.lasted_met);
        assert_eq!(
            two.failed,
            [
                (4, vec!["under 10 sprites"]),
                (7, vec!["sprites fell by half"])
            ]
        );
    }

    #[test]
    fn evolution_is_judged_over_the_seeds_that_last() {
        // #226: G4 is judged on the long run over the seeds that last, so
        // one that doesn't drops out of it rather than failing it.
        let mut runs = ten();
        for run in &mut runs {
            run.numbers = vec![1.0];
        }
        runs[3].census.fewest = 9;
        runs[6].numbers.clear();
        let (lasted, dropped) = evolution_seeds(&runs, &judge(&runs, 220).not_lasted);
        let seeds: Vec<u64> = lasted.iter().map(|(seed, _)| *seed).collect();
        assert_eq!(seeds, [1, 2, 3, 5, 6, 8, 9, 10]);
        assert_eq!(dropped, [4, 7]);
    }

    #[test]
    fn evolution_is_met_by_three_numbers_moved_the_same_way_in_every_seed() {
        // M2 design §12.7: at least 3 numbers moved a tenth the same way in
        // every seed that lasted, and none lasting meets nothing.
        let starter: Vec<(String, f32)> = ["a", "b", "c", "d"]
            .iter()
            .map(|name| (name.to_string(), 1.0))
            .collect();
        let three = [1.2, 0.8, 1.5, 1.0];
        assert!(evolution::evolution(
            &starter,
            &[(1, &three), (2, &three)],
            &[3]
        ));
        let two = [1.2, 0.8, 1.05, 1.0];
        assert!(!evolution::evolution(
            &starter,
            &[(1, &three), (2, &two)],
            &[]
        ));
        assert!(!evolution::evolution(&starter, &[], &[1, 2]));
        // A number none of a seed's living carry moves no way there.
        let lost = [1.2, 0.8, 1.5, f32::NAN];
        assert!(evolution::evolution(
            &starter,
            &[(1, &lost), (2, &lost)],
            &[]
        ));
        let half_lost = [1.2, 0.8, f32::NAN, 1.0];
        assert!(!evolution::evolution(
            &starter,
            &[(1, &three), (2, &half_lost)],
            &[]
        ));
    }

    #[test]
    fn the_bushes_are_judged_across_seeds() {
        // #224: the bushes hold in at least 9 seeds of 10, so one seed's
        // miss fails no bar and switches careful copying on for none.
        let mut runs = ten();
        runs[5].bushes[170..].fill(400);
        runs[5].lacking = lacking_thirst(0.9);
        let one = judge(&runs, 220);
        assert_eq!(one.bushes, Some((vec![6], 10)));
        assert!(one.bushes_met);
        assert_eq!((one.failed, one.trips), (vec![], vec![]));
        runs[1].bushes[170..].fill(499);
        let two = judge(&runs, 220);
        assert!(!two.bushes_met);
        assert_eq!(two.failed, [(2, vec!["bushes"]), (6, vec!["bushes"])]);
        assert_eq!(two.trips, [6]);
        // A run under 150 hours doesn't judge them.
        let short: Vec<Run> = (1..=10).map(|seed| lasted(seed, 140)).collect();
        assert_eq!(judge(&short, 140).bushes, None);
    }

    #[test]
    fn the_young_growing_up_fails_no_seed() {
        // #226: on the long run the young's bands are reported for
        // information.
        let mut runs = ten();
        runs[0].whole.grew_up = 1;
        runs[0].whole.died_young = 9;
        runs[0].lacking = lacking_thirst(0.9);
        let judged = judge(&runs, 220);
        assert_eq!((judged.failed, judged.trips), (vec![], vec![]));
    }

    #[test]
    fn adults_hunger_and_thirst_are_held_over_in_the_seeds_that_last() {
        // #226: adults dying of hunger or thirst stay under 1 in 20 of
        // grown-up deaths in every seed that lasts. Held over to #332 by
        // the owner (2026-10-24), a miss fails no seed and switches careful
        // copying on for none, though the seeds it would have are named.
        let mut runs = ten();
        for run in &mut runs[..2] {
            run.late.adult_hunger_thirst = 5;
            run.lacking = lacking_thirst(0.9);
        }
        runs[1].died_out = Some(200 * HOUR);
        runs[1].census.fewest = 0;
        let judged = judge(&runs, 220);
        assert!(judged.lasted_met);
        assert_eq!(judged.failed, []);
        assert_eq!(judged.held_over, [(1, vec![HUNGER_AND_THIRST])]);
        assert_eq!((judged.trips, judged.held_over_trips), (vec![], vec![1]));
    }

    #[test]
    fn too_full_bites_are_held_over() {
        // #224's bar, held over to #332 (the owner, 2026-10-24): at most 1
        // in 10 bites tried too full, over the run and its last 50 hours.
        let mut runs = ten();
        runs[2].late.too_full = 11;
        runs[4].whole.too_full = 10;
        let judged = judge(&runs, 220);
        assert_eq!(judged.failed, []);
        assert_eq!(judged.held_over, [(3, vec![TOO_FULL])]);
    }

    #[test]
    fn a_held_over_bar_missed_in_more_seeds_than_slice_29_s_run_is_clearly_worse() {
        // #331: more than 2 seeds for hunger and thirst, or more than 1 for
        // too full, goes back to the owner before merging.
        let mut runs = ten();
        for run in &mut runs[..2] {
            run.late.adult_hunger_thirst = 5;
        }
        runs[9].late.too_full = 11;
        assert_eq!(judge(&runs, 220).clearly_worse(), []);
        runs[2].whole.adult_hunger_thirst = 5;
        runs[3].whole.too_full = 11;
        assert_eq!(
            judge(&runs, 220).clearly_worse(),
            [(HUNGER_AND_THIRST, 3, 2), (TOO_FULL, 2, 1)]
        );
    }

    #[test]
    fn adults_on_the_move_a_quarter_of_the_time_is_judged_in_the_seeds_that_last() {
        // #331: over the last 50 hours, in each seed that lasts. A seed
        // that didn't last is judged on that alone.
        let mut runs = ten();
        for run in &mut runs {
            run.late.adult_ticks = 1_000;
            run.late.moving = 250;
        }
        assert_eq!(judge(&runs, 220).failed, []);
        runs[1].late.moving = 249;
        runs[7].late.moving = 100;
        runs[7].died_out = Some(200 * HOUR);
        runs[7].census.fewest = 0;
        runs[7].lacking = lacking_thirst(0.9);
        runs[1].lacking = lacking_thirst(0.9);
        let judged = judge(&runs, 220);
        assert_eq!(judged.failed, [(2, vec![ON_THE_MOVE])]);
        assert_eq!(judged.trips, [2]);
    }

    /// A world of one grown sprite of `genes` on each of its tiles.
    fn grown(genes: &[&str]) -> World {
        let data = DataPack::builtin().expect("the built-in data pack is valid");
        let sprites: Vec<(Pos, Option<Genome>)> = (genes.iter().enumerate())
            .map(|(x, genes)| {
                let text = format!("(format: 2, genes: [{genes}])");
                let genome = Genome::from_ron(&text, &data).expect("a valid genome");
                (Pos::new(2 * x as u16, 0), Some(genome))
            })
            .collect();
        on_a_row(data, &sprites)
    }

    /// A world of one row of grass, just long enough for `sprites`, with
    /// the starter's genes where they have none.
    fn on_a_row(data: DataPack, sprites: &[(Pos, Option<Genome>)]) -> World {
        let long = sprites.iter().map(|(pos, _)| pos.x + 1).max().unwrap_or(1);
        let row = ".".repeat(usize::from(long));
        let map = Map::from_ascii(&[&row], &data).expect("a valid drawing");
        let scenario = Scenario {
            map,
            objects: &[],
            sprites,
            scripted: &[],
            newborns: &[],
        };
        World::from_scenario(scenario, data, 1).expect("a valid scenario")
    }

    #[test]
    fn an_adult_s_wandering_is_switched_off_when_it_pulls_less_than_its_act_threshold() {
        // #318: a weakened gene, or none, under the pull it takes to act.
        // The young's gene doesn't work in an adult, and of two the first
        // is the one expressed (design §4.3).
        let act = r#"BrainParam(param: "act_threshold", value: 0.25),"#;
        let wander = |weight: f32, when: &str| {
            format!(
                r#"Instinct(inputs: [("always", false)], verb: Wander, weight: {weight:?}{when}),"#
            )
        };
        let world = grown(&[
            &format!("{act}{}", wander(0.3, ", from: Adult")),
            &format!("{act}{}", wander(0.2, ", from: Adult")),
            act,
            "",
            &format!("{act}{}", wander(0.3, ", until: Adult")),
            &format!("{act}{}{}", wander(0.3, ""), wander(0.2, "")),
        ]);
        let off: Vec<bool> = world.sprites().map(|s| switched_off(&s)).collect();
        assert_eq!(off, [false, true, true, false, true, false]);
        assert_eq!(wander_off(&world), (3, 6));
    }

    #[test]
    fn a_seed_that_died_out_is_judged_on_its_last_living() {
        // #237 rule 4: careful copying switches on for a world that fails a
        // bar while more than half its living lack a needs gene, a world
        // that died out too.
        let mut runs = ten();
        for (run, share) in runs.iter_mut().zip([0.8, 0.5]) {
            run.died_out = Some(180 * HOUR);
            run.census = Census {
                fewest: 0,
                hourly: vec![30; 180],
            };
            run.bushes.truncate(180);
            // Its last hatched sprites die in hour 171.
            run.measured(lacking_thirst(share), 170);
            run.measured(BTreeMap::new(), 171);
            assert_eq!(
                (&run.lacking, run.lacking_hour),
                (&lacking_thirst(share), 170)
            );
        }
        let judged = judge(&runs, 220);
        assert_eq!(
            judged.failed,
            [
                (1, vec!["died out", "under 10 sprites"]),
                (2, vec!["died out", "under 10 sprites"])
            ]
        );
        assert_eq!(judged.trips, [1]);
    }

    #[test]
    fn a_seed_is_won_when_the_evolved_side_s_young_grow_up_more_often() {
        // #226 Q7: a share against a share, so more young raised isn't a
        // win by itself, and a tie isn't one.
        assert_eq!(race(91, 100).won(), Some(true));
        assert_eq!(race(90, 100).won(), Some(false), "a tie");
        assert_eq!(
            race(180, 201).won(),
            Some(false),
            "more young, a smaller share"
        );
        assert_eq!(
            race(10, 11).won(),
            Some(true),
            "fewer young, a larger share"
        );
        assert_eq!(race(0, 0).won(), None, "no young to count");
    }

    #[test]
    fn the_race_is_won_by_8_seeds_and_lost_it_fails_the_seeds_not_won() {
        // #226 Q7, #237 rule 1: won when the evolved side wins at least 8
        // of the seeds raced. Lost, the species has fallen behind, and each
        // seed raced and not won has failed it, which can switch careful
        // copying on.
        let mut runs = ten();
        assert!(judge(&runs, 220).race_won);
        runs[1].race = Some(race(90, 100));
        runs[4].race = Some(race(80, 100));
        let two = judge(&runs, 220);
        assert!(two.race_won);
        assert_eq!(two.failed, []);
        runs[8].race = Some(race(85, 100));
        runs[8].lacking = lacking_thirst(0.9);
        let three = judge(&runs, 220);
        assert!(!three.race_won);
        assert_eq!(
            three.failed,
            [(2, vec![RACE]), (5, vec![RACE]), (9, vec![RACE])]
        );
        assert_eq!(three.trips, [9]);
    }

    #[test]
    fn a_seed_that_didn_t_last_or_wasn_t_raced_drops_out_of_the_race() {
        // #226 Q7: the race is over the seeds that lasted, so with three
        // out, the other seven can't win it.
        let mut runs = ten();
        runs[0].census.fewest = 9;
        runs[0].race = Some(race(10, 100));
        runs[1].race = None;
        runs[2].race = Some(Race::default());
        let judged = judge(&runs, 220);
        let seeds: Vec<u64> = judged.raced.iter().map(|(seed, _)| *seed).collect();
        assert_eq!(seeds, [4, 5, 6, 7, 8, 9, 10]);
        assert!(!judged.race_won);
        assert_eq!(judged.failed, [], "no seed raced lost it");
    }
}
