//! Actions (design §3.7, §5.5): what each sprite is doing, how an action
//! starts and ends, and how a moving one walks. Step 5 refreshes floods and
//! starts actions; step 6 carries them out.

use serde::Serialize;

use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::{Flood, Ground, Occupied};
use crate::physics::step_cost;
use crate::registry::Verb;
use crate::sprites::Sprite;
use crate::world::WorldState;

/// How an action ended (design §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Outcome {
    /// It did what it set out to do: arrived, or rested its bout.
    Applied,
    /// Blocked re-planning found no way through (design §3.7).
    Blocked,
    /// It couldn't be carried out.
    Failed,
    /// It was still going at the timeout.
    TimedOut,
}

/// An action to start a sprite on in a hand-made world, instead of what it
/// would choose, so tests and lab scenarios can set up exact situations.
/// When it ends, the sprite chooses for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ScriptedAction {
    /// Wander to `destination`.
    Wander { destination: Pos },
}

/// How far an action has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// On its way, with this many steps of its path left.
    Walking { steps_left: u32 },
    /// It ended, and how.
    Ended(Outcome),
}

/// A read-only view of a sprite's action: the one it's doing, or the one
/// that last ended until the next one starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionView {
    pub verb: Verb,
    /// Where a Wander is heading.
    pub destination: Option<Pos>,
    pub progress: Progress,
}

/// A sprite's action.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Action {
    pub(crate) verb: Verb,
    /// Where a Wander is heading.
    pub(crate) destination: Option<Pos>,
    /// How it ended, once it has.
    pub(crate) ended: Option<Outcome>,
}

impl Action {
    fn new(verb: Verb, destination: Option<Pos>) -> Action {
        Action {
            verb,
            destination,
            ended: None,
        }
    }

    /// The action as the screen sees it, for a sprite whose flood is `flood`.
    pub(crate) fn view(&self, flood: Option<&Flood>) -> ActionView {
        let progress = match self.ended {
            Some(outcome) => Progress::Ended(outcome),
            None => {
                let steps_left = self
                    .destination
                    .and_then(|to| flood?.path_to(to))
                    .map_or(0, |path| path.len() as u32);
                Progress::Walking { steps_left }
            }
        };
        ActionView {
            verb: self.verb,
            destination: self.destination,
            progress,
        }
    }
}

/// Whether `sprite` has an action that hasn't ended.
fn is_acting(sprite: &Sprite) -> bool {
    sprite.action.as_ref().is_some_and(|a| a.ended.is_none())
}

/// Step 5 for every sprite not marked dying (design §2.4): refresh its flood
/// if it moved, then start an action if it has none.
pub(crate) fn sense_and_decide(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    events: &mut Vec<Event>,
) {
    let ids: Vec<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
    for id in ids.into_iter().filter(|id| !dying.contains(id)) {
        let sprite = state.sprites.get(id).expect("a sprite taking its turn");
        let stale = sprite.flood.as_ref().is_none_or(|f| f.origin != sprite.pos);
        if stale {
            let ground = Ground {
                map: &state.map,
                objects: &state.objects,
                sprites: &state.sprites,
                data,
            };
            let radius = sprite.program.traits.sense_radius.round() as u16;
            let flood = Flood::new(ground, sprite.pos, radius, Occupied::Penalty(0), state.tick);
            state.sprites.get_mut(id).expect("the same sprite").flood = Some(flood);
        }
        let sprite = state.sprites.get_mut(id).expect("the same sprite");
        if is_acting(sprite) {
            continue;
        }
        if let Some(ScriptedAction::Wander { destination }) = sprite.scripted.take() {
            start(
                sprite,
                id,
                Verb::Wander,
                Some(destination),
                state.tick,
                events,
            );
        }
    }
}

/// Starts `sprite` (`id`) on an action. A Wander to a destination its flood
/// doesn't reach ends at once, as failed (design §5.5).
fn start(
    sprite: &mut Sprite,
    id: EntityId,
    verb: Verb,
    destination: Option<Pos>,
    tick: u64,
    events: &mut Vec<Event>,
) {
    let mut action = Action::new(verb, destination);
    events.push(Event {
        tick,
        kind: EventKind::ActionStarted { id, verb },
    });
    let flood = sprite.flood.as_ref().expect("step 5 made the flood");
    if destination.is_some_and(|to| flood.cost(to).is_none()) {
        end(&mut action, id, Outcome::Failed, tick, events);
    }
    sprite.action = Some(action);
}

/// Ends `action` (sprite `id`'s) with `outcome`, and reports it.
fn end(action: &mut Action, id: EntityId, outcome: Outcome, tick: u64, events: &mut Vec<Event>) {
    action.ended = Some(outcome);
    events.push(Event {
        tick,
        kind: EventKind::ActionEnded {
            id,
            verb: action.verb,
            outcome,
        },
    });
}

/// Step 6 (design §2.4, §3.7): every acting sprite not marked dying carries
/// out its action. Moving ones gain move points, in tenths, and step along
/// their path while the points last.
pub(crate) fn resolve(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    events: &mut Vec<Event>,
) {
    let ids: Vec<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
    for id in ids.into_iter().filter(|id| !dying.contains(id)) {
        let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
        if !is_acting(sprite) {
            continue;
        }
        let action = sprite.action.as_ref().expect("an action");
        let Some(destination) = action.destination else {
            continue;
        };
        let Some(path) = sprite.flood.as_ref().and_then(|f| f.path_to(destination)) else {
            continue;
        };
        sprite.move_points += (sprite.program.traits.speed * 10.0).round() as u32;
        for next in path {
            let sprite = state.sprites.get(id).expect("the walker");
            let from = sprite.pos;
            let dir = crate::map::Dir::ALL
                .into_iter()
                .find(|&d| state.map.neighbour(from, d) == Some(next))
                .expect("a path goes a step at a time");
            let Some(cost) = step_cost(&state.map, &state.objects, data, from, dir) else {
                break;
            };
            let cost = cost * 10;
            if sprite.move_points < cost {
                break;
            }
            state.sprites.move_to(id, next);
            let sprite = state.sprites.get_mut(id).expect("the walker");
            sprite.move_points -= cost;
            if next == destination {
                let action = sprite.action.as_mut().expect("an action");
                end(action, id, Outcome::Applied, state.tick, events);
                break;
            }
        }
    }
}
