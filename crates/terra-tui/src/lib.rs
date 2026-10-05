//! The Terra Sprites terminal front end. `main.rs` owns the terminal; the logic
//! lives here so it can be tested without one.

pub mod app;
pub mod args;
pub mod clock;
pub mod cp437;
pub mod files;
mod help;
pub mod input;
mod inspector;
pub mod policy;
pub mod saves;
pub mod session;
pub mod sprite_list;
mod text;
pub mod theme;
pub mod ui;
