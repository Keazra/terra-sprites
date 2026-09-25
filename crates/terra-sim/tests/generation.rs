use proptest::prelude::*;
use terra_sim::{DataPack, Dir, EventKind, Map, Pos, Removal, Terrain, World, WorldConfig};

fn generate(width: u16, height: u16, seed: u64) -> World {
    let data = DataPack::builtin().expect("valid pack");
    let config = WorldConfig::from_ron(&format!("(width: {width}, height: {height})"), &data)
        .expect("valid preset");
    World::new(config, data, seed)
}

fn positions(map: &Map) -> impl Iterator<Item = Pos> + '_ {
    (0..map.height()).flat_map(move |y| (0..map.width()).map(move |x| Pos { x, y }))
}

/// Counts the walkable tiles, and those reachable by legal steps from the first one.
fn walkable_and_reachable(map: &Map) -> (usize, usize) {
    let data = DataPack::builtin().expect("valid pack");
    let walkable = |pos: Pos| data.terrain(map.terrain(pos)).step_cost().is_some();
    let all: Vec<Pos> = positions(map).filter(|&pos| walkable(pos)).collect();
    let Some(&start) = all.first() else {
        return (0, 0);
    };
    let mut seen = vec![false; usize::from(map.width()) * usize::from(map.height())];
    let index = |pos: Pos| usize::from(pos.y) * usize::from(map.width()) + usize::from(pos.x);
    seen[index(start)] = true;
    let mut stack = vec![start];
    let mut reached = 0;
    while let Some(pos) = stack.pop() {
        reached += 1;
        for dir in Dir::ALL {
            if map.step_cost(pos, dir).is_none() {
                continue;
            }
            let (dx, dy) = match dir {
                Dir::N => (0, -1),
                Dir::NE => (1, -1),
                Dir::E => (1, 0),
                Dir::SE => (1, 1),
                Dir::S => (0, 1),
                Dir::SW => (-1, 1),
                Dir::W => (-1, 0),
                Dir::NW => (-1, -1),
            };
            let next = Pos {
                x: pos
                    .x
                    .checked_add_signed(dx)
                    .expect("legal steps stay on the map"),
                y: pos
                    .y
                    .checked_add_signed(dy)
                    .expect("legal steps stay on the map"),
            };
            if !seen[index(next)] {
                seen[index(next)] = true;
                stack.push(next);
            }
        }
    }
    (all.len(), reached)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn every_generated_map_is_one_region(
        seed: u64,
        width in 32u16..=160,
        height in 32u16..=120,
    ) {
        let world = generate(width, height, seed);
        let (walkable, reachable) = walkable_and_reachable(world.map());
        prop_assert!(walkable > 0, "no walkable tiles");
        prop_assert_eq!(reachable, walkable, "some walkable tiles are cut off");
    }
}

#[test]
fn every_default_size_map_is_one_region_with_all_six_terrains() {
    let default = WorldConfig::builtin(&DataPack::builtin().expect("valid pack"));
    for seed in 0..16 {
        let world = generate(default.width(), default.height(), seed);
        let map = world.map();
        let (walkable, reachable) = walkable_and_reachable(map);
        assert_eq!(
            reachable, walkable,
            "seed {seed}: some walkable tiles are cut off"
        );
        for terrain in Terrain::ALL {
            assert!(
                positions(map).any(|pos| map.terrain(pos) == terrain),
                "seed {seed} has no {terrain:?}"
            );
        }
    }
}

#[test]
fn maps_at_the_size_limits_are_one_region() {
    for (width, height, seed) in [(1024, 32, 1), (32, 1024, 2), (1024, 1024, 3)] {
        let world = generate(width, height, seed);
        let (walkable, reachable) = walkable_and_reachable(world.map());
        assert!(walkable > 0, "{width}x{height}: no walkable tiles");
        assert_eq!(
            reachable, walkable,
            "{width}x{height}: some walkable tiles are cut off"
        );
    }
}

/// A new world from the built-in preset and pack.
fn default_world(seed: u64) -> World {
    let data = DataPack::builtin().expect("valid pack");
    World::new(WorldConfig::builtin(&data), data, seed)
}

/// How many objects of each type the world has, by type name.
fn census(world: &World) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for object in world.objects() {
        *counts.entry(object.type_name().to_string()).or_insert(0) += 1;
    }
    counts
}

#[test]
fn a_new_world_gets_the_presets_objects_placed_by_the_rules() {
    for seed in 0..4 {
        let world = default_world(seed);
        let expected = [("ball", 16), ("berry_bush", 400), ("thornbush", 107)]
            .map(|(name, n)| (name.to_string(), n));
        assert_eq!(census(&world), expected.into(), "seed {seed}");
        assert_eq!(world.check_invariants(), Ok(()), "seed {seed}");
    }
}

#[test]
fn a_crowded_preset_places_what_fits_and_carries_on() {
    let data = DataPack::builtin().expect("valid pack");
    let preset = r#"(width: 32, height: 32, objects: {"berry_bush": 1, "ball": 1}, per_tiles: 1)"#;
    let config = WorldConfig::from_ron(preset, &data).expect("valid preset");
    let world = World::new(config, data, 5);
    let counts = census(&world);
    assert!(
        counts["berry_bush"] > 0 && counts["berry_bush"] < 1024,
        "{counts:?}"
    );
    assert!(counts["ball"] > 0, "items fill the gaps: {counts:?}");
    assert_eq!(world.check_invariants(), Ok(()));
}

#[test]
fn generated_objects_start_partway_through_their_lives() {
    let mut world = default_world(11);
    let bushes = || world.objects().filter(|o| o.type_name() == "berry_bush");
    let seedlings = bushes().filter(|o| o.stage() == Some("seedling")).count();
    // Seedlings last about 2,000 of a bush's 27,000 ticks: about 7% of them.
    assert!(
        (10..=60).contains(&seedlings),
        "{seedlings} of 400 are seedlings"
    );
    assert!(
        bushes().all(|o| o.counter("fruit") == Some(0)),
        "counters start at 0"
    );

    // Mature bushes gain fruit every 200 ticks, so the world has food at once.
    for _ in 0..200 {
        world.step();
    }
    let fruiting = world
        .objects()
        .filter(|o| o.counter("fruit").is_some_and(|fruit| fruit > 0))
        .count();
    assert!(
        fruiting > 300,
        "{fruiting} bushes carry fruit after 200 ticks"
    );
}

#[test]
fn generated_objects_expire_spread_out_not_all_at_once() {
    // A berry bush lives about 27,000 ticks. Started at random ages, about 15
    // of the 400 expire in any 1,000 ticks; started fresh, none would.
    let mut world = default_world(3);
    let mut expired = 0;
    for _ in 0..1_000 {
        expired += world
            .step()
            .iter()
            .filter(|e| {
                matches!(
                    &e.kind,
                    EventKind::ObjectRemoved { object_type, reason: Removal::Expired, .. }
                        if object_type == "berry_bush"
                )
            })
            .count();
    }
    assert!(
        (4..=40).contains(&expired),
        "{expired} berry bushes expired"
    );
}

#[test]
fn a_generated_object_gets_no_on_stage_enter_for_the_stage_it_starts_in() {
    let herb = r#"(id: 1, name: "herb", category: BerryBush, counters: {"entered": 9},
        stages: [(name: "a", ticks: (50, 50), next: Stage("b")),
                 (name: "b", ticks: (50, 50), next: Expire)],
        rules: [(trigger: OnStageEnter("a"), do: [AddCounter("entered", 1)]),
                (trigger: OnStageEnter("b"), do: [AddCounter("entered", 1)])])"#;
    let objects = format!("[{herb}]");
    let data = DataPack::from_sources(&[
        ("pack.ron", include_str!("../../../data/pack.ron")),
        ("terrain.ron", include_str!("../../../data/terrain.ron")),
        ("chemicals.ron", include_str!("../../../data/chemicals.ron")),
        ("loci.ron", include_str!("../../../data/loci.ron")),
        ("objects.ron", &objects),
    ])
    .expect("valid pack");
    let preset = r#"(width: 64, height: 64, objects: {"herb": 20}, per_tiles: 4096)"#;
    let config = WorldConfig::from_ron(preset, &data).expect("valid preset");
    let mut world = World::new(config, data, 9);
    world.step();
    assert!(
        world.objects().all(|o| o.counter("entered") == Some(0)),
        "no herb entered a stage on tick 0"
    );
}

/// How many regions the open ground is in: walkable tiles holding no
/// solid object. A legal diagonal step needs both tiles beside it open, so
/// joining by orthogonal steps gives the same regions.
fn open_regions(world: &World) -> usize {
    let map = world.map();
    let data = world.data();
    let open = |pos: Pos| {
        data.terrain(map.terrain(pos)).step_cost().is_some()
            && !world.object_at(pos).is_some_and(|o| o.is_solid())
    };
    let index = |pos: Pos| usize::from(pos.y) * usize::from(map.width()) + usize::from(pos.x);
    let mut seen = vec![false; usize::from(map.width()) * usize::from(map.height())];
    let mut regions = 0;
    for start in positions(map).filter(|&pos| open(pos)) {
        if seen[index(start)] {
            continue;
        }
        regions += 1;
        seen[index(start)] = true;
        let mut stack = vec![start];
        while let Some(pos) = stack.pop() {
            for dir in [Dir::N, Dir::E, Dir::S, Dir::W] {
                let (dx, dy) = match dir {
                    Dir::N => (0, -1),
                    Dir::E => (1, 0),
                    Dir::S => (0, 1),
                    _ => (-1, 0),
                };
                let (Some(x), Some(y)) =
                    (pos.x.checked_add_signed(dx), pos.y.checked_add_signed(dy))
                else {
                    continue;
                };
                let next = Pos { x, y };
                if x < map.width() && y < map.height() && open(next) && !seen[index(next)] {
                    seen[index(next)] = true;
                    stack.push(next);
                }
            }
        }
    }
    regions
}

#[test]
fn generation_never_splits_the_open_ground() {
    for seed in 0..4 {
        assert_eq!(open_regions(&default_world(seed)), 1, "seed {seed}");
    }
    // Crowded: a bush wanted on every tile.
    let data = DataPack::builtin().expect("valid pack");
    let preset = r#"(width: 48, height: 48, objects: {"berry_bush": 1}, per_tiles: 1)"#;
    for seed in 0..4 {
        let config = WorldConfig::from_ron(preset, &data).expect("valid preset");
        let world = World::new(config, data.clone(), seed);
        assert_eq!(open_regions(&world), 1, "crowded, seed {seed}");
    }
}
