//! Hand-made worlds, for tests and lab scenarios (design §7.1): a
//! `Scenario` and what `World` builds from one.

use super::World;
use crate::action::ScriptedAction;
use crate::data::DataPack;
use crate::ecology::new_object;
use crate::genome::Genome;
use crate::map::{Map, MapError, Pos};
use crate::sprites::Sprite;
use crate::variation::varied;

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
    /// Sprites as `(tile, genome)`: `None` is the starter genome with spawn
    /// variation, as for the `SpawnSprite` command. Each starts just grown
    /// up, as old as its genes say it grows up (M2 design §9.1), unless its
    /// tile is in `newborns`.
    pub sprites: &'a [(Pos, Option<Genome>)],
    /// Actions to start sprites on, by the tile each sprite starts on,
    /// instead of what they would choose. A sprite given several does them in
    /// the order given.
    pub scripted: &'a [(Pos, ScriptedAction)],
    /// The tiles of the sprites that start as newborns.
    pub newborns: &'a [Pos],
}

impl World {
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
            state.add_object(new_object(&world.data, kind, pos), &world.data);
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
            let sprite = if scenario.newborns.contains(&pos) {
                Sprite::newborn(genome, pos, state.tick, &world.data)
            } else {
                Sprite::grown_up(genome, pos, state.tick, &world.data)
            };
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
        state.objects.schedule(id, &self.data, state.tick);
        Ok(())
    }
}
