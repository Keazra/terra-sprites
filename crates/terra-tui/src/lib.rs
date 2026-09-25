//! The Terra Sprites terminal front end. `main.rs` owns the terminal; the logic
//! lives here so it can be tested without one.

pub mod app;
pub mod args;
pub mod clock;
pub mod cp437;
pub mod input;
mod inspector;
pub mod theme;
pub mod ui;
