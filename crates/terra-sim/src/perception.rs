//! Perception (design §3.6): each sprite's bounded Dijkstra flood, which
//! gives both what it can reach and the way there.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rand_chacha::ChaCha8Rng;
use serde::{Serialize, Serializer};

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::objects::Objects;
use crate::physics::{beside, entry_cost, step};
use crate::random::uniform;
use crate::sprites::Sprites;

/// How a search treats tiles that hold another sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupied {
    /// Crossable, at this many extra terrain units: sprites move.
    Penalty(u32),
}

/// No step reached the tile.
const UNREACHED: u32 = u32::MAX;
/// The direction marker for a tile no step reached, or the origin.
const NO_STEP: u8 = u8::MAX;

/// A sprite's flood: the cheapest cost to each tile it reaches within its
/// radius, and the step that reached it. It's cached and saved (design §2.8),
/// so it's part of the world state.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Flood {
    /// The tile it spreads from.
    pub(crate) origin: Pos,
    /// The tick it was made on, for its refresh.
    pub(crate) made: u64,
    /// How far it reaches, in tiles in any direction.
    radius: u16,
    /// The top-left tile of the square it covers.
    corner: Pos,
    width: u16,
    height: u16,
    /// The cost to reach each tile of the square, row by row, in terrain units.
    #[serde(serialize_with = "costs_as_bytes")]
    costs: Vec<u32>,
    /// The direction of the step that reached each tile, as its place in `Dir::ALL`.
    #[serde(serialize_with = "as_bytes")]
    steps: Vec<u8>,
}

/// What a flood spreads through: the map, and what stands on it.
#[derive(Clone, Copy)]
pub(crate) struct Ground<'a> {
    pub(crate) map: &'a Map,
    pub(crate) objects: &'a Objects,
    pub(crate) sprites: &'a Sprites,
    pub(crate) data: &'a DataPack,
}

impl Flood {
    /// The flood from `origin` over the tiles within Chebyshev distance
    /// `radius`, made on the tick `tick`.
    pub(crate) fn new(
        ground: Ground,
        origin: Pos,
        radius: u16,
        occupied: Occupied,
        tick: u64,
    ) -> Flood {
        let map = ground.map;
        let x0 = origin.x.saturating_sub(radius);
        let y0 = origin.y.saturating_sub(radius);
        let x1 = origin.x.saturating_add(radius).min(map.width() - 1);
        let y1 = origin.y.saturating_add(radius).min(map.height() - 1);
        let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
        let tiles = usize::from(width) * usize::from(height);
        let mut flood = Flood {
            origin,
            made: tick,
            radius,
            corner: Pos { x: x0, y: y0 },
            width,
            height,
            costs: vec![UNREACHED; tiles],
            steps: vec![NO_STEP; tiles],
        };
        // What stepping onto each tile of the square costs, found once.
        let entry: Vec<Option<u32>> = (0..tiles)
            .map(|index| entry_cost(map, ground.objects, ground.data, flood.pos(index)))
            .collect();
        let start = flood
            .local(origin)
            .expect("the origin is in its own square");
        flood.costs[start] = 0;
        // Ties settle in tile order, so the flood is the same everywhere.
        let mut queue = BinaryHeap::from([Reverse((0, start))]);
        while let Some(Reverse((cost, index))) = queue.pop() {
            if cost > flood.costs[index] {
                continue;
            }
            let from = flood.pos(index);
            for (d, &dir) in Dir::ALL.iter().enumerate() {
                let Some(to) = map.neighbour(from, dir) else {
                    continue;
                };
                let Some(next) = flood.local(to) else {
                    continue;
                };
                let open = |pos| flood.local(pos).is_some_and(|i| entry[i].is_some());
                let beside_open = || beside(from, to).into_iter().all(open);
                let Some(step) = step(entry[next], dir, beside_open) else {
                    continue;
                };
                let extra = match (occupied, ground.sprites.at(to)) {
                    (_, None) => 0,
                    _ if to == origin => 0,
                    (Occupied::Penalty(penalty), Some(_)) => penalty,
                };
                let total = cost + step + extra;
                if total < flood.costs[next] {
                    flood.costs[next] = total;
                    flood.steps[next] = d as u8;
                    queue.push(Reverse((total, next)));
                }
            }
        }
        flood
    }

    /// The cost of the cheapest way to `pos`, or `None` if the flood didn't reach it.
    pub(crate) fn cost(&self, pos: Pos) -> Option<u32> {
        let index = self.local(pos)?;
        (self.costs[index] != UNREACHED).then_some(self.costs[index])
    }

    /// The tiles of the cheapest way from the origin to `pos`, not counting
    /// the origin, or `None` if the flood didn't reach it.
    pub(crate) fn path_to(&self, pos: Pos) -> Option<Vec<Pos>> {
        self.cost(pos)?;
        let mut path = Vec::new();
        let mut at = pos;
        while at != self.origin {
            path.push(at);
            let index = self.local(at).expect("a reached tile is in the square");
            let dir = Dir::ALL[usize::from(self.steps[index])];
            at = back(at, dir);
        }
        path.reverse();
        Some(path)
    }

    /// Where a Wander heads (design §5.5): a tile drawn uniformly from those
    /// the flood reached more than half its radius from the origin; failing
    /// any, from every tile it reached but the origin; `None` if it reached
    /// none. One draw from `rng`, if there's a tile to draw.
    pub(crate) fn wander_destination(&self, rng: &mut ChaCha8Rng) -> Option<Pos> {
        let reached: Vec<Pos> = (0..self.costs.len())
            .filter(|&index| self.costs[index] != UNREACHED)
            .map(|index| self.pos(index))
            .filter(|&pos| pos != self.origin)
            .collect();
        let origin = self.origin;
        let far: Vec<Pos> = reached
            .iter()
            .copied()
            .filter(|pos| 2 * pos.x.abs_diff(origin.x).max(pos.y.abs_diff(origin.y)) > self.radius)
            .collect();
        let pool = if far.is_empty() { reached } else { far };
        if pool.is_empty() {
            return None;
        }
        Some(pool[uniform(rng, pool.len() as u64) as usize])
    }

    /// The index of `pos` in the square, or `None` if it's outside.
    fn local(&self, pos: Pos) -> Option<usize> {
        let x = pos.x.checked_sub(self.corner.x)?;
        let y = pos.y.checked_sub(self.corner.y)?;
        (x < self.width && y < self.height)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    /// The tile at `index` in the square.
    fn pos(&self, index: usize) -> Pos {
        let width = usize::from(self.width);
        Pos {
            x: self.corner.x + (index % width) as u16,
            y: self.corner.y + (index / width) as u16,
        }
    }
}

/// Serializes bytes compactly, for the state hash.
fn as_bytes<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(bytes)
}

/// Serializes costs compactly, as little-endian bytes, for the state hash.
fn costs_as_bytes<S: Serializer>(costs: &[u32], serializer: S) -> Result<S::Ok, S::Error> {
    let bytes: Vec<u8> = costs.iter().flat_map(|cost| cost.to_le_bytes()).collect();
    serializer.serialize_bytes(&bytes)
}

/// The tile one step back from `pos` against direction `dir`.
fn back(pos: Pos, dir: Dir) -> Pos {
    let (dx, dy) = dir.offset();
    Pos {
        x: (i32::from(pos.x) - dx) as u16,
        y: (i32::from(pos.y) - dy) as u16,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::objects::EntityId;
    use crate::sprites::Sprite;

    /// What a flood spreads through: a map drawn from `rows`, with no
    /// objects, and starter sprites on `sprites`.
    fn parts(rows: &[&str], sprites: &[Pos]) -> (Map, Objects, Sprites, DataPack) {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let map = Map::from_ascii(rows, &data).expect("valid drawing");
        let objects = Objects::new(&map);
        let mut placed = Sprites::new(&map);
        for (n, &pos) in sprites.iter().enumerate() {
            let sprite = Sprite::newborn(data.starter().clone(), pos, 0, &data);
            placed.place(EntityId(n as u64 + 1), sprite);
        }
        (map, objects, placed, data)
    }

    fn flood(rows: &[&str], sprites: &[Pos], origin: Pos, radius: u16, penalty: u32) -> Flood {
        let (map, objects, sprites, data) = parts(rows, sprites);
        let ground = Ground {
            map: &map,
            objects: &objects,
            sprites: &sprites,
            data: &data,
        };
        Flood::new(ground, origin, radius, Occupied::Penalty(penalty), 0)
    }

    fn at(x: u16, y: u16) -> Pos {
        Pos { x, y }
    }

    /// Every destination `flood` gives in 2,000 draws.
    fn destinations(flood: &Flood) -> BTreeSet<Option<Pos>> {
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        (0..2_000)
            .map(|_| flood.wander_destination(&mut rng))
            .collect()
    }

    #[test]
    fn a_wander_heads_for_a_tile_farther_than_half_the_radius_when_there_is_one() {
        // Radius 2 on open ground: the far tiles are the 16 of the outer ring.
        let flood = flood(&["....."; 5], &[], at(2, 2), 2, 0);
        let drawn = destinations(&flood);
        let ring: BTreeSet<Option<Pos>> = (0..5)
            .flat_map(|y| (0..5).map(move |x| at(x, y)))
            .filter(|p| p.x == 0 || p.x == 4 || p.y == 0 || p.y == 4)
            .map(Some)
            .collect();
        assert_eq!(drawn, ring);
    }

    #[test]
    fn with_no_tile_that_far_a_wander_heads_for_any_it_reaches() {
        // Radius 4 wants more than 2 tiles away; this corridor has only 2.
        let flood = flood(&["..."], &[], at(0, 0), 4, 0);
        let drawn = destinations(&flood);
        assert_eq!(drawn, BTreeSet::from([Some(at(1, 0)), Some(at(2, 0))]));
    }

    #[test]
    fn with_nowhere_to_go_a_wander_has_no_destination() {
        let flood = flood(&[".#"], &[], at(0, 0), 4, 0);
        assert_eq!(destinations(&flood), BTreeSet::from([None]));
    }

    #[test]
    fn a_tile_holding_another_sprite_costs_the_penalty_more_to_cross() {
        let origin = at(0, 0);
        let flood = flood(&["....."], &[origin, at(2, 0)], origin, 10, 30);
        assert_eq!(flood.cost(at(1, 0)), Some(10));
        assert_eq!(flood.cost(at(2, 0)), Some(50));
        assert_eq!(flood.cost(at(4, 0)), Some(70));
    }

    #[test]
    fn the_flood_reaches_tiles_within_its_radius_only() {
        let flood = flood(&["........."], &[], at(4, 0), 3, 0);
        assert_eq!(flood.cost(at(1, 0)), Some(30));
        assert_eq!(flood.cost(at(7, 0)), Some(30));
        assert_eq!(flood.cost(at(0, 0)), None);
        assert_eq!(flood.cost(at(8, 0)), None);
    }

    #[test]
    fn the_path_to_a_tile_is_its_steps_from_the_origin() {
        let flood = flood(&["...", ".#.", "..."], &[], at(0, 1), 5, 0);
        assert_eq!(flood.path_to(at(0, 1)), Some(vec![]));
        // Around the rock, without cutting its corners; the way over the
        // top and the way under cost the same, and the top comes first in
        // tile order.
        assert_eq!(
            flood.path_to(at(2, 1)),
            Some(vec![at(0, 0), at(1, 0), at(2, 0), at(2, 1)])
        );
    }
}
