//! Replays in the game (design §2.7, §6.7): every session writes its
//! session log, a panic included; `--replay` plays one back with the world
//! closed to the player's input, pausing at its end and where it first
//! parted from its recording; and `--data` reads a folder of pack files.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use terra_sim::{Command, DataPack, Map, Playback, Pos, Scenario, World, WorldConfig};
use terra_tui::app::{AUTOSAVE_EVERY, App, Areas, CursorMode, Flow, REPLAY_REFUSAL, Ticks};
use terra_tui::files;
use terra_tui::input::{Action, Button};
use terra_tui::session::{self, Session};
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A grass world with two starter sprites and a ball.
fn world() -> World {
    let rows = vec!["..........."; 8];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[(at(5, 5), "ball")],
        sprites: &[(at(2, 2), None), (at(8, 3), None)],
        scripted: &[],
    };
    World::from_scenario(scenario, pack(), 7).expect("valid scenario")
}

/// A fresh, empty folder.
fn scratch_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terra-replay-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn app_for(world: &World) -> App {
    let areas = Areas {
        tiles: Rect::new(1, 2, 11, 8),
        inspector: None,
        event_log: None,
        overlay: None,
    };
    App::new(world.map(), Theme::cp437(), world.seed(), areas)
}

/// The top bar, as drawn on a 120×32 screen.
fn top_bar(app: &App, world: &World) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).expect("a test terminal");
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .expect("drawn");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, 0)].symbol().to_string())
        .collect()
}

/// A live session on `world` that has run `ticks` ticks, the Cursor
/// picking up the ball on the way.
fn played(world: World, ticks: u64) -> Session {
    let mut session = Session::live(world);
    let ball = session.world().objects().next().expect("the ball").id();
    session.submit(vec![Command::PickUp { item: ball }]);
    for _ in 0..ticks {
        session.step();
    }
    session
}

/// The playback of the replay `session` writes.
fn replay_of(session: &Session, name: &str) -> Playback {
    let log = scratch_folder(name).join(files::SESSION_LOG);
    session.write_session_log(&log).expect("the log is written");
    Playback::new(&std::fs::read(&log).expect("the log is there")).expect("it plays")
}

/// The app following `playback` as `--replay` starts it.
fn replaying(playback: &Playback) -> App {
    let mut app = app_for(playback.world());
    app.start_replay("last_session.replay", playback);
    app
}

#[test]
fn a_live_session_writes_a_session_log_that_plays_back_to_the_same_world() {
    let session = played(world(), 1_200);
    let mut playback = replay_of(&session, "live");
    assert_eq!(playback.end(), 1_200);
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), session.world().state_hash());
}

#[test]
fn loading_a_save_starts_the_session_log_afresh_from_it() {
    let mut session = played(world(), 50);
    let mut other = world();
    for _ in 0..30 {
        other.step();
    }
    let loaded = World::load(&other.save()).expect("the save loads");
    session.load(loaded);
    session.step();
    let playback = replay_of(&session, "loaded");
    assert!(!playback.started_fresh());
    assert_eq!((playback.start(), playback.end()), (30, 31));
}

#[test]
fn a_replay_takes_no_commands_and_writes_no_session_log() {
    let live = played(world(), 20);
    let mut replay = Session::replay(replay_of(&live, "no-commands"));
    let before = replay.world().state_hash();
    let sprite = replay.world().sprites().next().expect("a sprite").id();
    replay.submit(vec![Command::Rename {
        sprite,
        name: "Ignored".into(),
    }]);
    assert_eq!(replay.world().state_hash(), before, "nothing was submitted");

    let log = scratch_folder("no-log").join(files::SESSION_LOG);
    replay.write_session_log(&log).expect("nothing to write");
    assert!(!log.exists(), "a replay never overwrites the log");
    assert!(replay.playback().is_some());
}

#[test]
fn a_session_that_ends_in_a_panic_still_writes_its_session_log() {
    let mut session = Session::live(world());
    let log = scratch_folder("panic").join(files::SESSION_LOG);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        session::writing_session_log_on_panic(&mut session, Some(&log), |session| {
            for _ in 0..1_100 {
                session.step();
            }
            panic!("something broke");
        })
    }));
    assert!(panicked.is_err(), "the panic carries on");
    let mut playback =
        Playback::new(&std::fs::read(&log).expect("the log was written")).expect("a valid replay");
    assert_eq!(playback.end(), 1_100);
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), session.world().state_hash());
}

#[test]
fn a_replay_starts_paused_saying_what_it_plays() {
    let playback = replay_of(&played(world(), 40), "start");
    let app = replaying(&playback);
    assert!(app.clock.is_paused());
    assert!(app.replaying());
    let notice = app.notice().expect("a notice");
    assert!(
        notice.contains("Replaying last_session.replay: from a save at tick 0 to tick 40"),
        "{notice}"
    );
    let bar = top_bar(&app, playback.world());
    assert!(bar.contains("│ replay to tick 40"), "{bar}");
    assert!(!bar.contains("saved"), "{bar}");
}

#[test]
fn a_new_world_replays_from_its_seed() {
    let data = pack();
    let world = World::new(WorldConfig::builtin(&data), data, 9);
    let session = played(world, 5);
    let playback = replay_of(&session, "fresh");
    let app = replaying(&playback);
    let notice = app.notice().expect("a notice");
    assert!(notice.contains("from a new world to tick 5"), "{notice}");
}

#[test]
fn a_replay_refuses_whatever_would_change_the_world() {
    let playback = replay_of(&played(world(), 10), "refuse");
    let world = playback.world();
    let mut app = replaying(&playback);
    for action in [
        Action::Mode(CursorMode::Train),
        Action::Mode(CursorMode::Grab),
        Action::Rename,
        Action::ToggleVisible,
        Action::Quickload,
        Action::OpenSaves,
        Action::Wheel {
            at: ratatui::layout::Position::new(4, 4),
            notches: 1,
        },
    ] {
        assert_eq!(app.apply(action, world), Flow::Continue);
        assert_eq!(app.refusal(), Some(REPLAY_REFUSAL), "{action:?}");
        assert_eq!(app.mode(), CursorMode::Select, "{action:?}");
    }
    // Clicks in Select only select; time and the view still work.
    let sprite = world.sprites().next().expect("a sprite");
    let pos = sprite.pos();
    // The pointer points a cell down and right of the tile, at the
    // Cursor's corner.
    let cell = app.cell_of(pos).expect("in view");
    let cell = ratatui::layout::Position::new(cell.x + 1, cell.y + 1);
    app.apply(
        Action::Click {
            at: cell,
            button: Button::Left,
            amplified: false,
        },
        world,
    );
    assert!(app.selection().is_some(), "selected");
    app.apply(Action::TogglePause, world);
    assert!(!app.clock.is_paused());
    app.apply(Action::Mode(CursorMode::Select), world);
    assert!(app.take_commands().is_empty(), "nothing for the world");
}

#[test]
fn a_replay_pauses_at_its_end_once_and_says_so() {
    let mut session = Session::replay(replay_of(&played(world(), 6), "end"));
    let mut app = replaying(session.playback().unwrap());
    app.apply(Action::TogglePause, session.world());
    let mut ticks = Ticks::default();
    let stops = app.replay_stops().expect("a replay");
    let mut ran = 0;
    loop {
        let events = session.step();
        ticks.note(events, session.world());
        ran += 1;
        if stops.at(session.playback().unwrap()) {
            break;
        }
    }
    assert_eq!(ran, 6, "it stops at the end");
    app.take_in(ticks, session.world());
    app.take_in_replay(session.playback().unwrap());
    assert!(app.clock.is_paused());
    let notice = app.notice().expect("a notice");
    assert!(
        notice.contains("The replay ends here, at tick 6"),
        "{notice}"
    );
    assert!(top_bar(&app, session.world()).contains("│ replay ended"));

    // Time runs on past it, without stopping again.
    app.apply(Action::TogglePause, session.world());
    session.step();
    let stops = app.replay_stops().expect("a replay");
    assert!(!stops.at(session.playback().unwrap()));
    app.take_in_replay(session.playback().unwrap());
    assert!(!app.clock.is_paused());
}

#[test]
fn a_replay_that_parts_from_its_recording_pauses_and_says_where() {
    let mut playback = replay_of(&played(world(), 2_100), "diverge");
    let mut app = replaying(&playback);
    app.apply(Action::TogglePause, playback.world());
    while playback.world().tick() < 1_200 {
        playback.step();
    }
    let sprite = playback.world().sprites().next().expect("a sprite").id();
    playback.inject(Command::Rename {
        sprite,
        name: "Stray".into(),
    });
    let stops = app.replay_stops().expect("a replay");
    while !stops.at(&playback) {
        playback.step();
    }
    assert_eq!(playback.world().tick(), 2_000);
    app.take_in_replay(&playback);
    assert!(app.clock.is_paused());
    let notice = app.notice().expect("a notice");
    assert!(
        notice.contains("parted from its recording between ticks 1,000 and 2,000"),
        "{notice}"
    );
    assert!(top_bar(&app, playback.world()).contains("│ replay diverged by tick 2,000"));
}

#[test]
fn a_replay_never_autosaves() {
    let mut playback = replay_of(&played(world(), 10), "autosave");
    let folder = scratch_folder("autosave-saves");
    let mut app = replaying(&playback);
    // It has run, so a live world would autosave.
    for _ in 0..5 {
        playback.step();
    }
    app.set_save_folder(folder.clone());
    app.apply(Action::TogglePause, playback.world());
    app.animate(AUTOSAVE_EVERY);
    assert!(app.autosave_if_due(playback.world()));
    app.autosave(playback.world());
    assert_eq!(
        std::fs::read_dir(&folder).unwrap().count(),
        0,
        "no autosaves"
    );
}

/// Writes `files`, each `(path within the folder, text)`, in a fresh
/// folder.
fn data_folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = scratch_folder(name);
    for (path, text) in files {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    dir
}

fn builtin_file(path: &str) -> &'static str {
    DataPack::builtin_sources()
        .iter()
        .find(|(p, _)| *p == path)
        .expect("a built-in file")
        .1
}

#[test]
fn a_data_folder_replaces_the_files_it_has_and_keeps_the_rest() {
    let renamed = builtin_file("pack.ron").replace("\"core\"", "\"Bigger\"");
    let preset = "(width: 64, height: 40)";
    let dir = data_folder(
        "data",
        &[("pack.ron", &renamed), ("presets/default.ron", preset)],
    );
    let files::DataFolder {
        sources,
        preset: default_preset,
    } = files::data_folder_files(&dir).expect("readable");
    let file = |path: &str| {
        sources
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, text)| text.as_str())
    };
    assert_eq!(file("pack.ron"), Some(renamed.as_str()));
    assert_eq!(file("objects.ron"), Some(builtin_file("objects.ron")));
    assert_eq!(sources.len(), DataPack::builtin_sources().len());
    assert_eq!(default_preset.as_deref(), Some(preset));

    let borrowed: Vec<(&str, &str)> = sources
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    assert_eq!(DataPack::from_sources(&borrowed).unwrap().name(), "Bigger");
}

#[test]
fn a_data_folder_that_isnt_one_is_refused_naming_it() {
    let missing = Path::new("no/such/folder");
    let err = files::data_folder_files(missing).expect_err("refused");
    assert!(err.contains("no/such/folder"), "{err}");
    let preset = files::data_folder_files(&data_folder("empty", &[]))
        .expect("readable")
        .preset;
    assert_eq!(preset, None, "no preset of its own");
}
