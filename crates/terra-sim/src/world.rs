use std::cell::Cell;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::Serialize;
use xxhash_rust::xxh3::xxh3_64_with_seed;

use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::ecology::{self, holds_without_drawing, new_object};
use crate::events::Event;
use crate::generate::{generate, place_objects};
use crate::map::{Map, MapError, Pos};
use crate::objects::{EntityId, Object, Objects};
use crate::regions::Regions;

/// Fixed seed for `state_hash`, so hashes are comparable across runs and builds.
const STATE_HASH_SEED: u64 = 0x7e22_a5b1_17e5_0001;

/// A simulated world, advanced one tick at a time.
pub struct World {
    state: WorldState,
    /// The data pack the world was made with. It never changes, so it isn't hashed.
    data: DataPack,
    /// The ID counter as `check_invariants` last saw it, to catch it going back.
    checked_next_id: Cell<u64>,
}

/// Everything that determines how the world evolves. Hashed by `state_hash`.
#[derive(Serialize)]
pub(crate) struct WorldState {
    pub(crate) tick: u64,
    /// The world's only source of randomness (design §2.3).
    pub(crate) rng: ChaCha8Rng,
    pub(crate) map: Map,
    /// The ID the next entity gets. It only goes up, so IDs are never reused.
    pub(crate) next_id: u64,
    pub(crate) objects: Objects,
}

impl WorldState {
    /// Gives `object` the next entity ID and puts it in the world. The caller
    /// has checked that it may go there.
    pub(crate) fn add_object(&mut self, object: Object) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.objects.place(id, object);
        id
    }
}

/// A broken internal invariant: always a bug in the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation(pub String);

/// Why a hand-made world could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioError {
    /// The map isn't usable for a world.
    Map(MapError),
    /// No object type of this name is in the data pack, or it's a pseudo type.
    NotAnObjectType(String),
    /// An object would break the placement rules (design §3.3–3.4).
    CantPlace { object_type: String, pos: Pos },
}

/// A read-only view of one object.
pub struct ObjectView<'a> {
    id: EntityId,
    object: &'a Object,
    world: &'a World,
}

impl<'a> ObjectView<'a> {
    /// The object's entity ID.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The name of the object's type, as `objects.ron` gives it.
    pub fn type_name(&self) -> &'a str {
        &self.world.data.object_types()[self.object.kind].name
    }

    /// The tile the object stands on.
    pub fn pos(&self) -> Pos {
        self.object.pos
    }

    /// Whether nothing can move through the object (design §3.3).
    pub fn is_solid(&self) -> bool {
        self.world.data.object_types()[self.object.kind].solid
    }

    /// The value of the object's counter called `name`, or `None` if its type has no such counter.
    pub fn counter(&self, name: &str) -> Option<u16> {
        let counters = &self.world.data.object_types()[self.object.kind].counters;
        let index = counters.iter().position(|c| c.name == name)?;
        Some(self.object.counters[index])
    }

    /// The name of the look a theme draws the object with: the state of the
    /// first of its type's visual rules whose conditions hold, or `"default"`.
    pub fn visual_state(&self) -> &'a str {
        let world = self.world;
        let object_type = &world.data.object_types()[self.object.kind];
        object_type
            .visual
            .iter()
            .find(|visual| {
                visual.conditions.iter().all(|condition| {
                    holds_without_drawing(
                        &world.state.map,
                        &world.state.objects,
                        &world.data,
                        self.object,
                        self.object.pos,
                        condition,
                    )
                    .expect("visual rules never use Chance")
                })
            })
            .map_or("default", |visual| visual.state.as_str())
    }

    /// The name of the object's current stage, or `None` if its type has no stages.
    pub fn stage(&self) -> Option<&'a str> {
        let stages = &self.world.data.object_types()[self.object.kind].stages;
        self.object.stage.map(|stage| stages[stage].name.as_str())
    }
}

impl World {
    /// A new world, generated from `config` and `seed`.
    pub fn new(config: WorldConfig, data: DataPack, seed: u64) -> World {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let map = generate(&config, &data, &mut rng);
        let mut world = World::with(map, data, rng);
        place_objects(&config, &world.data, &mut world.state);
        world
    }

    /// A world on a hand-drawn map, which must form exactly one region.
    pub fn from_map(map: Map, data: DataPack, seed: u64) -> Result<World, MapError> {
        let regions = Regions::find(&map).count();
        if regions != 1 {
            return Err(MapError::NotOneRegion { regions });
        }
        Ok(World::with(map, data, ChaCha8Rng::seed_from_u64(seed)))
    }

    /// A world on a hand-drawn map with objects placed by hand, as
    /// `(tile, object type name)`, for tests and lab scenarios. The objects get
    /// IDs in the order given, and each starts at the beginning of its first stage.
    pub fn from_scenario(
        map: Map,
        objects: &[(Pos, &str)],
        data: DataPack,
        seed: u64,
    ) -> Result<World, ScenarioError> {
        let mut world = World::from_map(map, data, seed).map_err(ScenarioError::Map)?;
        for &(pos, name) in objects {
            let not_an_object = || ScenarioError::NotAnObjectType(name.into());
            let kind = world
                .data
                .real_object_type(name)
                .ok_or_else(not_an_object)?;
            let state = &mut world.state;
            if !state.objects.can_place(&state.map, &world.data, kind, pos) {
                return Err(ScenarioError::CantPlace {
                    object_type: name.into(),
                    pos,
                });
            }
            state.add_object(new_object(&world.data, kind, pos));
        }
        Ok(world)
    }

    fn with(map: Map, data: DataPack, rng: ChaCha8Rng) -> World {
        let objects = Objects::new(&map);
        World {
            state: WorldState {
                tick: 0,
                rng,
                map,
                next_id: 1,
                objects,
            },
            data,
            checked_next_id: Cell::new(1),
        }
    }

    /// Advances the world by exactly one tick, running the canonical tick order
    /// (design §2.4), and reports what happened. Later slices fill in the steps
    /// that are empty today.
    pub fn step(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        self.apply_commands(); // 1
        self.run_environment(&mut events); // 2
        self.run_biochemistry(); // 3
        self.run_learning(); // 4
        self.sense_and_decide(); // 5
        self.resolve_actions(); // 6
        self.finish_tick(); // 7
        events
    }

    /// The world's map.
    pub fn map(&self) -> &Map {
        &self.state.map
    }

    /// The data pack the world was made with.
    pub fn data(&self) -> &DataPack {
        &self.data
    }

    /// Every object, in ascending ID order.
    pub fn objects(&self) -> impl Iterator<Item = ObjectView<'_>> {
        self.state.objects.iter().map(|(id, object)| ObjectView {
            id,
            object,
            world: self,
        })
    }

    /// The object on the tile at `pos`, if any. Off the map there is none.
    pub fn object_at(&self, pos: Pos) -> Option<ObjectView<'_>> {
        if !self.state.map.contains(pos) {
            return None;
        }
        let id = self.state.objects.at(pos)?;
        Some(ObjectView {
            id,
            object: self.state.objects.get(id)?,
            world: self,
        })
    }

    /// The number of ticks simulated so far.
    pub fn tick(&self) -> u64 {
        self.state.tick
    }

    /// A stable hash of the whole world state: xxh3 over its MessagePack encoding.
    pub fn state_hash(&self) -> u64 {
        let bytes = rmp_serde::to_vec_named(&self.state)
            .expect("world state always serializes to MessagePack");
        xxh3_64_with_seed(&bytes, STATE_HASH_SEED)
    }

    /// Step 1: apply the commands stamped for this tick.
    fn apply_commands(&mut self) {}

    /// Step 2: objects run their lifecycle rules.
    fn run_environment(&mut self, events: &mut Vec<Event>) {
        ecology::run(&mut self.state, &self.data, events);
    }

    /// Step 3: pulse latch, physics, reactions, decay, emitters, receptors; death check #1.
    fn run_biochemistry(&mut self) {}

    /// Step 4: reinforcement from consumed reward and punishment.
    fn run_learning(&mut self) {}

    /// Step 5: perception, attention and decisions.
    fn sense_and_decide(&mut self) {}

    /// Step 6: movement and verb effects, then trace entries.
    fn resolve_actions(&mut self) {}

    /// Step 7: death check #2, removals and events; then the tick counter advances.
    fn finish_tick(&mut self) {
        self.state.tick += 1;
    }

    /// Checks the world's internal invariants (design §7.1). Later slices add checks here.
    pub fn check_invariants(&self) -> Result<(), InvariantViolation> {
        let state = &self.state;
        // Keep the highest value seen, so a counter that went back stays caught.
        if state.next_id < self.checked_next_id.get() {
            return Err(InvariantViolation(format!(
                "the ID counter went back, to {}: IDs must only go up",
                state.next_id
            )));
        }
        self.checked_next_id.set(state.next_id);
        if let Some((id, _)) = state.objects.iter().find(|(id, _)| id.0 >= state.next_id) {
            return Err(InvariantViolation(format!(
                "{id:?} is at or above the ID counter, {}: IDs must only go up",
                state.next_id
            )));
        }
        state
            .objects
            .check(&state.map, &self.data)
            .map_err(InvariantViolation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 7×5 field of grass, with a pool of shallow water at (5, 3), and a
    /// berry bush at (2, 2), before any step.
    fn field_with_a_bush() -> World {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let rows = [".......", ".......", ".......", ".....~.", "......."];
        let map = Map::from_ascii(&rows, &data).expect("valid drawing");
        World::from_scenario(map, &[(Pos { x: 2, y: 2 }, "berry_bush")], data, 7)
            .expect("valid scenario")
    }

    /// Puts an object of type `name` on `pos` without checking the placement rules.
    fn force_place(world: &mut World, name: &str, pos: Pos) {
        let kind = world.data.object_type_named(name).expect("a built-in type");
        world.state.add_object(new_object(&world.data, kind, pos));
    }

    #[test]
    fn a_valid_world_passes_its_invariant_checks() {
        assert_eq!(field_with_a_bush().check_invariants(), Ok(()));
    }

    #[test]
    fn a_solid_object_on_terrain_that_does_not_allow_fixtures_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        force_place(&mut world, "thornbush", Pos { x: 3, y: 3 });
        assert_eq!(world.check_invariants(), Ok(()), "side by side is fine");
        force_place(&mut world, "thornbush", Pos { x: 5, y: 3 });
        assert!(world.check_invariants().is_err(), "in the pool");
    }

    #[test]
    fn an_id_the_counter_has_not_reached_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        world.state.next_id = 1;
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn an_id_counter_that_goes_back_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        force_place(&mut world, "berry", Pos { x: 5, y: 1 });
        let berry = world
            .state
            .objects
            .at(Pos { x: 5, y: 1 })
            .expect("the berry");
        world.state.objects.remove(berry);
        assert_eq!(world.check_invariants(), Ok(()));
        // Every object left is below the counter, but the berry's ID could now be reused.
        world.state.next_id -= 1;
        assert!(world.check_invariants().is_err());
        assert!(world.check_invariants().is_err(), "and it stays caught");
    }

    #[test]
    fn a_counter_above_its_maximum_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        let id = world
            .state
            .objects
            .at(Pos { x: 2, y: 2 })
            .expect("the bush");
        world.state.objects.get_mut(id).expect("the bush").counters[0] = 7;
        assert!(world.check_invariants().is_err());
    }
}
