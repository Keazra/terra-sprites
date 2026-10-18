//! Saves (design §2.8): a loaded world carries on exactly as the one saved
//! would have, with the data pack its save embeds; a save from a newer
//! build, or a damaged one, is refused rather than crashing.

mod common;
use common::{WorldBuilder, at, builtin, builtin_changing};

use terra_sim::{
    Command, DataPack, EntityId, EventKind, Genome, LoadError, SCHEMA_VERSION, World, WorldConfig,
};

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
fn a_save_keeps_where_the_screen_was_looking() {
    let world = busy_world(4, 10);
    let plain = World::load(&world.save()).expect("the save loads");
    assert_eq!(plain.view(), None, "a save made without a view has none");

    let loaded = World::load(&world.save_with_view(at(30, 12))).expect("the save loads");
    assert_eq!(loaded.view(), Some(at(30, 12)));
    assert_eq!(
        loaded.state_hash(),
        world.state_hash(),
        "the view isn't world state"
    );
    // Saved again as it stands, it keeps the view it was loaded with.
    let again = World::load(&loaded.save()).expect("the save loads");
    assert_eq!(again.view(), Some(at(30, 12)));
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
    // A pack whose name differs from the built-in one's, so the loaded
    // world's pack shows where it came from.
    let data = builtin_changing("pack.ron", &[("name: \"", "name: \"Modded ")]);
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
    let world = WorldBuilder::new(&["....", "...."])
        .data(data)
        .sprites_with(vec![(at(1, 1), Some(genome.clone()))])
        .build();

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
    // Past the header, the checksum catches any byte changed.
    for index in (100..bytes.len()).step_by(step) {
        let mut damaged = bytes.clone();
        damaged[index] ^= 0xa5;
        assert!(
            matches!(World::load(&damaged), Err(LoadError::Damaged(_))),
            "a change at {index} loaded"
        );
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
    let mut world = WorldBuilder::new(&[
        "............~~",
        "............~~",
        "..............",
        "..............",
        "..............",
        "..............",
    ])
    .objects(&[
        (at(2, 1), "berry_bush"),
        (at(8, 4), "thornbush"),
        (at(5, 2), "ball"),
        (at(10, 5), "ball"),
    ])
    .starters(&[at(3, 3), at(9, 2)])
    .seed(2026)
    .build();
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

// Unlike the golden genome files, a golden save isn't checked byte for byte
// on saving it again: a later build that adds a field (with a default, as
// design §2.8 allows) writes it, and the header names that build. What must
// hold is that it loads and plays on.
#[test]
fn golden_the_first_save_still_loads_and_carries_on() {
    let mut world = World::load(GOLDEN_V1).expect("the golden save loads");
    assert_eq!(world.tick(), 61);
    assert_eq!(world.seed(), 2026);
    assert_eq!(world.view(), None, "schema 1 saved no view at first");
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

/// A save of schema 1 from before eggs were items (#196), written by the
/// build that merged PR #199: on an 8 by 8 field, two females laid an egg
/// each on tick 0, as sprites that hatch on tick 750. The sprite 9, at
/// (1, 1) with a berry on every tile around her, laid hers, 11, on the
/// berry at (0, 0); the sprite 10's, 12, lies at (5, 6), and the player
/// named it "Pebble". Saved on tick 2.
const GOLDEN_V1_EGGS: &[u8] = include_bytes!("golden/save-v1-eggs.tspr");

#[test]
fn golden_a_save_with_eggs_from_before_they_were_items_loads_them_as_items() {
    // #196: each egg becomes an egg item with its ID, on its tile, or if an
    // item lies there, the nearest tile with no item and no sprite, and
    // hatches when it would have, into the sprite it was.
    let mut world = World::load(GOLDEN_V1_EGGS).expect("the save loads");
    assert_eq!(world.tick(), 2);
    let sprites: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    assert_eq!(sprites, [EntityId(9), EntityId(10)], "an egg is no sprite");
    let eggs: Vec<_> = world
        .eggs()
        .map(|egg| {
            let name = egg.name().map(String::from);
            (
                egg.id(),
                egg.pos(),
                egg.parents().to_vec(),
                egg.laid(),
                egg.hatches(),
                name,
            )
        })
        .collect();
    let (pebble, other) = (EntityId(12), EntityId(11));
    assert_eq!(
        eggs,
        [
            // The tiles around (0, 0) out to two away hold berries or a
            // sprite, so it moves three.
            (other, Some(at(3, 0)), vec![EntityId(9)], 0, 750, None),
            (
                pebble,
                Some(at(5, 6)),
                vec![EntityId(10)],
                0,
                750,
                Some("Pebble".into())
            ),
        ]
    );
    assert_eq!(world.object_at(at(0, 0)).unwrap().type_name(), "berry");
    assert_eq!(world.object_at(at(3, 0)).unwrap().type_name(), "egg");
    let mut hatched = Vec::new();
    while world.tick() < 800 {
        for event in world.step() {
            if let EventKind::Hatched { id, parents } = event.kind {
                hatched.push((id, event.tick, parents));
            }
        }
    }
    assert_eq!(
        hatched,
        [
            (other, 750, vec![EntityId(9)]),
            (pebble, 750, vec![EntityId(10)])
        ]
    );
    assert_eq!(world.sprite(pebble).unwrap().name(), Some("Pebble"));
    assert_eq!(world.sprite(other).unwrap().parents(), [EntityId(9)]);
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
