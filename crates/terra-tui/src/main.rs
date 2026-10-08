//! `terra-sprites`: owns the terminal and runs the title screen (M2 design
//! §8) and the frame loop (design §6.6).
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
use ratatui::crossterm::terminal::{self, supports_keyboard_enhancement};
use terra_sim::{Playback, World};
use terra_tui::app::{App, Flow, Ticks};
use terra_tui::args::{Args, USAGE, VERSION};
use terra_tui::files;
use terra_tui::input::{self, Keys};
use terra_tui::saves;
use terra_tui::session::{self, Session};
use terra_tui::start;
use terra_tui::theme::Theme;
use terra_tui::title::{self, Title, TitleFlow};
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
    if args.version {
        println!("terra-sprites {VERSION}");
        return ExitCode::SUCCESS;
    }
    // Flags that make or load a world go straight in (M2 design §8.5);
    // otherwise the game opens on the title screen. What the flags get
    // wrong stops the game here, before the terminal is taken.
    let first = if start::skips_title(&args) {
        let opening = match &args.replay {
            Some(path) => replay_from(path),
            None => start::new_world(&args, args.seed.unwrap_or_else(time_seed)).map(Opening::live),
        };
        match opening {
            Ok(opening) => Next::World(opening),
            Err(err) => {
                eprintln!("terra-sprites: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        match title_world(&args) {
            Ok(world) => Next::title(world),
            Err(err) => {
                eprintln!("terra-sprites: {err}");
                return ExitCode::FAILURE;
            }
        }
    };
    let data = match &first {
        Next::World(opening) => opening.session.world().data(),
        Next::Title { world, .. } => world.data(),
    };
    let theme = match start::theme(&args, data) {
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
    let mut keys = if enhanced_keys {
        Keys::kitty()
    } else {
        Keys::with_release_reporting(cfg!(windows))
    };
    let session_log = files::session_log();
    // What couldn't be said on screen, said once the terminal is restored.
    let mut afterwards = None;
    let mut next = first;
    let result = execute!(stdout(), EnableMouseCapture).and_then(|()| {
        loop {
            next = match next {
                Next::Title { world, refusal } => {
                    match run_title(&mut terminal, *world, refusal, &args, &theme, &mut keys)? {
                        Some(opening) => Next::World(opening),
                        None => return Ok(()),
                    }
                }
                Next::World(opening) => {
                    let setup = Setup {
                        theme: theme.clone(),
                        force_panic: args.force_panic,
                        session_log: session_log.as_deref(),
                    };
                    let (flow, unwritten) = run_world(&mut terminal, opening, &mut keys, setup)?;
                    if flow == Flow::Quit {
                        afterwards = unwritten;
                        return Ok(());
                    }
                    // Going back to the title screen wakes another world
                    // (M2 design §8.4).
                    Next::Title {
                        world: Box::new(title_world(&args).map_err(io::Error::other)?),
                        refusal: unwritten,
                    }
                }
            }
        }
    });
    undo_setup();
    ratatui::restore();
    if let Some(message) = afterwards {
        eprintln!("terra-sprites: {message}");
    }

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("terra-sprites: {err}");
            ExitCode::FAILURE
        }
    }
}

/// What the game shows next: the title screen, or a world.
enum Next {
    /// The title screen, waking `world`, saying `refusal` if there is one.
    Title {
        world: Box<World>,
        refusal: Option<String>,
    },
    World(Opening),
}

impl Next {
    fn title(world: World) -> Next {
        Next::Title {
            world: Box::new(world),
            refusal: None,
        }
    }
}

/// A world to play: the session, for a replay its file's name, and for a
/// save loaded from the title screen, its name.
struct Opening {
    session: Session,
    replay: Option<String>,
    loaded: Option<String>,
}

impl Opening {
    /// A new world, played live.
    fn live(world: World) -> Opening {
        Opening {
            session: Session::live(world),
            replay: None,
            loaded: None,
        }
    }
}

/// The title screen's world, sized to the terminal (M2 design §8.1).
fn title_world(args: &Args) -> Result<World, String> {
    let (width, height) = terminal::size().unwrap_or((ui::MIN_SIZE.width, ui::MIN_SIZE.height));
    start::title_world(args, time_seed(), (width, height))
}

/// Runs the title screen until the player quits, giving `None`, or starts a
/// world, giving it.
fn run_title(
    terminal: &mut DefaultTerminal,
    world: World,
    refusal: Option<String>,
    args: &Args,
    theme: &Theme,
    keys: &mut Keys,
) -> io::Result<Option<Opening>> {
    let mut title = Title::new(world, theme.clone(), time_seed());
    if let Some(folder) = files::save_folder() {
        title.set_saves(saves::list(&folder), SystemTime::now());
    }
    title.set_presets(start::presets(files::preset_folder().as_deref()));
    if let Some(folder) = files::data_folder() {
        title.set_data_folder(folder);
    }
    if let Some(why) = refusal {
        title.refuse(why);
    }
    let mut last_frame = Instant::now();
    loop {
        // The terminal may have been resized since the last frame.
        title.resize(terminal.size()?);
        terminal.draw(|frame| title::render(frame, &title))?;
        let deadline = last_frame + FRAME;
        while event::poll(deadline.saturating_duration_since(Instant::now()))? {
            let action = match event::read()? {
                // The New world box types its seed (M2 design §8.3).
                Event::Key(key) if title.typing() => keys.typed_action(key),
                Event::Key(key) => keys.action_for(key),
                Event::Mouse(mouse) => input::mouse_action(mouse),
                _ => None,
            };
            let Some(action) = action else { continue };
            match title.apply(action) {
                TitleFlow::Stay => {}
                TitleFlow::Quit => return Ok(None),
                TitleFlow::New { seed, preset } => {
                    let asked = Args {
                        seed: Some(seed),
                        preset,
                        ..args.clone()
                    };
                    match start::new_world(&asked, seed) {
                        Ok(world) => return Ok(Some(Opening::live(world))),
                        Err(why) => title.refuse(why),
                    }
                }
                TitleFlow::Load(save) => match saves::load(&save) {
                    Ok(world) => {
                        return Ok(Some(Opening {
                            session: Session::live(world),
                            replay: None,
                            loaded: Some(save.name),
                        }));
                    }
                    Err(why) => title.refuse(why),
                },
            }
        }
        let now = Instant::now();
        title.animate(now - last_frame);
        last_frame = now;
    }
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
        loaded: None,
    })
}

/// What the frame loop runs with, besides the terminal and the world.
struct Setup<'a> {
    theme: Theme,
    force_panic: bool,
    /// Where the session log goes, if anywhere (design §2.7).
    session_log: Option<&'a Path>,
}

/// Plays `opening` until the player quits or goes back to the title
/// screen, saying which, with why the world couldn't be autosaved or the
/// session log written as it was left, if it couldn't. A session that ends
/// in a panic still writes its replay (design §2.9), as does one the
/// terminal failed under (design §2.7).
fn run_world(
    terminal: &mut DefaultTerminal,
    opening: Opening,
    keys: &mut Keys,
    setup: Setup,
) -> io::Result<(Flow, Option<String>)> {
    let Opening {
        mut session,
        replay,
        loaded,
    } = opening;
    let session_log = setup.session_log;
    let areas = ui::areas(terminal.size()?, session.world().map());
    let world = session.world();
    let mut app = App::new(world.map(), setup.theme.clone(), world.seed(), areas);
    // A save loaded on the title screen starts as a load in a world does
    // (M2 design §8.2).
    if let Some(name) = loaded {
        let Session::Live(world) = session else {
            unreachable!("a save loads live");
        };
        app.load_from_title(*world, name);
        let world = app.take_loaded().expect("the world just handed over");
        session = Session::live(world);
    }
    let ran = session::writing_session_log_on_panic(&mut session, session_log, |session| {
        run(terminal, session, &mut app, keys, &setup, replay.as_deref())
    });
    // Leaving writes the session log, as does a session the terminal failed
    // under (design §2.7). A write that fails is said where it can be.
    let unwritten = session_log.and_then(|path| {
        session
            .write_session_log(path)
            .err()
            .map(|err| format!("couldn't write the replay {}: {err}", path.display()))
    });
    match ran {
        Ok((flow, unsaved)) => {
            let untold: Vec<String> = unsaved.into_iter().chain(unwritten).collect();
            Ok((flow, (!untold.is_empty()).then(|| untold.join("; "))))
        }
        Err(err) => match unwritten {
            Some(unwritten) => Err(io::Error::new(err.kind(), format!("{err}; {unwritten}"))),
            None => Err(err),
        },
    }
}

/// The frame loop, until the player quits or goes back to the title
/// screen, with why the world couldn't be autosaved as it was left, if it
/// couldn't.
fn run(
    terminal: &mut DefaultTerminal,
    session: &mut Session,
    app: &mut App,
    keys: &mut Keys,
    setup: &Setup,
    replay: Option<&str>,
) -> io::Result<(Flow, Option<String>)> {
    let session_log = setup.session_log;
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
        terminal.draw(|frame| ui::render(frame, app, session.world()))?;
        if setup.force_panic {
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
            let flow = action.map_or(Flow::Continue, |action| app.apply(action, session.world()));
            if flow != Flow::Continue {
                // Leaving saves, so a closed session is never lost (design
                // §6.7, M2 design §8.4). The session log is written next.
                let unsaved = app.autosave(session.world());
                return Ok((flow, unsaved));
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
            write_session_log(app, session);
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
