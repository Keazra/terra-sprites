//! Fast-forward (M2 design §16): a Lab world run with no screen, as fast
//! as the computer allows, for hours of play at 1×, then saved for the game
//! to open. `main.rs` runs it for `--fast-forward` and writes what it
//! leaves behind; what it runs and what it reports are here, so they can be
//! tested.

use std::collections::BTreeMap;
use std::time::Duration;

use terra_sim::{DeathCause, EntityId, EventKind, LifeStage, World};

use crate::text::group_thousands;

/// Ticks in an hour of play at 1×, 1.25 ticks a second (design §6.6).
pub const HOUR: u64 = 4_500;

/// How many rows a run reports: one each tenth of the way.
pub const ROWS: u64 = 10;

/// A world being fast-forwarded.
pub struct FastForward {
    world: World,
    /// The tick it started on, and the tick it runs to.
    start: u64,
    end: u64,
    /// How many rows it has reported.
    rows: u64,
    tally: Tally,
}

impl FastForward {
    /// Fast-forwards `world` by `hours` of play at 1×.
    pub fn new(world: World, hours: u64) -> FastForward {
        let start = world.tick();
        FastForward {
            start,
            end: start + hours * HOUR,
            rows: 0,
            tally: Tally::starting_from(&world),
            world,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn into_world(self) -> World {
        self.world
    }

    /// The deepest generation reached since it started.
    pub fn generation(&self) -> u32 {
        self.tally.deepest()
    }

    /// How many ticks it has run.
    pub fn ticks_run(&self) -> u64 {
        self.world.tick() - self.start
    }

    /// How many ticks it runs in all.
    pub fn ticks(&self) -> u64 {
        self.end - self.start
    }

    /// Whether it has run all its ticks.
    pub fn done(&self) -> bool {
        self.world.tick() >= self.end
    }

    /// The tick the next row is due on, or `None` once all have been: one
    /// each tenth of the way.
    pub fn next_row(&self) -> Option<u64> {
        (self.rows < ROWS).then(|| self.start + self.ticks() * (self.rows + 1) / ROWS)
    }

    /// Runs on to `tick`, but no further than its end.
    pub fn run_to(&mut self, tick: u64) {
        while self.world.tick() < tick.min(self.end) {
            for event in self.world.step() {
                self.tally.note(&event.kind);
            }
        }
    }

    /// How the world stands now, with the deaths since the row before, as
    /// the next row of the report.
    pub fn take_row(&mut self) -> Row {
        self.rows += 1;
        let mut row = Row {
            ticks_run: self.ticks_run(),
            young: 0,
            adults: 0,
            old: 0,
            eggs: self.world.eggs().count(),
            generation: self.tally.deepest(),
            died: self.tally.take_deaths(),
        };
        for sprite in self.world.sprites() {
            *match sprite.life_stage() {
                LifeStage::Young => &mut row.young,
                LifeStage::Adult => &mut row.adults,
                LifeStage::Old => &mut row.old,
            } += 1;
        }
        row
    }
}

/// What a run counts from the world's events: each sprite's generation,
/// and the deaths.
#[derive(Debug, Clone, Default)]
pub struct Tally {
    /// The generation of each sprite and egg in the world, counted from
    /// the sprites the run started with, which are the 0th.
    generations: BTreeMap<EntityId, u32>,
    /// The deepest generation reached.
    deepest: u32,
    /// Deaths since they were last taken, by cause.
    died: BTreeMap<DeathCause, u64>,
}

impl Tally {
    /// A tally from `world` as it is now: its sprites are the 0th
    /// generation, and its eggs the 1st.
    pub fn starting_from(world: &World) -> Tally {
        let sprites = world.sprites().map(|sprite| (sprite.id(), 0));
        let eggs = world.eggs().map(|egg| (egg.id(), 1));
        let generations: BTreeMap<EntityId, u32> = sprites.chain(eggs).collect();
        Tally {
            deepest: generations.values().copied().max().unwrap_or(0),
            generations,
            died: BTreeMap::new(),
        }
    }

    /// Takes in one of the world's events: an egg is a generation on from
    /// the deeper of its parents, a sprite the Cursor spawns is the 0th,
    /// and a death counts by its cause. The hatchling keeps its egg's ID,
    /// and so its generation.
    pub fn note(&mut self, kind: &EventKind) {
        match kind {
            EventKind::Laid { egg, by, with } => {
                let generation = [Some(by), with.as_ref()]
                    .into_iter()
                    .flatten()
                    .filter_map(|parent| self.generations.get(parent))
                    .max()
                    .map_or(1, |deepest| deepest + 1);
                self.generations.insert(*egg, generation);
                self.deepest = self.deepest.max(generation);
            }
            EventKind::Spawned { id, .. } => {
                self.generations.insert(*id, 0);
            }
            EventKind::Died { id, cause, .. } => {
                self.generations.remove(id);
                *self.died.entry(*cause).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    /// The deepest generation reached.
    pub fn deepest(&self) -> u32 {
        self.deepest
    }

    /// The deaths since they were last taken, by cause.
    pub fn take_deaths(&mut self) -> BTreeMap<DeathCause, u64> {
        std::mem::take(&mut self.died)
    }
}

/// How the world stood at a row of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The ticks run since the start.
    pub ticks_run: u64,
    pub young: usize,
    pub adults: usize,
    pub old: usize,
    pub eggs: usize,
    /// The deepest generation reached.
    pub generation: u32,
    /// Deaths since the row before, by cause.
    pub died: BTreeMap<DeathCause, u64>,
}

/// The report's column headings, over its rows.
pub const HEADER: &str =
    " Hours  Sprites  Young  Adults  Old  Eggs  Generation   Died  Old age  Hunger  Thirst  Hurt";

impl Row {
    /// The sprites, eggs included, as the world's limit counts them (M2
    /// design §10.6).
    pub fn sprites(&self) -> usize {
        self.young + self.adults + self.old + self.eggs
    }

    /// The row as the report prints it, under `HEADER`.
    pub fn line(&self) -> String {
        let died = |cause| self.died.get(&cause).copied().unwrap_or(0);
        let hurt: u64 = self
            .died
            .iter()
            .filter(|(cause, _)| matches!(cause, DeathCause::HurtBy(_)))
            .map(|(_, n)| n)
            .sum();
        format!(
            "{:>6}  {:>7}  {:>5}  {:>6}  {:>3}  {:>4}  {:>10}  {:>5}  {:>7}  {:>6}  {:>6}  {:>4}",
            hours(self.ticks_run),
            group_thousands(self.sprites() as u64),
            group_thousands(self.young as u64),
            group_thousands(self.adults as u64),
            group_thousands(self.old as u64),
            group_thousands(self.eggs as u64),
            self.generation,
            group_thousands(self.died.values().sum()),
            group_thousands(died(DeathCause::OldAge)),
            group_thousands(died(DeathCause::Starvation)),
            group_thousands(died(DeathCause::Dehydration)),
            group_thousands(hurt),
        )
    }
}

/// What the report says first: which world it runs, and for how long.
/// `loaded` is the save it came from, if any, and `preset` the preset a
/// new world was made from.
pub fn opening_line(world: &World, loaded: Option<&str>, preset: &str, hours: u64) -> String {
    let what = match loaded {
        Some(name) => format!(
            "{name}, a Lab world at tick {}",
            group_thousands(world.tick())
        ),
        None => format!("a new Lab world, seed {}, {preset} preset", world.seed()),
    };
    let unit = if hours == 1 { "hour" } else { "hours" };
    format!(
        "Fast-forwarding {what}, for {} {unit} ({} ticks).",
        group_thousands(hours),
        group_thousands(hours * HOUR)
    )
}

/// What the report says once a run ends: how far it got, how long that
/// took and how fast it ran. `stopped` is whether `Ctrl+C` stopped it
/// early.
pub fn closing_line(
    stopped: bool,
    ticks_run: u64,
    took: Duration,
    ticks_per_second: f64,
) -> String {
    let unit = if ticks_run == HOUR { "hour" } else { "hours" };
    let ran = hours(ticks_run);
    let took = spoken(took);
    let rate = group_thousands(ticks_per_second.round() as u64);
    if stopped {
        format!("Stopped at {ran} {unit}, after {took}, {rate} ticks a second.")
    } else {
        format!("Done: {ran} {unit} in {took}, {rate} ticks a second.")
    }
}

/// The name a run's save gets unless `--save` names it: what it's named
/// after, then the hours it ran, as `seed-7-200h` or `mira-50h`.
pub fn save_name(from: &str, ticks_run: u64) -> String {
    format!("{from}-{}h", hours(ticks_run).replace(',', ""))
}

/// `ticks` as hours of play at 1×, to the nearest tenth, with no ".0".
pub fn hours(ticks: u64) -> String {
    let tenths = (ticks * 10 + HOUR / 2) / HOUR;
    match tenths % 10 {
        0 => group_thousands(tenths / 10),
        tenth => format!("{}.{tenth}", group_thousands(tenths / 10)),
    }
}

/// The line that shows how a run is going, updated in place: how far it
/// has got, the generation and the sprites, its speed, and about how long
/// it has left at that speed.
pub fn progress_line(run: &FastForward, ticks_per_second: f64) -> String {
    let world = run.world();
    let sprites = world.sprites().count() + world.eggs().count();
    let mut line = format!(
        "► {} hours · generation {} · {} sprites",
        hours(run.ticks_run()),
        run.generation(),
        group_thousands(sprites as u64),
    );
    if ticks_per_second > 0.0 {
        let left = (run.ticks() - run.ticks_run()) as f64 / ticks_per_second;
        line += &format!(
            " · {} ticks a second · about {} left",
            group_thousands(ticks_per_second as u64),
            spoken(Duration::from_secs_f64(left)),
        );
    }
    line
}

/// `time` as the report says it: "45 seconds", "13 minutes", "2 hours 5
/// minutes".
pub fn spoken(time: Duration) -> String {
    let seconds = time.as_secs();
    let (hours, minutes) = (seconds / 3_600, seconds / 60 % 60);
    let plural = |n: u64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    match (hours, minutes) {
        (0, 0) => plural(seconds, "second"),
        (0, m) => plural(m, "minute"),
        (h, 0) => plural(h, "hour"),
        (h, m) => format!("{} {}", plural(h, "hour"), plural(m, "minute")),
    }
}
