use ratatui::layout::{Position, Rect};
use terra_sim::{DataPack, Map, Pos};
use terra_tui::app::{App, Flow, Screen};
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::theme::Theme;

/// An all-grass map.
fn grass(width: usize, height: usize) -> Map {
    let row = ".".repeat(width);
    let rows = vec![row.as_str(); height];
    Map::from_ascii(&rows, &DataPack::builtin().expect("valid pack")).expect("valid drawing")
}

/// Where the map view draws its tiles on screen: `width`×`height` tiles from cell (1, 2).
fn tile_area(width: u16, height: u16) -> Rect {
    Rect::new(1, 2, width, height)
}

fn app(map: &Map, tile_area: Rect) -> App {
    App::new(map, Theme::cp437(), 1, tile_area)
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn scroll(app: &mut App, dx: i32, dy: i32) {
    assert_eq!(app.apply(Action::Scroll { dx, dy }), Flow::Continue);
}

fn point(app: &mut App, column: u16, row: u16) {
    assert_eq!(
        app.apply(Action::Point(Position::new(column, row))),
        Flow::Continue
    );
}

#[test]
fn the_cursor_starts_at_the_centre_of_the_map() {
    assert_eq!(app(&grass(160, 96), tile_area(60, 20)).cursor(), at(80, 48));
    assert_eq!(app(&grass(5, 3), tile_area(5, 3)).cursor(), at(2, 1));
}

#[test]
fn the_viewport_starts_centred_on_the_cursor() {
    // The cursor starts at (80, 48); a 60×20 view centred on it starts at (50, 38).
    assert_eq!(
        app(&grass(160, 96), tile_area(60, 20)).viewport(),
        at(50, 38)
    );
}

#[test]
fn a_map_that_fits_in_the_view_is_shown_from_its_top_left_corner() {
    assert_eq!(app(&grass(20, 10), tile_area(20, 10)).viewport(), at(0, 0));
}

#[test]
fn scrolling_moves_the_viewport_and_stops_at_the_wall() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    assert_eq!(app.viewport(), at(70, 43));
    scroll(&mut app, 1, 0);
    assert_eq!(app.viewport(), at(71, 43));
    scroll(&mut app, -5, 2);
    assert_eq!(app.viewport(), at(66, 45));
    scroll(&mut app, -1000, -1000);
    assert_eq!(app.viewport(), at(0, 0));
    scroll(&mut app, 1000, 1000);
    assert_eq!(app.viewport(), at(140, 86));
}

#[test]
fn pointing_at_a_tile_puts_the_cursor_on_it() {
    // The view shows tiles (70, 43) to (89, 52), drawn from screen cell (1, 2).
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    point(&mut app, 1, 2);
    assert_eq!(app.cursor(), at(70, 43));
    point(&mut app, 20, 11);
    assert_eq!(app.cursor(), at(89, 52));
}

#[test]
fn pointing_outside_the_map_view_leaves_the_cursor_on_its_last_tile() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    point(&mut app, 5, 5);
    assert_eq!(app.cursor(), at(74, 46));
    for (column, row) in [(0, 5), (21, 5), (5, 1), (5, 12)] {
        point(&mut app, column, row);
        assert_eq!(app.cursor(), at(74, 46), "pointer at ({column}, {row})");
    }
}

#[test]
fn scrolling_under_a_still_pointer_moves_the_cursor_with_the_map() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    point(&mut app, 11, 7);
    assert_eq!(app.cursor(), at(80, 48));
    scroll(&mut app, 3, -1);
    assert_eq!(app.cursor(), at(83, 47));
}

#[test]
fn with_the_pointer_off_the_map_scrolling_leaves_the_cursor_on_its_tile() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    scroll(&mut app, 3, 0); // no pointer yet
    assert_eq!(app.cursor(), at(80, 48));
    point(&mut app, 11, 7); // over (83, 48) now
    point(&mut app, 0, 7); // the pointer leaves the map view
    scroll(&mut app, 3, 0);
    assert_eq!(app.cursor(), at(83, 48));
}

#[test]
fn a_click_puts_the_cursor_on_a_tile_without_scrolling() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    assert_eq!(
        app.apply(Action::Click(Position::new(1, 2))),
        Flow::Continue
    );
    assert_eq!(app.cursor(), at(70, 43));
    assert_eq!(app.viewport(), at(70, 43));
}

#[test]
fn a_bigger_view_after_a_resize_still_stops_at_the_wall() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    scroll(&mut app, 1000, 1000);
    assert_eq!(app.viewport(), at(140, 86));
    app.fit_viewport(tile_area(40, 20));
    assert_eq!(app.viewport(), at(120, 76));
}

#[test]
fn time_actions_reach_the_clock() {
    let mut app = app(&grass(40, 30), tile_area(20, 10));
    app.apply(Action::Faster { held: false });
    assert_eq!(app.clock.speed(), Speed::X2);
    app.apply(Action::TogglePause);
    assert!(app.clock.is_paused());
}

#[test]
fn escape_asks_to_quit_and_y_quits() {
    let mut app = app(&grass(40, 30), tile_area(20, 10));
    assert_eq!(app.apply(Action::Back), Flow::Continue);
    assert_eq!(app.screen(), Screen::QuitPrompt);
    assert_eq!(app.apply(Action::Confirm), Flow::Quit);
}

#[test]
fn a_second_escape_quits() {
    let mut app = app(&grass(40, 30), tile_area(20, 10));
    app.apply(Action::Back);
    assert_eq!(app.apply(Action::Back), Flow::Quit);
}

#[test]
fn any_other_key_cancels_the_quit_prompt_and_does_nothing_else() {
    let mut app = app(&grass(160, 96), tile_area(20, 10));
    for key in [
        Action::Dismiss,
        Action::TogglePause,
        Action::Scroll { dx: 1, dy: 0 },
    ] {
        app.apply(Action::Back);
        assert_eq!(app.apply(key), Flow::Continue, "{key:?}");
        assert_eq!(app.screen(), Screen::Normal, "{key:?} should cancel");
    }
    assert!(!app.clock.is_paused(), "space only cancelled the prompt");
    assert_eq!(
        app.viewport(),
        at(70, 43),
        "the scroll key only cancelled the prompt"
    );
    assert_eq!(
        app.apply(Action::Confirm),
        Flow::Continue,
        "y with no prompt open"
    );
}

#[test]
fn moving_the_mouse_leaves_the_quit_prompt_open() {
    let mut app = app(&grass(40, 30), tile_area(20, 10));
    app.apply(Action::Back);
    point(&mut app, 3, 3);
    assert_eq!(app.screen(), Screen::QuitPrompt);
}

#[test]
fn ctrl_c_quits_at_once() {
    let mut app = app(&grass(40, 30), tile_area(20, 10));
    assert_eq!(app.apply(Action::Quit), Flow::Quit);
}
