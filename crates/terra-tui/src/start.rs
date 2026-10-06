//! What the game starts with, from the command line's flags (design §6.7):
//! the new world and the theme. `main.rs` asks; the answers are here, so
//! they can be tested.

use std::path::{Path, PathBuf};

use terra_sim::{DataError, DataPack, World, WorldConfig};

use crate::args::Args;
use crate::files;
use crate::theme::Theme;

/// A new world, as the flags ask: from `seed`, with `--preset`, or the data
/// folder's default preset, or the built-in one; and the data pack
/// `--data` gives, or the built-in one.
pub fn new_world(args: &Args, seed: u64) -> Result<World, String> {
    let (config, data) = config(args)?;
    Ok(World::new(config, data, seed))
}

/// Whether the flags make or load a world, and so skip the title screen
/// (M2 design §8.5): `--seed`, `--preset` and `--replay` do; the others only
/// change how the game's worlds are made or drawn.
pub fn skips_title(args: &Args) -> bool {
    args.seed.is_some() || args.preset.is_some() || args.replay.is_some()
}

/// The world the title screen wakes (M2 design §8.1): a new one from
/// `seed`, made as the flags would make it, on a map the terminal's size,
/// `(width, height)` in cells.
pub fn title_world(args: &Args, seed: u64, (width, height): (u16, u16)) -> Result<World, String> {
    let (config, data) = config(args)?;
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
    let default = Preset {
        name: "default".into(),
        path: None,
    };
    std::iter::once(default).chain(files).collect()
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

/// The theme the flags ask for: `--ascii`, `--theme <file>` or the CP437
/// one. A theme names object types and drives, so it's checked against
/// `data` (design v34 §6.7).
pub fn theme(args: &Args, data: &DataPack) -> Result<Theme, String> {
    match &args.theme {
        None if args.ascii => Ok(Theme::ascii()),
        None => Ok(Theme::cp437()),
        Some(path) => std::fs::read_to_string(path)
            .map_err(|err| err.to_string())
            .and_then(|text| Theme::from_ron(&text, data))
            .map_err(|err| format!("can't use theme {}: {err}", path.display())),
    }
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
