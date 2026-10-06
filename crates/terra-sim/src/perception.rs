//! Perception (design §3.6): each sprite's bounded Dijkstra flood, which
//! gives both what it can reach and the way there.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::objects::{EntityId, Objects};
use crate::physics::{entry_cost, step};
use crate::random::uniform;
use crate::registry::CategoryId;
use crate::sprites::Sprites;
use crate::terrain::Terrain;

/// How a search treats tiles that hold another sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupied {
    /// Crossable, at this many extra terrain units: sprites move.
    Penalty(u32),
    /// Never entered: blocked re-planning's one-off search (design §3.7).
    Closed,
}

/// Something a sprite can aim a verb at (design §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    /// An object, such as a berry or a bush.
    Object(EntityId),
    /// A tile of drinkable water.
    Water(Pos),
    /// Another sprite.
    Sprite(EntityId),
    /// The Cursor, while sprites can see it (design v29 §6.5).
    Cursor,
}

impl Target {
    /// The sprite it is, if it's one.
    pub(crate) fn sprite(self) -> Option<EntityId> {
        match self {
            Target::Sprite(sprite) => Some(sprite),
            _ => None,
        }
    }
}

/// No step reached the tile.
const UNREACHED: u32 = u32::MAX;
/// The end of a list of a flood's waiting tiles.
const END_OF_LIST: usize = usize::MAX;
/// A step no walker may take, in a flood's working costs.
const BLOCKED: u32 = u32::MAX;
/// The direction marker for a tile no step reached, or the origin.
const NO_STEP: u8 = u8::MAX;

/// A sprite's flood: the cheapest cost to each tile it reaches within its
/// radius, and the step that reached it. It's cached and saved (design §2.8),
/// so it's part of the world state.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(
        serialize_with = "costs_as_bytes",
        deserialize_with = "costs_from_bytes"
    )]
    costs: Vec<u32>,
    /// The direction of the step that reached each tile, as its place in `Dir::ALL`.
    #[serde(
        serialize_with = "as_bytes",
        deserialize_with = "crate::save::read_bytes"
    )]
    steps: Vec<u8>,
    /// The Water candidate, once asked for: the drinkable tile with the
    /// cheapest goal tile, ties to the lower tile index, and that cost. It
    /// follows from the costs and the terrain, which never changes once the
    /// map is made, so it isn't state and isn't saved; the candidates of a
    /// flood that lives for several ticks find it once.
    #[serde(skip)]
    water: OnceLock<Option<(Pos, u32)>>,
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
        debug_assert_eq!(
            Dir::ALL.map(Dir::offset),
            [
                (0, -1),
                (1, -1),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1)
            ],
            "the search below takes its steps in this order"
        );
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
            water: OnceLock::new(),
        };
        // The search runs on the square with a ring of closed tiles round
        // it, so every step from a tile in the square lands on the padded
        // square, and needs no check that it's on the map. Row order is the
        // same in both, so ties settle the same way.
        let (w, h) = (usize::from(width), usize::from(height));
        let padded = w + 2;
        let at = |x: usize, y: usize| (y + 1) * padded + (x + 1);
        // What stepping onto each tile costs, straight and diagonally,
        // another sprite's penalty included; whether a diagonal step may
        // pass it at a corner; all found once.
        let mut straight = vec![BLOCKED; padded * (h + 2)];
        let mut diagonal = vec![BLOCKED; padded * (h + 2)];
        let mut open = vec![false; padded * (h + 2)];
        for (y, map_y) in (y0..=y1).enumerate() {
            for (x, map_x) in (x0..=x1).enumerate() {
                let pos = Pos { x: map_x, y: map_y };
                let Some(entry) = entry_cost(map, ground.objects, ground.data, pos) else {
                    continue;
                };
                let index = at(x, y);
                open[index] = true;
                let extra = match occupied {
                    _ if pos == origin || ground.sprites.at(pos).is_none() => 0,
                    Occupied::Penalty(penalty) => penalty,
                    Occupied::Closed => continue,
                };
                straight[index] = entry + extra;
                diagonal[index] = step(Some(entry), Dir::NE, || true).expect("walkable") + extra;
            }
        }
        let (ox, oy) = (usize::from(origin.x - x0), usize::from(origin.y - y0));
        let start = at(ox, oy);
        let mut costs = vec![UNREACHED; padded * (h + 2)];
        let mut steps = vec![NO_STEP; padded * (h + 2)];
        costs[start] = 0;
        // Tiles settle cheapest first, and ties in tile order, so the flood
        // is the same everywhere. Every step costs at least 1 and at most
        // `longest`, so the tiles waiting to settle all cost from the cost
        // being settled to `longest` more: a ring of that many buckets, one
        // per cost, holds them, and each bucket is sorted into tile order
        // when its turn comes. Nothing joins it then, since no step is free.
        let longest = straight
            .iter()
            .chain(&diagonal)
            .filter(|&&step| step != BLOCKED)
            .max()
            .map_or(1, |&step| step as usize);
        // Each bucket is a list threaded through `waiting`, newest first:
        // a tile, and the place of the next entry in its bucket.
        let ring = longest + 1;
        let mut buckets = vec![END_OF_LIST; ring];
        let mut waiting: Vec<(usize, usize)> = Vec::with_capacity(tiles * 2);
        waiting.push((start, END_OF_LIST));
        buckets[0] = 0;
        let mut queued = 1;
        let mut settling = Vec::new();
        let mut cost = 0;
        while queued > 0 {
            let slot = cost as usize % ring;
            let mut entry = std::mem::replace(&mut buckets[slot], END_OF_LIST);
            while entry != END_OF_LIST {
                let (index, next) = waiting[entry];
                settling.push(index);
                entry = next;
            }
            queued -= settling.len();
            settling.sort_unstable();
            for &index in &settling {
                // Already settled more cheaply.
                if cost > costs[index] {
                    continue;
                }
                let mut relax = |d: u8, next: usize, step: u32| {
                    if step == BLOCKED {
                        return;
                    }
                    let total = cost + step;
                    if total < costs[next] {
                        costs[next] = total;
                        steps[next] = d;
                        let slot = total as usize % ring;
                        waiting.push((next, buckets[slot]));
                        buckets[slot] = waiting.len() - 1;
                        queued += 1;
                    }
                };
                // The steps in `Dir::ALL` order, each diagonal one only past
                // two open corners: no corner is ever cut (design §3.1).
                let (up, down) = (index - padded, index + padded);
                let (north, east) = (open[up], open[index + 1]);
                let (south, west) = (open[down], open[index - 1]);
                relax(0, up, straight[up]);
                if north && east {
                    relax(1, up + 1, diagonal[up + 1]);
                }
                relax(2, index + 1, straight[index + 1]);
                if south && east {
                    relax(3, down + 1, diagonal[down + 1]);
                }
                relax(4, down, straight[down]);
                if south && west {
                    relax(5, down - 1, diagonal[down - 1]);
                }
                relax(6, index - 1, straight[index - 1]);
                if north && west {
                    relax(7, up - 1, diagonal[up - 1]);
                }
            }
            settling.clear();
            cost += 1;
        }
        for y in 0..h {
            let row = at(0, y);
            flood.costs[y * w..(y + 1) * w].copy_from_slice(&costs[row..row + w]);
            flood.steps[y * w..(y + 1) * w].copy_from_slice(&steps[row..row + w]);
        }
        flood
    }

    /// Whether a loaded flood fits `map` (design §2.8): its square is on
    /// the map and holds its origin, it has a cost and a step for each
    /// tile, and the way back from each tile it reached leads to the origin,
    /// each step to a tile reached more cheaply, as every step costs.
    pub(crate) fn fits(&self, map: &Map) -> bool {
        let tiles = usize::from(self.width) * usize::from(self.height);
        let on_map = u32::from(self.corner.x) + u32::from(self.width) <= u32::from(map.width())
            && u32::from(self.corner.y) + u32::from(self.height) <= u32::from(map.height());
        if !on_map || self.costs.len() != tiles || self.steps.len() != tiles {
            return false;
        }
        let Some(start) = self.local(self.origin) else {
            return false;
        };
        let leads_back = |index: usize| {
            let cost = self.costs[index];
            let Some(&dir) = Dir::ALL.get(usize::from(self.steps[index])) else {
                return false;
            };
            let from = self.local(back(self.pos(index), dir));
            from.is_some_and(|from| self.costs[from] < cost)
        };
        self.costs[start] == 0
            && (0..tiles)
                .filter(|&index| index != start && self.costs[index] != UNREACHED)
                .all(leads_back)
    }

    /// Sets the step to the first tile it reached past the origin to `byte`,
    /// unchecked, to test the checks on loading.
    #[cfg(test)]
    pub(crate) fn damage_a_step(&mut self, byte: u8) {
        let start = self
            .local(self.origin)
            .expect("the origin is in its square");
        let index = (0..self.costs.len())
            .find(|&index| index != start && self.costs[index] != UNREACHED)
            .expect("a tile it reached");
        self.steps[index] = byte;
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

    /// Of the tiles it reached that `free` allows, the one nearest `to` in a
    /// straight line, ties going to the cheaper way there, then to the first
    /// in row order: where a led sprite heads (design v23 §6.5). The origin
    /// is always allowed, so there's always one.
    pub(crate) fn nearest_reached(&self, to: Pos, free: impl Fn(Pos) -> bool) -> Pos {
        let squared = |pos: Pos| {
            let (dx, dy) = (
                u32::from(pos.x.abs_diff(to.x)),
                u32::from(pos.y.abs_diff(to.y)),
            );
            dx * dx + dy * dy
        };
        (0..self.costs.len())
            .filter(|&index| self.costs[index] != UNREACHED)
            .map(|index| (self.pos(index), self.costs[index], index))
            .filter(|&(pos, _, _)| pos == self.origin || free(pos))
            .min_by_key(|&(pos, cost, index)| (squared(pos), cost, index))
            .map_or(self.origin, |(pos, _, _)| pos)
    }

    /// The corners of every tile a thing with a goal tile in the flood's
    /// square could stand on, top-left and bottom-right: the square, and a
    /// tile round it, since things just outside it can have goal tiles
    /// inside it.
    fn near(&self, map: &Map) -> (Pos, Pos) {
        let x0 = self.corner.x.saturating_sub(1);
        let y0 = self.corner.y.saturating_sub(1);
        let x1 = (self.corner.x + self.width).min(map.width() - 1);
        let y1 = (self.corner.y + self.height).min(map.height() - 1);
        (Pos { x: x0, y: y0 }, Pos { x: x1, y: y1 })
    }

    /// Every other sprite the flood reaches, with the path cost to its
    /// nearest goal tile, in ID order (design v18 §3.6).
    fn sprites(&self, ground: Ground, me: EntityId) -> Vec<(EntityId, u32)> {
        let map = ground.map;
        let (top_left, bottom_right) = self.near(map);
        let near = |pos: Pos| {
            (top_left.x..=bottom_right.x).contains(&pos.x)
                && (top_left.y..=bottom_right.y).contains(&pos.y)
        };
        // Fewer sprites than tiles near: look at each sprite, in ID order,
        // rather than each tile.
        ground
            .sprites
            .iter()
            .filter(|&(id, sprite)| id != me && near(sprite.pos))
            .filter(|&(id, sprite)| ground.sprites.at(sprite.pos) == Some(id))
            .filter_map(|(id, sprite)| Some((id, self.goal_cost(sprite.pos, false)?)))
            .collect()
    }

    /// What each category around the flood's origin offers as its
    /// candidate (design v19 §3.6), with the cost of the way to each: for
    /// each object type in it, the nearest reachable thing of that type, the
    /// one with the lowest (cost, ID), where water's ID is its tile index; in
    /// ID order. Sprites are weighed one by one, so the Sprite category
    /// offers every other sprite it reaches, in ID order (design v18 §3.6),
    /// or, while it feels a hit by `attacker`, the attacker alone, if it
    /// reaches it. Which of them draws the eye most is the brain's to say. A
    /// thing is reachable if the flood reached one of its goal tiles: beside
    /// it, or, for an item, water or the Cursor, its own tile too. `me` is
    /// the sprite the flood is for, which is never its own candidate. The
    /// Cursor is one where sprites see it, `cursor` (design v29 §6.5).
    pub(crate) fn candidates(
        &self,
        ground: Ground,
        me: EntityId,
        attacker: Option<EntityId>,
        cursor: Option<Pos>,
    ) -> BTreeMap<CategoryId, Vec<(Target, u32)>> {
        let map = ground.map;
        // By category and object type, by its index; none for water, which
        // is of one type.
        let mut best: BTreeMap<(CategoryId, Option<usize>), (u32, u64, Target)> = BTreeMap::new();
        let mut offer = |key, target, id, cost| {
            let better = best.get(&key).is_none_or(|&(c, i, _)| (cost, id) < (c, i));
            if better {
                best.insert(key, (cost, id, target));
            }
        };
        let (top_left, bottom_right) = self.near(map);
        let (x0, x1) = (top_left.x, bottom_right.x);
        // Row by row, as the objects store their tiles.
        for y in top_left.y..=bottom_right.y {
            for (x, &object) in (x0..).zip(ground.objects.row(y, x0, x1)) {
                let Some(id) = object else {
                    continue;
                };
                let pos = Pos { x, y };
                let index = ground.objects.kind(id);
                let object_type = &ground.data.object_types()[index];
                let own_tile = !object_type.solid;
                if let Some(cost) = self.goal_cost(pos, own_tile) {
                    let key = (object_type.category, Some(index));
                    offer(key, Target::Object(id), id.0, cost);
                }
            }
        }
        let water = self.water.get_or_init(|| self.water(ground));
        if let &Some((pos, cost)) = water {
            offer(
                (ground.data.water_category(), None),
                Target::Water(pos),
                map.index(pos) as u64,
                cost,
            );
        }
        let mut offered: BTreeMap<CategoryId, Vec<(u64, Target, u32)>> = BTreeMap::new();
        for ((category, _), (cost, id, target)) in best {
            offered
                .entry(category)
                .or_default()
                .push((id, target, cost));
        }
        let mut candidates: BTreeMap<CategoryId, Vec<(Target, u32)>> = offered
            .into_iter()
            .map(|(category, mut things)| {
                things.sort_by_key(|&(id, ..)| id);
                let things = things.into_iter().map(|(_, target, cost)| (target, cost));
                (category, things.collect())
            })
            .collect();
        // While it feels a hit, the Sprite candidate is the attacker, if
        // it can reach it (design §3.6).
        let attacker = attacker.filter(|&id| id != me).and_then(|id| {
            let pos = ground.sprites.get(id)?.pos;
            Some((Target::Sprite(id), self.goal_cost(pos, false)?))
        });
        let sprites: Vec<(Target, u32)> = match attacker {
            Some(attacker) => vec![attacker],
            None => self
                .sprites(ground, me)
                .into_iter()
                .map(|(id, cost)| (Target::Sprite(id), cost))
                .collect(),
        };
        if !sprites.is_empty() {
            candidates.insert(ground.data.sprite_category(), sprites);
        }
        // It's light, so a sprite may stand under it (design v29 §3.6).
        if let Some(cost) = cursor.and_then(|tile| self.goal_cost(tile, true)) {
            let cursor = ground.data.cursor_category();
            candidates.insert(cursor, vec![(Target::Cursor, cost)]);
        }
        candidates
    }

    /// The drinkable tile whose goal tiles the flood reaches most cheaply,
    /// ties to the lower tile index, and that cost (design §3.6): the Water
    /// candidate.
    fn water(&self, ground: Ground) -> Option<(Pos, u32)> {
        let map = ground.map;
        let drinkable = Terrain::ALL.map(|terrain| ground.data.terrain(terrain).is_drinkable());
        let (top_left, bottom_right) = self.near(map);
        let (x0, x1) = (top_left.x, bottom_right.x);
        let mut water: Option<(Pos, u32)> = None;
        // Tiles come in index order, so the first of the cheapest is the one
        // with the lowest index.
        for y in top_left.y..=bottom_right.y {
            for (x, &terrain) in (x0..).zip(map.terrain_row(y, x0, x1)) {
                let pos = Pos { x, y };
                if drinkable[terrain as usize]
                    && let Some(cost) = self.goal_cost(pos, true)
                    && water.is_none_or(|(_, cheapest)| cost < cheapest)
                {
                    water = Some((pos, cost));
                }
            }
        }
        water
    }

    /// The cost of the cheapest way to a goal tile of a thing on `pos`: a
    /// tile beside it, or, if `own_tile`, its own tile too. `None` if the
    /// flood reached none.
    fn goal_cost(&self, pos: Pos, own_tile: bool) -> Option<u32> {
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        let local = (
            usize::from(pos.x).wrapping_sub(usize::from(self.corner.x)),
            usize::from(pos.y).wrapping_sub(usize::from(self.corner.y)),
        );
        let best = match local {
            // Inside the square, away from its edge: all nine tiles are in it.
            (x, y) if (1..w - 1).contains(&x) && (1..h - 1).contains(&y) => {
                let mut best = UNREACHED;
                for row in [y - 1, y, y + 1] {
                    let costs = &self.costs[row * w + x - 1..=row * w + x + 1];
                    best = best.min(costs[0]).min(costs[2]);
                    if row != y || own_tile {
                        best = best.min(costs[1]);
                    }
                }
                best
            }
            // Elsewhere, the tiles round `pos` the square holds: it lies
            // inside the map, so they're all on it.
            _ => {
                let (x, y) = (i32::from(pos.x), i32::from(pos.y));
                let (cx, cy) = (i32::from(self.corner.x), i32::from(self.corner.y));
                let (w, h) = (i32::from(self.width), i32::from(self.height));
                let mut best = UNREACHED;
                for ty in (y - 1).max(cy)..=(y + 1).min(cy + h - 1) {
                    let row = (ty - cy) * w;
                    for tx in (x - 1).max(cx)..=(x + 1).min(cx + w - 1) {
                        if (tx, ty) == (x, y) && !own_tile {
                            continue;
                        }
                        best = best.min(self.costs[(row + tx - cx) as usize]);
                    }
                }
                best
            }
        };
        (best != UNREACHED).then_some(best)
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

/// Reads costs back from `costs_as_bytes`, as a save holds them (design §2.8).
fn costs_from_bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u32>, D::Error> {
    let bytes = crate::save::read_bytes(deserializer)?;
    let (costs, rest) = bytes.as_chunks::<4>();
    if !rest.is_empty() {
        return Err(serde::de::Error::custom("flood costs aren't whole numbers"));
    }
    Ok(costs.iter().map(|&cost| u32::from_le_bytes(cost)).collect())
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
    use crate::test_util::at;

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

    /// What each category offers sprite 1, the first on `sprites`, which the
    /// flood spreads from, with a radius of 10, by the category's name.
    fn candidates_of(
        rows: &[&str],
        objects: &[(Pos, &str)],
        sprites: &[Pos],
    ) -> BTreeMap<String, Vec<(Target, u32)>> {
        candidates_hit_by(rows, objects, sprites, None)
    }

    /// `candidates_of`, for a sprite 1 that feels a hit by `attacker`.
    fn candidates_hit_by(
        rows: &[&str],
        objects: &[(Pos, &str)],
        sprites: &[Pos],
        attacker: Option<EntityId>,
    ) -> BTreeMap<String, Vec<(Target, u32)>> {
        let (map, objects, sprites_placed, data) = parts_with(rows, objects, sprites);
        let ground = Ground {
            map: &map,
            objects: &objects,
            sprites: &sprites_placed,
            data: &data,
        };
        let flood = Flood::new(ground, sprites[0], 10, Occupied::Penalty(30), 0);
        flood
            .candidates(ground, EntityId(1), attacker, None)
            .into_iter()
            .map(|(id, found)| (data.category(id).expect("a category").name.clone(), found))
            .collect()
    }

    #[test]
    fn the_candidate_of_each_object_type_is_the_nearest_one_the_sprite_can_reach() {
        let berries = [(at(3, 0), "berry"), (at(6, 0), "berry")];
        let found = candidates_of(&["........"], &berries, &[at(0, 0)]);
        // Berries are items: a sprite can take one from its tile or beside it.
        assert_eq!(found["fruit"], [(Target::Object(EntityId(101)), 20)]);
    }

    #[test]
    fn candidates_the_same_distance_away_go_to_the_lower_id() {
        let berries = [(at(4, 0), "berry"), (at(0, 0), "berry")];
        let found = candidates_of(&["....."], &berries, &[at(2, 0)]);
        assert_eq!(found["fruit"], [(Target::Object(EntityId(101)), 10)]);
    }

    #[test]
    fn water_is_told_apart_by_tile_index_and_can_be_drunk_from_its_own_tile() {
        let found = candidates_of(&["~.~", "..."], &[], &[at(1, 1)]);
        assert_eq!(found["water"], [(Target::Water(at(0, 0)), 0)]);
        let found = candidates_of(&["..~"], &[], &[at(0, 0)]);
        assert_eq!(found["water"], [(Target::Water(at(2, 0)), 10)]);
    }

    #[test]
    fn a_bush_or_a_sprite_is_reached_from_beside_it() {
        let found = candidates_of(
            &["......"],
            &[(at(5, 0), "thornbush")],
            &[at(0, 0), at(3, 0)],
        );
        assert_eq!(found["bush"], [(Target::Object(EntityId(101)), 40 + 30)]);
        assert_eq!(found["sprite"], [(Target::Sprite(EntityId(2)), 20)]);
    }

    #[test]
    fn a_nearer_thing_the_sprite_cannot_reach_is_never_its_candidate() {
        // The near berry is walled in; the far one is in the open.
        let rows = ["...#.#", "...###", "......"];
        let berries = [(at(4, 0), "berry"), (at(0, 2), "berry")];
        let found = candidates_of(&rows, &berries, &[at(2, 0)]);
        // Taken from beside it, one diagonal step away.
        assert_eq!(found["fruit"], [(Target::Object(EntityId(102)), 14)]);
    }

    #[test]
    fn a_hit_sprite_s_sprite_candidate_is_its_attacker_if_it_can_reach_it() {
        let sprites = [at(0, 0), at(1, 0), at(4, 0)];
        let found = candidates_hit_by(&["....."], &[], &sprites, Some(EntityId(3)));
        assert_eq!(found["sprite"][0].0, Target::Sprite(EntityId(3)));
    }

    #[test]
    fn every_other_sprite_in_reach_is_offered_in_id_order() {
        // Design v18 §3.6: each is weighed on its own. Sprite 4 is walled
        // off, across rock.
        let sprites = [at(2, 0), at(4, 0), at(0, 0), at(6, 0)];
        let found = candidates_of(&[".....#."], &[], &sprites);
        let offered = [
            (Target::Sprite(EntityId(2)), 10),
            (Target::Sprite(EntityId(3)), 10),
        ];
        assert_eq!(found["sprite"], offered);
    }

    #[test]
    fn an_attacker_out_of_reach_leaves_every_sprite_in_reach_offered() {
        // The attacker is walled off, across rock.
        let sprites = [at(0, 0), at(1, 0), at(4, 0)];
        let found = candidates_hit_by(&["...#."], &[], &sprites, Some(EntityId(3)));
        assert_eq!(found["sprite"], [(Target::Sprite(EntityId(2)), 0)]);
    }

    #[test]
    fn a_sprite_is_never_its_own_candidate() {
        let found = candidates_of(&["..."], &[], &[at(0, 0)]);
        assert!(!found.contains_key("sprite"));
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

    /// The flood of design §3.6 as first built, before it was made faster: the
    /// search over the map itself, with a binary heap. The faster flood must
    /// reach the same costs by the same steps, ties and all, or worlds would
    /// change (design §2.3). Returns the costs and steps over the square.
    fn reference_flood(
        ground: Ground,
        origin: Pos,
        radius: u16,
        occupied: Occupied,
    ) -> (Vec<u32>, Vec<u8>) {
        use crate::physics::beside;
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let map = ground.map;
        let x0 = origin.x.saturating_sub(radius);
        let y0 = origin.y.saturating_sub(radius);
        let x1 = origin.x.saturating_add(radius).min(map.width() - 1);
        let y1 = origin.y.saturating_add(radius).min(map.height() - 1);
        let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
        let tiles = usize::from(width) * usize::from(height);
        let local = |pos: Pos| {
            let x = pos.x.checked_sub(x0)?;
            let y = pos.y.checked_sub(y0)?;
            (x < width && y < height).then(|| usize::from(y) * usize::from(width) + usize::from(x))
        };
        let pos_of = |index: usize| Pos {
            x: x0 + (index % usize::from(width)) as u16,
            y: y0 + (index / usize::from(width)) as u16,
        };
        let mut costs = vec![UNREACHED; tiles];
        let mut steps = vec![NO_STEP; tiles];
        let entry: Vec<Option<u32>> = (0..tiles)
            .map(|index| entry_cost(map, ground.objects, ground.data, pos_of(index)))
            .collect();
        let start = local(origin).expect("the origin is in its own square");
        costs[start] = 0;
        let mut queue = BinaryHeap::from([Reverse((0, start))]);
        while let Some(Reverse((cost, index))) = queue.pop() {
            if cost > costs[index] {
                continue;
            }
            let from = pos_of(index);
            for (d, &dir) in Dir::ALL.iter().enumerate() {
                let Some(to) = map.neighbour(from, dir) else {
                    continue;
                };
                let Some(next) = local(to) else {
                    continue;
                };
                let open = |pos| local(pos).is_some_and(|i| entry[i].is_some());
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
                if total < costs[next] {
                    costs[next] = total;
                    steps[next] = d as u8;
                    queue.push(Reverse((total, next)));
                }
            }
        }
        (costs, steps)
    }

    /// What each category offered, as slice 9c's code worked it out before
    /// it was made faster (design §3.6): a scan of every tile near the
    /// square for objects, water and sprites, then step 5 offering every
    /// sprite in reach unless a reachable attacker stands for them.
    fn reference_candidates(
        flood: &Flood,
        ground: Ground,
        me: EntityId,
        attacker: Option<EntityId>,
        cursor: Option<Pos>,
    ) -> BTreeMap<CategoryId, Vec<(Target, u32)>> {
        let map = ground.map;
        let goal_cost = |pos: Pos, own_tile: bool| {
            goal_tiles(map, pos, own_tile)
                .filter_map(|goal| flood.cost(goal))
                .min()
        };
        let (top_left, bottom_right) = flood.near(map);
        let tiles: Vec<Pos> = (top_left.y..=bottom_right.y)
            .flat_map(|y| (top_left.x..=bottom_right.x).map(move |x| Pos { x, y }))
            .collect();
        let mut best: BTreeMap<(CategoryId, Option<usize>), (u32, u64, Target)> = BTreeMap::new();
        let mut offer = |key, target, id, cost| {
            let better = best.get(&key).is_none_or(|&(c, i, _)| (cost, id) < (c, i));
            if better {
                best.insert(key, (cost, id, target));
            }
        };
        for &pos in &tiles {
            if let Some(id) = ground.objects.at(pos) {
                let index = ground.objects.kind(id);
                let object_type = &ground.data.object_types()[index];
                if let Some(cost) = goal_cost(pos, !object_type.solid) {
                    offer(
                        (object_type.category, Some(index)),
                        Target::Object(id),
                        id.0,
                        cost,
                    );
                }
            }
            if ground.data.terrain(map.terrain(pos)).is_drinkable()
                && let Some(cost) = goal_cost(pos, true)
            {
                let key = (ground.data.water_category(), None);
                offer(key, Target::Water(pos), map.index(pos) as u64, cost);
            }
        }
        let mut found: BTreeMap<CategoryId, Vec<(u64, Target, u32)>> = BTreeMap::new();
        for ((category, _), (cost, id, target)) in best {
            found.entry(category).or_default().push((id, target, cost));
        }
        let mut found: BTreeMap<CategoryId, Vec<(Target, u32)>> = found
            .into_iter()
            .map(|(category, mut things)| {
                things.sort_by_key(|&(id, ..)| id);
                (
                    category,
                    things.into_iter().map(|(_, t, c)| (t, c)).collect(),
                )
            })
            .collect();
        let attacker = attacker.filter(|&id| id != me).and_then(|id| {
            let pos = ground.sprites.get(id)?.pos;
            Some((Target::Sprite(id), goal_cost(pos, false)?))
        });
        let mut sprites: Vec<(EntityId, u32)> = tiles
            .iter()
            .filter_map(|&pos| {
                let id = ground.sprites.at(pos).filter(|&id| id != me)?;
                Some((id, goal_cost(pos, false)?))
            })
            .collect();
        sprites.sort_by_key(|&(id, _)| id);
        let sprites: Vec<(Target, u32)> = match attacker {
            Some(attacker) => vec![attacker],
            None => sprites
                .into_iter()
                .map(|(id, c)| (Target::Sprite(id), c))
                .collect(),
        };
        if !sprites.is_empty() {
            found.insert(ground.data.sprite_category(), sprites);
        }
        if let Some(cost) = cursor.and_then(|tile| goal_cost(tile, true)) {
            found.insert(ground.data.cursor_category(), vec![(Target::Cursor, cost)]);
        }
        found
    }

    mod same_as_before {
        use proptest::prelude::*;

        use super::*;

        /// A map's rows, objects as `(tile, solid)`, sprites' tiles, and an origin.
        type Scene = (Vec<String>, Vec<(Pos, bool)>, Vec<Pos>, Pos);

        /// A map up to 14 tiles a side, of every terrain, with some solid
        /// and some walkable objects, some sprites, and an origin.
        fn scene() -> impl Strategy<Value = Scene> {
            (1u16..=14, 1u16..=14).prop_flat_map(|(width, height)| {
                let tile = prop::sample::select(vec!['.', '.', '.', ',', ':', '~', '=', '#']);
                let row = prop::collection::vec(tile, usize::from(width))
                    .prop_map(|tiles| tiles.into_iter().collect::<String>());
                let pos = (0..width, 0..height).prop_map(|(x, y)| Pos { x, y });
                (
                    prop::collection::vec(row, usize::from(height)),
                    prop::collection::vec((pos.clone(), any::<bool>()), 0..12),
                    prop::collection::vec(pos.clone(), 0..10),
                    pos,
                )
            })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn the_flood_and_its_goal_costs_are_what_they_were(
                (rows, objects, sprites, origin) in scene(),
                radius in 0u16..9,
                penalty in prop::option::of(0u32..60),
                attacker in prop::option::of(1u64..12),
                cursor in prop::option::of((0u16..14, 0u16..14)),
            ) {
                let cursor = cursor.map(|(x, y)| Pos { x, y });
                let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
                // One object and one sprite to a tile, and none on the origin.
                let mut taken = BTreeSet::from([origin]);
                let objects: Vec<(Pos, &str)> = objects
                    .into_iter()
                    .filter(|&(pos, _)| taken.insert(pos))
                    .map(|(pos, solid)| (pos, if solid { "thornbush" } else { "berry" }))
                    .collect();
                let mut on = BTreeSet::from([origin]);
                let sprites: Vec<Pos> = std::iter::once(origin)
                    .chain(sprites.into_iter().filter(|&pos| on.insert(pos)))
                    .collect();
                let (map, objects, sprites, data) = parts_with(&rows, &objects, &sprites);
                let ground = Ground {
                    map: &map,
                    objects: &objects,
                    sprites: &sprites,
                    data: &data,
                };
                let occupied = penalty.map_or(Occupied::Closed, Occupied::Penalty);
                let flood = Flood::new(ground, origin, radius, occupied, 0);
                let (costs, steps) = reference_flood(ground, origin, radius, occupied);
                prop_assert_eq!(&flood.costs, &costs);
                prop_assert_eq!(&flood.steps, &steps);
                // What each category offers sprite 1, on the origin, hit by
                // `attacker` and seeing the Cursor on `cursor`, if any.
                let attacker = attacker.map(EntityId);
                let cursor = cursor.filter(|&tile| map.contains(tile));
                let me = EntityId(1);
                prop_assert_eq!(
                    flood.candidates(ground, me, attacker, cursor),
                    reference_candidates(&flood, ground, me, attacker, cursor)
                );
                for y in 0..map.height() {
                    for x in 0..map.width() {
                        let pos = Pos { x, y };
                        for own_tile in [false, true] {
                            let before = goal_tiles(&map, pos, own_tile)
                                .filter_map(|goal| flood.cost(goal))
                                .min();
                            prop_assert_eq!(flood.goal_cost(pos, own_tile), before);
                        }
                    }
                }
            }
        }
    }
}
