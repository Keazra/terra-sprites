//! The player's commands (design §2.5): what the Cursor does to the world,
//! applied at step 1 of the tick they're stamped for.

use serde::{Deserialize, Serialize};

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
        reach_back: u64,
    },
    /// The Cursor's bad touch (design v21 §4.6): a zap, or amplified, a
    /// shock. It hurts without injuring. Its feeling looks back only the
    /// touch window, whatever the speed, so a late shock can't land on the
    /// wrong thing (design v21 §5.6).
    Correct { sprite: EntityId, amplified: bool },
}

/// One of the Cursor's four touches (design v21 §4.6): a Reward or a
/// Correct, amplified or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum CursorTouch {
    Pet,
    /// An amplified pet.
    Hug,
    Zap,
    /// An amplified zap.
    Shock,
}

impl CursorTouch {
    /// The touch `command` gives.
    pub fn of(command: &Command) -> CursorTouch {
        match *command {
            Command::Reward { amplified, .. } => CursorTouch::rewarding(amplified),
            Command::Correct { amplified, .. } => CursorTouch::correcting(amplified),
        }
    }

    /// The touch an event reports, if it reports one.
    pub fn reported(kind: &EventKind) -> Option<CursorTouch> {
        match *kind {
            EventKind::Rewarded { amplified, .. } => Some(CursorTouch::rewarding(amplified)),
            EventKind::Corrected { amplified, .. } => Some(CursorTouch::correcting(amplified)),
            _ => None,
        }
    }

    fn rewarding(amplified: bool) -> CursorTouch {
        if amplified {
            CursorTouch::Hug
        } else {
            CursorTouch::Pet
        }
    }

    fn correcting(amplified: bool) -> CursorTouch {
        if amplified {
            CursorTouch::Shock
        } else {
            CursorTouch::Zap
        }
    }

    /// Whether it's a Correct: a zap or a shock.
    pub fn corrects(self) -> bool {
        matches!(self, CursorTouch::Zap | CursorTouch::Shock)
    }

    /// The command that gives `sprite` this touch, a pet or hug looking
    /// `reach_back` ticks back.
    pub fn command(self, sprite: EntityId, reach_back: u64) -> Command {
        let amplified = matches!(self, CursorTouch::Hug | CursorTouch::Shock);
        if self.corrects() {
            Command::Correct { sprite, amplified }
        } else {
            Command::Reward {
                sprite,
                amplified,
                reach_back,
            }
        }
    }

    /// Its name: `pet`, `hug`, `zap` or `shock`.
    pub fn name(self) -> &'static str {
        match self {
            CursorTouch::Pet => "pet",
            CursorTouch::Hug => "hug",
            CursorTouch::Zap => "zap",
            CursorTouch::Shock => "shock",
        }
    }
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
        // A Reward looks back its reach back, within the bounds, and several
        // in a tick as far as the furthest; a Correct looks back only the
        // touch window, which its `shocked` pulse tells learning (design v21
        // §2.5, §5.6).
        let kind = match command {
            Command::Reward {
                amplified,
                reach_back,
                ..
            } => {
                let reach_back = reach_back.clamp(physiology.touch_window, cursor.max_reach_back);
                let brain = &mut touched.brain;
                brain.reach_back = brain.reach_back.max(Some(reach_back));
                let body = &mut touched.body;
                let reward = if amplified { cursor.hug } else { cursor.pet };
                body.raise(indices.reward, reward);
                body.pulse(indices.petted, None);
                EventKind::Rewarded {
                    id: sprite,
                    amplified,
                }
            }
            Command::Correct { amplified, .. } => {
                let body = &mut touched.body;
                let correction = if amplified { cursor.shock } else { cursor.zap };
                body.raise(indices.punishment, correction.punishment);
                body.raise(indices.pain, correction.pain);
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
