//! The Terra Sprites simulation.
//!
//! This crate has no terminal and no filesystem access: it receives its data
//! pack as already-read text and is driven one tick at a time.

mod data;
mod world;

pub use data::{DataError, DataPack};
pub use world::{InvariantViolation, World};
