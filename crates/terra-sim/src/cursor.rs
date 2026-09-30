//! The Cursor as far as it touches the world (design v23 §6.5): the sprite
//! it leads or the item it holds, one thing at a time.

use serde::Serialize;

use crate::map::Pos;
use crate::objects::EntityId;

/// What the Cursor has hold of, as the world knows it.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Cursor {
    pub(crate) grip: Option<Grip>,
    /// The tile it was last told it's on (design v23 §2.5), which a led
    /// sprite heads for.
    pub(crate) tile: Option<Pos>,
}

/// What the Cursor has hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Grip {
    /// A sprite it leads (design v23 §6.5).
    Leads(EntityId),
    /// An item it holds, off the map (design v23 §6.5).
    Holds(EntityId),
}

impl Cursor {
    /// The sprite it leads, if any.
    pub(crate) fn leads(&self) -> Option<EntityId> {
        match self.grip {
            Some(Grip::Leads(id)) => Some(id),
            _ => None,
        }
    }

    /// The item it holds, if any.
    pub(crate) fn holds(&self) -> Option<EntityId> {
        match self.grip {
            Some(Grip::Holds(id)) => Some(id),
            _ => None,
        }
    }
}
