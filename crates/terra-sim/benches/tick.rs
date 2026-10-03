//! Benchmarks for the performance target (design §3.9, §7.4 A5): a whole
//! tick of the default world with 100 sprites, and the two costliest parts
//! of one sprite's turn, its flood and its brain step.
//!
//! `cargo bench -p terra-sim`
//!
//! The quick check of A5 itself is `cargo run --release -p terra-sim --example
//! speed`, which reports ticks a second.

use criterion::{Criterion, criterion_group, criterion_main};
use terra_sim::bench::{crowded_world, decide_for, flood_of};
use terra_sim::{EntityId, World};

/// The seed the benchmarked world is made from.
const SEED: u64 = 7;

/// The sprite each per-sprite benchmark measures: the first.
fn first_sprite(world: &World) -> EntityId {
    world.sprites().next().expect("a sprite").id()
}

fn whole_tick(c: &mut Criterion) {
    let mut world = crowded_world(SEED);
    c.bench_function("tick, 100 sprites", |b| b.iter(|| world.step()));
}

fn flood(c: &mut Criterion) {
    let mut world = crowded_world(SEED);
    let id = first_sprite(&world);
    c.bench_function("one sprite's flood", |b| {
        b.iter(|| flood_of(&mut world, id))
    });
}

fn brain_step(c: &mut Criterion) {
    let mut world = crowded_world(SEED);
    let id = first_sprite(&world);
    c.bench_function("one sprite's brain step", |b| {
        b.iter(|| decide_for(&mut world, id))
    });
}

criterion_group!(benches, whole_tick, flood, brain_step);
criterion_main!(benches);
