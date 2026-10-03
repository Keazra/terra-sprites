//! Hooks for the benchmarks (design §3.9, §7.4 A5): the costliest parts of a
//! tick, for one sprite at a time, which the rest of the public API doesn't
//! offer. Nothing else should call them.

use std::hint::black_box;

use crate::action::flood;
use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::decide::decide;
use crate::objects::EntityId;
use crate::perception::Occupied;
use crate::world::World;

/// The most sprites a preset allows (design §3.9).
const CROWD: u16 = 100;

/// Ticks run before measuring, so sprites are spread out and busy, as in
/// play, rather than standing where they were placed.
const SETTLE: u64 = 300;

/// The default world made from `seed`, with the most sprites a preset
/// allows, after it has settled: what A5 is measured on (design §7.4).
pub fn crowded_world(seed: u64) -> World {
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let config = WorldConfig::builtin(&data).with_sprites(CROWD);
    let mut world = World::new(config, data, seed);
    while world.tick() < SETTLE {
        world.step();
    }
    world
}

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
/// world as those steps do: the sprite may start or end an action, so
/// repeated calls drift from where a tick left it. It runs outside a tick,
/// and the self-check doesn't follow it.
pub fn decide_for(world: &mut World, id: EntityId) {
    let (state, data) = world.parts();
    let mut events = Vec::new();
    decide(state, data, id, &mut events);
    black_box(events);
}
