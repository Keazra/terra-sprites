//! `terra-sprites`: owns the terminal and runs the frame loop (design §6.6).
//! All logic worth testing lives in the `terra_tui` library.

use std::cell::Cell;
use std::io::{self, stdout};
use std::path::Path;
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
use terra_sim::Playback;
use terra_tui::app::{App, Flow, Ticks};
use terra_tui::args::{Args, USAGE};
use terra_tui::files;
use terra_tui::input::{self, Keys};
use terra_tui::session::{self, Session};
use terra_tui::start;
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
    let start = match &args.replay {
        Some(path) => replay_from(path),
        None => start::new_world(&args, args.seed.unwrap_or_else(time_seed)).map(|world| Opening {
            session: Session::live(world),
            replay: None,
        }),
    };
    let Opening {
        mut session,
        replay,
    } = match start {
        Ok(start) => start,
        Err(err) => {
            eprintln!("terra-sprites: {err}");
            return ExitCode::FAILURE;
        }
    };
    let theme = match start::theme(&args, session.world().data()) {
        Ok(theme) => theme,
        Err(err) => {
            eprintln!("terra-sprites: {err}");
            return ExitCode::FAILURE;
        }
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
                    // A shifted key as the character it types, `?` or a
                    // capital, on any layout, rather than the key with
                    // Shift held (#124).
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
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
    let keys = if enhanced_keys {
        Keys::kitty()
    } else {
        Keys::with_release_reporting(cfg!(windows))
    };
    // A session that ends in a panic still writes its replay (design §2.9).
    let session_log = files::session_log();
    let result =
        session::writing_session_log_on_panic(&mut session, session_log.as_deref(), |session| {
            execute!(stdout(), EnableMouseCapture).and_then(|()| {
                let setup = Setup {
                    theme,
                    keys,
                    force_panic: args.force_panic,
                    session_log: session_log.as_deref(),
                    replay: replay.as_deref(),
                };
                run(&mut terminal, session, setup)
            })
        });
    undo_setup();
    ratatui::restore();
    // Quitting writes the session log, as does a session the terminal
    // failed under (design §2.7), and a write that fails says so here, as
    // there's no screen left to say it on.
    if let Some(path) = &session_log
        && let Err(err) = session.write_session_log(path)
    {
        eprintln!(
            "terra-sprites: couldn't write the replay {}: {err}",
            path.display()
        );
    }

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("terra-sprites: {err}");
            ExitCode::FAILURE
        }
    }
}

/// What `main` starts with: the session, and for a replay, its file's name.
struct Opening {
    session: Session,
    replay: Option<String>,
}

/// The replay in the file at `path`, to play back (design §2.7).
fn replay_from(path: &Path) -> Result<Opening, String> {
    let cant = |why: String| format!("can't play the replay {}: {why}", path.display());
    let bytes = std::fs::read(path).map_err(|err| cant(err.to_string()))?;
    let playback = Playback::new(&bytes).map_err(|err| cant(err.to_string()))?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Ok(Opening {
        session: Session::replay(playback),
        replay: Some(name),
    })
}

/// What the frame loop runs with, besides the terminal and the session.
struct Setup<'a> {
    theme: Theme,
    keys: Keys,
    force_panic: bool,
    /// Where the session log goes, if anywhere (design §2.7).
    session_log: Option<&'a Path>,
    /// For a replay, its file's name.
    replay: Option<&'a str>,
}

fn run(terminal: &mut DefaultTerminal, session: &mut Session, setup: Setup) -> io::Result<()> {
    let Setup {
        theme,
        mut keys,
        force_panic,
        session_log,
        replay,
    } = setup;
    let areas = ui::areas(terminal.size()?, session.world().map());
    let world = session.world();
    let mut app = App::new(world.map(), theme, world.seed(), areas);
    if let Some(folder) = files::data_folder() {
        app.set_data_folder(folder);
    }
    if let Some(folder) = files::genome_folder() {
        app.set_genome_folder(folder);
    }
    if let Some(folder) = files::save_folder() {
        app.set_save_folder(folder);
    }
    if let Some(folder) = files::theme_folder() {
        app.set_theme_folder(folder);
    }
    if let (Some(name), Some(playback)) = (replay, session.playback()) {
        app.start_replay(name, playback);
    }
    // The session log is written at each autosave (design §2.7), and a
    // write that fails says so on the status line.
    let write_session_log = |app: &mut App, session: &Session| {
        if let Some(session_log) = session_log
            && let Err(err) = session.write_session_log(session_log)
        {
            app.note_session_log_failed(&err.to_string());
        }
    };
    let mut last_frame = Instant::now();

    loop {
        // The terminal may have been resized since the last frame.
        app.fit(ui::areas(terminal.size()?, session.world().map()));
        terminal.draw(|frame| ui::render(frame, &app, session.world()))?;
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
                && app.apply(action, session.world()) == Flow::Quit
            {
                // Quitting saves, so a closed session is never lost (design
                // §6.7). `main` then writes the session log, once the
                // terminal is restored, so a failure can be told.
                app.autosave(session.world());
                return Ok(());
            }
            // A save the player loaded replaces the world from here on, and
            // the session log starts afresh from it (design §2.7).
            if let Some(loaded) = app.take_loaded() {
                session.load(loaded);
            }
            // Taking over a replay plays on from its world, live (design
            // v36 §2.7).
            if app.take_taken_over() {
                session.take_over();
            }
            // The player's clicks, stamped for the next tick (design
            // §2.5), as each is made, so a save made next holds them. A
            // replay takes none.
            session.submit(app.take_commands());
        }

        let now = Instant::now();
        let elapsed = now - last_frame;
        last_frame = now;
        app.animate(elapsed);
        let frame_start = Instant::now();
        let mut ticks = Ticks::default();
        // Playback stops at the recording's end, and where it first finds
        // the world different from it, so the screen pauses there.
        let stop = Cell::new(false);
        let stops = app.replay_stops();
        app.clock.advance(
            elapsed,
            || {
                let events = session.step();
                ticks.note(events, session.world());
                if let (Some(stops), Some(playback)) = (stops, session.playback()) {
                    stop.set(stops.at(playback));
                }
            },
            || stop.get() || frame_start.elapsed() >= SIM_BUDGET,
        );
        app.take_in(ticks, session.world());
        if let Some(playback) = session.playback() {
            app.take_in_replay(playback);
        }
        if app.autosave_if_due(session.world()) {
            write_session_log(&mut app, session);
        }
    }
}

/// A seed from the clock, used when `--seed` isn't given.
fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
