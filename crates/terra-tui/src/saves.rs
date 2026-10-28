//! Save files on the player's machine (design §2.8, §6.7): the quicksave,
//! saves by name and the autosaves, all in the saves folder.

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use terra_sim::World;

use crate::files;

/// A save file's extension, after the magic every save starts with.
pub const EXTENSION: &str = "tspr";

/// The quicksave's name, which `F5` writes and `F9` reads.
pub const QUICKSAVE: &str = "quicksave";

/// How many autosaves are kept: the newest is `autosave-1`.
pub const AUTOSAVES: usize = 3;

/// The longest a save's name may be, in characters.
pub const MAX_SAVE_NAME_CHARS: usize = 40;

/// A save in the saves folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    /// Its name: the file's, without the extension.
    pub name: String,
    pub path: PathBuf,
    /// When it was last written, if the system says.
    pub modified: Option<SystemTime>,
}

impl SaveFile {
    /// The save in the file at `path`, named after it.
    fn at(path: PathBuf) -> SaveFile {
        let name = path.file_stem().map_or_else(
            || path.display().to_string(),
            |stem| stem.to_string_lossy().into_owned(),
        );
        let modified = path.metadata().ok().and_then(|meta| meta.modified().ok());
        SaveFile {
            name,
            path,
            modified,
        }
    }
}

/// Reads `file` and loads the world in it (design §2.9), or says why it
/// couldn't: "Couldn't load mira: …". A world's `Ctrl+O` and `F9` and the
/// title screen's Continue and Load (M2 design §8.2) all load so.
pub fn load(file: &SaveFile) -> Result<World, String> {
    std::fs::read(&file.path)
        .map_err(|err| err.to_string())
        .and_then(|bytes| World::load(&bytes).map_err(|err| err.to_string()))
        .map_err(|why| format!("Couldn't load {}: {why}", file.name))
}

/// The file a save named `name` is written to, in `folder`, named by
/// `name_for`.
pub fn path_for(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!("{}.{EXTENSION}", name_for(name)))
}

/// The name a save called `name` is stored, and listed, by: one any
/// system's file names can hold. Each character some system forbids is made
/// a `-`; the dots and spaces Windows drops from the end go; and a name
/// Windows keeps for a device, such as `con`, gets a `-` after it.
pub fn name_for(name: &str) -> String {
    let safe: String = name
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let safe = safe.trim_end_matches(['.', ' ']);
    if safe.is_empty() {
        return "-".into();
    }
    // Windows reads only up to the first dot when it looks for a device.
    let (stem, rest) = safe.split_at(safe.find('.').unwrap_or(safe.len()));
    if is_device(stem) {
        format!("{stem}-{rest}")
    } else {
        safe.into()
    }
}

/// Whether Windows keeps `stem` for a device, in any case.
fn is_device(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    let numbered = |prefix| {
        upper
            .strip_prefix(prefix)
            .is_some_and(|n: &str| matches!(n.as_bytes(), [b'1'..=b'9']))
    };
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") || numbered("COM") || numbered("LPT")
}

/// Writes `bytes` as the save named `name` in `folder`, making the folder
/// if need be, and says where. It writes a file beside it first, then puts
/// that in place, so a save cut short never leaves a broken one where a
/// good one was.
pub fn write(folder: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let path = path_for(folder, name);
    files::write_in_place(&path, bytes)?;
    Ok(path)
}

/// The name a save called `name` is stored by (`name_for`), or if `folder`
/// has a save called that already, the first of `name-2`, `name-3` and on
/// that it hasn't, so a new save never writes over an old one.
pub fn unused_name(folder: &Path, name: &str) -> String {
    let name = name_for(name);
    (1..)
        .map(|n| match n {
            1 => name.clone(),
            n => format!("{name}-{n}"),
        })
        .find(|name| !path_for(folder, name).exists())
        .expect("a name no save has")
}

/// Writes `bytes` as the newest autosave, `autosave-1`, after moving each
/// older one down a place; the oldest of `AUTOSAVES` drops off the end.
pub fn autosave(folder: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    std::fs::create_dir_all(folder)?;
    for n in (1..AUTOSAVES).rev() {
        let older = path_for(folder, &autosave_name(n));
        if older.exists() {
            std::fs::rename(&older, path_for(folder, &autosave_name(n + 1)))?;
        }
    }
    write(folder, &autosave_name(1), bytes)
}

/// The name of the `n`th newest autosave, from 1.
pub fn autosave_name(n: usize) -> String {
    format!("autosave-{n}")
}

/// The save `name` names (M2 design §16): a file, if it's named by a path
/// ending in `.tspr`, or else the save of that name in `folder`, the saves
/// folder, as `list` lists it. `--load` opens it.
pub fn named(name: &str, folder: Option<&Path>) -> Result<SaveFile, String> {
    let as_file = Path::new(name);
    let path = if as_file.extension().is_some_and(|ext| ext == EXTENSION) {
        as_file.to_path_buf()
    } else {
        let folder = folder.ok_or("can't tell where the saves folder is")?;
        path_for(folder, name)
    };
    if !path.is_file() {
        return Err(format!(
            "there's no save called {name} ({})",
            path.display()
        ));
    }
    Ok(SaveFile::at(path))
}

/// Every save in `folder`, newest first, then by name. A folder that isn't
/// there has none.
pub fn list(folder: &Path) -> Vec<SaveFile> {
    let mut saves: Vec<SaveFile> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
            let is_save = path.extension().is_some_and(|ext| ext == EXTENSION);
            (is_file && is_save).then(|| SaveFile::at(path))
        })
        .collect();
    saves.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.cmp(&b.name))
    });
    saves
}

/// What a list of the saves in `folder`, the saves folder, says with none
/// to list: a world's `Ctrl+O` and the title screen's Load (M2 design
/// §8.2) say it alike.
pub fn none_listed(folder: Option<&Path>) -> String {
    match folder {
        Some(folder) => format!("No saves in {}", folder.display()),
        None => "No folder for saves".into(),
    }
}
