use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, Map, World, WorldConfig};
use terra_tui::app::App;
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn generated_world() -> World {
    World::new(WorldConfig::builtin(&pack()), pack(), 7)
}

/// A world on a drawn map, using the ascii legend.
fn drawn_world(rows: &[&str]) -> World {
    let map = Map::from_ascii(rows, &pack()).expect("valid drawing");
    World::from_map(map, pack(), 7).expect("one region")
}

/// A UI for `world` sized for a `width`×`height` screen, showing seed 7.
fn app_for(world: &World, theme: Theme, width: u16, height: u16) -> App {
    let tiles = ui::tile_area(Size::new(width, height), world.map());
    App::new(world.map(), theme, 7, tiles)
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
fn the_map_view_draws_its_tiles_inside_its_border() {
    let small = drawn_world(&SMALL_MAP);
    assert_eq!(
        ui::tile_area(Size::new(40, 8), small.map()),
        Rect::new(1, 2, 10, 4),
        "a small map gets a shrunk map view"
    );
    let row = ".".repeat(30);
    let big = drawn_world(&vec![row.as_str(); 20]);
    assert_eq!(
        ui::tile_area(Size::new(20, 8), big.map()),
        Rect::new(1, 2, 18, 4),
        "a big map fills the space between the top bar and the status line"
    );
}

#[test]
fn a_small_map_in_the_cp437_theme_shows_the_select_cursor() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 40, 8);
    let screen = render(&app, &world, 40, 8);
    // The cursor starts at the map's centre, (5, 2), and covers (4, 1) to (6, 3).
    assert_eq!(
        lines(&screen),
        [
            " Terra Sprites │ tick 0 │ ► 1x │ seed 7",
            "╔═ Map ════╗",
            "║....~~~~..║",
            "║..#.♦↓·~..║",
            "║..#.→~←.,,║",
            "║::..·↑♦,,,║",
            "╚══════════╝",
            " (5,2) shallow water │ SELECT",
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
            "║..#.Sv-~..║",
            "║..#.>~<.,,║",
            "║::..-^S,,,║",
            "╚══════════╝",
        ]
    );
}

#[test]
fn the_cursor_centre_is_reverse_video_and_its_marks_take_the_modes_colour() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 40, 8);
    let screen = render(&app, &world, 40, 8);
    // The cursor's centre, (5, 2), is at screen column 1 + 5, row 2 + 2.
    let centre = &screen[(6, 4)];
    assert_eq!(centre.symbol(), "~");
    assert!(centre.modifier.contains(Modifier::REVERSED));
    assert_eq!(centre.fg, Color::Cyan, "the tile keeps its own colour");
    for (column, row) in [
        (5, 3),
        (6, 3),
        (7, 3),
        (5, 4),
        (7, 4),
        (5, 5),
        (6, 5),
        (7, 5),
    ] {
        let piece = &screen[(column, row)];
        assert_eq!(piece.fg, Color::White, "Select is white: ({column}, {row})");
        assert!(!piece.modifier.contains(Modifier::REVERSED));
    }
}

#[test]
fn map_tiles_take_their_theme_colours() {
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 40, 8);
    // Point at the top-left tile, so the cursor sits clear of the tiles checked.
    app.apply(Action::Point(Position::new(1, 2)));
    let screen = render(&app, &world, 40, 8);
    assert_eq!(screen[(3, 2)].fg, Color::Green, "grass");
    assert_eq!(screen[(6, 3)].fg, Color::Blue, "deep water");
    assert_eq!(screen[(3, 3)].fg, Color::Gray, "rock");
}

#[test]
fn the_border_is_single_where_the_map_carries_on_and_double_at_the_wall() {
    let row = ".".repeat(30);
    let world = drawn_world(&vec![row.as_str(); 20]);
    let mut app = app_for(&world, Theme::cp437(), 20, 8);

    // In the middle of the map, every side has more map beyond it. The cursor
    // starts at (15, 10), in the middle of the view.
    assert_eq!(
        lines(&render(&app, &world, 20, 8))[1..7],
        [
            "┌─ Map ────────────┐",
            "│..................│",
            "│........♦↓·.......│",
            "│........→.←.......│",
            "│........·↑♦.......│",
            "└──────────────────┘",
        ]
    );

    // Scrolled to the top-left corner, the top and left sides are the wall.
    // The cursor stayed on (15, 10), which is now out of view.
    app.apply(Action::Scroll { dx: -100, dy: -100 });
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
fn the_cursor_is_clipped_at_the_edge_of_the_map_view() {
    let row = ".".repeat(30);
    let world = drawn_world(&vec![row.as_str(); 20]);
    let mut app = app_for(&world, Theme::cp437(), 20, 8);
    app.apply(Action::Point(Position::new(1, 2))); // the view's top-left tile
    assert_eq!(
        lines(&render(&app, &world, 20, 8))[1..7],
        [
            "┌─ Map ────────────┐",
            "│.←................│",
            "│↑♦................│",
            "│..................│",
            "│..................│",
            "└──────────────────┘",
        ]
    );
}

#[test]
fn a_cursor_whose_target_is_out_of_view_is_not_drawn_at_all() {
    let row = ".".repeat(30);
    let world = drawn_world(&vec![row.as_str(); 20]);
    let mut app = app_for(&world, Theme::cp437(), 20, 8);
    // Put the cursor on the view's rightmost column, (23, 10), then move the
    // pointer off the map and scroll left, so the cursor's tile leaves the view.
    app.apply(Action::Point(Position::new(18, 4)));
    assert_eq!(app.cursor(), terra_sim::Pos { x: 23, y: 10 });
    app.apply(Action::Point(Position::new(0, 4)));
    app.apply(Action::Scroll { dx: -1, dy: 0 });
    assert_eq!(
        lines(&render(&app, &world, 20, 8))[2..6],
        [
            "│..................│",
            "│..................│",
            "│..................│",
            "│..................│",
        ],
        "no stray arrows or marks at the view's edge"
    );
}

#[test]
fn the_status_line_names_the_terrain_under_the_cursor() {
    let world = drawn_world(&SMALL_MAP);
    let cases = [
        ((0, 0), " (0,0) grass │ SELECT"),
        ((8, 2), " (8,2) dirt │ SELECT"),
        ((0, 3), " (0,3) sand │ SELECT"),
        ((4, 0), " (4,0) shallow water │ SELECT"),
        ((5, 1), " (5,1) deep water │ SELECT"),
        ((2, 1), " (2,1) rock │ SELECT"),
    ];
    for ((x, y), expected) in cases {
        let mut app = app_for(&world, Theme::cp437(), 40, 8);
        // Tiles are drawn from screen cell (1, 2).
        app.apply(Action::Point(Position::new(1 + x, 2 + y)));
        assert_eq!(lines(&render(&app, &world, 40, 8))[7], expected);
    }
}

#[test]
fn with_room_the_status_line_also_shows_the_keys() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let status = lines(&render(&app, &world, 100, 30))[29].clone();
    assert!(
        status.starts_with(" (5,2) shallow water │ SELECT"),
        "{status}"
    );
    for hint in [
        "WASD scroll",
        "space pause",
        ". step",
        "+/- speed",
        "esc quit",
    ] {
        assert!(status.contains(hint), "{hint:?} missing from {status:?}");
    }
}

#[test]
fn the_quit_prompt_takes_over_the_status_line() {
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Back);
    assert_eq!(lines(&render(&app, &world, 100, 30))[29], " Quit? (y/n)");
}

#[test]
fn a_frame_bigger_than_the_fitted_view_draws_no_tiles_past_the_wall() {
    // The terminal can grow between the app fitting its view and the frame
    // being drawn, so for one frame the map view can be wider and taller than
    // the part of the map the viewport has room for.
    let row = ".".repeat(30);
    let world = drawn_world(&vec![row.as_str(); 20]);
    let mut app = app_for(&world, Theme::cp437(), 20, 8); // 18×4 tiles
    app.apply(Action::Scroll { dx: 100, dy: 100 }); // tiles (12, 16) to (29, 19): the bottom-right corner
    let screen = render(&app, &world, 40, 10); // room for 30×6 tiles
    for line in &lines(&screen)[2..6] {
        let tiles: String = line.chars().skip(1).take(19).collect();
        assert_eq!(tiles, format!("{} ", ".".repeat(18)), "{line:?}");
    }
}

/// A 10×5 field of grass with a berry bush, a thornbush, a berry and a ball.
fn garden(pack: DataPack) -> World {
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let objects = [
        (terra_sim::Pos { x: 1, y: 1 }, "berry_bush"),
        (terra_sim::Pos { x: 4, y: 1 }, "thornbush"),
        (terra_sim::Pos { x: 7, y: 1 }, "berry"),
        (terra_sim::Pos { x: 6, y: 3 }, "ball"),
    ];
    World::from_scenario(map, &objects, pack, 7).expect("valid scenario")
}

/// An app on `world` with the cursor pointed at tile `(x, y)` of a small map,
/// whose tiles are drawn from screen cell (1, 2).
fn pointing_at(world: &World, theme: Theme, width: u16, height: u16, x: u16, y: u16) -> App {
    let mut app = app_for(world, theme, width, height);
    app.apply(Action::Point(Position::new(1 + x, 2 + y)));
    app
}

#[test]
fn objects_are_drawn_with_their_themes_glyph_for_their_visual_state() {
    let world = garden(pack());
    let app = pointing_at(&world, Theme::cp437(), 40, 9, 0, 4); // the cursor out of the way
    let screen = render(&app, &world, 40, 9);
    assert_eq!(lines(&screen)[3], "║.'..♠..•..║");
    assert_eq!(
        screen[(7, 5)].symbol(),
        "○",
        "the ball, beside the cursor's arrows"
    );
    assert_eq!(screen[(2, 3)].fg, Color::Green, "a seedling");
    assert_eq!(screen[(5, 3)].fg, Color::Magenta, "a thornbush");
    assert_eq!(screen[(8, 3)].fg, Color::Red, "a berry");

    let ascii = pointing_at(&world, Theme::ascii(), 40, 9, 0, 4);
    assert_eq!(lines(&render(&ascii, &world, 40, 9))[3], "║.'..*..%..║");
}

#[test]
fn a_fruiting_bush_is_drawn_bold_red() {
    // A quick-growing berry bush: one tick as a seedling, a fruit every tick.
    let objects = include_str!("../../../data/objects.ron")
        .replace("ticks: (1500, 2500)", "ticks: (1, 1)")
        .replace("Every(200)", "Every(1)");
    let pack = DataPack::from_sources(&builtin_with("objects.ron", &objects)).expect("valid pack");
    let mut world = garden(pack);
    for _ in 0..3 {
        world.step();
    }
    let app = pointing_at(&world, Theme::cp437(), 40, 9, 0, 4);
    let bush = &render(&app, &world, 40, 9)[(2, 3)];
    assert_eq!(bush.symbol(), "♣");
    assert_eq!(bush.fg, Color::Red);
    assert!(bush.modifier.contains(Modifier::BOLD));
}

#[test]
fn the_status_line_names_the_object_under_the_cursor_with_its_stage() {
    let world = garden(pack());
    let cases = [
        ((1, 1), " (1,1) grass · berry bush (seedling) │ SELECT"),
        ((7, 1), " (7,1) grass · berry (fresh) │ SELECT"),
        ((6, 3), " (6,3) grass · ball │ SELECT"),
        ((0, 0), " (0,0) grass │ SELECT"),
    ];
    for ((x, y), expected) in cases {
        let app = pointing_at(&world, Theme::cp437(), 50, 9, x, y);
        assert_eq!(lines(&render(&app, &world, 50, 9))[8], expected);
    }
}

#[test]
fn the_top_bar_leaves_object_counts_to_the_world_tab() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let bar = lines(&render(&app, &world, 100, 30))[0].clone();
    assert!(bar.ends_with("│ seed 7"), "{bar}");
}

/// The right-hand `width` columns of each screen row, trimmed.
fn right_part(buffer: &Buffer, width: u16) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let row: String = (buffer.area.width - width..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            row.trim_end().to_string()
        })
        .collect()
}

#[test]
fn with_room_the_world_tab_shows_the_pack_and_every_object_types_numbers() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let inspector = right_part(&render(&app, &world, 100, 30), 46);
    assert!(inspector[1].starts_with("┌─ World ─"), "{:?}", inspector[1]);
    let body: Vec<&str> = inspector[2..9]
        .iter()
        .map(|line| {
            line.trim_start_matches('│')
                .trim_end_matches('│')
                .trim_end()
        })
        .collect();
    assert_eq!(
        body,
        [
            " data pack   core v1",
            " berry bush        1",
            "   seedling 1 · mature 0",
            "   fruit 0",
            " berry             1",
            " thornbush         1",
            " ball              1",
        ]
    );
}

#[test]
fn a_narrow_terminal_leaves_the_inspector_out_and_gives_the_map_view_the_width() {
    let row = ".".repeat(200);
    let world = drawn_world(&vec![row.as_str(); 20]);
    assert_eq!(
        ui::tile_area(Size::new(99, 30), world.map()).width,
        97,
        "below 100 columns, the map view takes it all"
    );
    assert_eq!(
        ui::tile_area(Size::new(100, 30), world.map()).width,
        52,
        "from 100 columns, the inspector takes 46"
    );
    let app = app_for(&world, Theme::cp437(), 99, 30);
    let screen = lines(&render(&app, &world, 99, 30));
    assert!(
        screen.iter().all(|line| !line.contains("World")),
        "no World tab"
    );
}

/// The built-in pack's files with `file` replaced by `text`.
fn builtin_with<'a>(file: &str, text: &'a str) -> Vec<(&'static str, &'a str)> {
    DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| (path, if path == file { text } else { builtin }))
        .collect()
}
