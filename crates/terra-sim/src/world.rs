use std::cell::Cell;
use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64_with_seed;

use crate::action::{self, ActionView, ScriptedAction};
use crate::biochem::{self, Senses, Traits};
use crate::brain::{Explanation, Memory};
use crate::command::{self, Command};
use crate::config::WorldConfig;
use crate::cursor::{Cursor, Grip};
use crate::data::DataPack;
use crate::ecology::{self, holds_without_drawing, new_object, square};
use crate::events::{DeathCause, Emptied, Event, EventKind};
use crate::expression::{Expression, expressions};
use crate::generate::{generate, place_objects, place_sprites};
use crate::genome::{GeneView, Genome};
use crate::learning::{self, Subject};
use crate::map::{Dir, Map, MapError, Pos};
use crate::objects::{EntityId, Object, Objects};
use crate::perception::{Flood, Target, goal_tiles};
use crate::regions::Regions;
use crate::registry::{CategoryId, ChemicalKind};
use crate::replay::{self, CHECKPOINT_EVERY, Recording, Start};
use crate::rolling;
use crate::save::{self, LoadError};
use crate::sliding;
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
    /// The seed the world was made from, which the top bar shows. The RNG's
    /// state carries on from it, so nothing in the sim reads it.
    seed: u64,
    /// The preset a generated world was made from, kept in its saves (design
    /// §2.8). A hand-made world has none. Nothing in the sim reads it.
    config: Option<WorldConfig>,
    /// The tile at the middle of the screen when the world was saved, if a
    /// screen saved it (design §2.8), so a load shows what the player saw.
    /// Nothing in the sim reads it.
    view: Option<Pos>,
    /// The session log, once the world has been asked to record one
    /// (design §2.7). Nothing in the sim reads it.
    recording: Option<Recording>,
}

/// Everything that determines how the world evolves. Hashed by `state_hash`.
#[derive(Serialize, Deserialize)]
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
    /// Whether sprites learn at step 4; a lab scenario's control run
    /// switches it off (design §5.6, §7.1).
    pub(crate) learning: bool,
    /// The commands submitted for the next tick, in order (design §2.5).
    pub(crate) commands: Vec<Command>,
    /// What the Cursor has hold of (design v23 §6.5).
    pub(crate) cursor: Cursor,
}

impl WorldState {
    /// Rebuilds what a save leaves out because it's derived from the rest
    /// (design §2.8): which entity stands on each tile, each sprite's
    /// compiled genome, and its levels before the tick, taken as its levels
    /// now, so each change on the Chem tab reads blank for one tick.
    fn rebuild(&mut self, data: &DataPack) -> Result<(), String> {
        self.map.check_shape(data)?;
        self.objects.rebuild(&self.map, data)?;
        self.sprites.rebuild(&self.map, data, self.tick)
    }

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

    /// The sprite on `pos` other than `actor`, or else the object there, as a
    /// target for `actor`'s Play or Hit.
    pub(crate) fn contact_target(&self, actor: EntityId, pos: Pos) -> Option<Target> {
        let other = self
            .sprite_target(pos)
            .filter(|&t| t != Target::Sprite(actor));
        other.or_else(|| self.object_target(pos))
    }

    /// Where `target` is, and whether a sprite may act on it from its own
    /// tile (an item, water or the Cursor) as well as from beside it (design
    /// §3.6). `None` if it's gone, off the map in the Cursor included, or
    /// it's the Cursor and sprites can't see it (design v29 §6.5).
    pub(crate) fn whereabouts(&self, data: &DataPack, target: Target) -> Option<(Pos, bool)> {
        match target {
            Target::Object(id) => {
                let object = self.objects.get(id).filter(|o| !o.held)?;
                Some((object.pos, !data.object_types()[object.kind].solid))
            }
            Target::Water(pos) => Some((pos, true)),
            Target::Sprite(id) => Some((self.sprites.get(id)?.pos, false)),
            // It's light, so a sprite may stand under it (design v29 §3.6).
            Target::Cursor => Some((self.cursor.seen_at()?, true)),
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
    /// with: a pseudo type for water, a sprite or the Cursor. `None` if it's gone, or
    /// the pack has no such pseudo type.
    pub(crate) fn kind_of(&self, data: &DataPack, target: Target) -> Option<usize> {
        match target {
            Target::Object(id) => self.objects.get(id).map(|o| o.kind),
            Target::Water(_) => data.pseudo_type(data.water_category()),
            Target::Sprite(_) => data.pseudo_type(data.sprite_category()),
            Target::Cursor => data.pseudo_type(data.cursor_category()),
        }
    }

    /// The stable ID of `target`'s object type, as `kind_of` finds it.
    pub(crate) fn type_of(&self, data: &DataPack, target: Target) -> Option<u16> {
        self.kind_of(data, target)
            .map(|kind| data.object_types()[kind].id)
    }

    /// The category `target` is perceived as (design v19 §3.5.5).
    pub(crate) fn category_of(&self, data: &DataPack, target: Target) -> CategoryId {
        match target {
            Target::Object(id) => data.object_types()[self.objects.kind(id)].category,
            Target::Water(_) => data.water_category(),
            Target::Sprite(_) => data.sprite_category(),
            Target::Cursor => data.cursor_category(),
        }
    }

    /// What a sprite learns about `target` as (design v19 §5.6): its object
    /// type, or its category if it has none.
    pub(crate) fn subject_of(&self, data: &DataPack, target: Target) -> Subject {
        Subject::of(self.category_of(data, target), self.type_of(data, target))
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

    /// The name the player gave it, if any (design v28 §6.5).
    pub fn name(&self) -> Option<&'a str> {
        self.sprite.name.as_deref()
    }

    /// Its genome, as it was born with it: what `g` exports (design v28
    /// §6.5).
    pub fn genome(&self) -> &'a Genome {
        &self.sprite.genome
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

    /// While the Cursor leads it, the steps left on its way to where it's
    /// heading, as near the Cursor as it can get: 0 once it's there, or as
    /// close as it can get (design v23 §6.5). `None` while it isn't led.
    pub fn lead_steps_left(&self) -> Option<u32> {
        action::lead_steps_left(self.sprite)
    }

    /// While a shove sends it, the way it slides and the tiles it has left
    /// to go (design v25 §3.5.4). `None` while it isn't sliding.
    pub fn slide(&self) -> Option<(Dir, u16)> {
        self.sprite.slide.map(|slide| (slide.dir, slide.left))
    }

    /// What its brain did at the latest step 5, explained (design §5.9), or
    /// `None` before its first decision.
    pub fn explain(&self) -> Option<Explanation<'a>> {
        self.sprite.brain.explain(&self.world.data)
    }

    /// What it has learned, furthest from nothing first, up to five
    /// (design §5.9).
    pub fn memory(&self) -> Vec<Memory> {
        self.sprite.brain.memory(&self.world.data)
    }

    /// The reward less the punishment it took in on its last tick, which
    /// learning used up (design §5.6): `last_r`.
    pub fn felt(&self) -> f32 {
        self.sprite.brain.felt
    }

    /// The tile of the one thing its attention was on at the latest step 5
    /// (design §5.3): an object's tile, a water tile, or where a sprite it
    /// attends to is now. `None` before its first decision, with nothing in
    /// reach, once that thing is gone, or while the Cursor leads it, when it
    /// attends to nothing (design v23 §6.5).
    pub fn attending_to(&self) -> Option<Pos> {
        if self.sprite.lead.is_some() {
            return None;
        }
        let target = self.sprite.brain.snapshot.as_ref()?.target?;
        let (pos, _) = self.world.state.whereabouts(&self.world.data, target)?;
        Some(pos)
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
        visual_state(self.world, self.object)
    }

    /// The name of the object's current stage, or `None` if its type has no stages.
    pub fn stage(&self) -> Option<&'a str> {
        let stages = &self.world.data.object_types()[self.object.kind].stages;
        self.object.stage.map(|stage| stages[stage].name.as_str())
    }
}

/// The name of the look a theme draws `object` with: the state of the first
/// of its type's visual rules whose conditions hold, or `"default"`.
fn visual_state<'a>(world: &'a World, object: &Object) -> &'a str {
    let object_type = &world.data.object_types()[object.kind];
    object_type
        .visual
        .iter()
        .find(|visual| {
            visual.conditions.iter().all(|condition| {
                holds_without_drawing(
                    &world.state.map,
                    &world.state.objects,
                    &world.data,
                    object,
                    object.pos,
                    condition,
                )
                .expect("visual rules never use Chance")
            })
        })
        .map_or("default", |visual| visual.state.as_str())
}

/// A read-only view of the Cursor, as far as it touches the world (design
/// v23 §6.5).
pub struct CursorView<'a> {
    world: &'a World,
}

impl<'a> CursorView<'a> {
    /// The sprite the Cursor leads, if any.
    pub fn leads(&self) -> Option<EntityId> {
        self.world.state.cursor.leads()
    }

    /// The tile the world was last told the Cursor is on, which a led
    /// sprite heads for (design v23 §2.5).
    pub fn tile(&self) -> Option<Pos> {
        self.world.state.cursor.tile
    }

    /// Whether sprites can see the Cursor (design v29 §6.5).
    pub fn visible(&self) -> bool {
        self.world.state.cursor.visible
    }

    /// The item the Cursor holds, if any.
    pub fn holds(&self) -> Option<HeldView<'a>> {
        let id = self.world.state.cursor.holds()?;
        let object = self.world.state.objects.get(id)?;
        Some(HeldView {
            id,
            object,
            world: self.world,
        })
    }
}

/// A read-only view of the item the Cursor holds: an object off the map.
pub struct HeldView<'a> {
    id: EntityId,
    object: &'a Object,
    world: &'a World,
}

impl<'a> HeldView<'a> {
    /// The item's entity ID.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The name of the item's type, as `objects.ron` gives it.
    pub fn type_name(&self) -> &'a str {
        &self.world.data.object_types()[self.object.kind].name
    }

    /// The name of the look a theme draws the item with, as for an object
    /// on the map (`ObjectView::visual_state`); held, it has no tile, so
    /// rules asking about one don't hold (design §3.5.2).
    pub fn visual_state(&self) -> &'a str {
        visual_state(self.world, self.object)
    }
}

impl World {
    /// A new world, generated from `config` and `seed`.
    pub fn new(config: WorldConfig, data: DataPack, seed: u64) -> World {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let map = generate(&config, &data, &mut rng);
        let mut world = World::with(map, data, rng, seed);
        place_objects(&config, &world.data, &mut world.state);
        place_sprites(&config, &world.data, &mut world.state);
        world.config = Some(config);
        world
    }

    /// A world on a hand-drawn map, which must form exactly one region.
    pub fn from_map(map: Map, data: DataPack, seed: u64) -> Result<World, MapError> {
        let regions = Regions::find(&map).count();
        if regions != 1 {
            return Err(MapError::NotOneRegion { regions });
        }
        Ok(World::with(
            map,
            data,
            ChaCha8Rng::seed_from_u64(seed),
            seed,
        ))
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

    /// Stops every sprite learning from here on, for a lab scenario's
    /// control run (design §7.1).
    pub(crate) fn switch_off_learning(&mut self) {
        self.state.learning = false;
    }

    fn with(map: Map, data: DataPack, rng: ChaCha8Rng, seed: u64) -> World {
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
                learning: true,
                commands: Vec::new(),
                cursor: Cursor::default(),
            },
            data,
            checked_next_id: Cell::new(1),
            seed,
            config: None,
            view: None,
            recording: None,
        }
    }

    /// Saves the world (design §2.8): everything that determines how it
    /// evolves, with the data pack and preset it was made with, so that
    /// loading it carries on exactly as this world would.
    /// It keeps the view it was loaded with, if any.
    pub fn save(&self) -> Vec<u8> {
        self.save_viewing(self.view)
    }

    /// Saves the world as `save` does, with `view`, the tile at the middle
    /// of the screen, for a load to show again (design §2.8, §6.7).
    pub fn save_with_view(&self, view: Pos) -> Vec<u8> {
        self.save_viewing(Some(view))
    }

    /// The world's save, with `view` as its view.
    fn save_viewing(&self, view: Option<Pos>) -> Vec<u8> {
        save::write(&save::Contents {
            seed: self.seed,
            config: self.config.as_ref().map(save::Preset::from),
            pack: self.data.sources().to_vec(),
            state: &self.state,
            view,
        })
    }

    /// Loads a world saved by `save`, with the data pack the save embeds,
    /// never the files on disk (design §2.8). A save from a newer build, or
    /// one that's damaged, is refused.
    pub fn load(bytes: &[u8]) -> Result<World, LoadError> {
        let save::Loaded {
            seed,
            config,
            pack,
            mut state,
            view,
        } = save::read(bytes)?;
        let sources: Vec<(&str, &str)> = pack
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        let data = DataPack::from_sources(&sources).map_err(LoadError::Pack)?;
        state.rebuild(&data).map_err(LoadError::Damaged)?;
        let world = World {
            checked_next_id: Cell::new(state.next_id),
            state,
            data,
            seed,
            config: config.map(WorldConfig::from),
            view,
            recording: None,
        };
        world
            .check_invariants()
            .map_err(|broken| LoadError::Damaged(broken.0))?;
        Ok(world)
    }

    /// Starts recording the world's session log (design §2.7) from the
    /// world as it is now, dropping any recording begun before: a world
    /// just generated starts it afresh from its seed and preset, and any
    /// other from its save. From here on the world records each command
    /// submitted and, at each tick that's a multiple of 1,000, its hash.
    pub fn start_recording(&mut self) {
        let start = match &self.config {
            Some(config) if self.is_as_generated(config) => Start::Fresh {
                seed: self.seed,
                config: save::Preset::from(config),
            },
            _ => Start::Snapshot(self.save()),
        };
        self.recording = Some(Recording {
            start,
            commands: Vec::new(),
            checkpoints: vec![(self.state.tick, self.state_hash())],
        });
    }

    /// Whether the world is exactly as `World::new` makes it from `config`
    /// and its seed: generating it again gives the same state.
    fn is_as_generated(&self, config: &WorldConfig) -> bool {
        self.state.tick == 0
            && World::new(config.clone(), self.data.clone(), self.seed).state_hash()
                == self.state_hash()
    }

    /// The session log recorded since `start_recording`, as a replay's
    /// bytes, or `None` if the world isn't recording (design §2.7).
    pub fn replay(&self) -> Option<Vec<u8>> {
        let recording = self.recording.as_ref()?;
        Some(replay::write(recording, &self.data, self.state.tick))
    }

    /// The seed the world was made from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The tile at the middle of the screen when the world was saved, if
    /// its save kept one (design §2.8).
    pub fn view(&self) -> Option<Pos> {
        self.view
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
        self.apply_commands(&mut events); // 1
        self.run_environment(&mut events); // 2
        let dying = self.run_biochemistry(); // 3
        self.run_learning(&dying, &mut events); // 4
        self.sense_and_decide(&dying, &mut events); // 5
        self.resolve_actions(&dying, &mut events); // 6
        self.finish_tick(&dying, &mut events); // 7
        self.checkpoint();
        #[cfg(debug_assertions)]
        if let Err(InvariantViolation(broken)) = self.check_invariants() {
            let tick = self.state.tick - 1;
            panic!("a broken invariant at the end of tick {tick}: {broken}");
        }
        events
    }

    /// Submits `command`, stamped for the next tick: it's applied at that
    /// tick's step 1, after any submitted before it (design §2.5).
    pub fn submit(&mut self, command: Command) {
        if let Some(recording) = &mut self.recording {
            recording.commands.push((self.state.tick, command.clone()));
        }
        self.state.commands.push(command);
    }

    /// While recording, takes the world's hash at each tick that's a
    /// multiple of `CHECKPOINT_EVERY` (design §2.7).
    fn checkpoint(&mut self) {
        let tick = self.state.tick;
        if self.recording.is_none() || !tick.is_multiple_of(CHECKPOINT_EVERY) {
            return;
        }
        let hash = self.state_hash();
        if let Some(recording) = &mut self.recording {
            recording.checkpoints.push((tick, hash));
        }
    }

    /// The world's map.
    pub fn map(&self) -> &Map {
        &self.state.map
    }

    /// The data pack the world was made with.
    pub fn data(&self) -> &DataPack {
        &self.data
    }

    /// Every object on the map, in ascending ID order: an item the Cursor
    /// holds is off it (`World::cursor`).
    pub fn objects(&self) -> impl Iterator<Item = ObjectView<'_>> {
        self.state
            .objects
            .iter()
            .filter(|(_, object)| !object.held)
            .map(|(id, object)| ObjectView {
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

    /// The Cursor, as far as it touches the world (design v23 §6.5).
    pub fn cursor(&self) -> CursorView<'_> {
        CursorView { world: self }
    }

    /// The furthest the Cursor throws what `grip` holds, or shoves what it
    /// leads, in tiles, by its size (design v25 §3.5.4). The screen asks it
    /// of a grip it has queued as well as of one the world has applied.
    pub fn furthest(&self, grip: Grip) -> u16 {
        let thing = match grip {
            Grip::Holds(item) => Target::Object(item),
            Grip::Leads(sprite) => Target::Sprite(sprite),
        };
        command::furthest(&self.state, &self.data, thing)
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

    /// The world's state and data pack, for the benchmark hooks.
    pub(crate) fn parts(&mut self) -> (&mut WorldState, &DataPack) {
        (&mut self.state, &self.data)
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
    fn apply_commands(&mut self, events: &mut Vec<Event>) {
        command::apply(&mut self.state, &self.data, events);
    }

    /// Step 2: objects run their lifecycle rules, then rolling items roll,
    /// then shoved sprites slide (design v25 §2.4).
    fn run_environment(&mut self, events: &mut Vec<Event>) {
        ecology::run(&mut self.state, &self.data, events);
        rolling::run(&mut self.state, &self.data, events);
        sliding::run(&mut self.state, &self.data, events);
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
        let injury = data.physiology().indices.injury;
        for (id, senses) in senses {
            let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
            // A crash at step 2 that took injury to 1 kills, as a verb's hurt
            // does at death check #2: healing can't undo it (design v25 §2.4).
            let crashed_to_death = sprite.body.chems[injury] >= 1.0;
            if biochem::step(&sprite.program, &mut sprite.body, &senses, data) || crashed_to_death {
                dying.push(id);
            }
        }
        dying
    }

    /// Step 4: reinforcement from consumed reward and punishment, for every
    /// sprite not marked dying.
    fn run_learning(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        learning::run(&mut self.state, &self.data, dying, events);
    }

    /// Step 5: perception, attention and decisions.
    fn sense_and_decide(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        action::sense_and_decide(&mut self.state, &self.data, dying, events);
    }

    /// Step 6: movement and verb effects, then trace entries.
    fn resolve_actions(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        action::resolve(&mut self.state, &self.data, dying, events);
        learning::commit(&mut self.state, &self.data);
    }

    /// Step 7: death check #2 marks the sprites step 6's verbs injured to 1;
    /// then the dying are removed, each with a `Died` event, in ascending ID
    /// order, a led one emptying the Cursor; then the tick counter advances.
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
                    name: sprite.name,
                },
            });
            if state.cursor.empty_of(Grip::Leads(id)) {
                events.push(Event {
                    tick: state.tick,
                    kind: EventKind::CursorEmptied {
                        reason: Emptied::Died { sprite: id },
                    },
                });
            }
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
            .and_then(|()| {
                state
                    .cursor
                    .check(&state.map, &state.objects, &state.sprites)
            })
            .map_err(InvariantViolation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Outcome, Walk};
    use crate::brain::Brain;
    use crate::cursor::Grip;
    use crate::learning::{Signals, Touch, TraceEntry, TypeMemory};
    use crate::map::Dir;
    use crate::objects::Roll;
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

    /// A save whose checksum holds but whose world doesn't fit its pack, as
    /// a build with a bug might write, is refused rather than crashing the
    /// next tick (design §2.8).
    #[test]
    fn a_save_that_doesnt_fit_its_pack_is_refused() {
        let damage: [fn(&mut WorldState, &DataPack); 9] = [
            |state, _| {
                let id = state.objects.at(Pos { x: 2, y: 2 }).expect("the bush");
                state.objects.get_mut(id).expect("the bush").kind = 127;
            },
            |state, _| {
                the_sprite(state).body.chems.pop();
            },
            // A flood whose way back from a tile it reached leads nowhere.
            |state, _| {
                let flood = the_sprite(state).flood.as_mut().expect("a flood");
                flood.damage_a_step(8);
            },
            |state, _| {
                let flood = the_sprite(state).flood.as_mut().expect("a flood");
                flood.damage_a_step(u8::MAX);
            },
            // A concept of an input the pack doesn't have.
            |state, _| the_sprite(state).brain.concepts[0] = vec![(999, false)],
            // Something learned about an object type the pack doesn't have.
            |state, data| {
                let experience = &mut the_sprite(state).brain.experience;
                let nothing = TypeMemory::new(data.needs().count());
                experience.types.insert(Subject::ObjectType(9999), nothing);
            },
            // A motive for a need the pack doesn't have.
            |state, _| {
                let trace = &mut the_sprite(state).brain.trace;
                trace.back_mut().expect("a trace entry").motive = Some(999);
            },
            // A sprite born, or a trace entry made, after the save.
            |state, _| {
                let tick = state.tick;
                the_sprite(state).born = tick + 1;
            },
            |state, _| {
                let tick = state.tick;
                let trace = &mut the_sprite(state).brain.trace;
                trace.back_mut().expect("a trace entry").tick = tick + 1;
            },
        ];
        for (n, damage) in damage.into_iter().enumerate() {
            let mut world = field_with_a_bush();
            world.submit(Command::SpawnSprite {
                tile: Pos { x: 1, y: 1 },
                genome: None,
            });
            for _ in 0..10 {
                world.step();
            }
            let sprite = the_sprite(&mut world.state);
            sprite.brain.trace.push_back(TraceEntry {
                tick: 10,
                verb: None,
                subject: None,
                motive: Some(0),
            });
            assert!(World::load(&world.save()).is_ok(), "damage {n}");
            damage(&mut world.state, &world.data);
            match World::load(&world.save()) {
                Err(LoadError::Damaged(_)) => {}
                Err(err) => panic!("damage {n} refused for the wrong reason: {err}"),
                Ok(_) => panic!("damage {n} loaded"),
            }
        }
    }

    /// The world's only sprite.
    fn the_sprite(state: &mut WorldState) -> &mut Sprite {
        let (id, _) = state.sprites.iter().next().expect("a sprite");
        state.sprites.get_mut(id).expect("the sprite")
    }

    /// Puts an object of type `name` on `pos` without checking the placement rules.
    fn force_place(world: &mut World, name: &str, pos: Pos) {
        let kind = world.data.object_type_named(name).expect("a built-in type");
        world.state.add_object(new_object(&world.data, kind, pos));
    }

    /// Puts a berry on `pos`, and gives its ID.
    fn force_berry(world: &mut World, pos: Pos) -> EntityId {
        force_place(world, "berry", pos);
        world.state.objects.at(pos).expect("the berry")
    }

    #[test]
    #[should_panic(expected = "at the end of tick 0: EntityId(1) is at or above the ID counter")]
    fn in_debug_builds_a_broken_invariant_fails_on_the_tick_it_is_found() {
        let mut world = field_with_a_bush();
        world.state.next_id = 1;
        world.step();
    }

    /// Sets the object on `pos` rolling east with `left` tiles to go.
    fn set_rolling(world: &mut World, pos: Pos, left: u16) {
        let id = world.state.objects.at(pos).expect("an object there");
        let roll = Roll { dir: Dir::E, left };
        world.state.objects.get_mut(id).expect("the object").roll = Some(roll);
    }

    #[test]
    fn a_rolling_item_with_tiles_to_go_passes_the_invariant_checks() {
        let mut world = field_with_a_bush();
        force_place(&mut world, "ball", Pos { x: 4, y: 1 });
        set_rolling(&mut world, Pos { x: 4, y: 1 }, 3);
        assert_eq!(world.check_invariants(), Ok(()));
    }

    #[test]
    fn a_rolling_solid_object_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        set_rolling(&mut world, Pos { x: 2, y: 2 }, 3);
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_roll_with_no_tiles_to_go_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        force_place(&mut world, "ball", Pos { x: 4, y: 1 });
        set_rolling(&mut world, Pos { x: 4, y: 1 }, 0);
        assert!(world.check_invariants().is_err());
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
    fn a_cursor_leading_and_holding_as_commands_leave_it_passes_the_invariant_checks() {
        let (mut world, first, _) = field_with_sprites();
        world.submit(Command::TakeHold { sprite: first });
        world.step();
        assert_eq!(world.check_invariants(), Ok(()));
        let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
        world.submit(Command::LetGo);
        world.submit(Command::PickUp { item: berry });
        world.step();
        assert_eq!(world.check_invariants(), Ok(()));
    }

    #[test]
    fn a_cursor_leading_a_sprite_that_is_not_there_breaks_an_invariant() {
        let (mut world, first, _) = field_with_sprites();
        world.submit(Command::TakeHold { sprite: first });
        world.step();
        world.state.sprites.remove(first);
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_sprite_led_by_no_cursor_breaks_an_invariant() {
        let (mut world, first, _) = field_with_sprites();
        world.state.sprites.get_mut(first).expect("a sprite").lead = Some(Walk::default());
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_held_item_the_cursor_does_not_hold_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
        world.state.objects.pick_up(berry);
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_held_item_in_a_stage_its_type_lacks_breaks_an_invariant() {
        // Gemini's review of PR #89: a held item's life goes on, so its
        // stage and counters are checked as on the map (design §3.5.2).
        let mut world = field_with_a_bush();
        let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
        world.state.objects.pick_up(berry);
        world.state.cursor.take(Grip::Holds(berry));
        assert_eq!(world.check_invariants(), Ok(()));
        world.state.objects.get_mut(berry).expect("the berry").stage = Some(9);
        assert!(world.check_invariants().is_err());
    }

    #[test]
    fn a_cursor_holding_an_item_on_the_map_breaks_an_invariant() {
        let mut world = field_with_a_bush();
        let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
        world.state.cursor.grip = Some(Grip::Holds(berry));
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
        let failed: Vec<(u64, Verb, Outcome)> = ended
            .into_iter()
            .map(|(tick, kind)| match kind {
                EventKind::ActionEnded {
                    id: who,
                    verb,
                    outcome,
                    ..
                } if who == id => (tick, verb, outcome),
                _ => unreachable!("only endings"),
            })
            .collect();
        assert_eq!(failed, [(1, Verb::Wander, Outcome::Failed)]);
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
                        s.action.as_ref().expect("wandering").walk.blocked_ticks,
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

    /// A sprite of `genes` on (1, 1) of the field with a bush, doing
    /// `script` first.
    fn field_with_one(genes: &str, script: &[ScriptedAction]) -> (World, EntityId) {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let rows = [".......", ".......", ".......", ".....~.", "......."];
        let map = Map::from_ascii(&rows, &data).expect("valid drawing");
        let text = format!("(format: 1, genes: [{genes}])");
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        let at = Pos { x: 1, y: 1 };
        let scripted: Vec<(Pos, ScriptedAction)> = script.iter().map(|&s| (at, s)).collect();
        let scenario = Scenario {
            map,
            objects: &[(Pos { x: 2, y: 2 }, "berry_bush")],
            sprites: &[(at, Some(genome))],
            scripted: &scripted,
        };
        let world = World::from_scenario(scenario, data, 7).expect("valid scenario");
        let id = world.sprites().next().expect("the sprite").id();
        (world, id)
    }

    /// The ticks of the entries in sprite `id`'s trace, oldest first.
    fn trace_ticks(world: &World, id: EntityId) -> Vec<u64> {
        let sprite = world.state.sprites.get(id).expect("the sprite");
        sprite.brain.trace.iter().map(|entry| entry.tick).collect()
    }

    #[test]
    fn a_deciding_sprite_adds_one_trace_entry_every_tick_even_while_its_action_continues() {
        // It rests for 10 ticks: one choice, then 4 ticks carrying on.
        let (mut world, id) = field_with_one(
            r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0)"#,
            &[],
        );
        let started = (0..5)
            .flat_map(|_| world.step())
            .filter(|e| matches!(e.kind, EventKind::ActionStarted { .. }))
            .count();
        assert_eq!(started, 1, "one rest, still going");
        assert_eq!(trace_ticks(&world, id), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_sprite_on_a_scripted_action_adds_no_trace_entries() {
        let (mut world, id) = field_with_one("", &[ScriptedAction::Rest]);
        for _ in 0..5 {
            world.step();
        }
        assert_eq!(trace_ticks(&world, id), [] as [u64; 0]);
    }

    /// A trace entry at `tick`, choosing Eat.
    fn eat_entry(tick: u64) -> TraceEntry {
        TraceEntry {
            tick,
            verb: Some(Verb::Eat),
            subject: None,
            motive: None,
        }
    }

    #[test]
    fn a_sprite_marked_dying_learns_nothing_at_step_4() {
        let (mut world, first, second) = field_with_sprites();
        let reward = world.data.physiology().indices.reward;
        for id in [first, second] {
            let entry = eat_entry(0);
            let sprite = world.state.sprites.get_mut(id).expect("a sprite");
            sprite.brain.trace.push_back(entry);
            sprite.body.chems[reward] = 0.5;
        }
        world.state.tick = 1;
        let mut events = Vec::new();
        learning::run(&mut world.state, &world.data, &[second], &mut events);
        let sprite = |id| world.state.sprites.get(id).expect("a sprite");
        assert_eq!(sprite(first).brain.felt, 0.5, "the living one learns");
        assert_eq!(sprite(first).body.chems[reward], 0.0);
        assert_eq!(sprite(second).brain.felt, 0.0, "the dying one doesn't");
        assert_eq!(
            sprite(second).body.chems[reward],
            0.5,
            "nor uses up its reward"
        );
    }

    #[test]
    fn with_learning_switched_off_step_4_uses_up_reward_and_learns_nothing() {
        // Design v16 §5.6, for a lab scenario's control run (§7.1).
        let (mut world, first, _) = field_with_sprites();
        world.state.learning = false;
        let indices = world.data.physiology().indices;
        let bush = world.data.category_named("bush").expect("a category");
        let sprite = world.state.sprites.get_mut(first).expect("a sprite");
        sprite.brain.touched = Some(Touch {
            tick: 0,
            verb: Some(Verb::Eat),
            subject: Subject::Category(bush),
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
        });
        sprite.body.chems[indices.reward] = 0.5;
        let mut events = Vec::new();
        learning::run(&mut world.state, &world.data, &[], &mut events);
        let sprite = world.state.sprites.get(first).expect("a sprite");
        assert_eq!(sprite.brain.felt, 0.5, "felt, and used up");
        assert_eq!(sprite.body.chems[indices.reward], 0.0);
        assert_eq!(sprite.brain.memory(&world.data), [], "learned nothing");
    }

    #[test]
    fn with_learning_switched_off_the_cursor_s_touch_lands_and_teaches_nothing() {
        // Design v21 §5.6: a control run's sprites still feel a pet, and its
        // reach back is used up with it.
        let (mut world, first, _) = field_with_sprites();
        world.state.learning = false;
        let bush = world.data.category_named("bush").expect("a category");
        world
            .state
            .sprites
            .get_mut(first)
            .expect("a sprite")
            .brain
            .touched = Some(Touch {
            tick: 0,
            verb: Some(Verb::Eat),
            subject: Subject::Category(bush),
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
        });
        world.submit(Command::Reward {
            sprite: first,
            amplified: false,
            reach_back: 10,
        });
        let mut events = Vec::new();
        world.apply_commands(&mut events);
        learning::run(&mut world.state, &world.data, &[], &mut events);
        let sprite = world.state.sprites.get(first).expect("a sprite");
        assert_eq!(sprite.brain.felt, 0.5, "felt, and used up");
        assert_eq!(sprite.brain.reach_back, None, "used up");
        assert_eq!(sprite.brain.memory(&world.data), [], "learned nothing");
    }

    #[test]
    fn the_state_hash_covers_what_every_sprite_has_learned() {
        let (mut world, _, second) = field_with_sprites();
        let entry = eat_entry(0);
        let before = world.state_hash();
        let data = world.data.clone();
        let brain = &mut world.state.sprites.get_mut(second).expect("a sprite").brain;
        brain.trace.push_back(entry);
        let traced = world.state_hash();
        assert_ne!(traced, before, "the trace is hashed");
        let brain = &mut world.state.sprites.get_mut(second).expect("a sprite").brain;
        let bush = data.category_named("bush").expect("a category");
        brain.touched = Some(Touch {
            tick: 0,
            verb: Some(Verb::Eat),
            subject: Subject::Category(bush),
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
        });
        let hunger = |brain: &mut Brain, level| {
            let needs = [vec![level], vec![0.0; data.need_places().len() - 1]].concat();
            Signals {
                relief: brain.relief(&needs, &data),
                needs,
                ..Default::default()
            }
        };
        let signals = hunger(brain, 1.0);
        brain.learn(0, &signals, 1.0, &data);
        let touched = world.state_hash();
        assert_ne!(touched, traced, "what it touched is hashed");
        let brain = &mut world.state.sprites.get_mut(second).expect("a sprite").brain;
        let signals = hunger(brain, 0.5);
        brain.learn(1, &signals, 1.0, &data);
        assert_ne!(world.state_hash(), touched, "what it learned is hashed");
    }

    #[test]
    fn a_learned_link_past_one_or_not_a_number_breaks_an_invariant() {
        for broken in [1.5, f32::NAN] {
            let (mut world, first, _) = field_with_sprites();
            assert_eq!(world.check_invariants(), Ok(()));
            let brain = &mut world.state.sprites.get_mut(first).expect("a sprite").brain;
            brain.break_a_link_for_test(broken);
            assert!(world.check_invariants().is_err(), "{broken}");
        }
    }

    #[test]
    fn a_learned_value_out_of_its_range_or_not_a_number_breaks_an_invariant() {
        // Design v16 §5.6: worth and good 0 to 1, bad −1 to 0, the rest −1 to 1.
        fn known(brain: &mut Brain) -> &mut TypeMemory {
            brain.experience.learn_about(Subject::ObjectType(1), 1)
        }
        let breaks: [fn(&mut Brain); 5] = [
            |brain| known(brain).worth[0] = -0.1,
            |brain| known(brain).good = 1.5,
            |brain| known(brain).bad = 0.2,
            |brain| known(brain).habits[0] = f32::NAN,
            |brain| known(brain).familiarity = 2.0,
        ];
        for (i, broken) in breaks.into_iter().enumerate() {
            let (mut world, first, _) = field_with_sprites();
            broken(&mut world.state.sprites.get_mut(first).expect("a sprite").brain);
            assert!(world.check_invariants().is_err(), "break {i}");
        }
    }

    #[test]
    fn a_felt_value_that_is_not_a_number_breaks_an_invariant() {
        let (mut world, first, _) = field_with_sprites();
        world
            .state
            .sprites
            .get_mut(first)
            .expect("a sprite")
            .brain
            .felt = f32::NAN;
        assert!(world.check_invariants().is_err());
    }
}
