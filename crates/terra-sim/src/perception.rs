//! Perception (design §3.6): each sprite's bounded Dijkstra flood, which
//! gives both what it can reach and the way there.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use serde::{Serialize, Serializer};

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::objects::{EntityId, Objects};
use crate::physics::{beside, entry_cost, step};
use crate::random::uniform;
use crate::registry::Category;
use crate::sprites::Sprites;

/// How a search treats tiles that hold another sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupied {
    /// Crossable, at this many extra terrain units: sprites move.
    Penalty(u32),
    /// Never entered: blocked re-planning's one-off search (design §3.7).
    Closed,
}

/// Something a sprite can aim a verb at (design §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Target {
    /// An object, such as a berry or a bush.
    Object(EntityId),
    /// A tile of drinkable water.
    Water(Pos),
    /// Another sprite.
    Sprite(EntityId),
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
                    (Occupied::Closed, Some(_)) => continue,
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

    /// How far it reaches, in tiles in any direction.
    pub(crate) fn reach(&self) -> u16 {
        self.radius
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
    /// the flood reached more than half of `sense_radius` from the origin,
    /// the sprite's trait as it is, before the flood rounds it; failing any,
    /// from every tile it reached but the origin; `None` if it reached none.
    /// One draw from `rng`, if there's a tile to draw.
    pub(crate) fn wander_destination(
        &self,
        sense_radius: f32,
        rng: &mut ChaCha8Rng,
    ) -> Option<Pos> {
        let reached: Vec<Pos> = (0..self.costs.len())
            .filter(|&index| self.costs[index] != UNREACHED)
            .map(|index| self.pos(index))
            .filter(|&pos| pos != self.origin)
            .collect();
        let origin = self.origin;
        let far: Vec<Pos> = reached
            .iter()
            .copied()
            .filter(|pos| {
                let distance = pos.x.abs_diff(origin.x).max(pos.y.abs_diff(origin.y));
                f32::from(distance) > sense_radius / 2.0
            })
            .collect();
        let pool = if far.is_empty() { reached } else { far };
        if pool.is_empty() {
            return None;
        }
        Some(pool[uniform(rng, pool.len() as u64) as usize])
    }

    /// The candidate of each category around the flood's origin (design
    /// §3.6), with the cost of the way to it: the nearest reachable thing of
    /// that category, the one with the lowest (cost, ID), where water's ID is
    /// its tile index. A thing is reachable if the flood reached one of its
    /// goal tiles: beside it, or, for an item or water, its own tile too.
    /// `me` is the sprite the flood is for, which is never its own candidate.
    pub(crate) fn candidates(
        &self,
        ground: Ground,
        me: EntityId,
    ) -> BTreeMap<Category, (Target, u32)> {
        let map = ground.map;
        let mut best: BTreeMap<Category, (u32, u64, Target)> = BTreeMap::new();
        let mut offer = |category, target, id, cost| {
            let better = best
                .get(&category)
                .is_none_or(|&(c, i, _)| (cost, id) < (c, i));
            if better {
                best.insert(category, (cost, id, target));
            }
        };
        // Things just outside the square can have goal tiles inside it.
        let x0 = self.corner.x.saturating_sub(1);
        let y0 = self.corner.y.saturating_sub(1);
        let x1 = (self.corner.x + self.width).min(map.width() - 1);
        let y1 = (self.corner.y + self.height).min(map.height() - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let pos = Pos { x, y };
                if let Some(id) = ground.objects.at(pos) {
                    let object_type = &ground.data.object_types()[ground.objects.kind(id)];
                    let own_tile = !object_type.solid;
                    if let Some(cost) = self.goal_cost(map, pos, own_tile) {
                        offer(object_type.category, Target::Object(id), id.0, cost);
                    }
                }
                if ground.data.terrain(map.terrain(pos)).is_drinkable()
                    && let Some(cost) = self.goal_cost(map, pos, true)
                {
                    offer(
                        Category::Water,
                        Target::Water(pos),
                        map.index(pos) as u64,
                        cost,
                    );
                }
                if let Some(id) = ground.sprites.at(pos).filter(|&id| id != me)
                    && let Some(cost) = self.goal_cost(map, pos, false)
                {
                    offer(Category::Sprite, Target::Sprite(id), id.0, cost);
                }
            }
        }
        best.into_iter()
            .map(|(category, (cost, _, target))| (category, (target, cost)))
            .collect()
    }

    /// The cost of the cheapest way to a goal tile of a thing on `pos`: a
    /// tile beside it, or, if `own_tile`, its own tile too. `None` if the
    /// flood reached none.
    fn goal_cost(&self, map: &Map, pos: Pos, own_tile: bool) -> Option<u32> {
        self.nearest_goal(map, pos, own_tile).map(|(_, cost)| cost)
    }

    /// The goal tile of a thing on `pos` the flood reaches most cheaply,
    /// with its cost: ties go to the lower tile index. `None` if it reached
    /// none.
    pub(crate) fn nearest_goal(&self, map: &Map, pos: Pos, own_tile: bool) -> Option<(Pos, u32)> {
        goal_tiles(map, pos, own_tile)
            .filter_map(|goal| Some((goal, self.cost(goal)?)))
            .min_by_key(|&(goal, cost)| (cost, map.index(goal)))
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

/// The goal tiles of a thing on `pos` (design §3.6): the tiles beside it,
/// and, if `own_tile`, its own tile too.
pub(crate) fn goal_tiles(
    map: &Map,
    pos: Pos,
    own_tile: bool,
) -> impl Iterator<Item = Pos> + use<'_> {
    let beside = Dir::ALL
        .into_iter()
        .filter_map(move |dir| map.neighbour(pos, dir));
    beside.chain(own_tile.then_some(pos))
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
    use std::collections::{BTreeMap, BTreeSet};

    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::ecology::new_object;
    use crate::sprites::Sprite;

    /// What a flood spreads through: a map drawn from `rows`, with no
    /// objects, and starter sprites on `sprites`, with IDs from 1.
    fn parts(rows: &[&str], sprites: &[Pos]) -> (Map, Objects, Sprites, DataPack) {
        parts_with(rows, &[], sprites)
    }

    /// `parts`, with objects as `(tile, object type)`, with IDs from 101.
    fn parts_with(
        rows: &[&str],
        objects: &[(Pos, &str)],
        sprites: &[Pos],
    ) -> (Map, Objects, Sprites, DataPack) {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let map = Map::from_ascii(rows, &data).expect("valid drawing");
        let mut placed_objects = Objects::new(&map);
        for (n, &(pos, name)) in objects.iter().enumerate() {
            let kind = data.object_type_named(name).expect("a built-in type");
            placed_objects.place(EntityId(n as u64 + 101), new_object(&data, kind, pos));
        }
        let mut placed = Sprites::new(&map);
        for (n, &pos) in sprites.iter().enumerate() {
            let sprite = Sprite::newborn(data.starter().clone(), pos, 0, &data);
            placed.place(EntityId(n as u64 + 1), sprite);
        }
        (map, placed_objects, placed, data)
    }

    /// The candidates of sprite 1, the first on `sprites`, which the flood
    /// spreads from, with a radius of 10.
    fn candidates_of(
        rows: &[&str],
        objects: &[(Pos, &str)],
        sprites: &[Pos],
    ) -> BTreeMap<Category, (Target, u32)> {
        let (map, objects, sprites_placed, data) = parts_with(rows, objects, sprites);
        let ground = Ground {
            map: &map,
            objects: &objects,
            sprites: &sprites_placed,
            data: &data,
        };
        let flood = Flood::new(ground, sprites[0], 10, Occupied::Penalty(30), 0);
        flood.candidates(ground, EntityId(1))
    }

    #[test]
    fn the_candidate_of_each_kind_is_the_nearest_one_the_sprite_can_reach() {
        let berries = [(at(3, 0), "berry"), (at(6, 0), "berry")];
        let found = candidates_of(&["........"], &berries, &[at(0, 0)]);
        // Berries are items: a sprite can take one from its tile or beside it.
        assert_eq!(found[&Category::Berry], (Target::Object(EntityId(101)), 20));
    }

    #[test]
    fn candidates_the_same_distance_away_go_to_the_lower_id() {
        let berries = [(at(4, 0), "berry"), (at(0, 0), "berry")];
        let found = candidates_of(&["....."], &berries, &[at(2, 0)]);
        assert_eq!(found[&Category::Berry], (Target::Object(EntityId(101)), 10));
    }

    #[test]
    fn water_is_told_apart_by_tile_index_and_can_be_drunk_from_its_own_tile() {
        let found = candidates_of(&["~.~", "..."], &[], &[at(1, 1)]);
        assert_eq!(found[&Category::Water], (Target::Water(at(0, 0)), 0));
        let found = candidates_of(&["..~"], &[], &[at(0, 0)]);
        assert_eq!(found[&Category::Water], (Target::Water(at(2, 0)), 10));
    }

    #[test]
    fn a_bush_or_a_sprite_is_reached_from_beside_it() {
        let found = candidates_of(
            &["......"],
            &[(at(5, 0), "thornbush")],
            &[at(0, 0), at(3, 0)],
        );
        assert_eq!(
            found[&Category::Thornbush],
            (Target::Object(EntityId(101)), 40 + 30)
        );
        assert_eq!(found[&Category::Sprite], (Target::Sprite(EntityId(2)), 20));
    }

    #[test]
    fn a_nearer_thing_the_sprite_cannot_reach_is_never_its_candidate() {
        // The near berry is walled in; the far one is in the open.
        let rows = ["...#.#", "...###", "......"];
        let berries = [(at(4, 0), "berry"), (at(0, 2), "berry")];
        let found = candidates_of(&rows, &berries, &[at(2, 0)]);
        // Taken from beside it, one diagonal step away.
        assert_eq!(found[&Category::Berry], (Target::Object(EntityId(102)), 14));
    }

    #[test]
    fn a_sprite_is_never_its_own_candidate() {
        let found = candidates_of(&["..."], &[], &[at(0, 0)]);
        assert!(!found.contains_key(&Category::Sprite));
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
            .map(|_| flood.wander_destination(f32::from(flood.radius), &mut rng))
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
    fn the_half_radius_a_wander_must_beat_is_the_sense_radius_s_not_the_flood_s() {
        // Sense radius 9.6: the flood reaches 10 tiles, and a wander heads
        // more than 4.8 away, so 5 tiles is far enough.
        let flood = flood(&["..........."], &[], at(0, 0), 10, 0);
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let drawn: BTreeSet<Option<Pos>> = (0..2_000)
            .map(|_| flood.wander_destination(9.6, &mut rng))
            .collect();
        let far: BTreeSet<Option<Pos>> = (5..=10).map(|x| Some(at(x, 0))).collect();
        assert_eq!(drawn, far);
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
