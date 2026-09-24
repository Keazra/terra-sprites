use ratatui::layout::Size;
use terra_sim::{DataPack, Map, Pos};
use terra_tui::app::App;
use terra_tui::theme::Theme;

/// An all-grass map.
fn grass(width: usize, height: usize) -> Map {
    let row = ".".repeat(width);
    let rows = vec![row.as_str(); height];
    Map::from_ascii(&rows, &DataPack::builtin().expect("valid pack")).expect("valid drawing")
}

fn app(map: &Map, view: Size) -> App {
    App::new(map, Theme::cp437(), 1, view)
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

#[test]
fn the_cursor_starts_at_the_centre_of_the_map() {
    assert_eq!(app(&grass(160, 96), Size::new(60, 20)).cursor(), at(80, 48));
    assert_eq!(app(&grass(5, 3), Size::new(5, 3)).cursor(), at(2, 1));
}

#[test]
fn the_cursor_moves_and_stops_at_the_wall() {
    let map = grass(40, 30);
    let view = Size::new(20, 10);
    let mut app = app(&map, view);
    app.move_cursor(3, -2, &map, view);
    assert_eq!(app.cursor(), at(23, 13));
    app.move_cursor(-100, -100, &map, view);
    assert_eq!(app.cursor(), at(0, 0));
    app.move_cursor(100, 100, &map, view);
    assert_eq!(app.cursor(), at(39, 29));
}

#[test]
fn the_viewport_starts_centred_on_the_cursor() {
    // The cursor starts at (80, 48); a 60×20 view centred on it starts at (50, 38).
    let app = app(&grass(160, 96), Size::new(60, 20));
    assert_eq!(app.viewport(), at(50, 38));
}

#[test]
fn a_map_that_fits_in_the_view_is_shown_from_its_top_left_corner() {
    let app = app(&grass(20, 10), Size::new(20, 10));
    assert_eq!(app.viewport(), at(0, 0));
}

#[test]
fn moving_the_cursor_scrolls_just_enough_to_keep_it_3_tiles_from_the_edge() {
    let map = grass(160, 96);
    let view = Size::new(20, 10);
    let mut app = app(&map, view);
    // Cursor (80, 48); the view shows x 70..=89 and y 43..=52.
    assert_eq!(app.viewport(), at(70, 43));

    app.move_cursor(6, 0, &map, view); // x 86: three tiles from the right edge
    assert_eq!(app.viewport(), at(70, 43), "no scroll yet");
    app.move_cursor(1, 0, &map, view); // x 87
    assert_eq!(app.viewport(), at(71, 43), "scrolled one tile");
    app.move_cursor(5, 0, &map, view); // x 92
    assert_eq!(app.viewport(), at(76, 43), "a Shift move scrolls five");

    app.move_cursor(-13, -2, &map, view); // (79, 46): three tiles from the left and top
    assert_eq!(app.viewport(), at(76, 43), "no scroll yet");
    app.move_cursor(-1, -1, &map, view); // (78, 45)
    assert_eq!(app.viewport(), at(75, 42));
}

#[test]
fn the_viewport_never_shows_past_the_wall() {
    let map = grass(160, 96);
    let view = Size::new(20, 10);
    let mut app = app(&map, view);
    app.move_cursor(-1000, -1000, &map, view);
    assert_eq!((app.cursor(), app.viewport()), (at(0, 0), at(0, 0)));
    app.move_cursor(1000, 1000, &map, view);
    assert_eq!((app.cursor(), app.viewport()), (at(159, 95), at(140, 86)));
}

#[test]
fn a_bigger_view_after_a_resize_still_stops_at_the_wall() {
    let map = grass(160, 96);
    let mut app = app(&map, Size::new(20, 10));
    app.move_cursor(1000, 1000, &map, Size::new(20, 10));
    assert_eq!(app.viewport(), at(140, 86));
    app.fit_viewport(&map, Size::new(40, 20));
    assert_eq!(app.viewport(), at(120, 76));
}

#[test]
fn a_smaller_view_after_a_resize_keeps_the_cursor_in_view() {
    let map = grass(160, 96);
    let mut app = app(&map, Size::new(40, 20));
    // Cursor (80, 48); the view shows x 60..=99 and y 38..=57.
    app.move_cursor(16, 6, &map, Size::new(40, 20)); // (96, 54), inside the margin
    assert_eq!(app.viewport(), at(60, 38));
    app.fit_viewport(&map, Size::new(20, 10));
    // Just enough to bring (96, 54) back into a 20×10 view.
    assert_eq!(app.viewport(), at(77, 45));
}
