//! Physics (design §3.1, §3.3): the fixed rules of how things move and
//! block, for any walker. Sprites are left to the caller, since whether a
//! sprite blocks depends on who's asking.

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::objects::Objects;

/// What a step onto the tile at `pos`, which must be on the map, costs, in
/// terrain units, or `None` if nothing may step there: unwalkable terrain, or
/// a solid object.
pub(crate) fn entry_cost(map: &Map, objects: &Objects, data: &DataPack, pos: Pos) -> Option<u32> {
    let cost = map.cost_onto(pos)?;
    (!objects.is_solid_at(data, pos)).then_some(u32::from(cost))
}

/// What a step in direction `dir` costs, onto a tile whose entry cost is
/// `entry`: a diagonal step costs 14 tenths as much, and is allowed only if
/// `beside_open` says both tiles beside it may be stepped on. No corner is
/// ever cut, past unwalkable terrain or a solid object.
pub(crate) fn step(
    entry: Option<u32>,
    dir: Dir,
    beside_open: impl FnOnce() -> bool,
) -> Option<u32> {
    let cost = entry?;
    if !dir.is_diagonal() {
        return Some(cost);
    }
    beside_open().then_some(cost * 14 / 10)
}

/// The two tiles beside a step from `from` to `to`: for a diagonal step,
/// the corners it passes.
pub(crate) fn beside(from: Pos, to: Pos) -> [Pos; 2] {
    [Pos { x: to.x, y: from.y }, Pos { x: from.x, y: to.y }]
}

/// What a step from `from` in direction `dir` costs, in terrain units, or
/// `None` if physics forbids it: past the wall, onto unwalkable terrain or a
/// solid object, or cutting a corner past either.
pub(crate) fn step_cost(
    map: &Map,
    objects: &Objects,
    data: &DataPack,
    from: Pos,
    dir: Dir,
) -> Option<u32> {
    let to = map.neighbour(from, dir)?;
    let open = |pos| entry_cost(map, objects, data, pos).is_some();
    step(entry_cost(map, objects, data, to), dir, || {
        beside(from, to).into_iter().all(open)
    })
}
