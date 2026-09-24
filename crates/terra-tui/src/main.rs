//! `terra-sprites`: owns the terminal and runs the frame loop (design §6.6).
//! All logic worth testing lives in the `terra_tui` library.

use std::io;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};
use terra_sim::{DataPack, World, WorldConfig};
use terra_tui::clock::Clock;
use terra_tui::input::{Action, Keys};
use terra_tui::ui;

/// About 30 frames per second.
const FRAME: Duration = Duration::from_millis(33);
/// The most simulation time a frame may spend, so the UI stays responsive at any speed.
const SIM_BUDGET: Duration = Duration::from_millis(25);

fn main() -> ExitCode {
    // Hidden developer flag: panics after the first frame, to check the terminal is restored.
    let force_panic = std::env::args().any(|arg| arg == "--force-panic");

    let data = match DataPack::builtin() {
        Ok(data) => data,
        Err(err) => {
            eprintln!("terra-sprites: the built-in data pack is invalid: {err:?}");
            return ExitCode::FAILURE;
        }
    };
    let world = World::new(WorldConfig::builtin(), data, time_seed());

    // Installs a panic hook that restores the terminal before the panic is reported.
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => {
            eprintln!("terra-sprites: could not set up the terminal: {err}");
            return ExitCode::FAILURE;
        }
    };
    let result = run(&mut terminal, world, force_panic);
    ratatui::restore();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("terra-sprites: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(terminal: &mut DefaultTerminal, mut world: World, force_panic: bool) -> io::Result<()> {
    let mut clock = Clock::new();
    let mut keys = Keys::new();
    let mut last_frame = Instant::now();

    loop {
        terminal.draw(|frame| ui::render(frame, &world, &clock))?;
        if force_panic {
            panic!("forced panic (--force-panic): the terminal should now be restored");
        }

        // Handle input until the next frame is due.
        let deadline = last_frame + FRAME;
        while event::poll(deadline.saturating_duration_since(Instant::now()))? {
            if let Event::Key(key) = event::read()? {
                match keys.action_for(key) {
                    Some(Action::Quit) => return Ok(()),
                    Some(Action::TogglePause) => clock.toggle_pause(),
                    Some(Action::StepOnce) => clock.step_once(),
                    Some(Action::Faster { held: false }) => clock.faster(),
                    Some(Action::Faster { held: true }) => clock.faster_held(),
                    Some(Action::Slower { held: false }) => clock.slower(),
                    Some(Action::Slower { held: true }) => clock.slower_held(),
                    None => {}
                }
            }
        }

        let now = Instant::now();
        let elapsed = now - last_frame;
        last_frame = now;
        let frame_start = Instant::now();
        clock.advance(
            elapsed,
            || world.step(),
            || frame_start.elapsed() >= SIM_BUDGET,
        );
    }
}

/// A seed from the clock. `--seed` arrives with world presets in slice 2.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
