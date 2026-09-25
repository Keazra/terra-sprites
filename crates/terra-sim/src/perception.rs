//! Perception (design §3.6): each sprite's bounded Dijkstra flood, which
//! gives both what it can reach and the way there.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::Serialize;

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::objects::Objects;
use crate::physics::step_cost;
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
    /// The top-left tile of the square it covers.
    corner: Pos,
    width: u16,
    height: u16,
    /// The cost to reach each tile of the square, row by row, in terrain units.
    costs: Vec<u32>,
    /// The direction of the step that reached each tile, as its place in `Dir::ALL`.
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
            corner: Pos { x: x0, y: y0 },
            width,
            height,
            costs: vec![UNREACHED; tiles],
            steps: vec![NO_STEP; tiles],
        };
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
                let Some(step) = step_cost(map, ground.objects, ground.data, from, dir) else {
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

/// The tile one step back from `pos` against direction `dir`.
fn back(pos: Pos, dir: Dir) -> Pos {
    let (dx, dy) = dir.offset();
    Pos {
        x: (i32::from(pos.x) - dx) as u16,
        y: (i32::from(pos.y) - dy) as u16,
    }
}
