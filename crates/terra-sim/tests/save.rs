//! Saves (design §2.8): a loaded world carries on exactly as the one saved
//! would have, with the data pack its save embeds; a save from a newer
//! build, or a damaged one, is refused rather than crashing.

use terra_sim::{
    Command, DataPack, EntityId, EventKind, Genome, LoadError, Map, Pos, SCHEMA_VERSION, Scenario,
    World, WorldConfig,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// The default world, with the Cursor busy: it leads a sprite for a while,
/// lets go, then picks up a ball and holds it.
fn busy_world(seed: u64, ticks: u64) -> World {
    let data = builtin();
    let mut world = World::new(WorldConfig::builtin(&data), data, seed);
    for _ in 0..ticks {
        match world.tick() {
            40 => {
                let sprite = world.sprites().next().expect("a sprite").id();
                world.submit(Command::TakeHold { sprite });
                let pos = world.sprite(sprite).expect("alive").pos();
                world.submit(Command::MoveCursor {
                    tile: at(pos.x.saturating_sub(3), pos.y),
                });
            }
            70 => world.submit(Command::LetGo),
            90 => {
                let ball = world
                    .objects()
                    .find(|o| o.type_name() == "ball")
                    .expect("a ball")
                    .id();
                world.submit(Command::PickUp { item: ball });
            }
            _ => {}
        }
        world.step();
    }
    world
}

#[test]
fn a_loaded_world_carries_on_exactly_as_one_never_saved() {
    let mut original = busy_world(7, 150);
    assert!(
        original.cursor().holds().is_some(),
        "the Cursor holds a ball"
    );
    // A pet waits for the next tick, as a click made while paused does.
    let sprite = original.sprites().next().expect("a sprite").id();
    original.submit(Command::Reward {
        sprite,
        amplified: false,
        reach_back: 3,
    });

    let mut loaded = World::load(&original.save()).expect("the save loads");
    assert_eq!(loaded.tick(), original.tick());
    assert_eq!(loaded.state_hash(), original.state_hash());
    for _ in 0..200 {
        let (a, b) = (original.step(), loaded.step());
        assert_eq!(a, b, "the events differ at tick {}", original.tick());
        assert_eq!(
            loaded.state_hash(),
            original.state_hash(),
            "diverged at tick {}",
            original.tick()
        );
    }
}

#[test]
fn saving_a_loaded_world_writes_the_same_bytes() {
    let original = busy_world(3, 120);
    let bytes = original.save();
    let loaded = World::load(&bytes).expect("the save loads");
    assert!(loaded.save() == bytes, "the saves differ");
}

#[test]
fn a_loaded_world_keeps_its_seed_and_names() {
    let mut world = busy_world(11, 20);
    let sprite = world.sprites().next().expect("a sprite").id();
    world.submit(Command::Rename {
        sprite,
        name: "Mira".into(),
    });
    world.step();

    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.seed(), 11);
    assert_eq!(loaded.sprite(sprite).and_then(|s| s.name()), Some("Mira"));
}

#[test]
fn a_loaded_sprite_shows_its_chemicals_at_once() {
    // The levels from before the tick aren't saved (design §2.8), but the
    // Chem tab lists every chemical straight after a load.
    let world = busy_world(5, 10);
    let loaded = World::load(&world.save()).expect("the save loads");
    let sprite = loaded.sprites().next().expect("a sprite");
    let listed = sprite.chemicals().count();
    assert_eq!(
        listed,
        world.sprite(sprite.id()).unwrap().chemicals().count()
    );
    assert!(listed > 0);
}

#[test]
fn a_loaded_world_uses_the_pack_its_save_embeds_not_the_built_in_one() {
    // A pack whose name differs, and whose berry bushes are walls of a
    // different name, so a world made with it plays differently.
    let sources: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let text = if path == "pack.ron" {
                text.replace("name: \"", "name: \"Modded ")
            } else {
                text.to_string()
            };
            (path, text)
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    let data = DataPack::from_sources(&borrowed).expect("the modded pack is valid");
    let name = data.name().to_string();
    assert!(name.starts_with("Modded "), "{name}");
    let mut original = World::new(WorldConfig::builtin(&data), data, 9);
    for _ in 0..20 {
        original.step();
    }

    let mut loaded = World::load(&original.save()).expect("the save loads");
    assert_eq!(loaded.data().name(), name);
    assert_ne!(loaded.data().name(), builtin().name());
    for _ in 0..20 {
        original.step();
        loaded.step();
    }
    assert_eq!(loaded.state_hash(), original.state_hash());
}

#[test]
fn an_unknown_gene_survives_a_save_and_load_byte_for_byte() {
    let data = builtin();
    let starter = data_starter_ron();
    let unknown = starter.replace(
        "genes: [",
        "genes: [\n        Gene(type: 900, version: 3, payload: \"c0ffee\"),",
    );
    let genome = Genome::from_ron(&unknown, &data).expect("a valid genome");
    let map = Map::from_ascii(&["....", "...."], &data).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(at(1, 1), Some(genome.clone()))],
        scripted: &[],
    };
    let world = World::from_scenario(scenario, data, 1).expect("a valid scenario");

    let loaded = World::load(&world.save()).expect("the save loads");
    let sprite = loaded.sprites().next().expect("the sprite");
    assert_eq!(
        sprite.genome().to_ron(loaded.data()),
        genome.to_ron(world.data())
    );
    assert!(sprite.genome().to_ron(loaded.data()).contains("c0ffee"));
}

/// The built-in starter genome's file.
fn data_starter_ron() -> String {
    DataPack::builtin_sources()
        .iter()
        .find(|(path, _)| *path == "genomes/starter.ron")
        .expect("the starter genome")
        .1
        .to_string()
}

#[test]
fn a_save_from_a_newer_build_is_refused_saying_so() {
    let mut bytes = busy_world(1, 5).save();
    // The header's schema version follows its name, as a one-byte number.
    let field = b"schema_version";
    let at = bytes
        .windows(field.len())
        .position(|window| window == field)
        .expect("the header names the schema")
        + field.len();
    assert_eq!(u32::from(bytes[at]), SCHEMA_VERSION);
    bytes[at] = (SCHEMA_VERSION + 1) as u8;

    match World::load(&bytes) {
        Err(err @ LoadError::Newer { schema, .. }) => {
            assert_eq!(schema, SCHEMA_VERSION + 1);
            let message = err.to_string();
            assert!(message.contains("newer"), "{message}");
        }
        Err(err) => panic!("refused for the wrong reason: {err}"),
        Ok(_) => panic!("a newer save loaded"),
    }
}

#[test]
fn what_isnt_a_save_is_refused() {
    for bytes in [&b""[..], b"hello, world", b"TSPR", b"TSPR\x00\x01\x02"] {
        assert_eq!(
            World::load(bytes).err(),
            Some(LoadError::NotASave),
            "{bytes:?}"
        );
    }
}

#[test]
fn a_damaged_save_is_refused_without_a_crash() {
    let bytes = busy_world(2, 30).save();
    // Cut short anywhere past the header, or with a byte changed.
    let step = (bytes.len() / 97).max(1);
    for len in (40..bytes.len()).step_by(step) {
        assert!(World::load(&bytes[..len]).is_err(), "cut at {len} loaded");
    }
    for index in (40..bytes.len()).step_by(step) {
        let mut damaged = bytes.clone();
        damaged[index] ^= 0xa5;
        // Some changes leave a valid world, such as a chemical's level; any
        // that doesn't must be refused rather than crash.
        let _ = World::load(&damaged);
    }
}

/// The golden save of schema 1 (design §2.8, §7.2): a small hand-made world,
/// written by `write_the_golden_save` when the schema was 1. Every later build
/// must load it.
const GOLDEN_V1: &[u8] = include_bytes!("golden/save-v1.tspr");

/// The world in the golden save: two sprites and some objects on a small
/// map, after 60 ticks, with a sprite named, the Cursor holding a ball, and
/// a rename waiting for the next tick.
fn golden_world() -> World {
    let data = builtin();
    let map = Map::from_ascii(
        &[
            "............~~",
            "............~~",
            "..............",
            "..............",
            "..............",
            "..............",
        ],
        &data,
    )
    .expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[
            (at(2, 1), "berry_bush"),
            (at(8, 4), "thornbush"),
            (at(5, 2), "ball"),
            (at(10, 5), "ball"),
        ],
        sprites: &[(at(3, 3), None), (at(9, 2), None)],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, 2026).expect("a valid scenario");
    let [first, second] = sprite_ids(&world);
    world.submit(Command::Rename {
        sprite: first,
        name: "Mira".into(),
    });
    for _ in 0..60 {
        world.step();
    }
    let ball = world.object_at(at(10, 5)).expect("the ball").id();
    world.submit(Command::PickUp { item: ball });
    world.step();
    world.submit(Command::Rename {
        sprite: second,
        name: "Oren".into(),
    });
    world
}

fn sprite_ids(world: &World) -> [EntityId; 2] {
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    ids.try_into().expect("two sprites")
}

#[test]
fn golden_the_first_save_still_loads_and_carries_on() {
    let mut world = World::load(GOLDEN_V1).expect("the golden save loads");
    assert_eq!(world.tick(), 61);
    assert_eq!(world.seed(), 2026);
    world.check_invariants().expect("invariants hold");
    let names: Vec<Option<&str>> = world.sprites().map(|s| s.name()).collect();
    assert_eq!(names.first().copied().flatten(), Some("Mira"));
    let held = world.cursor().holds().expect("the Cursor holds the ball");
    assert_eq!(held.type_name(), "ball");

    // The rename waiting in the save applies at the next tick.
    let events = world.step();
    assert!(
        events.iter().any(|e| matches!(
            &e.kind,
            EventKind::Renamed { name, .. } if name == "Oren"
        )),
        "{events:?}"
    );
    for _ in 0..50 {
        world.step();
    }
}

/// Writes the golden save. Run it by hand, once, when a new schema is
/// released: `cargo test -p terra-sim --test save -- --ignored write_the_golden_save`.
#[test]
#[ignore = "writes the golden save; run by hand when the schema changes"]
#[allow(clippy::disallowed_methods)] // the sim has no filesystem; its tests may
fn write_the_golden_save() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/save-v1.tspr");
    std::fs::write(path, golden_world().save()).expect("the golden save is written");
}
