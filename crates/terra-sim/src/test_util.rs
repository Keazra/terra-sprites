//! Shared helpers for terra-sim unit tests; integration tests use `tests/common`.

use crate::data::DataPack;
use crate::map::Pos;

/// The built-in data pack, with `changes` made to `file`'s text, as
/// `tests/common`'s does.
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

/// A position at `(x, y)`.
pub fn at(x: u16, y: u16) -> Pos {
    Pos::new(x, y)
}
