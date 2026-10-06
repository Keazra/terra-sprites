//! Replays (design §2.7): a recorded session plays back exactly, every
//! checkpoint matching; playback says at which checkpoint a world that has
//! parted from its recording was first found different; and a replay from
//! another build, or with the wrong data pack, is refused, never a crash.

mod common;
use common::{at, builtin};

use std::panic::{catch_unwind, AssertUnwindSafe};

use terra_sim::{
    Command, Dir, Divergence, EventKind, Map, Playback, ReplayError, Scenario, World, WorldConfig,
    CHECKPOINT_EVERY,
};

/// The default world, recording from its first tick.
fn recorded_world(seed: u64) -> World {
    let data = builtin();
    let mut world = World::new(WorldConfig::builtin(&data), data, seed);
    world.start_recording();
    world
}

/// Runs `world` on to `until`, the Cursor busy as a player's would be: it
/// leads a sprite, lets go, has a let-go refused for leading nothing, picks
/// up a ball and throws it, and pets a sprite. Returns each tick's events.
fn play(world: &mut World, until: u64) -> Vec<Vec<terra_sim::Event>> {
    let mut events = Vec::new();
    while world.tick() < until {
        match world.tick() % 1_500 {
            40 => {
                let sprite = world.sprites().next().expect("a sprite").id();
                world.submit(Command::TakeHold { sprite });
                let pos = world.sprite(sprite).expect("alive").pos();
                world.submit(Command::MoveCursor {
                    tile: at(pos.x.saturating_sub(3), pos.y),
                });
            }
            70 => world.submit(Command::LetGo),
            // Leading nothing, so it's refused.
            71 => world.submit(Command::LetGo),
            90 => {
                let ball = world.objects().find(|o| o.type_name() == "ball");
                if let Some(item) = ball.map(|ball| ball.id()) {
                    world.submit(Command::PickUp { item });
                }
            }
            95 => {
                if world.cursor().holds().is_some() {
                    world.submit(Command::Throw {
                        from: at(10, 10),
                        toward: Dir::E,
                        tiles: 4,
                    });
                }
            }
            120 => {
                let sprite = world.sprites().last().expect("a sprite").id();
                world.submit(Command::Reward {
                    sprite,
                    amplified: true,
                    reach_back: 5,
                });
            }
            _ => {}
        }
        events.push(world.step());
    }
    events
}

/// Plays `playback` to its end, returning each tick's events.
fn play_back(playback: &mut Playback) -> Vec<Vec<terra_sim::Event>> {
    let mut events = Vec::new();
    while playback.world().tick() < playback.end() {
        events.push(playback.step());
    }
    events
}

#[test]
fn replaying_a_recorded_session_reproduces_every_checkpoint() {
    let mut world = recorded_world(7);
    let recorded = play(&mut world, 2_500);
    let bytes = world.replay().expect("the world records");

    let mut playback = Playback::new(&bytes).expect("the replay plays");
    assert!(playback.started_fresh(), "a new world starts afresh");
    assert_eq!(playback.start(), 0);
    assert_eq!(playback.end(), 2_500);
    let replayed = play_back(&mut playback);
    assert_eq!(playback.divergence(), None);
    assert_eq!(replayed, recorded, "the same events, tick by tick");
    assert_eq!(playback.world().state_hash(), world.state_hash());
    // The refused let-go was recorded and refused again.
    let refused = |events: &[Vec<terra_sim::Event>]| {
        events
            .iter()
            .flatten()
            .filter(|e| matches!(e.kind, EventKind::CommandRejected { .. }))
            .count()
    };
    assert!(refused(&replayed) > 0, "a refusal replayed");
    assert_eq!(refused(&replayed), refused(&recorded));
}

#[test]
fn a_session_started_from_a_save_replays_from_it() {
    let data = builtin();
    let mut world = World::new(WorldConfig::builtin(&data), data, 3);
    play(&mut world, 300);
    // A click made while paused waits in the save, for the next tick.
    let sprite = world.sprites().next().expect("a sprite").id();
    world.submit(Command::Correct {
        sprite,
        amplified: false,
    });
    let mut world = World::load(&world.save()).expect("the save loads");
    world.start_recording();
    play(&mut world, 2_100);

    let mut playback = Playback::new(&world.replay().unwrap()).expect("the replay plays");
    assert!(
        !playback.started_fresh(),
        "a loaded world starts from its save"
    );
    assert_eq!(playback.start(), 300);
    play_back(&mut playback);
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
}

#[test]
fn a_hand_made_world_replays_from_its_save() {
    let data = builtin();
    let map = Map::from_ascii(&["......", "......", "......", "......"], &data).unwrap();
    let scenario = Scenario {
        map,
        objects: &[(at(4, 1), "ball")],
        sprites: &[(at(1, 1), None), (at(2, 3), None)],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, 5).unwrap();
    world.start_recording();
    let item = world.objects().next().unwrap().id();
    world.submit(Command::PickUp { item });
    for _ in 0..1_200 {
        world.step();
    }
    let mut playback = Playback::new(&world.replay().unwrap()).unwrap();
    assert!(
        !playback.started_fresh(),
        "a hand-made world can't be generated"
    );
    play_back(&mut playback);
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
}

#[test]
fn a_world_that_has_run_records_from_its_save_not_its_seed() {
    let data = builtin();
    let mut world = World::new(WorldConfig::builtin(&data), data, 4);
    world.step();
    world.start_recording();
    let playback = Playback::new(&world.replay().unwrap()).unwrap();
    assert!(!playback.started_fresh());
    assert_eq!(playback.start(), 1);
    assert_eq!(playback.divergence(), None);
}

#[test]
fn an_injected_divergence_is_reported_at_the_first_checkpoint_after_it() {
    let mut world = recorded_world(11);
    play(&mut world, 3_500);
    let mut playback = Playback::new(&world.replay().unwrap()).unwrap();
    while playback.world().tick() < 1_500 {
        playback.step();
    }
    assert_eq!(playback.divergence(), None, "matching so far");
    // A name the recording never gave.
    let sprite = playback.world().sprites().next().expect("a sprite").id();
    playback.inject(Command::Rename {
        sprite,
        name: "Stray".into(),
    });
    while playback.world().tick() < 1_999 {
        playback.step();
    }
    assert_eq!(playback.divergence(), None, "not found until a checkpoint");
    playback.step();
    let found = Some(Divergence {
        tick: 2 * CHECKPOINT_EVERY,
        matched: Some(CHECKPOINT_EVERY),
    });
    assert_eq!(playback.divergence(), found);
    // It keeps the first.
    play_back(&mut playback);
    assert_eq!(playback.divergence(), found);
}

#[test]
fn a_session_that_ends_in_a_panic_still_leaves_a_valid_replay() {
    let mut world = recorded_world(2);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        play(&mut world, 1_300);
        panic!("something broke");
    }));
    assert!(panicked.is_err());
    let mut playback = Playback::new(&world.replay().unwrap()).expect("a valid replay");
    assert_eq!(playback.end(), 1_300);
    play_back(&mut playback);
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
}

#[test]
fn a_world_not_recording_has_no_replay() {
    let data = builtin();
    let world = World::new(WorldConfig::builtin(&data), data, 1);
    assert!(world.replay().is_none());
}

/// Where a replay's body starts, after the magic and the header.
fn body_start(bytes: &[u8]) -> usize {
    let mut rest = &bytes[4..];
    let _: serde::de::IgnoredAny = rmp_serde::from_read(&mut rest).expect("a header");
    bytes.len() - rest.len()
}

/// The position just after the first `field` at or after `from`.
fn after(bytes: &[u8], from: usize, field: &[u8]) -> usize {
    from + bytes[from..]
        .windows(field.len())
        .position(|window| window == field)
        .expect("the field")
        + field.len()
}

/// Puts the header's checksum right after the body was changed.
fn fix_checksum(bytes: &mut [u8]) {
    let body = body_start(bytes);
    let sum = xxhash_rust::xxh3::xxh3_64(&bytes[body..]);
    // A u64 in MessagePack: 0xcf, then 8 bytes, big-endian.
    let at = after(bytes, 0, b"checksum");
    assert_eq!(bytes[at], 0xcf);
    bytes[at + 1..at + 9].copy_from_slice(&sum.to_be_bytes());
}

#[test]
fn a_replay_from_another_build_is_refused_saying_so() {
    let mut world = recorded_world(1);
    play(&mut world, 10);
    let bytes = world.replay().unwrap();

    // Another sim version: its first digit changed.
    let mut other = bytes.clone();
    let at = after(&other, 0, b"sim_version") + 1;
    other[at] = b'9';
    match Playback::new(&other) {
        Err(err @ ReplayError::OtherBuild { .. }) => {
            let message = err.to_string();
            assert!(
                message.contains("only in the build that recorded it"),
                "{message}"
            );
            assert!(
                message.contains("recorded by Terra Sprites 9."),
                "{message}"
            );
        }
        Err(err) => panic!("refused for the wrong reason: {err}"),
        Ok(_) => panic!("another build's replay played"),
    }

    // Another save format.
    let mut other = bytes;
    let at = after(&other, 0, b"schema_version");
    other[at] += 1;
    assert!(matches!(
        Playback::new(&other),
        Err(ReplayError::OtherBuild { .. })
    ));
}

#[test]
fn a_replay_naming_another_data_pack_is_refused_saying_so() {
    for fresh in [true, false] {
        let mut world = if fresh {
            recorded_world(1)
        } else {
            let data = builtin();
            let mut world = World::new(WorldConfig::builtin(&data), data, 1);
            world.step();
            world.start_recording();
            world
        };
        play(&mut world, 20);
        let mut bytes = world.replay().unwrap();
        // The pack it names comes first: `core` becomes `cord`.
        let body = body_start(&bytes);
        let name = after(&bytes, body, b"name") + 1;
        assert_eq!(&bytes[name..name + 4], b"core");
        bytes[name + 3] = b'd';
        fix_checksum(&mut bytes);

        match Playback::new(&bytes) {
            Err(err @ ReplayError::PackMismatch { .. }) => {
                let message = err.to_string();
                assert!(message.contains("names the data pack cord 1"), "{message}");
                assert!(message.contains("made with core 1"), "{message}");
            }
            Err(err) => panic!("refused for the wrong reason: {err}"),
            Ok(_) => panic!("a replay naming another pack played"),
        }
    }
}

#[test]
fn what_isnt_a_replay_is_refused() {
    let data = builtin();
    let save = World::new(WorldConfig::builtin(&data), data, 1).save();
    for bytes in [&b""[..], b"hello", b"TSRP", &save] {
        assert!(
            matches!(Playback::new(bytes), Err(ReplayError::NotAReplay)),
            "{:?}",
            &bytes[..bytes.len().min(8)]
        );
    }
    // Nor is a replay a save.
    let mut world = recorded_world(1);
    world.step();
    assert!(World::load(&world.replay().unwrap()).is_err());
}

#[test]
fn a_damaged_replay_is_refused_without_a_crash() {
    let mut world = recorded_world(2);
    play(&mut world, 40);
    let bytes = world.replay().unwrap();
    let step = (bytes.len() / 97).max(1);
    for len in (0..bytes.len()).step_by(step) {
        assert!(Playback::new(&bytes[..len]).is_err(), "cut at {len} played");
    }
    let body = body_start(&bytes);
    for index in (body..bytes.len()).step_by(step) {
        let mut damaged = bytes.clone();
        damaged[index] ^= 0xa5;
        assert!(
            matches!(Playback::new(&damaged), Err(ReplayError::Damaged(_))),
            "a change at {index} played"
        );
    }
}

#[test]
fn a_replay_with_commands_after_its_end_is_refused() {
    let mut world = recorded_world(3);
    play(&mut world, 100);
    let mut bytes = world.replay().unwrap();
    // The end is the body's last field: 100, a one-byte number, made 30,
    // before the commands recorded at ticks 40 to 95.
    let at = after(&bytes, body_start(&bytes), b"\xa3end");
    assert_eq!(at + 1, bytes.len(), "the end is the last byte");
    assert_eq!(bytes[at], 100);
    bytes[at] = 30;
    fix_checksum(&mut bytes);
    assert!(
        matches!(Playback::new(&bytes), Err(ReplayError::Damaged(_))),
        "a command after the end played"
    );
}
