//! The renderer (#241): draws the game off-screen into a GIF, an MP4 or an
//! animated SVG, through the game's own UI code, so a clip looks exactly
//! like the game. `cargo run --release -p terra-tui --example render --
//! --help` says how to use it.

mod boxes;
mod clip;
mod glyphs;
mod look;
mod options;

use std::process::ExitCode;

use options::{Options, USAGE};

fn main() -> ExitCode {
    let options = match Options::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("render: {err}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let _ = options;
    ExitCode::SUCCESS
}
