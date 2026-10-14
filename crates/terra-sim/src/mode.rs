//! Story mode and Lab mode (M2 design §16).

use serde::{Deserialize, Serialize};

/// How a world is played (M2 design §16): Story mode, the game as it's
/// meant to be played, or Lab mode, for experiments. A world is made in
/// one and keeps it for its whole life, through its saves and replays; a
/// world from before the modes is a Story world.
///
/// In milestone 2 nothing in the sim reads it: the front end caps Story
/// mode's speed, and only Lab worlds fast-forward. On the ship it decides
/// what the keeper may do (M4 design §2.2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Story,
    Lab,
}
