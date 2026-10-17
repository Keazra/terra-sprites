//! Story mode and Lab mode (M2 design §16): a world keeps the mode it was
//! made in for its whole life, through its saves and its replays, and a
//! world from before the modes is a Story world. In milestone 2 the mode
//! changes nothing in how a world runs.

mod common;
use common::builtin;

use terra_sim::{Mode, Playback, World, WorldConfig};

/// The default world from `seed`, made in `mode`.
fn world_in(mode: Mode, seed: u64) -> World {
    let data = builtin();
    World::new_in(WorldConfig::builtin(&data), data, seed, mode)
}

#[test]
fn a_new_world_is_a_story_world_unless_made_in_lab_mode() {
    let data = builtin();
    let world = World::new(WorldConfig::builtin(&data), data, 7);
    assert_eq!(world.mode(), Mode::Story);
    assert_eq!(world_in(Mode::Lab, 7).mode(), Mode::Lab);
}

#[test]
fn the_mode_changes_nothing_in_how_a_world_runs() {
    let (mut story, mut lab) = (world_in(Mode::Story, 7), world_in(Mode::Lab, 7));
    for _ in 0..300 {
        assert_eq!(story.step(), lab.step(), "at tick {}", story.tick());
    }
}

#[test]
fn the_mode_is_in_the_state_hash() {
    // M2 design §16.1: the same seed makes the same world in both modes,
    // but the mode is world state, so their hashes differ.
    let (story, lab) = (world_in(Mode::Story, 7), world_in(Mode::Lab, 7));
    assert_ne!(story.state_hash(), lab.state_hash());
}

#[test]
fn a_save_keeps_its_worlds_mode() {
    let mut world = world_in(Mode::Lab, 7);
    for _ in 0..20 {
        world.step();
    }
    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.mode(), Mode::Lab);
    assert_eq!(loaded.state_hash(), world.state_hash());
}

#[test]
fn a_save_from_before_the_modes_loads_as_a_story_world() {
    let world = World::load(include_bytes!("golden/save-v1.tspr")).expect("the golden save loads");
    assert_eq!(world.mode(), Mode::Story);
}

#[test]
fn a_lab_worlds_replay_starts_afresh_in_lab_mode() {
    let mut world = world_in(Mode::Lab, 7);
    world.start_recording();
    for _ in 0..1_100 {
        world.step();
    }
    let bytes = world.replay().expect("the world records");
    let mut playback = Playback::new(&bytes).expect("the replay plays");
    assert!(playback.started_fresh(), "from its seed, not from a save");
    assert_eq!(playback.world().mode(), Mode::Lab);
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
}
