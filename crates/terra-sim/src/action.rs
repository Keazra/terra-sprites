//! Actions (design §3.7, §5.5): what each sprite is doing, how an action
//! starts and ends, and how a moving one walks. Step 5 refreshes floods and
//! starts actions; step 6 carries them out.

use std::collections::BTreeSet;

use rand_chacha::ChaCha8Rng;
use serde::Serialize;

use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::map::{Dir, Pos};
use crate::objects::EntityId;
use crate::perception::{Flood, Ground, Occupied};
use crate::physics::step_cost;
use crate::random::uniform;
use crate::registry::Verb;
use crate::sprites::Sprite;
use crate::standin;
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
    /// Rest for a bout.
    Rest,
}

/// How far an action has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// On its way, with this many steps of its path left.
    Walking { steps_left: u32 },
    /// Held up: it had the points for its next step but couldn't take it,
    /// this many ticks in a row.
    Waiting { blocked_ticks: u32 },
    /// Resting, `ticks` into a bout of `of`.
    Resting { ticks: u32, of: u32 },
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
    /// The tick it started on, for the timeout.
    pub(crate) started: u64,
    /// The ticks it has been carried out on, at step 6.
    pub(crate) ticks: u32,
    /// Ticks in a row it had the points for its next step but couldn't take it.
    pub(crate) blocked_ticks: u32,
    /// The rest of the way round blocking sprites that blocked re-planning
    /// found, which the sprite keeps to while it lasts (design §3.7).
    pub(crate) committed: Option<Vec<Pos>>,
    /// How it ended, once it has.
    pub(crate) ended: Option<Outcome>,
}

/// What a sprite did at step 6, which its body feels at the next tick's
/// step 3 (design §2.4).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub(crate) struct Did {
    /// The steps it took.
    pub(crate) steps: u32,
    /// Whether it rested.
    pub(crate) rested: bool,
}

impl Action {
    fn new(verb: Verb, destination: Option<Pos>, tick: u64) -> Action {
        Action {
            verb,
            destination,
            started: tick,
            ticks: 0,
            blocked_ticks: 0,
            committed: None,
            ended: None,
        }
    }
}

/// `sprite`'s action as the screen sees it, if it has had one.
pub(crate) fn view(sprite: &Sprite, data: &DataPack) -> Option<ActionView> {
    let action = sprite.action.as_ref()?;
    let progress = if let Some(outcome) = action.ended {
        Progress::Ended(outcome)
    } else if action.verb == Verb::Rest {
        let of = data.physiology().actions.rest_bout;
        Progress::Resting {
            ticks: action.ticks,
            of,
        }
    } else if action.blocked_ticks > 0 {
        Progress::Waiting {
            blocked_ticks: action.blocked_ticks,
        }
    } else {
        let steps_left = way_ahead(sprite).map_or(0, |way| way.len() as u32);
        Progress::Walking { steps_left }
    };
    Some(ActionView {
        verb: action.verb,
        destination: action.destination,
        progress,
    })
}

/// Whether `sprite` has an action that hasn't ended.
fn is_acting(sprite: &Sprite) -> bool {
    sprite.action.as_ref().is_some_and(|a| a.ended.is_none())
}

/// Step 5 for every sprite not marked dying (design §2.4): refresh its flood
/// if it moved or the flood is due; end its action if that has timed out, or
/// is a Wander whose destination the flood no longer reaches (5.0); then, if
/// it has none, start one: its next scripted one, or else the stand-in's
/// choice (5b).
pub(crate) fn sense_and_decide(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    events: &mut Vec<Event>,
) {
    let physiology = data.physiology();
    let (refresh, timeout) = (
        physiology.movement.flood_refresh,
        physiology.actions.timeout,
    );
    let ids: Vec<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
    for id in ids.into_iter().filter(|id| !dying.contains(id)) {
        let sprite = state.sprites.get(id).expect("a sprite taking its turn");
        let stale = sprite
            .flood
            .as_ref()
            .is_none_or(|f| f.origin != sprite.pos || state.tick >= f.made + u64::from(refresh));
        if stale {
            let penalty = Occupied::Penalty(physiology.movement.occupied_penalty);
            let flood = flood(state, data, sprite, penalty);
            state.sprites.get_mut(id).expect("the same sprite").flood = Some(flood);
        }
        let sprite = state.sprites.get_mut(id).expect("the same sprite");
        if is_acting(sprite) {
            let flood = sprite.flood.as_ref().expect("the flood made above");
            let action = sprite.action.as_mut().expect("an action");
            if state.tick >= action.started + u64::from(timeout) {
                end(action, id, Outcome::TimedOut, state.tick, events);
            } else if action.committed.is_none()
                && action
                    .destination
                    .is_some_and(|to| flood.cost(to).is_none())
            {
                end(action, id, Outcome::Failed, state.tick, events);
            } else {
                continue;
            }
        }
        let (verb, destination) = match sprite.scripted.pop_front() {
            Some(ScriptedAction::Wander { destination }) => (Verb::Wander, Some(destination)),
            Some(ScriptedAction::Rest) => (Verb::Rest, None),
            None => match standin::choose(&mut state.rng) {
                Verb::Wander => {
                    let flood = sprite.flood.as_ref().expect("the flood made above");
                    (Verb::Wander, flood.wander_destination(&mut state.rng))
                }
                verb => (verb, None),
            },
        };
        start(sprite, id, verb, destination, state.tick, events);
    }
}

/// Starts `sprite` (`id`) on an action. A Wander with no destination, or one
/// its flood doesn't reach, ends at once, as failed (design §5.5).
fn start(
    sprite: &mut Sprite,
    id: EntityId,
    verb: Verb,
    destination: Option<Pos>,
    tick: u64,
    events: &mut Vec<Event>,
) {
    let mut action = Action::new(verb, destination, tick);
    events.push(Event {
        tick,
        kind: EventKind::ActionStarted { id, verb },
    });
    let flood = sprite.flood.as_ref().expect("step 5 made the flood");
    let lost = verb == Verb::Wander && destination.is_none_or(|to| flood.cost(to).is_none());
    if lost {
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

/// Step 6 (design §2.4, §3.7): every sprite not marked dying carries out its
/// action, in an order shuffled each tick by the world RNG. Moving sprites
/// all gain their move points first, in tenths; then each, on its turn,
/// steps along its path while its points last. The first to claim a tile
/// gets it: a sprite can't enter a tile another sprite stands on, except by
/// a head-on swap, so it waits there, banking points only up to the cost of
/// the step it's waiting to take.
pub(crate) fn resolve(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    events: &mut Vec<Event>,
) {
    let rest_bout = data.physiology().actions.rest_bout;
    let mut order: Vec<EntityId> = state
        .sprites
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !dying.contains(id))
        .collect();
    shuffle(&mut order, &mut state.rng);
    for &id in &order {
        let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
        sprite.did = Did::default();
        if is_walking(sprite) {
            sprite.move_points += (sprite.program.traits.speed * 10.0).round() as u32;
        }
    }
    // The sprites whose movement this tick is over: by stepping, or by a swap.
    let mut moved = BTreeSet::new();
    for &id in &order {
        let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
        if !is_acting(sprite) {
            continue;
        }
        let action = sprite.action.as_mut().expect("an action");
        action.ticks += 1;
        if action.verb == Verb::Rest {
            sprite.did.rested = true;
            if action.ticks >= rest_bout {
                end(action, id, Outcome::Applied, state.tick, events);
            }
        } else if !moved.contains(&id) {
            walk(state, data, id, &mut moved, events);
        }
    }
}

/// Shuffles `ids` with the world RNG: Fisher–Yates, one draw per place but the first.
fn shuffle(ids: &mut [EntityId], rng: &mut ChaCha8Rng) {
    for i in (1..ids.len()).rev() {
        let j = uniform(rng, i as u64 + 1) as usize;
        ids.swap(i, j);
    }
}

/// Whether `sprite` is doing an action that walks.
fn is_walking(sprite: &Sprite) -> bool {
    is_acting(sprite)
        && sprite
            .action
            .as_ref()
            .is_some_and(|a| a.destination.is_some())
}

/// The flood `sprite` would make from where it stands now, treating other
/// sprites as `occupied` says.
fn flood(state: &WorldState, data: &DataPack, sprite: &Sprite, occupied: Occupied) -> Flood {
    let ground = Ground {
        map: &state.map,
        objects: &state.objects,
        sprites: &state.sprites,
        data,
    };
    let radius = sprite.program.traits.sense_radius.round() as u16;
    Flood::new(ground, sprite.pos, radius, occupied, state.tick)
}

/// The tiles a walking sprite has still to step through: its committed path,
/// or else its flood's path to its destination from where it stands. `None`
/// if it has no way there.
fn way_ahead(sprite: &Sprite) -> Option<Vec<Pos>> {
    let action = sprite.action.as_ref().filter(|_| is_walking(sprite))?;
    if let Some(committed) = &action.committed {
        return Some(committed.clone());
    }
    let flood = sprite.flood.as_ref()?;
    let path = flood.path_to(action.destination?)?;
    if sprite.pos == flood.origin {
        return Some(path);
    }
    // It has stepped since the flood was made, this tick.
    let here = path.iter().position(|&pos| pos == sprite.pos)?;
    Some(path[here + 1..].to_vec())
}

/// The tile a walking sprite steps onto next, or `None` if it has no way on.
fn next_step(sprite: &Sprite) -> Option<Pos> {
    way_ahead(sprite)?.first().copied()
}

/// What `sprite`'s step onto `next` costs, in tenths, or `None` if physics forbids it.
fn cost_onto(state: &WorldState, data: &DataPack, sprite: &Sprite, next: Pos) -> Option<u32> {
    let dir = Dir::ALL
        .into_iter()
        .find(|&d| state.map.neighbour(sprite.pos, d) == Some(next))?;
    step_cost(&state.map, &state.objects, data, sprite.pos, dir).map(|cost| cost * 10)
}

/// Sprite `id`'s turn to walk: it steps along its path while its points last.
fn walk(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    moved: &mut BTreeSet<EntityId>,
    events: &mut Vec<Event>,
) {
    loop {
        let sprite = state.sprites.get(id).expect("the walker");
        let Some(next) = next_step(sprite) else {
            return;
        };
        let Some(cost) = cost_onto(state, data, sprite, next) else {
            wait(state, data, id, sprite.move_points, events);
            return;
        };
        if sprite.move_points < cost {
            return;
        }
        if let Some(other) = state.sprites.at(next) {
            if !swaps(state, data, id, other, moved) {
                wait(state, data, id, cost, events);
                return;
            }
            let other_cost = cost_onto(
                state,
                data,
                state.sprites.get(other).expect("it"),
                sprite.pos,
            )
            .expect("checked by swaps");
            state.sprites.swap(id, other);
            moved.extend([id, other]);
            stepped(state, other, other_cost, events);
            stepped(state, id, cost, events);
            return;
        }
        state.sprites.move_to(id, next);
        moved.insert(id);
        if stepped(state, id, cost, events) {
            return;
        }
    }
}

/// Whether `walker` may swap with `other`, the sprite on the tile it's
/// stepping onto: a head-on swap, where `other` hasn't moved this tick, is
/// stepping onto the walker's tile next, and has the points for it.
fn swaps(
    state: &WorldState,
    data: &DataPack,
    walker: EntityId,
    other: EntityId,
    moved: &BTreeSet<EntityId>,
) -> bool {
    if moved.contains(&other) {
        return false;
    }
    let here = state.sprites.get(walker).expect("the walker").pos;
    let other = state.sprites.get(other).expect("the sprite in the way");
    next_step(other) == Some(here)
        && cost_onto(state, data, other, here).is_some_and(|cost| other.move_points >= cost)
}

/// Sprite `id` has just stepped, for `cost` tenths. Returns whether that
/// brought it to its destination, which ends its action.
fn stepped(state: &mut WorldState, id: EntityId, cost: u32, events: &mut Vec<Event>) -> bool {
    let sprite = state.sprites.get_mut(id).expect("the walker");
    sprite.move_points -= cost;
    sprite.did.steps += 1;
    let action = sprite.action.as_mut().expect("an action");
    action.blocked_ticks = 0;
    if let Some(committed) = &mut action.committed {
        committed.remove(0);
    }
    if action.destination != Some(sprite.pos) {
        return false;
    }
    end(action, id, Outcome::Applied, state.tick, events);
    true
}

/// Sprite `id` had the points for its next step but couldn't take it: it
/// banks points only up to `cost`, the step's cost, and counts a blocked
/// tick. Blocked on its committed path, it drops the path and starts
/// counting again. At `replan_after` blocked ticks in a row, it searches
/// for a way round every sprite in its way (design §3.7): a way found
/// becomes its committed path; with none, its action ends as blocked.
fn wait(state: &mut WorldState, data: &DataPack, id: EntityId, cost: u32, events: &mut Vec<Event>) {
    let replan_after = data.physiology().movement.replan_after;
    let sprite = state.sprites.get_mut(id).expect("the walker");
    sprite.move_points = sprite.move_points.min(cost);
    let action = sprite.action.as_mut().expect("an action");
    action.blocked_ticks = if action.committed.take().is_some() {
        1
    } else {
        action.blocked_ticks + 1
    };
    if action.blocked_ticks < replan_after {
        return;
    }
    let destination = action.destination.expect("a walker has a destination");
    let sprite = state.sprites.get(id).expect("the walker");
    let way = flood(state, data, sprite, Occupied::Impassable).path_to(destination);
    let action = state
        .sprites
        .get_mut(id)
        .expect("the walker")
        .action
        .as_mut();
    let action = action.expect("an action");
    match way {
        Some(way) => {
            action.committed = Some(way);
            action.blocked_ticks = 0;
        }
        None => end(action, id, Outcome::Blocked, state.tick, events),
    }
}
