//! World generation (design §3.2): seeded value noise, terrain bands by
//! percentile, then one region.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::Rng;

use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::map::Map;
use crate::regions;
use crate::terrain::Terrain;

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

/// A random value in [0, 1) from the top 24 bits of a draw, so it is exact in `f32`.
fn unit(rng: &mut ChaCha8Rng) -> f32 {
    (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32
}
