//! What the game starts with, from the command line's flags (design §6.7):
//! the new world and the theme, its glyph set and palette. `main.rs` asks,
//! and reads what the answers need from the environment; the answers are
//! here, so they can be tested.

use std::path::{Path, PathBuf};

use terra_sim::{DataError, DataPack, Mode, World, WorldConfig};

use crate::args::Args;
use crate::files;
use crate::saves;
use crate::theme::{ColourPalette, GlyphSet, Pick, Theme};

/// A new world, as the flags ask: from `seed`, with `--preset`, or the data
/// folder's default preset, or the built-in one; and the data pack
/// `--data` gives, or the built-in one. It's a Lab world for `--lab` and
/// `--fast-forward`, and otherwise a Story world (M2 design §16).
pub fn new_world(args: &Args, seed: u64) -> Result<World, String> {
    let (config, data) = config(args)?;
    let mode = if args.fast_forward.is_some() {
        Mode::Lab
    } else {
        args.mode
    };
    Ok(World::new_in(config, data, seed, mode))
}

/// Whether the flags make or load a world, and so skip the title screen
/// (M2 design §8.5, §16): `--seed`, `--preset`, `--lab`, `--load` and
/// `--replay` do; the others only change how the game's worlds are made or
/// drawn.
pub fn skips_title(args: &Args) -> bool {
    args.seed.is_some()
        || args.preset.is_some()
        || args.mode == Mode::Lab
        || args.load.is_some()
        || args.replay.is_some()
}

/// The name of the preset a new world is made from, as fast-forward's
/// report gives it (M2 design §16.3): `--preset`'s file name without
/// `.ron`, or `default`.
pub fn preset_name(args: &Args) -> String {
    args.preset
        .as_ref()
        .and_then(|path| path.file_stem())
        .map_or_else(
            || "default".to_string(),
            |stem| stem.to_string_lossy().into_owned(),
        )
}

/// The world `--fast-forward` runs (M2 design §16), and what its save is
/// named after: the save `--load` names, which must be a Lab world, or a
/// new Lab world from `seed`, named after it.
pub fn fast_forward_world(
    args: &Args,
    seed: u64,
    folder: Option<&Path>,
) -> Result<(World, String), String> {
    match &args.load {
        Some(name) => {
            let file = saves::named(name, folder)?;
            let world = saves::load(&file)?;
            if world.mode() != Mode::Lab {
                return Err(format!(
                    "{} is a Story world: only Lab worlds fast-forward",
                    file.name
                ));
            }
            Ok((world, file.name))
        }
        None => Ok((new_world(args, seed)?, format!("seed-{seed}"))),
    }
}

/// The world the title screen wakes (M2 design §8.1): a new one from
/// `seed`, made from the default preset of the data pack the flags give,
/// whatever preset they name, on a map the terminal's size, `(width,
/// height)` in cells.
pub fn title_world(args: &Args, seed: u64, (width, height): (u16, u16)) -> Result<World, String> {
    let default = Args {
        preset: None,
        ..args.clone()
    };
    let (config, data) = config(&default)?;
    Ok(World::new(config.sized(width, height), data, seed))
}

/// A preset New world offers (M2 design §8.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// Its file's name without `.ron`, or `default`.
    pub name: String,
    /// Its file, or `None` for the default: the built-in preset, or
    /// `--data`'s own.
    pub path: Option<PathBuf>,
}

impl Default for Preset {
    /// The default preset, offered first.
    fn default() -> Preset {
        Preset {
            name: "default".into(),
            path: None,
        }
    }
}

/// The presets New world offers (M2 design §8.3): the default first, then
/// each `.ron` file in `folder`, by name. A folder that isn't there has
/// none.
pub fn presets(folder: Option<&Path>) -> Vec<Preset> {
    let mut files: Vec<Preset> = folder
        .and_then(|folder| std::fs::read_dir(folder).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let is_preset = path.is_file() && path.extension().is_some_and(|ext| ext == "ron");
            let name = path.file_stem()?.to_string_lossy().into_owned();
            is_preset.then_some(Preset {
                name,
                path: Some(path),
            })
        })
        .collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    std::iter::once(Preset::default()).chain(files).collect()
}

/// The config and data pack a new world is made from, as the flags ask.
fn config(args: &Args) -> Result<(WorldConfig, DataPack), String> {
    let (data, folder_preset) = match &args.data {
        None => (
            DataPack::builtin()
                .map_err(|err| format!("the built-in data pack is invalid: {err:?}"))?,
            None,
        ),
        Some(dir) => {
            let files::DataFolder { sources, preset } = files::data_folder_files(dir)
                .map_err(|err| format!("can't use the data in {}: {err}", dir.display()))?;
            let borrowed: Vec<(&str, &str)> = sources
                .iter()
                .map(|(path, text)| (path.as_str(), text.as_str()))
                .collect();
            let data = DataPack::from_sources(&borrowed).map_err(|err| {
                format!(
                    "can't use the data in {}: {}",
                    dir.display(),
                    pack_problem(err)
                )
            })?;
            (
                data,
                preset.map(|text| (dir.join(files::DEFAULT_PRESET), text)),
            )
        }
    };
    // A preset names object types, so it's checked against the pack.
    let preset = match &args.preset {
        Some(path) => Some((
            path.clone(),
            std::fs::read_to_string(path)
                .map_err(|err| format!("can't use preset {}: {err}", path.display()))?,
        )),
        None => folder_preset,
    };
    let config = match preset {
        None => WorldConfig::builtin(&data),
        Some((path, text)) => WorldConfig::from_ron(&text, &data)
            .map_err(|err| format!("can't use preset {}: {err}", path.display()))?,
    };
    Ok((config, data))
}

/// The glyph set the game starts with (design §6.2, §6.7, #307):
/// `--ascii`'s, or `--theme`'s file's, which last only for the run; else
/// the one the player saved; else CP437.
pub fn glyph_set(args: &Args, saved: Option<Pick<GlyphSet>>) -> Pick<GlyphSet> {
    if args.ascii {
        Pick::BuiltIn(GlyphSet::Ascii)
    } else if let Some(path) = &args.theme {
        Pick::File(path.clone())
    } else {
        saved.unwrap_or(Pick::BuiltIn(GlyphSet::Cp437))
    }
}

/// The palette the game starts with (design §6.2, §6.7, #307): `--theme`'s
/// file's, which lasts only for the run; else the one the player saved;
/// else mono, where `NO_COLOR`'s value, `no_color`, is set to anything but
/// nothing, as no-color.org asks; else the default. `main.rs` reads
/// `NO_COLOR` and passes it in.
pub fn palette(
    args: &Args,
    saved: Option<Pick<ColourPalette>>,
    no_color: Option<&str>,
) -> Pick<ColourPalette> {
    if let Some(path) = &args.theme {
        Pick::File(path.clone())
    } else if let Some(saved) = saved {
        saved
    } else if no_color.is_some_and(|value| !value.is_empty()) {
        Pick::BuiltIn(ColourPalette::Mono)
    } else {
        Pick::BuiltIn(ColourPalette::Default)
    }
}

/// The theme a glyph set and a palette make (design §6.2, #307), reading
/// the theme files they name, once each. A theme names object types and
/// drives, so it's checked against `data` (design v34 §6.7).
pub fn theme(
    glyphs: &Pick<GlyphSet>,
    palette: &Pick<ColourPalette>,
    data: &DataPack,
) -> Result<Theme, String> {
    let glyph_theme = match glyphs {
        Pick::BuiltIn(set) => set.source(),
        Pick::File(path) => theme_file(path, data)?,
    };
    let palette_theme = match palette {
        Pick::BuiltIn(palette) => palette.source(),
        Pick::File(path) if matches!(glyphs, Pick::File(same) if same == path) => {
            glyph_theme.clone()
        }
        Pick::File(path) => theme_file(path, data)?,
    };
    Ok(Theme::compose(&glyph_theme, &palette_theme))
}

/// The theme in the file at `path`, checked against `data`.
fn theme_file(path: &Path, data: &DataPack) -> Result<Theme, String> {
    std::fs::read_to_string(path)
        .map_err(|err| err.to_string())
        .and_then(|text| Theme::from_ron(&text, data))
        .map_err(|err| format!("can't use theme {}: {err}", path.display()))
}

/// What's wrong with a data pack, naming the file.
fn pack_problem(err: DataError) -> String {
    match err {
        DataError::MissingFile(file) => format!("it has no {file}"),
        DataError::Parse { file, message } | DataError::Invalid { file, message } => {
            format!("{file}: {message}")
        }
    }
}
