//! Where the game keeps its files on the player's machine (design §6.7).

use std::path::{Path, PathBuf};

use terra_sim::DataPack;

/// The game's folder in the platform's data folder: `%APPDATA%` on
/// Windows, `~/Library/Application Support` on macOS, and elsewhere
/// `$XDG_DATA_HOME` or `~/.local/share`. `None` if the environment names
/// none of them.
pub fn data_folder() -> Option<PathBuf> {
    let var = |name| std::env::var_os(name).filter(|value| !value.is_empty());
    let base = if cfg!(windows) {
        var("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    };
    base.map(|base| base.join("terra-sprites"))
}

/// The folder genomes are exported to and read from (design §6.7).
pub fn genome_folder() -> Option<PathBuf> {
    data_folder().map(|folder| folder.join("genomes"))
}

/// The folder saves go in (design §6.7).
pub fn save_folder() -> Option<PathBuf> {
    data_folder().map(|folder| folder.join("saves"))
}

/// The session log every session writes (design §2.7), in the game's
/// folder.
pub fn session_log() -> Option<PathBuf> {
    data_folder().map(|folder| folder.join(SESSION_LOG))
}

/// The session log's name.
pub const SESSION_LOG: &str = "last_session.replay";

/// Where the default preset sits in a data folder.
const DEFAULT_PRESET: &str = "presets/default.ron";

/// What `--data <dir>` reads from its folder (design §6.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFolder {
    /// The data pack's files, as `(path within the pack, RON text)`: each
    /// the folder has in place of the built-in one, which stays for any it
    /// lacks.
    pub sources: Vec<(String, String)>,
    /// The folder's default preset, `presets/default.ron`, if it has one.
    pub preset: Option<String>,
}

/// Reads what `--data <dir>` gives (design §6.7). Fails naming a file the
/// folder has but can't be read, or the folder if it isn't one.
pub fn data_folder_files(dir: &Path) -> Result<DataFolder, String> {
    if !dir.is_dir() {
        return Err(format!("{} isn't a folder", dir.display()));
    }
    let read = |path: &str| -> Result<Option<String>, String> {
        let file = dir.join(path);
        if !file.is_file() {
            return Ok(None);
        }
        std::fs::read_to_string(&file)
            .map(Some)
            .map_err(|err| format!("can't read {}: {err}", file.display()))
    };
    let mut sources = Vec::new();
    for &(path, builtin) in DataPack::builtin_sources() {
        let text = read(path)?.unwrap_or_else(|| builtin.to_string());
        sources.push((path.to_string(), text));
    }
    Ok(DataFolder {
        sources,
        preset: read(DEFAULT_PRESET)?,
    })
}
