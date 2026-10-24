//! Lab scenarios (design §7.1): a world, run for any seed, with the applied
//! actions and the deaths in each window counted. They're both tests and
//! the main tool for tuning. This module reads a scenario from its text;
//! only the lab example reads files.

use std::collections::{BTreeMap, BTreeSet};

use ron::extensions::Extensions;
use serde::Deserialize;

use crate::action::Outcome;
use crate::ageing::LifeStage;
use crate::brain::{Learned, Thing};
use crate::command::{Command, CursorTouch};
use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::events::{DeathCause, Event, EventKind};
use crate::map::{Map, MapError, Pos};
use crate::objects::EntityId;
use crate::registry::Verb;
use crate::world::{Scenario, ScenarioError, World};

/// A lab scenario, checked.
#[derive(Debug, Clone)]
pub struct LabScenario {
    world: LabWorld,
    grows_old: Option<f32>,
    most_sprites: Option<u16>,
    ticks: u64,
    windows: Vec<(u64, u64)>,
    control: Control,
    trainer: Option<Trainer>,
    young: Option<Young>,
}

/// Young for a scenario's sprites (M2 design §18.6): on the tick `at`, each
/// sprite in the world gets a newborn of its own beside it.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Young {
    at: u64,
}

/// A second run of each seed to compare with, if any (design §7.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
enum Control {
    /// Just the learning run.
    #[default]
    Off,
    /// The same world with learning switched off.
    NoLearning,
    /// The same world without the trainer, learning still on (design v21
    /// §7.3).
    NoTrainer,
    /// The same world with the grown-ups taken away as their young appear,
    /// so the young are orphans (M2 design §18.6).
    Orphans,
}

/// What a control run is without (design §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Without {
    /// Learning, switched off world-wide.
    Learning,
    /// The trainer.
    Trainer,
    /// The young's parents (M2 design §18.6).
    Parents,
}

/// A lab scenario's trainer (design v21 §7.1), checked: it answers each
/// applied `verb` on an object of the type `target` with `give` for the one
/// who did it, `delay` ticks later, for actions before `until`.
#[derive(Debug, Clone, Copy)]
struct Trainer {
    verb: Verb,
    /// The object type's stable ID.
    target: u16,
    give: CursorTouch,
    delay: u64,
    reach_back: u64,
    until: u64,
}

/// A trainer in a lab scenario file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrainerFile {
    /// The action it answers: a verb, on an object type by name.
    on: (Verb, String),
    give: CursorTouch,
    /// How many ticks it waits after the action's tick; with none, its
    /// command is stamped for the tick after.
    #[serde(default)]
    delay: u64,
    /// The reach back its pets or hugs carry; the pack's touch window if
    /// left out. A zap or shock takes none (design v21 §5.6).
    #[serde(default)]
    reach_back: Option<u64>,
    /// It answers actions before this tick.
    until: u64,
}

/// A lab scenario file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LabFile {
    world: LabWorld,
    /// The age its drawn sprites grow old at, in place of what their genes
    /// say: for a scenario that watches the same sprites for longer than a
    /// life (M2 design §2), so they don't die of old age partway.
    #[serde(default)]
    grows_old: Option<f32>,
    /// The most sprites its drawn world holds, eggs included, in place of
    /// the default (M2 design §10): its own sprites alone, for a scenario
    /// that watches only them.
    #[serde(default)]
    most_sprites: Option<u16>,
    ticks: u64,
    /// A control run of each seed, if any.
    #[serde(default)]
    control: Control,
    /// A trainer, if any (design v21 §7.1).
    #[serde(default)]
    trainer: Option<TrainerFile>,
    /// Young for its sprites, if any (M2 design §18.6).
    #[serde(default)]
    young: Option<Young>,
    /// Tick ranges, each from its first tick up to but not including its last.
    windows: Vec<(u64, u64)>,
}

/// The world a lab scenario runs.
#[derive(Debug, Clone, Deserialize)]
enum LabWorld {
    /// A hand-drawn map: rows in the ascii theme's terrain glyphs, and a key
    /// for what else a glyph stands for, on grass.
    Drawn {
        rows: Vec<String>,
        key: BTreeMap<char, Placed>,
    },
    /// A world generated from the built-in default preset.
    Generated,
    /// The unattended run's world (M2 design §19.6): the built-in default
    /// preset's world as generated, with its first sprites newborns spread
    /// over the whole map, and nobody else.
    Unattended,
}

/// What a key's glyph puts on a tile.
#[derive(Debug, Clone, Deserialize)]
enum Placed {
    /// An object of this type, at the start of its first stage.
    Object(String),
    /// A sprite of the starter genome, with spawn variation.
    Sprite,
}

/// Why a lab scenario can't be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabError {
    /// The text is not valid RON for a lab scenario.
    Parse(String),
    /// A window that's empty, or runs past the end of the run.
    BadWindow { from: u64, to: u64 },
    /// The drawn map isn't a valid map.
    Map(MapError),
    /// Something in the key can't go where it's drawn.
    Scenario(ScenarioError),
    /// The trainer answers actions on an object type the pack doesn't have.
    UnknownTarget(String),
    /// A control run without the trainer, in a scenario with no trainer.
    NoTrainer,
    /// A trainer that zaps or shocks, given a reach back: a Correct always
    /// looks back the touch window (design v21 §5.6).
    ReachBackOnCorrect,
    /// A control run of orphans, in a scenario that gives its sprites no
    /// young.
    NoYoung,
    /// Young that appear after the run ends.
    LateYoung(u64),
}

impl std::fmt::Display for LabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabError::Parse(message) => f.write_str(message),
            LabError::BadWindow { from, to } => write!(
                f,
                "the window ({from}, {to}) must be from an earlier tick to a later one, within the run"
            ),
            LabError::Map(error) => write!(f, "the map: {error:?}"),
            LabError::Scenario(error) => write!(f, "the world: {error:?}"),
            LabError::UnknownTarget(name) => {
                write!(
                    f,
                    "the trainer answers actions on `{name}`, which isn't an object type"
                )
            }
            LabError::NoTrainer => {
                f.write_str("the control run is without the trainer, but there's no trainer")
            }
            LabError::ReachBackOnCorrect => f.write_str(
                "a trainer that zaps or shocks takes no reach_back: a zap or shock always looks back the touch window",
            ),
            LabError::NoYoung => {
                f.write_str("the control run is of orphans, but the sprites are given no young")
            }
            LabError::LateYoung(at) => {
                write!(f, "the young appear at tick {at}, after the run ends")
            }
        }
    }
}

impl std::error::Error for LabError {}

/// What one seed's run counted in each window.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LabRun {
    pub windows: Vec<Window>,
    /// The control run's windows, if the scenario asks for one.
    pub control: Option<Vec<Window>>,
    /// What the control run is without, if there is one.
    pub without: Option<Without>,
}

/// What happened in one window of a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Window {
    pub from: u64,
    pub to: u64,
    /// Actions that ended `applied`, by verb and the stable ID of their
    /// target's object type (`None` for Rest and Wander).
    pub applied: BTreeMap<(Verb, Option<u16>), u64>,
    /// Deaths, by cause.
    pub deaths: BTreeMap<DeathCause, u64>,
    /// The Cursor's touches, such as a trainer's pets (design v21 §7.1).
    pub touches: BTreeMap<CursorTouch, u64>,
    /// Lessons learned (design §5.6), each as what was learned and whether
    /// it rose.
    pub lessons: BTreeMap<(Learned, bool), u64>,
    /// What the young the scenario gives its sprites did and learned, once
    /// they've appeared (M2 design §18.6), if it gives them any.
    pub young: Option<YoungCounts>,
    /// What became of the world's first sprites that started young (M2
    /// design §19.6).
    pub first: Childhood,
    /// What became of the young that appeared after the start: hatched, or
    /// given as the scenario's young.
    pub later: Childhood,
    /// Deaths, by the life stage each died at and their cause (M2 design
    /// §19.6): a grown-up death is an adult's or an old sprite's.
    pub stage_deaths: BTreeMap<(LifeStage, DeathCause), u64>,
    /// How many of the world's first sprites died, at any life stage.
    pub first_died: u64,
}

/// How many young grew up in a window, and how many died young (M2 design
/// §19.6). A young sprite still growing at the window's end is in neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Childhood {
    pub grew_up: u64,
    pub died_young: u64,
}

impl Childhood {
    /// The share of those it counts that grew up, or none if it counts none.
    pub fn share(&self) -> Option<f64> {
        let of = self.grew_up + self.died_young;
        (of > 0).then(|| self.grew_up as f64 / of as f64)
    }
}

/// What became of a sprite in a tick (M2 design §19.6): `first` if it was
/// one of the world's first sprites.
#[derive(Debug, Clone, Copy)]
enum Fate {
    GrewUp {
        first: bool,
    },
    /// It died at `stage`, of `cause`.
    Died {
        first: bool,
        stage: LifeStage,
        cause: DeathCause,
    },
}

/// What a window counted of the young a scenario gives its sprites (M2
/// design §18.6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YoungCounts {
    /// How many there are.
    pub young: u64,
    /// Their actions that ended `applied`, as `Window::applied` counts
    /// everyone's.
    pub applied: BTreeMap<(Verb, Option<u16>), u64>,
    /// Their deaths, by cause.
    pub deaths: BTreeMap<DeathCause, u64>,
    /// How many of them had learned each lesson by the window's end: about
    /// things and sprites in general, not particular sprites.
    pub learned: BTreeMap<(Learned, bool), u64>,
    /// For each of those lessons, the median of the ticks each took to
    /// learn it from when it appeared: one that hadn't by the window's end
    /// counts the ticks to then, so a lesson most never learn shows as long
    /// as the window allows.
    pub learned_after: BTreeMap<(Learned, bool), u64>,
    /// The ticks from when they appeared to the window's end: what a lesson
    /// none of them learned counts.
    pub ticks: u64,
}

impl Window {
    /// Counts what became of a sprite in one of the window's ticks.
    fn record(&mut self, fate: Fate) {
        match fate {
            Fate::GrewUp { first } => self.childhood(first).grew_up += 1,
            Fate::Died {
                first,
                stage,
                cause,
            } => {
                *self.stage_deaths.entry((stage, cause)).or_insert(0) += 1;
                self.first_died += u64::from(first);
                if stage == LifeStage::Young {
                    self.childhood(first).died_young += 1;
                }
            }
        }
    }

    /// What it counts of the first sprites' childhoods if `first`, or else
    /// of those that appeared later.
    fn childhood(&mut self, first: bool) -> &mut Childhood {
        if first {
            &mut self.first
        } else {
            &mut self.later
        }
    }

    /// How many `verb`s aimed at an object of type `target` applied.
    pub fn applied_on(&self, verb: Verb, target: &str, data: &DataPack) -> u64 {
        self.applied
            .iter()
            .filter(|((v, t), _)| {
                *v == verb && t.and_then(|t| data.object_type_name(t)) == Some(target)
            })
            .map(|(_, &n)| n)
            .sum()
    }
}

impl LabScenario {
    /// Reads a lab scenario from its RON text, checking it against `data`.
    pub fn from_ron(text: &str, data: &DataPack) -> Result<LabScenario, LabError> {
        // `implicit_some` lets optional fields be written as plain values:
        // `trainer: (on: (Play, "ball"), …)`.
        let file: LabFile = ron::Options::default()
            .with_default_extension(Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|e| LabError::Parse(e.to_string()))?;
        if let Some(&(from, to)) = file
            .windows
            .iter()
            .find(|&&(from, to)| from >= to || to > file.ticks)
        {
            return Err(LabError::BadWindow { from, to });
        }
        let trainer = file
            .trainer
            .map(|trainer| {
                if trainer.give.corrects() && trainer.reach_back.is_some() {
                    return Err(LabError::ReachBackOnCorrect);
                }
                let (verb, name) = trainer.on;
                let index = data
                    .object_type_named(&name)
                    .ok_or(LabError::UnknownTarget(name))?;
                Ok(Trainer {
                    verb,
                    target: data.object_types()[index].id,
                    give: trainer.give,
                    delay: trainer.delay,
                    reach_back: trainer.reach_back.unwrap_or(data.physiology().touch_window),
                    until: trainer.until,
                })
            })
            .transpose()?;
        if file.control == Control::NoTrainer && trainer.is_none() {
            return Err(LabError::NoTrainer);
        }
        if file.control == Control::Orphans && file.young.is_none() {
            return Err(LabError::NoYoung);
        }
        if let Some(young) = file.young.filter(|young| young.at >= file.ticks) {
            return Err(LabError::LateYoung(young.at));
        }
        let lab = LabScenario {
            world: file.world,
            grows_old: file.grows_old,
            most_sprites: file.most_sprites,
            ticks: file.ticks,
            windows: file.windows,
            control: file.control,
            trainer,
            young: file.young,
        };
        // Building it once finds anything that can't go where it's drawn.
        lab.world(data.clone(), 0)?;
        Ok(lab)
    }

    /// Runs the scenario for `seed`, counting each window.
    pub fn run(&self, data: DataPack, seed: u64) -> LabRun {
        let (control, without) = match self.control {
            Control::Off => (None, None),
            Control::NoLearning => {
                let mut world = self.world(data.clone(), seed).expect("checked when read");
                world.switch_off_learning();
                let windows = self.count(world, self.trainer);
                (Some(windows), Some(Without::Learning))
            }
            Control::NoTrainer => {
                let world = self.world(data.clone(), seed).expect("checked when read");
                (Some(self.count(world, None)), Some(Without::Trainer))
            }
            Control::Orphans => {
                let world = self.world(data.clone(), seed).expect("checked when read");
                let windows = self.counting(world, self.trainer, true);
                (Some(windows), Some(Without::Parents))
            }
        };
        let world = self.world(data, seed).expect("checked when read");
        LabRun {
            windows: self.count(world, self.trainer),
            control,
            without,
        }
    }

    /// Runs `world` for the scenario's ticks with `trainer`, if any,
    /// counting each window.
    fn count(&self, world: World, trainer: Option<Trainer>) -> Vec<Window> {
        self.counting(world, trainer, false)
    }

    /// Runs `world` for the scenario's ticks with `trainer`, if any,
    /// counting each window; with the scenario's young, if it gives any,
    /// and their parents taken away as they appear if `orphans`.
    fn counting(&self, mut world: World, trainer: Option<Trainer>, orphans: bool) -> Vec<Window> {
        let mut windows: Vec<Window> = self
            .windows
            .iter()
            .map(|&(from, to)| Window {
                from,
                to,
                ..Default::default()
            })
            .collect();
        // The world's first sprites, and each sprite's life stage, to count
        // what became of the young and at what stage each died (M2 design
        // §19.6).
        let first: BTreeSet<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
        let mut stages: BTreeMap<EntityId, LifeStage> = world
            .sprites()
            .map(|sprite| (sprite.id(), sprite.life_stage()))
            .collect();
        // The young, by when each appeared, and when each learned each
        // lesson (M2 design §18.6).
        let mut young: BTreeMap<EntityId, u64> = BTreeMap::new();
        let mut learned: BTreeMap<(EntityId, (Learned, bool)), u64> = BTreeMap::new();
        // The trainer's commands, each with the tick it's submitted before.
        let mut due: Vec<(u64, Command)> = Vec::new();
        for _ in 0..self.ticks {
            let tick = world.tick();
            for (_, command) in due.extract_if(.., |(at, _)| *at == tick) {
                world.submit(command);
            }
            if self.young.is_some_and(|young| young.at == tick) {
                let grown: Vec<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
                let given = world.give_young();
                stages.extend(given.iter().map(|&id| (id, LifeStage::Young)));
                young.extend(given.into_iter().map(|id| (id, tick)));
                if orphans {
                    world.take_away(&grown);
                }
            }
            let events = world.step();
            let fates = fates(&events, &mut stages, &first);
            for event in &events {
                if let EventKind::LearnedMilestone {
                    id,
                    learned: l,
                    good,
                    ..
                } = &event.kind
                    && young.contains_key(id)
                    && !about_a_sprite(l)
                {
                    learned.entry((*id, (l.clone(), *good))).or_insert(tick);
                }
            }
            if let Some(trainer) = trainer.filter(|t| tick < t.until) {
                for event in &events {
                    if let EventKind::ActionEnded {
                        id,
                        verb,
                        outcome: Outcome::Applied,
                        action,
                    } = &event.kind
                        && *verb == trainer.verb
                        && action.target_type == Some(trainer.target)
                    {
                        let command = trainer.give.command(*id, trainer.reach_back);
                        due.push((tick + 1 + trainer.delay, command));
                    }
                }
            }
            for window in windows
                .iter_mut()
                .filter(|w| (w.from..w.to).contains(&tick))
            {
                for event in &events {
                    match &event.kind {
                        EventKind::ActionEnded {
                            verb,
                            outcome: Outcome::Applied,
                            action,
                            ..
                        } => {
                            *window
                                .applied
                                .entry((*verb, action.target_type))
                                .or_insert(0) += 1
                        }
                        EventKind::Died { cause, .. } => {
                            *window.deaths.entry(*cause).or_insert(0) += 1;
                        }
                        EventKind::LearnedMilestone { learned, good, .. } => {
                            *window.lessons.entry((learned.clone(), *good)).or_insert(0) += 1;
                        }
                        kind => {
                            if let Some(touch) = CursorTouch::reported(kind) {
                                *window.touches.entry(touch).or_insert(0) += 1;
                            }
                        }
                    }
                }
                for &fate in &fates {
                    window.record(fate);
                }
                if !young.is_empty() {
                    let counts = window.young.get_or_insert_with(YoungCounts::default);
                    for event in &events {
                        match &event.kind {
                            EventKind::ActionEnded {
                                id,
                                verb,
                                outcome: Outcome::Applied,
                                action,
                            } if young.contains_key(id) => {
                                *counts
                                    .applied
                                    .entry((*verb, action.target_type))
                                    .or_insert(0) += 1
                            }
                            EventKind::Died { id, cause, .. } if young.contains_key(id) => {
                                *counts.deaths.entry(*cause).or_insert(0) += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        for window in &mut windows {
            if let Some(counts) = window.young.as_mut() {
                young_lessons(counts, &young, &learned, window.to);
            }
        }
        windows
    }

    /// The scenario's world for `seed`. A drawn one gets its objects in
    /// reading order, then its sprites.
    fn world(&self, data: DataPack, seed: u64) -> Result<World, LabError> {
        let mut world = match &self.world {
            LabWorld::Generated => {
                let config = WorldConfig::builtin(&data);
                World::new(config, data, seed)
            }
            LabWorld::Unattended => {
                let config = WorldConfig::builtin(&data);
                World::unattended(config, data, seed)
            }
            LabWorld::Drawn { rows, key } => drawn(rows, key, data, seed)?,
        };
        if let Some(age) = self.grows_old {
            world.set_grows_old(age);
        }
        if let Some(most) = self.most_sprites {
            world.set_most_sprites(most);
        }
        Ok(world)
    }
}

/// What became of each sprite whose life stage changed or ended in a tick's
/// `events`, in order, keeping `stages`, each sprite's life stage, up to
/// date; `first` holds the world's first sprites (M2 design §19.6).
fn fates(
    events: &[Event],
    stages: &mut BTreeMap<EntityId, LifeStage>,
    first: &BTreeSet<EntityId>,
) -> Vec<Fate> {
    let mut fates = Vec::new();
    for event in events {
        match &event.kind {
            EventKind::Hatched { id, .. } | EventKind::Spawned { id, .. } => {
                stages.insert(*id, LifeStage::Young);
            }
            EventKind::GrewUp { id } => {
                stages.insert(*id, LifeStage::Adult);
                let first = first.contains(id);
                fates.push(Fate::GrewUp { first });
            }
            EventKind::GrewOld { id } => {
                stages.insert(*id, LifeStage::Old);
            }
            EventKind::Died { id, cause, .. } => {
                let stage = stages.remove(id).unwrap_or_default();
                let (first, cause) = (first.contains(id), *cause);
                fates.push(Fate::Died {
                    first,
                    stage,
                    cause,
                });
            }
            _ => {}
        }
    }
    fates
}

/// Fills in `counts` with how many of `young`, each by the tick it
/// appeared, had learned each lesson by the tick `to`, by when each learned
/// it in `learned`, and the median ticks each took (M2 design §18.6).
fn young_lessons(
    counts: &mut YoungCounts,
    young: &BTreeMap<EntityId, u64>,
    learned: &BTreeMap<(EntityId, (Learned, bool)), u64>,
    to: u64,
) {
    counts.young = young.len() as u64;
    let appeared = young.values().copied().min().unwrap_or(to);
    counts.ticks = to.saturating_sub(appeared);
    let lessons: BTreeSet<&(Learned, bool)> = learned
        .iter()
        .filter(|&(_, &tick)| tick < to)
        .map(|((_, lesson), _)| lesson)
        .collect();
    for lesson in lessons {
        let took: Vec<u64> = young
            .iter()
            .map(|(&id, &appeared)| {
                let by = learned
                    .get(&(id, lesson.clone()))
                    .copied()
                    .filter(|&tick| tick < to);
                by.unwrap_or(to).saturating_sub(appeared)
            })
            .collect();
        let knew = learned
            .iter()
            .filter(|&((_, l), &tick)| l == lesson && tick < to)
            .count();
        counts.learned.insert(lesson.clone(), knew as u64);
        counts
            .learned_after
            .insert(lesson.clone(), median(&took) as u64);
    }
}

/// Whether `learned` is about a particular sprite.
fn about_a_sprite(learned: &Learned) -> bool {
    match learned {
        Learned::Worth { thing, .. }
        | Learned::Bad { thing }
        | Learned::Fear { thing }
        | Learned::Habit { thing, .. } => matches!(thing, Thing::Sprite(_)),
        Learned::NewThings => false,
    }
}

/// The world drawn in `rows`, by `key`, from `seed`: its objects in
/// reading order, then its sprites.
fn drawn(
    rows: &[String],
    key: &BTreeMap<char, Placed>,
    data: DataPack,
    seed: u64,
) -> Result<World, LabError> {
    let mut objects: Vec<(Pos, &str)> = Vec::new();
    let mut sprites = Vec::new();
    let mut terrain: Vec<String> = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        let mut drawn = String::new();
        for (x, glyph) in row.chars().enumerate() {
            let pos = Pos {
                x: x as u16,
                y: y as u16,
            };
            match key.get(&glyph) {
                Some(Placed::Object(name)) => objects.push((pos, name)),
                Some(Placed::Sprite) => sprites.push((pos, None)),
                None => {
                    drawn.push(glyph);
                    continue;
                }
            }
            drawn.push('.');
        }
        terrain.push(drawn);
    }
    let terrain: Vec<&str> = terrain.iter().map(String::as_str).collect();
    let map = Map::from_ascii(&terrain, &data).map_err(LabError::Map)?;
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &sprites,
        scripted: &[],
        newborns: &[],
    };
    World::from_scenario(scenario, data, seed).map_err(LabError::Scenario)
}

/// Each window of `runs`, one per seed, as a table: a row for each count
/// any seed has, a column for each seed, and the median; then the same for
/// the control runs, if there are any.
pub fn report(runs: &[(u64, LabRun)], data: &DataPack) -> String {
    let learning: Vec<(u64, &[Window])> = runs
        .iter()
        .map(|(seed, run)| (*seed, run.windows.as_slice()))
        .collect();
    let mut out = windows_report(&learning, data);
    let controls: Option<Vec<(u64, &[Window])>> = runs
        .iter()
        .map(|(seed, run)| run.control.as_deref().map(|windows| (*seed, windows)))
        .collect();
    let without = runs.first().and_then(|(_, run)| run.without);
    if let (Some(controls), Some(without)) = (controls.filter(|c| !c.is_empty()), without) {
        out.push_str(match without {
            Without::Learning => "\ncontrol, without learning\n",
            Without::Trainer => "\ncontrol, without the trainer\n",
            Without::Parents => {
                "\ncontrol, orphans: the grown-ups taken away as their young appear\n"
            }
        });
        out.push_str(&windows_report(&controls, data));
    }
    out
}

/// Each window of `runs`, with a column per seed and the median.
fn windows_report(runs: &[(u64, &[Window])], data: &DataPack) -> String {
    let mut out = String::new();
    let Some((_, first)) = runs.first() else {
        return out;
    };
    for (w, window) in first.iter().enumerate() {
        let windows: Vec<&Window> = runs.iter().map(|(_, run)| &run[w]).collect();
        let head = format!(
            "ticks {} to {}",
            thousands(window.from),
            thousands(window.to)
        );
        out.push_str(&format!("{head:<26}"));
        for (seed, _) in runs {
            out.push_str(&format!("{:>10}", format!("seed {seed}")));
        }
        out.push_str(&format!("{:>10}\n", "median"));
        out.push_str(&rows(
            &windows,
            |w| &w.applied,
            |&key| applied_name(key, data),
        ));
        out.push_str(&rows(
            &windows,
            |w| &w.deaths,
            |&cause| format!("died: {}", cause_name(cause, data)),
        ));
        out.push_str(&childhood_rows(&windows));
        out.push_str(&rows(
            &windows,
            |w| &w.stage_deaths,
            |&(stage, cause)| format!("died, {}: {}", stage.name(), cause_name(cause, data)),
        ));
        out.push_str(&rows(
            &windows,
            |w| &w.touches,
            |touch| format!("given: {}", touch.name()),
        ));
        out.push_str(&rows(
            &windows,
            |w| &w.lessons,
            |lesson| format!("lesson: {}", lesson_name(lesson)),
        ));
        out.push_str(&young_rows(&windows, data));
    }
    out
}

/// The rows for what became of the young (M2 design §19.6), the world's
/// first sprites and those that appeared later apart, where any of
/// `windows` counted any.
fn childhood_rows(windows: &[&Window]) -> String {
    let mut out = String::new();
    for whose in ["first", "later"] {
        let of = |w: &Window| if whose == "first" { w.first } else { w.later };
        if windows.iter().all(|&w| of(w) == Childhood::default()) {
            continue;
        }
        let grew_up = windows.iter().map(|&w| of(w).grew_up);
        out.push_str(&row(&format!("grew up: {whose}"), grew_up));
        let died_young = windows.iter().map(|&w| of(w).died_young);
        out.push_str(&row(&format!("died young: {whose}"), died_young));
    }
    out
}

/// The rows for the young a scenario gives its sprites (M2 design §18.6),
/// if any of `windows` counted them: how many, what they did, how they
/// died, how many learned each lesson, and the median ticks they took.
fn young_rows(windows: &[&Window], data: &DataPack) -> String {
    if windows.iter().all(|w| w.young.is_none()) {
        return String::new();
    }
    let none = YoungCounts::default();
    let young: Vec<&YoungCounts> = windows
        .iter()
        .map(|w| w.young.as_ref().unwrap_or(&none))
        .collect();
    let mut out = row("young", young.iter().map(|y| y.young));
    out.push_str(&rows(
        &young,
        |y| &y.applied,
        |&key| format!("young: {}", applied_name(key, data)),
    ));
    out.push_str(&rows(
        &young,
        |y| &y.deaths,
        |&cause| format!("young died: {}", cause_name(cause, data)),
    ));
    out.push_str(&rows(
        &young,
        |y| &y.learned,
        |lesson| format!("young learned: {}", lesson_name(lesson)),
    ));
    // A seed none of whose young learned a lesson took the whole window.
    let lessons: BTreeSet<&(Learned, bool)> =
        young.iter().flat_map(|y| y.learned_after.keys()).collect();
    for lesson in lessons {
        let took = young
            .iter()
            .map(|y| y.learned_after.get(lesson).copied().unwrap_or(y.ticks));
        out.push_str(&row(
            &format!("ticks to learn: {}", lesson_name(lesson)),
            took,
        ));
    }
    out
}

/// A report row, named by `name`, for each key any of `windows` counts in
/// the counts `of` picks out, in key order.
fn rows<T, K: Ord>(
    windows: &[&T],
    of: impl Fn(&T) -> &BTreeMap<K, u64>,
    name: impl Fn(&K) -> String,
) -> String {
    let keys: BTreeSet<&K> = windows.iter().flat_map(|w| of(w).keys()).collect();
    keys.into_iter()
        .map(|key| {
            let counts = windows.iter().map(|w| of(w).get(key).copied().unwrap_or(0));
            row(&name(key), counts)
        })
        .collect()
}

/// A lesson in a report: what was learned, and which way it went.
fn lesson_name((learned, rose): &(Learned, bool)) -> String {
    let good = if *rose { "good" } else { "bad" };
    match learned {
        Learned::Worth {
            thing,
            need: Some(need),
        } => format!("{} is good for {need}", thing_name(thing)),
        Learned::Worth { thing, need: None } => format!("{} is good", thing_name(thing)),
        Learned::Bad { thing } => format!("{} is bad", thing_name(thing)),
        Learned::Fear { thing } => format!("{} is frightening", thing_name(thing)),
        Learned::Habit { thing, verb } => {
            let verb = format!("{verb:?}").to_lowercase();
            format!("{verb} {} is {good}", thing_name(thing))
        }
        Learned::NewThings => format!("new things are {good}"),
    }
}

/// What a lesson is about, in a report.
fn thing_name(thing: &Thing) -> String {
    match thing {
        Thing::ObjectType(name) => name.clone(),
        Thing::Category(name) => format!("{name} in general"),
        Thing::Sprite(id) => format!("Sprite #{}", id.0),
        Thing::Cursor => "the Cursor".into(),
    }
}

/// One row of a report: `name`, each seed's count, and their median.
fn row(name: &str, counts: impl Iterator<Item = u64>) -> String {
    let counts: Vec<u64> = counts.collect();
    let mut line = format!("  {name:<24}");
    for count in &counts {
        line.push_str(&format!("{count:>10}"));
    }
    line.push_str(&format!("{:>10}\n", number(median(&counts))));
    line
}

/// The median of `counts`: the middle one, or the mean of the middle two.
pub fn median(counts: &[u64]) -> f64 {
    let mut sorted = counts.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    match n {
        0 => 0.0,
        _ if n % 2 == 1 => sorted[n / 2] as f64,
        _ => (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0,
    }
}

/// A median as the report writes it: whole, or with its half.
fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// An applied count's name: the verb, and its target's object type.
fn applied_name((verb, target): (Verb, Option<u16>), data: &DataPack) -> String {
    let verb = format!("{verb:?}").to_lowercase();
    match target.and_then(|t| data.object_type_name(t)) {
        Some(target) => format!("{verb} {target}"),
        None => verb,
    }
}

/// A cause of death by name: "hurt by thornbush" names the object type.
fn cause_name(cause: DeathCause, data: &DataPack) -> String {
    match cause {
        DeathCause::Starvation => "starvation".into(),
        DeathCause::Dehydration => "dehydration".into(),
        DeathCause::OldAge => "old age".into(),
        DeathCause::HurtBy(id) => format!("hurt by {}", data.object_type_name(id).unwrap_or("?")),
    }
}

/// `5000` → `"5,000"`.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window with nothing counted yet.
    fn empty() -> Window {
        Window {
            from: 0,
            to: 2,
            ..Default::default()
        }
    }

    /// The sprite `id` died of `cause`.
    fn died(id: u64, cause: DeathCause) -> EventKind {
        EventKind::Died {
            id: EntityId(id),
            name: None,
            cause,
            age: 0,
        }
    }

    #[test]
    fn a_window_counts_childhoods_by_first_or_later_and_deaths_by_life_stage() {
        // M2 design §19.6: sprite 1 is a first sprite that starts young,
        // grows up and dies of thirst as an adult; sprite 2 a first sprite,
        // grown up, that grows old and then dies of hunger, counted apart
        // from adults' hunger; and sprite 3 hatches, and dies young, not
        // counted among the first sprites' deaths.
        let first = BTreeSet::from([EntityId(1), EntityId(2)]);
        let mut stages = BTreeMap::from([
            (EntityId(1), LifeStage::Young),
            (EntityId(2), LifeStage::Adult),
        ]);
        let ticks = [
            vec![
                EventKind::Hatched {
                    id: EntityId(3),
                    parents: vec![EntityId(2)],
                },
                EventKind::GrewUp { id: EntityId(1) },
                EventKind::GrewOld { id: EntityId(2) },
            ],
            vec![
                died(1, DeathCause::Dehydration),
                died(2, DeathCause::Starvation),
                died(3, DeathCause::Dehydration),
            ],
        ];
        let mut window = empty();
        for (tick, kinds) in (0..).zip(ticks) {
            let events: Vec<Event> = kinds.into_iter().map(|kind| Event { tick, kind }).collect();
            for fate in fates(&events, &mut stages, &first) {
                window.record(fate);
            }
        }
        let childhood = |grew_up, died_young| Childhood {
            grew_up,
            died_young,
        };
        assert_eq!(window.first, childhood(1, 0));
        assert_eq!(window.later, childhood(0, 1));
        assert_eq!(
            window.stage_deaths,
            BTreeMap::from([
                ((LifeStage::Young, DeathCause::Dehydration), 1),
                ((LifeStage::Adult, DeathCause::Dehydration), 1),
                ((LifeStage::Old, DeathCause::Starvation), 1),
            ])
        );
        assert_eq!(window.first_died, 2, "sprites 1 and 2, not sprite 3");
        assert!(stages.is_empty(), "every sprite died: {stages:?}");
    }
}
