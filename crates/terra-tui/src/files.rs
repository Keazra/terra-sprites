//! Where the game keeps its files on the player's machine (design §6.7).

use std::path::PathBuf;

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
