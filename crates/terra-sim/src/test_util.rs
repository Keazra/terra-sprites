//! Shared helpers for unit tests inside terra-sim.
//!
//! Integration tests use `tests/common` instead. This module covers the
//! helpers that unit tests under `src/` used to copy into each `#[cfg(test)]`
//! block — mainly `at` for building a `Pos`.

use crate::map::Pos;

/// A position at `(x, y)`.
pub fn at(x: u16, y: u16) -> Pos {
    Pos::new(x, y)
}
