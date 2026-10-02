//! What happened during a tick, reported by `World::step` (design §2.5).

use crate::action::{ActionView, Outcome};
use crate::brain::{Learned, Thing};
use crate::command::{Command, Rejection};
use crate::map::Pos;
use crate::objects::EntityId;
use crate::registry::Verb;

/// Something that happened during a tick.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// The tick it happened in.
    pub tick: u64,
    pub kind: EventKind,
}

/// What kind of thing happened, with the entities involved.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// An object was created, by a lifecycle rule.
    ObjectSpawned {
        id: EntityId,
        object_type: String,
        pos: Pos,
    },
    /// A sprite started an action.
    ActionStarted { id: EntityId, verb: Verb },
    /// A sprite's action ended.
    ActionEnded {
        id: EntityId,
        verb: Verb,
        outcome: Outcome,
        /// The action as it ended: what it was aimed at, and whether it got
        /// to make its attempt, for describing it afterwards.
        action: ActionView,
    },
    /// A sprite died, and left the world.
    Died {
        id: EntityId,
        /// The name the player gave it, if any (design v28 §2.5).
        name: Option<String>,
        cause: DeathCause,
        /// Its age in ticks.
        age: u64,
    },
    /// A sprite learned a lesson (design §5.6): something it learned got
    /// half a point from nothing for the first time, up if `good`.
    LearnedMilestone {
        id: EntityId,
        learned: Learned,
        good: bool,
    },
    /// The Cursor rewarded a sprite (design v21 §4.6): a pet, or amplified,
    /// a hug.
    Rewarded { id: EntityId, amplified: bool },
    /// The Cursor corrected a sprite (design v21 §4.6): a zap, or amplified,
    /// a shock.
    Corrected { id: EntityId, amplified: bool },
    /// The Cursor placed a new object (design v28 §2.5).
    Placed {
        id: EntityId,
        object_type: String,
        pos: Pos,
    },
    /// A new sprite was spawned by the Cursor (design v28 §2.5).
    Spawned { id: EntityId, pos: Pos },
    /// The player named a sprite (design v28 §2.5).
    Renamed { id: EntityId, name: String },
    /// The Cursor took hold of a sprite, which it now leads (design v23
    /// §6.5).
    TookHold { sprite: EntityId },
    /// The Cursor picked up an item, and holds it off the map (design v23
    /// §6.5).
    PickedUp { item: EntityId, object_type: String },
    /// The Cursor threw the item it held, which rolls (design v25 §3.5.4).
    Threw { item: EntityId, object_type: String },
    /// The Cursor shoved the sprite it led, which slides (design v25 §3.5.4).
    Shoved { sprite: EntityId },
    /// A sliding sprite crashed into a thing: a sprite, or an object, by its
    /// type's name (design v25 §3.5.4); and whether the crash hurt it.
    Crashed {
        sprite: EntityId,
        into: Thing,
        hurt: bool,
    },
    /// The Cursor put the item it held down on a tile (design v23 §6.5).
    PutDown {
        item: EntityId,
        object_type: String,
        pos: Pos,
    },
    /// The Cursor let go of the sprite it led (design v23 §6.5).
    LetGo { sprite: EntityId },
    /// The Cursor lost what it had hold of by itself (design v23 §2.5).
    CursorEmptied { reason: Emptied },
    /// A command was refused, and did nothing (design §2.5).
    CommandRejected { command: Command, reason: Rejection },
    /// An object left the world.
    ObjectRemoved {
        id: EntityId,
        object_type: String,
        reason: Removal,
    },
}

/// What caused most of a dead sprite's recent injury (design §4.10). Ties
/// are settled in this order: physiology's three causes, then objects by
/// type ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum DeathCause {
    /// Its energy ran out.
    Starvation,
    /// Its hydration ran out.
    Dehydration,
    /// It lived past its lifespan.
    OldAge,
    /// Injury an object's verb injected: the stable ID of the object type
    /// whose verb table did it, such as a thornbush's.
    HurtBy(u16),
}

impl DeathCause {
    /// Physiology's causes, built in, in the order ties are settled.
    pub const PHYSIOLOGY: [DeathCause; 3] = [
        DeathCause::Starvation,
        DeathCause::Dehydration,
        DeathCause::OldAge,
    ];
}

/// How the Cursor lost what it had hold of by itself (design v23 §2.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Emptied {
    /// The sprite it led died.
    Died { sprite: EntityId },
    /// The item it held left the world by itself: it expired, say.
    Removed {
        item: EntityId,
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
