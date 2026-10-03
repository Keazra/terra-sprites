//! Hooks for the benchmarks (design §3.9, §7.4 A5): the costliest parts of a
//! tick, for one sprite at a time, which the rest of the public API doesn't
//! offer. Nothing else should call them.

use std::hint::black_box;

use crate::action::flood;
use crate::decide::decide;
use crate::objects::EntityId;
use crate::perception::Occupied;
use crate::world::World;

/// Makes sprite `id`'s flood afresh, as step 5 does when it's stale (design
/// §3.6), and throws it away.
pub fn flood_of(world: &mut World, id: EntityId) {
    let (state, data) = world.parts();
    let sprite = state.sprites.get(id).expect("a sprite in the world");
    let penalty = Occupied::Penalty(data.physiology().movement.occupied_penalty);
    black_box(flood(state, data, sprite, penalty));
}

/// Runs steps 5a and 5b for sprite `id`, attention and the decision, on the
/// flood it has (design §5.3, §5.5), which a tick has made. It changes the
/// world as those steps do: the sprite may start or end an action.
pub fn decide_for(world: &mut World, id: EntityId) {
    let (state, data) = world.parts();
    let mut events = Vec::new();
    decide(state, data, id, &mut events);
    black_box(events);
}
