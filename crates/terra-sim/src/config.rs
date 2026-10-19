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

/// The allowed range for the first population (design §3.9).
const SPRITES: std::ops::RangeInclusive<u16> = 20..=100;

/// The most sprites a world holds with no `most_sprites` in its preset,
/// eggs included (M2 design §10): the size the speed check measures (A5).
pub(crate) const MOST_SPRITES: u16 = 100;

/// `MOST_SPRITES`, for a save from before breeding.
pub(crate) fn most_sprites() -> u16 {
    MOST_SPRITES
}

/// The allowed range for `most_sprites` (M2 design §10).
const MOST: std::ops::RangeInclusive<u16> = 1..=1000;

/// The share of a map's tiles under water, as a percentage, with no `water`
/// in its preset: the watery world of the game before Slice 29, which a
/// save or preset from before it was made in (M2 design §19.1).
pub(crate) const WATER: u8 = 32;

/// The allowed range for `water`: some water, and room above it for the
/// beaches and the land (design §3.2).
const WATERS: std::ops::RangeInclusive<u8> = 1..=60;

/// The allowed range for `lake_size` (M2 design §19.1).
const LAKE_SIZES: std::ops::RangeInclusive<u8> = 1..=5;

/// `WATER`, for a save from before Slice 29.
pub(crate) fn water() -> u8 {
    WATER
}

/// A lake size of 1, for a save from before Slice 29.
pub(crate) fn lake_size() -> u8 {
    1
}

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
    /// How many sprites a new world starts with.
    sprites: u16,
    /// The most sprites the world holds, eggs included: past it, breeding
    /// is a fruitless try (M2 design §10).
    most_sprites: u16,
    /// The share of the map's tiles under water, as a percentage (M2 design
    /// §19.1).
    water: u8,
    /// How big its lakes are: the map's hills and hollows are this many
    /// times as wide, so the same water lies in fewer, bigger lakes.
    lake_size: u8,
    /// The first sprites start within this many tiles of drinkable water,
    /// as the crow flies; anywhere, if `None` (M2 design §19.1).
    start_near_water: Option<u16>,
}

/// A config's fields, as a save or replay keeps them (design §2.7, §2.8).
#[derive(Debug, Clone)]
pub(crate) struct Parts {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) objects: BTreeMap<String, u32>,
    pub(crate) per_tiles: u32,
    pub(crate) sprites: u16,
    pub(crate) most_sprites: u16,
    pub(crate) water: u8,
    pub(crate) lake_size: u8,
    pub(crate) start_near_water: Option<u16>,
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
    #[serde(default)]
    sprites: Option<u16>,
    #[serde(default)]
    most_sprites: Option<u16>,
    #[serde(default)]
    water: Option<u8>,
    #[serde(default)]
    lake_size: Option<u8>,
    #[serde(default)]
    start_near_water: Option<u16>,
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
        WorldConfig::checked(preset, data)
    }

    /// The config `preset` gives, if it keeps the rules, checking the
    /// object types it names against `data`.
    fn checked(preset: Preset, data: &DataPack) -> Result<WorldConfig, ConfigError> {
        for (side, value) in [("width", preset.width), ("height", preset.height)] {
            if !SIDE.contains(&value) {
                return Err(ConfigError::Invalid(format!(
                    "`{side}` is {value}, but must be from {} to {}",
                    SIDE.start(),
                    SIDE.end()
                )));
            }
        }
        if let Some(name) = preset
            .objects
            .keys()
            .find(|name| data.real_object_type(name).is_none())
        {
            return Err(ConfigError::Invalid(format!(
                "`objects` names `{name}`, which isn't an object type in the data pack that can have objects"
            )));
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
        // A preset that leaves the sprites out gets none, as a type it
        // doesn't name gets no objects.
        let sprites = match preset.sprites {
            None => 0,
            Some(count) if SPRITES.contains(&count) => count,
            Some(count) => {
                return Err(ConfigError::Invalid(format!(
                    "`sprites` is {count}, but must be from {} to {}",
                    SPRITES.start(),
                    SPRITES.end()
                )));
            }
        };
        let most_sprites = preset.most_sprites.unwrap_or(MOST_SPRITES);
        if !MOST.contains(&most_sprites) || most_sprites < sprites {
            return Err(ConfigError::Invalid(format!(
                "`most_sprites` is {most_sprites}, but must be from {} to {}, and at least `sprites`",
                MOST.start(),
                MOST.end()
            )));
        }
        let water = preset.water.unwrap_or(WATER);
        if !WATERS.contains(&water) {
            return Err(ConfigError::Invalid(format!(
                "`water` is {water}, but must be from {} to {}",
                WATERS.start(),
                WATERS.end()
            )));
        }
        let lake_size = preset.lake_size.unwrap_or(1);
        if !LAKE_SIZES.contains(&lake_size) {
            return Err(ConfigError::Invalid(format!(
                "`lake_size` is {lake_size}, but must be from {} to {}",
                LAKE_SIZES.start(),
                LAKE_SIZES.end()
            )));
        }
        Ok(WorldConfig {
            width: preset.width,
            height: preset.height,
            objects: preset.objects,
            per_tiles,
            sprites,
            most_sprites,
            water,
            lake_size,
            start_near_water: preset.start_near_water,
        })
    }

    /// The fields, for a save to keep (design §2.8).
    pub(crate) fn parts(&self) -> Parts {
        Parts {
            width: self.width,
            height: self.height,
            objects: self.objects.clone(),
            per_tiles: self.per_tiles,
            sprites: self.sprites,
            most_sprites: self.most_sprites,
            water: self.water,
            lake_size: self.lake_size,
            start_near_water: self.start_near_water,
        }
    }

    /// A config from the fields a save or replay kept, checked again as a
    /// preset is (design §2.7, §2.8): a damaged file could hold any values,
    /// and a replay generates its world from them again.
    pub(crate) fn from_parts(parts: Parts, data: &DataPack) -> Result<WorldConfig, ConfigError> {
        let preset = Preset {
            width: parts.width,
            height: parts.height,
            objects: parts.objects,
            per_tiles: Some(parts.per_tiles),
            // A config with no sprites came from a preset that left them out.
            sprites: (parts.sprites > 0).then_some(parts.sprites),
            most_sprites: Some(parts.most_sprites),
            water: Some(parts.water),
            lake_size: Some(parts.lake_size),
            start_near_water: parts.start_near_water,
        };
        WorldConfig::checked(preset, data)
    }

    /// The map's width, in tiles.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// The map's height, in tiles.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// How many sprites a new world starts with.
    pub fn sprites(&self) -> usize {
        usize::from(self.sprites)
    }

    /// The most sprites the world holds, eggs included (M2 design §10).
    pub fn most_sprites(&self) -> usize {
        usize::from(self.most_sprites)
    }

    /// The share of the map's tiles under water, as a percentage (M2 design
    /// §19.1).
    pub fn water(&self) -> u8 {
        self.water
    }

    /// How big its lakes are: 1 for the ponds of the game before Slice 29,
    /// more for the same water in fewer, bigger lakes (M2 design §19.1).
    pub fn lake_size(&self) -> u8 {
        self.lake_size
    }

    /// How near drinkable water the first sprites start, in tiles as the
    /// crow flies; `None` for anywhere (M2 design §19.1).
    pub fn start_near_water(&self) -> Option<u16> {
        self.start_near_water
    }

    /// The same preset on a map `width` by `height`, each side kept within
    /// what a generated map may be: its densities scale to the new area, and
    /// the first population stays. The title screen's world is made so, at
    /// the terminal's size (M2 design §8.1).
    pub fn sized(self, width: u16, height: u16) -> WorldConfig {
        WorldConfig {
            width: width.clamp(*SIDE.start(), *SIDE.end()),
            height: height.clamp(*SIDE.start(), *SIDE.end()),
            ..self
        }
    }

    /// The same preset, starting with `sprites` sprites, for the benchmarks.
    pub(crate) fn with_sprites(self, sprites: u16) -> WorldConfig {
        WorldConfig { sprites, ..self }
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
