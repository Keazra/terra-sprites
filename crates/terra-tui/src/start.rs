//! What the game starts with, from the command line's flags (design §6.7):
//! the new world and the theme. `main.rs` asks; the answers are here, so
//! they can be tested.

use terra_sim::{DataError, DataPack, World, WorldConfig};

use crate::args::Args;
use crate::files;
use crate::theme::Theme;

/// A new world, as the flags ask: from `seed`, with `--preset`, or the data
/// folder's default preset, or the built-in one; and the data pack
/// `--data` gives, or the built-in one.
pub fn new_world(args: &Args, seed: u64) -> Result<World, String> {
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
    Ok(World::new(config, data, seed))
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
