//! The player's commands (design §2.5): what the Cursor does to the world,
//! applied at step 1 of the tick they're stamped for.

use serde::Serialize;

use crate::biochem::Body;
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
    /// The Cursor's bad touch (design v21 §4.6): a zap, or amplified, a
    /// shock. It hurts without injuring. Its feeling looks back only the
    /// touch window, whatever the speed, so a late shock can't land on the
    /// wrong thing (design v21 §5.6).
    Correct { sprite: EntityId, amplified: bool },
}

/// Why a command was refused (design §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The sprite isn't in the world: it never was, or it has died.
    Gone,
}

/// Step 1 (design §2.4): applies the commands stamped for this tick, in the
/// order they were submitted.
pub(crate) fn apply(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let physiology = data.physiology();
    let (cursor, indices) = (&physiology.cursor, &physiology.indices);
    for command in std::mem::take(&mut state.commands) {
        let (Command::Reward { sprite, .. } | Command::Correct { sprite, .. }) = command;
        let Some(touched) = state.sprites.get_mut(sprite) else {
            events.push(Event {
                tick: state.tick,
                kind: EventKind::CommandRejected {
                    command,
                    reason: Rejection::Gone,
                },
            });
            continue;
        };
        // A Reward looks back its reach back, within the bounds; a Correct
        // the touch window. Several in a tick look back as far as the
        // furthest (design v21 §2.5, §5.6).
        let reach_back = match command {
            Command::Reward { reach_back, .. } => {
                u64::from(reach_back).clamp(physiology.touch_window, cursor.max_reach_back)
            }
            Command::Correct { .. } => physiology.touch_window,
        };
        let brain = &mut touched.brain;
        brain.reach_back = brain.reach_back.max(Some(reach_back));
        let body = &mut touched.body;
        let kind = match command {
            Command::Reward { amplified, .. } => {
                let reward = if amplified { cursor.hug } else { cursor.pet };
                raise(body, indices.reward, reward);
                body.pulse(indices.petted, None);
                EventKind::Rewarded {
                    id: sprite,
                    amplified,
                }
            }
            Command::Correct { amplified, .. } => {
                let touch = if amplified { cursor.shock } else { cursor.zap };
                raise(body, indices.punishment, touch.punishment);
                raise(body, indices.pain, touch.pain);
                body.pulse(indices.shocked, None);
                EventKind::Corrected {
                    id: sprite,
                    amplified,
                }
            }
        };
        events.push(Event {
            tick: state.tick,
            kind,
        });
    }
}

/// Raises `body`'s chemical at `index` by `amount`, no further than 1.
fn raise(body: &mut Body, index: usize, amount: f32) {
    body.chems[index] = (body.chems[index] + amount).min(1.0);
}
