//! The cross-platform determinism check (design §2.6, §7.5): runs the default
//! world from a fixed seed for a fixed number of ticks, and prints the state
//! hash at every checkpoint. Then it runs the same world again with the
//! soak's script of the Cursor's commands (§7.6), so the commands a
//! player's clicks send are compared too.
//!
//! `cargo run --release -p terra-sim --example determinism`
//!
//! CI runs it on every runner in the determinism matrix and compares what
//! each printed: any difference fails the check. Its lines name nothing
//! about the machine, so the same simulation prints the same text anywhere.

use terra_sim::{DataPack, Soak, World, WorldConfig};

/// The seed the default world is made from.
const SEED: u64 = 7;

/// How many ticks the world runs.
const TICKS: u64 = 20_000;

/// How many ticks the world runs with the soak's script.
const SOAK_TICKS: u64 = 10_000;

/// How often a hash is printed, as for a replay's checkpoints (design §2.7):
/// a mismatch then names the stretch of ticks it began in.
const CHECKPOINT_EVERY: u64 = 1_000;

fn main() {
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let mut world = World::new(WorldConfig::builtin(&data), data.clone(), SEED);
    println!("default world, seed {SEED}, {TICKS} ticks");
    checkpoint(&world);
    while world.tick() < TICKS {
        world.step();
        if world.tick().is_multiple_of(CHECKPOINT_EVERY) {
            checkpoint(&world);
        }
    }

    let mut world = World::new(WorldConfig::builtin(&data), data, SEED);
    let mut script = Soak::new(&world, SEED);
    println!("default world with the soak's commands, seed {SEED}, {SOAK_TICKS} ticks");
    checkpoint(&world);
    while world.tick() < SOAK_TICKS {
        for command in script.commands(&world) {
            world.submit(command);
        }
        world.step();
        if world.tick().is_multiple_of(CHECKPOINT_EVERY) {
            checkpoint(&world);
        }
    }
}

/// One checkpoint's line: the tick, the state hash, and how many sprites and
/// objects there are, which the hash covers too but a person can read.
fn checkpoint(world: &World) {
    println!(
        "tick {:>6}  hash {:016x}  sprites {:>3}  objects {:>4}",
        world.tick(),
        world.state_hash(),
        world.sprites().count(),
        world.objects().count(),
    );
}
