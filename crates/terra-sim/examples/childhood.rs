//! The harsher world's long run (M2 design §19.6): runs the default world,
//! left alone, from the given seeds for 220 hours of play each, and
//! reports every 10 hours how its sprites stand, then each seed against
//! Slice 29's bars and the bars judged across seeds, then lone newborns
//! given the genes of the sprites alive at the end.
//!
//! Each row: the hour; sprites, the young among them, and eggs; berry
//! bushes; and since the last row, the hatched young that grew up and that
//! died young, the grown-up deaths and how many of them were adults dying
//! of hunger or thirst, the share of bites tried by a young sprite or an
//! adult too full to eat and the same share of old sprites' bites, the
//! share of adults' time on the move and of the young's, and how many of
//! the eleven needs genes (M2 design §19.6) more than half of the hatched
//! sprites alive lack. A bite tried is an Eat that got to its food and made
//! its try (design §5.5), a Too full one too; an Eat that ended on its way
//! isn't one.
//!
//! The bars for each seed:
//!
//! - **Adults dying of hunger or thirst are under 1 in 20 of grown-up
//!   deaths,** over the run and over its last 50 hours (#226). Old
//!   sprites' are reported apart.
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
//! Reported with no bar: the young hatched that grew up, whose bar is the
//! viability run's (#226); adults' time on the move over the last 50 hours,
//! which #318 asks to be a quarter, and the young's (#237 rule 5); what
//! killed the hatched young, and how those that saw a parent for most of
//! their childhood fared against those that didn't; the share of the
//! hatched sprites alive at the end lacking each needs gene, and each other
//! starter gene that a tenth or more of some seed's living lack (#237 rule
//! 4), a gene with a life-stage switch listed by it, so the young's and
//! grown sprites' Wander genes show apart; and lone newborns. For each seed whose world lasted, the
//! unattended run (`scenarios/unattended.ron`) on the same seed, its
//! newborns taking after the hatched sprites alive at the end, until each
//! grows up or dies, set beside the unattended band of 10%–30%. Whether the
//! species has fallen behind is the race against the starter's genes,
//! which Slice 30 runs (#237 rules 1 and 8).
//!
//! Careful copying would switch on for a seed that failed a bar (or the
//! race is lost overall, which Slice 30 runs and this doesn't) while more
//! than half its living lack a needs gene (#237 rule 4, M2 design §19.6).
//! A seed that died out is judged on its living at the last hour any
//! hatched sprite was alive.
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
#[path = "common/genes.rs"]
mod genes;
#[path = "common/lasting.rs"]
mod lasting;

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;
use std::sync::Mutex;

use lasting::{Census, HOUR, LATE_HOURS};
use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, GeneView, LifeStage, Outcome, Pos, Verb, World,
    WorldConfig,
};

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

/// The share of a seed's living lacking a starter gene from which the gene
/// table lists it, as `generations` does; the needs genes are always
/// listed (#237 rule 4).
const LISTED_FROM: f64 = 0.1;

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
#[derive(Debug, Default, Clone, Copy)]
struct Raised {
    grew_up: u64,
    died_young: u64,
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
    died_young_of: BTreeMap<String, u64>,
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
                            *self.died_young_of.entry(cause_name(cause)).or_insert(0) += 1;
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
    /// Each starter gene, by what it does and when, and the share of the
    /// hatched sprites alive at the end of hour `lacking_hour` that lack
    /// it: the last hour any was alive, so the end for a world that lasted
    /// and the last before it died out for one that didn't. Empty if none
    /// ever was.
    lacking: BTreeMap<String, f64>,
    lacking_hour: u64,
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
    let starter = starter_genes(&data);
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
            let lacking = lacking(&world, starter);
            let needs_lacked = needs_lacked(&lacking).len();
            if !lacking.is_empty() {
                out.lacking = lacking;
                out.lacking_hour = hour;
            }
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
                    needs_lacked,
                });
            }
        }
        if world.sprites().count() + world.eggs().count() == 0 {
            out.died_out = Some(ticks);
            break;
        }
    }
    out.died_young_of = tally.died_young_of;
    out.by_parent = tally.by_parent;
    if out.died_out.is_none() {
        out.newborns = World::unattended_like(config, data.clone(), seed, &world).map(newborns);
    }
    out
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

/// What a death was of, by name.
fn cause_name(cause: DeathCause) -> String {
    match cause {
        DeathCause::Starvation => "hunger".into(),
        DeathCause::Dehydration => "thirst".into(),
        DeathCause::OldAge => "old age".into(),
        DeathCause::HurtBy(id) => format!("hurt by object type {id}"),
    }
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
        "{:>5} {:>7} {:>6} {:>5} {:>7} {:>7} {:>10} {:>6} {:>8} {:>8} {:>8} {:>7} {:>8} {:>6}",
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
        "Needs"
    );
    for row in &run.rows {
        let c = row.counts;
        println!(
            "{:>5} {:>7} {:>6} {:>5} {:>7} {:>7} {:>10} {:>6} {:>8} {:>8} {:>8} {:>7} {:>8} {:>6}",
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
    /// every seed, or one judged across seeds that wasn't met and that it
    /// missed.
    failed: Vec<(u64, Vec<&'static str>)>,
    /// The seeds among them with more than half their living lacking a
    /// needs gene, at the end or the last hour before dying out, for which
    /// careful copying would switch on (#237 rule 4).
    trips: Vec<u64>,
}

/// Judges `runs`, each `hours` long, against the bars (M2 design §19.6).
/// The young's growing up is reported, not judged, on the long run (#226).
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
    let mut failed = Vec::new();
    for run in runs {
        let mut fails = Vec::new();
        if !lasted_met && let Some((_, why)) = not_lasted.iter().find(|(s, _)| *s == run.seed) {
            fails.extend(why);
        }
        let (whole, late) = (run.whole, run.late);
        if ![whole, late]
            .iter()
            .all(|c| 20 * c.adult_hunger_thirst < c.grown_deaths.max(1))
        {
            fails.push("adults' hunger and thirst");
        }
        if ![whole, late].iter().all(|c| 10 * c.too_full <= c.bites) {
            fails.push("too full");
        }
        if !bushes_met
            && let Some((missed, _)) = &bushes
            && missed.contains(&run.seed)
        {
            fails.push("bushes");
        }
        if !fails.is_empty() {
            failed.push((run.seed, fails));
        }
    }
    let trips = runs
        .iter()
        .filter(|run| failed.iter().any(|(seed, _)| *seed == run.seed))
        .filter(|run| !needs_lacked(&run.lacking).is_empty())
        .map(|run| run.seed)
        .collect();
    Verdicts {
        not_lasted,
        lasted_met,
        bushes,
        bushes_met,
        failed,
        trips,
    }
}

/// Each seed against the bars, all the seeds together, and the starter
/// genes lacked at the end.
fn bars(runs: &[Run], options: &Options) {
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
         {LATE_HOURS} hours, adults on the move {} (#318 asks a quarter) and the young {}.",
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
    if verdicts.failed.is_empty() {
        if short {
            println!("\nEvery bar judged was met.");
        } else {
            println!("\nEvery bar was met.");
        }
        return;
    }
    let lines: Vec<String> = verdicts
        .failed
        .iter()
        .map(|(seed, fails)| format!("seed {seed}: {}", fails.join(", ")))
        .collect();
    println!("\nBars failed: {}.", lines.join("; "));
    if !verdicts.trips.is_empty() {
        let seeds: Vec<String> = verdicts.trips.iter().map(u64::to_string).collect();
        let before: Vec<String> = runs
            .iter()
            .filter(|run| run.died_out.is_some() && verdicts.trips.contains(&run.seed))
            .map(|run| format!("seed {}'s at hour {}", run.seed, run.lacking_hour))
            .collect();
        let before = if before.is_empty() {
            String::new()
        } else {
            format!(" ({}, before it died out)", before.join("; "))
        };
        println!(
            "Careful copying would switch on: seeds {} failed a bar with more than half their \
             living lacking a needs gene{before}.",
            seeds.join(", ")
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
    use terra_sim::{ActionView, Hurt, Progress};

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
            bushes: vec![1_000; hours as usize],
            census: Census {
                fewest: 30,
                hourly: vec![30; hours as usize],
            },
            lacking_hour: hours,
            ..Run::default()
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
    fn a_seed_that_died_out_is_judged_on_its_last_living() {
        // #237 rule 4: careful copying switches on for a world that fails a
        // bar while more than half its living lack a needs gene, a world
        // that died out too.
        let mut runs = ten();
        for run in &mut runs[..2] {
            run.died_out = Some(180 * HOUR);
            run.census = Census {
                fewest: 0,
                hourly: vec![30; 180],
            };
            run.bushes.truncate(180);
            run.lacking_hour = 170;
        }
        runs[0].lacking = lacking_thirst(0.8);
        runs[1].lacking = lacking_thirst(0.5);
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
}
