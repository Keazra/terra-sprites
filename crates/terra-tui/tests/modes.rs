//! Story mode and Lab mode in the game (M2 design §16): a Story world runs
//! at 4× at most and says so when `+` asks for more, a Lab world runs up to
//! Max, and the top bar says when a world is a Lab world.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Size;
use terra_sim::{DataPack, Mode, World, WorldConfig};
use terra_tui::app::App;
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

/// A small generated world, made in `mode`.
fn world_in(mode: Mode) -> World {
    let data = pack();
    let config = WorldConfig::builtin(&data).sized(40, 32);
    World::new_in(config, data, 7, mode)
}

/// An app for `world` on a 100×30 screen.
fn app_for(world: &World) -> App {
    let areas = ui::areas(Size::new(100, 30));
    App::new(world.map(), Theme::cp437(), world.seed(), areas)
}

/// `+` pressed, or held, `times` times.
fn faster(app: &mut App, world: &World, held: bool, times: usize) {
    for _ in 0..times {
        app.apply(Action::Faster { held }, world);
    }
}

/// The top bar of a 100×30 frame.
fn top_bar(app: &App, world: &World) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..100).map(|x| buffer[(x, 0)].symbol()).collect()
}

#[test]
fn in_a_story_world_plus_stops_at_4x_and_says_so() {
    let world = world_in(Mode::Story);
    let mut app = app_for(&world);
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
    let mut app = app_for(&world);
    // A fresh press crosses 1×, as a held key won't (design §6.6).
    faster(&mut app, &world, false, 1);
    faster(&mut app, &world, true, 5);
    assert_eq!(app.clock.speed(), Speed::X4);
    assert_eq!(app.notice(), Some("4x is Story mode's fastest"));
}

#[test]
fn in_a_lab_world_plus_goes_on_to_max() {
    let world = world_in(Mode::Lab);
    let mut app = app_for(&world);
    faster(&mut app, &world, false, 6);
    assert_eq!(app.clock.speed(), Speed::Max);
    assert_eq!(app.notice(), None);
}

#[test]
fn loading_a_story_world_slows_a_faster_clock_to_4x() {
    let lab = world_in(Mode::Lab);
    let mut app = app_for(&lab);
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
    let app = app_for(&lab);
    assert!(
        top_bar(&app, &lab).contains("► 1x lab │"),
        "{}",
        top_bar(&app, &lab)
    );
    let story = world_in(Mode::Story);
    let app = app_for(&story);
    let bar = top_bar(&app, &story);
    assert!(bar.contains("► 1x │") && !bar.contains("lab"), "{bar}");
}
