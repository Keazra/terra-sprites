use std::cell::Cell;
use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::Serialize;
use xxhash_rust::xxh3::xxh3_64_with_seed;

use crate::action::{self, ActionView, ScriptedAction};
use crate::biochem::{self, Senses, Traits};
use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::ecology::{self, holds_without_drawing, new_object, square};
use crate::events::{DeathCause, Event, EventKind};
use crate::expression::{Expression, expressions};
use crate::generate::{generate, place_objects, place_sprites};
use crate::genome::{GeneView, Genome};
use crate::map::{Map, MapError, Pos};
use crate::objects::{EntityId, Object, Objects};
use crate::perception::{Flood, Target, goal_tiles};
use crate::regions::Regions;
use crate::registry::{Category, ChemicalKind};
use crate::sprites::{Sprite, Sprites};
use crate::variation::varied;

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
    pub(crate) sprites: Sprites,
    /// How many sprites have died of each cause, for the causes any has.
    pub(crate) deaths: BTreeMap<DeathCause, u64>,
}

impl WorldState {
    /// Gives `object` the next entity ID and puts it in the world. The caller
    /// has checked that it may go there.
    pub(crate) fn add_object(&mut self, object: Object) -> EntityId {
        let id = self.new_id();
        self.objects.place(id, object);
        id
    }

    /// Gives `sprite` the next entity ID and puts it in the world. The caller
    /// has checked that its tile is free.
    pub(crate) fn add_sprite(&mut self, sprite: Sprite) -> EntityId {
        let id = self.new_id();
        self.sprites.place(id, sprite);
        id
    }

    /// Whether an object of type `kind` may go on the tile at `pos` (design
    /// §3.3–3.4): as the objects and terrain allow, and, if it's solid, onto
    /// no sprite.
    pub(crate) fn can_place(&self, data: &DataPack, kind: usize, pos: Pos) -> bool {
        self.objects.can_place(&self.map, data, kind, pos)
            && !(data.object_types()[kind].solid && self.sprites.at(pos).is_some())
    }

    /// Whether a sprite may stand on the tile at `pos` (design §3.4): a
    /// walkable tile on the map holding no sprite and no solid object.
    pub(crate) fn can_stand(&self, data: &DataPack, pos: Pos) -> bool {
        self.map.contains(pos)
            && self.map.is_walkable(pos)
            && self.sprites.at(pos).is_none()
            && !self.objects.is_solid_at(data, pos)
    }

    /// The object on `pos`, as a target.
    pub(crate) fn object_target(&self, pos: Pos) -> Option<Target> {
        self.map
            .contains(pos)
            .then(|| self.objects.at(pos))?
            .map(Target::Object)
    }

    /// The water on `pos`, as a target, if it's drinkable.
    pub(crate) fn water_target(&self, data: &DataPack, pos: Pos) -> Option<Target> {
        let drinkable =
            self.map.contains(pos) && data.terrain(self.map.terrain(pos)).is_drinkable();
        drinkable.then_some(Target::Water(pos))
    }

    /// The sprite on `pos`, as a target.
    pub(crate) fn sprite_target(&self, pos: Pos) -> Option<Target> {
        self.map
            .contains(pos)
            .then(|| self.sprites.at(pos))?
            .map(Target::Sprite)
    }

    /// Where `target` is, and whether a sprite may act on it from its own
    /// tile (an item or water) as well as from beside it (design §3.6).
    /// `None` if it's gone.
    pub(crate) fn whereabouts(&self, data: &DataPack, target: Target) -> Option<(Pos, bool)> {
        match target {
            Target::Object(id) => {
                let object = self.objects.get(id)?;
                Some((object.pos, !data.object_types()[object.kind].solid))
            }
            Target::Water(pos) => Some((pos, true)),
            Target::Sprite(id) => Some((self.sprites.get(id)?.pos, false)),
        }
    }

    /// Where a sprite with `flood` heads to act on `target`: its nearest
    /// reachable goal tile. `None` if the target is gone or the flood
    /// reaches none of its goal tiles.
    pub(crate) fn goal_for(&self, data: &DataPack, flood: &Flood, target: Target) -> Option<Pos> {
        let (there, own_tile) = self.whereabouts(data, target)?;
        flood
            .nearest_goal(&self.map, there, own_tile)
            .map(|(goal, _)| goal)
    }

    /// The index of `target`'s object type, whose verb table it answers
    /// with: a pseudo type for water or a sprite. `None` if it's gone, or
    /// the pack has no such pseudo type.
    pub(crate) fn kind_of(&self, data: &DataPack, target: Target) -> Option<usize> {
        match target {
            Target::Object(id) => self.objects.get(id).map(|o| o.kind),
            Target::Water(_) => data.pseudo_type(Category::Water),
            Target::Sprite(_) => data.pseudo_type(Category::Sprite),
        }
    }

    /// The stable ID of `target`'s object type, as `kind_of` finds it.
    pub(crate) fn type_of(&self, data: &DataPack, target: Target) -> Option<u16> {
        self.kind_of(data, target)
            .map(|kind| data.object_types()[kind].id)
    }

    /// Whether `pos` is a goal tile of `target` (design §3.6): beside it, or
    /// for an item or water, its own tile too.
    pub(crate) fn on_goal_tile(&self, data: &DataPack, pos: Pos, target: Target) -> bool {
        self.whereabouts(data, target)
            .is_some_and(|(there, own_tile)| {
                goal_tiles(&self.map, there, own_tile).any(|g| g == pos)
            })
    }

    fn new_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
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
    /// A sprite can't stand on this tile (design §3.4).
    CantPlaceSprite(Pos),
    /// A scripted action is for a tile with no sprite on it.
    NoSpriteToScript(Pos),
    /// There's no object on this tile to start.
    NoObject(Pos),
    /// The object's type has no stage of this name.
    NoSuchStage { object_type: String, stage: String },
    /// The object's type has no counter of this name.
    NoSuchCounter {
        object_type: String,
        counter: String,
    },
}

/// A hand-made world, for tests and lab scenarios.
pub struct Scenario<'a> {
    /// A hand-drawn map, which must form exactly one region.
    pub map: Map,
    /// Objects as `(tile, object type name)`. Each starts at the beginning of its first stage.
    pub objects: &'a [(Pos, &'a str)],
    /// Newborn sprites as `(tile, genome)`: `None` is the starter genome with
    /// spawn variation, as for the `SpawnSprite` command.
    pub sprites: &'a [(Pos, Option<Genome>)],
    /// Actions to start sprites on, by the tile each sprite starts on,
    /// instead of what they would choose. A sprite given several does them in
    /// the order given.
    pub scripted: &'a [(Pos, ScriptedAction)],
}

/// A chemical's level in one sprite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChemicalLevel<'a> {
    /// The chemical's name in the data pack.
    pub name: &'a str,
    /// Physical, drive, learning signal or hormone.
    pub kind: ChemicalKind,
    /// From 0 to 1.
    pub level: f32,
    /// The level now less the level one tick ago.
    pub change: f32,
}

/// A read-only view of one sprite.
pub struct SpriteView<'a> {
    id: EntityId,
    sprite: &'a Sprite,
    world: &'a World,
}

impl<'a> SpriteView<'a> {
    /// The sprite's entity ID.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The tile the sprite stands on.
    pub fn pos(&self) -> Pos {
        self.sprite.pos
    }

    /// Ticks since the sprite was born.
    pub fn age(&self) -> u64 {
        self.sprite.age(self.world.state.tick)
    }

    /// What the sprite is doing, or the action that last ended until the
    /// next one starts; `None` before its first.
    pub fn action(&self) -> Option<ActionView> {
        action::view(self.sprite, &self.world.data)
    }

    /// The traits its body has: its genes', clamped to physiology's ranges.
    pub fn traits(&self) -> Traits {
        self.sprite.program.traits
    }

    /// Every gene in the sprite's genome, in order, with how it's expressed.
    pub fn genes(&self) -> Vec<(GeneView<'a>, Expression)> {
        let data = &self.world.data;
        let genome = &self.sprite.genome;
        genome
            .genes
            .iter()
            .map(|gene| gene.view(data))
            .zip(expressions(genome, data))
            .collect()
    }

    /// Every chemical in the sprite, in the data pack's order.
    pub fn chemicals(&self) -> impl Iterator<Item = ChemicalLevel<'a>> + use<'a> {
        let body = &self.sprite.body;
        self.world
            .data
            .chemicals()
            .iter()
            .zip(body.chems.iter().zip(&body.chems_before_tick))
            .map(|(chemical, (&level, &before))| ChemicalLevel {
                name: &chemical.name,
                kind: chemical.kind(),
                level,
                change: level - before,
            })
    }

    /// The level of the chemical called `name`, or `None` if the pack has no such chemical.
    pub fn chemical(&self, name: &str) -> Option<f32> {
        let index = self
            .world
            .data
            .chemicals()
            .iter()
            .position(|c| c.name == name)?;
        Some(self.sprite.body.chems[index])
    }
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
        place_sprites(&config, &world.data, &mut world.state);
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

    /// A world made by hand, for tests and lab scenarios. The objects get IDs
    /// in the order given, then the sprites.
    pub fn from_scenario(
        scenario: Scenario,
        data: DataPack,
        seed: u64,
    ) -> Result<World, ScenarioError> {
        let mut world = World::from_map(scenario.map, data, seed).map_err(ScenarioError::Map)?;
        for &(pos, name) in scenario.objects {
            let not_an_object = || ScenarioError::NotAnObjectType(name.into());
            let kind = world
                .data
                .real_object_type(name)
                .ok_or_else(not_an_object)?;
            let state = &mut world.state;
            if !state.can_place(&world.data, kind, pos) {
                return Err(ScenarioError::CantPlace {
                    object_type: name.into(),
                    pos,
                });
            }
            state.add_object(new_object(&world.data, kind, pos));
        }
        for (pos, genome) in scenario.sprites {
            let state = &mut world.state;
            let pos = *pos;
            if !state.can_stand(&world.data, pos) {
                return Err(ScenarioError::CantPlaceSprite(pos));
            }
            let genome = match genome {
                Some(genome) => genome.clone(),
                None => varied(world.data.starter(), &world.data, &mut state.rng),
            };
            let sprite = Sprite::newborn(genome, pos, state.tick, &world.data);
            state.add_sprite(sprite);
        }
        for &(pos, script) in scenario.scripted {
            let sprites = &mut world.state.sprites;
            let id = world
                .state
                .map
                .contains(pos)
                .then(|| sprites.at(pos))
                .flatten()
                .ok_or(ScenarioError::NoSpriteToScript(pos))?;
            let sprite = sprites.get_mut(id).expect("the sprite there");
            sprite.scripted.push_back(script);
        }
        Ok(world)
    }

    /// For a hand-made world: starts the object on `pos` in `stage`, with
    /// the counters `counters` names set as given (each capped at its
    /// maximum), instead of at the start of its first stage. It enters the
    /// stage on its first turn, as a new object enters its first.
    pub fn start_object(
        &mut self,
        pos: Pos,
        stage: &str,
        counters: &[(&str, u16)],
    ) -> Result<(), ScenarioError> {
        let state = &mut self.state;
        let id = state
            .map
            .contains(pos)
            .then(|| state.objects.at(pos))
            .flatten()
            .ok_or(ScenarioError::NoObject(pos))?;
        let object = state.objects.get_mut(id).expect("the object there");
        let object_type = &self.data.object_types()[object.kind];
        let named = |what: &str| (object_type.name.clone(), what.to_string());
        let index = object_type
            .stages
            .iter()
            .position(|s| s.name == stage)
            .ok_or_else(|| {
                let (object_type, stage) = named(stage);
                ScenarioError::NoSuchStage { object_type, stage }
            })?;
        object.stage = Some(index);
        for &(name, value) in counters {
            let counter = object_type
                .counters
                .iter()
                .position(|c| c.name == name)
                .ok_or_else(|| {
                    let (object_type, counter) = named(name);
                    ScenarioError::NoSuchCounter {
                        object_type,
                        counter,
                    }
                })?;
            object.counters[counter] = value.min(object_type.counters[counter].max);
        }
        Ok(())
    }

    fn with(map: Map, data: DataPack, rng: ChaCha8Rng) -> World {
        let objects = Objects::new(&map);
        let sprites = Sprites::new(&map);
        World {
            state: WorldState {
                tick: 0,
                rng,
                map,
                next_id: 1,
                objects,
                sprites,
                deaths: BTreeMap::new(),
            },
            data,
            checked_next_id: Cell::new(1),
        }
    }

    /// Advances the world by exactly one tick, running the canonical tick order
    /// (design §2.4), and reports what happened. Later slices fill in the steps
    /// that are empty today.
    ///
    /// In debug builds and tests, the world then checks its invariants (design
    /// §7.1), and panics naming the tick if one is broken.
    pub fn step(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        self.remember_levels();
        self.apply_commands(); // 1
        self.run_environment(&mut events); // 2
        let dying = self.run_biochemistry(); // 3
        self.run_learning(); // 4
        self.sense_and_decide(&dying, &mut events); // 5
        self.resolve_actions(&dying, &mut events); // 6
        self.finish_tick(&dying, &mut events); // 7
        #[cfg(debug_assertions)]
        if let Err(InvariantViolation(broken)) = self.check_invariants() {
            let tick = self.state.tick - 1;
            panic!("a broken invariant at the end of tick {tick}: {broken}");
        }
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

    /// Every sprite, in ascending ID order.
    pub fn sprites(&self) -> impl Iterator<Item = SpriteView<'_>> {
        self.state.sprites.iter().map(|(id, sprite)| SpriteView {
            id,
            sprite,
            world: self,
        })
    }

    /// The sprite `id`, if it's in the world.
    pub fn sprite(&self, id: EntityId) -> Option<SpriteView<'_>> {
        Some(SpriteView {
            id,
            sprite: self.state.sprites.get(id)?,
            world: self,
        })
    }

    /// The sprite on the tile at `pos`, if any. Off the map there is none.
    pub fn sprite_at(&self, pos: Pos) -> Option<SpriteView<'_>> {
        if !self.state.map.contains(pos) {
            return None;
        }
        let id = self.state.sprites.at(pos)?;
        Some(SpriteView {
            id,
            sprite: self.state.sprites.get(id)?,
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

    /// How many sprites have died of `cause` since the world began.
    pub fn deaths(&self, cause: DeathCause) -> u64 {
        self.state.deaths.get(&cause).copied().unwrap_or(0)
    }

    /// Every cause any sprite has died of since the world began, in order,
    /// with how many.
    pub fn deaths_by_cause(&self) -> impl Iterator<Item = (DeathCause, u64)> + '_ {
        self.state.deaths.iter().map(|(&cause, &n)| (cause, n))
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

    /// Keeps every sprite's chemical levels as they are before the tick, for
    /// the change the Chem tab shows. It's not a step: nothing reads them.
    fn remember_levels(&mut self) {
        for body in self.state.sprites.bodies_mut() {
            body.chems_before_tick.clone_from(&body.chems);
        }
    }

    /// Step 1: apply the commands stamped for this tick.
    fn apply_commands(&mut self) {}

    /// Step 2: objects run their lifecycle rules.
    fn run_environment(&mut self, events: &mut Vec<Event>) {
        ecology::run(&mut self.state, &self.data, events);
    }

    /// Step 3: every sprite's chemistry (design §4.4), then death check #1.
    /// Returns the sprites marked dying, in ascending ID order.
    fn run_biochemistry(&mut self) -> Vec<EntityId> {
        let state = &mut self.state;
        let data = &self.data;
        let nearby = data.physiology().nearby_sprites;
        let senses: Vec<(EntityId, Senses)> = state
            .sprites
            .iter()
            .map(|(id, sprite)| {
                let others = square(&state.map, sprite.pos, nearby.radius)
                    .filter(|&tile| tile != sprite.pos && state.sprites.at(tile).is_some())
                    .count();
                let senses = Senses {
                    age: sprite.age(state.tick),
                    nearby_sprites: (others as f32 / f32::from(nearby.full)).min(1.0),
                    steps: sprite.did.steps,
                    resting: sprite.did.rested,
                };
                (id, senses)
            })
            .collect();
        let mut dying = Vec::new();
        for (id, senses) in senses {
            let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
            if biochem::step(&sprite.program, &mut sprite.body, &senses, data) {
                dying.push(id);
            }
        }
        dying
    }

    /// Step 4: reinforcement from consumed reward and punishment.
    fn run_learning(&mut self) {}

    /// Step 5: perception, attention and decisions.
    fn sense_and_decide(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        action::sense_and_decide(&mut self.state, &self.data, dying, events);
    }

    /// Step 6: movement and verb effects, then trace entries.
    fn resolve_actions(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        action::resolve(&mut self.state, &self.data, dying, events);
    }

    /// Step 7: death check #2 marks the sprites step 6's verbs injured to 1;
    /// then the dying are removed, each with a `Died` event, in ascending ID
    /// order; then the tick counter advances.
    fn finish_tick(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        let state = &mut self.state;
        let injury = self.data.physiology().indices.injury;
        let dying: Vec<EntityId> = state
            .sprites
            .iter()
            .filter(|(id, sprite)| dying.contains(id) || sprite.body.chems[injury] >= 1.0)
            .map(|(id, _)| id)
            .collect();
        for id in dying {
            let sprite = state.sprites.remove(id);
            let cause = sprite.body.cause_of_death();
            *state.deaths.entry(cause).or_insert(0) += 1;
            events.push(Event {
                tick: state.tick,
                kind: EventKind::Died {
                    id,
                    cause,
                    age: sprite.age(state.tick),
                },
            });
        }
        state.tick += 1;
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
        let mut ids = state
            .objects
            .iter()
            .map(|(id, _)| id)
            .chain(state.sprites.iter().map(|(id, _)| id));
        if let Some(id) = ids.find(|id| id.0 >= state.next_id) {
            return Err(InvariantViolation(format!(
                "{id:?} is at or above the ID counter, {}: IDs must only go up",
                state.next_id
            )));
        }
        state
            .objects
            .check(&state.map, &self.data)
            .and_then(|()| state.sprites.check(&state.map, &state.objects, &self.data))
            .map_err(InvariantViolation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Outcome;
    use crate::registry::Verb;

    /// A 7×5 field of grass, with a pool of shallow water at (5, 3), and a
    /// berry bush at (2, 2), before any step.
    fn field_with_a_bush() -> World {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let rows = [".......", ".......", ".......", ".....~.", "......."];
        let map = Map::from_ascii(&rows, &data).expect("valid drawing");
        let scenario = Scenario {
            map,
            objects: &[(Pos { x: 2, y: 2 }, "berry_bush")],
            sprites: &[],
            scripted: &[],
        };
        World::from_scenario(scenario, data, 7).expect("valid scenario")
    }

    /// Puts an object of type `name` on `pos` without checking the placement rules.
    fn force_place(world: &mut World, name: &str, pos: Pos) {
        let kind = world.data.object_type_named(name).expect("a built-in type");
        world.state.add_object(new_object(&world.data, kind, pos));
    }

    #[test]
    #[should_panic(expected = "at the end of tick 0: EntityId(1) is at or above the ID counter")]
    fn in_debug_builds_a_broken_invariant_fails_on_the_tick_it_is_found() {
        let mut world = field_with_a_bush();
        world.state.next_id = 1;
        world.step();
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

    /// The field with a bush, plus starter sprites at (5, 1) and (5, 2).
    fn field_with_sprites() -> (World, EntityId, EntityId) {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let rows = [".......", ".......", ".......", ".....~.", "......."];
        let map = Map::from_ascii(&rows, &data).expect("valid drawing");
        let scenario = Scenario {
            map,
            objects: &[(Pos { x: 2, y: 2 }, "berry_bush")],
            sprites: &[(Pos { x: 5, y: 1 }, None), (Pos { x: 5, y: 2 }, None)],
            scripted: &[],
        };
        let world = World::from_scenario(scenario, data, 7).expect("valid scenario");
        let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
        (world, ids[0], ids[1])
    }

    #[test]
    fn valid_sprites_pass_the_invariant_checks() {
        assert_eq!(field_with_sprites().0.check_invariants(), Ok(()));
    }

    #[test]
    fn a_sprite_the_tile_index_has_elsewhere_breaks_an_invariant() {
        let (mut world, first, _) = field_with_sprites();
        world.state.sprites.get_mut(first).expect("a sprite").pos = Pos { x: 0, y: 0 };
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn two_sprites_on_one_tile_break_an_invariant() {
        let (mut world, first, _) = field_with_sprites();
        world.state.sprites.get_mut(first).expect("a sprite").pos = Pos { x: 5, y: 2 };
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_sprite_on_a_solid_object_breaks_an_invariant() {
        let (mut world, _, _) = field_with_sprites();
        force_place(&mut world, "thornbush", Pos { x: 5, y: 1 });
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_level_outside_0_to_1_breaks_an_invariant() {
        let (mut world, _, second) = field_with_sprites();
        world
            .state
            .sprites
            .get_mut(second)
            .expect("a sprite")
            .body
            .chems[0] = 1.5;
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn levels_a_tick_ago_that_do_not_match_the_chemicals_break_an_invariant() {
        // As a body loaded from a save would have, if loading didn't fill them in.
        let (mut world, first, _) = field_with_sprites();
        let body = &mut world.state.sprites.get_mut(first).expect("a sprite").body;
        body.chems_before_tick.clear();
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_locus_that_is_not_a_number_breaks_an_invariant() {
        let (mut world, first, _) = field_with_sprites();
        world
            .state
            .sprites
            .get_mut(first)
            .expect("a sprite")
            .body
            .loci[1] = f32::NAN;
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_sprite_id_the_counter_has_not_reached_breaks_an_invariant() {
        let (mut world, _, second) = field_with_sprites();
        world.state.next_id = second.0;
        world.checked_next_id.set(second.0);
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn the_state_hash_covers_every_sprite_s_chemistry() {
        let (mut world, _, second) = field_with_sprites();
        let before = world.state_hash();
        world
            .state
            .sprites
            .get_mut(second)
            .expect("a sprite")
            .body
            .chems[5] = 0.5;
        assert_ne!(world.state_hash(), before);
    }

    #[test]
    fn nearby_sprites_counts_the_others_within_3_tiles_over_4_capped_at_1() {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let map = Map::from_ascii(&["........."; 9], &data).expect("valid drawing");
        let at = |x, y| (Pos { x, y }, None);
        let sprites = [
            at(3, 3),
            at(0, 0),
            at(6, 6),
            at(3, 4),
            at(4, 4),
            at(2, 2),
            at(7, 3),
            at(8, 8),
        ];
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted: &[],
        };
        let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
        world.step();
        let index = world.data.physiology().indices.nearby_sprites;
        let reading = |x, y| {
            let pos = Pos { x, y };
            let id = world.state.sprites.at(pos).expect("a sprite");
            let (_, sprite) = world
                .state
                .sprites
                .iter()
                .find(|&(i, _)| i == id)
                .expect("a sprite");
            sprite.body.loci[index]
        };
        assert_eq!(reading(3, 3), 1.0, "5 others within 3 tiles, capped");
        assert_eq!(
            reading(7, 3),
            0.5,
            "(4, 4) and (6, 6); (3, 3) is 4 tiles away"
        );
        assert_eq!(reading(8, 8), 0.25, "(6, 6) only");
    }

    /// A row of grass `length` tiles long with one walker of speed 10, a
    /// step a tick, at its west end, starting on `scripted`.
    fn row_with_a_walker(length: usize, scripted: &[ScriptedAction]) -> (World, EntityId) {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let row = ".".repeat(length);
        let map = Map::from_ascii(&[row.as_str()], &data).expect("valid drawing");
        let start = Pos { x: 0, y: 0 };
        let scripted: Vec<(Pos, ScriptedAction)> = scripted.iter().map(|&a| (start, a)).collect();
        let genes = r#"(format: 1, genes: [Trait(trait: "speed", value: 10.0)])"#;
        let genome = Genome::from_ron(genes, &data).expect("a valid genome");
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &[(start, Some(genome))],
            scripted: &scripted,
        };
        let world = World::from_scenario(scenario, data, 1).expect("valid scenario");
        let id = world.sprites().next().expect("the walker").id();
        (world, id)
    }

    #[test]
    fn a_sprite_that_stays_put_makes_its_flood_again_every_8_ticks() {
        let (mut world, id) = row_with_a_walker(3, &[ScriptedAction::Rest; 3]);
        for tick in 0..24 {
            world.step();
            let flood = world.state.sprites.get(id).expect("alive").flood.as_ref();
            assert_eq!(flood.expect("a flood").made, tick / 8 * 8, "tick {tick}");
        }
    }

    #[test]
    fn a_wander_ends_as_failed_once_its_destination_can_no_longer_be_reached() {
        let destination = Pos { x: 5, y: 0 };
        let (mut world, id) = row_with_a_walker(6, &[ScriptedAction::Wander { destination }]);
        world.step();
        force_place(&mut world, "thornbush", destination);
        let ended: Vec<(u64, EventKind)> = world
            .step()
            .into_iter()
            .filter(|e| matches!(e.kind, EventKind::ActionEnded { .. }))
            .map(|e| (e.tick, e.kind))
            .collect();
        let failed = EventKind::ActionEnded {
            id,
            verb: Verb::Wander,
            outcome: Outcome::Failed,
        };
        assert_eq!(ended, [(1, failed)]);
    }

    #[test]
    fn two_sprites_never_swap_diagonally_past_a_bush_that_grew_since_their_floods() {
        // Speed 4: 40 tenths a tick against a diagonal's 140, so neither
        // steps for the first ticks, and both floods still show the diagonal
        // when the bush grows.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let map = Map::from_ascii(&["..", ".."], &data).expect("valid drawing");
        let genes = r#"(format: 1, genes: [Trait(trait: "speed", value: 4.0)])"#;
        let genome = Genome::from_ron(genes, &data).expect("a valid genome");
        let (a, b) = (Pos { x: 0, y: 0 }, Pos { x: 1, y: 1 });
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &[(a, Some(genome.clone())), (b, Some(genome))],
            scripted: &[
                (a, ScriptedAction::Wander { destination: b }),
                (b, ScriptedAction::Wander { destination: a }),
            ],
        };
        let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
        world.step();
        force_place(&mut world, "thornbush", Pos { x: 1, y: 0 });
        let waiting = |world: &World| -> Vec<(u32, u32)> {
            world
                .state
                .sprites
                .iter()
                .map(|(_, s)| {
                    (
                        s.action.as_ref().expect("wandering").blocked_ticks,
                        s.move_points,
                    )
                })
                .collect()
        };
        world.step();
        assert_eq!(waiting(&world), [(0, 80); 2], "80 tenths: not yet blocked");
        world.step();
        world.step();
        assert_eq!(
            waiting(&world),
            [(1, 140); 2],
            "blocked, banked up to the step"
        );
        assert!(
            world.sprite_at(a).is_some() && world.sprite_at(b).is_some(),
            "no swap"
        );
    }

    #[test]
    fn a_dying_sprite_takes_no_part_in_a_swap() {
        // Head-on in a corridor. The east sprite (speed 10) banks a step's
        // points on the first tick while the west one (speed 5) can't yet
        // pay for a swap; then the east one starves to death.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let map = Map::from_ascii(&["...."], &data).expect("valid drawing");
        let speed = |value: f32| {
            let genes = format!(r#"(format: 1, genes: [Trait(trait: "speed", value: {value:?})])"#);
            Genome::from_ron(&genes, &data).expect("a valid genome")
        };
        let (west, east) = (Pos { x: 1, y: 0 }, Pos { x: 2, y: 0 });
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &[(west, Some(speed(5.0))), (east, Some(speed(10.0)))],
            scripted: &[
                (
                    west,
                    ScriptedAction::Wander {
                        destination: Pos { x: 3, y: 0 },
                    },
                ),
                (
                    east,
                    ScriptedAction::Wander {
                        destination: Pos { x: 0, y: 0 },
                    },
                ),
            ],
        };
        let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
        let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
        world.step();
        let indices = world.data.physiology().indices;
        let dying = world.state.sprites.get_mut(ids[1]).expect("east");
        dying.body.chems[indices.energy] = 0.0;
        dying.body.chems[indices.injury] = 1.0;
        let events = world.step();
        let about_dying: Vec<&EventKind> = events
            .iter()
            .map(|e| &e.kind)
            .filter(|k| matches!(k, EventKind::ActionEnded { id, .. } if *id == ids[1]))
            .collect();
        assert!(about_dying.is_empty(), "{about_dying:?}");
        assert!(world.sprite(ids[1]).is_none(), "it died");
        assert_eq!(world.sprite(ids[0]).expect("alive").pos(), west, "no swap");
    }

    #[test]
    fn a_sprite_arriving_keeps_at_most_one_step_s_worth_of_points() {
        let (mut world, id) = row_with_a_walker(
            6,
            &[ScriptedAction::Wander {
                destination: Pos { x: 1, y: 0 },
            }],
        );
        world
            .state
            .sprites
            .get_mut(id)
            .expect("the walker")
            .move_points = 1_000;
        world.step();
        let walker = world.state.sprites.get(id).expect("the walker");
        assert_eq!(walker.pos, Pos { x: 1, y: 0 });
        assert_eq!(walker.move_points, 100, "one grass step's worth");
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
