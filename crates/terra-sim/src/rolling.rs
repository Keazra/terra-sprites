//! Rolling items (design §3.5.4): a push sets an item rolling, and at the
//! end of step 2 each rolling item moves one tile, bouncing off what's bigger,
//! knocking on what's its size and crushing what's smaller and softer.

use std::collections::BTreeSet;

use crate::data::DataPack;
use crate::ecology;
use crate::events::{Event, Removal};
use crate::map::{Dir, Pos};
use crate::object_types::{Build, ObjectType};
use crate::objects::{EntityId, Roll};
use crate::physics::step_cost;
use crate::world::WorldState;

/// Sprite `actor` pushes the item `id` up to `tiles` away (design §3.5.2),
/// setting it rolling from the next tick. A push on an item already rolling
/// starts a fresh roll.
pub(crate) fn push(state: &mut WorldState, actor: EntityId, id: EntityId, tiles: u16) {
    let pusher = state.sprites.get(actor).expect("the pusher");
    let (from, last_step) = (pusher.pos, pusher.last_step);
    let object = state.objects.get_mut(id).expect("the pushed item");
    // A pusher acts from a goal tile, which for an item is its own tile or
    // one beside it (design §3.6), so the direction away from it is exact.
    let dir = Dir::towards(from, object.pos)
        .or(last_step)
        .unwrap_or(Dir::N);
    object.roll = (tiles > 0).then_some(Roll { dir, left: tiles });
}

/// Every rolling item, in ascending ID order, moves one tile. An item moves
/// at most once a tick, so one knocked on waits for the next.
pub(crate) fn run(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let rolling: Vec<EntityId> = state
        .objects
        .iter()
        .filter(|(_, object)| object.roll.is_some())
        .map(|(id, _)| id)
        .collect();
    let mut knocked = BTreeSet::new();
    for id in rolling {
        // A rolling item crushed earlier in the tick is gone.
        if state.objects.get(id).is_some() && !knocked.contains(&id) {
            roll(state, data, id, &mut knocked, events);
        }
    }
}

/// What a rolling item finds on the tile a step away (design §3.5.4).
enum Meeting {
    /// Nothing: it moves there.
    Open(Pos),
    /// An item it swaps rolls with.
    KnockOn(EntityId),
    /// A smaller, softer item it destroys, moving onto its tile.
    Crush(Pos, EntityId),
    /// Something that stops it.
    Bounce,
}

/// The item `id` rolls a tile on its way, bouncing if something stops it.
/// Either way, that's one of its tiles used up.
fn roll(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    knocked: &mut BTreeSet<EntityId>,
    events: &mut Vec<Event>,
) {
    let object = state.objects.get(id).expect("a rolling item");
    let (from, Some(Roll { mut dir, left })) = (object.pos, object.roll) else {
        return;
    };
    let mut meeting = meet(state, data, id, from, dir);
    if let Meeting::Bounce = meeting {
        dir = bounce(dir, |side| {
            matches!(meet(state, data, id, from, side), Meeting::Bounce)
        });
        meeting = meet(state, data, id, from, dir);
    }
    let rest = (left > 1).then_some(Roll {
        dir,
        left: left - 1,
    });
    let roll = match meeting {
        Meeting::Open(to) => {
            state.objects.move_to(id, to);
            rest
        }
        Meeting::Crush(to, other) => {
            let crushed = state.objects.remove(other);
            ecology::removed(state, data, other, crushed.kind, Removal::Destroyed, events);
            state.objects.move_to(id, to);
            rest
        }
        Meeting::KnockOn(other) => {
            // The two swap rolls, an exchange of momentum (design §3.5.4).
            knocked.insert(other);
            let ahead = state.objects.get_mut(other).expect("the item ahead");
            std::mem::replace(&mut ahead.roll, rest)
        }
        // A bounce with nowhere to go ends the roll where it is.
        Meeting::Bounce => None,
    };
    state.objects.get_mut(id).expect("the same item").roll = roll;
}

/// What the item `id`, on `from`, meets a step away in direction `dir`. It
/// bounces off the map's edge, unwalkable terrain, a corner it may not cut, a
/// sprite, anything solid and anything bigger than itself.
fn meet(state: &WorldState, data: &DataPack, id: EntityId, from: Pos, dir: Dir) -> Meeting {
    if step_cost(&state.map, &state.objects, data, from, dir).is_none() {
        return Meeting::Bounce;
    }
    let Some(to) = state.map.neighbour(from, dir) else {
        return Meeting::Bounce;
    };
    if state.sprites.at(to).is_some() {
        return Meeting::Bounce;
    }
    let Some(other) = state.objects.at(to) else {
        return Meeting::Open(to);
    };
    let type_of = |id| &data.object_types()[state.objects.kind(id)];
    if type_of(other).solid {
        return Meeting::Bounce;
    }
    let (mine, theirs) = (build(type_of(id)), build(type_of(other)));
    if theirs.size > mine.size {
        Meeting::Bounce
    } else if theirs.size < mine.size && theirs.hardness < mine.hardness {
        Meeting::Crush(to, other)
    } else {
        Meeting::KnockOn(other)
    }
}

/// The size and hardness of a type with objects, which the pack guarantees it has.
fn build(object_type: &ObjectType) -> Build {
    object_type
        .build
        .expect("a type with objects has a size and a hardness")
}

/// The way an item rolling in direction `dir` goes when something stops it
/// (design §3.5.4), where `stopped` says whether a roll in an orthogonal
/// direction would be stopped too. Head on, it goes back the way it came;
/// slantwise, with just one of the two sides beside a diagonal stopping it, it
/// glances off, reversing only the part of its way that ran into it.
fn bounce(dir: Dir, stopped: impl Fn(Dir) -> bool) -> Dir {
    let Some((across, along)) = dir.parts() else {
        return dir.reverse();
    };
    match (stopped(across), stopped(along)) {
        (true, false) => dir.mirrored(across),
        (false, true) => dir.mirrored(along),
        _ => dir.reverse(),
    }
}
