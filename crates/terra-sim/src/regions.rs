//! Regions: the largest sets of walkable tiles that can all reach one another by
//! legal steps (design §3.2).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::map::{Dir, Map};
use crate::terrain::Terrain;

/// Makes the map one region (design §3.2). The largest region is the mainland.
/// Every other region of at least `min_region` tiles is joined to it by carving;
/// smaller ones become rock.
pub(crate) fn connect(map: &mut Map, min_region: usize) {
    let regions = Regions::find(map);
    let Some(mainland) = regions.largest() else {
        return;
    };
    let members = &regions.members;

    let mut in_mainland = vec![false; map.tile_count()];
    for &index in &members[mainland] {
        in_mainland[index] = true;
    }
    for (region, tiles) in members.iter().enumerate() {
        if region != mainland && tiles.len() < min_region {
            for &index in tiles {
                map.set(map.pos(index), Terrain::Rock);
            }
        }
    }

    // Join the rest, largest first; ties go to the region found first.
    let mut to_join: Vec<usize> = (0..regions.count())
        .filter(|&region| region != mainland && members[region].len() >= min_region)
        .collect();
    to_join.sort_by_key(|&region| (Reverse(members[region].len()), region));
    for region in to_join {
        // An earlier route may already have crossed this region.
        if in_mainland[members[region][0]] {
            continue;
        }
        let route = cheapest_route(map, &members[region], &in_mainland);
        for index in route {
            let pos = map.pos(index);
            if !map.is_walkable(pos) {
                map.set(pos, carved(map.terrain(pos)));
            }
            // Crossing a region that is still land joins all of it; a small
            // region that became rock only gains the tiles actually carved.
            let crossed = regions.of_tile[index]
                .map(|r| &members[r])
                .filter(|tiles| tiles.len() >= min_region)
                .map_or(&[][..], Vec::as_slice);
            for &tile in crossed.iter().chain([&index]) {
                in_mainland[tile] = true;
            }
        }
        for &index in &members[region] {
            in_mainland[index] = true;
        }
    }
}

/// What an unwalkable terrain becomes when a route is carved through it.
fn carved(terrain: Terrain) -> Terrain {
    match terrain {
        Terrain::DeepWater => Terrain::ShallowWater,
        _ => Terrain::Dirt,
    }
}

/// The tiles between `from` and the nearest mainland tile, by orthogonal steps:
/// the route with the fewest unwalkable tiles to carve, then the fewest steps.
/// Ties go to the lowest tile index, so the route is deterministic.
fn cheapest_route(map: &Map, from: &[usize], in_mainland: &[bool]) -> Vec<usize> {
    const UNSEEN: usize = usize::MAX;
    let mut best: Vec<Option<(u32, u32)>> = vec![None; map.tile_count()];
    let mut came_from = vec![UNSEEN; map.tile_count()];
    let mut queue = BinaryHeap::new();
    for &index in from {
        best[index] = Some((0, 0));
        queue.push(Reverse((0, 0, index)));
    }
    while let Some(Reverse((carves, steps, index))) = queue.pop() {
        if best[index] != Some((carves, steps)) {
            continue; // superseded by a cheaper visit
        }
        if in_mainland[index] {
            let mut route = Vec::new();
            let mut at = came_from[index];
            while at != UNSEEN && best[at] != Some((0, 0)) {
                route.push(at);
                at = came_from[at];
            }
            return route;
        }
        let pos = map.pos(index);
        for dir in [Dir::N, Dir::E, Dir::S, Dir::W] {
            let Some(next) = map.neighbour(pos, dir) else {
                continue;
            };
            let next_index = map.index(next);
            let cost = (carves + u32::from(!map.is_walkable(next)), steps + 1);
            if best[next_index].is_none_or(|known| cost < known) {
                best[next_index] = Some(cost);
                came_from[next_index] = index;
                queue.push(Reverse((cost.0, cost.1, next_index)));
            }
        }
    }
    unreachable!("every tile can be carved, so the mainland is always reachable")
}

/// A map's regions, found by flood fill under the movement rules.
pub(crate) struct Regions {
    /// Each tile's region, by tile index; `None` for unwalkable tiles.
    of_tile: Vec<Option<usize>>,
    /// Each region's tile indices, in ascending order. Regions are numbered in
    /// order of their first tile.
    members: Vec<Vec<usize>>,
}

impl Regions {
    pub(crate) fn find(map: &Map) -> Regions {
        let mut of_tile: Vec<Option<usize>> = vec![None; map.tile_count()];
        let mut sizes = Vec::new();
        let mut stack = Vec::new();
        for start in map.positions() {
            if !map.is_walkable(start) || of_tile[map.index(start)].is_some() {
                continue;
            }
            let region = sizes.len();
            let mut size = 0;
            of_tile[map.index(start)] = Some(region);
            stack.push(start);
            while let Some(pos) = stack.pop() {
                size += 1;
                for dir in Dir::ALL {
                    if map.step_cost(pos, dir).is_none() {
                        continue;
                    }
                    let next = map
                        .neighbour(pos, dir)
                        .expect("a legal step stays on the map");
                    let slot = &mut of_tile[map.index(next)];
                    if slot.is_none() {
                        *slot = Some(region);
                        stack.push(next);
                    }
                }
            }
            sizes.push(size);
        }
        let mut members = vec![Vec::new(); sizes.len()];
        for (index, region) in of_tile.iter().enumerate() {
            if let Some(region) = *region {
                members[region].push(index);
            }
        }
        Regions { of_tile, members }
    }

    /// The largest region: the mainland. Ties go to the region found first.
    fn largest(&self) -> Option<usize> {
        let mut largest: Option<usize> = None;
        for (region, tiles) in self.members.iter().enumerate() {
            if largest.is_none_or(|best| tiles.len() > self.members[best].len()) {
                largest = Some(region);
            }
        }
        largest
    }

    /// How many regions the map has.
    pub(crate) fn count(&self) -> usize {
        self.members.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::DataPack;

    fn draw(rows: &[&str]) -> Map {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        Map::from_ascii(rows, &data).expect("valid drawing")
    }

    /// Joins `drawing` with a minimum region size of `min_region` and returns the result.
    fn connected(drawing: &[&str], min_region: usize) -> Vec<String> {
        let mut map = draw(drawing);
        connect(&mut map, min_region);
        assert_eq!(Regions::find(&map).count(), 1, "{:#?}", map.to_ascii());
        map.to_ascii()
    }

    #[test]
    fn a_region_below_the_minimum_becomes_rock() {
        let drawing = [
            "....#.", //
            "....##", //
        ];
        let expected = [
            "....##", //
            "....##", //
        ];
        assert_eq!(connected(&drawing, 4), expected);
    }

    #[test]
    fn the_mainland_stays_even_when_it_is_below_the_minimum() {
        assert_eq!(connected(&["...#.:"], 64), ["...###"]);
    }

    #[test]
    fn a_region_at_the_minimum_is_joined_through_the_thinnest_rock_which_becomes_dirt() {
        let drawing = [
            "..##..", //
            "..#...", //
            "..##..", //
        ];
        let expected = [
            "..##..", //
            "..,...", //
            "..##..", //
        ];
        assert_eq!(connected(&drawing, 4), expected);
    }

    #[test]
    fn carved_deep_water_becomes_shallow_water() {
        let drawing = [
            "..==..", //
            "..=...", //
            "..==..", //
        ];
        let expected = [
            "..==..", //
            "..~...", //
            "..==..", //
        ];
        assert_eq!(connected(&drawing, 4), expected);
    }

    #[test]
    fn the_route_carves_as_few_tiles_as_possible_across_mixed_barriers() {
        // The top crossing is one rock and one deep water; the others are three tiles thick.
        let drawing = [
            "..#=..", //
            "..#=#.", //
            "..###.", //
        ];
        let expected = [
            "..,~..", //
            "..#=#.", //
            "..###.", //
        ];
        assert_eq!(connected(&drawing, 4), expected);
    }

    #[test]
    fn a_route_through_a_small_region_turned_to_rock_joins_only_what_it_carves() {
        // The top-right region's route carves through the two-tile region in the
        // middle column, which became rock. The bottom region must still be joined
        // all the way, not just to that region's other, still-rock tile.
        let drawing = [
            "....#.#....", //
            "#####.#####", //
            "###########", //
            "#####....##", //
        ];
        let result = connected(&drawing, 4);
        assert_eq!(result[0], "....,,,....");
    }

    #[test]
    fn between_routes_carving_the_same_number_of_tiles_the_shorter_wins() {
        // The left region can reach the mainland (bottom) by carving two tiles
        // straight down (3 steps), or by carving one tile into the right region,
        // crossing it, and carving one more (4+ steps). It must go straight down.
        let drawing = [
            "...#...", //
            "...#...", //
            "#######", //
            "####...", //
            ".......", //
        ];
        let result = connected(&drawing, 4);
        let carved = |rows: &[String], columns: std::ops::Range<usize>| -> usize {
            rows.iter()
                .map(|row| row[columns.clone()].matches(',').count())
                .sum()
        };
        // Two carves straight below the left region; then one more joins the right region.
        assert_eq!(carved(&result[2..4], 0..3), 2, "{result:#?}");
        assert_eq!(carved(&result, 0..7), 3, "{result:#?}");
    }

    #[test]
    fn regions_touching_only_at_a_corner_are_joined_by_one_orthogonal_carve() {
        let drawing = [
            "..###", //
            "..###", //
            "##...", //
            "##...", //
        ];
        let result = connected(&drawing, 4);
        let carved: Vec<(usize, usize)> = result
            .iter()
            .enumerate()
            .flat_map(|(y, row)| row.char_indices().map(move |(x, c)| (x, y, c)))
            .filter(|&(_, _, c)| c == ',')
            .map(|(x, y, _)| (x, y))
            .collect();
        assert!(
            carved == [(2, 1)] || carved == [(1, 2)],
            "expected one carve beside the corner, got {carved:?} in {result:#?}"
        );
    }
}
