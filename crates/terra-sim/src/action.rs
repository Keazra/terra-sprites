//! Actions (design §3.7, §5.5): what each sprite is doing, how an action
//! starts and ends, and how a moving one walks. Step 5 refreshes floods and
//! starts actions; step 6 carries them out.

use std::collections::BTreeSet;

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::data::DataPack;
use crate::decide::decide;
use crate::events::{Event, EventKind};
use crate::learning::{Subject, Touch};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The Cursor took hold of the sprite (design v23 §6.5).
    PulledAway,
}

/// An action to start a sprite on in a hand-made world, instead of what it
/// would choose, so tests and lab scenarios can set up exact situations.
/// When it ends, the sprite chooses for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Back away from the sprite on `at`, or else the object there, or else the water.
    Retreat { at: Pos },
}

/// How far an action has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// On its way, with this many steps of its path left, or of its
    /// retreat's bout.
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
    /// Which sprites its attempt hurt (design §4.10).
    pub hurt: Hurt,
    /// How far it has got, or how it ended.
    pub progress: Progress,
}

/// Which sprites an action's attempt hurt: the actor, biting a thornbush
/// say, or a sprite it aimed at, by hitting it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hurt {
    /// The sprite whose action it was.
    pub actor: bool,
    /// The sprite it was aimed at, if it was aimed at one.
    pub target: bool,
}

/// A sprite's action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Action {
    pub(crate) verb: Verb,
    /// Its way to where it's heading: a Wander's destination, or the goal
    /// tile an aimed action is walking to, found again at every 5.0.
    pub(crate) walk: Walk,
    /// What an aimed action is aimed at (design §5.3).
    pub(crate) target: Option<Target>,
    /// The stable ID of the target's object type.
    pub(crate) target_type: Option<u16>,
    /// Whether it got to its target and made its attempt.
    pub(crate) attempted: bool,
    /// Where its target stood at the latest 5.0.
    pub(crate) target_at: Option<Pos>,
    /// Whether its target had left the world when it ended.
    pub(crate) target_gone: bool,
    /// Which sprites its attempt hurt.
    pub(crate) hurt: Hurt,
    /// The tick it started on, for the timeout.
    pub(crate) started: u64,
    /// The ticks it has been carried out on, at step 6.
    pub(crate) ticks: u32,
    /// The steps a Retreat has taken.
    pub(crate) retreat_steps: u32,
    /// How it ended, once it has.
    pub(crate) ended: Option<Outcome>,
    /// A hand-made world started it: the brain leaves it be until it ends.
    pub(crate) scripted: bool,
}

/// A sprite's way to where it's heading (design §3.7).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Walk {
    /// Where it's heading.
    pub(crate) destination: Option<Pos>,
    /// Ticks in a row it had the points for its next step but couldn't take it.
    pub(crate) blocked_ticks: u32,
    /// The rest of the way round blocking sprites that blocked re-planning
    /// found, which the sprite keeps to while it lasts (design §3.7).
    pub(crate) committed: Option<Vec<Pos>>,
}

/// What a sprite did at step 6, which its body feels at the next tick's
/// step 3 (design §2.4).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub(crate) struct Did {
    /// The steps it took.
    pub(crate) steps: u32,
    /// Whether it rested.
    pub(crate) rested: bool,
}

impl Action {
    pub(crate) fn new(
        verb: Verb,
        destination: Option<Pos>,
        target: Option<(Target, Option<u16>)>,
        scripted: bool,
        tick: u64,
    ) -> Action {
        Action {
            verb,
            walk: Walk {
                destination,
                ..Walk::default()
            },
            target: target.map(|(target, _)| target),
            target_type: target.and_then(|(_, kind)| kind),
            attempted: false,
            target_at: None,
            target_gone: false,
            hurt: Hurt::default(),
            started: tick,
            ticks: 0,
            retreat_steps: 0,
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
    } else if action.walk.blocked_ticks > 0 {
        Progress::Waiting {
            blocked_ticks: action.walk.blocked_ticks,
        }
    } else if action.verb == Verb::Retreat {
        let bout = data.physiology().actions.retreat_bout;
        Progress::Walking {
            steps_left: bout.saturating_sub(action.retreat_steps),
        }
    } else {
        let steps_left = way_ahead(sprite).map_or(0, |way| way.len() as u32);
        Progress::Walking { steps_left }
    };
    Some(ActionView {
        verb: action.verb,
        destination: action.walk.destination,
        target: action.target,
        target_type: action.target_type,
        attempted: action.attempted,
        target_gone: action.target_gone,
        hurt: action.hurt,
        progress,
    })
}

/// While the Cursor leads `sprite`, the steps left on its way to where it's
/// heading, as near the Cursor as it can get (design v23 §6.5): 0 once
/// it's there.
pub(crate) fn lead_steps_left(sprite: &Sprite) -> Option<u32> {
    sprite
        .lead
        .as_ref()
        .map(|_| way_ahead(sprite).map_or(0, |way| way.len() as u32))
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
        // A sliding sprite chooses nothing (design v25 §2.4).
        if sprite.slide.is_some() {
            continue;
        }
        if sprite.lead.is_some() {
            follow_cursor(state, id);
            continue;
        }
        if is_acting(sprite) {
            let flood = sprite.flood.as_ref().expect("the flood made above");
            let action = sprite.action.as_ref().expect("an action");
            // An aimed action heads for its target's nearest goal tile as
            // things stand now (design §3.6), so it follows a target that
            // moves; a target that's gone, or out of reach, ends it. A
            // retreat heads for no goal tile, and getting out of reach is
            // what it's for (design §5.5).
            let aim = action
                .target
                .filter(|_| action.verb.heads_for_goal())
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
                    .walk
                    .destination
                    .is_some_and(|to| flood.cost(to).is_none());
            let outcome = if gone || action.walk.committed.is_none() && lost {
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
                        // A committed way round leads to where the target
                        // was; once it moves (a sprite, or a rolling item),
                        // it's dropped (design §3.6, §3.7). Where it was
                        // first seen isn't a move.
                        let moved = action.target_at.is_some_and(|at| there != Some(at));
                        if moved {
                            action.walk.committed = None;
                        }
                        action.target_at = there;
                        if action.walk.committed.is_none() {
                            action.walk.destination = Some(goal);
                        }
                    }
                }
            }
        }
        decide(state, data, id, events);
    }
}

/// Sprite `id`, led, heads for the Cursor's tile, or as near as its flood
/// reaches, onto no other sprite; it chooses nothing (design v23 §6.5). A
/// committed way round is dropped once the Cursor moves elsewhere.
fn follow_cursor(state: &mut WorldState, id: EntityId) {
    let tile = state
        .cursor
        .tile
        .expect("a Cursor leading a sprite has a tile");
    let sprites = &state.sprites;
    let sprite = sprites.get(id).expect("the led sprite");
    let flood = sprite.flood.as_ref().expect("step 5 made the flood");
    let to = flood.nearest_reached(tile, |pos| sprites.at(pos).is_none());
    let lead = state
        .sprites
        .get_mut(id)
        .expect("the same sprite")
        .lead
        .as_mut();
    let lead = lead.expect("a led sprite's way");
    if lead.destination != Some(to) {
        lead.committed = None;
        lead.destination = Some(to);
    }
}

/// Starts `sprite` (`id`) on an action. A Wander with no destination, or one
/// its flood doesn't reach, ends at once, as failed (design §5.5); one to the
/// tile it stands on ends at once, as applied. An aimed action heads for its
/// target's nearest goal tile, its destination; with none, it ends at once,
/// as failed. A target comes with the stable ID of its type, if it has one
/// (design v19 §3.5.5). A `scripted`
/// action is left be by the brain.
#[expect(clippy::too_many_arguments, reason = "an action's every part")]
pub(crate) fn start(
    sprite: &mut Sprite,
    id: EntityId,
    verb: Verb,
    destination: Option<Pos>,
    target: Option<(Target, Option<u16>)>,
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
    let homeless = verb.heads_for_goal() && destination.is_none();
    if lost || homeless || (verb.is_aimed() && target.is_none()) {
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
        destination: action.walk.destination,
        target: action.target,
        target_type: action.target_type,
        attempted: action.attempted,
        target_gone: action.target_gone,
        hurt: action.hurt,
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
        // A retreat has no goal tile, and a led sprite's action has ended.
        let arrived = sprite
            .action
            .as_ref()
            .filter(|a| a.verb.heads_for_goal() && sprite.lead.is_none())
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
        // A sliding sprite slid at step 2, and takes no steps (design v25
        // §2.4).
        if sprite.slide.is_some() {
            continue;
        }
        if sprite.lead.is_some() {
            if !moved.contains(&id) {
                walk(state, data, id, &mut moved, events);
            }
            continue;
        }
        if !is_acting(sprite) {
            continue;
        }
        let action = sprite.action.as_mut().expect("an action");
        if action.verb == Verb::Rest {
            sprite.did.rested = true;
            if action.ticks >= rest_bout {
                end(action, id, Outcome::Applied, state.tick, events);
            }
        } else if action.verb == Verb::Retreat {
            let target = action.target.expect("a retreat has a target");
            if !moved.contains(&id) {
                retreat(state, data, id, target, &mut moved, events);
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
    // Read before the try, which may use the target up, as eating a berry does.
    let subject = state.subject_of(data, target);
    let (outcome, hurt) = match verb {
        Verb::Approach => (Outcome::Applied, Hurt::default()),
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
    action.hurt = hurt;
    end(action, id, outcome, state.tick, events);
    if verb.is_interaction() {
        touched(state, data, id, target, subject, Some(verb));
    }
}

/// Sprite `id` touched `target`, learned about as `subject`, by trying
/// `verb` on it, or with no verb by crashing into it (design v23 §5.6): what
/// the next few ticks' feelings are about. Touching one, it knows its object
/// type (design v19 §5.6).
pub(crate) fn touched(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    target: Target,
    subject: Subject,
    verb: Option<Verb>,
) {
    let brain = &mut state.sprites.get_mut(id).expect("the toucher").brain;
    let needs = data.need_places().len();
    brain.experience.learn_about(subject, needs).touched = true;
    brain.touched = Some(Touch {
        tick: state.tick,
        verb,
        subject,
        sprite: target.sprite(),
        novelty: brain.novelty(subject),
        by_cursor: false,
    });
}

/// Shuffles `ids` with the world RNG: Fisher–Yates, one draw per place but the first.
fn shuffle(ids: &mut [EntityId], rng: &mut ChaCha8Rng) {
    for i in (1..ids.len()).rev() {
        let j = uniform(rng, i as u64 + 1) as usize;
        ids.swap(i, j);
    }
}

/// Whether `sprite` is on its way somewhere: led, and not yet where it's
/// heading, or doing an action that walks, to a destination or away from
/// its target.
fn is_walking(sprite: &Sprite) -> bool {
    match &sprite.lead {
        Some(lead) => lead.destination.is_some_and(|to| to != sprite.pos),
        None => {
            is_acting(sprite)
                && sprite
                    .action
                    .as_ref()
                    .is_some_and(|a| a.walk.destination.is_some() || a.verb == Verb::Retreat)
        }
    }
}

/// The way `sprite` is walking: its lead's while it's led, or else its action's.
fn walk_of(sprite: &Sprite) -> Option<&Walk> {
    sprite
        .lead
        .as_ref()
        .or_else(|| sprite.action.as_ref().map(|a| &a.walk))
}

/// The way `sprite` is walking, to change.
fn walk_of_mut(sprite: &mut Sprite) -> Option<&mut Walk> {
    match sprite.lead {
        Some(ref mut lead) => Some(lead),
        None => sprite.action.as_mut().map(|a| &mut a.walk),
    }
}

/// The flood `sprite` would make from where it stands now, treating other
/// sprites as `occupied` says.
pub(crate) fn flood(
    state: &WorldState,
    data: &DataPack,
    sprite: &Sprite,
    occupied: Occupied,
) -> Flood {
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
    let walk = walk_of(sprite).filter(|_| is_walking(sprite))?;
    if let Some(committed) = &walk.committed {
        return Some(committed.clone());
    }
    let flood = sprite.flood.as_ref()?;
    let path = flood.path_to(walk.destination?)?;
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
    let pos = sprite.pos;
    let walk = walk_of_mut(sprite).expect("a walker's way");
    let arrived = walk.destination == Some(pos);
    walk.blocked_ticks = 0;
    if let Some(committed) = &mut walk.committed {
        committed.remove(0);
    }
    if arrived {
        sprite.move_points = sprite.move_points.min(cost);
    }
    // A led sprite goes on following the Cursor (design v23 §6.5).
    if let Some(action) = sprite.action.as_mut().filter(|_| sprite.lead.is_none())
        && arrived
        && action.target.is_none()
    {
        end(action, id, Outcome::Applied, state.tick, events);
    }
    arrived
}

/// `sprite` had the points for a step costing `cost` tenths but couldn't
/// take it: it banks points only up to that step's cost, and counts a
/// blocked tick, starting the count again if that drops a committed path.
/// Returns the blocked ticks in a row.
fn held_up(sprite: &mut Sprite, cost: u32) -> u32 {
    sprite.move_points = sprite.move_points.min(cost);
    let walk = walk_of_mut(sprite).expect("a walker's way");
    walk.blocked_ticks = if walk.committed.take().is_some() {
        1
    } else {
        walk.blocked_ticks + 1
    };
    walk.blocked_ticks
}

/// Sprite `id` had the points for its next step but couldn't take it: it
/// banks points only up to `cost`, the step's cost, and counts a blocked
/// tick. Blocked on its committed path, it drops the path and starts
/// counting again. At `replan_after` blocked ticks in a row, it searches
/// for a way round every sprite in its way (design §3.7): a way found
/// becomes its committed path; with none, its action ends as blocked. A led
/// sprite never gives up: it waits on, and searches again later (design
/// v23 §6.5).
fn wait(state: &mut WorldState, data: &DataPack, id: EntityId, cost: u32, events: &mut Vec<Event>) {
    let replan_after = data.physiology().movement.replan_after;
    let sprite = state.sprites.get_mut(id).expect("the walker");
    if held_up(sprite, cost) < replan_after {
        return;
    }
    let destination = walk_of(sprite).and_then(|walk| walk.destination);
    let destination = destination.expect("a walker has a destination");
    let sprite = state.sprites.get(id).expect("the walker");
    let search = flood(state, data, sprite, Occupied::Closed);
    // An aimed action looks for a way to any of its target's goal tiles,
    // as one taken by a sprite may be all that blocks it; a Wander, or a
    // led sprite, for where it's heading (design §3.7).
    let aimed = sprite
        .action
        .as_ref()
        .filter(|action| sprite.lead.is_none() && action.verb.heads_for_goal())
        .and_then(|action| action.target);
    // A target gone since this tick's 5.0, such as a berry another sprite
    // ate first, ends the action as failed at the next 5.0, not as blocked
    // here (design §5.5).
    if aimed.is_some_and(|target| state.whereabouts(data, target).is_none()) {
        return;
    }
    let goal = match aimed {
        Some(target) => state.goal_for(data, &search, target),
        None => Some(destination),
    };
    let way = goal.and_then(|goal| Some((goal, search.path_to(goal)?)));
    let sprite = state.sprites.get_mut(id).expect("the walker");
    let led = sprite.lead.is_some();
    let walk = walk_of_mut(sprite).expect("a walker's way");
    match way {
        Some((goal, way)) => {
            walk.destination = Some(goal);
            walk.committed = Some(way);
            walk.blocked_ticks = 0;
        }
        None if led => walk.blocked_ticks = 0,
        None => {
            let action = sprite.action.as_mut().expect("an action");
            end(action, id, Outcome::Blocked, state.tick, events);
        }
    }
}

/// Sprite `id`'s turn to back away from `target` (design §5.5): it steps
/// while its points last, and is done after `retreat_bout` steps, keeping
/// at most the last step's worth of points, as an arrival does (§3.7).
/// With no step away it's cornered (§3.7): the retreat ends as blocked, and
/// the sprite feels the `cornered` pulse. If sprites stand on every step
/// away, it waits as a walker does, banking points only up to the step's
/// cost, and is cornered after `replan_after` blocked ticks in a row.
fn retreat(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    target: Target,
    moved: &mut BTreeSet<EntityId>,
    events: &mut Vec<Event>,
) {
    let bout = data.physiology().actions.retreat_bout;
    loop {
        let Some((there, _)) = state.whereabouts(data, target) else {
            return;
        };
        let sprite = state.sprites.get(id).expect("the retreater");
        let (next, tenths) = match step_away(state, data, sprite.pos, there) {
            Away::Step(next, cost) => (next, cost * 10),
            Away::HeldUp(cost) => {
                let tenths = cost * 10;
                if sprite.move_points < tenths {
                    return;
                }
                let sprite = state.sprites.get_mut(id).expect("the retreater");
                if held_up(sprite, tenths) >= data.physiology().movement.replan_after {
                    cornered(state, data, id, events);
                }
                return;
            }
            Away::Cornered => {
                cornered(state, data, id, events);
                return;
            }
        };
        if sprite.move_points < tenths {
            return;
        }
        state.sprites.move_to(id, next);
        moved.insert(id);
        stepped(state, id, tenths, events);
        let sprite = state.sprites.get_mut(id).expect("the retreater");
        let action = sprite.action.as_mut().expect("a retreat");
        action.retreat_steps += 1;
        if action.retreat_steps >= bout {
            end(action, id, Outcome::Applied, state.tick, events);
            sprite.move_points = sprite.move_points.min(tenths);
            return;
        }
    }
}

/// Sprite `id`'s retreat is cornered: it ends as blocked, and the sprite
/// feels the `cornered` pulse (design §3.7).
fn cornered(state: &mut WorldState, data: &DataPack, id: EntityId, events: &mut Vec<Event>) {
    let sprite = state.sprites.get_mut(id).expect("the retreater");
    sprite.body.pulse(data.physiology().indices.cornered, None);
    let action = sprite.action.as_mut().expect("a retreat");
    end(action, id, Outcome::Blocked, state.tick, events);
}

/// Which way a retreating sprite can back away.
enum Away {
    /// A step onto this tile, costing this much in terrain units.
    Step(Pos, u32),
    /// Sprites stand on every step away: the best of them would cost this much.
    HeldUp(u32),
    /// No step gains any distance, sprites or not.
    Cornered,
}

/// How a sprite on `from` backs away from `there`: onto the free neighbour
/// that gains the most Chebyshev distance from `there`, and among those the
/// one pointing most directly away, ties going by direction order (design
/// §5.5). A free neighbour is one physics lets it step onto with no sprite
/// there.
fn step_away(state: &WorldState, data: &DataPack, from: Pos, there: Pos) -> Away {
    let now = chebyshev(from, there);
    let away = (
        i64::from(from.x) - i64::from(there.x),
        i64::from(from.y) - i64::from(there.y),
    );
    let best = |free: bool| {
        let mut best: Option<(Pos, u32, u16, i64)> = None;
        for dir in Dir::ALL {
            let Some(next) = state.map.neighbour(from, dir) else {
                continue;
            };
            let Some(cost) = step_cost(&state.map, &state.objects, data, from, dir) else {
                continue;
            };
            let gained = chebyshev(next, there);
            if gained <= now || free && state.sprites.at(next).is_some() {
                continue;
            }
            let directness = directness(dir, away);
            if best.is_none_or(|(_, _, g, d)| (gained, directness) > (g, d)) {
                best = Some((next, cost, gained, directness));
            }
        }
        best
    };
    match (best(true), best(false)) {
        (Some((next, cost, ..)), _) => Away::Step(next, cost),
        (None, Some((_, cost, ..))) => Away::HeldUp(cost),
        (None, None) => Away::Cornered,
    }
}

/// How directly a step in `dir` points along `line`: the cosine of the
/// angle between them, squared with its sign kept, times a factor the same
/// for every step along that line (twice the line's squared length), so it
/// stays an exact whole number.
fn directness(dir: Dir, line: (i64, i64)) -> i64 {
    let (dx, dy) = dir.offset();
    let dot = i64::from(dx) * line.0 + i64::from(dy) * line.1;
    // A diagonal step is √2 long: dividing the square by 2 is dividing the
    // cosine by √2.
    let length_squared = i64::from(dx * dx + dy * dy);
    dot * dot.abs() * 2 / length_squared
}

/// The Chebyshev distance between two tiles: the most tiles apart they are
/// along either axis.
pub(crate) fn chebyshev(a: Pos, b: Pos) -> u16 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}
