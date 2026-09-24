//! What happened during a tick, reported by `World::step` (design §2.5).

use crate::map::Pos;
use crate::objects::EntityId;

/// Something that happened during a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The tick it happened in.
    pub tick: u64,
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// An object was created, by a lifecycle rule.
    ObjectSpawned {
        id: EntityId,
        object_type: String,
        pos: Pos,
    },
    /// An object left the world.
    ObjectRemoved {
        id: EntityId,
        object_type: String,
        reason: Removal,
    },
}

/// Why an object left the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// Its last stage ended.
    Expired,
    /// A `DestroySelf` effect.
    Destroyed,
    /// A `ReplaceWith` effect put a new object in its place.
    Replaced,
}
