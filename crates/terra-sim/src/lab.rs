//! Lab scenarios (design §7.1): a world, run for any seed, with the applied
//! actions and the deaths in each window counted. They're both tests and
//! the main tool for tuning. This module reads a scenario from its text;
//! only the lab example reads files.

use std::collections::{BTreeMap, BTreeSet};

use ron::extensions::Extensions;
use serde::Deserialize;

use crate::action::Outcome;
use crate::brain::{Learned, Thing};
use crate::command::Command;
use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::events::{DeathCause, EventKind};
use crate::map::{Map, MapError, Pos};
use crate::objects::EntityId;
use crate::registry::Verb;
use crate::world::{Scenario, ScenarioError, World};

/// A lab scenario, checked.
#[derive(Debug, Clone)]
pub struct LabScenario {
    world: LabWorld,
    ticks: u64,
    windows: Vec<(u64, u64)>,
    control: Control,
    trainer: Option<Trainer>,
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
}

/// What a control run is without (design §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Without {
    /// Learning, switched off world-wide.
    Learning,
    /// The trainer.
    Trainer,
}

/// One of the Cursor's four touches (design v21 §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum CursorTouch {
    Pet,
    /// An amplified pet.
    Hug,
    Zap,
    /// An amplified zap.
    Shock,
}

impl CursorTouch {
    /// The touch an event reports, if it reports one.
    fn of(kind: &EventKind) -> Option<CursorTouch> {
        match *kind {
            EventKind::Rewarded { amplified, .. } => {
                Some(if amplified { Self::Hug } else { Self::Pet })
            }
            EventKind::Corrected { amplified, .. } => {
                Some(if amplified { Self::Shock } else { Self::Zap })
            }
            _ => None,
        }
    }

    /// The command that gives `sprite` this touch, a pet or hug looking
    /// `reach_back` ticks back.
    fn command(self, sprite: EntityId, reach_back: u16) -> Command {
        let amplified = matches!(self, Self::Hug | Self::Shock);
        match self {
            Self::Pet | Self::Hug => Command::Reward {
                sprite,
                amplified,
                reach_back,
            },
            Self::Zap | Self::Shock => Command::Correct { sprite, amplified },
        }
    }

    /// Its name in a report.
    fn name(self) -> &'static str {
        match self {
            Self::Pet => "pet",
            Self::Hug => "hug",
            Self::Zap => "zap",
            Self::Shock => "shock",
        }
    }
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
    reach_back: u16,
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
    reach_back: Option<u16>,
    /// It answers actions before this tick.
    until: u64,
}

/// A lab scenario file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LabFile {
    world: LabWorld,
    ticks: u64,
    /// A control run of each seed, if any.
    #[serde(default)]
    control: Control,
    /// A trainer, if any (design v21 §7.1).
    #[serde(default)]
    trainer: Option<TrainerFile>,
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
        }
    }
}

impl std::error::Error for LabError {}

/// What one seed's run counted in each window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabRun {
    pub windows: Vec<Window>,
    /// The control run's windows, if the scenario asks for one.
    pub control: Option<Vec<Window>>,
    /// What the control run is without, if there is one.
    pub without: Option<Without>,
}

/// What happened in one window of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl Window {
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
                let correcting = matches!(trainer.give, CursorTouch::Zap | CursorTouch::Shock);
                if correcting && trainer.reach_back.is_some() {
                    return Err(LabError::ReachBackOnCorrect);
                }
                let (verb, name) = trainer.on;
                let index = data
                    .object_type_named(&name)
                    .ok_or(LabError::UnknownTarget(name))?;
                let touch_window = data.physiology().touch_window;
                Ok(Trainer {
                    verb,
                    target: data.object_types()[index].id,
                    give: trainer.give,
                    delay: trainer.delay,
                    reach_back: trainer
                        .reach_back
                        .unwrap_or(u16::try_from(touch_window).unwrap_or(u16::MAX)),
                    until: trainer.until,
                })
            })
            .transpose()?;
        if file.control == Control::NoTrainer && trainer.is_none() {
            return Err(LabError::NoTrainer);
        }
        let lab = LabScenario {
            world: file.world,
            ticks: file.ticks,
            windows: file.windows,
            control: file.control,
            trainer,
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
    fn count(&self, mut world: World, trainer: Option<Trainer>) -> Vec<Window> {
        let mut windows: Vec<Window> = self
            .windows
            .iter()
            .map(|&(from, to)| Window {
                from,
                to,
                applied: BTreeMap::new(),
                deaths: BTreeMap::new(),
                touches: BTreeMap::new(),
                lessons: BTreeMap::new(),
            })
            .collect();
        // The trainer's commands, each with the tick it's submitted before.
        let mut due: Vec<(u64, Command)> = Vec::new();
        for _ in 0..self.ticks {
            let tick = world.tick();
            for (_, command) in due.extract_if(.., |(at, _)| *at == tick) {
                world.submit(command);
            }
            let events = world.step();
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
                            if let Some(touch) = CursorTouch::of(kind) {
                                *window.touches.entry(touch).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }
        windows
    }

    /// The scenario's world for `seed`. A drawn one gets its objects in
    /// reading order, then its sprites.
    fn world(&self, data: DataPack, seed: u64) -> Result<World, LabError> {
        let (rows, key) = match &self.world {
            LabWorld::Generated => {
                let config = WorldConfig::builtin(&data);
                return Ok(World::new(config, data, seed));
            }
            LabWorld::Drawn { rows, key } => (rows, key),
        };
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
        };
        World::from_scenario(scenario, data, seed).map_err(LabError::Scenario)
    }
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
        let applied: BTreeSet<(Verb, Option<u16>)> = windows
            .iter()
            .flat_map(|w| w.applied.keys().copied())
            .collect();
        for key in applied {
            let counts = windows
                .iter()
                .map(|w| w.applied.get(&key).copied().unwrap_or(0));
            out.push_str(&row(&applied_name(key, data), counts));
        }
        let causes: BTreeSet<DeathCause> = windows
            .iter()
            .flat_map(|w| w.deaths.keys().copied())
            .collect();
        for cause in causes {
            let counts = windows
                .iter()
                .map(|w| w.deaths.get(&cause).copied().unwrap_or(0));
            out.push_str(&row(&format!("died: {}", cause_name(cause, data)), counts));
        }
        let touches: BTreeSet<CursorTouch> = windows
            .iter()
            .flat_map(|w| w.touches.keys().copied())
            .collect();
        for touch in touches {
            let counts = windows
                .iter()
                .map(|w| w.touches.get(&touch).copied().unwrap_or(0));
            out.push_str(&row(&format!("given: {}", touch.name()), counts));
        }
        let lessons: BTreeSet<&(Learned, bool)> =
            windows.iter().flat_map(|w| w.lessons.keys()).collect();
        for lesson in lessons {
            let counts = windows
                .iter()
                .map(|w| w.lessons.get(lesson).copied().unwrap_or(0));
            out.push_str(&row(&format!("lesson: {}", lesson_name(lesson)), counts));
        }
    }
    out
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
        Thing::Sprite(id) => format!("sprite #{}", id.0),
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
