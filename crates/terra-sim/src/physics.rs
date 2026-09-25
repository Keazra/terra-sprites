//! Physics (design §3.1, §3.3): the fixed rules of how things move and
//! block, for any walker. Sprites are left to the caller, since whether a
//! sprite blocks depends on who's asking.

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::objects::Objects;

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
    let cost = map.step_cost(from, dir)?;
    let to = map.neighbour(from, dir)?;
    if objects.is_solid_at(data, to) {
        return None;
    }
    // No corner-cutting past a solid object either.
    let beside = [Pos { x: to.x, y: from.y }, Pos { x: from.x, y: to.y }];
    if beside.iter().any(|&pos| objects.is_solid_at(data, pos)) {
        return None;
    }
    Some(cost)
}
