//! The Cursor as far as it touches the world (design v23 §6.5): the sprite
//! it leads or the item it holds, one thing at a time.

use serde::Serialize;

use crate::map::{Map, Pos};
use crate::objects::{EntityId, Objects};
use crate::sprites::Sprites;

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

    /// Checks that its tile is on the map, and that the one sprite led and
    /// the one item held off the map are the ones it has hold of;
    /// describes the first problem.
    pub(crate) fn check(
        &self,
        map: &Map,
        objects: &Objects,
        sprites: &Sprites,
    ) -> Result<(), String> {
        if let Some(tile) = self.tile.filter(|&tile| !map.contains(tile)) {
            return Err(format!("the Cursor is off the map, at {tile:?}"));
        }
        let led = sprites
            .iter()
            .filter(|(_, s)| s.lead.is_some())
            .map(|(id, _)| id);
        let held = objects.iter().filter(|(_, o)| o.held).map(|(id, _)| id);
        let (led, held): (Vec<EntityId>, Vec<EntityId>) = (led.collect(), held.collect());
        if led != self.leads().into_iter().collect::<Vec<_>>() {
            return Err(format!(
                "the Cursor leads {:?}, but {led:?} are led",
                self.leads()
            ));
        }
        if held != self.holds().into_iter().collect::<Vec<_>>() {
            return Err(format!(
                "the Cursor holds {:?}, but {held:?} are held",
                self.holds()
            ));
        }
        Ok(())
    }
}
