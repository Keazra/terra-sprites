//! The world's objects and where they stand (design §3.3–3.4).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::occupancy::Occupancy;

/// An entity's ID. IDs come from a world counter that only goes up, and are
/// never reused (design §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct EntityId(pub u64);

/// One object in the world.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Object {
    /// Its type's index in the data pack.
    pub(crate) kind: usize,
    pub(crate) pos: Pos,
    /// The current stage's index, or `None` if the type has no stages.
    pub(crate) stage: Option<usize>,
    /// The tick on whose turn the current stage has run its full length.
    pub(crate) stage_ends: u64,
    /// Each counter's value, in the type's counter order.
    pub(crate) counters: Vec<u16>,
    /// It hasn't had its first turn yet, on which it enters its first stage.
    pub(crate) fresh: bool,
    /// Its roll, while a push has it rolling (design §3.5.4).
    pub(crate) roll: Option<Roll>,
}

/// A rolling item's way on (design §3.5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Roll {
    pub(crate) dir: Dir,
    /// The tiles it has left to go.
    pub(crate) left: u16,
}

/// Every object in the world, and the tile each stands on. A tile holds at most
/// one object (design §3.4).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Objects {
    /// In ascending ID order, the order every per-object pass takes (design §2.3).
    by_id: BTreeMap<EntityId, Object>,
    /// The object on each tile. It's derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    on_tile: Occupancy,
}

impl Objects {
    /// No objects, on `map`.
    pub(crate) fn new(map: &Map) -> Objects {
        Objects {
            by_id: BTreeMap::new(),
            on_tile: Occupancy::new(map),
        }
    }

    /// The object on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.on_tile.at(pos)
    }

    /// The object `id`, if it exists.
    pub(crate) fn get(&self, id: EntityId) -> Option<&Object> {
        self.by_id.get(&id)
    }

    /// The object `id`, if it exists, to change.
    pub(crate) fn get_mut(&mut self, id: EntityId) -> Option<&mut Object> {
        self.by_id.get_mut(&id)
    }

    /// Every object, in ascending ID order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (EntityId, &Object)> {
        self.by_id.iter().map(|(&id, object)| (id, object))
    }

    /// The type index of the object `id`, which must exist.
    pub(crate) fn kind(&self, id: EntityId) -> usize {
        self.by_id[&id].kind
    }

    /// Whether an object of type `kind` may go on the tile at `pos` (design §3.3–3.4):
    /// a walkable tile on the map holding no object, whose terrain allows
    /// fixtures if the object is solid. Whether it would cut a path is for the
    /// rules to ask, with `keeps_paths_open`; whether a sprite is in the way is
    /// for `WorldState::can_place`.
    pub(crate) fn can_place(&self, map: &Map, data: &DataPack, kind: usize, pos: Pos) -> bool {
        map.contains(pos)
            && map.is_walkable(pos)
            && self.at(pos).is_none()
            && (!data.object_types()[kind].solid
                || data.terrain(map.terrain(pos)).allows_fixtures())
    }

    /// Whether a solid object on `pos` would leave the open tiles on its four
    /// sides joined to one another around it, through the 8 tiles that surround
    /// it (design §3.3). An open tile is walkable and holds no solid object.
    /// Where this holds, a solid object can't split the map: any path through
    /// the tile can go around it instead.
    pub(crate) fn keeps_paths_open(&self, map: &Map, data: &DataPack, pos: Pos) -> bool {
        // The surrounding tiles clockwise from N. Each is an orthogonal step
        // from the next, so a run of open ones is a path around the tile.
        let open: Vec<bool> = Dir::ALL
            .iter()
            .map(|&dir| {
                map.neighbour(pos, dir)
                    .is_some_and(|n| map.is_walkable(n) && !self.is_solid_at(data, n))
            })
            .collect();
        let Some(closed) = open.iter().position(|&is_open| !is_open) else {
            return true;
        };
        // Number the runs of open tiles, walking once around from a closed one.
        let mut run = vec![0; open.len()];
        let mut current = 0;
        for step in 1..=open.len() {
            let i = (closed + step) % open.len();
            if open[i] {
                run[i] = current;
            } else {
                current += 1;
            }
        }
        // N, E, S and W are the even positions; the open ones must share a run.
        let mut sides = (0..open.len())
            .step_by(2)
            .filter(|&i| open[i])
            .map(|i| run[i]);
        match sides.next() {
            Some(first) => sides.all(|r| r == first),
            None => true,
        }
    }

    /// Puts a new object on its tile. The caller has checked `can_place`.
    pub(crate) fn place(&mut self, id: EntityId, object: Object) {
        self.on_tile.put(object.pos, id);
        self.by_id.insert(id, object);
    }

    /// Moves the object `id`, which must exist, onto the tile at `to`, which
    /// must hold no object.
    pub(crate) fn move_to(&mut self, id: EntityId, to: Pos) {
        let object = self
            .by_id
            .get_mut(&id)
            .expect("moving an object that exists");
        self.on_tile.clear(object.pos);
        self.on_tile.put(to, id);
        object.pos = to;
    }

    /// Takes the object `id`, which must exist, out of the world.
    pub(crate) fn remove(&mut self, id: EntityId) -> Object {
        let object = self
            .by_id
            .remove(&id)
            .expect("removing an object that exists");
        self.on_tile.clear(object.pos);
        object
    }

    /// Checks that every object stands where the rules allow, alone on its
    /// tile, with its stage and counters in range; describes the first problem.
    pub(crate) fn check(&self, map: &Map, data: &DataPack) -> Result<(), String> {
        let indexed = self.on_tile.count();
        if indexed != self.by_id.len() {
            return Err(format!(
                "{indexed} tiles hold objects, but there are {} objects",
                self.by_id.len()
            ));
        }
        for (&id, object) in &self.by_id {
            let object_type = &data.object_types()[object.kind];
            let problem = if self.at(object.pos) != Some(id) {
                "isn't on its tile in the index"
            } else if object_type.pseudo {
                "is of a pseudo type"
            } else if !map.is_walkable(object.pos) {
                "stands on a tile that isn't walkable"
            } else if object_type.solid && !data.terrain(map.terrain(object.pos)).allows_fixtures()
            {
                "is solid on terrain that doesn't allow fixtures"
            } else if object.stage.is_some_and(|s| s >= object_type.stages.len()) {
                "is in a stage its type doesn't have"
            } else if object.counters.len() != object_type.counters.len()
                || object
                    .counters
                    .iter()
                    .zip(&object_type.counters)
                    .any(|(&value, counter)| value > counter.max)
            {
                "has a counter out of range"
            } else {
                continue;
            };
            return Err(format!(
                "{} {id:?} at {:?} {problem}",
                object_type.name, object.pos
            ));
        }
        Ok(())
    }

    /// Whether a solid object stands on the tile at `pos`.
    pub(crate) fn is_solid_at(&self, data: &DataPack, pos: Pos) -> bool {
        self.at(pos)
            .is_some_and(|id| data.object_types()[self.kind(id)].solid)
    }
}

#[cfg(test)]
mod tests {
    use proptest::collection::vec;
    use proptest::prelude::*;

    use super::*;
    use crate::data::DataPack;
    use crate::map::{Dir, Map, Pos};

    fn at(x: u16, y: u16) -> Pos {
        Pos { x, y }
    }

    /// A drawn map with a store of objects on it, and the built-in pack's types.
    struct Scene {
        data: DataPack,
        map: Map,
        objects: Objects,
        next_id: u64,
    }

    impl Scene {
        /// `rows` use the ascii legend: `.` grass, `~` shallow water, `=` deep water, `#` rock.
        fn new(rows: &[&str]) -> Scene {
            let data = DataPack::builtin().expect("built-in data pack is valid");
            let map = Map::from_ascii(rows, &data).expect("valid drawing");
            let objects = Objects::new(&map);
            Scene {
                data,
                map,
                objects,
                next_id: 1,
            }
        }

        fn kind(&self, name: &str) -> usize {
            self.data.object_type_named(name).expect("a built-in type")
        }

        fn can_place(&self, name: &str, pos: Pos) -> bool {
            self.objects
                .can_place(&self.map, &self.data, self.kind(name), pos)
        }

        /// Places an object called `name` at `pos`, which must be allowed.
        fn place(&mut self, name: &str, pos: Pos) -> EntityId {
            assert!(self.can_place(name, pos), "{name} at {pos:?}");
            let id = EntityId(self.next_id);
            self.next_id += 1;
            let object = Object {
                kind: self.kind(name),
                pos,
                stage: None,
                stage_ends: 0,
                counters: Vec::new(),
                fresh: false,
                roll: None,
            };
            self.objects.place(id, object);
            id
        }

        /// How many regions the open ground is in: walkable tiles free
        /// of solid objects, joined by legal steps (no step onto a solid
        /// object, and no diagonal past one, design §3.1).
        fn open_regions(&self) -> usize {
            let open = |pos: Pos| {
                self.map.is_walkable(pos)
                    && !self
                        .objects
                        .at(pos)
                        .is_some_and(|id| self.data.object_types()[self.objects.kind(id)].solid)
            };
            let mut seen = vec![false; self.map.tile_count()];
            let mut regions = 0;
            for start in self.map.positions().filter(|&pos| open(pos)) {
                if seen[self.map.index(start)] {
                    continue;
                }
                regions += 1;
                seen[self.map.index(start)] = true;
                let mut stack = vec![start];
                while let Some(pos) = stack.pop() {
                    for dir in Dir::ALL {
                        let Some(to) = self.map.neighbour(pos, dir) else {
                            continue;
                        };
                        let sides_open = [Pos { x: to.x, y: pos.y }, Pos { x: pos.x, y: to.y }]
                            .into_iter()
                            .all(open);
                        if open(to) && sides_open && !seen[self.map.index(to)] {
                            seen[self.map.index(to)] = true;
                            stack.push(to);
                        }
                    }
                }
            }
            regions
        }
    }

    #[test]
    fn an_item_goes_on_any_walkable_tile_that_holds_no_object() {
        let mut scene = Scene::new(&[
            ".~=#", //
            "....", //
        ]);
        assert!(scene.can_place("berry", at(0, 0)), "grass");
        assert!(scene.can_place("berry", at(1, 0)), "shallow water");
        assert!(!scene.can_place("berry", at(2, 0)), "deep water");
        assert!(!scene.can_place("berry", at(3, 0)), "rock");

        scene.place("berry", at(0, 1));
        assert!(!scene.can_place("ball", at(0, 1)), "the tile holds a berry");
    }

    #[test]
    fn solid_objects_may_stand_side_by_side_and_beside_rock_or_the_wall() {
        let mut scene = Scene::new(&[
            ".......", //
            ".......", //
            ".......", //
            "....#..", //
            ".......", //
        ]);
        assert!(scene.can_place("berry_bush", at(0, 2)), "beside the wall");
        assert!(scene.can_place("berry_bush", at(3, 2)), "beside rock");

        scene.place("berry_bush", at(1, 1));
        assert!(scene.can_place("thornbush", at(2, 2)), "beside a bush");
        assert!(!scene.can_place("berry", at(1, 1)), "the tile holds a bush");

        scene.place("berry", at(2, 1));
        assert!(
            !scene.can_place("thornbush", at(2, 1)),
            "the tile holds a berry"
        );
    }

    fn keeps_paths_open(scene: &Scene, pos: Pos) -> bool {
        scene.objects.keeps_paths_open(&scene.map, &scene.data, pos)
    }

    #[test]
    fn keeps_paths_open_holds_beside_other_solid_objects() {
        let mut scene = Scene::new(&["......"; 5]);
        assert!(keeps_paths_open(&scene, at(2, 2)), "in open ground");
        scene.place("berry_bush", at(1, 1));
        assert!(keeps_paths_open(&scene, at(2, 1)), "beside a bush");
        scene.place("berry_bush", at(2, 1));
        scene.place("berry_bush", at(1, 2));
        assert!(keeps_paths_open(&scene, at(2, 2)), "completing a 2x2 clump");
        assert!(keeps_paths_open(&scene, at(3, 1)), "beside the clump");
    }

    #[test]
    fn keeps_paths_open_judges_only_the_tiles_around_so_it_errs_on_the_safe_side() {
        // A bush in the corner at (0, 0) would split its two open sides around
        // it, though they'd still meet the long way round the clump.
        let mut scene = Scene::new(&["......"; 5]);
        for (x, y) in [(1, 1), (2, 1), (1, 2)] {
            scene.place("berry_bush", at(x, y));
        }
        assert!(!keeps_paths_open(&scene, at(0, 0)));
    }

    #[test]
    fn keeps_paths_open_fails_where_a_solid_object_would_plug_a_corridor() {
        let scene = Scene::new(&[
            "#####", //
            ".....", //
            "#####", //
        ]);
        assert!(!keeps_paths_open(&scene, at(2, 1)));
        assert!(
            keeps_paths_open(&scene, at(0, 1)),
            "the dead end at the wall"
        );
    }

    #[test]
    fn keeps_paths_open_fails_for_the_piece_that_would_close_a_wall() {
        // A diagonal line of bushes is a wall, since sprites can't cut corners.
        let mut scene = Scene::new(&["....."; 5]);
        for i in 0..4 {
            scene.place("berry_bush", at(i, i));
        }
        assert!(
            !keeps_paths_open(&scene, at(4, 4)),
            "carrying the line to the wall"
        );
        assert!(
            !keeps_paths_open(&scene, at(4, 3)),
            "touching the wall beside the line's end"
        );

        let mut scene = Scene::new(&["......."; 7]);
        for i in 1..4 {
            scene.place("berry_bush", at(i, i));
        }
        assert!(
            keeps_paths_open(&scene, at(4, 3)),
            "beside the line's end, in the open"
        );

        // A ring of bushes around (2, 2), missing only its last piece.
        let mut scene = Scene::new(&["....."; 5]);
        for (x, y) in [(1, 1), (2, 1), (3, 1), (3, 2), (3, 3), (2, 3), (1, 3)] {
            scene.place("berry_bush", at(x, y));
        }
        assert!(!keeps_paths_open(&scene, at(1, 2)), "closing the ring");
    }

    #[test]
    fn a_solid_object_needs_terrain_that_allows_fixtures() {
        let pond = Scene::new(&[
            "~~~", //
            "~~~", //
            "~~~", //
        ]);
        assert!(!pond.can_place("berry_bush", at(1, 1)), "in shallow water");
        let shore = Scene::new(&[
            "~~~", //
            "~.~", //
            "~~~", //
        ]);
        assert!(
            shore.can_place("berry_bush", at(1, 1)),
            "on grass by the water"
        );
    }

    #[test]
    fn nothing_goes_off_the_map() {
        let scene = Scene::new(&["...", "...", "..."]);
        // x past the right wall would otherwise land on the next row's first tile.
        assert!(!scene.can_place("berry", at(3, 1)));
        assert!(!scene.can_place("berry", at(0, 3)));
    }

    #[test]
    fn removing_an_object_frees_its_tile() {
        let mut scene = Scene::new(&[
            "...", //
            "...", //
            "...", //
        ]);
        let bush = scene.place("berry_bush", at(1, 1));
        scene.objects.remove(bush);
        assert_eq!(scene.objects.at(at(1, 1)), None);
        assert!(scene.can_place("berry", at(1, 1)));
    }

    /// A map with rooms, one-tile corridors and a pond, so a careless placement could cut it.
    const WARREN: [&str; 9] = [
        "......#.......",
        "......#.......",
        "..............",
        "######.#####.#",
        ".....#.#......",
        ".~~..#...#....",
        ".~~......#....",
        ".....#####.###",
        "..............",
    ];

    proptest! {
        #[test]
        fn placing_solid_objects_where_paths_stay_open_never_splits_the_open_ground(
            ops in vec((0u16..14, 0u16..9, any::<bool>(), any::<prop::sample::Index>()), 0..120)
        ) {
            let mut scene = Scene::new(&WARREN);
            let mut placed: Vec<EntityId> = Vec::new();
            for (x, y, add, which) in ops {
                let kind = if (x + y) % 3 == 0 { "thornbush" } else { "berry_bush" };
                if add || placed.is_empty() {
                    if scene.can_place(kind, at(x, y)) && keeps_paths_open(&scene, at(x, y)) {
                        let before = scene.open_regions();
                        placed.push(scene.place(kind, at(x, y)));
                        prop_assert!(scene.open_regions() <= before, "placed at ({x}, {y})");
                    }
                } else {
                    // A removal can leave a pocket where a filled dead end was,
                    // but it held nothing but the object, so nothing is trapped.
                    let id = placed.swap_remove(which.index(placed.len()));
                    scene.objects.remove(id);
                }
            }
        }
    }
}
