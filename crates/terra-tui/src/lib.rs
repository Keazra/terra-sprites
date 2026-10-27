//! The Terra Sprites terminal front end. `main.rs` owns the terminal; the logic
//! lives here so it can be tested without one.

pub mod app;
pub mod args;
pub mod clock;
pub mod controls;
pub mod cp437;
pub mod debug_overlay;
mod family_tree;
pub mod fast_forward;
pub mod files;
mod graph;
mod help;
pub mod input;
mod inspector;
mod listed;
pub mod policy;
pub mod saves;
pub mod session;
pub mod space;
mod spots;
pub mod sprite_list;
pub mod start;
mod text;
pub mod theme;
pub mod title;
mod tooltip;
pub mod ui;
