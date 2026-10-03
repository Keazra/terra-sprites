//! The Terra Sprites simulation.
//!
//! This crate has no terminal and no filesystem access: it receives its data
//! pack as already-read text and is driven one tick at a time.

mod action;
#[doc(hidden)]
pub mod bench;
mod biochem;
mod brain;
mod brain_io;
mod categories;
mod command;
mod config;
mod cp437;
mod cursor;
mod data;
mod decide;
mod ecology;
mod events;
mod expression;
mod generate;
mod genome;
mod lab;
mod learning;
mod map;
mod names;
mod object_types;
mod objects;
mod occupancy;
mod perception;
mod physics;
mod physiology;
mod random;
mod regions;
mod registry;
mod rolling;
mod sliding;
mod sprites;
mod tags;
mod terrain;
mod variation;
mod verbs;
mod world;

pub use action::{ActionView, Hurt, Outcome, Progress, ScriptedAction};
pub use biochem::Traits;
pub use brain::{Contribution, Explanation, Learned, Memory, Part, Thing};
pub use command::{Blocker, Command, CursorTouch, PlaceRule, Rejection};
pub use config::{ConfigError, WorldConfig};
pub use cp437::is_cp437;
pub use cursor::Grip;
pub use data::{DataError, DataPack};
pub use events::{DeathCause, Emptied, Event, EventKind, Removal};
pub use expression::Expression;
pub use genome::{EmitterMode, GeneView, Genome, GenomeError};
pub use lab::{LabError, LabRun, LabScenario, Window, Without, median, report};
pub use map::{Dir, Map, MapError, Pos};
pub use names::{MAX_NAME_CHARS, NameProblem};
pub use objects::EntityId;
pub use perception::Target;
pub use registry::{ChemicalKind, Trait, Verb};
pub use terrain::{Terrain, TerrainProps};
pub use world::{
    ChemicalLevel, CursorView, HeldView, InvariantViolation, ObjectView, Scenario, ScenarioError,
    SpriteView, World,
};
