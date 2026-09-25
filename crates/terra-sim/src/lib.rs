//! The Terra Sprites simulation.
//!
//! This crate has no terminal and no filesystem access: it receives its data
//! pack as already-read text and is driven one tick at a time.

mod biochem;
mod config;
mod data;
mod ecology;
mod events;
mod expression;
mod generate;
mod genome;
mod map;
mod object_types;
mod objects;
mod physiology;
mod random;
mod regions;
mod registry;
mod terrain;
mod variation;
mod world;

pub use config::{ConfigError, WorldConfig};
pub use data::{DataError, DataPack};
pub use events::{Event, EventKind, Removal};
pub use genome::{Genome, GenomeError};
pub use map::{Dir, Map, MapError, Pos};
pub use objects::EntityId;
pub use terrain::{Terrain, TerrainProps};
pub use world::{InvariantViolation, ObjectView, ScenarioError, World};
