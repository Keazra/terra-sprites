//! The Terra Sprites simulation.
//!
//! This crate has no terminal and no filesystem access: it receives its data
//! pack as already-read text and is driven one tick at a time.

mod config;
mod data;
mod generate;
mod map;
mod object_types;
mod objects;
mod regions;
mod registry;
mod terrain;
mod world;

pub use config::{ConfigError, WorldConfig};
pub use data::{DataError, DataPack};
pub use map::{Dir, Map, MapError, Pos};
pub use terrain::{Terrain, TerrainProps};
pub use world::{InvariantViolation, World};
