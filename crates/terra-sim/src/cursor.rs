//! The Cursor as far as it touches the world (design v23 §6.5): the sprite
//! it leads or the item it holds, one thing at a time.

use serde::Serialize;

use crate::objects::EntityId;

/// What the Cursor has hold of, as the world knows it.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Cursor {
    pub(crate) grip: Option<Grip>,
}

/// What the Cursor has hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum Grip {
    /// A sprite it leads (design v23 §6.5).
    Leads(EntityId),
}

impl Cursor {
    /// The sprite it leads, if any.
    pub(crate) fn leads(&self) -> Option<EntityId> {
        match self.grip {
            Some(Grip::Leads(id)) => Some(id),
            None => None,
        }
    }
}

/// A read-only view of the Cursor, as far as it touches the world.
pub struct CursorView<'a> {
    pub(crate) cursor: &'a Cursor,
}

impl CursorView<'_> {
    /// The sprite the Cursor leads, if any (design v23 §6.5).
    pub fn leads(&self) -> Option<EntityId> {
        self.cursor.leads()
    }
}
