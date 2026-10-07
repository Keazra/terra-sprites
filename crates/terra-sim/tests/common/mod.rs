//! Shared helpers for terra-sim integration tests.
//!
//! Each test file is its own crate, so it `mod common;` and uses only the
//! pieces it needs. Helpers a given file never calls are allowed here so the
//! shared API stays one place (#155).
#![allow(dead_code)]

use terra_sim::{
    DataPack, Event, EventKind, Genome, Map, Outcome, Pos, Scenario, ScriptedAction, Verb, World,
};

/// The built-in data pack.
pub fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A position at `(x, y)`.
pub fn at(x: u16, y: u16) -> Pos {
    Pos::new(x, y)
}

/// True when two levels are the same, give or take a little.
pub fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A genome with only traits: speed 10, sense radius **10** — so a script
/// decides what the sprite does. For a different sense radius (e.g. movement
/// tests at 14), call [`traits_only`] instead.
pub fn walker(data: &DataPack) -> Genome {
    traits_only(10.0, 10.0, data)
}

/// A genome with only `speed` and `sense_radius`.
pub fn traits_only(speed: f32, sense_radius: f32, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: {speed:?}),
            Trait(trait: "sense_radius", value: {sense_radius:?}),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A genome of `genes` plus speed and sense 10, with learning that does not
/// fade, so what is learned reads exactly.
pub fn genome(genes: &str, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: 10.0),
            Trait(trait: "sense_radius", value: 10.0),
            {genes}
            // No fading, so what is learned reads exactly.
            BrainParam(param: "worth_fade_good", value: 0.0),
            BrainParam(param: "worth_fade_bad", value: 0.0),
            BrainParam(param: "habit_fade", value: 0.0),
            BrainParam(param: "habit_fade_bad", value: 0.0),
            BrainParam(param: "fear_fade", value: 0.0),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// Every action that ended in `events`, as `(verb, outcome)`.
pub fn endings(events: &[Event]) -> Vec<(Verb, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionEnded { verb, outcome, .. } => Some((verb, outcome)),
            _ => None,
        })
        .collect()
}

/// The built-in data pack, with `changes` made to `file`'s text.
pub fn builtin_changing(file: &str, changes: &[(&str, &str)]) -> DataPack {
    let sources: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let mut text = text.to_string();
            if path == file {
                for &(from, to) in changes {
                    assert!(text.contains(from), "{from:?} is in {file}");
                    text = text.replace(from, to);
                }
            }
            (path, text)
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    DataPack::from_sources(&borrowed).expect("the changed pack is valid")
}

/// Builds a hand-drawn test world: rows, objects, sprites (optional genome),
/// scripted actions, and a seed.
///
/// Sprite and script methods append; [`Self::objects`] replaces.
pub struct WorldBuilder<'a> {
    rows: &'a [&'a str],
    objects: &'a [(Pos, &'a str)],
    sprites: Vec<(Pos, Option<Genome>)>,
    scripted: Vec<(Pos, ScriptedAction)>,
    data: Option<DataPack>,
    seed: u64,
}

impl<'a> WorldBuilder<'a> {
    /// Starts from an ascii drawing of the map.
    pub fn new(rows: &'a [&'a str]) -> Self {
        Self {
            rows,
            objects: &[],
            sprites: Vec::new(),
            scripted: Vec::new(),
            data: None,
            seed: 1,
        }
    }

    /// Objects as `(tile, object type name)` (replaces any previous list).
    pub fn objects(mut self, objects: &'a [(Pos, &'a str)]) -> Self {
        self.objects = objects;
        self
    }

    /// Uses this data pack instead of the built-in one.
    pub fn data(mut self, data: DataPack) -> Self {
        self.data = Some(data);
        self
    }

    /// The world seed (default 1).
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sprites already built as `(pos, optional genome)` (appends).
    pub fn sprites_with(mut self, sprites: Vec<(Pos, Option<Genome>)>) -> Self {
        self.sprites.extend(sprites);
        self
    }

    /// Scripted actions by starting tile (appends).
    pub fn scripts(mut self, scripted: &[(Pos, ScriptedAction)]) -> Self {
        self.scripted.extend_from_slice(scripted);
        self
    }

    /// Builds the world.
    pub fn build(self) -> World {
        let data = self.data.unwrap_or_else(builtin);
        let map = Map::from_ascii(self.rows, &data).expect("valid drawing");
        let scenario = Scenario {
            map,
            objects: self.objects,
            sprites: &self.sprites,
            scripted: &self.scripted,
        };
        World::from_scenario(scenario, data, self.seed).expect("a valid scenario")
    }
}
