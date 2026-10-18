//! Story mode and Lab mode in the game (M2 design §16): a Story world runs
//! at 4× at most and says so when `+` asks for more, in a replay too, a Lab
//! world runs up to Max, and the top bar says when a world is a Lab world.

mod common;
use common::{app_on, pack, render, row, scratch_folder};

use terra_sim::{Mode, Playback, World, WorldConfig};
use terra_tui::app::App;
use terra_tui::clock::Speed;
use terra_tui::files;
use terra_tui::input::Action;
use terra_tui::session::Session;

/// A small generated world, made in `mode`.
fn world_in(mode: Mode) -> World {
    let data = pack();
    let config = WorldConfig::builtin(&data).sized(40, 32);
    World::new_in(config, data, 7, mode)
}

/// `+` pressed, or held, `times` times.
fn faster(app: &mut App, world: &World, held: bool, times: usize) {
    for _ in 0..times {
        app.apply(Action::Faster { held }, world);
    }
}

/// The top bar of a 100×30 frame.
fn top_bar(app: &App, world: &World) -> String {
    row(&render(app, world), 0)
}

#[test]
fn in_a_story_world_plus_stops_at_4x_and_says_so() {
    let world = world_in(Mode::Story);
    let mut app = app_on(&world);
    faster(&mut app, &world, false, 2);
    assert_eq!(app.clock.speed(), Speed::X4);
    assert_eq!(app.notice(), None, "nothing said on the way up");
    faster(&mut app, &world, false, 1);
    assert_eq!(app.clock.speed(), Speed::X4);
    assert_eq!(app.notice(), Some("4x is Story mode's fastest"));
}

#[test]
fn holding_plus_in_a_story_world_stops_at_4x_too() {
    let world = world_in(Mode::Story);
    let mut app = app_on(&world);
    // A fresh press crosses 1×, as a held key won't (design §6.6).
    faster(&mut app, &world, false, 1);
    faster(&mut app, &world, true, 5);
    assert_eq!(app.clock.speed(), Speed::X4);
    assert_eq!(app.notice(), Some("4x is Story mode's fastest"));
}

#[test]
fn the_population_graph_keeps_a_story_world_to_4x() {
    // Time runs as usual with the graph open (M2 design §14).
    let world = world_in(Mode::Story);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    faster(&mut app, &world, false, 3);
    assert_eq!(app.clock.speed(), Speed::X4);
    faster(&mut app, &world, true, 3);
    assert_eq!(app.clock.speed(), Speed::X4);
}

/// The replay of `world`, a new world, run for 20 ticks, as `--replay`
/// plays it.
fn replayed(world: World, name: &str) -> (Playback, App) {
    let mut session = Session::live(world);
    for _ in 0..20 {
        session.step();
    }
    let folder = scratch_folder(name);
    let log = folder.join(files::SESSION_LOG);
    session.write_session_log(&log).expect("the log is written");
    let playback = Playback::new(&std::fs::read(&log).expect("the log")).expect("it plays");
    let _ = std::fs::remove_dir_all(&folder);
    let mut app = app_on(playback.world());
    app.start_replay("last_session.replay", &playback);
    (playback, app)
}

#[test]
fn a_replay_plays_at_its_worlds_speeds() {
    // M2 design §16.2: the cap holds in a replay of a Story world.
    let (playback, mut app) = replayed(world_in(Mode::Story), "story");
    assert_eq!(playback.world().mode(), Mode::Story);
    faster(&mut app, playback.world(), false, 6);
    assert_eq!(app.clock.speed(), Speed::X4);
    assert_eq!(app.notice(), Some("4x is Story mode's fastest"));
    let (playback, mut app) = replayed(world_in(Mode::Lab), "lab");
    assert_eq!(playback.world().mode(), Mode::Lab);
    faster(&mut app, playback.world(), false, 6);
    assert_eq!(app.clock.speed(), Speed::Max);
}

#[test]
fn in_a_lab_world_plus_goes_on_to_max() {
    let world = world_in(Mode::Lab);
    let mut app = app_on(&world);
    faster(&mut app, &world, false, 6);
    assert_eq!(app.clock.speed(), Speed::Max);
    assert_eq!(app.notice(), None);
}

#[test]
fn loading_a_story_world_slows_a_faster_clock_to_4x() {
    let lab = world_in(Mode::Lab);
    let mut app = app_on(&lab);
    faster(&mut app, &lab, false, 4);
    assert_eq!(app.clock.speed(), Speed::X16);
    app.load_from_title(world_in(Mode::Story), "story".into());
    app.take_loaded().expect("the world loaded");
    assert_eq!(app.clock.speed(), Speed::X4);
    assert!(app.clock.is_paused(), "a load starts paused");
}

#[test]
fn a_lab_worlds_top_bar_says_lab_after_the_speed() {
    let lab = world_in(Mode::Lab);
    let app = app_on(&lab);
    assert!(
        top_bar(&app, &lab).contains("► 1x lab │"),
        "{}",
        top_bar(&app, &lab)
    );
    let story = world_in(Mode::Story);
    let app = app_on(&story);
    let bar = top_bar(&app, &story);
    assert!(bar.contains("► 1x │") && !bar.contains("lab"), "{bar}");
}

#[test]
fn a_paused_lab_worlds_top_bar_says_lab_after_the_speed_too() {
    // M2 design §16.1: `|| paused 4x lab`.
    let lab = world_in(Mode::Lab);
    let mut app = app_on(&lab);
    faster(&mut app, &lab, false, 2);
    app.apply(Action::TogglePause, &lab);
    let bar = top_bar(&app, &lab);
    assert!(bar.contains("|| paused 4x lab │"), "{bar}");
}
