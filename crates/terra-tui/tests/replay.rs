//! Replays in the game (design §2.7, §6.7): every session writes its
//! session log, a panic included; `--replay` plays one back with the world
//! closed to the player's input, pausing at its end and where it first
//! parted from its recording; and `--data` reads a folder of pack files.

mod common;
use common::screen::hint_cells;
use common::{
    WorldBuilder, app_on, at, ball_world, map_app, pack, pointer_on, render, render_on, row, run,
    scratch_folder, type_in,
};

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use ratatui::layout::{Position, Rect, Size};
use terra_sim::{Command, DataPack, Playback, World, WorldConfig};
use terra_tui::app::{AUTOSAVE_EVERY, App, CursorMode, Flow, REPLAY_REFUSAL, Ticks};
use terra_tui::files;
use terra_tui::input::Action;
use terra_tui::session::{self, Session};

/// An app on `world` showing 11×8 tiles of it from screen cell (1, 2), with
/// no panels.
fn app_for(world: &World) -> App {
    map_app(world, Rect::new(1, 2, 11, 8))
}

/// The top bar, as drawn on a 120×32 screen.
fn top_bar(app: &App, world: &World) -> String {
    row(&render_on(app, world, Size::new(120, 32)), 0)
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
    let session = played(ball_world(), 1_200);
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
    let mut session = played(ball_world(), 50);
    let mut other = ball_world();
    run(&mut other, 30);
    let loaded = World::load(&other.save()).expect("the save loads");
    session.load(loaded);
    session.step();
    let playback = replay_of(&session, "loaded");
    assert!(!playback.started_fresh());
    assert_eq!((playback.start(), playback.end()), (30, 31));
}

#[test]
fn a_replay_takes_no_commands_and_writes_no_session_log() {
    let live = played(ball_world(), 20);
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
    let mut session = Session::live(ball_world());
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
    let playback = replay_of(&played(ball_world(), 40), "start");
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
    let playback = replay_of(&played(ball_world(), 10), "refuse");
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
    app.apply(Action::left_click(pointer_on(&app, sprite.pos())), world);
    assert!(app.selection().is_some(), "selected");
    app.apply(Action::TogglePause, world);
    assert!(!app.clock.is_paused());
    app.apply(Action::Mode(CursorMode::Select), world);
    assert!(app.take_commands().is_empty(), "nothing for the world");
}

#[test]
fn a_replay_greys_the_hints_of_keys_it_would_refuse() {
    // #303: hints are greyed rather than hidden where they'd do nothing.
    let playback = replay_of(&played(ball_world(), 10), "greyed");
    let world = playback.world();
    let mut app = app_on(world);
    app.start_replay("last_session.replay", &playback);
    // Once what it plays has been said, the hints are back.
    app.animate(std::time::Duration::from_secs(5));
    let screen = render(&app, world);
    let look = app.theme.text();
    for (hint, ready) in [("Z select", true), ("X train", false), ("C grab", false)] {
        let key = hint_cells(&screen, hint)[0];
        assert_eq!(
            key.modifier.contains(look.key.add_modifier),
            ready,
            "{hint}"
        );
    }
}

#[test]
fn in_a_replay_the_palette_mutes_the_keys_it_would_refuse() {
    let playback = replay_of(&played(ball_world(), 10), "palette");
    let world = playback.world();
    let mut app = app_on(world);
    app.start_replay("last_session.replay", &playback);
    app.animate(std::time::Duration::from_secs(5));
    let look = app.theme.text();
    app.apply(Action::Palette, world);
    for (typed, key, ready) in [("name it", "r", false), ("sprite list", "l", true)] {
        type_in(&mut app, world, typed);
        let screen = render(&app, world);
        // The palette's first row, under what's typed: its key is the last
        // thing in it, before a space and the box's right edge.
        let (x, y) = (0..screen.area.height)
            .find_map(|y| {
                let text: String = (0..screen.area.width)
                    .map(|x| screen[(x, y)].symbol())
                    .collect();
                let at = text.find(&format!(" {key} │"))?;
                Some((text[..at].chars().count() as u16 + 1, y))
            })
            .unwrap_or_else(|| panic!("a row for {typed}"));
        let cell = &screen[(x, y)];
        assert_eq!(cell.symbol(), key);
        assert_eq!(
            cell.modifier.contains(look.key.add_modifier),
            ready,
            "{typed}"
        );
        let enter = hint_cells(&screen, "enter do it")[0];
        assert_eq!(
            enter.modifier.contains(look.key.add_modifier),
            ready,
            "enter, for {typed}"
        );
        for _ in typed.chars() {
            app.apply(Action::Erase, world);
        }
    }
}

#[test]
fn a_replay_pauses_at_its_end_once_and_says_so() {
    let mut session = Session::replay(replay_of(&played(ball_world(), 6), "end"));
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
    let mut playback = replay_of(&played(ball_world(), 2_100), "diverge");
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
    let mut playback = replay_of(&played(ball_world(), 10), "autosave");
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

#[test]
fn ctrl_r_takes_over_a_replay() {
    let mut keys = terra_tui::input::Keys::with_release_reporting(false);
    let key = ratatui::crossterm::event::KeyEvent::new(
        ratatui::crossterm::event::KeyCode::Char('r'),
        ratatui::crossterm::event::KeyModifiers::CONTROL,
    );
    assert_eq!(keys.action_for(key), Some(Action::TakeOver));
}

#[test]
fn taking_over_a_replay_plays_on_live_without_the_rest_of_the_recording() {
    // The recording puts the ball down at tick 800.
    let mut live = played(ball_world(), 800);
    live.submit(vec![Command::PutDown { tile: at(5, 5) }]);
    for _ in 0..400 {
        live.step();
    }
    assert!(live.world().cursor().holds().is_none());
    let mut session = Session::replay(replay_of(&live, "take-over"));
    for _ in 0..500 {
        session.step();
    }
    session.take_over();
    assert!(session.playback().is_none(), "it's live");
    for _ in 0..400 {
        session.step();
    }
    assert!(
        session.world().cursor().holds().is_some(),
        "the recorded putting down never played"
    );
    // The player's commands change the world now.
    let sprite = session.world().sprites().next().expect("a sprite").id();
    session.submit(vec![Command::Rename {
        sprite,
        name: "Mine".into(),
    }]);
    session.step();
    let named = session.world().sprite(sprite).expect("the sprite");
    assert_eq!(named.name(), Some("Mine"));

    // Its session log starts where it was taken over, and replays it.
    let mut playback = replay_of(&session, "taken-over");
    assert!(!playback.started_fresh());
    assert_eq!((playback.start(), playback.end()), (500, 901));
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), session.world().state_hash());
}

#[test]
fn taking_over_a_live_session_changes_nothing() {
    let mut session = played(ball_world(), 5);
    let before = session.world().state_hash();
    session.take_over();
    assert!(session.playback().is_none());
    assert_eq!(session.world().state_hash(), before);
}

#[test]
fn after_taking_over_the_world_is_the_player_s() {
    let mut playback = replay_of(&played(ball_world(), 30), "app-take-over");
    let mut app = replaying(&playback);
    for _ in 0..5 {
        playback.step();
    }
    let world = playback.world();
    assert_eq!(app.apply(Action::TakeOver, world), Flow::Continue);
    assert!(!app.replaying());
    assert!(app.take_taken_over(), "the frame loop is told");
    assert!(!app.take_taken_over(), "once");
    let notice = app.notice().expect("a notice");
    assert!(
        notice.contains("You took over the replay at tick 5: it's your world now"),
        "{notice}"
    );
    // The recorded Cursor holds the ball, and it's the player's now.
    assert_eq!(app.mode(), CursorMode::Grab);
    assert!(app.grip(world).is_some());
    app.apply(Action::Rename, world);
    assert_ne!(app.refusal(), Some(REPLAY_REFUSAL));
    assert!(!top_bar(&app, world).contains("replay"));
}

#[test]
fn f3_shows_the_debug_overlay_in_a_replay_too() {
    let session = played(ball_world(), 10);
    let playback = replay_of(&session, "debug-overlay");
    let mut app = replaying(&playback);
    let flow = app.apply(Action::DebugOverlay, playback.world());
    assert_eq!(flow, Flow::Continue);
    assert!(
        app.debug_overlay(),
        "it only reads how the game runs (#311)"
    );
    assert_eq!(app.refusal(), None);
}

#[test]
fn in_a_replay_the_wheel_on_the_debug_overlay_is_not_refused() {
    // The overlay's box takes the wheel, which never reaches the map
    // beneath it, where a replay refuses it (design §6.1).
    // A map bigger than the view, so the box is over its tiles.
    let world = WorldBuilder::grass(60, 30)
        .objects(&[(at(5, 5), "ball")])
        .starters(&[at(2, 2)])
        .build();
    let playback = replay_of(&played(world, 10), "overlay-wheel");
    let world = playback.world();
    let mut app = app_on(world);
    app.start_replay("last_session.replay", &playback);
    app.apply(Action::DebugOverlay, world);
    let screen = render(&app, world);
    let title = (0..screen.area.height)
        .find_map(|y| {
            let row = row(&screen, y);
            let x = row.find(" Debug ")?;
            Some(Position::new(row[..x].chars().count() as u16, y))
        })
        .expect("the overlay's box");
    let wheel = |at| Action::Wheel { at, notches: 1 };
    let on_box = Position::new(title.x + 3, title.y + 2);
    assert!(app.tile_at(on_box).is_some(), "a tile beneath it");
    app.apply(wheel(on_box), world);
    assert_eq!(app.refusal(), None, "on its box");
    app.apply(wheel(Position::new(title.x - 3, title.y + 2)), world);
    assert_eq!(app.refusal(), Some(REPLAY_REFUSAL), "on the map");
}

#[test]
fn ctrl_r_outside_a_replay_does_nothing() {
    let world = ball_world();
    let mut app = app_for(&world);
    app.apply(Action::TakeOver, &world);
    assert!(!app.take_taken_over());
    assert_eq!(app.notice(), None);
    assert_eq!(app.mode(), CursorMode::Select);
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

#[test]
fn a_data_folder_with_something_not_a_file_in_a_file_s_place_is_refused_naming_it() {
    let dir = data_folder("not-a-file", &[]);
    std::fs::create_dir_all(dir.join("objects.ron")).unwrap();
    let err = files::data_folder_files(&dir).expect_err("refused");
    assert!(
        err.contains("objects.ron") && err.contains("isn't a file"),
        "{err}"
    );
}

#[test]
fn a_replay_names_the_sprites_its_world_starts_with_before_it_plays() {
    // A replay that starts from a save starts with the save's names (#137).
    let mut world = ball_world();
    let sprite = world.sprites().next().expect("a sprite").id();
    world.submit(Command::Rename {
        sprite,
        name: "Mira".into(),
    });
    world.step();
    let live = played(world, 5);
    let playback = replay_of(&live, "names");
    assert!(!playback.started_fresh());
    let app = replaying(&playback);
    assert_eq!(app.names().label(sprite), format!("Mira #{}", sprite.0));
}

/// The status line, as drawn on a 120×32 screen.
fn status_line(app: &App, world: &World) -> String {
    row(&render_on(app, world, Size::new(120, 32)), 31)
}

#[test]
fn in_a_replay_the_recorded_cursor_isn_t_drawn_and_holds_and_leads_nothing() {
    // Design §2.7: the viewer has a Cursor of their own; the recorded one
    // is the world's, and isn't shown.
    let held = played(ball_world(), 5);
    assert!(held.world().cursor().holds().is_some(), "it holds the ball");
    let mut leading = Session::live(ball_world());
    let sprite = leading.world().sprites().next().expect("a sprite").id();
    leading.submit(vec![
        Command::TakeHold { sprite },
        Command::MoveCursor { tile: at(9, 6) },
    ]);
    for _ in 0..5 {
        leading.step();
    }
    assert_eq!(leading.world().cursor().leads(), Some(sprite));
    for (session, name) in [(held, "held"), (leading, "led")] {
        let mut replay = Session::replay(replay_of(&session, name));
        let mut app = replaying(replay.playback().expect("a replay"));
        let cursor = app.cursor();
        // As the frame loop does it.
        let mut ticks = Ticks::default();
        while replay.world().tick() < 5 {
            let events = replay.step();
            ticks.note(events, replay.world());
        }
        app.take_in(ticks, replay.world());
        app.take_in_replay(replay.playback().expect("a replay"));
        let world = replay.world();
        assert_eq!(app.grip(world), None, "{name}");
        assert_eq!(app.leash(), None, "{name}");
        assert_eq!(app.cursor(), cursor, "the viewer's own, unmoved: {name}");
        let status = status_line(&app, world);
        assert!(
            !status.contains("holding") && !status.contains("leading"),
            "{name}: {status}"
        );
    }
}
