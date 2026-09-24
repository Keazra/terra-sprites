//! The world's objects and where they stand (design §3.3–3.4).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};

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
}

/// Every object in the world, and the tile each stands on. A tile holds at most
/// one object (design §3.4).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Objects {
    /// In ascending ID order, the order every per-object pass takes (design §2.3).
    by_id: BTreeMap<EntityId, Object>,
    /// The object on each tile, by tile index. It's derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    on_tile: Vec<Option<EntityId>>,
    /// The map's width, to find a tile's index.
    #[serde(skip)]
    width: u16,
}

impl Objects {
    /// No objects, on `map`.
    pub(crate) fn new(map: &Map) -> Objects {
        Objects {
            by_id: BTreeMap::new(),
            on_tile: vec![None; map.tile_count()],
            width: map.width(),
        }
    }

    /// The object on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.on_tile[self.index(pos)]
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

    /// Whether an object of type `kind` may go on the tile at `pos` (design §3.3–3.4).
    /// An item needs a walkable tile holding no object. A solid object also needs
    /// terrain that allows fixtures, and 8 walkable neighbours holding no solid
    /// object: the ring that keeps the map connected.
    pub(crate) fn can_place(&self, map: &Map, data: &DataPack, kind: usize, pos: Pos) -> bool {
        if !map.is_walkable(pos) || self.at(pos).is_some() {
            return false;
        }
        !data.object_types()[kind].solid || self.has_clear_ring(map, data, pos)
    }

    /// Puts a new object on its tile. The caller has checked `can_place`.
    pub(crate) fn place(&mut self, id: EntityId, object: Object) {
        let index = self.index(object.pos);
        debug_assert!(
            self.on_tile[index].is_none(),
            "{:?} already holds an object",
            object.pos
        );
        self.on_tile[index] = Some(id);
        self.by_id.insert(id, object);
    }

    /// Takes the object `id`, which must exist, out of the world.
    pub(crate) fn remove(&mut self, id: EntityId) -> Object {
        let object = self
            .by_id
            .remove(&id)
            .expect("removing an object that exists");
        let index = self.index(object.pos);
        self.on_tile[index] = None;
        object
    }

    /// Checks that every object stands where the rules allow, alone on its
    /// tile, with its stage and counters in range; describes the first problem.
    pub(crate) fn check(&self, map: &Map, data: &DataPack) -> Result<(), String> {
        let indexed = self.on_tile.iter().flatten().count();
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
            } else if object_type.solid && !self.has_clear_ring(map, data, object.pos) {
                "is solid but lacks its clear ring"
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

    /// Whether a solid object on `pos` meets the ring rule: terrain that allows
    /// fixtures, and 8 walkable neighbours holding no solid object.
    fn has_clear_ring(&self, map: &Map, data: &DataPack, pos: Pos) -> bool {
        data.terrain(map.terrain(pos)).allows_fixtures()
            && Dir::ALL.iter().all(|&dir| {
                map.neighbour(pos, dir)
                    .is_some_and(|n| map.is_walkable(n) && !self.is_solid_at(data, n))
            })
    }

    fn is_solid_at(&self, data: &DataPack, pos: Pos) -> bool {
        self.at(pos)
            .is_some_and(|id| data.object_types()[self.kind(id)].solid)
    }

    /// The tile index of `pos`: `y × width + x`.
    fn index(&self, pos: Pos) -> usize {
        usize::from(pos.y) * usize::from(self.width) + usize::from(pos.x)
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
            };
            self.objects.place(id, object);
            id
        }

        /// Whether every walkable tile free of solid objects can reach every
        /// other by legal steps: no step onto a solid object, and no diagonal
        /// past one (design §3.1).
        fn open_tiles_connected(&self) -> bool {
            let open = |pos: Pos| {
                self.map.is_walkable(pos)
                    && !self
                        .objects
                        .at(pos)
                        .is_some_and(|id| self.data.object_types()[self.objects.kind(id)].solid)
            };
            let all: Vec<Pos> = self.map.positions().filter(|&pos| open(pos)).collect();
            let Some(&start) = all.first() else {
                return true;
            };
            let mut seen = vec![false; self.map.tile_count()];
            seen[self.map.index(start)] = true;
            let mut stack = vec![start];
            let mut reached = 1;
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
                        reached += 1;
                        stack.push(to);
                    }
                }
            }
            reached == all.len()
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
    fn a_solid_object_needs_eight_walkable_neighbours_free_of_solid_objects() {
        let mut scene = Scene::new(&[
            ".......", //
            ".......", //
            ".......", //
            "....#..", //
            ".......", //
        ]);
        assert!(!scene.can_place("berry_bush", at(0, 2)), "beside the wall");
        assert!(!scene.can_place("berry_bush", at(3, 2)), "beside rock");

        scene.place("berry_bush", at(1, 1));
        assert!(!scene.can_place("thornbush", at(2, 2)), "beside a bush");
        assert!(!scene.can_place("berry", at(1, 1)), "the tile holds a bush");

        scene.place("berry", at(2, 1));
        assert!(scene.can_place("thornbush", at(3, 1)), "beside a berry");
        assert!(
            !scene.can_place("thornbush", at(2, 1)),
            "the tile holds a berry"
        );
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
        fn placing_and_removing_solid_objects_never_disconnects_the_map(
            ops in vec((0u16..14, 0u16..9, any::<bool>(), any::<prop::sample::Index>()), 0..120)
        ) {
            let mut scene = Scene::new(&WARREN);
            let mut placed: Vec<EntityId> = Vec::new();
            for (x, y, add, which) in ops {
                let kind = if (x + y) % 3 == 0 { "thornbush" } else { "berry_bush" };
                if add || placed.is_empty() {
                    if scene.can_place(kind, at(x, y)) {
                        placed.push(scene.place(kind, at(x, y)));
                    }
                } else {
                    let id = placed.swap_remove(which.index(placed.len()));
                    scene.objects.remove(id);
                }
                prop_assert!(scene.open_tiles_connected());
            }
        }
    }
}
