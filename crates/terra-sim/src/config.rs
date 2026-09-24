use serde::Deserialize;

/// The default preset, embedded at compile time from the repository's `data/presets/`.
const BUILTIN: &str = include_str!("../../../data/presets/default.ron");

/// The allowed range for each side of a generated map, in tiles.
const SIDE: std::ops::RangeInclusive<u16> = 32..=1024;

/// The settings a new world is made from. A preset file holds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldConfig {
    width: u16,
    height: u16,
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
}

impl WorldConfig {
    /// The default preset, `data/presets/default.ron`.
    pub fn builtin() -> WorldConfig {
        WorldConfig::from_ron(BUILTIN).expect("the built-in preset is valid")
    }

    /// Reads a preset from its RON text.
    pub fn from_ron(text: &str) -> Result<WorldConfig, ConfigError> {
        let preset: Preset = ron::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        for (side, value) in [("width", preset.width), ("height", preset.height)] {
            if !SIDE.contains(&value) {
                return Err(ConfigError::Invalid(format!(
                    "`{side}` is {value}, but must be from {} to {}",
                    SIDE.start(),
                    SIDE.end()
                )));
            }
        }
        Ok(WorldConfig {
            width: preset.width,
            height: preset.height,
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
}
