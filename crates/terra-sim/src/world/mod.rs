//! The world (design §2.2–2.4): `World`, stepped one tick at a time, and
//! `WorldState`, everything that decides how it evolves. The front end
//! reads it through the views in `views`; tests and lab scenarios build
//! one by hand from a `Scenario`.

use std::cell::Cell;
use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64_with_seed;

use crate::action::{self, chebyshev};
use crate::ageing::LifeStage;
use crate::biochem::{self, Senses};
use crate::breeding::{self, Sex};
use crate::command::{self, Command};
use crate::config::{MOST_SPRITES, WorldConfig};
use crate::courtship::{self, Company};
use crate::cursor::{Cursor, Grip};
use crate::data::DataPack;
use crate::ecology::{self, new_object, square};
use crate::events::{DeathCause, Emptied, Event, EventKind};
use crate::family;
use crate::generate::{generate, place_newborns, place_objects, place_sprites};
use crate::genome::Genome;
use crate::learning::{self, Subject};
use crate::lineage::{self, FIRST_GENERATION, Record, RecordOfTheDead};
use crate::map::{Map, MapError, Pos};
use crate::mode::Mode;
use crate::objects::{EntityId, Object, Objects};
use crate::perception::{Flood, Target, goal_tiles};
use crate::population::{PopulationCount, PopulationRecord};
use crate::regions::Regions;
use crate::registry::CategoryId;
use crate::replay::{self, CHECKPOINT_EVERY, Recording, Start};
use crate::rolling;
use crate::save::{self, LoadError};
use crate::sliding;
use crate::sprites::{Sprite, Sprites};
use crate::variation::varied;

mod scenario;
#[cfg(test)]
mod tests;
mod views;

pub use scenario::{Scenario, ScenarioError};
pub use views::{ChemicalLevel, CursorView, DeadView, EggView, ObjectView, SpriteView};

/// Fixed seed for `state_hash`, so hashes are comparable across runs and builds.
const STATE_HASH_SEED: u64 = 0x7e22_a5b1_17e5_0001;

/// A simulated world, advanced one tick at a time.
pub struct World {
    state: WorldState,
    /// The data pack the world was made with. It never changes, so it isn't hashed.
    data: DataPack,
    /// The ID counter as `check_invariants` last saw it, to catch it going back.
    checked_next_id: Cell<u64>,
    /// How many sprites had died when `check_invariants` last found the
    /// record of the dead whole. The record changes only when one dies (M2
    /// design §17.1), so its own rules are checked again only then; that
    /// nothing in the world is remembered as dead is checked every tick.
    checked_deaths: Cell<u64>,
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
    /// The most sprites the world holds, eggs included (M2 design §10):
    /// its preset's, or for a hand-made world or a save from before
    /// breeding, the default.
    #[serde(default = "crate::config::most_sprites")]
    pub(crate) most_sprites: u16,
    /// Story or Lab (M2 design §16), kept for the world's life. A save
    /// from before the modes is a Story world.
    #[serde(default)]
    pub(crate) mode: Mode,
    /// The population over the world's whole run, for the population graph
    /// (M2 design §14). Nothing in the world reads it. A save from before
    /// it has none, so its record begins where it was loaded.
    #[serde(default)]
    pub(crate) population: PopulationRecord,
    /// The world's record of the dead, for the family tree (M2 design
    /// §17.1). Nothing in the world reads it. A save from before it has none,
    /// so it remembers only the sprites that die after it's loaded.
    #[serde(default)]
    pub(crate) dead: RecordOfTheDead,
}

impl WorldState {
    /// Rebuilds what a save leaves out because it's derived from the rest
    /// (design §2.8): which entity stands on each tile, each sprite's
    /// compiled genome, and its levels before the tick, taken as its levels
    /// now, so each change on the Chem tab reads blank for one tick. A save
    /// from when eggs were sprites has them turned into egg items first (M2
    /// design §10.4).
    fn rebuild(&mut self, data: &DataPack) -> Result<(), String> {
        self.map.check_shape(data)?;
        self.objects.rebuild(&self.map, data, self.tick)?;
        self.sprites.rebuild(&self.map, data, self.tick)?;
        self.old_eggs_to_items(data);
        self.number_generations();
        Ok(())
    }

    /// Gives each sprite and egg in a save from before generations its
    /// generation (M2 design §17.1), in ascending ID order, so a parent has
    /// its own before its young: one after its newer parent's, where the
    /// world knows that parent, and the first for one with no parents. A
    /// parent the world doesn't know, which died before generations, counts
    /// as the first.
    fn number_generations(&mut self) {
        let mut unnumbered: Vec<EntityId> = self
            .sprites
            .iter()
            .filter(|(_, sprite)| sprite.generation == 0)
            .map(|(id, _)| id)
            .chain(
                self.objects
                    .iter()
                    .filter(|(_, object)| {
                        object
                            .unhatched
                            .as_ref()
                            .is_some_and(|egg| egg.generation == 0)
                    })
                    .map(|(id, _)| id),
            )
            .collect();
        unnumbered.sort();
        for id in unnumbered {
            let parents = match self.sprites.get(id) {
                Some(sprite) => sprite.parents.clone(),
                None => self
                    .objects
                    .get(id)
                    .and_then(|o| o.unhatched.as_ref())
                    .expect("an egg")
                    .parents
                    .clone(),
            };
            let generation = lineage::generation_after(parents.iter().map(|&parent| {
                self.sprites
                    .get(parent)
                    .map(|sprite| sprite.generation)
                    .or_else(|| self.dead.get(parent).map(|record| record.generation))
                    .unwrap_or(FIRST_GENERATION)
            }));
            match self.sprites.get_mut(id) {
                Some(sprite) => sprite.generation = generation,
                None => {
                    let object = self.objects.get_mut(id).expect("an egg");
                    object.unhatched.as_mut().expect("an egg").generation = generation;
                }
            }
        }
    }

    /// Turns each sprite that's still an egg, in a save from before eggs
    /// were items (#196), into an egg item with its ID (M2 design §10.4):
    /// on its tile, or if an item lies there, the nearest tile it may go on,
    /// hatching when it would have. In a pack with no egg type, or a world
    /// with nowhere to put it, it hatches at once.
    fn old_eggs_to_items(&mut self, data: &DataPack) {
        let old: Vec<EntityId> = self
            .sprites
            .iter()
            .filter(|(_, sprite)| sprite.hatches.is_some())
            .map(|(id, _)| id)
            .collect();
        for id in old {
            let sprite = self.sprites.remove(id);
            let pos = sprite.pos;
            let (unhatched, hatches) = sprite.unhatched().expect("a sprite that's still an egg");
            // Before eggs were items, the Cursor could lead one for a while.
            if self.cursor.leads() == Some(id) {
                self.cursor.grip = None;
            }
            let egg = data.egg_type().and_then(|kind| {
                let tile = self.nearest_free(data, kind, pos)?;
                let stage = data.object_types()[kind].hatching_stage()?;
                let mut egg = new_object(data, kind, tile);
                (egg.fresh, egg.stage, egg.stage_ends) = (false, Some(stage), hatches);
                Some(egg)
            });
            match egg {
                Some(mut egg) => {
                    egg.unhatched = Some(Box::new(unhatched));
                    self.objects.place(id, egg, data, self.tick);
                }
                None => {
                    let hatchling = Sprite::hatchling(unhatched, pos, self.tick, data);
                    self.sprites.place(id, hatchling);
                }
            }
        }
    }

    /// The nearest tile to `pos` that an egg of the type `kind` may lie on,
    /// searching outwards a ring at a time, each row by row.
    fn nearest_free(&self, data: &DataPack, kind: usize, pos: Pos) -> Option<Pos> {
        let rings = self.map.width().max(self.map.height());
        (0..rings).find_map(|radius| {
            square(&self.map, pos, radius)
                .filter(|&tile| chebyshev(tile, pos) == radius)
                .find(|&tile| self.can_lie(data, kind, tile))
        })
    }

    /// Gives `object` the next entity ID and puts it in the world, scheduled
    /// from the current tick. The caller has checked that it may go there.
    pub(crate) fn add_object(&mut self, object: Object, data: &DataPack) -> EntityId {
        let from = self.tick;
        self.add_object_from(object, data, from)
    }

    /// Gives `object` the next entity ID and puts it in the world, scheduled
    /// from `from` (M4 design §10), such as `tick + 1` for an object created
    /// during step 2 or step 6. The caller has checked that it may go there.
    pub(crate) fn add_object_from(
        &mut self,
        object: Object,
        data: &DataPack,
        from: u64,
    ) -> EntityId {
        let id = self.new_id();
        self.objects.place(id, object, data, from);
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

    /// Whether an egg of the type `kind` may lie on the tile at `pos`, as
    /// it's laid (M2 design §10.4): an item may go there, and a sprite may
    /// stand there, so it can hatch.
    pub(crate) fn can_lie(&self, data: &DataPack, kind: usize, pos: Pos) -> bool {
        self.can_stand(data, pos) && self.can_place(data, kind, pos)
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
    /// with: a pseudo type for water, a sprite or the Cursor. `None` if it's
    /// gone, or the pack has no such pseudo type.
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

impl World {
    /// A new world, generated from `config` and `seed`, in Story mode.
    pub fn new(config: WorldConfig, data: DataPack, seed: u64) -> World {
        World::new_in(config, data, seed, Mode::Story)
    }

    /// A new world, generated from `config` and `seed`, in `mode`, which
    /// it keeps for its whole life (M2 design §16).
    pub fn new_in(config: WorldConfig, data: DataPack, seed: u64, mode: Mode) -> World {
        World::generated(config, data, seed, mode, |config, data, state| {
            place_sprites(config, data, state, &[])
        })
    }

    /// The world `new` makes from `config` and `seed`, but with its first
    /// sprites taking after the hatched sprites alive in `from`, in turn,
    /// rather than wild: each one's genome and hidden flaws, on the tile the
    /// starter's sprite in its place has, its age and its levels drawn as
    /// that sprite's are (M2 design §19.6). It's the race's evolved side: the
    /// same fresh world as the starter's genes get, for a species' genes.
    /// None if no hatched sprite is alive in `from`, whose data pack must be
    /// `data`.
    pub fn new_like(config: WorldConfig, data: DataPack, seed: u64, from: &World) -> Option<World> {
        let like = from.hatched_genomes();
        if like.is_empty() {
            return None;
        }
        Some(World::generated(
            config,
            data,
            seed,
            Mode::Story,
            |config, data, state| place_sprites(config, data, state, &like),
        ))
    }

    /// The unattended run's world (M2 design §19.6): `config`'s world as
    /// generated from `seed`, in Lab mode, but with its first sprites
    /// newborns, placed as the Cursor places them and spread over the whole
    /// map, and nobody else, so none has anyone to learn from.
    pub fn unattended(config: WorldConfig, data: DataPack, seed: u64) -> World {
        World::generated(config, data, seed, Mode::Lab, |config, data, state| {
            place_newborns(config, data, state, &[])
        })
    }

    /// The unattended run's world as `unattended` makes it, but with its
    /// newborns taking after the hatched sprites alive in `from`, in turn,
    /// rather than wild: each one's genome and hidden flaws (M2 design
    /// §19.6). It's how a species' genes fare with nobody to look after
    /// their young. None if no hatched sprite is alive in `from`, whose
    /// data pack must be `data`.
    pub fn unattended_like(
        config: WorldConfig,
        data: DataPack,
        seed: u64,
        from: &World,
    ) -> Option<World> {
        let like = from.hatched_genomes();
        if like.is_empty() {
            return None;
        }
        Some(World::generated(
            config,
            data,
            seed,
            Mode::Lab,
            |config, data, state| place_newborns(config, data, state, &like),
        ))
    }

    /// The genome and hidden flaws of each hatched sprite alive, in ID
    /// order, for a world whose first sprites take after them.
    fn hatched_genomes(&self) -> Vec<(Genome, Vec<u8>)> {
        self.state
            .sprites
            .iter()
            .map(|(_, sprite)| sprite)
            .filter(|sprite| !sprite.parents.is_empty())
            .map(|sprite| (sprite.genome.clone(), sprite.flaws.clone()))
            .collect()
    }

    /// `config`'s world as generated from `seed`, in `mode`, with its
    /// objects, and its first sprites as `place` puts them: the steps every
    /// generated world shares, so the unattended run's world is the one
    /// `new_in` makes but for its sprites (M2 design §19.6).
    fn generated(
        config: WorldConfig,
        data: DataPack,
        seed: u64,
        mode: Mode,
        place: impl FnOnce(&WorldConfig, &DataPack, &mut WorldState),
    ) -> World {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let map = generate(&config, &data, &mut rng);
        let mut world = World::with(map, data, rng, seed);
        world.state.most_sprites = config.most_sprites() as u16;
        world.state.mode = mode;
        place_objects(&config, &world.data, &mut world.state);
        place(&config, &world.data, &mut world.state);
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

    /// Has the world hold at most `most` sprites, eggs included, for a lab
    /// scenario that watches only its own (M2 design §10).
    pub(crate) fn set_most_sprites(&mut self, most: u16) {
        self.state.most_sprites = most;
    }

    /// Has every sprite grow old at `age` ticks, whatever its genes say,
    /// for a lab scenario that watches the same sprites for longer than a
    /// life (M2 design §2).
    pub(crate) fn set_grows_old(&mut self, age: f32) {
        let data = &self.data;
        let tick = self.state.tick;
        for (_, sprite) in self.state.sprites.iter_mut() {
            sprite.set_grows_old(age, tick, data);
        }
    }

    /// For a lab scenario (M2 design §18.6): gives each sprite in the world,
    /// in ascending ID order, a newborn of its own, of the starter genome
    /// with spawn variation, on the nearest tile a sprite can stand on.
    /// Returns the young, in the same order.
    pub(crate) fn give_young(&mut self) -> Vec<EntityId> {
        let (state, data) = (&mut self.state, &self.data);
        let parents: Vec<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
        let mut young = Vec::new();
        for parent in parents {
            let sprite = state.sprites.get(parent).expect("a sprite in the world");
            let (pos, generation) = (sprite.pos, sprite.generation);
            let rings = state.map.width().max(state.map.height());
            let Some(tile) = (1..rings).find_map(|radius| {
                square(&state.map, pos, radius)
                    .filter(|&tile| chebyshev(tile, pos) == radius)
                    .find(|&tile| state.can_stand(data, tile))
            }) else {
                continue;
            };
            let genome = varied(data.starter(), data, &mut state.rng);
            let mut child = Sprite::newborn(genome, tile, state.tick, data);
            child.parents = vec![parent];
            child.generation = generation + 1;
            young.push(state.add_sprite(child));
        }
        young
    }

    /// For a lab scenario (M2 design §18.6): takes the sprites `ids` out of
    /// the world, as if they had never been there: no death, and no record
    /// of the dead. Their young are left orphans.
    pub(crate) fn take_away(&mut self, ids: &[EntityId]) {
        for &id in ids {
            if self.state.sprites.get(id).is_some() {
                self.state.sprites.remove(id);
            }
        }
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
                most_sprites: MOST_SPRITES,
                mode: Mode::Story,
                population: PopulationRecord::default(),
                dead: RecordOfTheDead::default(),
            },
            data,
            checked_next_id: Cell::new(1),
            checked_deaths: Cell::new(0),
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
        let config = config
            .map(|preset| preset.config(&data))
            .transpose()
            .map_err(LoadError::Damaged)?;
        let world = World {
            checked_next_id: Cell::new(state.next_id),
            // Never a count of deaths, so a load checks the record.
            checked_deaths: Cell::new(u64::MAX),
            state,
            data,
            seed,
            config,
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
                mode: self.state.mode,
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
            && World::new_in(
                config.clone(),
                self.data.clone(),
                self.seed,
                self.state.mode,
            )
            .state_hash()
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

    /// Whether it's a Story world or a Lab world (M2 design §16).
    pub fn mode(&self) -> Mode {
        self.state.mode
    }

    /// The tile at the middle of the screen when the world was saved, if
    /// its save kept one (design §2.8).
    pub fn view(&self) -> Option<Pos> {
        self.view
    }

    /// Advances the world by exactly one tick, running the canonical tick order
    /// (design §2.4), and reports what happened.
    ///
    /// In debug builds and tests, the world then checks its invariants (design
    /// §7.1), and panics naming the tick if one is broken.
    pub fn step(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let state = &mut self.state;
        state
            .population
            .begin(&state.sprites, &state.objects, state.tick);
        self.remember_levels();
        self.apply_commands(&mut events); // 1
        self.run_environment(&mut events); // 2
        self.grow(&mut events); // 3, first
        let mut company = courtship::look_round(&mut self.state, &self.data, &mut events); // 3
        family::look_for_family(&mut self.state, &self.data, &mut company);
        let dying = self.run_biochemistry(&company); // 3
        self.run_learning(&dying, &company, &mut events); // 4
        self.sense_and_decide(&dying, &mut events); // 5
        self.resolve_actions(&dying, &mut events); // 6
        self.finish_tick(&dying, &mut events); // 7
        let state = &mut self.state;
        state
            .population
            .end(&state.sprites, &state.objects, state.tick, &events);
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

    /// Every egg, on the map or in the Cursor, in ascending ID order (M2
    /// design §10.4).
    pub fn eggs(&self) -> impl Iterator<Item = EggView<'_>> {
        self.state.objects.iter().filter_map(|(id, _)| self.egg(id))
    }

    /// The egg `id`, if it's in the world and hasn't hatched (M2 design
    /// §10.4).
    pub fn egg(&self, id: EntityId) -> Option<EggView<'_>> {
        let object = self.state.objects.get(id)?;
        Some(EggView {
            id,
            object,
            unhatched: object.unhatched.as_deref()?,
            world: self,
        })
    }

    /// The dead sprite `id`, if the world remembers it (M2 design §17.1).
    pub fn dead(&self, id: EntityId) -> Option<DeadView<'_>> {
        let record = self.state.dead.get(id)?;
        Some(DeadView { id, record })
    }

    /// Every dead sprite the world remembers, in ascending ID order (M2
    /// design §17.1): the newest 10,000, and every one the player named.
    pub fn dead_sprites(&self) -> impl Iterator<Item = DeadView<'_>> {
        self.state
            .dead
            .iter()
            .map(|(id, record)| DeadView { id, record })
    }

    /// The young of the sprite `id` (M2 design §17.1): every sprite, egg and
    /// remembered dead sprite with `id` among its parents, in ascending ID
    /// order.
    pub fn young(&self, id: EntityId) -> Vec<EntityId> {
        let state = &self.state;
        let mut young: Vec<EntityId> = state
            .sprites
            .iter()
            .filter(|(_, sprite)| sprite.parents.contains(&id))
            .map(|(young, _)| young)
            .chain(
                self.eggs()
                    .filter(|egg| egg.parents().contains(&id))
                    .map(|egg| egg.id()),
            )
            .chain(
                state
                    .dead
                    .iter()
                    .filter(|(_, record)| record.parents.contains(&id))
                    .map(|(young, _)| young),
            )
            .collect();
        young.sort();
        young
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
        self.sprite(self.state.sprites.at(pos)?)
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

    /// The world's population over its whole run, oldest first (M2 design
    /// §14): a count at the first tick it stepped from, then one every 100
    /// ticks, or less often once the record has squeezed its counts.
    pub fn population(&self) -> &[PopulationCount] {
        self.state.population.counts()
    }

    /// The population now, with what has happened since the last count, as
    /// a count taken now would have it (M2 design §14).
    pub fn population_now(&self) -> PopulationCount {
        let state = &self.state;
        state
            .population
            .now(&state.sprites, &state.objects, state.tick)
    }

    /// Ticks between the population record's counts now (M2 design §14).
    pub fn population_every(&self) -> u64 {
        self.state.population.every()
    }

    /// The most sprites the world holds, eggs included (M2 design §10.6).
    pub fn most_sprites(&self) -> u16 {
        self.state.most_sprites
    }

    /// Whether the world holds its most sprites, eggs included, so that no
    /// sprite can breed until one dies (M2 design §10.6).
    pub fn is_full(&self) -> bool {
        !breeding::room(&self.state)
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

    /// Step 3 begins with each sprite growing to its life stage (M2 design
    /// §9.2), so genes that switch on with it work from this tick's
    /// chemistry on, then ageing its genes (§9.4), and then a male adult
    /// turning female if its hormone says so (§10), in ascending ID order.
    fn grow(&mut self, events: &mut Vec<Event>) {
        let state = &mut self.state;
        let tick = state.tick;
        for (id, sprite) in state.sprites.iter_mut() {
            let grew = sprite.grow(tick, &self.data);
            sprite.age_genes(&self.data, &mut state.rng);
            if sprite.turn(&self.data) {
                events.push(Event {
                    tick,
                    kind: EventKind::TurnedFemale { id },
                });
            }
            let Some((was, stage)) = grew else {
                continue;
            };
            // One whose genes have it grow old before it grows up does both
            // at once (M2 design §9.1).
            if was <= LifeStage::Young && stage.is_grown() {
                events.push(Event {
                    tick,
                    kind: EventKind::GrewUp { id },
                });
            }
            if stage == LifeStage::Old {
                events.push(Event {
                    tick,
                    kind: EventKind::GrewOld { id },
                });
            }
        }
    }

    /// Step 3: every sprite's chemistry (design §4.4), then death check #1.
    /// Its `company` says whether it sees or has lately heard a female or
    /// male adult, or its partner (M2 design §10.1, §15). Returns the
    /// sprites marked dying, in ascending ID order.
    fn run_biochemistry(&mut self, company: &BTreeMap<EntityId, Company>) -> Vec<EntityId> {
        let state = &mut self.state;
        let data = &self.data;
        let nearby = data.physiology().nearby_sprites;
        let (tick, sprites) = (state.tick, &state.sprites);
        let senses: Vec<(EntityId, Senses)> = sprites
            .iter()
            .map(|(id, sprite)| {
                let others = square(&state.map, sprite.pos, nearby.radius)
                    .filter(|&tile| tile != sprite.pos && sprites.at(tile).is_some())
                    .count();
                let company = &company[&id];
                let senses = Senses {
                    age: sprite.age(tick),
                    nearby_sprites: (others as f32 / f32::from(nearby.full)).min(1.0),
                    steps: sprite.did.steps,
                    resting: sprite.did.rested,
                    standing: action::is_standing(sprite),
                    stage: sprite.stage(),
                    sooner: sprite.sooner(data),
                    fertile: breeding::fertile(state, id, data),
                    female: sprite.sex == Sex::Female,
                    female_near: company.female_near,
                    male_near: company.male_near,
                    partner_near: company.partner_near,
                    paired: sprite.partner().is_some(),
                    family: company.family,
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
    /// sprite not marked dying, and getting to know the sprites in its
    /// `company` (M2 design §15.2).
    fn run_learning(
        &mut self,
        dying: &[EntityId],
        company: &BTreeMap<EntityId, Company>,
        events: &mut Vec<Event>,
    ) {
        learning::run(&mut self.state, &self.data, dying, company, events);
    }

    /// Step 5: perception, attention and decisions, for every sprite not
    /// marked dying.
    fn sense_and_decide(&mut self, dying: &[EntityId], events: &mut Vec<Event>) {
        action::sense_and_decide(&mut self.state, &self.data, dying, events);
    }

    /// Step 6: movement and verb effects, then trace entries, for every
    /// sprite not marked dying.
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
        for &id in &dying {
            let sprite = state.sprites.remove(id);
            let cause = sprite.body.cause_of_death();
            *state.deaths.entry(cause).or_insert(0) += 1;
            state
                .dead
                .remember(id, Record::of(&sprite, cause, state.tick));
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
        courtship::mourn(state, &self.data, &dying, events);
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
        // An egg hands its ID on to the sprite it hatches into (M2 design
        // §10.4), so an ID is an object's or a sprite's, never both.
        if let Some((id, _)) = state
            .sprites
            .iter()
            .find(|&(id, _)| state.objects.get(id).is_some())
        {
            return Err(InvariantViolation(format!(
                "{id:?} is both a sprite and an object"
            )));
        }
        let ids = || {
            state
                .objects
                .iter()
                .map(|(id, _)| id)
                .chain(state.sprites.iter().map(|(id, _)| id))
        };
        if let Some(id) = ids().find(|id| id.0 >= state.next_id) {
            return Err(InvariantViolation(format!(
                "{id:?} is at or above the ID counter, {}: IDs must only go up",
                state.next_id
            )));
        }
        // An ID is never used again, so nothing in the world is one the
        // world remembers as dead (M2 design §17.1).
        if let Some(id) = ids().find(|&id| state.dead.get(id).is_some()) {
            return Err(InvariantViolation(format!(
                "{id:?} is remembered as dead but is in the world"
            )));
        }
        state
            .objects
            .check(&state.map, &self.data, state.tick)
            .and_then(|()| state.sprites.check(&state.map, &state.objects, &self.data))
            .and_then(|()| {
                state
                    .cursor
                    .check(&state.map, &state.objects, &state.sprites)
            })
            .and_then(|()| state.population.check())
            .and_then(|()| {
                let deaths = state.deaths.values().sum();
                if deaths == self.checked_deaths.get() {
                    return Ok(());
                }
                // Noted only once it holds, so a record found broken is
                // found broken again however often it's checked.
                state.dead.check(&state.sprites)?;
                self.checked_deaths.set(deaths);
                Ok(())
            })
            .map_err(InvariantViolation)
    }
}
