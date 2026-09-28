//! Lab scenarios (design §7.1): a world, run for any seed, with the applied
//! actions and the deaths in each window counted. They're both tests and
//! the main tool for tuning. This module reads a scenario from its text;
//! only the lab example reads files.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::action::Outcome;
use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::events::{DeathCause, EventKind};
use crate::map::{Map, MapError, Pos};
use crate::registry::Verb;
use crate::world::{Scenario, ScenarioError, World};

/// A lab scenario, checked.
#[derive(Debug, Clone)]
pub struct LabScenario {
    world: LabWorld,
    ticks: u64,
    windows: Vec<(u64, u64)>,
    control: Control,
}

/// A second run of each seed to compare with, if any (design §7.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
enum Control {
    /// Just the learning run.
    #[default]
    Off,
    /// The same world with learning switched off.
    NoLearning,
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
        let file: LabFile = ron::from_str(text).map_err(|e| LabError::Parse(e.to_string()))?;
        if let Some(&(from, to)) = file
            .windows
            .iter()
            .find(|&&(from, to)| from >= to || to > file.ticks)
        {
            return Err(LabError::BadWindow { from, to });
        }
        let lab = LabScenario {
            world: file.world,
            ticks: file.ticks,
            windows: file.windows,
            control: file.control,
        };
        // Building it once finds anything that can't go where it's drawn.
        lab.world(data.clone(), 0)?;
        Ok(lab)
    }

    /// Runs the scenario for `seed`, counting each window.
    pub fn run(&self, data: DataPack, seed: u64) -> LabRun {
        let control = (self.control == Control::NoLearning).then(|| {
            let mut world = self.world(data.clone(), seed).expect("checked when read");
            world.switch_off_learning();
            self.count(world)
        });
        let world = self.world(data, seed).expect("checked when read");
        LabRun {
            windows: self.count(world),
            control,
        }
    }

    /// Runs `world` for the scenario's ticks, counting each window.
    fn count(&self, mut world: World) -> Vec<Window> {
        let mut windows: Vec<Window> = self
            .windows
            .iter()
            .map(|&(from, to)| Window {
                from,
                to,
                applied: BTreeMap::new(),
                deaths: BTreeMap::new(),
            })
            .collect();
        for _ in 0..self.ticks {
            let tick = world.tick();
            let events = world.step();
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
                        _ => {}
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
    if let Some(controls) = controls.filter(|c| !c.is_empty()) {
        out.push_str(
            "
control, without learning
",
        );
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
    }
    out
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
