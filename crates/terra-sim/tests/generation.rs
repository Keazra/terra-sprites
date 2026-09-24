use proptest::prelude::*;
use terra_sim::{DataPack, Dir, Map, Pos, Terrain, World, WorldConfig};

fn generate(width: u16, height: u16, seed: u64) -> World {
    let config = WorldConfig::from_ron(&format!("(width: {width}, height: {height})"))
        .expect("valid preset");
    World::new(config, DataPack::builtin().expect("valid pack"), seed)
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
    let default = WorldConfig::builtin();
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
