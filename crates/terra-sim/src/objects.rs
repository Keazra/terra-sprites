//! The world's objects and where they stand (design §3.3–3.4).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::breeding::Unhatched;
use crate::data::DataPack;
use crate::map::{Dir, Map, Pos};
use crate::object_types::ObjectType;
use crate::occupancy::Occupancy;

/// An entity's ID. IDs come from a world counter that only goes up, and are
/// never reused (design §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// One object in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Whether the Cursor holds it, off the map (design v23 §6.5). Its
    /// `pos` is then where it was picked up, and means nothing.
    pub(crate) held: bool,
    /// The sprite it holds, if it's an egg (M2 design §10.4, #196), which
    /// hatches as its type's stage says. Every object of a type that
    /// hatches holds one, and no other object does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) unhatched: Option<Box<Unhatched>>,
}

/// A rolling item's way on (design §3.5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Roll {
    /// The way it's rolling.
    pub(crate) dir: Dir,
    /// The tiles it has left to go.
    pub(crate) left: u16,
}

impl Object {
    /// The soonest tick at or after `from` on which this object has something
    /// to do (M4 design §10): entering its first stage, finishing its current
    /// stage, or firing a periodic `Every(n)` rule; `None` if it has none.
    pub(crate) fn next_turn(
        &self,
        object_type: &ObjectType,
        id: EntityId,
        from: u64,
    ) -> Option<u64> {
        if self.fresh {
            return Some(from);
        }
        let stage_turn = self.stage.map(|_| self.stage_ends.max(from));
        let rule_turns = object_type
            .rules
            .iter()
            .filter_map(|rule| rule.trigger.next_firing(id, from));
        stage_turn.into_iter().chain(rule_turns).min()
    }
}

/// Every object in the world, and the tile each stands on. A tile holds at most
/// one object (design §3.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Objects {
    /// In ascending ID order, the order every per-object pass takes (design §2.3).
    by_id: BTreeMap<EntityId, Object>,
    /// The object on each tile. It's derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    on_tile: Occupancy,
    /// How many objects hold a sprite: the eggs (M2 design §10.4). It's
    /// derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    eggs: usize,
    /// The objects due for a turn on each tick, in ascending ID order (M4
    /// design §10). Derived from `by_id` and the world's tick, so it isn't
    /// hashed.
    #[serde(skip)]
    due_on: BTreeMap<u64, BTreeSet<EntityId>>,
    /// The tick each scheduled object is next due on. Derived from `by_id`
    /// and the world's tick, so it isn't hashed.
    #[serde(skip)]
    next_due: BTreeMap<EntityId, u64>,
}

impl Objects {
    /// No objects, on `map`.
    pub(crate) fn new(map: &Map) -> Objects {
        Objects {
            by_id: BTreeMap::new(),
            on_tile: Occupancy::new(map),
            eggs: 0,
            due_on: BTreeMap::new(),
            next_due: BTreeMap::new(),
        }
    }

    /// How many eggs there are, held or on the map (M2 design §10.4): the
    /// objects that hold a sprite.
    pub(crate) fn eggs(&self) -> usize {
        self.eggs
    }

    /// The object on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.on_tile.at(pos)
    }

    /// The objects on the tiles of row `y` from `x0` to `x1`, inclusive,
    /// which must be on the map.
    pub(crate) fn row(&self, y: u16, x0: u16, x1: u16) -> &[Option<EntityId>] {
        self.on_tile.row(y, x0, x1)
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

    /// Puts a new object on its tile, scheduled from `from`. The caller has
    /// checked `can_place`.
    pub(crate) fn place(&mut self, id: EntityId, object: Object, data: &DataPack, from: u64) {
        self.on_tile.put(object.pos, id);
        self.eggs += usize::from(object.unhatched.is_some());
        self.by_id.insert(id, object);
        self.schedule(id, data, from);
    }

    /// Schedules the object `id`, if it exists, for its next turn at or after
    /// `from` (M4 design §10), replacing any turn it had scheduled.
    pub(crate) fn schedule(&mut self, id: EntityId, data: &DataPack, from: u64) {
        self.unschedule(id);
        let Some(object) = self.by_id.get(&id) else {
            return;
        };
        let Some(object_type) = data.object_types().get(object.kind) else {
            return;
        };
        if let Some(due) = object.next_turn(object_type, id, from) {
            self.due_on.entry(due).or_default().insert(id);
            self.next_due.insert(id, due);
        }
    }

    /// Removes any scheduled turn for `id`.
    fn unschedule(&mut self, id: EntityId) {
        if let Some(was) = self.next_due.remove(&id)
            && let Some(set) = self.due_on.get_mut(&was)
        {
            set.remove(&id);
            if set.is_empty() {
                self.due_on.remove(&was);
            }
        }
    }

    /// Takes and returns the objects due for a turn on `tick`, in ascending
    /// ID order (M4 design §10).
    pub(crate) fn take_due(&mut self, tick: u64) -> Vec<EntityId> {
        let Some(due) = self.due_on.remove(&tick) else {
            return Vec::new();
        };
        for &id in &due {
            self.next_due.remove(&id);
        }
        due.into_iter().collect()
    }

    /// Rebuilds which object stands on each tile of `map` and when each is
    /// next due from `tick`, after a load: every object but a held one, which
    /// is off the map. Says what's wrong if one is of a type `data` doesn't
    /// have, or is off the map or shares a tile.
    pub(crate) fn rebuild(&mut self, map: &Map, data: &DataPack, tick: u64) -> Result<(), String> {
        self.on_tile = Occupancy::new(map);
        self.due_on.clear();
        self.next_due.clear();
        self.eggs = self
            .by_id
            .values()
            .filter(|o| o.unhatched.is_some())
            .count();
        let ids: Vec<EntityId> = self.by_id.keys().copied().collect();
        for id in ids {
            let object = &self.by_id[&id];
            if object.kind >= data.object_types().len() {
                return Err(format!(
                    "object {} is of a type its pack doesn't have",
                    id.0
                ));
            }
            if !object.held {
                if !map.contains(object.pos) || self.on_tile.at(object.pos).is_some() {
                    return Err(format!("object {} has no tile of its own", id.0));
                }
                self.on_tile.put(object.pos, id);
            }
            self.schedule(id, data, tick);
        }
        Ok(())
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
        self.unschedule(id);
        let object = self
            .by_id
            .remove(&id)
            .expect("removing an object that exists");
        if !object.held {
            self.on_tile.clear(object.pos);
        }
        self.eggs -= usize::from(object.unhatched.is_some());
        object
    }

    /// Puts the held object `id` down on the tile at `to`, which must hold
    /// no object (design v23 §6.5).
    pub(crate) fn put_down(&mut self, id: EntityId, to: Pos) {
        let object = self
            .by_id
            .get_mut(&id)
            .expect("putting down an object that exists");
        object.held = false;
        object.pos = to;
        self.on_tile.put(to, id);
    }

    /// Picks the object `id`, which must exist and be on the map, off its
    /// tile for the Cursor to hold, at rest (design v23 §6.5).
    pub(crate) fn pick_up(&mut self, id: EntityId) {
        let object = self
            .by_id
            .get_mut(&id)
            .expect("picking up an object that exists");
        self.on_tile.clear(object.pos);
        object.held = true;
        object.roll = None;
    }

    /// Checks that every object stands where the rules allow, alone on its
    /// tile, with its stage, schedule and counters in range, and rolls only if
    /// it's an item with tiles to go; describes the first problem.
    pub(crate) fn check(&self, map: &Map, data: &DataPack, tick: u64) -> Result<(), String> {
        let indexed = self.on_tile.count();
        let on_map = self.by_id.values().filter(|o| !o.held).count();
        if indexed != on_map {
            return Err(format!(
                "{indexed} tiles hold objects, but there are {on_map} objects on the map"
            ));
        }
        let eggs = self
            .by_id
            .values()
            .filter(|o| o.unhatched.is_some())
            .count();
        if eggs != self.eggs {
            return Err(format!(
                "{eggs} objects hold a sprite, but the count of eggs is {}",
                self.eggs
            ));
        }
        for (&id, object) in &self.by_id {
            let object_type = &data.object_types()[object.kind];
            // Where it is: off the map in the Cursor, or on its tile. Then,
            // either way, its own state, since a held item's life goes on
            // (design §3.5.2).
            let whereabouts = if object.held {
                if object_type.solid {
                    Some("is held, yet solid")
                } else if object.roll.is_some() {
                    Some("is held, yet rolling")
                } else {
                    None
                }
            } else if self.at(object.pos) != Some(id) {
                Some("isn't on its tile in the index")
            } else if !map.is_walkable(object.pos) {
                Some("stands on a tile that isn't walkable")
            } else if object_type.solid && !data.terrain(map.terrain(object.pos)).allows_fixtures()
            {
                Some("is solid on terrain that doesn't allow fixtures")
            } else {
                None
            };
            let problem = if let Some(problem) = whereabouts {
                problem
            } else if object_type.pseudo {
                "is of a pseudo type"
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
            } else if object.roll.is_some() && object_type.solid {
                "is solid, yet rolling"
            } else if object.roll.is_some_and(|roll| roll.left == 0) {
                "is rolling with no tiles to go"
            } else if object.unhatched.is_some() != object_type.hatches() {
                "holds a sprite unless it hatches, or hatches without one"
            } else if self.next_due.get(&id).copied() != object.next_turn(object_type, id, tick) {
                "isn't scheduled for the tick its rules and stage say"
            } else if let Some(Err(problem)) = object
                .unhatched
                .as_ref()
                .map(|unhatched| unhatched.check(id, data))
            {
                return Err(format!(
                    "{} {id:?} at {:?} {problem}",
                    object_type.name, object.pos
                ));
            } else {
                continue;
            };
            return Err(format!(
                "{} {id:?} at {:?} {problem}",
                object_type.name, object.pos
            ));
        }
        let queued: usize = self.due_on.values().map(BTreeSet::len).sum();
        if queued != self.next_due.len() {
            return Err(format!(
                "{queued} object turns are queued, but {} objects have a next turn",
                self.next_due.len()
            ));
        }
        for (&id, &due) in &self.next_due {
            if !self.by_id.contains_key(&id) {
                return Err(format!(
                    "{id:?} has a turn scheduled, but isn't in the world"
                ));
            }
            if !self.due_on.get(&due).is_some_and(|set| set.contains(&id)) {
                return Err(format!(
                    "{id:?} is due on tick {due}, but not in that tick's queue"
                ));
            }
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
    use crate::test_util::at;

    use proptest::collection::vec;
    use proptest::prelude::*;

    use super::*;
    use crate::data::DataPack;
    use crate::ecology::new_object;
    use crate::map::{Dir, Map, Pos};

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

        /// A new object called `name` on `pos`, as the world makes one.
        fn object(&self, name: &str, pos: Pos) -> Object {
            new_object(&self.data, self.kind(name), pos)
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
            // With no stage or counters, past its first turn.
            let object = Object {
                stage: None,
                counters: Vec::new(),
                fresh: false,
                ..self.object(name, pos)
            };
            self.objects.place(id, object, &self.data, 0);
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

    #[test]
    fn next_turn_schedules_only_when_a_stage_or_periodic_rule_is_due() {
        let mut scene = Scene::new(&[
            "...", //
            "...", //
            "...", //
        ]);
        // A ball has no stages and no periodic rules, so once past its fresh
        // turn it needs no turn at all.
        let ball = scene.place("ball", at(0, 0));
        assert_eq!(scene.objects.next_due.get(&ball), None);

        // A fresh object is due on the tick it is scheduled from.
        let berry = EntityId(scene.next_id);
        scene.next_id += 1;
        let object = Object {
            stage_ends: 500,
            ..scene.object("berry", at(1, 0))
        };
        scene.objects.place(berry, object, &scene.data, 10);
        assert_eq!(scene.objects.take_due(10), vec![berry]);

        // Once no longer fresh, a berry (stages only, no Every rule) is due
        // when its stage ends.
        scene.objects.get_mut(berry).expect("the berry").fresh = false;
        scene.objects.schedule(berry, &scene.data, 11);
        assert_eq!(scene.objects.next_due.get(&berry), Some(&500));

        // A berry bush has Every(200) and Every(50) rules alongside its
        // stage; with id 3, (tick + 3) % 50 == 0 first holds at tick 47,
        // before stage_ends 800.
        let bush = EntityId(scene.next_id);
        scene.next_id += 1;
        assert_eq!(bush, EntityId(3));
        let object = Object {
            stage_ends: 800,
            fresh: false,
            ..scene.object("berry_bush", at(2, 0))
        };
        scene.objects.place(bush, object, &scene.data, 1);
        assert_eq!(scene.objects.next_due.get(&bush), Some(&47));
        assert_eq!(scene.objects.check(&scene.map, &scene.data, 1), Ok(()));
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
