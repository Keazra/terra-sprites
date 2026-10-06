//! Shared helpers for terra-sim unit tests; integration tests use `tests/common`.

use crate::map::Pos;

/// A position at `(x, y)`.
pub fn at(x: u16, y: u16) -> Pos {
    Pos::new(x, y)
}
