//! World generation (design §3.2): seeded value noise, terrain bands by
//! percentile, then one region.

use std::collections::VecDeque;

use rand_chacha::ChaCha8Rng;

use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::ecology::aged_object;
use crate::map::{Dir, Map, Pos};
use crate::mutation;
use crate::random::{uniform, unit};
use crate::regions;
use crate::sprites::Sprite;
use crate::terrain::Terrain;
use crate::world::WorldState;

/// Regions smaller than this become rock; larger ones are joined to the mainland.
const MIN_REGION: usize = 64;

/// Noise octaves as (lattice spacing in tiles, weight): broad shapes plus detail.
const OCTAVES: [(u16, f32); 3] = [(24, 1.0), (12, 0.5), (6, 0.25)];

/// Height bands, in hundredths of a percent of all tiles: the water's share
/// comes from the preset (M2 design §19.1), deep and shallow in the
/// proportion 20 to 12 of the watery world before it; then a band of sand
/// above the water, the land, and rock above it.
const DEEP_SHARE_OF_WATER: (usize, usize) = (20, 32);
const SAND_BAND: usize = 800;
const LAND_TOP: usize = 9_000; // above this is rock
/// The driest share of the land, as a percentage, is dirt; the rest is grass.
const DIRT_SHARE: usize = 40;

/// Generates a connected map for a new world, drawing randomness from the world's RNG.
pub(crate) fn generate(config: &WorldConfig, data: &DataPack, rng: &mut ChaCha8Rng) -> Map {
    let mut map = banded(config, data, rng);
    regions::connect(&mut map, MIN_REGION);
    map
}

/// The map's terrain by height and moisture, before its regions are joined.
fn banded(config: &WorldConfig, data: &DataPack, rng: &mut ChaCha8Rng) -> Map {
    let (width, height) = (config.width(), config.height());
    // Bigger lakes come from broader hills and hollows (M2 design §19.1).
    let heights = noise(width, height, u16::from(config.lake_size()), rng);
    let moisture = noise(width, height, 1, rng);

    let shallow_top = usize::from(config.water()) * 100;
    let (deep, water) = DEEP_SHARE_OF_WATER;
    let deep_top = (shallow_top * deep + water / 2) / water;
    let sand_top = shallow_top + SAND_BAND;

    let mut map = Map::filled(width, height, Terrain::Grass, data);
    let count = map.tile_count();
    let mut land = Vec::new();
    for (rank, index) in ranked(&heights).into_iter().enumerate() {
        let share_below = rank * 10_000;
        let terrain = if share_below < count * deep_top {
            Terrain::DeepWater
        } else if share_below < count * shallow_top {
            Terrain::ShallowWater
        } else if share_below < count * sand_top {
            Terrain::Sand
        } else if share_below < count * LAND_TOP {
            land.push(index);
            continue;
        } else {
            Terrain::Rock
        };
        map.set(map.pos(index), terrain);
    }

    let land_moisture: Vec<f32> = land.iter().map(|&index| moisture[index]).collect();
    for (rank, position) in ranked(&land_moisture).into_iter().enumerate() {
        if rank * 100 < land.len() * DIRT_SHARE {
            map.set(map.pos(land[position]), Terrain::Dirt);
        }
    }
    map
}

/// Places the preset's objects on the mainland (design §3.2): solid objects
/// first, then items, each type in ID order. Each goes on a tile drawn
/// uniformly from the type's remaining candidates, where it may go and, if it's
/// solid, where it keeps paths open; if none is left, the rest of that type is
/// skipped. Every object starts at a random point in its life.
pub(crate) fn place_objects(config: &WorldConfig, data: &DataPack, state: &mut WorldState) {
    let types = data.object_types();
    let solid_first = (0..types.len())
        .filter(|&kind| types[kind].solid)
        .chain((0..types.len()).filter(|&kind| !types[kind].solid));
    for kind in solid_first {
        let count = config.object_count(&types[kind].name);
        if count == 0 {
            continue;
        }
        // After joining, every walkable tile is on the mainland.
        let mut candidates: Vec<Pos> = state
            .map
            .positions()
            .filter(|&pos| state.map.is_walkable(pos))
            .collect();
        let solid = types[kind].solid;
        for _ in 0..count {
            // A tile found unusable is set aside for this type. (One that would
            // cut a path might become usable once a neighbour fills in; setting
            // it aside anyway keeps generation quick.)
            let pos = loop {
                if candidates.is_empty() {
                    break None;
                }
                let choice = uniform(&mut state.rng, candidates.len() as u64) as usize;
                let pos = candidates.swap_remove(choice);
                if state.can_place(data, kind, pos)
                    && (!solid || state.objects.keeps_paths_open(&state.map, data, pos))
                {
                    break Some(pos);
                }
            };
            let Some(pos) = pos else {
                break;
            };
            let object = aged_object(data, &mut state.rng, kind, pos);
            state.add_object(object, data);
        }
    }
}

/// Places the preset's first population (design §3.2), after the objects:
/// each sprite on a tile drawn uniformly from the walkable tiles holding no
/// object and no sprite, within the preset's `start_near_water` of
/// drinkable water if it gives one and any such tile is free (M2 design
/// §19.1), made from the starter genome with spawn variation,
/// at an age drawn uniformly through adulthood (M2 design §9.1), and with
/// its energy and hydration drawn between physiology's
/// `first_population` fractions of the newborn level, so the sprites don't
/// all run dry together. If the map runs out of room, the rest are skipped.
pub(crate) fn place_sprites(config: &WorldConfig, data: &DataPack, state: &mut WorldState) {
    let mut candidates = standing_room(data, state);
    if let Some(near) = config.start_near_water() {
        let far = from_drinkable(&state.map, data);
        let near: Vec<Pos> = candidates
            .iter()
            .copied()
            .filter(|&pos| far[state.map.index(pos)] <= u32::from(near))
            .collect();
        if !near.is_empty() {
            candidates = near;
        }
    }
    let physiology = data.physiology();
    let (low, high) = physiology.first_population;
    for _ in 0..config.sprites() {
        if candidates.is_empty() {
            break;
        }
        let choice = uniform(&mut state.rng, candidates.len() as u64) as usize;
        let pos = candidates.swap_remove(choice);
        // Wild sprites carry a few hidden flaws (M2 design §12).
        let (genome, flaws) = mutation::wild(data, &mut state.rng);
        // The first sprites are adults, of all ages (M2 design §9.1).
        let (grows_up, grows_old) = Sprite::adulthood(&genome, data);
        let age = grows_up + uniform(&mut state.rng, grows_old.saturating_sub(grows_up).max(1));
        let mut sprite = Sprite::aged(genome, pos, state.tick, age, data);
        sprite.flaws = flaws;
        for index in [physiology.indices.energy, physiology.indices.hydration] {
            let level = sprite.body.chems[index] * (low + (high - low) * unit(&mut state.rng));
            sprite.body.start_at(index, level);
        }
        state.add_sprite(sprite);
    }
}

/// The unattended run's first sprites (M2 design §19.6): as many as
/// `config` starts with, each a newborn with a wild sprite's flaws, placed
/// as the Cursor places one, on a free tile drawn from the whole map, near
/// water or not.
pub(crate) fn place_newborns(config: &WorldConfig, data: &DataPack, state: &mut WorldState) {
    let mut candidates = standing_room(data, state);
    for _ in 0..config.sprites() {
        if candidates.is_empty() {
            break;
        }
        let choice = uniform(&mut state.rng, candidates.len() as u64) as usize;
        let pos = candidates.swap_remove(choice);
        let (genome, flaws) = mutation::wild(data, &mut state.rng);
        let mut sprite = Sprite::newborn(genome, pos, state.tick, data);
        sprite.flaws = flaws;
        sprite.placed = true;
        state.add_sprite(sprite);
    }
}

/// Every tile a first sprite may start on: one it may stand on, with no
/// object, in tile order.
fn standing_room(data: &DataPack, state: &WorldState) -> Vec<Pos> {
    state
        .map
        .positions()
        .filter(|&pos| state.can_stand(data, pos) && state.objects.at(pos).is_none())
        .collect()
}

/// How far each tile is from the nearest drinkable water, in tiles as the
/// crow flies (any step, diagonals too, over anything), in tile-index
/// order; `u32::MAX` on a map with none.
fn from_drinkable(map: &Map, data: &DataPack) -> Vec<u32> {
    let mut far = vec![u32::MAX; map.tile_count()];
    let mut queue = VecDeque::new();
    for pos in map.positions() {
        if data.terrain(map.terrain(pos)).is_drinkable() {
            far[map.index(pos)] = 0;
            queue.push_back(pos);
        }
    }
    while let Some(pos) = queue.pop_front() {
        let next = far[map.index(pos)] + 1;
        for step in Dir::ALL
            .into_iter()
            .filter_map(|dir| map.neighbour(pos, dir))
        {
            if far[map.index(step)] == u32::MAX {
                far[map.index(step)] = next;
                queue.push_back(step);
            }
        }
    }
    far
}

/// Indices of `values` from lowest to highest value; ties go to the lower index.
fn ranked(values: &[f32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]).then(a.cmp(&b)));
    order
}

/// A field of smooth value noise, one value per tile in tile-index order,
/// its shapes `scale` times as broad as the octaves give.
///
/// Each octave draws random values on a coarse lattice and blends between them
/// with smoothstep. Only + − × ÷ are used, so results are identical everywhere.
fn noise(width: u16, height: u16, scale: u16, rng: &mut ChaCha8Rng) -> Vec<f32> {
    let mut field = vec![0.0; usize::from(width) * usize::from(height)];
    for (spacing, weight) in OCTAVES {
        let spacing = spacing * scale;
        let lattice_width = usize::from(width / spacing) + 2;
        let lattice_height = usize::from(height / spacing) + 2;
        let lattice: Vec<f32> = (0..lattice_width * lattice_height)
            .map(|_| unit(rng))
            .collect();
        let at = |x: usize, y: usize| lattice[y * lattice_width + x];
        for y in 0..height {
            let (cell_y, ty) = cell(y, spacing);
            for x in 0..width {
                let (cell_x, tx) = cell(x, spacing);
                let top = lerp(at(cell_x, cell_y), at(cell_x + 1, cell_y), tx);
                let bottom = lerp(at(cell_x, cell_y + 1), at(cell_x + 1, cell_y + 1), tx);
                let index = usize::from(y) * usize::from(width) + usize::from(x);
                field[index] += weight * lerp(top, bottom, ty);
            }
        }
    }
    field
}

/// The lattice cell containing `coord`, and the smoothstepped position within it.
fn cell(coord: u16, spacing: u16) -> (usize, f32) {
    let t = f32::from(coord % spacing) / f32::from(spacing);
    (usize::from(coord / spacing), t * t * (3.0 - 2.0 * t))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::test_util::builtin;

    /// How many tiles of each terrain `map` has.
    fn counts(map: &Map) -> [usize; 6] {
        let mut counts = [0; 6];
        for pos in map.positions() {
            let slot = match map.terrain(pos) {
                Terrain::DeepWater => 0,
                Terrain::ShallowWater => 1,
                Terrain::Sand => 2,
                Terrain::Dirt => 3,
                Terrain::Grass => 4,
                Terrain::Rock => 5,
            };
            counts[slot] += 1;
        }
        counts
    }

    #[test]
    fn the_heights_fall_into_bands_of_20_12_8_50_and_10_percent_and_40_percent_of_land_is_dirt() {
        let data = builtin();
        let config = WorldConfig::from_ron("(width: 64, height: 40)", &data).expect("valid");
        for seed in [1, 2, 3] {
            let map = banded(&config, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            // 2,560 tiles: a tile is in a band if under that share of the
            // tiles rank below it.
            let [deep, shallow, sand, dirt, grass, rock] = counts(&map);
            assert_eq!((deep, shallow, sand), (512, 308, 204), "seed {seed}");
            assert_eq!((dirt + grass, rock), (1280, 256), "seed {seed}");
            assert_eq!(dirt, 512, "40% of the land, seed {seed}");
        }
    }

    #[test]
    fn with_4_percent_water_the_sand_land_and_rock_bands_keep_their_tops() {
        // M2 design §19.1: deep and shallow water keep the proportion 20 to
        // 12 (2.5% and 1.5%), sand stays an 8% band above the water, and
        // rock the top 10%, so the land is 78%.
        let data = builtin();
        let config =
            WorldConfig::from_ron("(width: 64, height: 40, water: 4, lake_size: 3)", &data)
                .expect("valid");
        for seed in [1, 2, 3] {
            let map = banded(&config, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            let [deep, shallow, sand, dirt, grass, rock] = counts(&map);
            assert_eq!((deep, shallow, sand), (64, 39, 205), "seed {seed}");
            assert_eq!((dirt + grass, rock), (1996, 256), "seed {seed}");
        }
    }

    #[test]
    fn a_region_of_64_tiles_is_joined_and_one_of_63_becomes_rock() {
        // The mainland on the left; two regions on the right, 64 tiles
        // above and 63 below, each a wall away from it.
        let mut rows = Vec::new();
        for y in 0..17 {
            let right = match y {
                8 => "########",
                16 => ".......#",
                _ => "........",
            };
            rows.push(format!("{}#{right}", ".".repeat(12)));
        }
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let mut map = Map::from_ascii(&rows, &builtin()).expect("valid drawing");
        regions::connect(&mut map, MIN_REGION);
        let walkable = |x: u16, y: u16| map.is_walkable(Pos { x, y });
        assert!(walkable(13, 0), "the 64 stay land");
        assert!(!walkable(13, 9), "the 63 became rock");
        assert!(
            (0..8).any(|y| walkable(12, y)),
            "the 64 were joined through the wall: {:#?}",
            map.to_ascii()
        );
    }
}
