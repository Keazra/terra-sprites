use ratatui::buffer::Buffer;
use ratatui::layout::Size;
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, Map, World, WorldConfig};
use terra_tui::app::App;
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn generated_world() -> World {
    World::new(WorldConfig::builtin(), pack(), 7)
}

/// A world on a drawn map, using the ascii legend.
fn drawn_world(rows: &[&str]) -> World {
    let map = Map::from_ascii(rows, &pack()).expect("valid drawing");
    World::from_map(map, pack(), 7).expect("one region")
}

/// A UI for `world` sized for a `width`×`height` screen, showing seed 7.
fn app_for(world: &World, theme: Theme, width: u16, height: u16) -> App {
    let view = ui::map_view_size(Size::new(width, height), world.map());
    App::new(world.map(), theme, 7, view)
}

fn render(app: &App, world: &World, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// The screen's text, one string per row, without trailing spaces.
fn lines(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            row.trim_end().to_string()
        })
        .collect()
}

/// Renders one frame of the default world and returns the top line as text.
fn top_bar(app: &App) -> String {
    let world = generated_world();
    lines(&render(app, &world, 100, 30))[0].clone()
}

fn default_app() -> App {
    app_for(&generated_world(), Theme::cp437(), 100, 30)
}

#[test]
fn the_top_bar_shows_the_tick_and_speed() {
    let bar = top_bar(&default_app());
    assert!(bar.contains("Terra Sprites"), "{bar}");
    assert!(bar.contains("tick 0"), "{bar}");
    assert!(bar.contains("► 1x"), "{bar}");
}

#[test]
fn the_top_bar_groups_tick_digits_in_thousands() {
    let mut world = generated_world();
    for _ in 0..1_234 {
        world.step();
    }
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let bar = lines(&render(&app, &world, 100, 30))[0].clone();
    assert!(bar.contains("tick 1,234"), "{bar}");
}

#[test]
fn the_top_bar_shows_when_time_is_paused() {
    let mut app = default_app();
    app.clock.toggle_pause();
    let bar = top_bar(&app);
    assert!(bar.contains("|| paused"), "{bar}");
    assert!(!bar.contains('►'), "{bar}");
}

#[test]
fn the_top_bar_labels_slow_speeds_as_fractions() {
    let mut app = default_app();
    for expected in ["1/2x", "1/4x", "1/8x"] {
        app.clock.slower();
        let bar = top_bar(&app);
        assert!(bar.contains(&format!("► {expected}")), "{bar}");
    }
}

const SMALL_MAP: [&str; 4] = [
    "....~~~~..", //
    "..#.~==~..", //
    "..#..~~.,,", //
    "::....,,,,", //
];

#[test]
fn a_small_map_in_the_cp437_theme_is_framed_by_the_wall() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 40, 8);
    let screen = render(&app, &world, 40, 8);
    assert_eq!(
        lines(&screen),
        [
            " Terra Sprites │ tick 0 │ ► 1x │ seed 7",
            "╔═ Map ════╗",
            "║....~~~~..║",
            "║..#.~≈≈~..║",
            "║..#..~~.,,║",
            "║::....,,,,║",
            "╚══════════╝",
            " (5,2) shallow water",
        ]
    );
}

#[test]
fn a_small_map_in_the_ascii_theme_differs_only_in_its_glyphs() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::ascii(), 40, 8);
    let screen = render(&app, &world, 40, 8);
    assert_eq!(
        lines(&screen)[1..7],
        [
            "╔═ Map ════╗",
            "║....~~~~..║",
            "║..#.~==~..║",
            "║..#..~~.,,║",
            "║::....,,,,║",
            "╚══════════╝",
        ]
    );
}

#[test]
fn the_cursor_cell_is_reverse_video_and_keeps_its_glyph() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 40, 8);
    let screen = render(&app, &world, 40, 8);
    // The cursor starts at the map's centre, (5, 2): screen column 1 + 5, row 2 + 2.
    let cursor = &screen[(6, 4)];
    assert_eq!(cursor.symbol(), "~");
    assert!(cursor.modifier.contains(Modifier::REVERSED));
    assert!(!screen[(5, 4)].modifier.contains(Modifier::REVERSED));
}

#[test]
fn map_tiles_take_their_theme_colours() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 40, 8);
    let screen = render(&app, &world, 40, 8);
    assert_eq!(screen[(1, 2)].fg, Color::Green, "grass");
    assert_eq!(screen[(6, 3)].fg, Color::Blue, "deep water");
    assert_eq!(screen[(3, 3)].fg, Color::Gray, "rock");
}

#[test]
fn the_frame_is_single_where_the_map_carries_on_and_double_at_the_wall() {
    let row = ".".repeat(30);
    let world = drawn_world(&vec![row.as_str(); 20]);
    let mut app = app_for(&world, Theme::cp437(), 20, 8);
    let view = ui::map_view_size(Size::new(20, 8), world.map());
    assert_eq!(view, Size::new(18, 4));

    // In the middle of the map, every side has more map beyond it.
    assert_eq!(
        lines(&render(&app, &world, 20, 8))[1..7],
        [
            "┌─ Map ────────────┐",
            "│..................│",
            "│..................│",
            "│..................│",
            "│..................│",
            "└──────────────────┘",
        ]
    );

    // In the top-left corner, the top and left sides are the wall.
    app.move_cursor(-100, -100);
    assert_eq!(
        lines(&render(&app, &world, 20, 8))[1..7],
        [
            "╔═ Map ════════════╕",
            "║..................│",
            "║..................│",
            "║..................│",
            "║..................│",
            "╙──────────────────┘",
        ]
    );
}

#[test]
fn the_status_line_names_the_terrain_under_the_cursor() {
    let world = drawn_world(&SMALL_MAP);
    let cases = [
        ((0, 0), " (0,0) grass"),
        ((8, 2), " (8,2) dirt"),
        ((0, 3), " (0,3) sand"),
        ((4, 0), " (4,0) shallow water"),
        ((5, 1), " (5,1) deep water"),
        ((2, 1), " (2,1) rock"),
    ];
    for ((x, y), expected) in cases {
        let mut app = app_for(&world, Theme::cp437(), 40, 8);
        let (from_x, from_y) = (i32::from(app.cursor().x), i32::from(app.cursor().y));
        app.move_cursor(x - from_x, y - from_y);
        assert_eq!(lines(&render(&app, &world, 40, 8))[7], expected);
    }
}

#[test]
fn with_room_the_status_line_also_shows_the_keys() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let status = lines(&render(&app, &world, 100, 30))[29].clone();
    assert!(status.starts_with(" (5,2) shallow water"), "{status}");
    for hint in ["hjkl move", "space pause", ". step", "+/- speed", "q quit"] {
        assert!(status.contains(hint), "{hint:?} missing from {status:?}");
    }
}

#[test]
fn clicking_a_tile_puts_the_cursor_on_it_without_scrolling() {
    let row = ".".repeat(30);
    let world = drawn_world(&vec![row.as_str(); 20]);
    let screen = Size::new(20, 8);
    let mut app = app_for(&world, Theme::cp437(), 20, 8);
    // An 18×4 view whose top-left tile is (6, 8), drawn from screen cell (1, 2).
    assert_eq!(app.viewport(), terra_sim::Pos { x: 6, y: 8 });

    let tile = ui::tile_at(screen, &app, world.map(), 1, 2).expect("a tile");
    assert_eq!(tile, terra_sim::Pos { x: 6, y: 8 });
    app.place_cursor(tile);
    assert_eq!(app.cursor(), tile);
    assert_eq!(app.viewport(), terra_sim::Pos { x: 6, y: 8 }, "no scroll");

    let far_corner = ui::tile_at(screen, &app, world.map(), 18, 5);
    assert_eq!(far_corner, Some(terra_sim::Pos { x: 23, y: 11 }));
}

#[test]
fn clicks_off_the_map_views_tiles_are_not_on_a_tile() {
    let world = drawn_world(&SMALL_MAP);
    let screen = Size::new(40, 8);
    let app = app_for(&world, Theme::cp437(), 40, 8);
    // The top bar, the frame's corners and sides, the status line, and the
    // empty space beside the shrunk map view.
    for (column, row) in [(3, 0), (0, 1), (0, 3), (11, 3), (5, 6), (5, 7), (20, 3)] {
        assert_eq!(
            ui::tile_at(screen, &app, world.map(), column, row),
            None,
            "({column}, {row})"
        );
    }
}
