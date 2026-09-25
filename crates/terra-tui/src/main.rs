//! `terra-sprites`: owns the terminal and runs the frame loop (design §6.6).
//! All logic worth testing lives in the `terra_tui` library.

use std::io::{self, stdout};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use terra_sim::{DataPack, World, WorldConfig};
use terra_tui::app::{App, Flow};
use terra_tui::args::{Args, USAGE};
use terra_tui::input::{self, Keys};
use terra_tui::theme::Theme;
use terra_tui::ui;

/// About 30 frames per second.
const FRAME: Duration = Duration::from_millis(33);
/// The most simulation time a frame may spend, so the UI stays responsive at any speed.
const SIM_BUDGET: Duration = Duration::from_millis(25);

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("terra-sprites: {err}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let data = match DataPack::builtin() {
        Ok(data) => data,
        Err(err) => {
            eprintln!("terra-sprites: the built-in data pack is invalid: {err:?}");
            return ExitCode::FAILURE;
        }
    };
    // A preset names object types, so it's checked against the pack.
    let config = match &args.preset {
        None => WorldConfig::builtin(&data),
        Some(path) => {
            let loaded = std::fs::read_to_string(path)
                .map_err(|err| err.to_string())
                .and_then(|text| {
                    WorldConfig::from_ron(&text, &data).map_err(|err| err.to_string())
                });
            match loaded {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("terra-sprites: can't use preset {}: {err}", path.display());
                    return ExitCode::FAILURE;
                }
            }
        }
    };
    let seed = args.seed.unwrap_or_else(time_seed);
    let world = World::new(config, data, seed);
    let theme = if args.ascii {
        Theme::ascii()
    } else {
        Theme::cp437()
    };

    // Installs a panic hook that restores the terminal before the panic is reported.
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => {
            eprintln!("terra-sprites: could not set up the terminal: {err}");
            return ExitCode::FAILURE;
        }
    };
    // Mouse capture isn't part of ratatui's restore, so the panic hook turns it off too.
    let restore_terminal = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableMouseCapture);
        restore_terminal(info);
    }));
    let result = execute!(stdout(), EnableMouseCapture)
        .and_then(|()| run(&mut terminal, world, theme, seed, args.force_panic));
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("terra-sprites: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    terminal: &mut DefaultTerminal,
    mut world: World,
    theme: Theme,
    seed: u64,
    force_panic: bool,
) -> io::Result<()> {
    let areas = ui::areas(terminal.size()?, world.map());
    let mut app = App::new(world.map(), theme, seed, areas);
    let mut keys = Keys::new();
    let mut last_frame = Instant::now();

    loop {
        // The terminal may have been resized since the last frame.
        app.fit(ui::areas(terminal.size()?, world.map()));
        terminal.draw(|frame| ui::render(frame, &app, &world))?;
        if force_panic {
            panic!("forced panic (--force-panic): the terminal should now be restored");
        }

        // Handle input until the next frame is due.
        let deadline = last_frame + FRAME;
        while event::poll(deadline.saturating_duration_since(Instant::now()))? {
            let action = match event::read()? {
                Event::Key(key) => keys.action_for(key),
                Event::Mouse(mouse) => input::mouse_action(mouse),
                _ => None,
            };
            if let Some(action) = action
                && app.apply(action, &world) == Flow::Quit
            {
                return Ok(());
            }
        }

        let now = Instant::now();
        let elapsed = now - last_frame;
        last_frame = now;
        let frame_start = Instant::now();
        let mut events = Vec::new();
        app.clock.advance(
            elapsed,
            || events.extend(world.step()),
            || frame_start.elapsed() >= SIM_BUDGET,
        );
        app.record(&events);
    }
}

/// A seed from the clock, used when `--seed` isn't given.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
