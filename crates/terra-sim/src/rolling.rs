//! Rolling items (design §3.5.4): a push sets an item rolling, and at the
//! end of step 2 each rolling item moves one tile.

use crate::data::DataPack;
use crate::map::{Dir, Pos};
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

/// Every rolling item, in ascending ID order, moves one tile.
pub(crate) fn run(state: &mut WorldState, data: &DataPack) {
    let rolling: Vec<EntityId> = state
        .objects
        .iter()
        .filter(|(_, object)| object.roll.is_some())
        .map(|(id, _)| id)
        .collect();
    for id in rolling {
        roll(state, data, id);
    }
}

/// The item `id` rolls a tile on its way, bouncing if something stops it.
/// Either way, that's one of its tiles used up.
fn roll(state: &mut WorldState, data: &DataPack, id: EntityId) {
    let object = state.objects.get(id).expect("a rolling item");
    let (from, Some(Roll { mut dir, left })) = (object.pos, object.roll) else {
        return;
    };
    let mut to = open(state, data, from, dir);
    if to.is_none() {
        dir = dir.reverse();
        to = open(state, data, from, dir);
    }
    if let Some(to) = to {
        state.objects.move_to(id, to);
    }
    let object = state.objects.get_mut(id).expect("the same item");
    object.roll = (left > 1).then_some(Roll { dir, left: left - 1 });
}

/// The tile a step from `from` in direction `dir` reaches, if a rolling item
/// may go there: on the map, walkable, holding no sprite or object, and not
/// past a corner.
fn open(state: &WorldState, data: &DataPack, from: Pos, dir: Dir) -> Option<Pos> {
    step_cost(&state.map, &state.objects, data, from, dir)?;
    let to = state.map.neighbour(from, dir)?;
    let empty = state.sprites.at(to).is_none() && state.objects.at(to).is_none();
    empty.then_some(to)
}
