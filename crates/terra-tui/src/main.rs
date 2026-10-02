//! `terra-sprites`: owns the terminal and runs the frame loop (design §6.6).
//! All logic worth testing lives in the `terra_tui` library.

use std::io::{self, stdout};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use terra_sim::{DataPack, World, WorldConfig};
use terra_tui::app::{App, Flow, Ticks};
use terra_tui::args::{Args, USAGE};
use terra_tui::files;
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
    // Where the terminal supports the kitty keyboard protocol, it reports
    // repeats and releases too, so held keys can be told from presses (design
    // v27 §6.6). Windows reports them anyway, and says it has no support.
    let enhanced_keys = supports_keyboard_enhancement().unwrap_or(false)
        && execute!(
            stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    // Plain letters and `+` send only text without this, with
                    // no repeats or releases.
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        )
        .is_ok();
    // Neither mouse capture nor the keyboard flags are part of ratatui's
    // restore, so the panic hook undoes them too. The flags go first: the
    // terminal keeps them per screen, and ratatui's restore leaves the
    // alternate screen they were set on.
    // Once only: a panic after a normal exit mustn't pop the flags again, on
    // the screen ratatui has gone back to.
    static SETUP_UNDONE: AtomicBool = AtomicBool::new(false);
    let undo_setup = move || {
        if SETUP_UNDONE.swap(true, Ordering::SeqCst) {
            return;
        }
        if enhanced_keys {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = execute!(stdout(), DisableMouseCapture);
    };
    let restore_terminal = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        undo_setup();
        restore_terminal(info);
    }));
    let keys = Keys::with_release_reporting(cfg!(windows) || enhanced_keys);
    let result = execute!(stdout(), EnableMouseCapture)
        .and_then(|()| run(&mut terminal, world, theme, seed, keys, args.force_panic));
    undo_setup();
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
    mut keys: Keys,
    force_panic: bool,
) -> io::Result<()> {
    let areas = ui::areas(terminal.size()?, world.map());
    let mut app = App::new(world.map(), theme, seed, areas);
    if let Some(folder) = files::data_folder() {
        app.set_data_folder(folder);
    }
    if let Some(folder) = files::genome_folder() {
        app.set_genome_folder(folder);
    }
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
                // While naming, keys type letters (design v28 §6.5).
                Event::Key(key) if app.typing() => keys.typed_action(key),
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

        // The player's clicks, stamped for the next tick (design §2.5).
        for command in app.take_commands() {
            world.submit(command);
        }

        let now = Instant::now();
        let elapsed = now - last_frame;
        last_frame = now;
        app.animate(elapsed);
        let frame_start = Instant::now();
        let mut ticks = Ticks::default();
        app.clock.advance(
            elapsed,
            || ticks.step(&mut world),
            || frame_start.elapsed() >= SIM_BUDGET,
        );
        app.take_in(ticks, &world);
    }
}

/// A seed from the clock, used when `--seed` isn't given.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
