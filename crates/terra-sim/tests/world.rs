use terra_sim::{DataPack, Map, MapError, Pos, Terrain, World, WorldConfig};

fn new_world(seed: u64) -> World {
    World::new(
        WorldConfig::builtin(),
        DataPack::builtin().expect("built-in data pack is valid"),
        seed,
    )
}

#[test]
fn a_new_world_starts_at_tick_zero_and_each_step_advances_one_tick() {
    let mut world = new_world(1);
    assert_eq!(world.tick(), 0);

    world.step();
    assert_eq!(world.tick(), 1);

    for _ in 0..10 {
        world.step();
    }
    assert_eq!(world.tick(), 11);
}

#[test]
fn worlds_with_the_same_seed_have_identical_state_hashes_on_every_tick() {
    let mut a = new_world(42);
    let mut b = new_world(42);

    for _ in 0..1_000 {
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "diverged at tick {}",
            a.tick()
        );
        a.check_invariants().expect("invariants hold");
        a.step();
        b.step();
    }
}

#[test]
fn worlds_with_different_seeds_have_different_state_hashes() {
    let mut a = new_world(1);
    let mut b = new_world(2);

    for _ in 0..10 {
        assert_ne!(
            a.state_hash(),
            b.state_hash(),
            "seeds collided at tick {}",
            a.tick()
        );
        a.step();
        b.step();
    }
}

fn drawn(rows: &[&str]) -> Result<World, MapError> {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    World::from_map(map, data, 1)
}

#[test]
fn a_world_can_be_made_from_a_drawn_map_with_one_region() {
    let world = drawn(&[
        "..~", //
        "#.:", //
    ])
    .expect("one region");
    assert_eq!((world.map().width(), world.map().height()), (3, 2));
    assert_eq!(
        world.map().terrain(Pos { x: 2, y: 0 }),
        Terrain::ShallowWater
    );
}

#[test]
fn a_drawn_map_must_have_exactly_one_region() {
    let cases: [(&[&str], usize); 3] = [
        (&["#="], 0),
        (&[".#."], 2),
        // Touching only at a corner, past rock on both sides.
        (&[".#", "#."], 2),
    ];
    for (rows, regions) in cases {
        assert_eq!(
            drawn(rows).err(),
            Some(MapError::NotOneRegion { regions }),
            "{rows:?}"
        );
    }
}

fn generated(config: &str, seed: u64) -> World {
    World::new(
        WorldConfig::from_ron(config).expect("valid preset"),
        DataPack::builtin().expect("built-in data pack is valid"),
        seed,
    )
}

/// Every tile's terrain, row by row.
fn terrain_of(world: &World) -> Vec<Terrain> {
    let map = world.map();
    (0..map.height())
        .flat_map(|y| (0..map.width()).map(move |x| map.terrain(Pos { x, y })))
        .collect()
}

#[test]
fn a_new_world_has_a_map_of_the_configured_size() {
    let world = generated("(width: 48, height: 32)", 1);
    assert_eq!((world.map().width(), world.map().height()), (48, 32));
}

#[test]
fn the_seed_decides_the_map() {
    let config = "(width: 64, height: 40)";
    assert_eq!(
        terrain_of(&generated(config, 5)),
        terrain_of(&generated(config, 5)),
        "the same seed gives the same map"
    );
    for (a, b) in [(1, 2), (2, 3), (5, 500)] {
        assert_ne!(
            terrain_of(&generated(config, a)),
            terrain_of(&generated(config, b)),
            "seeds {a} and {b} give the same map"
        );
    }
}
