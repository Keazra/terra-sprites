//! Benchmarks for the performance target (design §3.9, §7.4 A5): a whole
//! tick of the default world with 100 sprites, and the two costliest parts
//! of one sprite's turn, its flood and its brain step.
//!
//! `cargo bench -p terra-sim`
//!
//! The quick check of A5 itself is `cargo run --release -p terra-sim --example
//! speed`, which reports ticks a second.

use criterion::{Criterion, criterion_group, criterion_main};
use terra_sim::bench::{decide_for, flood_of};
use terra_sim::{DataPack, EntityId, World, WorldConfig};

/// The seed the benchmarked world is made from.
const SEED: u64 = 7;

/// Ticks run before measuring, so sprites are spread out and busy, as in
/// play, rather than standing where they were placed.
const SETTLE: u64 = 300;

/// The default world, with the most sprites a preset allows (design §3.9),
/// settled.
fn crowded_world() -> World {
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let default = include_str!("../../../data/presets/default.ron");
    let preset = default.replace("sprites: 30,", "sprites: 100,");
    assert_ne!(preset, default, "the default preset names its sprite count");
    let config = WorldConfig::from_ron(&preset, &data).expect("a valid preset");
    let mut world = World::new(config, data, SEED);
    while world.tick() < SETTLE {
        world.step();
    }
    world
}

/// The sprite each per-sprite benchmark measures: the first.
fn first_sprite(world: &World) -> EntityId {
    world.sprites().next().expect("a sprite").id()
}

fn whole_tick(c: &mut Criterion) {
    let mut world = crowded_world();
    c.bench_function("tick, 100 sprites", |b| b.iter(|| world.step()));
}

fn flood(c: &mut Criterion) {
    let mut world = crowded_world();
    let id = first_sprite(&world);
    c.bench_function("one sprite's flood", |b| {
        b.iter(|| flood_of(&mut world, id))
    });
}

fn brain_step(c: &mut Criterion) {
    let mut world = crowded_world();
    let id = first_sprite(&world);
    c.bench_function("one sprite's brain step", |b| {
        b.iter(|| decide_for(&mut world, id))
    });
}

criterion_group!(benches, whole_tick, flood, brain_step);
criterion_main!(benches);
