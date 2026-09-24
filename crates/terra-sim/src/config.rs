use std::collections::BTreeMap;

use ron::extensions::Extensions;
use serde::Deserialize;

use crate::data::DataPack;
use crate::map::MAX_SIDE;

/// The default preset, embedded at compile time from the repository's `data/presets/`.
const BUILTIN: &str = include_str!("../../../data/presets/default.ron");

/// The allowed range for each side of a generated map, in tiles: room enough
/// for generation to make a mainland, up to the limit for any map.
const SIDE: std::ops::RangeInclusive<u16> = 32..=MAX_SIDE;

/// The settings a new world is made from. A preset file holds one.
///
/// A config is checked against the data pack it was parsed with, and must be
/// used with that pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldConfig {
    width: u16,
    height: u16,
    /// How many of each object type go in every `per_tiles` tiles (design §3.9).
    objects: BTreeMap<String, u32>,
    per_tiles: u32,
}

/// Why a preset could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The text is not valid RON for a preset.
    Parse(String),
    /// The preset parses but breaks a rule.
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Parse(message) | ConfigError::Invalid(message) => f.write_str(message),
        }
    }
}

/// A preset file, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Preset {
    width: u16,
    height: u16,
    #[serde(default)]
    objects: BTreeMap<String, u32>,
    #[serde(default)]
    per_tiles: Option<u32>,
}

impl WorldConfig {
    /// The default preset, `data/presets/default.ron`, for the built-in pack.
    pub fn builtin(data: &DataPack) -> WorldConfig {
        WorldConfig::from_ron(BUILTIN, data).expect("the built-in preset is valid")
    }

    /// Reads a preset from its RON text, checking the object types it names against `data`.
    pub fn from_ron(text: &str, data: &DataPack) -> Result<WorldConfig, ConfigError> {
        // `implicit_some` lets optional fields be written as plain values: `per_tiles: 15360`.
        let preset: Preset = ron::Options::default()
            .with_default_extension(Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        for (side, value) in [("width", preset.width), ("height", preset.height)] {
            if !SIDE.contains(&value) {
                return Err(ConfigError::Invalid(format!(
                    "`{side}` is {value}, but must be from {} to {}",
                    SIDE.start(),
                    SIDE.end()
                )));
            }
        }
        for name in preset.objects.keys() {
            match data.object_type_named(name) {
                None => {
                    return Err(ConfigError::Invalid(format!(
                        "`objects` names `{name}`, which isn't an object type in the data pack"
                    )));
                }
                Some(index) if data.object_types()[index].pseudo => {
                    return Err(ConfigError::Invalid(format!(
                        "`objects` names `{name}`, a pseudo type, which has no objects"
                    )));
                }
                Some(_) => {}
            }
        }
        let per_tiles = match preset.per_tiles {
            Some(per_tiles) if per_tiles > 0 => per_tiles,
            _ if preset.objects.is_empty() => 1,
            _ => {
                return Err(ConfigError::Invalid(
                    "`objects` are counted per area, so `per_tiles` must be given, above 0".into(),
                ));
            }
        };
        Ok(WorldConfig {
            width: preset.width,
            height: preset.height,
            objects: preset.objects,
            per_tiles,
        })
    }

    /// The map's width, in tiles.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// The map's height, in tiles.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// How many objects of the type called `name` a new world gets: the preset's
    /// density scaled to the map's area, rounding halves up. 0 for a type it doesn't name.
    pub fn object_count(&self, name: &str) -> usize {
        let Some(&count) = self.objects.get(name) else {
            return 0;
        };
        let tiles = u64::from(self.width) * u64::from(self.height);
        let per_tiles = u64::from(self.per_tiles);
        ((u64::from(count) * tiles + per_tiles / 2) / per_tiles) as usize
    }
}
