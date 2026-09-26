//! Actions (design §3.7, §5.5): what each sprite is doing, how an action
//! starts and ends, and how a moving one walks. Step 5 refreshes floods and
//! starts actions; step 6 carries them out.

use std::collections::BTreeSet;

use rand_chacha::ChaCha8Rng;
use serde::Serialize;

use crate::data::DataPack;
use crate::decide::decide;
use crate::events::{Event, EventKind};
use crate::map::{Dir, Pos};
use crate::objects::EntityId;
use crate::perception::{Flood, Ground, Occupied, Target};
use crate::physics::step_cost;
use crate::random::uniform;
use crate::registry::Verb;
use crate::sprites::Sprite;
use crate::verbs;
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
    /// The sprite changed its mind: attention moved off its target, or
    /// another verb beat it by more than the switch margin (design §5.5).
    Interrupted,
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
    /// Eat the object on `at`.
    Eat { at: Pos },
    /// Drink the water on `at`.
    Drink { at: Pos },
    /// Approach the sprite on `at`, or else the object there, or else the water.
    Approach { at: Pos },
    /// Play with the sprite on `at`, or else the object there.
    Play { at: Pos },
    /// Hit the sprite on `at`, or else the object there.
    Hit { at: Pos },
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
    /// What kind of action it is.
    pub verb: Verb,
    /// Where it's heading: a Wander's destination, or the goal tile an
    /// aimed action is walking to.
    pub destination: Option<Pos>,
    /// What an action aimed at something is aimed at.
    pub target: Option<Target>,
    /// The stable ID of the target's object type, a pseudo type for water
    /// or a sprite: kept from the start, so it names a target that's gone.
    pub target_type: Option<u16>,
    /// Whether it got to its target and made its attempt (design §5.5).
    pub attempted: bool,
    /// Whether its target had left the world when it ended: eaten whole, say.
    pub target_gone: bool,
    /// How far it has got, or how it ended.
    pub progress: Progress,
}

/// A sprite's action.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Action {
    pub(crate) verb: Verb,
    /// Where it's heading: a Wander's destination, or the goal tile an
    /// aimed action is walking to, found again at every 5.0.
    pub(crate) destination: Option<Pos>,
    /// What an Approach, Eat or Drink is aimed at (design §5.3).
    pub(crate) target: Option<Target>,
    /// The stable ID of the target's object type.
    pub(crate) target_type: Option<u16>,
    /// Whether it got to its target and made its attempt.
    pub(crate) attempted: bool,
    /// Where its target stood at the latest 5.0.
    pub(crate) target_at: Option<Pos>,
    /// Whether its target had left the world when it ended.
    pub(crate) target_gone: bool,
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
    /// A hand-made world started it: the brain leaves it be until it ends.
    pub(crate) scripted: bool,
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
    fn new(
        verb: Verb,
        destination: Option<Pos>,
        target: Option<(Target, u16)>,
        scripted: bool,
        tick: u64,
    ) -> Action {
        Action {
            verb,
            destination,
            target: target.map(|(target, _)| target),
            target_type: target.map(|(_, kind)| kind),
            attempted: false,
            target_at: None,
            target_gone: false,
            started: tick,
            ticks: 0,
            blocked_ticks: 0,
            committed: None,
            ended: None,
            scripted,
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
        target: action.target,
        target_type: action.target_type,
        attempted: action.attempted,
        target_gone: action.target_gone,
        progress,
    })
}

/// Whether `sprite` has an action that hasn't ended.
pub(crate) fn is_acting(sprite: &Sprite) -> bool {
    sprite.action.as_ref().is_some_and(|a| a.ended.is_none())
}

/// Step 5 for every sprite not marked dying (design §2.4): refresh its flood
/// if it moved or the flood is due; end its action if that has timed out, or
/// lost its target or destination (5.0); then attention and the decision
/// (5a, 5b).
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
        let sprite = state.sprites.get(id).expect("the same sprite");
        if is_acting(sprite) {
            let flood = sprite.flood.as_ref().expect("the flood made above");
            let action = sprite.action.as_ref().expect("an action");
            // An aimed action heads for its target's nearest goal tile as
            // things stand now (design §3.6), so it follows a target that
            // moves; a target that's gone, or out of reach, ends it.
            let aim = action
                .target
                .map(|target| state.goal_for(data, flood, target));
            let there = action
                .target
                .and_then(|target| state.whereabouts(data, target))
                .map(|(pos, _)| pos);
            // Losing the target or the way comes first in 5.0 (design §5.5).
            // A sprite keeping to a committed way round goes on with it
            // whatever the flood reaches (§3.7), unless the target is gone.
            let gone = action.target.is_some() && there.is_none();
            let lost = aim == Some(None)
                || action
                    .destination
                    .is_some_and(|to| flood.cost(to).is_none());
            let outcome = if gone || action.committed.is_none() && lost {
                Some(Outcome::Failed)
            } else if state.tick >= action.started + u64::from(timeout) {
                Some(Outcome::TimedOut)
            } else {
                None
            };
            let sprite = state.sprites.get_mut(id).expect("the same sprite");
            let action = sprite.action.as_mut().expect("an action");
            match outcome {
                Some(outcome) => {
                    action.target_gone = gone;
                    end(action, id, outcome, state.tick, events);
                }
                None => {
                    if let Some(Some(goal)) = aim {
                        // A committed way round leads to where a sprite
                        // target was; once it moves, it's dropped (design §3.7).
                        // Where it was first seen isn't a move.
                        let moved = action.target_at.is_some_and(|at| there != Some(at));
                        if moved && matches!(action.target, Some(Target::Sprite(_))) {
                            action.committed = None;
                        }
                        action.target_at = there;
                        if action.committed.is_none() {
                            action.destination = Some(goal);
                        }
                    }
                }
            }
        }
        decide(state, data, id, events);
    }
}

/// Starts `sprite` (`id`) on an action. A Wander with no destination, or one
/// its flood doesn't reach, ends at once, as failed (design §5.5); one to the
/// tile it stands on ends at once, as applied. An aimed action heads for its
/// target's nearest goal tile, its destination; with none, it ends at once,
/// as failed. A target comes with the stable ID of its type. A `scripted`
/// action is left be by the brain.
#[expect(clippy::too_many_arguments, reason = "an action's every part")]
pub(crate) fn start(
    sprite: &mut Sprite,
    id: EntityId,
    verb: Verb,
    destination: Option<Pos>,
    target: Option<(Target, u16)>,
    scripted: bool,
    tick: u64,
    events: &mut Vec<Event>,
) {
    let mut action = Action::new(verb, destination, target, scripted, tick);
    events.push(Event {
        tick,
        kind: EventKind::ActionStarted { id, verb },
    });
    let flood = sprite.flood.as_ref().expect("step 5 made the flood");
    let lost = verb == Verb::Wander && destination.is_none_or(|to| flood.cost(to).is_none());
    if lost || (verb.is_aimed() && destination.is_none()) {
        end(&mut action, id, Outcome::Failed, tick, events);
    } else if verb == Verb::Wander && destination == Some(sprite.pos) {
        // Already there.
        end(&mut action, id, Outcome::Applied, tick, events);
    }
    sprite.action = Some(action);
}

/// Ends `action` (sprite `id`'s) with `outcome`, and reports it.
pub(crate) fn end(
    action: &mut Action,
    id: EntityId,
    outcome: Outcome,
    tick: u64,
    events: &mut Vec<Event>,
) {
    action.ended = Some(outcome);
    let view = ActionView {
        verb: action.verb,
        destination: action.destination,
        target: action.target,
        target_type: action.target_type,
        attempted: action.attempted,
        target_gone: action.target_gone,
        progress: Progress::Ended(outcome),
    };
    events.push(Event {
        tick,
        kind: EventKind::ActionEnded {
            id,
            verb: action.verb,
            outcome,
            action: view,
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
        let sprite = state.sprites.get(id).expect("a sprite taking its turn");
        // An aimed action already on a goal tile acts where it stands: it
        // has no walking to do, so it banks no points for later (design §3.7).
        let arrived = sprite
            .action
            .as_ref()
            .and_then(|a| a.target)
            .is_some_and(|target| state.on_goal_tile(data, sprite.pos, target));
        let sprite = state.sprites.get_mut(id).expect("the same sprite");
        sprite.did = Did::default();
        if is_walking(sprite) && !arrived {
            sprite.move_points += (sprite.program.traits.speed * 10.0).round() as u32;
        }
        // Counted before anyone's turn, so a swap ending an action on
        // another's turn doesn't make the count depend on the order.
        if let Some(action) = sprite.action.as_mut().filter(|a| a.ended.is_none()) {
            action.ticks += 1;
        }
    }
    // The sprites whose movement this tick is over: by stepping, or by a
    // swap; and the dying, which take no part (design §2.4), so nothing
    // swaps with them.
    let mut moved: BTreeSet<EntityId> = dying.iter().copied().collect();
    for &id in &order {
        let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
        if !is_acting(sprite) {
            continue;
        }
        let action = sprite.action.as_mut().expect("an action");
        if action.verb == Verb::Rest {
            sprite.did.rested = true;
            if action.ticks >= rest_bout {
                end(action, id, Outcome::Applied, state.tick, events);
            }
        } else if let Some(target) = action.target {
            let pos = sprite.pos;
            if !state.on_goal_tile(data, pos, target) && !moved.contains(&id) {
                walk(state, data, id, &mut moved, events);
            }
            let sprite = state.sprites.get(id).expect("the actor");
            if is_acting(sprite) && state.on_goal_tile(data, sprite.pos, target) {
                act(state, data, id, target, events);
            }
        } else if !moved.contains(&id) {
            walk(state, data, id, &mut moved, events);
        }
    }
}

/// Sprite `id`, on a goal tile of `target`, ends its aimed action: an
/// Approach has arrived, and Eat or Drink makes its one attempt (design §5.5).
fn act(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    target: Target,
    events: &mut Vec<Event>,
) {
    let verb = state.sprites.get(id).expect("the actor").action.as_ref();
    let verb = verb.expect("an action").verb;
    let outcome = match verb {
        Verb::Approach => Outcome::Applied,
        verb => verbs::attempt(state, data, id, verb, target, events),
    };
    let gone = state.whereabouts(data, target).is_none();
    let action = state
        .sprites
        .get_mut(id)
        .expect("the actor")
        .action
        .as_mut();
    let action = action.expect("an action");
    action.attempted = true;
    action.target_gone = gone;
    end(action, id, outcome, state.tick, events);
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

/// The direction of the step from `from` onto `next`, a tile beside it.
fn direction(state: &WorldState, from: Pos, next: Pos) -> Dir {
    Dir::ALL
        .into_iter()
        .find(|&d| state.map.neighbour(from, d) == Some(next))
        .expect("a path goes a step at a time")
}

/// What `sprite`'s step onto `next` costs, in tenths, or `None` if physics forbids it.
fn step_tenths(state: &WorldState, data: &DataPack, sprite: &Sprite, next: Pos) -> Option<u32> {
    let dir = direction(state, sprite.pos, next);
    step_cost(&state.map, &state.objects, data, sprite.pos, dir).map(|cost| cost * 10)
}

/// What `sprite`'s step onto `next` would cost over bare terrain, in tenths,
/// whatever stands there now. Terrain never changes, so a step on a path is
/// always walkable terrain.
fn terrain_tenths(state: &WorldState, sprite: &Sprite, next: Pos) -> u32 {
    let dir = direction(state, sprite.pos, next);
    let cost = state.map.step_cost(sprite.pos, dir);
    cost.expect("a path only crosses walkable terrain") * 10
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
        let (cost, possible) = match step_tenths(state, data, sprite, next) {
            Some(cost) => (cost, true),
            // Impossible now, onto a bush grown since the flood, say: what
            // the step costs over bare terrain decides whether it had the
            // points to be blocked.
            None => (terrain_tenths(state, sprite, next), false),
        };
        if sprite.move_points < cost {
            return;
        }
        if !possible {
            wait(state, data, id, cost, events);
            return;
        }
        if let Some(other) = state.sprites.at(next) {
            if !swaps(state, data, id, other, moved) {
                wait(state, data, id, cost, events);
                return;
            }
            let other_cost = step_tenths(
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
        && step_tenths(state, data, other, here).is_some_and(|cost| other.move_points >= cost)
}

/// Sprite `id` has just stepped, for `cost` tenths. Returns whether that
/// brought it to its destination. Arriving, it keeps at most that step's
/// worth of points (design §3.7), and a Wander ends; an aimed action acts
/// once its walk is over.
fn stepped(state: &mut WorldState, id: EntityId, cost: u32, events: &mut Vec<Event>) -> bool {
    let sprite = state.sprites.get_mut(id).expect("the walker");
    sprite.move_points -= cost;
    sprite.did.steps += 1;
    let arrived = sprite.action.as_ref().expect("an action").destination == Some(sprite.pos);
    if arrived {
        sprite.move_points = sprite.move_points.min(cost);
    }
    let action = sprite.action.as_mut().expect("an action");
    action.blocked_ticks = 0;
    if let Some(committed) = &mut action.committed {
        committed.remove(0);
    }
    if arrived && action.target.is_none() {
        end(action, id, Outcome::Applied, state.tick, events);
    }
    arrived
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
    let way = flood(state, data, sprite, Occupied::Closed).path_to(destination);
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
