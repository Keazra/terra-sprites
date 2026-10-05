use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, Blocker, Command, DataPack, DeathCause, Emptied, EntityId, Event, EventKind, Grip,
    Hurt, Learned, Map, Outcome, Pos, Progress, Rejection, Removal, Scenario, ScriptedAction,
    Target, Terrain, Thing, Verb, World, WorldConfig,
};
use terra_tui::app::{App, CursorMode, Flow, Tab};
use terra_tui::input::{Action, Button};
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
/// The game draws nothing smaller than 100×30 (design §6.1), so a smaller
/// screen is a corner of one that size: see `render`.
fn app_for(world: &World, theme: Theme, width: u16, height: u16) -> App {
    let areas = ui::areas(at_least_min(width, height), world.map());
    App::new(world.map(), theme, 7, areas)
}

/// A screen of at least the smallest size the game draws on.
fn at_least_min(width: u16, height: u16) -> Size {
    Size::new(
        width.max(ui::MIN_SIZE.width),
        height.max(ui::MIN_SIZE.height),
    )
}

/// One frame on a `width`×`height` screen. Smaller than 100×30, which the
/// game doesn't draw on (design §6.1), it's the top-left `width`×`height`
/// of a frame that size, with its status line as the last row: so the tests
/// of small maps read as they did before the minimum.
fn render(app: &App, world: &World, width: u16, height: u16) -> Buffer {
    let full = at_least_min(width, height);
    let mut terminal = Terminal::new(TestBackend::new(full.width, full.height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    let drawn = terminal.backend().buffer().clone();
    if Size::new(width, height) == full {
        return drawn;
    }
    let mut corner = Buffer::empty(Rect::new(0, 0, width, height));
    for y in 0..height {
        let from = if y + 1 == height { full.height - 1 } else { y };
        for x in 0..width {
            corner[(x, y)] = drawn[(x, from)].clone();
        }
    }
    corner
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

/// The status line of a 100×30 screen.
fn status_line(app: &App, world: &World) -> String {
    lines(&render(app, world, 100, 30))[29].clone()
}

/// The status line up to its key hints, which the gap of two spaces before
/// them marks.
fn before_hints(status: &str) -> &str {
    status.split("  ").next().expect("a status line")
}

/// The screen cell the pointer is on to point at tile (x, y), in a map view
/// drawn from cell (1, 2) and not scrolled: one down and right of the tile
/// (design v27 §6.5).
fn pointer_on(x: u16, y: u16) -> Position {
    Position::new(2 + x, 3 + y)
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
    assert!(bar.contains("|| paused 1x"), "{bar}");
    assert!(!bar.contains('►'), "{bar}");
}

#[test]
fn while_paused_the_top_bar_shows_the_speed_time_resumes_at() {
    // Design v27 §6.6: `+` and `-` still change the speed while paused.
    let mut app = default_app();
    app.clock.toggle_pause();
    app.clock.faster();
    app.clock.faster();
    let bar = top_bar(&app);
    assert!(bar.contains("|| paused 4x"), "{bar}");
    app.clock.slower();
    app.clock.slower();
    app.clock.slower();
    let bar = top_bar(&app);
    assert!(bar.contains("|| paused 1/2x"), "{bar}");
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
        ui::areas(Size::new(100, 30), small.map()).tiles,
        Rect::new(1, 2, 10, 4),
        "a small map gets a shrunk map view"
    );
    assert_eq!(
        ui::areas(Size::new(100, 30), big_world().map()).tiles,
        Rect::new(1, 2, 52, 21),
        "a big map fills the space between the top bar and the event log"
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
fn in_train_mode_the_cursor_is_light_magenta_in_every_theme() {
    // Design v22 §6.5: it stands out on grass, where it mostly sits.
    let world = drawn_world(&SMALL_MAP);
    for theme in [Theme::cp437(), Theme::ascii()] {
        let mut app = app_for(&world, theme, 40, 8);
        app.apply(Action::Mode(CursorMode::Train), &world);
        let screen = render(&app, &world, 40, 8);
        // The cursor's mode mark, at the top left of its centre (6, 4).
        assert_eq!(screen[(5, 3)].fg, Color::LightMagenta);
        assert_eq!(screen[(6, 3)].fg, Color::LightMagenta, "and its arrows");
    }
}

#[test]
fn following_the_cursor_s_arrows_are_solid() {
    // Design v22 §6.2: they clamp the sprite. The ASCII theme has no solid
    // arrows; its status line says the Cursor follows it. Following doesn't
    // select it (design v26 §6.5), so it's drawn as any sprite is.
    let world = garden_with_sprites();
    let on_it = pointer_on(2, 3);
    for (theme, expected) in [
        (Theme::cp437(), [".♦▼·......", ".►☺◄..○...", ".·▲♦......"]),
        (Theme::ascii(), [".Sv-......", ".>@<..o...", ".-^S......"]),
    ] {
        let mut app = app_for(&world, theme, 40, 10);
        app.apply(Action::middle_click(on_it), &world);
        let map_rows: Vec<String> = lines(&render(&app, &world, 40, 10))[4..7]
            .iter()
            .map(|row| row.chars().skip(1).take(10).collect())
            .collect();
        assert_eq!(map_rows, expected);
    }
}

#[test]
fn the_status_marks_show_what_the_cursor_reports() {
    // Design v21 §6.5: a Train click flashes `+` in both status marks, top
    // right and bottom left, in the mode's colour.
    let world = garden_with_sprites();
    let on_it = pointer_on(2, 3);
    let mut app = app_for(&world, Theme::cp437(), 40, 10);
    app.apply(Action::Mode(CursorMode::Train), &world);
    app.apply(Action::left_click(on_it), &world);
    let screen = render(&app, &world, 40, 10);
    for (column, row) in [(4, 4), (2, 6)] {
        assert_eq!(screen[(column, row)].symbol(), "+", "({column}, {row})");
        assert_eq!(screen[(column, row)].fg, Color::LightMagenta);
    }
}

#[test]
fn in_grab_mode_the_status_marks_show_what_the_cursor_has_hold_of() {
    // Design v23 §6.5: empty, `↑` and `░`; holding or leading, `↓` and the
    // thing's glyph; all in Grab's yellow. The sprite is on map tile (2, 3)
    // and the ball on (6, 3); the marks sit top right and bottom left of the
    // Cursor's centre.
    let world = garden_with_sprites();
    let marks = |app: &App, centre: (u16, u16)| {
        let screen = render(app, &world, 40, 10);
        [(centre.0 + 1, centre.1 - 1), (centre.0 - 1, centre.1 + 1)].map(|cell| {
            assert_eq!(screen[cell].fg, Color::Yellow, "{cell:?}");
            screen[cell].symbol().to_string()
        })
    };
    let (sprite, ball) = (pointer_on(2, 3), pointer_on(6, 3));
    for (on, glyph) in [(sprite, "☺"), (ball, "○")] {
        let mut app = app_for(&world, Theme::cp437(), 40, 10);
        app.apply(Action::Mode(CursorMode::Grab), &world);
        app.apply(Action::Point(on), &world);
        // The Cursor's centre is up and left of the pointer.
        let centre = (on.x - 1, on.y - 1);
        assert_eq!(marks(&app, centre), ["↑", "░"], "empty");
        app.apply(Action::left_click(on), &world);
        assert_eq!(marks(&app, centre), ["↓", glyph], "a grab queued");
    }
}

#[test]
fn a_grab_click_with_nothing_to_act_on_flashes_a_question_mark() {
    let world = garden_with_sprites();
    let empty = pointer_on(4, 3);
    let mut app = app_for(&world, Theme::cp437(), 40, 10);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(empty), &world);
    let screen = render(&app, &world, 40, 10);
    // The Cursor's centre is up and left of the pointer.
    let centre = (empty.x - 1, empty.y - 1);
    for cell in [(centre.0 + 1, centre.1 - 1), (centre.0 - 1, centre.1 + 1)] {
        assert_eq!(screen[cell].symbol(), "?", "{cell:?}");
    }
}

#[test]
fn a_flashing_dotted_leash_runs_from_the_cursor_to_the_led_sprite_over_empty_ground() {
    // Design v23 §6.5. The sprite is on map tile (1, 2) and a berry on
    // (4, 2); a map tile (x, y) is drawn at screen cell (x + 1, y + 2).
    let pack = pack();
    let map = Map::from_ascii(&["............"; 5], &pack).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[(Pos { x: 4, y: 2 }, "berry")],
        sprites: &[(Pos { x: 1, y: 2 }, None)],
        scripted: &[(Pos { x: 1, y: 2 }, ScriptedAction::Rest)],
    };
    let world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 40, 10);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(pointer_on(1, 2)), &world);
    app.apply(Action::Point(pointer_on(6, 2)), &world);
    let row = |app: &App| -> String {
        lines(&render(app, &world, 40, 10))[4]
            .chars()
            .skip(1)
            .take(12)
            .collect()
    };
    assert_eq!(row(&app), ".☺··•→.←....", "dots, and the berry over them");
    let dot = &render(&app, &world, 40, 10)[(1 + 2, 2 + 2)];
    assert_eq!(dot.fg, Color::Yellow);
    app.animate(Duration::from_millis(500));
    assert_eq!(
        row(&app),
        ".☺..•→.←....",
        "flashing, like the Decision marker"
    );
    app.animate(Duration::from_millis(500));
    app.apply(Action::Mode(CursorMode::Train), &world);
    assert_eq!(row(&app), ".☺··•→.←....", "in every mode");
}

#[test]
fn map_tiles_take_their_theme_colours() {
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 40, 8);
    // Point at the top-left tile, so the cursor sits clear of the tiles checked.
    app.apply(Action::Point(Position::new(1, 2)), &world);
    let screen = render(&app, &world, 40, 8);
    assert_eq!(screen[(3, 2)].fg, Color::Green, "grass");
    assert_eq!(screen[(6, 3)].fg, Color::Blue, "deep water");
    assert_eq!(screen[(3, 3)].fg, Color::Gray, "rock");
}

#[test]
fn the_border_is_single_where_the_map_carries_on_and_double_at_the_wall() {
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    // In the middle of the map, every side has more map beyond it.
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    let ends = |row: &String| {
        let chars: Vec<char> = row.chars().collect();
        (chars[0], chars[chars.len() - 1])
    };
    assert!(rows[0].starts_with("┌─ Map ──"), "{}", rows[0]);
    assert_eq!(ends(&rows[0]), ('┌', '┐'));
    assert!(rows[1..22].iter().all(|row| ends(row) == ('│', '│')));
    assert_eq!(ends(&rows[22]), ('└', '┘'));
    assert!(rows[22].chars().skip(1).take(52).all(|c| c == '─'));

    // Scrolled to the top-left corner, the top and left sides are the wall.
    app.apply(Action::Scroll { dx: -200, dy: -200 }, &world);
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    assert!(rows[0].starts_with("╔═ Map ══"), "{}", rows[0]);
    assert_eq!(ends(&rows[0]), ('╔', '╕'));
    assert!(rows[1..22].iter().all(|row| ends(row) == ('║', '│')));
    assert_eq!(ends(&rows[22]), ('╙', '┘'));
}

#[test]
fn the_cursor_is_clipped_at_the_edge_of_the_map_view() {
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Point(Position::new(1, 2)), &world); // the view's top-left tile
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    assert!(rows[1].starts_with("│.←..."), "{}", rows[1]);
    assert!(rows[2].starts_with("│↑♦..."), "{}", rows[2]);
    assert!(rows[3].starts_with("│....."), "{}", rows[3]);
}

/// A field of grass bigger than the map view of a 100×30 screen, which
/// shows 52×21 of its tiles from cell (1, 2).
fn big_world() -> World {
    let row = ".".repeat(120);
    drawn_world(&vec![row.as_str(); 60])
}

/// The map view's rows on a 100×30 screen, border included: the left 54
/// columns of rows 1 to 23.
fn map_view_rows(screen: &Buffer) -> Vec<String> {
    (1..24)
        .map(|y| (0..54).map(|x| screen[(x, y)].symbol()).collect())
        .collect()
}

#[test]
fn a_cursor_whose_target_is_out_of_view_is_not_drawn_at_all() {
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    // Put the cursor on the view's rightmost column, with the pointer on the
    // border beside it, then move the pointer off the map and scroll left, so
    // the cursor's tile leaves the view.
    app.apply(Action::Point(Position::new(53, 12)), &world);
    assert_eq!(app.cursor().x, app.viewport().x + 51);
    app.apply(Action::Point(Position::new(0, 12)), &world);
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    for row in &rows[1..22] {
        assert_eq!(
            row,
            &format!("│{}│", ".".repeat(52)),
            "no stray arrows or marks at the view's edge"
        );
    }
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
        app.apply(Action::Point(pointer_on(x, y)), &world);
        assert_eq!(
            before_hints(&lines(&render(&app, &world, 40, 8))[7]),
            expected
        );
    }
}

#[test]
fn with_room_the_status_line_also_shows_the_keys() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 120, 30);
    let status = lines(&render(&app, &world, 120, 30))[29].clone();
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
    app.apply(Action::Back, &world);
    assert_eq!(lines(&render(&app, &world, 100, 30))[29], " Quit? (y/n)");
}

#[test]
fn a_frame_bigger_than_the_fitted_view_draws_no_tiles_past_the_wall() {
    // The terminal can grow between the app fitting its view and the frame
    // being drawn, so for one frame the map view can be wider and taller than
    // the part of the map the viewport has room for.
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30); // 52×21 tiles
    app.apply(Action::Scroll { dx: 200, dy: 200 }, &world); // tiles (68, 39) to (119, 59): the bottom-right corner
    let screen = render(&app, &world, 140, 40); // room for 92×31 tiles
    for y in 2..23 {
        let tiles: String = (1..54).map(|x| screen[(x, y)].symbol()).collect();
        assert_eq!(tiles, format!("{} ", ".".repeat(52)), "row {y}");
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
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &[],
        scripted: &[],
    };
    World::from_scenario(scenario, pack, 7).expect("valid scenario")
}

/// The garden, with starter sprites at (2, 3) and on the berry at (7, 1).
fn garden_with_sprites() -> World {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let objects = [
        (terra_sim::Pos { x: 1, y: 1 }, "berry_bush"),
        (terra_sim::Pos { x: 4, y: 1 }, "thornbush"),
        (terra_sim::Pos { x: 7, y: 1 }, "berry"),
        (terra_sim::Pos { x: 6, y: 3 }, "ball"),
    ];
    let sprites = [
        (terra_sim::Pos { x: 2, y: 3 }, None),
        (terra_sim::Pos { x: 7, y: 1 }, None),
    ];
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, pack, 7).expect("valid scenario")
}

#[test]
fn sprites_are_drawn_with_their_theme_glyph_over_any_item() {
    let world = garden_with_sprites();
    let cp437 = pointing_at(&world, Theme::cp437(), 40, 9, 0, 4);
    let screen = render(&cp437, &world, 40, 9);
    assert_eq!(lines(&screen)[3], "║.'..♠..☺..║", "the sprite on the berry");
    assert_eq!(screen[(3, 5)].symbol(), "☺");
    let ascii = pointing_at(&world, Theme::ascii(), 40, 9, 0, 4);
    assert_eq!(lines(&render(&ascii, &world, 40, 9))[3], "║.'..*..@..║");
}

#[test]
fn the_selected_sprite_is_drawn_with_its_own_glyph() {
    let world = garden_with_sprites();
    let mut cp437 = pointing_at(&world, Theme::cp437(), 40, 9, 7, 1);
    cp437.apply(Action::left_click(pointer_on(7, 1)), &world);
    cp437.apply(Action::Point(pointer_on(0, 4)), &world); // the cursor out of the way
    let screen = render(&cp437, &world, 40, 9);
    assert_eq!(lines(&screen)[3], "║.'..♠..☻..║", "the selected sprite");
    assert_eq!(screen[(3, 5)].symbol(), "☺", "the other sprite");

    // In ascii, `&` is already the berry bush, so the selected sprite is `@` in reverse video.
    let mut ascii = pointing_at(&world, Theme::ascii(), 40, 9, 7, 1);
    ascii.apply(Action::left_click(pointer_on(7, 1)), &world);
    ascii.apply(Action::Point(pointer_on(0, 4)), &world);
    let screen = render(&ascii, &world, 40, 9);
    assert_eq!(screen[(8, 3)].symbol(), "@");
    assert!(screen[(8, 3)].modifier.contains(Modifier::REVERSED));
    assert!(!screen[(3, 5)].modifier.contains(Modifier::REVERSED));
}

/// An app on `world` with the cursor pointed at tile `(x, y)` of a small map,
/// whose tiles are drawn from screen cell (1, 2).
fn pointing_at(world: &World, theme: Theme, width: u16, height: u16, x: u16, y: u16) -> App {
    let mut app = app_for(world, theme, width, height);
    app.apply(Action::Point(pointer_on(x, y)), world);
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
        assert_eq!(
            before_hints(&lines(&render(&app, &world, 50, 9))[8]),
            expected
        );
    }
}

#[test]
fn the_status_line_shows_a_sprite_under_the_cursor_by_its_id() {
    let world = garden_with_sprites();
    let sprite = world
        .sprite_at(terra_sim::Pos { x: 7, y: 1 })
        .expect("the sprite on the berry");
    let app = pointing_at(&world, Theme::cp437(), 60, 9, 7, 1);
    // Sprites have no names until the player gives them one (design v7 §6.5).
    let expected = format!(
        " (7,1) grass · Sprite #{} · berry (fresh) │ SELECT",
        sprite.id().0
    );
    assert_eq!(
        before_hints(&lines(&render(&app, &world, 60, 9))[8]),
        expected
    );
}

#[test]
fn the_top_bar_ends_with_how_to_open_help() {
    // Design §6.1.
    assert!(top_bar(&default_app()).ends_with("     ? help"));
}

#[test]
fn the_top_bar_leaves_object_counts_to_the_world_tab() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let bar = lines(&render(&app, &world, 100, 30))[0].clone();
    for object in ["bush", "berr", "thorn", "ball"] {
        assert!(!bar.contains(object), "{bar}");
    }
}

#[test]
fn the_top_bar_shows_the_population_after_the_seed() {
    let world = garden_with_sprites();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let bar = lines(&render(&app, &world, 100, 30))[0].clone();
    assert!(bar.contains("│ seed 7 │ sprites 2 "), "{bar}");
    assert!(
        top_bar(&default_app()).contains("│ sprites 30 "),
        "the built-in preset's"
    );
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
    assert_eq!(
        inspector[1], "┌─ Body Brain Chem Genome [World] ───────────┐",
        "with nothing selected, the title is just the tabs"
    );
    let body: Vec<&str> = inspector[2..12]
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
            " sprites           0",
            " deaths            0",
            "   starvation 0 · dehydration 0 · old age 0",
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
fn below_100_by_30_the_screen_says_the_terminal_is_too_small() {
    // Design §6.1: it replaces every panel, and nothing can be clicked.
    let world = big_world();
    for (width, height) in [(99, 30), (100, 29), (80, 24)] {
        let areas = ui::areas(Size::new(width, height), world.map());
        assert!(areas.tiles.is_empty() && areas.inspector.is_none());
        let app = app_for(&world, Theme::cp437(), 100, 30);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| ui::render(frame, &app, &world))
            .unwrap();
        let screen = lines(terminal.backend().buffer());
        let text: Vec<&str> = screen
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();
        let size = format!("needs 100x30, this is {width}x{height}");
        assert_eq!(text, ["Terminal too small", size.as_str()]);
    }
    assert_eq!(
        ui::areas(Size::new(100, 30), world.map()).tiles.width,
        52,
        "from 100 columns, the inspector takes 46"
    );
}

#[test]
fn while_the_terminal_is_too_small_the_game_runs_on_and_esc_still_asks_to_quit() {
    // Design v29 §6.1: shrinking the window doesn't pause it.
    let mut world = big_world();
    let small = Size::new(80, 24);
    let mut app = App::new(
        world.map(),
        Theme::cp437(),
        7,
        ui::areas(small, world.map()),
    );
    let before = world.tick();
    let mut ran = 0;
    for _ in 0..10 {
        ran += app
            .clock
            .advance(Duration::from_millis(100), || drop(world.step()), || false);
    }
    assert!(ran > 0 && world.tick() > before, "time goes on");
    app.apply(Action::TogglePause, &world);
    assert!(app.clock.is_paused(), "space still pauses");
    app.apply(Action::Back, &world);
    let mut terminal = Terminal::new(TestBackend::new(small.width, small.height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, &app, &world))
        .unwrap();
    let screen = lines(terminal.backend().buffer());
    assert!(
        screen.iter().any(|line| line.trim() == "Quit? (y/n)"),
        "the quit prompt shows: {screen:?}"
    );
    assert_eq!(app.apply(Action::Confirm, &world), Flow::Quit);
}

/// The built-in pack's files with `file` replaced by `text`.
fn builtin_with<'a>(file: &str, text: &'a str) -> Vec<(&'static str, &'a str)> {
    DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| (path, if path == file { text } else { builtin }))
        .collect()
}

fn died(tick: u64, id: u64, cause: DeathCause, age: u64) -> Event {
    Event {
        tick,
        kind: EventKind::Died {
            id: EntityId(id),
            name: None,
            cause,
            age,
        },
    }
}

/// The text inside a bordered row, without the borders or trailing spaces.
fn inside(row: &str) -> &str {
    row.trim_start_matches('│').trim_end_matches('│').trim()
}

#[test]
fn the_event_log_lists_deaths_newest_first_under_the_map() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.record(&[died(4_012, 31, DeathCause::Dehydration, 4_012)], &world);
    app.record(
        &[
            Event {
                tick: 4_100,
                kind: EventKind::ObjectRemoved {
                    id: EntityId(3),
                    object_type: "berry".into(),
                    reason: Removal::Expired,
                },
            },
            died(4_100, 12, DeathCause::Starvation, 3_900),
            died(4_100, 13, DeathCause::OldAge, 66_000),
        ],
        &world,
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(screen[24].starts_with("┌─ Events ─"), "{:?}", screen[24]);
    assert_eq!(
        inside(&screen[25]),
        "4,100  Sprite #13 died (old age, age 66,000)"
    );
    assert_eq!(
        inside(&screen[26]),
        "4,100  Sprite #12 died (starvation, age 3,900)"
    );
    assert_eq!(
        inside(&screen[27]),
        "4,012  Sprite #31 died (dehydration, age 4,012)"
    );
    assert!(screen[28].starts_with('└'), "three lines of events");
}

#[test]
fn the_event_log_leaves_object_and_action_events_out_and_keeps_the_latest_100() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let spawned = Event {
        tick: 1,
        kind: EventKind::ObjectSpawned {
            id: EntityId(9),
            object_type: "berry".into(),
            pos: terra_sim::Pos { x: 1, y: 1 },
        },
    };
    app.record(&[spawned], &world);
    let id = EntityId(3);
    app.record(
        &[
            Event {
                tick: 1,
                kind: EventKind::ActionStarted {
                    id,
                    verb: Verb::Wander,
                },
            },
            Event {
                tick: 1,
                kind: EventKind::ActionEnded {
                    id,
                    verb: Verb::Wander,
                    outcome: Outcome::Failed,
                    action: ActionView {
                        verb: Verb::Wander,
                        destination: None,
                        target: None,
                        target_type: None,
                        attempted: false,
                        target_gone: false,
                        hurt: Hurt::default(),
                        progress: Progress::Ended(Outcome::Failed),
                    },
                },
            },
        ],
        &world,
    );
    assert_eq!(app.event_log().count(), 0, "nor action events");
    for tick in 0..150 {
        app.record(&[died(tick, tick, DeathCause::Starvation, tick)], &world);
    }
    let ticks: Vec<u64> = app.event_log().map(|(event, _)| event.tick).collect();
    assert_eq!(ticks.len(), 100);
    assert_eq!((ticks[0], ticks[99]), (149, 50), "newest first");
}

/// Nobody hurt.
const UNHURT: Hurt = Hurt {
    actor: false,
    target: false,
};

/// The actor hurt, by what it did.
const HURT_ITSELF: Hurt = Hurt {
    actor: true,
    target: false,
};

/// Sprite `id` finished `verb` at `target`, of the object type `target_type`,
/// on `tick`, with `outcome`, its attempt hurting whom `hurt` says.
fn acted(
    tick: u64,
    id: u64,
    verb: Verb,
    (target, target_type): (Target, u16),
    outcome: Outcome,
    hurt: Hurt,
) -> Event {
    let action = ActionView {
        verb,
        destination: None,
        target: Some(target),
        target_type: Some(target_type),
        attempted: outcome == Outcome::Applied,
        target_gone: false,
        hurt,
        progress: Progress::Ended(outcome),
    };
    Event {
        tick,
        kind: EventKind::ActionEnded {
            id: EntityId(id),
            verb,
            outcome,
            action,
        },
    }
}

#[test]
fn the_event_log_shows_every_play_and_hit_and_whatever_hurt_a_sprite() {
    use Outcome::{Applied, Interrupted};
    // Object types: berry_bush 1, thornbush 3, ball 4, sprite 101.
    let ball = (Target::Object(EntityId(40)), 4);
    let thorns = (Target::Object(EntityId(77)), 3);
    let bush = (Target::Object(EntityId(80)), 1);
    let sprite = |id| (Target::Sprite(EntityId(id)), 101);
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let log = |app: &App| -> Vec<String> {
        let screen = lines(&render(app, &world, 100, 30));
        screen[25..28]
            .iter()
            .map(|row| inside(row).to_string())
            .collect()
    };
    app.record(
        &[
            acted(10, 4, Verb::Play, ball, Applied, UNHURT),
            acted(11, 5, Verb::Eat, bush, Applied, UNHURT),
            acted(12, 9, Verb::Play, sprite(2), Applied, UNHURT),
            acted(13, 7, Verb::Hit, sprite(12), Interrupted, UNHURT),
            acted(14, 7, Verb::Hit, sprite(12), Applied, UNHURT),
        ],
        &world,
    );
    assert_eq!(
        log(&app),
        [
            "14  Sprite #7 hit Sprite #12",
            "12  Sprite #9 played with Sprite #2",
            "10  Sprite #4 kicked a ball",
        ],
        "not eating that hurt nobody, nor what didn't get done"
    );
    app.record(
        &[
            acted(20, 3, Verb::Eat, thorns, Applied, HURT_ITSELF),
            acted(21, 3, Verb::Play, thorns, Applied, HURT_ITSELF),
            acted(22, 6, Verb::Hit, ball, Applied, UNHURT),
        ],
        &world,
    );
    assert_eq!(
        log(&app),
        [
            "22  Sprite #6 hit a ball",
            "21  Sprite #3 played with a thornbush and got hurt",
            "20  Sprite #3 tried to eat a thornbush and got hurt",
        ]
    );
}

/// The event log's rows, newest first, once an app on `world` has recorded a
/// lesson for each of `lessons`: (tick, sprite ID, what it learned, good).
fn logged_lessons(world: &World, lessons: Vec<(u64, u64, Learned, bool)>) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    let events: Vec<Event> = lessons
        .into_iter()
        .map(|(tick, id, learned, good)| Event {
            tick,
            kind: EventKind::LearnedMilestone {
                id: EntityId(id),
                learned,
                good,
            },
        })
        .collect();
    app.record(&events, world);
    let screen = lines(&render(&app, world, 100, 30));
    screen[25..25 + events.len()]
        .iter()
        .map(|row| inside(row).to_string())
        .collect()
}

#[test]
fn the_event_log_says_what_a_sprite_learned_in_plain_words() {
    // Design v16 §6.1.
    let world = garden(pack());
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let water_for_thirst = Learned::Worth {
        thing: "water".into(),
        need: Some("thirst".into()),
    };
    let eating_balls = Learned::Habit {
        thing: "ball".into(),
        verb: Verb::Eat,
    };
    let log = logged_lessons(
        &world,
        vec![
            (30, 12, thorns_bad, false),
            (31, 12, water_for_thirst, true),
            (32, 9, eating_balls, false),
        ],
    );
    assert_eq!(
        log,
        [
            "32  Sprite #9 learned: eating balls is bad",
            "31  Sprite #12 learned: water is good for thirst",
            "30  Sprite #12 learned: thornbushes are bad",
        ]
    );
}

#[test]
fn a_lesson_words_an_object_type_as_it_names_itself() {
    // Design v17 §3.5.1: thornbushes renamed brambles, and balls with no
    // plural, as if they were a thing you don't count.
    let builtin = include_str!("../../../data/objects.ron");
    let (thorns, balls) = (r#"plural: "thornbushes""#, r#" plural: "balls","#);
    assert!(builtin.contains(thorns) && builtin.contains(balls));
    let objects = builtin
        .replace(thorns, r#"plural: "brambles""#)
        .replace(balls, "");
    let pack = DataPack::from_sources(&builtin_with("objects.ron", &objects)).expect("valid pack");
    let world = garden(pack);
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let ball_for_boredom = Learned::Worth {
        thing: "ball".into(),
        need: Some("boredom".into()),
    };
    let playing_with_balls = Learned::Habit {
        thing: "ball".into(),
        verb: Verb::Play,
    };
    let log = logged_lessons(
        &world,
        vec![
            (30, 12, thorns_bad, false),
            (31, 12, ball_for_boredom, true),
            (32, 9, playing_with_balls, true),
        ],
    );
    assert_eq!(
        log,
        [
            "32  Sprite #9 learned: playing with ball is good",
            "31  Sprite #12 learned: ball is good for boredom",
            "30  Sprite #12 learned: brambles are bad",
        ]
    );
}

#[test]
fn a_100_by_30_screen_has_every_panel() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(screen[0].starts_with(" Terra Sprites"));
    assert!(screen[1].contains("Map") && screen[1].contains("World"));
    assert!(screen[24].contains("Events"));
}

/// Presses `]` until `tab` is open.
fn open(app: &mut App, world: &World, tab: Tab) {
    while app.tab() != tab {
        app.apply(Action::NextTab, world);
    }
}

/// The inspector's rows on a 100×30 screen: its title, then the text inside
/// its border, trimmed.
fn inspector(app: &App, world: &World) -> (String, Vec<String>) {
    let rows = right_part(&render(app, world, 100, 30), 46);
    let text = rows[2..rows.len() - 7]
        .iter()
        .map(|row| inside(row).to_string())
        .collect();
    (rows[1].clone(), text)
}

#[test]
fn the_sprite_tabs_say_how_to_select_a_sprite_when_none_is() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    for (tab, title) in [
        ("Body", "┌─ [Body] Brain Chem Genome World ───────────┐"),
        ("Brain", "┌─ Body [Brain] Chem Genome World ───────────┐"),
        ("Chem", "┌─ Body Brain [Chem] Genome World ───────────┐"),
        ("Genome", "┌─ Body Brain Chem [Genome] World ───────────┐"),
    ] {
        app.apply(Action::NextTab, &world);
        let (top, text) = inspector(&app, &world);
        assert_eq!(top, title, "{tab}");
        assert_eq!(
            text[0], "No sprite selected: click one, or press Tab",
            "{tab}"
        );
        assert!(text[1..].iter().all(String::is_empty), "{tab}: {text:?}");
    }
}

#[test]
fn the_title_names_the_selected_sprite_before_the_tabs() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let id = world.sprites().next().expect("a sprite").id().0;
    let (top, _) = inspector(&app, &world);
    // "Sprite #5" and five tabs don't fit in 46 columns, so the label is
    // shortened to the ID, never the tabs (design §6.1).
    let title = format!("┌─ #{id} ── [Body] Brain Chem Genome World ─");
    assert!(top.starts_with(&title), "{top:?}");
    assert!(top.ends_with("─┐"), "{top:?}");
    app.apply(Action::PreviousTab, &world);
    let (top, _) = inspector(&app, &world);
    assert!(
        top.starts_with(&format!("┌─ #{id} ── Body Brain Chem Genome [World] ─")),
        "{top:?}"
    );
}

#[test]
fn when_the_selected_sprite_dies_its_tabs_say_how_and_at_what_age() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    app.record(&[died(4_012, id.0, DeathCause::Dehydration, 4_012)], &world);
    for tab in ["Body", "Brain", "Chem", "Genome"] {
        let (top, text) = inspector(&app, &world);
        assert!(
            top.starts_with(&format!("┌─ #{} ──", id.0)),
            "{tab}: {top:?}"
        );
        assert_eq!(
            text[0],
            format!("Sprite #{} died of dehydration at age 4,012", id.0),
            "{tab}"
        );
        app.apply(Action::NextTab, &world);
    }
}

#[test]
fn a_sprite_hurt_to_death_by_an_object_is_named_after_its_type() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    // The thornbush is object type 3.
    app.record(&[died(4_012, id.0, DeathCause::HurtBy(3), 4_012)], &world);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..2].join(" "),
        format!("Sprite #{} died hurt by thornbush at age 4,012", id.0)
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert_eq!(
        inside(&screen[25]),
        format!(
            "4,012  Sprite #{} died (hurt by thornbush, age 4,012)",
            id.0
        )
    );
}

/// A 10×5 field of grass with one sprite at (2, 3), made from `genome`, and
/// an app on it that has selected the sprite and stepped the world `ticks`
/// times.
fn one_sprite(genome: &str, ticks: u32) -> (World, App) {
    one_sprite_doing(genome, &[], ticks)
}

/// `one_sprite`, starting on the `scripted` actions.
fn one_sprite_doing(genome: &str, scripted: &[ScriptedAction], ticks: u32) -> (World, App) {
    one_sprite_among(genome, &[], scripted, ticks)
}

/// `one_sprite_doing`, with `objects` in the field.
fn one_sprite_among(
    genome: &str,
    objects: &[(Pos, &str)],
    scripted: &[ScriptedAction],
    ticks: u32,
) -> (World, App) {
    one_sprite_in(pack(), genome, objects, scripted, ticks)
}

/// `one_sprite_among`, in `pack`.
fn one_sprite_in(
    pack: DataPack,
    genome: &str,
    objects: &[(Pos, &str)],
    scripted: &[ScriptedAction],
    ticks: u32,
) -> (World, App) {
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let genome = terra_sim::Genome::from_ron(genome, &pack).expect("a valid genome");
    let start = Pos { x: 2, y: 3 };
    let sprites = [(start, Some(genome))];
    let scripted: Vec<(Pos, ScriptedAction)> = scripted.iter().map(|&a| (start, a)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    for _ in 0..ticks {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    (world, app)
}

/// Traits, and drives set so one tick changes them in ways worked out by
/// hand: hunger halves every tick, and boredom gains .1 a tick.
const WORKED_GENOME: &str = r#"(format: 1, genes: [
    Trait(trait: "speed", value: 7.25),
    Trait(trait: "sense_radius", value: 9.5),
    Trait(trait: "lifespan", value: 61204.0),
    InitialConcentration(chem: "hunger", value: 0.8),
    HalfLife(chem: "hunger", ticks: 1),
    InitialConcentration(chem: "thirst", value: 0.18),
    InitialConcentration(chem: "tiredness", value: 0.39),
    InitialConcentration(chem: "boredom", value: 0.2),
    Emitter(locus: Locus("always"), mode: Level, gain: 0.1, chem: "boredom"),
])"#;

#[test]
fn the_body_tab_shows_age_traits_drives_and_physical_levels() {
    let (world, app) = one_sprite_doing(WORKED_GENOME, &[ScriptedAction::Rest], 1);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..14],
        [
            "Resting · 9 ticks left",
            "age 1 · lifespan 61,204",
            "speed 7.25 · sense 9.5",
            "",
            "hunger      ████░░░░░░ .40 ▼",
            "thirst      ██░░░░░░░░ .18",
            "pain        ░░░░░░░░░░ .00",
            "tiredness   ████░░░░░░ .39",
            "boredom     ███░░░░░░░ .30 ▲",
            "loneliness  ░░░░░░░░░░ .00",
            "crowdedness ░░░░░░░░░░ .00",
            "",
            // A newborn's are full, and one tick at rest takes them only to .9998 or so.
            "energy 1.00 · hydration 1.00 · stamina 1.00",
            "food .00 · water .00 · injury .00",
        ]
    );
    assert_eq!(text[14..17], ["", "Observed", "nothing yet"]);
    assert!(text[17..].iter().all(String::is_empty), "{text:?}");
}

/// A genome that walks a grass step a tick.
const SPEED_10: &str = r#"(format: 1, genes: [Trait(trait: "speed", value: 10.0)])"#;

#[test]
fn the_body_tab_starts_with_what_the_sprite_is_doing_in_plain_words() {
    let wander = ScriptedAction::Wander {
        destination: Pos { x: 7, y: 3 },
    };
    let (world, app) = one_sprite_doing(SPEED_10, &[wander], 1);
    assert_eq!(
        inspector(&app, &world).1[0],
        "Wandering off · 4 tiles to go"
    );
    let (world, app) = one_sprite_doing(SPEED_10, &[wander], 5);
    assert_eq!(inspector(&app, &world).1[0], "Arrived");
}

#[test]
fn the_body_tab_says_a_led_sprite_is_being_led_and_how_far_behind() {
    // Design v23 §6.1. It walks a grass step a tick, from (2, 3).
    let (mut world, mut app) = one_sprite_doing(SPEED_10, &[], 0);
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.submit(Command::MoveCursor {
        tile: Pos { x: 7, y: 3 },
    });
    world.step();
    assert_eq!(inspector(&app, &world).1[0], "Being led · 4 tiles behind");
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(
        inspector(&app, &world).1[0],
        "LED → (7,3) · walking (4 tiles)"
    );
    for _ in 0..4 {
        world.step();
    }
    assert_eq!(inspector(&app, &world).1[0], "LED → (7,3)");
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(inspector(&app, &world).1[0], "Being led", "caught up");
}

#[test]
fn a_led_sprite_as_close_as_it_can_get_to_the_cursor_has_caught_up() {
    // Design v23 §6.1: the Cursor is on a bush, which it can't stand on.
    let bush = Pos { x: 7, y: 3 };
    let (mut world, app) = one_sprite_among(SPEED_10, &[(bush, "berry_bush")], &[], 0);
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.submit(Command::MoveCursor { tile: bush });
    for _ in 0..6 {
        world.step();
    }
    assert_eq!(world.sprite(sprite).expect("it").pos(), Pos { x: 6, y: 3 });
    assert_eq!(inspector(&app, &world).1[0], "Being led");
}

#[test]
fn the_decision_marker_flashes_an_x_where_the_selected_sprite_is_heading() {
    let destination = Pos { x: 7, y: 3 };
    let wander = ScriptedAction::Wander { destination };
    let (world, mut app) = one_sprite_doing(SPEED_10, &[wander], 1);
    let cell = app.cell_of(destination).expect("in view");
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[cell].symbol(), "X", "without the detail view");
    assert_eq!(screen[cell].fg, Color::White);
    assert!(
        !screen[cell].modifier.contains(Modifier::REVERSED),
        "a plain X"
    );

    // Like a text cursor, in real time: half a second the X, half the tile.
    app.animate(Duration::from_millis(400));
    assert_eq!(render(&app, &world, 100, 30)[cell].symbol(), "X");
    app.animate(Duration::from_millis(100));
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[cell].symbol(), ".");
    assert_eq!(screen[cell].fg, Color::Green, "the grass as it is");
    app.animate(Duration::from_millis(500));
    assert_eq!(render(&app, &world, 100, 30)[cell].symbol(), "X");

    // `v` only changes the action line.
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(
        inspector(&app, &world).1[0],
        "WANDER → (7,3) · walking (4 tiles)"
    );
    assert_eq!(render(&app, &world, 100, 30)[cell].symbol(), "X");
}

/// A hungry sprite whose instincts point it at berries and to eating, with
/// a nudge against eating for no reason and a mild habit of wandering, and
/// no curiosity, so its attention is its instincts and nearness alone.
const HUNGRY_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 0.8),
    BrainParam(param: "curiosity", value: 0.0),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "fruit", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
    Instinct(inputs: [("hunger", false), ("target_adjacent", true)], verb: Eat, weight: 0.5),
    Instinct(inputs: [("always", false)], verb: Eat, weight: -0.1),
    Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
    Instinct(inputs: [("target_adjacent", true)], verb: Eat, weight: 0.004),
])"#;

#[test]
fn the_brain_tab_shows_attention_scores_and_what_adds_most_to_the_decision() {
    let objects = [
        (Pos { x: 8, y: 3 }, "berry_bush"),
        (Pos { x: 6, y: 1 }, "berry"),
    ];
    let (world, mut app) = one_sprite_among(HUNGRY_GENOME, &objects, &[], 1);
    open(&mut app, &world, Tab::Brain);
    let (top, text) = inspector(&app, &world);
    assert!(top.contains("[Brain]"), "{top:?}");
    // Attention: hunger's 1 × .8 on the berry, plus salience .5 × (1 − its
    // distance, .34); the bush has salience alone, .5 × (1 − .5).
    // The decision: hunger .8 × 1, .8 × (1 − 0) × .5 and always 1 × −.1;
    // not target adjacent's .004 rounds to nothing, so it's left out.
    assert_eq!(
        text[..8],
        [
            "ATTENTION",
            "► berry                               1.13",
            "berry bush                           .25",
            "",
            "DECISION: EAT                         1.10",
            "hunger                              +.80",
            "hunger & not target adjacent        +.40",
            "always                              -.10",
        ]
    );
    assert!(text[8..].iter().all(String::is_empty), "{text:?}");
}

/// A hungry sprite drawn to a berry and to eating it, whose eating halves
/// its hunger, as the starter genome's does.
const BERRY_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 1.0),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "fruit", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
])"#;

#[test]
fn the_brain_tab_shows_what_the_sprite_has_learned_as_memory() {
    // It eats the berry beside it at tick 0; at tick 1 its hunger falls by
    // .5, and it learns berries are good for hunger: .5 × .5.
    let objects = [(Pos { x: 3, y: 3 }, "berry")];
    let (world, mut app) = one_sprite_among(BERRY_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let at = text
        .iter()
        .position(|row| row.starts_with("MEMORY"))
        .unwrap_or_else(|| panic!("{text:?}"));
    assert_eq!(text[at - 1], "", "a blank row before it");
    assert_eq!(
        text[at..at + 2],
        ["MEMORY", "berries are good for hunger         +.25"]
    );
}

#[test]
fn the_brain_tab_leaves_out_what_rounds_to_nothing() {
    // Learning at .004, it learns berries are worth .004 × .5 = .002 for
    // hunger: something, but .00 to two places (design §5.9).
    let genome = BERRY_GENOME.replace(
        "])",
        "    BrainParam(param: \"worth_rate_good\", value: 0.004),\n])",
    );
    let objects = [(Pos { x: 3, y: 3 }, "berry")];
    let (world, mut app) = one_sprite_among(&genome, &objects, &[], 2);
    let memory = world.sprites().next().expect("the sprite").memory();
    assert!(
        memory.iter().any(|m| m.amount > 0.0 && m.amount < 0.005),
        "{memory:?}"
    );
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        !text.iter().any(|row| row.starts_with("MEMORY")),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_leaves_memory_out_until_it_has_learned_something() {
    let (world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        !text.iter().any(|row| row.starts_with("MEMORY")),
        "{text:?}"
    );
}

#[test]
fn the_attention_marker_shades_the_one_thing_the_selected_sprite_attends_to() {
    // Away from the cursor, which starts on the map's centre, (5, 2).
    let (berry, bush) = (Pos { x: 0, y: 1 }, Pos { x: 8, y: 4 });
    let objects = [(bush, "berry_bush"), (berry, "berry")];
    let (world, mut app) = one_sprite_among(HUNGRY_GENOME, &objects, &[], 1);
    let (berry_cell, bush_cell) = (
        app.cell_of(berry).expect("in view"),
        app.cell_of(bush).expect("in view"),
    );
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[berry_cell].symbol(), "•", "the berry, still drawn");
    assert_eq!(screen[berry_cell].bg, Color::DarkGray, "a steady mark");
    assert_eq!(screen[bush_cell].bg, Color::Reset);
    app.animate(Duration::from_millis(250));
    assert_eq!(
        render(&app, &world, 100, 30)[berry_cell].bg,
        Color::DarkGray
    );

    // With no sprite selected, nothing is marked.
    let grass = app.cell_of(Pos { x: 0, y: 0 }).expect("in view");
    app.apply(Action::left_click(grass), &world);
    assert_eq!(app.selection(), None);
    assert_eq!(render(&app, &world, 100, 30)[berry_cell].bg, Color::Reset);
}

#[test]
fn a_led_sprite_shows_no_attention_marker() {
    // Design v23 §6.5: led, it attends to nothing, so the mark from before
    // would be stale.
    let (berry, bush) = (Pos { x: 0, y: 1 }, Pos { x: 8, y: 4 });
    let objects = [(bush, "berry_bush"), (berry, "berry")];
    let (mut world, app) = one_sprite_among(HUNGRY_GENOME, &objects, &[], 1);
    let berry_cell = app.cell_of(berry).expect("in view");
    assert_eq!(
        render(&app, &world, 100, 30)[berry_cell].bg,
        Color::DarkGray
    );
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    assert_eq!(render(&app, &world, 100, 30)[berry_cell].bg, Color::Reset);
}

#[test]
fn the_brain_tab_shows_memory_before_the_first_decision() {
    // Design v17 §6.1. A scripted bite at tick 0 hurts; at tick 1, still on
    // its scripted rest, it learns thornbushes are bad: .8 × a punishment of 1.
    let objects = [(Pos { x: 3, y: 3 }, "thornbush")];
    let scripted = [
        ScriptedAction::Eat {
            at: Pos { x: 3, y: 3 },
        },
        ScriptedAction::Rest,
    ];
    let (world, mut app) = one_sprite_among(THORN_GENOME, &objects, &scripted, 2);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..4],
        [
            "Nothing decided yet",
            "",
            "MEMORY",
            "thornbushes are bad                 -.80",
        ]
    );
}

#[test]
fn the_brain_tab_shows_what_the_sprite_thinks_of_a_category_once_it_counts() {
    // Design v19 §5.6, §6.1: a bite of a thornbush makes it −.8 bad, and a
    // bite of a berry bush with no fruit teaches nothing, but the sprite
    // knows both types of bush now: bushes are the mean, −.4, at half
    // strength, −.2. A category reads as its own plural.
    let (thornbush, bush) = (Pos { x: 3, y: 3 }, Pos { x: 1, y: 3 });
    let objects = [(thornbush, "thornbush"), (bush, "berry_bush")];
    let scripted = [
        ScriptedAction::Eat { at: thornbush },
        ScriptedAction::Eat { at: bush },
        ScriptedAction::Rest,
    ];
    let genome = r#"(format: 1, genes: [
        Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
    ])"#;
    let (world, mut app) = one_sprite_among(genome, &objects, &scripted, 3);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        text.contains(&"bushes are bad                      -.20".to_string()),
        "{text:?}"
    );
}

#[test]
fn a_lesson_about_a_category_words_it_as_the_category_names_itself() {
    // Design v19 §3.5.5, §6.1: bushes are counted; fruit isn't, so it
    // keeps its name and takes "is".
    let world = garden(pack());
    let bushes_bad = Learned::Bad {
        thing: Thing::Category("bush".into()),
    };
    let fruit_for_hunger = Learned::Worth {
        thing: Thing::Category("fruit".into()),
        need: Some("hunger".into()),
    };
    let log = logged_lessons(
        &world,
        vec![
            (30, 12, bushes_bad, false),
            (31, 12, fruit_for_hunger, true),
        ],
    );
    assert_eq!(
        log,
        [
            "31  Sprite #12 learned: fruit is good for hunger",
            "30  Sprite #12 learned: bushes are bad",
        ]
    );
}

#[test]
fn the_brain_tab_says_a_led_sprite_decides_nothing() {
    // Design v23 §2.4: led, it makes no decisions, so the one before it was
    // taken hold of would mislead.
    let (mut world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    assert_ne!(
        inspector(&app, &world).1[0],
        "Being led: it decides nothing"
    );
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    assert_eq!(
        inspector(&app, &world).1[0],
        "Being led: it decides nothing"
    );
}

#[test]
fn the_brain_tab_says_when_nothing_has_been_decided_or_is_in_sight() {
    let (world, mut app) = one_sprite(HUNGRY_GENOME, 0);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(text[0], "Nothing decided yet");

    let (world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..4],
        [
            "ATTENTION",
            "nothing in sight",
            "",
            "DECISION: WANDER                       .30",
        ]
    );
}

#[test]
fn the_chem_tab_lists_every_chemical_with_its_level_and_change_per_tick() {
    let (world, mut app) = one_sprite(WORKED_GENOME, 1);
    open(&mut app, &world, Tab::Chem);
    let (top, text) = inspector(&app, &world);
    assert!(top.contains("[Chem]"), "{top:?}");
    assert_eq!(
        text,
        [
            // One tick at rest: basal metabolism with sense radius 9.5 costs
            // .00016 of energy, and hydration loses .00033 (Appendix B).
            "energy       1.00  -.0002",
            "hydration    1.00  -.0003",
            "stamina      1.00",
            "food          .00",
            "water         .00",
            "injury        .00",
            "",
            "hunger        .40  -.4000",
            "thirst        .18",
            "pain          .00",
            "tiredness     .39",
            "boredom       .30  +.1000",
            "loneliness    .00",
            "crowdedness   .00",
            // Nothing felt: learning used up no reward or punishment.
            "reward        .00  felt +.00",
            "punishment    .00",
            "HORMONES",
            "h0   .00   h1   .00   h2   .00   h3   .00",
            "h4   .00   h5   .00   h6   .00   h7   .00",
            "h8   .00   h9   .00   h10  .00   h11  .00",
            "h12  .00   h13  .00   h14  .00   h15  .00",
        ],
        "all of it fits at 100×30"
    );
}

/// A hungry sprite drawn to bushes and to eating them, even more so
/// beside one, whose every prick punishes it by 1.
const THORN_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 1.0),
    Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "bush", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
    Instinct(inputs: [("hunger", false), ("target_adjacent", false)], verb: Eat, weight: 1.0),
])"#;

#[test]
fn the_brain_tab_names_a_thing_s_worth_neutrally_whatever_its_sign() {
    // Its bite at tick 0 pricks, and at tick 1 it learns thornbushes are bad;
    // so hungry, it bites again, and that worth takes from biting. The row
    // reads "worth: thornbush" whatever its sign, beside "habit: …".
    let objects = [(Pos { x: 3, y: 3 }, "thornbush")];
    let (world, mut app) = one_sprite_among(THORN_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        text.iter().any(|row| row.starts_with("worth: thornbush")),
        "{text:?}"
    );
    assert!(!text.iter().any(|row| row.contains("worth it")), "{text:?}");
}

#[test]
fn the_chem_tab_shows_what_was_felt_on_the_reward_line() {
    // The bite at tick 0 pricks; at tick 1 learning uses up a punishment of 1.
    let objects = [(Pos { x: 3, y: 3 }, "thornbush")];
    let (world, mut app) = one_sprite_among(THORN_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Chem);
    let (_, text) = inspector(&app, &world);
    let row = |name: &str| text.iter().find(|r| r.starts_with(name)).cloned();
    assert_eq!(
        row("reward").as_deref(),
        Some("reward        .00  felt -1.00")
    );
    assert_eq!(row("punishment").as_deref(), Some("punishment    .00"));
}

#[test]
fn the_genome_tab_marks_genes_naming_a_category_the_world_lacks() {
    // Unmatched genes (design v19 §5.7), dimmed with their reason as the
    // other marks are: an attention instinct and an instinct by name, and a
    // gene by number, which shows as its number.
    let genome = r#"(format: 1, genes: [
        AttentionInstinct(input: "hunger", category: "tree", weight: 0.8),
        Instinct(inputs: [("hunger", false), ("attended_tree", false)], verb: Eat, weight: 0.5),
        Gene(type: 9, version: 1, payload: "93011aca3f4ccccd"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 40), 46);
    let text = rows[2..30]
        .iter()
        .map(|row| inside(row))
        .collect::<Vec<_>>()
        .join(" ");
    for gene in [
        "hunger & attended tree → eat +.5",
        "hunger → attends to tree +.8",
        "type 9, version 1, 8 bytes",
    ] {
        assert!(text.contains(gene), "{gene:?} in {text:?}");
    }
    let reason = "unmatched: names a category this world doesn't have";
    assert_eq!(text.matches(reason).count(), 3, "{text:?}");
}

#[test]
fn the_genome_tab_shows_brain_settings_instincts_and_attention_instincts() {
    let genome = r#"(format: 1, genes: [
        AttentionInstinct(input: "hunger", category: "bush", weight: 0.8),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.2),
        Instinct(inputs: [("thirst", false), ("target_adjacent", true)], verb: Drink, weight: -0.5),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 30), 46);
    assert!(rows[1].contains("[Genome]"), "{:?}", rows[1]);
    let text: Vec<&str> = rows[2..11].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "BRAIN SETTINGS",
            "tau base .2",
            "INSTINCTS",
            "hunger → eat +1",
            "thirst & not target adjacent → drink -.5",
            "ATTENTION INSTINCTS",
            "hunger → attends to bush +.8",
            "",
            "",
        ]
    );
}

#[test]
fn the_genome_tab_groups_genes_as_plain_lines_and_marks_those_with_no_effect() {
    let genome = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 7.25),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.00428, chem: "hunger"),
        HalfLife(chem: "hunger", ticks: 2041),
        Trait(trait: "sense_radius", value: 9.5),
        Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
        Emitter(locus: Chem("hunger"), mode: Fall, threshold: 0.0198, gain: 1.02, chem: "reward"),
        Emitter(locus: Chem("injury"), mode: Rise, gain: 10.0, chem: "pain"),
        Emitter(locus: Locus("nearby_sprites"), mode: Level, invert: true, threshold: 0.5, gain: 0.0005, chem: "loneliness"),
        Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food"),
        HalfLife(chem: "hunger", ticks: 20),
        Trait(trait: "lifespan", value: 61204.0),
        Reaction(reactants: [("h0", 2)], products: [("h1", 1), ("h2", 1)], rate: 0.1),
        Receptor(chem: "reward", threshold: 0.2, gain: 0.5, target: "learning_rate_mod"),
        Trait(trait: "speed", value: 9.0),
        InitialConcentration(chem: "boredom", value: 0.2),
        Gene(type: 900, version: 1, payload: "c0ffee"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let screen = render(&app, &world, 100, 40);
    let rows = right_part(&screen, 46);
    assert!(rows[1].contains("[Genome]"), "{:?}", rows[1]);
    let text: Vec<&str> = rows[2..30].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "TRAITS",
            "speed 7.25 · sense 9.5 · lifespan 61,204",
            "speed 9",
            "unexpressed: an earlier gene sets this",
            "HALF-LIVES",
            "hunger halves every 2,041 ticks",
            "hunger halves every 20 ticks",
            "unexpressed: an earlier gene sets this",
            "REACTIONS",
            "2 h0 → h1 + h2, rate .1",
            "EMITTERS",
            "low energy → hunger +.00428 past .5",
            "ate → hunger -.5",
            "hunger falls → reward +1.02 past .0198",
            "injury rises → pain +10",
            // Too long for one line, it wraps, keeping "past" with its number.
            "low nearby sprites → loneliness +.0005",
            "past .5",
            "ate → food +.3",
            "flagged: writes food, but only physiology",
            "and verbs change a physical chemical",
            "RECEPTORS",
            "reward past .2 → learning rate mod +.5",
            "STARTING LEVELS",
            "boredom starts at .2",
            "UNKNOWN GENES",
            "type 900, version 1, 3 bytes",
            "unknown: this version can't read it",
            "",
        ]
    );
    // A gene with no effect is dimmed, and so is its reason, indented under it.
    let row_of = |text: &str| {
        2 + rows[2..]
            .iter()
            .position(|r| inside(r) == text)
            .expect(text)
    };
    let (x, flagged) = (100 - 46 + 2, row_of("ate → food +.3") as u16);
    assert_eq!(screen[(x, flagged)].fg, Color::DarkGray);
    assert_eq!(
        screen[(x, flagged + 1)].symbol(),
        " ",
        "the reason is indented"
    );
    assert_eq!(screen[(x + 2, flagged + 1)].symbol(), "f");
    assert_eq!(screen[(x + 2, flagged + 1)].fg, Color::DarkGray);
    let expressed = row_of("ate → hunger -.5") as u16;
    assert_eq!(screen[(x, expressed)].fg, Color::Reset);
}

/// A genome whose Genome tab is 61 lines: the heading, then 60 emitters
/// whose gains, .001 to .060, tell them apart.
fn long_genome() -> String {
    let emitters: Vec<String> = (1..=60)
        .map(|n| {
            let gain = n as f32 / 1000.0;
            format!(
                r#"Emitter(locus: Locus("always"), mode: Level, gain: {gain}, chem: "boredom")"#
            )
        })
        .collect();
    format!("(format: 1, genes: [{}])", emitters.join(", "))
}

/// The first and last lines the inspector shows.
fn first_and_last(app: &App, world: &World) -> (String, String) {
    let (_, text) = inspector(app, world);
    (text[0].clone(), text[text.len() - 1].clone())
}

#[test]
fn page_down_and_up_scroll_a_long_tab_a_page_and_stop_at_either_end() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    // 21 rows fit at 100×30, so the tab scrolls at most 40 lines.
    let mut page = |pages: i32| {
        app.apply(Action::ScrollTab { pages }, &world);
        first_and_last(&app, &world)
    };
    let shown = |first: &str, last: &str| (first.to_string(), last.to_string());
    let gain = |gain: &str| format!("always → boredom +{gain}");
    assert_eq!(page(1), shown(&gain(".021"), &gain(".041")), "a page down");
    assert_eq!(
        page(1),
        shown(&gain(".04"), &gain(".06")),
        "no further than the end"
    );
    assert_eq!(page(-1), shown(&gain(".019"), &gain(".039")), "a page up");
    assert_eq!(
        page(-1),
        shown("EMITTERS", &gain(".02")),
        "no further than the top"
    );
}

#[test]
fn the_wheel_over_the_inspector_scrolls_3_lines_a_notch_and_elsewhere_does_not() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    let over_inspector = Position::new(80, 10);
    app.apply(
        Action::Wheel {
            at: over_inspector,
            notches: 2,
        },
        &world,
    );
    assert_eq!(first_and_last(&app, &world).0, "always → boredom +.006");
    app.apply(
        Action::Wheel {
            at: over_inspector,
            notches: -1,
        },
        &world,
    );
    assert_eq!(first_and_last(&app, &world).0, "always → boredom +.003");
    assert_eq!(app.mode(), CursorMode::Select, "the mode stays");
    app.apply(
        Action::Wheel {
            at: Position::new(4, 4),
            notches: 2,
        },
        &world,
    );
    assert_eq!(
        first_and_last(&app, &world).0,
        "always → boredom +.003",
        "over the map"
    );
    assert_eq!(
        app.cursor(),
        terra_sim::Pos { x: 2, y: 1 },
        "a wheel event points, like every mouse event"
    );
}

#[test]
fn a_tab_goes_back_to_the_top_when_the_tab_or_the_selection_changes() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    app.apply(Action::NextTab, &world);
    app.apply(Action::PreviousTab, &world);
    assert_eq!(
        first_and_last(&app, &world).0,
        "EMITTERS",
        "after switching tabs"
    );
}

#[test]
fn selecting_another_sprite_starts_its_tab_from_the_top_and_the_same_one_again_does_not() {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let genome = || Some(terra_sim::Genome::from_ron(&long_genome(), &pack).expect("valid"));
    let sprites = [
        (terra_sim::Pos { x: 2, y: 3 }, genome()),
        (terra_sim::Pos { x: 6, y: 1 }, genome()),
    ];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    let world = World::from_scenario(scenario, pack.clone(), 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3).
    app.apply(Action::left_click(pointer_on(2, 3)), &world);
    assert_eq!(
        first_and_last(&app, &world).0,
        "always → boredom +.021",
        "the same sprite again"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(first_and_last(&app, &world).0, "EMITTERS", "another sprite");
}

#[test]
fn after_the_inspector_grows_page_up_scrolls_from_what_is_shown() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 2 }, &world); // the end: line 40 at the top
    // At 100×40 the tab has 31 rows, so the end is line 30 at the top.
    app.fit(ui::areas(Size::new(100, 40), world.map()));
    app.apply(Action::ScrollTab { pages: -1 }, &world);
    let rows = right_part(&render(&app, &world, 100, 40), 46);
    assert_eq!(inside(&rows[2]), "EMITTERS", "a page up from line 30");
}

#[test]
fn a_change_too_small_to_show_leaves_no_number_and_no_arrow() {
    // Boredom gains .00005 a tick, which rounds to .0000 at 4 decimals.
    let genome = r#"(format: 1, genes: [
        Emitter(locus: Locus("always"), mode: Level, gain: 0.00005, chem: "boredom"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 1);
    let (_, body) = inspector(&app, &world);
    // After the action line, age, traits, a blank line and four drives.
    assert_eq!(body[8], "boredom     ░░░░░░░░░░ .00", "the Body tab");
    open(&mut app, &world, Tab::Chem);
    let (_, chem) = inspector(&app, &world);
    assert_eq!(chem[11], "boredom       .00", "the Chem tab");
}

#[test]
fn big_and_negative_gene_values_keep_3_significant_figures_and_their_sign() {
    let genome = r#"(format: 1, genes: [
        Trait(trait: "lifespan", value: -1234.5),
        Emitter(locus: Locus("always"), mode: Level, gain: 1234.5, chem: "boredom"),
        Emitter(locus: Locus("always"), mode: Level, gain: -98765.0, chem: "boredom"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..5],
        [
            "TRAITS",
            "lifespan -1,235",
            "EMITTERS",
            "always → boredom +1,230",
            "always → boredom -98,800",
        ]
    );
}

#[test]
fn every_tab_s_text_is_within_cp437() {
    // Design §6.2: any CP437 font or tileset can draw every panel.
    let mut world = generated_world();
    for _ in 0..500 {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    app.apply(Action::SelectNext, &world);
    for _ in terra_tui::app::Tab::ALL {
        let screen = render(&app, &world, 100, 40);
        for line in lines(&screen) {
            for c in line.chars() {
                assert!(terra_tui::cp437::contains(c), "{c:?} in {line:?}");
            }
        }
        app.apply(Action::NextTab, &world);
    }
}

#[test]
fn the_world_tab_shows_the_population_and_the_deaths_by_cause() {
    // Water runs out in two ticks, and then dehydration kills in two more.
    let physiology = include_str!("../../../data/physiology.ron")
        .replace("hydration_loss: 0.00033", "hydration_loss: 0.5")
        .replace("dehydration: 0.0011", "dehydration: 0.5");
    let pack =
        DataPack::from_sources(&builtin_with("physiology.ron", &physiology)).expect("valid pack");
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let at = |x, y| (terra_sim::Pos { x, y }, None);
    let sprites = [at(1, 1), at(4, 2), at(8, 3)];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[1..4],
        [
            "sprites           3",
            "deaths            0",
            "starvation 0 · dehydration 0 · old age 0",
        ]
    );
    for _ in 0..10 {
        world.step();
    }
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[1..4],
        [
            "sprites           0",
            "deaths            3",
            "starvation 0 · dehydration 3 · old age 0",
        ]
    );
    assert_eq!(text[4], "berry bush        0", "then the objects");
}

#[test]
fn a_death_message_too_long_for_one_line_wraps() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    app.record(
        &[died(4_012, id.0, DeathCause::Dehydration, 12_345_678)],
        &world,
    );
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..3],
        [
            format!("Sprite #{} died of dehydration at", id.0),
            "age 12,345,678".to_string(),
            String::new(),
        ],
        "a line break keeps \"age\" with its number"
    );
}

#[test]
fn a_world_tab_list_too_long_for_one_line_wraps_after_a_dot() {
    let objects = include_str!("../../../data/objects.ron")
        .replace("\"mature\"", "\"fully_grown_and_bearing_fruit\"");
    let world =
        garden(DataPack::from_sources(&builtin_with("objects.ron", &objects)).expect("valid pack"));
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let rows = right_part(&render(&app, &world, 100, 30), 46);
    let text: Vec<&str> = rows[2..10]
        .iter()
        .map(|row| row.trim_start_matches('│').trim_end_matches('│').trim_end())
        .collect();
    assert_eq!(
        text[4..7],
        [
            " berry bush        1",
            "   seedling 1 ·",
            "   fully_grown_and_bearing_fruit 0",
        ]
    );
}

#[test]
fn selecting_the_selected_sprite_from_a_scrolled_world_tab_opens_body_at_the_top() {
    // At 100×12 the inspector has 8 rows: too few for the World tab's 10
    // lines, or the Body tab's 13.
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 12);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::PreviousTab, &world);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3).
    app.apply(Action::left_click(pointer_on(2, 3)), &world);
    let rows = right_part(&render(&app, &world, 100, 12), 46);
    assert!(rows[1].contains("[Body]"), "{:?}", rows[1]);
    assert!(inside(&rows[2]).starts_with("age "), "{:?}", rows[2]);
}

#[test]
fn chemical_names_show_with_spaces_for_underscores() {
    // "boredom" renamed "bored_ness" in the pack, its brain inputs and the starter genome.
    let files: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let renamed = match path {
                "chemicals.ron" | "brain_io.ron" | "genomes/starter.ron" => {
                    text.replace("\"boredom\"", "\"bored_ness\"")
                }
                _ => text.to_string(),
            };
            (path, renamed)
        })
        .collect();
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
    let pack = DataPack::from_sources(&files).expect("valid pack");
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let sprites = [(terra_sim::Pos { x: 2, y: 3 }, None)];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    let world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let (_, body) = inspector(&app, &world);
    assert!(body[7].starts_with("bored ness  "), "{:?}", body[7]);
    open(&mut app, &world, Tab::Chem);
    let (_, chem) = inspector(&app, &world);
    assert!(chem[11].starts_with("bored ness  "), "{:?}", chem[11]);
}

/// The Body tab's lines from "Observed" to the end, on a screen tall
/// enough to show it all, keeping their leading spaces.
fn observed_section(app: &App, world: &World) -> Vec<String> {
    let rows = right_part(&render(app, world, 100, 60), 46);
    let text: Vec<String> = rows
        .iter()
        .map(|row| {
            let inner = row.trim_start_matches('│').trim_end_matches('│');
            inner.trim_end().to_string()
        })
        .collect();
    let start = text
        .iter()
        .position(|t| t == " Observed")
        .expect("the section");
    let end = text[start..]
        .iter()
        .position(|t| t.is_empty() || t.starts_with('└'))
        .map_or(text.len(), |n| start + n);
    text[start..end].to_vec()
}

/// Sprite `id` finished `verb`, aimed at nothing, with `outcome`, on `tick`.
fn finished(tick: u64, id: EntityId, verb: Verb, outcome: Outcome) -> Event {
    let action = ActionView {
        verb,
        destination: None,
        target: None,
        target_type: None,
        attempted: false,
        target_gone: false,
        hurt: Hurt::default(),
        progress: Progress::Ended(outcome),
    };
    Event {
        tick,
        kind: EventKind::ActionEnded {
            id,
            verb,
            outcome,
            action,
        },
    }
}

#[test]
fn the_body_tab_ends_with_what_was_observed_and_how_long_ago() {
    let (mut world, _) = one_sprite(WORKED_GENOME, 0);
    let mut app = app_for(&world, Theme::cp437(), 100, 60);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        observed_section(&app, &world),
        [" Observed", "   nothing yet"]
    );
    let id = world.sprites().next().expect("the sprite").id();
    for _ in 0..1_300 {
        world.step();
    }
    // The world has run ticks 0 to 1,299.
    app.record(
        &[
            finished(3, id, Verb::Rest, Outcome::Applied),
            finished(1_280, id, Verb::Wander, Outcome::Applied),
            finished(1_290, id, Verb::Wander, Outcome::Applied),
            finished(1_291, id, Verb::Wander, Outcome::Blocked),
            finished(1_299, id, Verb::Rest, Outcome::Applied),
        ],
        &world,
    );
    // Newest first; the times right-aligned, and a long line wrapped under
    // its text.
    assert_eq!(
        observed_section(&app, &world),
        [
            " Observed",
            "        just now · Rested",
            "     8 ticks ago · Wandered off, but gave",
            "                   up: the way was blocked",
            "     9 ticks ago · Wandered off ×2",
            " 1,296 ticks ago · Rested",
        ]
    );
}

#[test]
fn a_hurt_sprite_takes_turns_with_a_red_bang_for_a_second() {
    for theme in [Theme::cp437(), Theme::ascii()] {
        let world = garden_with_sprites();
        let mut app = app_for(&world, theme, 100, 30);
        let biter = world.sprite_at(Pos { x: 2, y: 3 }).expect("a sprite");
        let other = world.sprite_at(Pos { x: 7, y: 1 }).expect("a sprite");
        let (biter, other) = (biter.id(), other.id());
        let biter_cell = app.cell_of(Pos { x: 2, y: 3 }).expect("in view");
        let other_cell = app.cell_of(Pos { x: 7, y: 1 }).expect("in view");
        let sprite_glyph = render(&app, &world, 100, 30)[biter_cell]
            .symbol()
            .to_string();
        // A thornbush bite, which hurts the biter, and a harmless kick.
        let thorns = (Target::Object(EntityId(77)), 3);
        let ball = (Target::Object(EntityId(40)), 4);
        app.record(
            &[
                acted(1, biter.0, Verb::Eat, thorns, Outcome::Applied, HURT_ITSELF),
                acted(1, other.0, Verb::Play, ball, Outcome::Applied, UNHURT),
            ],
            &world,
        );
        let at = |app: &App, cell| {
            let screen = render(app, &world, 100, 30);
            let cell = &screen[cell];
            (cell.symbol().to_string(), cell.fg)
        };
        // A quarter of a second the bang, a quarter the sprite, for a second.
        let bang = ("!".to_string(), Color::Red);
        assert_eq!(at(&app, biter_cell), bang);
        assert_eq!(at(&app, other_cell).0, sprite_glyph, "a kick hurts nobody");
        app.animate(Duration::from_millis(250));
        assert_eq!(at(&app, biter_cell).0, sprite_glyph);
        app.animate(Duration::from_millis(250));
        assert_eq!(at(&app, biter_cell), bang);
        app.animate(Duration::from_millis(500));
        assert_eq!(at(&app, biter_cell).0, sprite_glyph, "and then it's over");
        app.animate(Duration::from_millis(250));
        assert_eq!(at(&app, biter_cell).0, sprite_glyph);

        // A hit hurts the sprite hit, not the hitter.
        let target = Hurt {
            actor: false,
            target: true,
        };
        let hit = acted(
            2,
            biter.0,
            Verb::Hit,
            (Target::Sprite(other), 101),
            Outcome::Applied,
            target,
        );
        app.record(&[hit], &world);
        assert_eq!(at(&app, other_cell), bang);
        assert_eq!(at(&app, biter_cell).0, sprite_glyph);
    }
}

/// A sprite that attends to sprites, would rather rest a little, and feels
/// every hit as a punishment of 1.
const SKITTISH_GENOME: &str = r#"(format: 1, genes: [
    Trait(trait: "speed", value: 10.0),
    Trait(trait: "sense_radius", value: 10.0),
    BrainParam(param: "tau_base", value: 0.05),
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false)], verb: Rest, weight: 0.3),
    Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
])"#;

/// A skittish sprite, #1, hit by a bully, #2, that then rests beside it,
/// `ticks` ticks on, with an app that logged every tick and selects #1.
fn hit_by_a_bully(ticks: u32) -> (World, App) {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let genome = |text: &str| terra_sim::Genome::from_ron(text, &pack).expect("a valid genome");
    let walker = r#"(format: 1, genes: [Trait(trait: "speed", value: 10.0)])"#;
    let (me, bully) = (Pos { x: 2, y: 3 }, Pos { x: 3, y: 3 });
    let sprites = [
        (me, Some(genome(SKITTISH_GENOME))),
        (bully, Some(genome(walker))),
    ];
    let scripted = [
        (bully, ScriptedAction::Hit { at: me }),
        (bully, ScriptedAction::Rest),
    ];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &scripted,
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    for _ in 0..ticks {
        let events = world.step();
        app.record(&events, &world);
    }
    app.apply(Action::SelectNext, &world);
    (world, app)
}

#[test]
fn the_brain_tab_names_the_sprite_it_attends_to_and_how_frightening_it_is() {
    // Design v18 §6.1. Hit at tick 0, it learns at tick 1 to fear the
    // bully, −1 × fear_fade's little; from tick 2 it backs away from it:
    // flight (.8) × its fear, with the bully beside it.
    let (world, mut app) = hit_by_a_bully(3);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let row = |start: &str| text.iter().find(|row| row.starts_with(start)).cloned();
    assert!(text[1].starts_with("► Sprite #2"), "{text:?}");
    assert!(
        row("DECISION: RETREAT").is_some_and(|r| r.ends_with(".80")),
        "{text:?}"
    );
    assert!(
        row("fear: Sprite #2").is_some_and(|r| r.ends_with("+.80")),
        "{text:?}"
    );
    assert!(
        row("Sprite #2 is frightening").is_some_and(|r| r.ends_with("-1.00")),
        "{text:?}"
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(
        screen
            .iter()
            .any(|row| row.contains("Sprite #1 learned: Sprite #2 is frightening")),
        "{screen:?}"
    );
}

#[test]
fn the_event_log_says_when_sprites_in_general_turn_frightening() {
    // Design v18 §6.1.
    let world = garden(pack());
    let log = logged_lessons(
        &world,
        vec![(
            40,
            3,
            Learned::Fear {
                thing: Thing::Category("sprite".into()),
            },
            false,
        )],
    );
    assert_eq!(log, ["40  Sprite #3 learned: sprites are frightening"]);
}

#[test]
fn the_status_line_names_train_mode_and_what_the_cursor_follows() {
    // Design v21 §6.1: tiles are drawn from screen cell (1, 2).
    let world = garden_with_sprites();
    let id = world.sprite_at(Pos { x: 2, y: 3 }).expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let on_it = pointer_on(2, 3);
    app.apply(Action::left_click(on_it), &world);
    app.apply(Action::middle_click(on_it), &world);
    app.apply(Action::Mode(CursorMode::Train), &world);
    let status = status_line(&app, &world);
    let expected = format!(
        " (2,3) grass · Sprite #{0} │ TRAIN │ following Sprite #{0}",
        id.0
    );
    assert!(status.starts_with(&expected), "{status}");
}

#[test]
fn the_status_line_says_what_the_cursor_holds_or_leads_in_every_mode() {
    // Design v23 §6.1. The sprite is on map tile (2, 3), the ball on (6, 3).
    let world = garden_with_sprites();
    let sprite = world
        .sprite_at(terra_sim::Pos { x: 2, y: 3 })
        .expect("a sprite")
        .id();
    let cases = [
        (
            pointer_on(2, 3),
            format!(" │ leading: Sprite #{}", sprite.0),
        ),
        (pointer_on(6, 3), " │ holding: ball".to_string()),
    ];
    for (on, says) in cases {
        let mut app = app_for(&world, Theme::cp437(), 100, 30);
        app.apply(Action::Mode(CursorMode::Grab), &world);
        app.apply(Action::left_click(on), &world);
        let status = status_line(&app, &world);
        assert!(before_hints(&status).ends_with(&says), "{status}");
        app.apply(Action::Mode(CursorMode::Select), &world);
        let status = status_line(&app, &world);
        assert!(
            before_hints(&status).ends_with(&says),
            "in Select: {status}"
        );
    }
}

#[test]
fn the_key_hints_lead_with_the_mode_keys() {
    // Design v22 §6.1, with Grab's from v23.
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 120, 30);
    let status = lines(&render(&app, &world, 120, 30))[29].clone();
    assert!(
        status.ends_with(
            "Z select  X train  C grab  WASD scroll  space pause  . step  +/- speed  esc quit"
        ),
        "{status}"
    );
}

#[test]
fn short_of_room_whole_key_hints_drop_from_the_end() {
    // Design v22 §6.1: at 100 columns, with a sprite under the Cursor, which
    // follows it, the mode keys and the first hints after them still show.
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let on_it = pointer_on(2, 3);
    app.apply(Action::middle_click(on_it), &world);
    app.apply(Action::Mode(CursorMode::Train), &world);
    let status = status_line(&app, &world);
    assert!(status.contains("│ following Sprite #"), "{status}");
    assert!(
        status.ends_with("  Z select  X train  C grab  WASD scroll"),
        "{status}"
    );
}

#[test]
fn a_train_click_with_nothing_to_act_on_says_so_in_the_hints_place_for_3_seconds() {
    // Design v22 §6.1: it sends nothing, so the event log has nothing to say.
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Train), &world);
    let empty = pointer_on(1, 1);
    for (button, amplified, expected) in [
        (Button::Left, false, "No sprite here to pet"),
        (Button::Left, true, "No sprite here to hug"),
        (Button::Right, false, "No sprite here to zap"),
        (Button::Right, true, "No sprite here to shock"),
    ] {
        let click = Action::Click {
            at: empty,
            button,
            amplified,
        };
        app.apply(click, &world);
        let line = status_line(&app, &world);
        assert!(line.trim_end().ends_with(expected), "{line}");
        assert!(!line.contains("WASD scroll"), "in the hints' place: {line}");
    }
    app.animate(Duration::from_millis(2_900));
    assert!(
        status_line(&app, &world).contains("No sprite here"),
        "still"
    );
    app.animate(Duration::from_millis(200));
    assert!(
        status_line(&app, &world).contains("WASD scroll"),
        "the hints are back"
    );
}

/// The Cursor's events for sprite 12, at `tick`: a pet, hug, zap or shock by
/// name, or one refused.
fn touched(tick: u64, what: &str) -> Event {
    let id = EntityId(12);
    let kind = match what {
        "pet" => EventKind::Rewarded {
            id,
            amplified: false,
        },
        "hug" => EventKind::Rewarded {
            id,
            amplified: true,
        },
        "zap" => EventKind::Corrected {
            id,
            amplified: false,
        },
        "shock" => EventKind::Corrected {
            id,
            amplified: true,
        },
        "refused pet" => EventKind::CommandRejected {
            command: Command::Reward {
                sprite: id,
                amplified: false,
                reach_back: 3,
            },
            reason: Rejection::Gone,
        },
        "refused shock" => EventKind::CommandRejected {
            command: Command::Correct {
                sprite: id,
                amplified: true,
            },
            reason: Rejection::Gone,
        },
        _ => unreachable!("{what}"),
    };
    Event { tick, kind }
}

/// The event log's first three rows once an app has recorded `events`.
fn logged(world: &World, events: &[Event]) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    for event in events {
        app.record(std::slice::from_ref(event), world);
    }
    let screen = lines(&render(&app, world, 100, 30));
    screen[25..28]
        .iter()
        .map(|row| inside(row).to_string())
        .collect()
}

#[test]
fn the_event_log_tells_the_player_what_they_did_through_the_cursor() {
    // Design v21 §6.1: spoken to the player.
    let world = garden(pack());
    let first = logged(
        &world,
        &[touched(1, "pet"), touched(2, "hug"), touched(3, "zap")],
    );
    assert_eq!(
        first,
        [
            "3  You zapped Sprite #12",
            "2  You hugged Sprite #12",
            "1  You petted Sprite #12",
        ]
    );
    let then = logged(
        &world,
        &[
            touched(4, "shock"),
            touched(5, "refused pet"),
            touched(6, "refused shock"),
        ],
    );
    assert_eq!(
        then,
        [
            "6  Couldn't shock Sprite #12: it's gone",
            "5  Couldn't pet Sprite #12: it's gone",
            "4  You shocked Sprite #12",
        ]
    );
}

/// `kinds` as events on ticks 1, 2, 3 and on.
fn on_ticks(kinds: Vec<EventKind>) -> Vec<Event> {
    (1..)
        .zip(kinds)
        .map(|(tick, kind)| Event { tick, kind })
        .collect()
}

#[test]
fn the_event_log_tells_the_player_what_they_grabbed_and_let_go() {
    // Design v23 §6.1: spoken to the player.
    let world = garden(pack());
    let (sprite, ball, berry) = (EntityId(12), EntityId(20), EntityId(21));
    let first = on_ticks(vec![
        EventKind::TookHold { sprite },
        EventKind::LetGo { sprite },
        EventKind::PickedUp {
            item: ball,
            object_type: "ball".into(),
        },
    ]);
    assert_eq!(
        logged(&world, &first),
        [
            "3  You picked up a ball",
            "2  You let go of Sprite #12",
            "1  You took hold of Sprite #12",
        ]
    );
    let then = on_ticks(vec![
        EventKind::PutDown {
            item: ball,
            object_type: "ball".into(),
            pos: Pos { x: 1, y: 1 },
        },
        EventKind::CursorEmptied {
            reason: Emptied::Removed {
                item: berry,
                object_type: "berry".into(),
                reason: Removal::Expired,
            },
        },
        // A led sprite's death has its own line, and needs no other.
        EventKind::CursorEmptied {
            reason: Emptied::Died { sprite },
        },
    ]);
    assert_eq!(
        logged(&world, &then)[..2],
        [
            "2  The berry you were holding expired",
            "1  You put the ball down",
        ]
    );
}

#[test]
fn the_event_log_says_why_a_grab_was_refused() {
    // Design v23 §6.1. A ball's stable type ID is 4, a berry's 2.
    let world = garden(pack());
    let (sprite, other) = (EntityId(12), EntityId(13));
    let refused = |command, reason| EventKind::CommandRejected { command, reason };
    let put_down = Command::PutDown {
        tile: Pos { x: 1, y: 1 },
    };
    let blocked = |blocker| Rejection::InTheWay {
        item_type: 4,
        blocker,
    };
    let first = on_ticks(vec![
        refused(Command::TakeHold { sprite }, Rejection::Gone),
        refused(
            Command::TakeHold { sprite },
            Rejection::Busy(Grip::Leads(other)),
        ),
        refused(put_down.clone(), blocked(Blocker::Object(2))),
    ]);
    assert_eq!(
        logged(&world, &first),
        [
            "3  Couldn't put the ball down: a berry is there",
            "2  Couldn't take hold of Sprite #12: you're already leading Sprite #13",
            "1  Couldn't take hold of Sprite #12: it's gone",
        ]
    );
    let then = on_ticks(vec![
        refused(put_down.clone(), blocked(Blocker::Terrain(Terrain::Rock))),
        refused(put_down, blocked(Blocker::Terrain(Terrain::DeepWater))),
        refused(Command::LetGo, Rejection::NotLeading),
    ]);
    assert_eq!(
        logged(&world, &then),
        [
            "3  Couldn't let go: you're not leading a sprite",
            "2  Couldn't put the ball down: it can't go in deep water",
            "1  Couldn't put the ball down: it can't go on rock",
        ]
    );
}

#[test]
fn the_same_line_again_merges_into_one_with_a_count() {
    // Design v21 §6.1: spam-clicking doesn't bury the log.
    let world = garden(pack());
    let mut events: Vec<Event> = (1..=10).map(|tick| touched(tick, "pet")).collect();
    events.push(touched(11, "zap"));
    assert_eq!(
        logged(&world, &events),
        [
            "11  You zapped Sprite #12",
            "10  You petted Sprite #12 ×10",
            "",
        ]
    );
}

#[test]
fn a_pet_shows_a_heart_and_a_zap_a_yellow_double_bang_for_a_second() {
    // Design v21 §6.3: Pleased and Shocked, taking turns with the sprite.
    for (theme, heart, bang) in [(Theme::cp437(), "♥", "‼"), (Theme::ascii(), "+", "/")] {
        let world = garden_with_sprites();
        let mut app = app_for(&world, theme, 100, 30);
        let (petted, zapped) = (Pos { x: 2, y: 3 }, Pos { x: 7, y: 1 });
        let ids = |pos| world.sprite_at(pos).expect("a sprite").id();
        let cells = |pos| app.cell_of(pos).expect("in view");
        let (petted_cell, zapped_cell) = (cells(petted), cells(zapped));
        let sprite_glyph = render(&app, &world, 100, 30)[petted_cell]
            .symbol()
            .to_string();
        let touch = |id, correct: bool| Event {
            tick: 1,
            kind: if correct {
                EventKind::Corrected {
                    id,
                    amplified: false,
                }
            } else {
                EventKind::Rewarded {
                    id,
                    amplified: true,
                }
            },
        };
        app.record(
            &[touch(ids(petted), false), touch(ids(zapped), true)],
            &world,
        );
        let at = |app: &App, cell| {
            let screen = render(app, &world, 100, 30);
            let cell = &screen[cell];
            (cell.symbol().to_string(), cell.fg)
        };
        assert_eq!(at(&app, petted_cell).0, heart);
        assert_eq!(at(&app, zapped_cell), (bang.to_string(), Color::Yellow));
        app.animate(Duration::from_millis(250));
        assert_eq!(at(&app, petted_cell).0, sprite_glyph);
        app.animate(Duration::from_millis(1000));
        assert_eq!(at(&app, petted_cell).0, sprite_glyph, "and then it's over");
        assert_ne!(at(&app, zapped_cell).0, bang);
    }
}

#[test]
fn lines_that_read_the_same_merge_though_their_events_differ() {
    // Design v21 §6.1: two different balls read the same, and so do pets
    // refused at different speeds.
    let world = garden(pack());
    let kick = |tick, ball| {
        let ball = (Target::Object(EntityId(ball)), 4);
        acted(tick, 12, Verb::Play, ball, Outcome::Applied, UNHURT)
    };
    let refused = |tick, reach_back| Event {
        tick,
        kind: EventKind::CommandRejected {
            command: Command::Reward {
                sprite: EntityId(12),
                amplified: false,
                reach_back,
            },
            reason: Rejection::Gone,
        },
    };
    let rows = logged(
        &world,
        &[kick(1, 40), kick(2, 41), refused(3, 3), refused(4, 20)],
    );
    assert_eq!(
        rows,
        [
            "4  Couldn't pet Sprite #12: it's gone ×2",
            "2  Sprite #12 kicked a ball ×2",
            "",
        ]
    );
}

#[test]
fn a_refused_command_says_why_in_the_hints_place_for_3_seconds() {
    // Design v22 §6.1: worded as the event log words it.
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.record(&[touched(1, "refused pet")], &world);
    let line = status_line(&app, &world);
    assert!(
        line.trim_end()
            .ends_with("Couldn't pet Sprite #12: it's gone"),
        "{line}"
    );
    assert!(!line.contains("WASD scroll"), "in the hints' place: {line}");
    app.animate(Duration::from_millis(3_100));
    assert!(
        status_line(&app, &world).contains("WASD scroll"),
        "the hints are back"
    );
}

#[test]
fn a_refusal_shows_even_when_the_status_line_is_crowded() {
    // Design v22 §6.1: following a sprite standing on a berry, the line
    // is too full for the reason, so what's under the Cursor is cut short.
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let on_it = pointer_on(7, 1);
    app.apply(Action::middle_click(on_it), &world);
    app.record(&[touched(1, "refused pet")], &world);
    let status = status_line(&app, &world);
    assert!(status.starts_with(" (7,1) grass · Sprite #"), "{status}");
    assert!(
        status.ends_with("  Couldn't pet Sprite #12: it's gone"),
        "{status}"
    );
    assert!(status.chars().count() <= 100);
}

/// A world on a 20×5 grass map with `objects`, and the app for it in Grab
/// mode, holding the item on `held`, on a 60×12 screen. A map tile (x, y) is
/// drawn at screen cell (x + 1, y + 2).
fn holding_world(objects: &[(Pos, &str)], held: Pos) -> (World, App) {
    let pack = pack();
    let map = Map::from_ascii(&["...................."; 5], &pack).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects,
        sprites: &[],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 60, 12);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(pointer_on(held.x, held.y)), &world);
    for command in app.take_commands() {
        world.submit(command.clone());
    }
    let events = world.step();
    app.record(&events, &world);
    assert!(world.cursor().holds().is_some());
    (world, app)
}

#[test]
fn a_steady_aim_line_runs_the_way_the_thing_will_go_as_far_as_it_can_over_empty_ground() {
    // Design v25 §6.5. A ball held on (12, 2) is pulled 7 tiles east: it'll
    // go west, but a ball goes at most 6 tiles. The berry on its way shows,
    // and the ball under the Cursor, where it'll be thrown from.
    let (world, mut app) = holding_world(
        &[(Pos { x: 12, y: 2 }, "ball"), (Pos { x: 8, y: 2 }, "berry")],
        Pos { x: 12, y: 2 },
    );
    app.apply(Action::right_click(pointer_on(12, 2)), &world);
    app.apply(Action::Point(pointer_on(19, 2)), &world);
    let row = |app: &App| -> String {
        lines(&render(app, &world, 60, 12))[2 + 2]
            .chars()
            .skip(1)
            .take(20)
            .collect()
    };
    assert_eq!(row(&app), "......°·•··→○←......");
    let end = &render(&app, &world, 60, 12)[(6 + 1, 2 + 2)];
    assert_eq!(end.fg, Color::Yellow);
    app.animate(Duration::from_millis(500));
    assert_eq!(row(&app), "......°·•··→○←......", "steady");
}

#[test]
fn while_aiming_a_held_item_it_is_drawn_under_the_cursor_where_it_will_be_thrown_from() {
    // Design v25 §6.5: held in place while aimed, rather than gone from the
    // map, and in reverse video, as the Cursor's target.
    let (world, mut app) = holding_world(&[(Pos { x: 12, y: 2 }, "ball")], Pos { x: 12, y: 2 });
    let under_the_cursor = |app: &App| {
        let cell = render(app, &world, 60, 12)[(12 + 1, 2 + 2)].clone();
        (
            cell.symbol().to_string(),
            cell.modifier.contains(Modifier::REVERSED),
        )
    };
    assert_eq!(under_the_cursor(&app), (".".into(), true), "held");
    app.apply(Action::right_click(pointer_on(12, 2)), &world);
    assert_eq!(under_the_cursor(&app), ("○".into(), true), "aimed");
    app.apply(Action::Back, &world);
    assert_eq!(under_the_cursor(&app), (".".into(), true), "cancelled");
}

#[test]
fn while_aiming_the_hints_say_how_to_send_it_or_cancel() {
    let (world, mut app) = holding_world(&[(Pos { x: 12, y: 2 }, "ball")], Pos { x: 12, y: 2 });
    app.apply(Action::right_click(pointer_on(12, 2)), &world);
    let status = lines(&render(&app, &world, 100, 30))[29].clone();
    assert!(
        status.ends_with("let go to throw  esc cancel"),
        "{status:?}"
    );
}

#[test]
fn the_body_tab_says_a_shoved_sprite_slides_and_how_far_it_has_to_go() {
    // Design v25 §6.1. It's shoved 3 tiles east from (2, 3), and slides a
    // tile a tick.
    let (mut world, mut app) = one_sprite_doing(SPEED_10, &[], 0);
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    world.submit(Command::Shove {
        toward: terra_sim::Dir::E,
        tiles: 3,
    });
    world.step();
    assert_eq!(inspector(&app, &world).1[0], "Shoved · 2 tiles to go");
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(inspector(&app, &world).1[0], "SHOVED → E · 2 tiles left");
}

#[test]
fn the_brain_tab_says_a_sliding_sprite_decides_nothing() {
    let (mut world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    world.submit(Command::Shove {
        toward: terra_sim::Dir::E,
        tiles: 3,
    });
    world.step();
    assert_eq!(inspector(&app, &world).1[0], "Shoved: it decides nothing");
}

#[test]
fn the_event_log_tells_of_throws_shoves_and_crashes_that_hurt() {
    // Design v25 §6.1: spoken to the player; a crash only if it hurt.
    let world = garden(pack());
    let (sprite, other, ball) = (EntityId(12), EntityId(13), EntityId(20));
    let events = on_ticks(vec![
        EventKind::Threw {
            item: ball,
            object_type: "ball".into(),
        },
        EventKind::Shoved { sprite },
        EventKind::Crashed {
            sprite,
            into: Thing::ObjectType("thornbush".into()),
            hurt: true,
        },
        EventKind::Crashed {
            sprite,
            into: Thing::Sprite(other),
            hurt: false,
        },
    ]);
    assert_eq!(
        logged(&world, &events),
        [
            "3  Sprite #12 was shoved into a thornbush and got hurt",
            "2  You shoved Sprite #12",
            "1  You threw the ball",
        ]
    );
}

#[test]
fn a_short_aim_s_end_shows_even_beside_the_cursor() {
    // Design v25 §6.5: a ball pulled one tile east goes one tile west, onto
    // the Cursor's left arm; the end mark shows there, so it isn't lost.
    let (world, mut app) = holding_world(&[(Pos { x: 12, y: 2 }, "ball")], Pos { x: 12, y: 2 });
    app.apply(Action::right_click(pointer_on(12, 2)), &world);
    app.apply(Action::Point(pointer_on(13, 2)), &world);
    let row: String = lines(&render(&app, &world, 60, 12))[2 + 2]
        .chars()
        .skip(1)
        .take(20)
        .collect();
    assert_eq!(row, "...........°○←......");
}
