//! The Terra Sprites simulation.
//!
//! This crate has no terminal and no filesystem access: it receives its data
//! pack as already-read text and is driven one tick at a time.

mod action;
mod biochem;
mod brain;
mod brain_io;
mod config;
mod data;
mod decide;
mod ecology;
mod events;
mod expression;
mod generate;
mod genome;
mod map;
mod object_types;
mod objects;
mod occupancy;
mod perception;
mod physics;
mod physiology;
mod random;
mod regions;
mod registry;
mod sprites;
mod terrain;
mod variation;
mod verbs;
mod world;

pub use action::{ActionView, Outcome, Progress, ScriptedAction};
pub use biochem::Traits;
pub use brain::{Contribution, Explanation};
pub use config::{ConfigError, WorldConfig};
pub use data::{DataError, DataPack};
pub use events::{DeathCause, Event, EventKind, Removal};
pub use expression::Expression;
pub use genome::{EmitterMode, GeneView, Genome, GenomeError};
pub use map::{Dir, Map, MapError, Pos};
pub use objects::EntityId;
pub use perception::Target;
pub use registry::{ChemicalKind, Trait, Verb};
pub use terrain::{Terrain, TerrainProps};
pub use world::{
    ChemicalLevel, InvariantViolation, ObjectView, Scenario, ScenarioError, SpriteView, World,
};
