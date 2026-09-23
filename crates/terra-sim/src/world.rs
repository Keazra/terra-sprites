use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::Serialize;
use xxhash_rust::xxh3::xxh3_64_with_seed;

use crate::data::DataPack;

/// Fixed seed for `state_hash`, so hashes are comparable across runs and builds.
const STATE_HASH_SEED: u64 = 0x7e22_a5b1_17e5_0001;

/// A simulated world, advanced one tick at a time.
pub struct World {
    state: WorldState,
}

/// Everything that determines how the world evolves. Hashed by `state_hash`.
#[derive(Serialize)]
struct WorldState {
    tick: u64,
    /// The world's only source of randomness (design §2.3).
    rng: ChaCha8Rng,
}

/// A broken internal invariant: always a bug in the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation(pub String);

impl World {
    pub fn new(_data: DataPack, seed: u64) -> World {
        World {
            state: WorldState {
                tick: 0,
                rng: ChaCha8Rng::seed_from_u64(seed),
            },
        }
    }

    /// Advances the world by exactly one tick, running the canonical tick order
    /// (design §2.4). Later slices fill in the steps that are empty today.
    pub fn step(&mut self) {
        self.apply_commands(); // 1
        self.run_environment(); // 2
        self.run_biochemistry(); // 3
        self.run_learning(); // 4
        self.sense_and_decide(); // 5
        self.resolve_actions(); // 6
        self.finish_tick(); // 7
    }

    /// The number of ticks simulated so far.
    pub fn tick(&self) -> u64 {
        self.state.tick
    }

    /// A stable hash of the whole world state: xxh3 over its MessagePack encoding.
    pub fn state_hash(&self) -> u64 {
        let bytes = rmp_serde::to_vec_named(&self.state)
            .expect("world state always serializes to MessagePack");
        xxh3_64_with_seed(&bytes, STATE_HASH_SEED)
    }

    /// Step 1: apply the commands stamped for this tick.
    fn apply_commands(&mut self) {}

    /// Step 2: object lifecycle rules.
    fn run_environment(&mut self) {}

    /// Step 3: pulse latch, physics, reactions, decay, emitters, receptors; death check #1.
    fn run_biochemistry(&mut self) {}

    /// Step 4: reinforcement from consumed reward and punishment.
    fn run_learning(&mut self) {}

    /// Step 5: perception, attention and decisions.
    fn sense_and_decide(&mut self) {}

    /// Step 6: movement and verb effects, then trace entries.
    fn resolve_actions(&mut self) {}

    /// Step 7: death check #2, removals and events; then the tick counter advances.
    fn finish_tick(&mut self) {
        self.state.tick += 1;
    }

    /// Checks the world's internal invariants. Later slices add checks here.
    pub fn check_invariants(&self) -> Result<(), InvariantViolation> {
        Ok(())
    }
}
