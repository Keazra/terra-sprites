//! The default world's ecology over thousands of ticks (design §3.5.3).

use std::collections::BTreeMap;

use terra_sim::{DataPack, EventKind, Removal, World, WorldConfig};

fn default_world(seed: u64) -> World {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    World::new(WorldConfig::builtin(&data), data, seed)
}

/// What the ecology did: counts of `(object type, what happened)`.
type Tally = BTreeMap<(String, &'static str), usize>;

#[test]
fn at_speed_bushes_fruit_berries_drop_then_expire_or_sprout_and_bushes_spread() {
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let mut world = default_world(21);
    let mut tally = Tally::new();
    // Berries that sprouted, each paired with its bush.
    let mut sprouts = 0;
    // Fruit ripens within about 1,200 ticks and berries expire 1,500–2,500 ticks
    // later; grown berry bushes and thornbushes try to spread every 2,000.
    // 5,000 ticks shows them all.
    for tick in 0..5_000 {
        let events = world.step();
        // A sprouting berry is reported gone and its bush spawned, one
        // after the other (design §2.5).
        for pair in events.windows(2) {
            if let EventKind::ObjectRemoved {
                object_type,
                reason: Removal::Replaced,
                ..
            } = &pair[0].kind
                && object_type == "berry"
            {
                assert!(
                    matches!(&pair[1].kind, EventKind::ObjectSpawned { object_type, .. }
                        if object_type == "berry_bush"),
                    "a sprouting berry became {:?}",
                    pair[1].kind
                );
                sprouts += 1;
            }
        }
        for event in events {
            let entry = match event.kind {
                EventKind::ObjectSpawned {
                    object_type, pos, ..
                } => {
                    if object_type == "berry_bush" {
                        let fertility = pack.terrain(world.map().terrain(pos)).fertility();
                        assert!(fertility >= 0.5, "a bush sprouted on barren {pos:?}");
                    }
                    (object_type, "spawned")
                }
                EventKind::ObjectRemoved {
                    object_type,
                    reason,
                    ..
                } => (
                    object_type,
                    match reason {
                        Removal::Expired => "expired",
                        Removal::Destroyed => "destroyed",
                        Removal::Replaced => "replaced",
                    },
                ),
                // Only the objects are tallied here.
                _ => continue,
            };
            *tally.entry(entry).or_insert(0) += 1;
        }
        if tick % 1_000 == 0 {
            assert_eq!(world.check_invariants(), Ok(()), "tick {tick}");
        }
    }
    let count = |name: &str, what: &'static str| tally.get(&(name.into(), what)).copied();
    assert!(
        count("berry", "spawned") > Some(0),
        "fruit drops as berries: {tally:?}"
    );
    assert!(
        count("berry", "expired") > Some(0),
        "berries expire: {tally:?}"
    );
    assert!(
        count("berry", "replaced") > Some(0),
        "berries sprout: {tally:?}"
    );
    assert_eq!(
        Some(sprouts),
        count("berry", "replaced"),
        "every sprouting berry becomes a bush: {tally:?}"
    );
    // Grown bushes put up seedlings too (M2 design §19.6): the bushes
    // spawned that no berry became.
    assert!(
        count("berry_bush", "spawned") > Some(sprouts),
        "bushes put up seedlings: {tally:?}"
    );
    assert!(
        count("thornbush", "spawned") > Some(0),
        "thornbushes spread: {tally:?}"
    );
    assert!(
        count("berry_bush", "expired") > Some(0),
        "old bushes expire: {tally:?}"
    );
}

#[test]
fn the_same_seed_gives_identical_ecology_hashes_over_10000_ticks() {
    let mut a = default_world(99);
    let mut b = default_world(99);
    for tick in 0..10_000 {
        let (events_a, events_b) = (a.step(), b.step());
        assert_eq!(events_a, events_b, "events diverged at tick {tick}");
        if tick % 250 == 0 {
            assert_eq!(a.state_hash(), b.state_hash(), "diverged at tick {tick}");
        }
    }
    assert_eq!(a.state_hash(), b.state_hash());
}
