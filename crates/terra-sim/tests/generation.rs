mod common;
use common::{builtin_with, default_world, run_for};

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
            let next = map
                .neighbour(pos, dir)
                .expect("legal steps stay on the map");
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
    run_for(&mut world, 200);
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
    let herb = r#"(id: 1, name: "herb", category: "bush", size: Small, hardness: 0.5, counters: {"entered": 9},
        stages: [(name: "a", ticks: (50, 50), next: Stage("b")),
                 (name: "b", ticks: (50, 50), next: Expire)],
        rules: [(trigger: OnStageEnter("a"), do: [AddCounter("entered", 1)]),
                (trigger: OnStageEnter("b"), do: [AddCounter("entered", 1)])])"#;
    let objects = format!("[{herb}]");
    let data = builtin_with("objects.ron", &objects);
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
                if let Some(next) = map.neighbour(pos, dir)
                    && open(next)
                    && !seen[index(next)]
                {
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

/// A world from the built-in pack with `preset`'s text as its preset.
fn world_of(preset: &str, seed: u64) -> World {
    let data = DataPack::builtin().expect("valid pack");
    let config = WorldConfig::from_ron(preset, &data).expect("valid preset");
    World::new(config, data, seed)
}

/// The share of `map`'s tiles under water, deep or shallow.
fn water_share(map: &Map) -> f64 {
    let wet = positions(map)
        .filter(|&pos| matches!(map.terrain(pos), Terrain::DeepWater | Terrain::ShallowWater))
        .count();
    wet as f64 / (usize::from(map.width()) * usize::from(map.height())) as f64
}

/// How many separate bodies of water `map` has: tiles of deep or shallow
/// water touching, diagonals too.
fn lakes(map: &Map) -> usize {
    let wet = |pos: Pos| matches!(map.terrain(pos), Terrain::DeepWater | Terrain::ShallowWater);
    let width = usize::from(map.width());
    let mut seen = vec![false; width * usize::from(map.height())];
    let index = |pos: Pos| usize::from(pos.y) * width + usize::from(pos.x);
    let mut lakes = 0;
    for start in positions(map).filter(|&pos| wet(pos)) {
        if seen[index(start)] {
            continue;
        }
        lakes += 1;
        seen[index(start)] = true;
        let mut stack = vec![start];
        while let Some(pos) = stack.pop() {
            for dir in Dir::ALL {
                if let Some(next) = map.neighbour(pos, dir)
                    && wet(next)
                    && !seen[index(next)]
                {
                    seen[index(next)] = true;
                    stack.push(next);
                }
            }
        }
    }
    lakes
}

/// How far `pos` is from the nearest drinkable tile, as the crow flies: in
/// tiles, diagonals counting one.
fn from_drinkable(map: &Map, pos: Pos, data: &DataPack) -> u16 {
    positions(map)
        .filter(|&there| data.terrain(map.terrain(there)).is_drinkable())
        .map(|there| pos.x.abs_diff(there.x).max(pos.y.abs_diff(there.y)))
        .min()
        .expect("the map has drinkable water")
}

#[test]
fn a_preset_sets_the_share_of_the_map_under_water() {
    // M2 design §19.1: the default world has 4% of its tiles under water,
    // and a preset that leaves it out the 32% of the world before it.
    for seed in [1, 2, 3] {
        let watery = world_of("(width: 256, height: 160)", seed);
        let dry = world_of("(width: 256, height: 160, water: 4)", seed);
        let (watery, dry) = (water_share(watery.map()), water_share(dry.map()));
        assert!((0.30..=0.33).contains(&watery), "seed {seed}: {watery}");
        assert!((0.035..=0.042).contains(&dry), "seed {seed}: {dry}");
    }
}

#[test]
fn bigger_lakes_hold_the_same_water_in_fewer_lakes() {
    // M2 design §19.1: lake size 3 makes the map's hills and hollows three
    // times as wide, so the same 4% of water lies in a few big lakes:
    // under half as many on each map, and a third as many over four.
    let (mut all_small, mut all_big) = (0, 0);
    for seed in [1, 2, 3, 4] {
        let ponds = world_of("(width: 256, height: 160, water: 4)", seed);
        let lakes_3 = world_of("(width: 256, height: 160, water: 4, lake_size: 3)", seed);
        let (small, big) = (lakes(ponds.map()), lakes(lakes_3.map()));
        assert!(
            big * 2 < small,
            "seed {seed}: {big} big lakes, {small} ponds"
        );
        let share = water_share(lakes_3.map());
        assert!((0.035..=0.042).contains(&share), "seed {seed}: {share}");
        (all_small, all_big) = (all_small + small, all_big + big);
    }
    assert!(
        all_big * 3 <= all_small,
        "{all_big} big lakes, {all_small} ponds"
    );
}

#[test]
fn the_first_sprites_start_near_drinkable_water_when_the_preset_says() {
    // M2 design §19.1: the default world's first sprites start within 10
    // tiles of drinkable water; a preset that leaves it out puts them
    // anywhere, and in a dry world some start far from it.
    let data = DataPack::builtin().expect("valid pack");
    let mut anywhere_far = 0;
    for seed in [1, 2, 3, 4] {
        let near = World::new(WorldConfig::builtin(&data), data.clone(), seed);
        assert_eq!(near.sprites().count(), 30, "seed {seed}");
        for sprite in near.sprites() {
            let far = from_drinkable(near.map(), sprite.pos(), &data);
            assert!(
                far <= 10,
                "seed {seed}: a first sprite {far} tiles from water"
            );
        }
        let preset = "(width: 256, height: 160, sprites: 30, water: 4, lake_size: 3)";
        let anywhere = world_of(preset, seed);
        anywhere_far += anywhere
            .sprites()
            .filter(|sprite| from_drinkable(anywhere.map(), sprite.pos(), &data) > 10)
            .count();
    }
    assert!(
        anywhere_far > 30,
        "{anywhere_far} of 120 started far from water"
    );
}

#[test]
fn once_the_tiles_near_water_are_taken_the_other_first_sprites_start_anywhere() {
    // M2 design §19.1: a small, dry world has fewer free tiles within 1 of
    // drinkable water than first sprites. Each of those tiles gets one,
    // and the rest start farther out, so the world starts with all 40.
    let data = DataPack::builtin().expect("valid pack");
    let preset = "(width: 32, height: 32, sprites: 40, water: 1, start_near_water: 1)";
    for seed in [1, 2, 3, 4] {
        let world = world_of(preset, seed);
        let map = world.map();
        let near: Vec<Pos> = positions(map)
            .filter(|&pos| {
                from_drinkable(map, pos, &data) <= 1
                    && data.terrain(map.terrain(pos)).step_cost().is_some()
                    && world.object_at(pos).is_none()
            })
            .collect();
        assert!(
            (1..40).contains(&near.len()),
            "seed {seed}: {} free tiles near water",
            near.len()
        );
        assert_eq!(world.sprites().count(), 40, "seed {seed}");
        for pos in near {
            assert!(
                world.sprite_at(pos).is_some(),
                "seed {seed}: nobody started on {pos:?}, near water"
            );
        }
    }
}

#[test]
fn the_unattended_run_s_newborns_start_anywhere_near_water_or_not() {
    // M2 design §19.1, §19.6: the default preset starts its first sprites
    // within 10 tiles of drinkable water, but the unattended run spreads
    // its newborns over the whole map. Over the same seeds, the same world
    // settings with no `start_near_water` start more than 30 far from it
    // (`the_first_sprites_start_near_drinkable_water_when_the_preset_says`).
    let data = DataPack::builtin().expect("valid pack");
    let mut far = 0;
    for seed in [1, 2, 3, 4] {
        let world = World::unattended(WorldConfig::builtin(&data), data.clone(), seed);
        far += world
            .sprites()
            .filter(|sprite| from_drinkable(world.map(), sprite.pos(), &data) > 10)
            .count();
    }
    assert!(far > 30, "{far} of 120 newborns started far from water");
}
