//! What the screen may show the player (design §6.4). Every display that
//! tells the player something about the world asks the app's `InfoPolicy`
//! first. In M1 the answer is always yes; a later diegetic mode is a new
//! policy, and only changes what the screen reveals, never the world.

use terra_sim::{EntityId, Pos};

use crate::app::Tab;

/// A display that tells the player something about the world (design §6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// A sprite's colour on the map, which tells its strongest drive (design
    /// §6.3).
    MapColours,
    /// A sprite's emotes on the map (design §6.3).
    Emotes,
    /// The selected sprite's Decision and Attention markers on the map
    /// (design §6.1).
    Markers,
    /// A line of the event log.
    EventLog,
    /// The top bar's counts: the population.
    Counts,
    /// A row of the sprite list.
    SpriteList,
    /// The status line's part about the tile under the Cursor.
    TileInfo,
    /// An inspector tab.
    Tab(Tab),
}

/// What a display is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// The world as a whole: the population, the World tab, an event about
    /// no sprite.
    World,
    /// One sprite.
    Sprite(EntityId),
    /// One tile, with whatever is on it.
    Tile(Pos),
}

/// Decides what the screen may show (design §6.4).
pub trait InfoPolicy {
    /// Whether `panel` may show what it knows about `subject`.
    fn can_view(&self, panel: Panel, subject: Subject) -> bool;
}

/// M1's policy: the player sees everything (design §6.4).
#[derive(Debug, Clone, Copy, Default)]
pub struct Omniscient;

impl InfoPolicy for Omniscient {
    fn can_view(&self, _panel: Panel, _subject: Subject) -> bool {
        true
    }
}
