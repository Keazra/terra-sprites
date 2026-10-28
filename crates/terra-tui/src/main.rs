//! `terra-sprites`: owns the terminal and runs the title screen (M2 design
//! §8) and the frame loop (design §6.6).
//! All logic worth testing lives in the `terra_tui` library.

use std::cell::Cell;
use std::fmt::Display;
use std::io::{self, IsTerminal, Write, stdout};
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{self, supports_keyboard_enhancement};
use ratatui::{DefaultTerminal, Frame};
use terra_sim::{Playback, World};
use terra_tui::app::{App, Flow, Ticks};
use terra_tui::args::{Args, USAGE, VERSION};
use terra_tui::clock::SIM_BUDGET;
use terra_tui::debug_overlay::FrameTimes;
use terra_tui::fast_forward::{self, FastForward};
use terra_tui::files;
use terra_tui::input::{self, Keys};
use terra_tui::saves;
use terra_tui::session::{self, Session};
use terra_tui::settings::{self, SavedSettings, Settings};
use terra_tui::start;
use terra_tui::theme::{self, Theme};
use terra_tui::title::{self, Title, TitleFlow};
use terra_tui::ui;

/// About 30 frames per second.
const FRAME: Duration = Duration::from_millis(33);

/// Draws `frame`'s exact colours as the nearest of 256 in a terminal that
/// doesn't show exact colours (design §6.2), as the game finds when it
/// starts.
fn fit_colours(frame: &mut Frame) {
    static EXACT: LazyLock<bool> = LazyLock::new(|| {
        let colorterm = std::env::var("COLORTERM").ok();
        theme::shows_exact_colours(colorterm.as_deref(), cfg!(windows))
    });
    if !*EXACT {
        theme::in_256_colours(frame.buffer_mut());
    }
}

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(err) => return fail(format!("{err}\n{USAGE}")),
    };
    if args.version {
        println!("terra-sprites {VERSION}");
        return ExitCode::SUCCESS;
    }
    // Fast-forward runs with no screen at all (M2 design §16).
    if let Some(hours) = args.fast_forward {
        return fast_forward(&args, hours);
    }
    // Flags that make or load a world go straight in (M2 design §8.5);
    // otherwise the game opens on the title screen. What the flags get
    // wrong stops the game here, before the terminal is taken.
    let first = if start::skips_title(&args) {
        match (&args.replay, &args.load) {
            (Some(path), _) => replay_from(path),
            (None, Some(name)) => load_from(name),
            (None, None) => {
                start::new_world(&args, args.seed.unwrap_or_else(time_seed)).map(Opening::live)
            }
        }
        .map(Next::World)
    } else {
        title_world(&args).map(Next::title)
    };
    let mut first = match first {
        Ok(first) => first,
        Err(err) => return fail(err),
    };
    let data = match &first {
        Next::World(opening) => opening.session.world().data(),
        Next::Title { world, .. } => world.data(),
    };
    // The flags win for the run, then what the player picked on the
    // settings screen, saved in the game's folder, then `NO_COLOR` (design
    // §6.2, §6.7, #307). Any value of it but nothing counts, even one that
    // isn't Unicode. A settings file that can't be read, or names a theme
    // that no longer loads, is left alone, and the game starts on its own,
    // saying why.
    let no_color = std::env::var_os("NO_COLOR").map(|value| value.to_string_lossy().into_owned());
    let settings_file = files::settings_file();
    let saved = settings_file
        .as_deref()
        .map_or_else(|| Ok(SavedSettings::default()), settings::read);
    let theme_folder = files::theme_folder();
    let (mut settings, mut unread) = match start::settings(
        &args,
        saved,
        theme_folder.as_deref(),
        no_color.as_deref(),
        data,
    ) {
        Ok(found) => found,
        Err(err) => return fail(err),
    };
    if let Next::Title { refusal, .. } = &mut first {
        *refusal = unread.take();
    }
    start::write_colours();

    // Installs a panic hook that restores the terminal before the panic is reported.
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => return fail(format!("could not set up the terminal: {err}")),
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
                    // The title screen draws in the look the player has
                    // (design §6.7, #307).
                    let theme = settings.look.theme();
                    match run_title(&mut terminal, *world, refusal, &args, &theme, &mut keys)? {
                        Some(opening) => Next::World(opening),
                        None => return Ok(()),
                    }
                }
                Next::World(opening) => {
                    let setup = Setup {
                        settings: settings.clone(),
                        unread: unread.take(),
                        force_panic: args.force_panic,
                        session_log: session_log.as_deref(),
                    };
                    let (flow, unwritten, left) =
                        run_world(&mut terminal, opening, &mut keys, setup)?;
                    // The next world, or the title screen, starts as the
                    // player left the settings (design §6.7, #307).
                    settings = left;
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
        Err(err) => fail(err),
    }
}

/// Says what went wrong, after the game's name, and fails.
fn fail(why: impl Display) -> ExitCode {
    eprintln!("terra-sprites: {why}");
    ExitCode::FAILURE
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

    /// The world in the save called `name`, played live.
    fn loaded(world: World, name: String) -> Opening {
        Opening {
            loaded: Some(name),
            ..Opening::live(world)
        }
    }
}

/// The title screen's world, sized to the terminal (M2 design §8.1).
fn title_world(args: &Args) -> Result<World, String> {
    let smallest = title::MIN_SIZE;
    let (width, height) = terminal::size().unwrap_or((smallest.width, smallest.height));
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
    let mut title = Title::new(world, theme.clone(), time_seed(), terminal.size()?);
    if let Some(folder) = files::save_folder() {
        title.set_saves(saves::list(&folder), SystemTime::now());
        title.set_save_folder(folder);
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
        terminal.draw(|frame| {
            title::render(frame, &title);
            fit_colours(frame);
        })?;
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
                TitleFlow::New { seed, preset, mode } => {
                    let asked = Args {
                        seed: Some(seed),
                        preset,
                        mode,
                        ..args.clone()
                    };
                    match start::new_world(&asked, seed) {
                        Ok(world) => return Ok(Some(Opening::live(world))),
                        Err(why) => title.refuse(why),
                    }
                }
                TitleFlow::Load(save) => match saves::load(&save) {
                    Ok(world) => return Ok(Some(Opening::loaded(world, save.name))),
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

/// The save `--load` names, to open as Load does on the title screen (M2
/// design §16).
fn load_from(name: &str) -> Result<Opening, String> {
    let file = saves::named(name, files::save_folder().as_deref())
        .map_err(|why| format!("can't load {name}: {why}"))?;
    Ok(Opening::loaded(saves::load(&file)?, file.name))
}

/// How many ticks a fast-forward runs between looks at `Ctrl+C` and the
/// progress line: a fraction of a second even in a full world.
const FAST_FORWARD_CHUNK: u64 = 100;

/// How often a fast-forward's progress line is redrawn.
const PROGRESS_EVERY: Duration = Duration::from_millis(500);

/// `--fast-forward`: runs a Lab world `hours` of play at 1× with no screen,
/// a row of the report each tenth of the way and a progress line in
/// between, then saves it in the saves folder, under a name no save has
/// yet (M2 design §16). `Ctrl+C` stops it early and saves what has run; a
/// second one stops at once.
fn fast_forward(args: &Args, hours: u64) -> ExitCode {
    let Some(folder) = files::save_folder() else {
        return fail("can't tell where the saves folder is, so there's nowhere to save");
    };
    let seed = args.seed.unwrap_or_else(time_seed);
    let (world, from) = match start::fast_forward_world(args, seed, Some(&folder)) {
        Ok(found) => found,
        Err(why) => return fail(why),
    };
    let preset = start::preset_name(args);
    let loaded = args.load.is_some().then_some(from.as_str());
    println!(
        "{}",
        fast_forward::opening_line(&world, loaded, &preset, hours)
    );
    let stopping = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&stopping);
    let handled = ctrlc::set_handler(move || {
        if stop.swap(true, Ordering::SeqCst) {
            std::process::exit(130);
        }
    });
    if handled.is_ok() {
        println!("Ctrl+C stops it early and saves what has run.");
    }
    println!("\n{}", fast_forward::HEADER);

    let mut run = FastForward::new(world, hours);
    let started = Instant::now();
    let mut progress = Progress::new(stdout().is_terminal());
    while !run.done() && !stopping.load(Ordering::SeqCst) {
        match run.advance(FAST_FORWARD_CHUNK) {
            Some(row) => {
                progress.clear();
                println!("{}", row.line());
            }
            None => progress
                .show(|| fast_forward::progress_line(&run, per_second(run.ticks_run(), started))),
        }
    }
    progress.clear();

    let stopped_at = run.end();
    let stopped = stopped_at.is_some();
    if let Some(row) = stopped_at {
        println!("{}", row.line());
    }
    let ticks_run = run.ticks_run();
    let rate = per_second(ticks_run, started);
    let genes = run.genes().clone();
    let world = run.into_world();
    let name = args
        .save
        .clone()
        .unwrap_or_else(|| fast_forward::save_name(&from, ticks_run));
    let name = saves::unused_name(&folder, &name);
    if let Err(err) = saves::write(&folder, &name, &world.save()) {
        return fail(format!("couldn't save the world as {name}: {err}"));
    }
    println!(
        "\n{}",
        fast_forward::closing_line(stopped, ticks_run, started.elapsed(), rate)
    );
    println!("{}", fast_forward::saved_line(&name, &folder));
    let tables = fast_forward::tables(&name, &world, &genes);
    let mut written = Vec::new();
    for (file, table) in &tables {
        match std::fs::write(folder.join(file), table) {
            Ok(()) => written.push(file.as_str()),
            Err(err) => eprintln!("terra-sprites: couldn't write {file}: {err}"),
        }
    }
    if let Some(line) = fast_forward::tables_line(&written) {
        println!("{line}");
    }
    println!();
    for line in genes.ending() {
        println!("{line}");
    }
    ExitCode::SUCCESS
}

/// Ticks a second, for `ticks` run since `started`.
fn per_second(ticks: u64, started: Instant) -> f64 {
    let seconds = started.elapsed().as_secs_f64();
    if seconds > 0.0 {
        ticks as f64 / seconds
    } else {
        0.0
    }
}

/// A fast-forward's progress line, redrawn in place on a terminal, and
/// never shown where the report goes to a file.
struct Progress {
    shown: bool,
    on_terminal: bool,
    /// How wide the line on screen is, to rub out.
    width: usize,
    last: Instant,
}

impl Progress {
    fn new(on_terminal: bool) -> Progress {
        Progress {
            shown: false,
            on_terminal,
            width: 0,
            last: Instant::now(),
        }
    }

    /// Redraws the line from `line`, at most every `PROGRESS_EVERY`.
    fn show(&mut self, line: impl FnOnce() -> String) {
        if !self.on_terminal || self.last.elapsed() < PROGRESS_EVERY {
            return;
        }
        let line = line();
        let width = line.chars().count();
        let rub_out = " ".repeat(self.width.saturating_sub(width));
        print!("\r{line}{rub_out}");
        let _ = stdout().flush();
        (self.shown, self.width, self.last) = (true, width, Instant::now());
    }

    /// Rubs the line out, for a row of the report or the end to take its
    /// place.
    fn clear(&mut self) {
        if self.shown {
            print!("\r{}\r", " ".repeat(self.width));
            let _ = stdout().flush();
            (self.shown, self.width) = (false, 0);
        }
    }
}

/// What the frame loop runs with, besides the terminal and the world.
struct Setup<'a> {
    /// The settings the world starts with: the look, motion and square
    /// tiles, with what the player has changed of them (design §6.7, #307).
    /// `--ascii`, `--theme` and `--no-motion` give them for every world the
    /// game opens until the player changes them.
    settings: Settings,
    /// Why the saved settings weren't all taken, to say as the world
    /// opens, if the game started on it.
    unread: Option<String>,
    force_panic: bool,
    /// Where the session log goes, if anywhere (design §2.7).
    session_log: Option<&'a Path>,
}

/// Plays `opening` until the player quits or goes back to the title
/// screen, saying which, with why the world couldn't be autosaved or the
/// session log written as it was left, if it couldn't, and the settings as
/// the player left them (design §6.7, #307). A session that ends in a panic
/// still writes its replay (design §2.9), as does one the terminal failed
/// under (design §2.7).
fn run_world(
    terminal: &mut DefaultTerminal,
    opening: Opening,
    keys: &mut Keys,
    setup: Setup,
) -> io::Result<(Flow, Option<String>, Settings)> {
    let Opening {
        mut session,
        replay,
        loaded,
    } = opening;
    let session_log = setup.session_log;
    let areas = ui::areas(terminal.size()?);
    let world = session.world();
    let mut app = App::new(
        world.map(),
        setup.settings.look.theme(),
        world.seed(),
        areas,
    );
    app.use_settings(setup.settings.clone());
    if let Some(path) = files::settings_file() {
        app.set_settings_file(path);
    }
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
    if let Some(why) = &setup.unread {
        app.note_unread_settings(why.clone());
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
            let untold = (!untold.is_empty()).then(|| untold.join("; "));
            Ok((flow, untold, app.settings()))
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
        app.fit(ui::areas(terminal.size()?));
        let draw_start = Instant::now();
        terminal.draw(|frame| {
            ui::render(frame, app, session.world());
            fit_colours(frame);
        })?;
        let drawing = draw_start.elapsed();
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
        let ran = app.clock.advance(
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
        // What the debug overlay reads (design §6.1): the time the budget
        // counts, and the drawing's.
        let times = FrameTimes {
            elapsed,
            ticks: ran,
            simulating: frame_start.elapsed(),
            drawing,
        };
        app.note_frame(times, keys.reports());
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
