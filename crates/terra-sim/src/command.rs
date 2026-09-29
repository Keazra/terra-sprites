//! The player's commands (design §2.5): what the Cursor does to the world,
//! applied at step 1 of the tick they're stamped for.

use serde::Serialize;

use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::objects::EntityId;
use crate::world::WorldState;

/// Something the player does to the world through the Cursor. It carries
/// values, never references (design §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Command {
    /// The Cursor's good touch (design v21 §4.6): a pet, or amplified, a hug.
    Reward {
        sprite: EntityId,
        amplified: bool,
        /// How many ticks back its feeling looks for the sprite's latest
        /// attempt (design v21 §5.6).
        reach_back: u16,
    },
}

/// Step 1 (design §2.4): applies the commands stamped for this tick, in the
/// order they were submitted.
pub(crate) fn apply(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let physiology = data.physiology();
    let indices = &physiology.indices;
    for command in std::mem::take(&mut state.commands) {
        match command {
            Command::Reward {
                sprite: id,
                amplified,
                ..
            } => {
                let sprite = state.sprites.get_mut(id).expect("a sprite in the world");
                let reward = &mut sprite.body.chems[indices.reward];
                *reward = (*reward + physiology.cursor.pet).min(1.0);
                sprite.body.pulse(indices.petted, None);
                events.push(Event {
                    tick: state.tick,
                    kind: EventKind::Rewarded { id, amplified },
                });
            }
        }
    }
}
