//! World generation (design §3.2): seeded value noise, terrain bands by
//! percentile, then one region.

use rand_chacha::ChaCha8Rng;

use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::ecology::aged_object;
use crate::map::{Map, Pos};
use crate::random::{uniform, unit};
use crate::regions;
use crate::sprites::Sprite;
use crate::terrain::Terrain;
use crate::variation::varied;
use crate::world::WorldState;

/// Regions smaller than this become rock; larger ones are joined to the mainland.
const MIN_REGION: usize = 64;

/// Noise octaves as (lattice spacing in tiles, weight): broad shapes plus detail.
const OCTAVES: [(u16, f32); 3] = [(24, 1.0), (12, 0.5), (6, 0.25)];

/// Height bands, as the percentage of all tiles at or below each band's top.
const DEEP_WATER_TOP: usize = 20;
const SHALLOW_WATER_TOP: usize = 32;
const SAND_TOP: usize = 40;
const LAND_TOP: usize = 90; // above this is rock
/// The driest share of the land, as a percentage, is dirt; the rest is grass.
const DIRT_SHARE: usize = 40;

/// Generates a connected map for a new world, drawing randomness from the world's RNG.
pub(crate) fn generate(config: &WorldConfig, data: &DataPack, rng: &mut ChaCha8Rng) -> Map {
    let (width, height) = (config.width(), config.height());
    let heights = noise(width, height, rng);
    let moisture = noise(width, height, rng);

    let mut map = Map::filled(width, height, Terrain::Grass, data);
    let count = map.tile_count();
    let mut land = Vec::new();
    for (rank, index) in ranked(&heights).into_iter().enumerate() {
        let percent_below = rank * 100;
        let terrain = if percent_below < count * DEEP_WATER_TOP {
            Terrain::DeepWater
        } else if percent_below < count * SHALLOW_WATER_TOP {
            Terrain::ShallowWater
        } else if percent_below < count * SAND_TOP {
            Terrain::Sand
        } else if percent_below < count * LAND_TOP {
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

    regions::connect(&mut map, MIN_REGION);
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
            state.add_object(object);
        }
    }
}

/// Places the preset's first population (design §3.2), after the objects:
/// each sprite on a tile drawn uniformly from the walkable tiles holding no
/// object and no sprite, made from the starter genome with spawn variation,
/// and with its energy and hydration drawn between physiology's
/// `first_population` fractions of the newborn level, so the sprites don't
/// all run dry together. If the map runs out of room, the rest are skipped.
pub(crate) fn place_sprites(config: &WorldConfig, data: &DataPack, state: &mut WorldState) {
    let mut candidates: Vec<Pos> = state
        .map
        .positions()
        .filter(|&pos| state.can_stand(data, pos) && state.objects.at(pos).is_none())
        .collect();
    let physiology = data.physiology();
    let (low, high) = physiology.first_population;
    for _ in 0..config.sprites() {
        if candidates.is_empty() {
            break;
        }
        let choice = uniform(&mut state.rng, candidates.len() as u64) as usize;
        let pos = candidates.swap_remove(choice);
        let genome = varied(data.starter(), data, &mut state.rng);
        let mut sprite = Sprite::newborn(genome, pos, state.tick, data);
        for index in [physiology.indices.energy, physiology.indices.hydration] {
            let level = sprite.body.chems[index] * (low + (high - low) * unit(&mut state.rng));
            sprite.body.start_at(index, level);
        }
        state.add_sprite(sprite);
    }
}

/// Indices of `values` from lowest to highest value; ties go to the lower index.
fn ranked(values: &[f32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]).then(a.cmp(&b)));
    order
}

/// A field of smooth value noise, one value per tile in tile-index order.
///
/// Each octave draws random values on a coarse lattice and blends between them
/// with smoothstep. Only + − × ÷ are used, so results are identical everywhere.
fn noise(width: u16, height: u16, rng: &mut ChaCha8Rng) -> Vec<f32> {
    let mut field = vec![0.0; usize::from(width) * usize::from(height)];
    for (spacing, weight) in OCTAVES {
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
