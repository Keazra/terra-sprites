//! The player's commands (design §2.5): what the Cursor does to the world,
//! applied at step 1 of the tick they're stamped for.

use serde::{Deserialize, Serialize};

use crate::action::{self, Outcome, Walk};
use crate::cursor::Grip;
use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::map::Pos;
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
    /// Takes hold of a sprite, which the Cursor then leads (design v23 §6.5).
    TakeHold { sprite: EntityId },
    /// Lets go of the sprite the Cursor leads, which chooses for itself
    /// again at its next step 5 (design v23 §6.5).
    LetGo,
    /// Where the Cursor is: sent while it leads a sprite, which heads there
    /// (design v23 §2.5).
    MoveCursor { tile: Pos },
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
    /// The touch `command` gives, if it's a touch.
    pub fn of(command: &Command) -> Option<CursorTouch> {
        match *command {
            Command::Reward { amplified, .. } => Some(CursorTouch::rewarding(amplified)),
            Command::Correct { amplified, .. } => Some(CursorTouch::correcting(amplified)),
            Command::TakeHold { .. } | Command::LetGo | Command::MoveCursor { .. } => None,
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
    /// The Cursor already has hold of something (design v23 §6.5).
    Busy(Grip),
    /// The Cursor leads no sprite to let go of.
    NotLeading,
}

/// Step 1 (design §2.4): applies the commands stamped for this tick, in the
/// order they were submitted.
pub(crate) fn apply(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    for command in std::mem::take(&mut state.commands) {
        let applied = match command {
            Command::Reward {
                sprite,
                amplified,
                reach_back,
            } => reward(state, data, sprite, amplified, reach_back),
            Command::Correct { sprite, amplified } => correct(state, data, sprite, amplified),
            Command::TakeHold { sprite } => take_hold(state, sprite, events),
            Command::LetGo => let_go(state),
            // Nothing to report: it moves many times a second while leading.
            Command::MoveCursor { tile } => {
                state.cursor.tile = Some(tile);
                continue;
            }
        };
        let kind = match applied {
            Ok(kind) => kind,
            Err(reason) => EventKind::CommandRejected { command, reason },
        };
        events.push(Event {
            tick: state.tick,
            kind,
        });
    }
}

/// The Cursor's good touch on `sprite` (design v21 §4.6): a pet, or
/// amplified, a hug. Its feeling looks back its reach back, within the
/// bounds, and several in a tick as far as the furthest (design v21 §2.5,
/// §5.6).
fn reward(
    state: &mut WorldState,
    data: &DataPack,
    sprite: EntityId,
    amplified: bool,
    reach_back: u64,
) -> Result<EventKind, Rejection> {
    let physiology = data.physiology();
    let (cursor, indices) = (&physiology.cursor, &physiology.indices);
    let touched = state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?;
    let reach_back = reach_back.clamp(physiology.touch_window, cursor.max_reach_back);
    let brain = &mut touched.brain;
    brain.reach_back = brain.reach_back.max(Some(reach_back));
    let body = &mut touched.body;
    let reward = if amplified { cursor.hug } else { cursor.pet };
    body.raise(indices.reward, reward);
    body.pulse(indices.petted, None);
    Ok(EventKind::Rewarded {
        id: sprite,
        amplified,
    })
}

/// The Cursor's bad touch on `sprite` (design v21 §4.6): a zap, or
/// amplified, a shock. Its feeling looks back only the touch window, which
/// its `shocked` pulse tells learning (design v21 §5.6).
fn correct(
    state: &mut WorldState,
    data: &DataPack,
    sprite: EntityId,
    amplified: bool,
) -> Result<EventKind, Rejection> {
    let physiology = data.physiology();
    let (cursor, indices) = (&physiology.cursor, &physiology.indices);
    let body = &mut state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?.body;
    let correction = if amplified { cursor.shock } else { cursor.zap };
    body.raise(indices.punishment, correction.punishment);
    body.raise(indices.pain, correction.pain);
    body.pulse(indices.shocked, None);
    Ok(EventKind::Corrected {
        id: sprite,
        amplified,
    })
}

/// The Cursor takes hold of `sprite`, and leads it (design v23 §6.5):
/// whatever it was doing ends, pulled away.
fn take_hold(
    state: &mut WorldState,
    sprite: EntityId,
    events: &mut Vec<Event>,
) -> Result<EventKind, Rejection> {
    if let Some(grip) = state.cursor.grip {
        return Err(Rejection::Busy(grip));
    }
    let led = state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?;
    if let Some(doing) = led.action.as_mut().filter(|a| a.ended.is_none()) {
        action::end(doing, sprite, Outcome::PulledAway, state.tick, events);
    }
    led.lead = Some(Walk::default());
    state.cursor.grip = Some(Grip::Leads(sprite));
    state.cursor.tile = Some(led.pos);
    Ok(EventKind::TookHold { sprite })
}

/// The Cursor lets go of the sprite it leads (design v23 §6.5).
fn let_go(state: &mut WorldState) -> Result<EventKind, Rejection> {
    let sprite = state.cursor.leads().ok_or(Rejection::NotLeading)?;
    state.cursor.grip = None;
    let led = state.sprites.get_mut(sprite).expect("the led sprite");
    led.lead = None;
    Ok(EventKind::LetGo { sprite })
}
