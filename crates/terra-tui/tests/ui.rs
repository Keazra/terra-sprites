mod common;
use common::{
    LETHAL_THORNS, LONE, WorldBuilder, finished, group, inspector_line, inspector_rows, lines,
    open, pack, pack_changing, render_on, select, sized_app, status_line,
};

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, Blocker, Command, CopyChangeView, DataPack, DeathCause, Emptied, EntityId, Event,
    EventKind, GeneView, Grip, Hurt, Learned, Map, Outcome, Pos, Progress, Rejection, Removal,
    ScriptedAction, Target, Terrain, Thing, Verb, World, WorldConfig,
};
use terra_tui::app::{App, CursorMode, Flow, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;
use terra_tui::ui;

fn generated_world() -> World {
    World::new(WorldConfig::builtin(&pack()), pack(), 7)
}

/// A world on a drawn map, using the ascii legend.
fn drawn_world(rows: &[&str]) -> World {
    let map = Map::from_ascii(rows, &pack()).expect("valid drawing");
    World::from_map(map, pack(), 7).expect("one region")
}

/// A UI for `world` sized for a `width`×`height` screen. The game draws
/// nothing smaller than 100×30 (design §6.1), so a smaller screen is a
/// corner of one that size: see `render`.
fn app_for(world: &World, theme: Theme, width: u16, height: u16) -> App {
    sized_app(world, theme, at_least_min(width, height))
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
    let drawn = render_on(app, world, full);
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

/// The status line up to its key hints, which the gap of two spaces before
/// them marks.
fn before_hints(status: &str) -> &str {
    status.split("  ").next().expect("a status line")
}

/// The screen cell where `world`'s map tile (x, y) is drawn on a 100×30
/// screen, as `app_for` makes for any smaller one, not scrolled. A small map
/// is drawn in the middle of the map view (M2 design §11.1).
fn cell(world: &World, x: u16, y: u16) -> Position {
    let app = app_for(world, Theme::cp437(), 100, 30);
    app.cell_of(Pos { x, y }).expect("in view")
}

/// The screen cell the pointer is on to point at `world`'s map tile (x, y)
/// on a 100×30 screen, not scrolled, as `cell` places the tile.
fn pointer_on(world: &World, x: u16, y: u16) -> Position {
    let app = app_for(world, Theme::cp437(), 100, 30);
    common::pointer_on(&app, Pos { x, y })
}

/// `world`'s map as drawn on `screen`, a row a line, with the glass round it
/// (M2 design §11.2): the first line is the glass along its top, and each
/// line starts and ends with the glass at its sides.
fn map_rows(screen: &Buffer, world: &World) -> Vec<String> {
    let first = cell(world, 0, 0);
    let (width, height) = (world.map().width(), world.map().height());
    (first.y - 1..=first.y + height)
        .map(|y| {
            (first.x - 1..=first.x + width)
                .map(|x| screen[(x, y)].symbol())
                .collect()
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
fn the_map_view_takes_the_room_between_the_top_bar_and_the_event_log() {
    // M2 design §11.1: a small map no longer shrinks it.
    assert_eq!(ui::areas(Size::new(100, 30)).tiles, Rect::new(1, 2, 52, 21));
}

#[test]
fn a_small_map_is_drawn_in_the_middle_of_the_map_view_with_space_round_it() {
    // M2 design §11.1: the 10×4 map, in the 52×21 tiles of a 100×30 screen's
    // map view, has 21 columns of space on either side, its glass included,
    // 8 rows above and 9 below.
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    assert_eq!(app.cell_of(Pos { x: 0, y: 0 }), Some(Position::new(22, 10)));
    assert_eq!(app.cell_of(Pos { x: 9, y: 3 }), Some(Position::new(31, 13)));
    let screen = render(&app, &world, 100, 30);
    let corners = [(21, 9, "╔"), (32, 9, "╗"), (21, 14, "╚"), (32, 14, "╝")];
    for (x, y, pane) in corners {
        assert_eq!(screen[(x, y)].symbol(), pane, "({x}, {y})");
    }
    for (x, y) in [(20, 10), (33, 13), (22, 8), (31, 15), (1, 2), (52, 22)] {
        let star = screen[(x, y)].symbol();
        assert!(matches!(star, " " | "·" | "∙" | "*"), "space at ({x}, {y})");
    }
    // It doesn't scroll, in any direction.
    for (dx, dy) in [(-5, 0), (5, 0), (0, -5), (0, 5)] {
        app.apply(Action::Scroll { dx, dy }, &world);
        assert_eq!(app.cell_of(Pos { x: 0, y: 0 }), Some(Position::new(22, 10)));
    }
}

#[test]
fn a_small_map_in_the_cp437_theme_shows_the_select_cursor_inside_its_glass() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    // The cursor starts at the map's centre, (5, 2), and covers (4, 1) to
    // (6, 3). The glass, one tile out, is light cyan (M2 design §11.2).
    assert_eq!(
        map_rows(&screen, &world),
        [
            "╔══════════╗",
            "║....~~~~..║",
            "║..#.♦↓·~..║",
            "║..#.→~←.,,║",
            "║::..·↑♦,,,║",
            "╚══════════╝",
        ]
    );
    let corner = cell(&world, 0, 0);
    assert_eq!(screen[(corner.x - 1, corner.y - 1)].fg, Color::LightCyan);
    assert!(lines(&screen)[29].starts_with(" (5,2) shallow water │ SELECT"));
}

#[test]
fn a_small_map_in_the_ascii_theme_differs_only_in_its_glyphs() {
    // In ASCII the glass is `-`, `|` and `+` (M2 design §11.5).
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::ascii(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    assert_eq!(
        map_rows(&screen, &world),
        [
            "+----------+",
            "|....~~~~..|",
            "|..#.Sv-~..|",
            "|..#.>~<.,,|",
            "|::..-^S,,,|",
            "+----------+",
        ]
    );
}

#[test]
fn the_cursor_centre_is_reverse_video_and_its_marks_take_the_modes_colour() {
    let world = drawn_world(&SMALL_MAP);
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    // The cursor's centre is the map's, (5, 2).
    let centre = cell(&world, 5, 2);
    assert_eq!(screen[centre].symbol(), "~");
    assert!(screen[centre].modifier.contains(Modifier::REVERSED));
    assert_eq!(
        screen[centre].fg,
        Color::Cyan,
        "the tile keeps its own colour"
    );
    for (dx, dy) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        let piece = &screen[(
            centre.x.wrapping_add_signed(dx),
            centre.y.wrapping_add_signed(dy),
        )];
        assert_eq!(piece.fg, Color::White, "Select is white: ({dx}, {dy})");
        assert!(!piece.modifier.contains(Modifier::REVERSED));
    }
}

#[test]
fn in_train_mode_the_cursor_is_light_magenta_in_every_theme() {
    // Design v22 §6.5: it stands out on grass, where it mostly sits.
    let world = drawn_world(&SMALL_MAP);
    let centre = cell(&world, 5, 2);
    for theme in [Theme::cp437(), Theme::ascii()] {
        let mut app = app_for(&world, theme, 100, 30);
        app.apply(Action::Mode(CursorMode::Train), &world);
        let screen = render(&app, &world, 100, 30);
        // The cursor's mode mark, at the top left of its centre.
        assert_eq!(screen[(centre.x - 1, centre.y - 1)].fg, Color::LightMagenta);
        assert_eq!(
            screen[(centre.x, centre.y - 1)].fg,
            Color::LightMagenta,
            "and its arrows"
        );
    }
}

#[test]
fn following_the_cursor_s_arrows_are_solid() {
    // Design v22 §6.2: they clamp the sprite. The ASCII theme has no solid
    // arrows; its status line says the Cursor follows it. Following doesn't
    // select it (design v26 §6.5), so it's drawn as any sprite is.
    let world = garden_with_sprites();
    let on_it = pointer_on(&world, 2, 3);
    for (theme, expected) in [
        (Theme::cp437(), [".♦▼·......", ".►☺◄..○...", ".·▲♦......"]),
        (Theme::ascii(), [".Sv-......", ".>@<..o...", ".-^S......"]),
    ] {
        let mut app = app_for(&world, theme, 100, 30);
        app.apply(Action::middle_click(on_it), &world);
        let rows: Vec<String> = map_rows(&render(&app, &world, 100, 30), &world)[3..6]
            .iter()
            .map(|row| row.chars().skip(1).take(10).collect())
            .collect();
        assert_eq!(rows, expected);
    }
}

#[test]
fn the_status_marks_show_what_the_cursor_reports() {
    // Design v21 §6.5: a Train click flashes `+` in both status marks, top
    // right and bottom left, in the mode's colour.
    let world = garden_with_sprites();
    let on_it = pointer_on(&world, 2, 3);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Train), &world);
    app.apply(Action::left_click(on_it), &world);
    let screen = render(&app, &world, 100, 30);
    let centre = cell(&world, 2, 3);
    for at in [(centre.x + 1, centre.y - 1), (centre.x - 1, centre.y + 1)] {
        assert_eq!(screen[at].symbol(), "+", "{at:?}");
        assert_eq!(screen[at].fg, Color::LightMagenta);
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
        let screen = render(app, &world, 100, 30);
        [(centre.0 + 1, centre.1 - 1), (centre.0 - 1, centre.1 + 1)].map(|cell| {
            assert_eq!(screen[cell].fg, Color::Yellow, "{cell:?}");
            screen[cell].symbol().to_string()
        })
    };
    let (sprite, ball) = (pointer_on(&world, 2, 3), pointer_on(&world, 6, 3));
    for (on, glyph) in [(sprite, "☺"), (ball, "○")] {
        let mut app = app_for(&world, Theme::cp437(), 100, 30);
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
    let empty = pointer_on(&world, 4, 3);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(empty), &world);
    let screen = render(&app, &world, 100, 30);
    // The Cursor's centre is up and left of the pointer.
    let centre = (empty.x - 1, empty.y - 1);
    for cell in [(centre.0 + 1, centre.1 - 1), (centre.0 - 1, centre.1 + 1)] {
        assert_eq!(screen[cell].symbol(), "?", "{cell:?}");
    }
}

#[test]
fn a_flashing_dotted_leash_runs_from_the_cursor_to_the_led_sprite_over_empty_ground() {
    // Design v23 §6.5. The sprite is on map tile (1, 2) and a berry on
    // (4, 2).
    let world = WorldBuilder::grass(12, 5)
        .objects(&[(Pos { x: 4, y: 2 }, "berry")])
        .starters(&[Pos { x: 1, y: 2 }])
        .scripts(&[(Pos { x: 1, y: 2 }, ScriptedAction::Rest)])
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(pointer_on(&world, 1, 2)), &world);
    app.apply(Action::Point(pointer_on(&world, 6, 2)), &world);
    let row = |app: &App| -> String {
        map_rows(&render(app, &world, 100, 30), &world)[1 + 2]
            .chars()
            .skip(1)
            .take(12)
            .collect()
    };
    assert_eq!(row(&app), ".☺··•→.←....", "dots, and the berry over them");
    let dot = &render(&app, &world, 100, 30)[cell(&world, 2, 2)];
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
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    // Point at the top-left tile, so the cursor sits clear of the tiles checked.
    app.apply(Action::Point(pointer_on(&world, 0, 0)), &world);
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[cell(&world, 2, 0)].fg, Color::Green, "grass");
    assert_eq!(screen[cell(&world, 5, 1)].fg, Color::Blue, "deep water");
    assert_eq!(screen[cell(&world, 2, 1)].fg, Color::Gray, "rock");
}

#[test]
fn the_border_is_single_lined_on_every_side_and_the_wall_is_glass() {
    // M2 design §11.2: the glass shows where the wall is, one tile outside
    // the map, so the border needn't.
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let ends = |row: &String| {
        let chars: Vec<char> = row.chars().collect();
        (chars[0], chars[chars.len() - 1])
    };
    for (dx, dy) in [(0, 0), (-200, -200), (200, 200)] {
        app.apply(Action::Scroll { dx, dy }, &world);
        let rows = map_view_rows(&render(&app, &world, 100, 30));
        assert!(rows[0].starts_with("┌─ Map ──"), "{}", rows[0]);
        assert_eq!(ends(&rows[0]), ('┌', '┐'));
        assert!(rows[1..22].iter().all(|row| ends(row) == ('│', '│')));
        assert_eq!(ends(&rows[22]), ('└', '┘'));
        assert!(rows[22].chars().skip(1).take(52).all(|c| c == '─'));
    }
    // Scrolled to the top-left corner, the view is a quarter of itself past
    // the wall: 13 columns and 5 rows, the glass in the last of each.
    app.apply(Action::Scroll { dx: -200, dy: -200 }, &world);
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    let glass: String = rows[5].chars().skip(13).take(6).collect();
    assert_eq!(glass, "╔═════");
    let side: String = rows[6].chars().skip(13).take(3).collect();
    assert_eq!(side, "║..");
}

#[test]
fn the_glass_runs_all_round_the_map_one_tile_out() {
    // M2 design §11.2. From each corner of a big map, scrolled as far as it
    // goes, the glass's corner is a tile out from the map's, and its sides
    // run along the map's edges.
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    for (dx, dy, corner, glass, beside) in [
        (-200, -200, Pos { x: 0, y: 0 }, "╔", ["═", "║"]),
        (200, -200, Pos { x: 119, y: 0 }, "╗", ["═", "║"]),
        (-200, 200, Pos { x: 0, y: 59 }, "╚", ["═", "║"]),
        (200, 200, Pos { x: 119, y: 59 }, "╝", ["═", "║"]),
    ] {
        app.apply(Action::Scroll { dx, dy }, &world);
        let screen = render(&app, &world, 100, 30);
        let at = app.cell_of(corner).expect("the corner in view");
        let (out_x, out_y) = (
            if corner.x == 0 { at.x - 1 } else { at.x + 1 },
            if corner.y == 0 { at.y - 1 } else { at.y + 1 },
        );
        assert_eq!(screen[(out_x, out_y)].symbol(), glass, "{corner:?}");
        assert_eq!(screen[(out_x, out_y)].fg, Color::LightCyan);
        assert_eq!(screen[(at.x, out_y)].symbol(), beside[0], "{corner:?}");
        assert_eq!(screen[(out_x, at.y)].symbol(), beside[1], "{corner:?}");
        assert_eq!(screen[at].symbol(), ".", "the map's corner tile");
    }
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
    assert_eq!(i32::from(app.cursor().x), app.viewport().x + 51);
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
        app.apply(Action::Point(pointer_on(&world, x, y)), &world);
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
    assert_eq!(
        lines(&render(&app, &world, 100, 30))[29],
        " Quit? y quit the game  t title screen  any other key stays"
    );
}

#[test]
fn a_frame_bigger_than_the_fitted_view_draws_the_glass_and_space_past_the_wall() {
    // The terminal can grow between the app fitting its view and the frame
    // being drawn, so for one frame the map view can be wider and taller than
    // the part of the map the viewport has room for: it shows the map as far
    // as it goes, and the glass and space past it.
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30); // 52×21 tiles
    // Tiles (81, 44) on: a quarter of the view, 13 and 5, past the wall.
    app.apply(Action::Scroll { dx: 200, dy: 200 }, &world);
    let screen = render(&app, &world, 140, 40); // room for 92×31 tiles
    let row = |y: u16| -> String { (1..41).map(|x| screen[(x, y)].symbol()).collect() };
    for y in 2..18 {
        assert_eq!(row(y), format!("{}║", ".".repeat(39)), "row {y}");
    }
    assert_eq!(row(18), format!("{}╝", "═".repeat(39)));
    for y in 19..33 {
        for x in 41..93 {
            assert!(
                matches!(screen[(x, y)].symbol(), " " | "·" | "∙" | "*"),
                "space at ({x}, {y})"
            );
        }
    }
}

/// A 10×5 field of grass with a berry bush, a thornbush, a berry and a ball.
fn garden(pack: DataPack) -> World {
    let objects = [
        (terra_sim::Pos { x: 1, y: 1 }, "berry_bush"),
        (terra_sim::Pos { x: 4, y: 1 }, "thornbush"),
        (terra_sim::Pos { x: 7, y: 1 }, "berry"),
        (terra_sim::Pos { x: 6, y: 3 }, "ball"),
    ];
    WorldBuilder::grass(10, 5)
        .objects(&objects)
        .data(pack)
        .seed(7)
        .build()
}

/// The garden, with starter sprites at (2, 3) and on the berry at (7, 1).
fn garden_with_sprites() -> World {
    let objects = [
        (terra_sim::Pos { x: 1, y: 1 }, "berry_bush"),
        (terra_sim::Pos { x: 4, y: 1 }, "thornbush"),
        (terra_sim::Pos { x: 7, y: 1 }, "berry"),
        (terra_sim::Pos { x: 6, y: 3 }, "ball"),
    ];
    WorldBuilder::grass(10, 5)
        .objects(&objects)
        .starters(&[Pos { x: 2, y: 3 }, Pos { x: 7, y: 1 }])
        .seed(7)
        .build()
}

#[test]
fn sprites_are_drawn_with_their_theme_glyph_over_any_item() {
    let world = garden_with_sprites();
    let cp437 = pointing_at(&world, Theme::cp437(), 100, 30, 0, 4);
    let screen = render(&cp437, &world, 100, 30);
    assert_eq!(
        map_rows(&screen, &world)[2],
        "║.'..♠..☺..║",
        "the sprite on the berry"
    );
    assert_eq!(screen[cell(&world, 2, 3)].symbol(), "☺");
    let ascii = pointing_at(&world, Theme::ascii(), 100, 30, 0, 4);
    let screen = render(&ascii, &world, 100, 30);
    assert_eq!(map_rows(&screen, &world)[2], "|.'..*..@..|");
}

#[test]
fn the_selected_sprite_is_drawn_with_its_own_glyph() {
    let world = garden_with_sprites();
    let mut cp437 = pointing_at(&world, Theme::cp437(), 100, 30, 7, 1);
    cp437.apply(Action::left_click(pointer_on(&world, 7, 1)), &world);
    cp437.apply(Action::Point(pointer_on(&world, 0, 4)), &world); // the cursor out of the way
    let screen = render(&cp437, &world, 100, 30);
    assert_eq!(
        map_rows(&screen, &world)[2],
        "║.'..♠..☻..║",
        "the selected sprite"
    );
    let (selected, other) = (cell(&world, 7, 1), cell(&world, 2, 3));
    assert_eq!(screen[other].symbol(), "☺", "the other sprite");
    // In reverse video too, as a selected young sprite is (design §6.2, #282).
    assert!(screen[selected].modifier.contains(Modifier::REVERSED));
    assert!(!screen[other].modifier.contains(Modifier::REVERSED));

    // In ascii, `&` is already the berry bush, so the selected sprite is `@` in reverse video.
    let mut ascii = pointing_at(&world, Theme::ascii(), 100, 30, 7, 1);
    ascii.apply(Action::left_click(pointer_on(&world, 7, 1)), &world);
    ascii.apply(Action::Point(pointer_on(&world, 0, 4)), &world);
    let screen = render(&ascii, &world, 100, 30);
    let (selected, other) = (cell(&world, 7, 1), cell(&world, 2, 3));
    assert_eq!(screen[selected].symbol(), "@");
    assert!(screen[selected].modifier.contains(Modifier::REVERSED));
    assert!(!screen[other].modifier.contains(Modifier::REVERSED));
}

/// An app on `world` with the cursor pointed at tile `(x, y)` of a small map.
fn pointing_at(world: &World, theme: Theme, width: u16, height: u16, x: u16, y: u16) -> App {
    let mut app = app_for(world, theme, width, height);
    app.apply(Action::Point(pointer_on(world, x, y)), world);
    app
}

#[test]
fn objects_are_drawn_with_their_themes_glyph_for_their_visual_state() {
    let world = garden(pack());
    let app = pointing_at(&world, Theme::cp437(), 100, 30, 0, 4); // the cursor out of the way
    let screen = render(&app, &world, 100, 30);
    assert_eq!(map_rows(&screen, &world)[2], "║.'..♠..•..║");
    assert_eq!(
        screen[cell(&world, 6, 3)].symbol(),
        "○",
        "the ball, beside the cursor's arrows"
    );
    assert_eq!(screen[cell(&world, 1, 1)].fg, Color::Green, "a seedling");
    assert_eq!(screen[cell(&world, 4, 1)].fg, Color::Magenta, "a thornbush");
    assert_eq!(screen[cell(&world, 7, 1)].fg, Color::Red, "a berry");

    let ascii = pointing_at(&world, Theme::ascii(), 100, 30, 0, 4);
    let screen = render(&ascii, &world, 100, 30);
    assert_eq!(map_rows(&screen, &world)[2], "|.'..*..%..|");
}

#[test]
fn a_fruiting_bush_is_drawn_bold_red() {
    // A quick-growing berry bush: one tick as a seedling, a fruit every tick.
    let mut world = garden(pack_changing(&[(
        "objects.ron",
        &[
            ("ticks: (1500, 2500)", "ticks: (1, 1)"),
            ("Every(200)", "Every(1)"),
        ],
    )]));
    for _ in 0..3 {
        world.step();
    }
    let app = pointing_at(&world, Theme::cp437(), 100, 30, 0, 4);
    let bush = &render(&app, &world, 100, 30)[cell(&world, 1, 1)];
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

/// The lines the World tab starts with: the small population graph's title,
/// its six rows and axis, and a blank line (M2 design §14).
const GRAPH_LINES: usize = 9;

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
    let body: Vec<&str> = inspector[2 + GRAPH_LINES..12 + GRAPH_LINES]
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
        let areas = ui::areas(Size::new(width, height));
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
        ui::areas(Size::new(100, 30)).tiles.width,
        52,
        "from 100 columns, the inspector takes 46"
    );
}

#[test]
fn while_the_terminal_is_too_small_the_game_runs_on_and_esc_still_asks_to_quit() {
    // Design v29 §6.1: shrinking the window doesn't pause it.
    let mut world = big_world();
    let small = Size::new(80, 24);
    let mut app = sized_app(&world, Theme::cp437(), small);
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
    let screen = lines(&render_on(&app, &world, small));
    assert!(
        screen
            .iter()
            .any(|line| line.trim() == "Quit? y quit the game  t title screen  any other key stays"),
        "the quit prompt shows: {screen:?}"
    );
    assert_eq!(app.apply(Action::Confirm, &world), Flow::Quit);
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
fn a_line_that_repeats_in_the_event_log_counts_up_within_cp437() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    // Two lines that read the same, in a row.
    app.record(&[died(4_012, 31, DeathCause::Dehydration, 4_012)], &world);
    app.record(&[died(4_100, 31, DeathCause::Dehydration, 4_012)], &world);
    let screen = lines(&render(&app, &world, 100, 30));
    assert_eq!(
        inside(&screen[25]),
        "4,100  Sprite #31 died (dehydration, age 4,012) x2"
    );
    assert!(screen[25].chars().all(terra_tui::cp437::contains));
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
                        remembered: false,
                        heard: None,
                        egg_at: None,
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
        remembered: false,
        heard: None,
        egg_at: None,
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
    let watched = lessons
        .into_iter()
        .map(|(tick, id, learned, good)| (tick, id, learned, good, None))
        .collect();
    logged_watched(world, watched)
}

/// The same, each lesson with the sprite ID it was learned from watching,
/// if any (M2 design §18.4).
fn logged_watched(
    world: &World,
    lessons: Vec<(u64, u64, Learned, bool, Option<u64>)>,
) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    let events: Vec<Event> = lessons
        .into_iter()
        .map(|(tick, id, learned, good, watching)| Event {
            tick,
            kind: EventKind::LearnedMilestone {
                id: EntityId(id),
                learned,
                good,
                watching: watching.map(EntityId),
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
fn the_event_log_says_whom_a_lesson_was_learned_from_watching() {
    // M2 design §18.4.
    let world = garden(pack());
    let water_for_thirst = Learned::Worth {
        thing: "water".into(),
        need: Some("thirst".into()),
    };
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let log = logged_watched(
        &world,
        vec![
            (30, 21, water_for_thirst, true, Some(12)),
            (31, 21, thorns_bad, false, Some(12)),
        ],
    );
    assert_eq!(
        log,
        [
            "31  Sprite #21 learned from watching Sprite #12: thornbushes are bad",
            "30  Sprite #21 learned from watching Sprite #12: water is good for thirst",
        ]
    );
}

#[test]
fn a_lesson_words_an_object_type_as_it_names_itself() {
    // Design v17 §3.5.1: thornbushes renamed brambles, and balls with no
    // plural, as if they were a thing you don't count.
    let world = garden(pack_changing(&[(
        "objects.ron",
        &[
            (r#"plural: "thornbushes""#, r#"plural: "brambles""#),
            (r#" plural: "balls","#, ""),
        ],
    )]));
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

/// The inspector's rows on a 100×30 screen: its title, then the text inside
/// its border, trimmed.
fn inspector(app: &App, world: &World) -> (String, Vec<String>) {
    inspector_on(app, world, 30)
}

/// `inspector`, on a screen `height` rows tall.
fn inspector_on(app: &App, world: &World, height: u16) -> (String, Vec<String>) {
    let (title, rows) = inspector_rows(app, world, height);
    let text = rows
        .iter()
        .map(|row| row.trim_start().to_string())
        .collect();
    (title, text)
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
    let genome = terra_sim::Genome::from_ron(genome, &pack).expect("a valid genome");
    let start = Pos { x: 2, y: 3 };
    let scripted: Vec<(Pos, ScriptedAction)> = scripted.iter().map(|&a| (start, a)).collect();
    let mut world = WorldBuilder::grass(10, 5)
        .objects(objects)
        .sprites_with(vec![(start, Some(genome))])
        .scripts(&scripted)
        .data(pack)
        .seed(7)
        .build();
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
    Trait(trait: "grows_old", value: 13500.0),
    InitialConcentration(chem: "hunger", value: 0.8),
    HalfLife(chem: "hunger", ticks: 1),
    InitialConcentration(chem: "thirst", value: 0.18),
    InitialConcentration(chem: "tiredness", value: 0.39),
    InitialConcentration(chem: "boredom", value: 0.2),
    Emitter(locus: Locus("always"), mode: Level, gain: 0.1, chem: "boredom"),
])"#;

#[test]
fn the_body_tab_shows_age_life_stage_traits_drives_physical_levels_and_wear() {
    let (world, app) = one_sprite_doing(WORKED_GENOME, &[ScriptedAction::Rest], 1);
    let (_, text) = inspector_on(&app, &world, 32);
    assert_eq!(
        text[..19],
        [
            "Resting · 9 ticks left",
            "age 7,501 · adult · grows old 13,500",
            "speed 7.25 · sense 9.5",
            // Its sex, and fed and watered, fertile (M2 design §10).
            "male · fertile",
            // Where it came from, and its generation (M2 design §17.5).
            "one of the first sprites · generation 1",
            "",
            "hunger      ████░░░░░░ .40 ▼",
            "thirst      ██░░░░░░░░ .18",
            "pain        ░░░░░░░░░░ .00",
            "tiredness   ████░░░░░░ .39",
            "boredom     ███░░░░░░░ .30 ▲",
            "loneliness  ░░░░░░░░░░ .00",
            "crowdedness ░░░░░░░░░░ .00",
            "arousal     ░░░░░░░░░░ .00",
            "anxiety     ░░░░░░░░░░ .00",
            "",
            // A newborn's are full, and one tick at rest takes them only to .9998 or so.
            "energy 1.00 · hydration 1.00 · stamina 1.00",
            "food .00 · water .00 · injury .00",
            "wear .00",
        ]
    );
    assert_eq!(text[19..22], ["", "Observed", "nothing yet"]);
    assert!(text[22..].iter().all(String::is_empty), "{text:?}");
}

#[test]
fn the_body_tab_says_when_the_young_grow_up_and_the_old_are_old() {
    // M2 design §9.6: `age … · young · grows up 4,500`, then `· adult ·
    // grows old …`, then `· old`. In this pack a sprite may grow old as
    // soon as it likes.
    let pack = pack_changing(&[(
        "physiology.ron",
        &[
            ("grows_old: (6000.0, 60000.0)", "grows_old: (10.0, 60000.0)"),
            ("grows_up: (1500.0, 13500.0)", "grows_up: (10.0, 13500.0)"),
        ],
    )]);
    let genes = |grows_up: f32, grows_old: f32| {
        let genes = format!(
            r#"(format: 1, genes: [
                Trait(trait: "grows_up", value: {grows_up:?}),
                Trait(trait: "grows_old", value: {grows_old:?}),
            ])"#
        );
        terra_sim::Genome::from_ron(&genes, &pack).expect("a valid genome")
    };
    let (young, grown) = (Pos { x: 2, y: 3 }, Pos { x: 6, y: 1 });
    let sprites = vec![
        (young, Some(genes(4500.0, 13500.0))),
        (grown, Some(genes(10.0, 20.0))),
    ];
    let mut world = WorldBuilder::grass(10, 5)
        .sprites_with(sprites)
        .newborns(&[young])
        .data(pack)
        .seed(7)
        .build();
    world.step();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        inspector_line(&app, &world, "age "),
        "age 1 · young · grows up 4,500"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        inspector_line(&app, &world, "age "),
        "age 11 · adult · grows old 20"
    );
    for _ in 0..10 {
        world.step();
    }
    assert_eq!(inspector_line(&app, &world, "age "), "age 21 · old");
}

/// Changes after which water runs out at once and thirst injures badly,
/// and a bad injury scars a gene every tick, by .38 of its health (M2
/// design §9.4).
const SCARRING: common::Changes = (
    "physiology.ron",
    &[
        ("hydration_loss: 0.00033", "hydration_loss: 1.0"),
        ("dehydration: 0.0011", "dehydration: 0.3"),
        ("chance: 0.002, hit: 0.05", "chance: 1.0, hit: 0.38"),
    ],
);

#[test]
fn a_worn_sprite_grows_old_sooner_and_the_body_tab_says_its_genes_health() {
    // M2 design §9.4, §9.6. In this pack water runs out at once and thirst
    // injures badly, which wears the sprite, and a bad injury scars a gene
    // every tick: once one has weakened, the wear line says the genes'
    // average health too. Its one gene that weakens, the emitter, is whole
    // to begin with, as the wear line says: a scenario's sprite carries no
    // flaws (M2 design §12.3). The first scar takes .38 of its health.
    let genome = r#"(format: 1, genes: [
        Trait(trait: "grows_old", value: 13500.0),
        Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom"),
    ])"#;
    let (mut world, app) = one_sprite_in(pack_changing(&[SCARRING]), genome, &[], &[], 0);
    assert_eq!(
        inspector_line(&app, &world, "age"),
        "age 7,500 · adult · grows old 13,500"
    );
    assert_eq!(
        inspector_line(&app, &world, "wear"),
        "wear .00",
        "no gene weakened yet"
    );
    let vigour = |world: &World| world.sprites().next().expect("a sprite").vigour();
    while vigour(&world) == 1.0 {
        world.step();
    }
    let sprite = world.sprites().next().expect("a sprite");
    let grows_old = sprite.grows_old_at();
    assert!(grows_old < 13_499.5, "worn sooner");
    let said = format!(
        "age {} · adult · grows old {}",
        group(sprite.age()),
        group(grows_old.round() as u64)
    );
    assert_eq!(inspector_line(&app, &world, "age"), said);
    assert_eq!(inspector_line(&app, &world, "wear"), "wear .00 · genes .62");
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
fn a_sprite_with_nothing_to_do_shows_how_its_action_ended_then_what_it_watches() {
    // M2 design §19.2: the ending shows on the tick the action ended. From
    // the next, with nothing pulling as hard as the act threshold, it
    // stands, its eye on the bush beside its way.
    let genome = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        BrainParam(param: "act_threshold", value: 0.25),
    ])"#;
    let wander = ScriptedAction::Wander {
        destination: Pos { x: 7, y: 3 },
    };
    let bush = Pos { x: 5, y: 1 };
    let (mut world, mut app) = one_sprite_among(genome, &[(bush, "berry_bush")], &[wander], 5);
    assert_eq!(inspector(&app, &world).1[0], "Arrived");
    world.step();
    assert_eq!(inspector(&app, &world).1[0], "Watching the berry bush");
    app.apply(Action::ToggleDetail, &world);
    let id = world.object_at(bush).expect("the bush").id();
    assert_eq!(
        inspector(&app, &world).1[0],
        format!("STAND → berry_bush #{}", id.0)
    );
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

/// Eating a berry eases all six needs, so berries have six lessons.
const EVERY_NEED_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 1.0),
    InitialConcentration(chem: "thirst", value: 1.0),
    InitialConcentration(chem: "tiredness", value: 1.0),
    InitialConcentration(chem: "boredom", value: 1.0),
    InitialConcentration(chem: "loneliness", value: 1.0),
    InitialConcentration(chem: "crowdedness", value: 1.0),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "thirst"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "tiredness"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "boredom"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "loneliness"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "crowdedness"),
    BrainParam(param: "habit_rate", value: 0.0),
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
        .position(|row| row == "BERRIES")
        .unwrap_or_else(|| panic!("{text:?}"));
    assert_eq!(text[at - 1], "", "a blank row before it");
    assert!(
        text[at + 1].starts_with("good for hunger") && text[at + 1].ends_with("+.25"),
        "{:?}",
        text[at + 1]
    );
}

/// The inspector's text, keeping the indent the Brain tab uses.
fn brain_lines(app: &App, world: &World) -> Vec<String> {
    let rows = right_part(&render(app, world, 100, 30), 46);
    rows[2..rows.len() - 7]
        .iter()
        .map(|row| {
            row.trim_start_matches('│')
                .trim_end_matches('│')
                .trim_end()
                .to_string()
        })
        .collect()
}

/// A left click on the inspector row that contains `needle`.
fn click_brain_row(app: &mut App, world: &World, needle: &str) {
    let at = brain_cell(app, world, needle);
    app.apply(Action::left_click(at), world);
}

/// The screen cell of the inspector row that contains `needle`.
fn brain_cell(app: &App, world: &World, needle: &str) -> Position {
    let rows = brain_lines(app, world);
    let row = rows
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} not in {rows:?}"));
    // The inspector's text starts one cell in from its left border, on the
    // row under the title. On a 100-wide screen the border is at column 54.
    Position::new(55, 2 + row as u16)
}

#[test]
fn the_brain_tab_opens_the_rest_of_a_things_lessons() {
    // Six lessons about berries, all the same size, so the old list of five
    // would have dropped the last. Three show until the player opens them.
    let objects = [(Pos { x: 3, y: 3 }, "berry")];
    let (world, mut app) = one_sprite_among(EVERY_NEED_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    assert!(text.iter().any(|row| row == " ► BERRIES"), "{text:?}");
    assert!(
        text.iter().any(|row| row.contains("and 3 more")),
        "{text:?}"
    );
    assert!(
        !text.iter().any(|row| row.contains("crowdedness")),
        "{text:?}"
    );
    click_brain_row(&mut app, &world, "good for hunger");
    assert!(
        !brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness")),
        "a lesson line does not open the rest"
    );
    click_brain_row(&mut app, &world, "and 3 more");
    let text = brain_lines(&app, &world);
    assert!(text.iter().any(|row| row == " ▼ BERRIES"), "{text:?}");
    assert!(
        text.iter().any(|row| row.contains("crowdedness")),
        "{text:?}"
    );
    assert!(!text.iter().any(|row| row.contains("and ")), "{text:?}");
    click_brain_row(&mut app, &world, "BERRIES");
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("and 3 more")),
        "clicking the name shuts it"
    );
    let at = brain_cell(&app, &world, "and 3 more");
    app.apply(Action::Point(at), &world);
    app.apply(Action::Enter, &world);
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness")),
        "Enter opens the thing under the pointer"
    );
}

#[test]
fn choosing_another_sprite_shuts_lessons_that_were_open() {
    let pack = pack();
    let genome = terra_sim::Genome::from_ron(EVERY_NEED_GENOME, &pack).expect("a valid genome");
    let neighbor = terra_sim::Genome::from_ron("(format: 1, genes: [])", &pack).expect("valid");
    let (eater, bystander) = (Pos { x: 2, y: 3 }, Pos { x: 8, y: 1 });
    let sprites = vec![(eater, Some(genome)), (bystander, Some(neighbor))];
    let objects = [(Pos { x: 3, y: 3 }, "berry")];
    let mut world = WorldBuilder::grass(10, 5)
        .objects(&objects)
        .sprites_with(sprites)
        .seed(7)
        .build();
    for _ in 0..2 {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let eater_id = app.selection().expect("the eater").id();
    open(&mut app, &world, Tab::Brain);
    click_brain_row(&mut app, &world, "and 3 more");
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness"))
    );
    app.apply(Action::SelectNext, &world);
    assert_ne!(app.selection().map(|s| s.id()), Some(eater_id));
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.selection().map(|s| s.id()), Some(eater_id));
    let text = brain_lines(&app, &world);
    assert!(
        text.iter().any(|row| row.contains("and 3 more")),
        "{text:?}"
    );
    assert!(
        !text.iter().any(|row| row.contains("crowdedness")),
        "{text:?}"
    );
}

#[test]
fn sprites_in_general_sit_above_the_sprites_it_remembers() {
    // Two bullies, each a punishment of 1, so each is frightening at -1.
    // Sprites in general are half that average until it knows three: -.50,
    // and that line comes before either of them.
    let pack = pack();
    let genome = |text: &str| terra_sim::Genome::from_ron(text, &pack).expect("a valid genome");
    let walker = r#"(format: 1, genes: [Trait(trait: "speed", value: 10.0)])"#;
    let (me, first, second) = (Pos { x: 2, y: 3 }, Pos { x: 3, y: 3 }, Pos { x: 1, y: 3 });
    let sprites = vec![
        (me, Some(genome(SKITTISH_GENOME))),
        (first, Some(genome(walker))),
        (second, Some(genome(walker))),
    ];
    let scripted = [
        (me, ScriptedAction::Rest),
        (first, ScriptedAction::Hit { at: me }),
        (
            second,
            ScriptedAction::Wander {
                destination: Pos { x: 0, y: 3 },
            },
        ),
        (second, ScriptedAction::Hit { at: me }),
    ];
    let mut world = WorldBuilder::grass(10, 5)
        .sprites_with(sprites)
        .scripts(&scripted)
        .seed(7)
        .build();
    for _ in 0..8 {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    let sprites_at = text
        .iter()
        .position(|row| row == " SPRITES")
        .unwrap_or_else(|| panic!("{text:?}"));
    let general = text[sprites_at + 1].as_str();
    assert!(
        general.starts_with("   frightening") && general.ends_with("-.50"),
        "{text:?}"
    );
    let people: Vec<_> = text[sprites_at + 1..]
        .iter()
        .filter(|row| row.starts_with("   #"))
        .collect();
    assert_eq!(people.len(), 2, "{text:?}");
    assert!(
        text.iter()
            .any(|row| row.starts_with("     frightening") && row.ends_with("-1.00")),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_groups_what_it_thinks_of_new_things() {
    // A first prick is from something new, so new things are bad on their
    // own, not under the thornbush (design v16 §5.6, v40 §6.1).
    let genome = r#"(format: 1, genes: [
        Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
    ])"#;
    let thornbush = Pos { x: 3, y: 3 };
    let objects = [(thornbush, "thornbush")];
    let scripted = [ScriptedAction::Eat { at: thornbush }, ScriptedAction::Rest];
    let (world, mut app) = one_sprite_among(genome, &objects, &scripted, 2);
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    let at = text
        .iter()
        .position(|row| row == " NEW THINGS")
        .unwrap_or_else(|| panic!("{text:?}"));
    assert!(
        text[at + 1].starts_with("   bad") && text[at + 1].contains("-"),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_lists_the_cursor_on_its_own() {
    // Eating the Cursor teaches a habit about the Cursor, under its own
    // heading. It is not a sprite (design v29 §5.6, v40 §6.1).
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "hunger", value: 1.0),
        AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 3.0),
        BrainParam(param: "tau_base", value: 0.05),
    ])"#;
    let (mut world, mut app) = one_sprite(genome, 0);
    world.submit(Command::MoveCursor {
        tile: Pos { x: 3, y: 3 },
    });
    world.submit(Command::ShowCursor { visible: true });
    world.step();
    world.step();
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    let at = text
        .iter()
        .position(|row| row == " THE CURSOR")
        .unwrap_or_else(|| panic!("{text:?}"));
    assert!(
        text[at + 1].starts_with("   eating it") && text[at + 1].contains("-"),
        "{text:?}"
    );
    assert!(
        !text.iter().any(|row| row == " SPRITES"),
        "the Cursor is not under Sprites: {text:?}"
    );
}

#[test]
fn a_habit_about_sprites_sits_with_what_it_thinks_of_them_in_general() {
    // It hits two sprites, and they hit it back. The habit of hitting them,
    // and what it thinks of sprites in general, are the first step under
    // Sprites. That step is not folded at three (design v40 §6.1).
    let pack = pack();
    let hitter = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        InitialConcentration(chem: "loneliness", value: 1.0),
        Emitter(locus: Locus("did_hit"), mode: Level, gain: 0.5, chem: "reward"),
        Emitter(locus: Locus("did_hit"), mode: Level, gain: -0.5, chem: "loneliness"),
        Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
        BrainParam(param: "tau_base", value: 0.05),
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Hit, weight: 1.0),
    ])"#;
    let bully = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Hit, weight: 1.0),
    ])"#;
    let genome = |text: &str| terra_sim::Genome::from_ron(text, &pack).expect("a valid genome");
    let (me, first, second) = (Pos { x: 2, y: 3 }, Pos { x: 3, y: 3 }, Pos { x: 1, y: 3 });
    let sprites = vec![
        (me, Some(genome(hitter))),
        (first, Some(genome(bully))),
        (second, Some(genome(bully))),
    ];
    let mut world = WorldBuilder::grass(10, 5)
        .sprites_with(sprites)
        .seed(7)
        .build();
    for _ in 0..16 {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    let sprites_at = text
        .iter()
        .position(|row| row == " SPRITES")
        .unwrap_or_else(|| panic!("{text:?}"));
    let individual = text[sprites_at + 1..]
        .iter()
        .position(|row| row.starts_with("   #"))
        .unwrap_or_else(|| panic!("{text:?}"));
    let general = &text[sprites_at + 1..sprites_at + 1 + individual];
    assert!(
        general.iter().any(|row| row.starts_with("   hitting them")),
        "{text:?}"
    );
    assert!(
        general.len() > 3 && general.iter().all(|row| !row.contains("and ")),
        "sprites in general are not folded at three: {text:?}"
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
    app.apply(Action::left_click(pointer_on(&world, 0, 0)), &world);
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
            "THORNBUSHES",
            "bad                                 -.80",
        ]
    );
}

#[test]
fn the_brain_tab_lists_the_places_the_sprite_remembers() {
    // M2 design §7: a scripted drink at the water on its west eases its
    // thirst, so it remembers the water there.
    let pack = pack();
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "thirst", value: 1.0),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
    ])"#;
    let genome = terra_sim::Genome::from_ron(genome, &pack).expect("a valid genome");
    let start = Pos { x: 1, y: 3 };
    let sprites = vec![(start, Some(genome))];
    let water = Pos { x: 0, y: 3 };
    let scripted = [
        (start, ScriptedAction::Drink { at: water }),
        (start, ScriptedAction::Rest),
    ];
    let mut world = WorldBuilder::new(&["~........."; 5])
        .sprites_with(sprites)
        .scripts(&scripted)
        .seed(7)
        .build();
    for _ in 0..2 {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let at = text
        .iter()
        .position(|row| row.starts_with("PLACES"))
        .unwrap_or_else(|| panic!("{text:?}"));
    assert_eq!(
        text[at..at + 2],
        ["PLACES", "water · 1 tile W                    1.00"]
    );
}

#[test]
fn the_brain_tab_lists_the_places_best_remembered_first_below_what_it_has_learned() {
    // M2 design §7.5: it drinks at the water on its west, then eats from
    // the bush on its east, and its places fade fast, so the water is
    // remembered less well by the time it looks.
    let pack = pack();
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "thirst", value: 1.0),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
        InitialConcentration(chem: "hunger", value: 1.0),
        Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
        BrainParam(param: "place_fade", value: 0.01),
    ])"#;
    let genome = terra_sim::Genome::from_ron(genome, &pack).expect("a valid genome");
    let (start, water, bush) = (Pos { x: 1, y: 3 }, Pos { x: 0, y: 3 }, Pos { x: 4, y: 3 });
    let sprites = vec![(start, Some(genome))];
    let scripted = [
        (start, ScriptedAction::Drink { at: water }),
        (start, ScriptedAction::Eat { at: bush }),
        (start, ScriptedAction::Rest),
    ];
    let mut world = WorldBuilder::new(&["~........."; 5])
        .objects(&[(bush, "berry_bush")])
        .sprites_with(sprites)
        .scripts(&scripted)
        .seed(7)
        .build();
    world
        .start_object(bush, "mature", &[("fruit", 6)])
        .expect("the bush");
    for _ in 0..20 {
        world.step();
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let row = |starts: &str| {
        let at = text.iter().position(|row| row.starts_with(starts));
        at.unwrap_or_else(|| panic!("no {starts:?} in {text:?}"))
    };
    let places = row("PLACES");
    assert!(row("WATER") < places, "{text:?}");
    assert!(row("BERRY BUSHES") < places, "{text:?}");
    assert_eq!(
        (row("berry bush ·"), row("water ·")),
        (places + 1, places + 2),
        "{text:?}"
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
    assert!(text.iter().any(|row| row == "BUSHES"), "{text:?}");
    assert!(
        text.iter()
            .any(|row| row.starts_with("bad") && row.ends_with("-.20")),
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
    let (top, text) = inspector_on(&app, &world, 32);
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
            "arousal       .00",
            "anxiety       .00",
            // Nothing felt: learning used up no reward or punishment.
            "reward        .00  felt +.00",
            "punishment    .00",
            "HORMONES",
            "h0   .00   h1   .00   h2   .00   h3   .00",
            "h4   .00   h5   .00   h6   .00   h7   .00",
            "h8   .00   h9   .00   h10  .00   h11  .00",
            "h12  .00   h13  .00   h14  .00   h15  .00",
        ],
        "as much as fits at 100×32"
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
fn the_genome_tab_shows_whom_an_attention_instinct_is_about() {
    // M2 design §15.6: one about anything draws the eye to the category,
    // and one about whom it is, only to them. Grown-ups aren't its own
    // (§18.4); a parent showing it something is, and wraps.
    let genome = r#"(format: 2, genes: [
        AttentionInstinct(input: "arousal", category: "sprite", weight: 1.0),
        AttentionInstinct(input: "arousal", category: "sprite", whom: Kin, weight: -1.0),
        AttentionInstinct(input: "anxiety", category: "egg", whom: Family, weight: 1.0),
        AttentionInstinct(input: "anxiety", category: "sprite", whom: Young, weight: 1.0),
        AttentionInstinct(input: "always", category: "sprite", whom: Grown, weight: 0.3),
        AttentionInstinct(input: "always", category: "sprite", whom: Showing, weight: 1.0),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 30), 46);
    let text: Vec<&str> = rows[2..10].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "ATTENTION INSTINCTS",
            "arousal → attends to sprite +1",
            "arousal → attends to its kin (sprite) -1",
            "anxiety → attends to its family (egg) +1",
            "anxiety → attends to its young (sprite) +1",
            "always → attends to grown-ups (sprite) +.3",
            "always → attends to its parent showing",
            "something (sprite) +1",
        ]
    );
}

#[test]
fn the_genome_tab_shows_switches_dormant_genes_and_weakened_genes() {
    // M2 design §9.6. In this pack water runs out at once and thirst
    // injures badly, and a bad injury scars a gene every tick: the tick its
    // injury first passes .5, its one gene that weakens, the emitter, is
    // scarred to .62. Its brain setting switches off as it grows up, so in
    // an adult it's dormant.
    let genome = r#"(format: 2, genes: [
        BrainParam(param: "curiosity", value: 0.6, until: Adult),
        Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom"),
    ])"#;
    let (mut world, mut app) = one_sprite_in(pack_changing(&[SCARRING]), genome, &[], &[], 0);
    let vigour = |world: &World| world.sprites().next().expect("a sprite").vigour();
    for _ in 0..10 {
        if vigour(&world) < 1.0 {
            break;
        }
        world.step();
    }
    assert!(vigour(&world) < 1.0, "a gene was scarred");
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 30), 46);
    let text: Vec<&str> = rows[2..7].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "EMITTERS",
            "always → boredom +.001 · health .62",
            "BRAIN SETTINGS",
            "curiosity .6 · until adult",
            "dormant: switched off at this life stage",
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
        Trait(trait: "grows_old", value: 13500.0),
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
            "speed 7.25 · sense 9.5 · grows old 13,500",
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
            at: pointer_on(&world, 2, 1),
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
    let genome = || Some(terra_sim::Genome::from_ron(&long_genome(), &pack).expect("valid"));
    let sprites = vec![
        (terra_sim::Pos { x: 2, y: 3 }, genome()),
        (terra_sim::Pos { x: 6, y: 1 }, genome()),
    ];
    let world = WorldBuilder::grass(10, 5)
        .sprites_with(sprites)
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3).
    app.apply(Action::left_click(pointer_on(&world, 2, 3)), &world);
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
    app.fit(ui::areas(Size::new(100, 40)));
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
    // After the action line, age, traits, sex, where it came from, a
    // blank line and four drives.
    assert_eq!(body[10], "boredom     ░░░░░░░░░░ .00", "the Body tab");
    open(&mut app, &world, Tab::Chem);
    let (_, chem) = inspector(&app, &world);
    assert_eq!(chem[11], "boredom       .00", "the Chem tab");
}

#[test]
fn big_and_negative_gene_values_keep_3_significant_figures_and_their_sign() {
    // `lifespan` is what genome files from before M2 call `grows_old`.
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
            "grows old -1,235",
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
    let pack = pack_changing(&[(
        "physiology.ron",
        &[
            ("hydration_loss: 0.00033", "hydration_loss: 0.5"),
            ("dehydration: 0.0011", "dehydration: 0.5"),
        ],
    )]);
    let at = |x, y| Pos { x, y };
    let mut world = WorldBuilder::grass(10, 5)
        .starters(&[at(1, 1), at(4, 2), at(8, 3)])
        .data(pack)
        .seed(7)
        .build();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &world);
    let text = &text[GRAPH_LINES..];
    assert_eq!(
        text[1..5],
        [
            "sprites           3",
            // The generations of the living (M2 design §17.5).
            "generation        1",
            "deaths            0",
            "starvation 0 · dehydration 0 · old age 0",
        ]
    );
    for _ in 0..10 {
        world.step();
    }
    let (_, text) = inspector(&app, &world);
    let text = &text[GRAPH_LINES..];
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
    let world = garden(pack_changing(&[(
        "objects.ron",
        &[("\"mature\"", "\"fully_grown_and_bearing_fruit\"")],
    )]));
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, rows) = inspector_rows(&app, &world, 30);
    let text = &rows[GRAPH_LINES..8 + GRAPH_LINES];
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
    // At 100×12 the inspector has 8 rows: too few for the World tab's 19
    // lines, or the Body tab's 13.
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 12);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::PreviousTab, &world);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3).
    app.apply(Action::left_click(pointer_on(&world, 2, 3)), &world);
    let rows = right_part(&render(&app, &world, 100, 12), 46);
    assert!(rows[1].contains("[Body]"), "{:?}", rows[1]);
    // Its first line is what it's doing: nothing has moved it to act yet
    // (M2 design §19.2).
    assert!(inside(&rows[2]).starts_with("Standing"), "{:?}", rows[2]);
    assert!(inside(&rows[3]).starts_with("age "), "{:?}", rows[3]);
}

#[test]
fn chemical_names_show_with_spaces_for_underscores() {
    // "boredom" renamed "bored_ness" in the pack, its brain inputs and the starter genome.
    let renamed: &[(&str, &str)] = &[("\"boredom\"", "\"bored_ness\"")];
    let pack = pack_changing(&[
        ("chemicals.ron", renamed),
        ("brain_io.ron", renamed),
        ("genomes/starter.ron", renamed),
    ]);
    let world = WorldBuilder::grass(10, 5)
        .starters(&[Pos { x: 2, y: 3 }])
        .data(pack)
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let (_, body) = inspector(&app, &world);
    assert!(body[10].starts_with("bored ness  "), "{:?}", body[10]);
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
            "     9 ticks ago · Wandered off x2",
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
    let genome = |text: &str| terra_sim::Genome::from_ron(text, &pack).expect("a valid genome");
    let walker = r#"(format: 1, genes: [Trait(trait: "speed", value: 10.0)])"#;
    let (me, bully) = (Pos { x: 2, y: 3 }, Pos { x: 3, y: 3 });
    let sprites = vec![
        (me, Some(genome(SKITTISH_GENOME))),
        (bully, Some(genome(walker))),
    ];
    let scripted = [
        (bully, ScriptedAction::Hit { at: me }),
        (bully, ScriptedAction::Rest),
    ];
    let mut world = WorldBuilder::grass(10, 5)
        .sprites_with(sprites)
        .scripts(&scripted)
        .seed(7)
        .build();
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
    let kept = brain_lines(&app, &world);
    assert!(kept.iter().any(|row| row == " SPRITES"), "{kept:?}");
    assert!(kept.iter().any(|row| row == "   #2"), "{kept:?}");
    assert!(
        kept.iter()
            .any(|row| row.starts_with("     frightening") && row.ends_with("-1.00")),
        "{kept:?}"
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(
        screen
            .iter()
            .any(|row| row.contains("Sprite #1 learned: Sprite #2 is frightening")),
        "{screen:?}"
    );
    let bully = world.sprites().nth(1).expect("the bully").id();
    app.record(
        &[Event {
            tick: 3,
            kind: EventKind::Renamed {
                id: bully,
                name: "Mira".into(),
            },
        }],
        &world,
    );
    assert!(
        brain_lines(&app, &world).iter().any(|row| row == "   Mira"),
        "{:?}",
        brain_lines(&app, &world)
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
    let on_it = pointer_on(&world, 2, 3);
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
            pointer_on(&world, 2, 3),
            format!(" │ leading: Sprite #{}", sprite.0),
        ),
        (pointer_on(&world, 6, 3), " │ holding: ball".to_string()),
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
    let on_it = pointer_on(&world, 2, 3);
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
    let empty = pointer_on(&world, 1, 1);
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
    common::touched(tick, EntityId(12), what)
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
            "10  You petted Sprite #12 x10",
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
            "4  Couldn't pet Sprite #12: it's gone x2",
            "2  Sprite #12 kicked a ball x2",
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
    let on_it = pointer_on(&world, 7, 1);
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
/// mode, holding the item on `held`, on a 100×30 screen.
fn holding_world(objects: &[(Pos, &str)], held: Pos) -> (World, App) {
    let mut world = WorldBuilder::grass(20, 5).objects(objects).seed(7).build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(
        Action::left_click(pointer_on(&world, held.x, held.y)),
        &world,
    );
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
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
    app.apply(Action::Point(pointer_on(&world, 19, 2)), &world);
    let row = |app: &App| -> String {
        map_rows(&render(app, &world, 100, 30), &world)[2 + 1]
            .chars()
            .skip(1)
            .take(20)
            .collect()
    };
    assert_eq!(row(&app), "......°·•··→○←......");
    let end = &render(&app, &world, 100, 30)[cell(&world, 6, 2)];
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
        let cell = render(app, &world, 100, 30)[cell(&world, 12, 2)].clone();
        (
            cell.symbol().to_string(),
            cell.modifier.contains(Modifier::REVERSED),
        )
    };
    assert_eq!(under_the_cursor(&app), (".".into(), true), "held");
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
    assert_eq!(under_the_cursor(&app), ("○".into(), true), "aimed");
    app.apply(Action::Back, &world);
    assert_eq!(under_the_cursor(&app), (".".into(), true), "cancelled");
}

#[test]
fn while_aiming_the_hints_say_how_to_send_it_or_cancel() {
    let (world, mut app) = holding_world(&[(Pos { x: 12, y: 2 }, "ball")], Pos { x: 12, y: 2 });
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
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
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
    app.apply(Action::Point(pointer_on(&world, 13, 2)), &world);
    let row: String = map_rows(&render(&app, &world, 100, 30), &world)[2 + 1]
        .chars()
        .skip(1)
        .take(20)
        .collect();
    assert_eq!(row, "...........°○←......");
}

#[test]
fn the_event_log_says_when_a_sprite_grows_up_and_grows_old() {
    // M2 design §9.6.
    let world = garden(pack());
    let id = EntityId(12);
    let rows = logged(
        &world,
        &[
            Event {
                tick: 4_500,
                kind: EventKind::GrewUp { id },
            },
            Event {
                tick: 13_500,
                kind: EventKind::GrewOld { id },
            },
        ],
    );
    assert_eq!(
        rows[..2],
        ["13,500  Sprite #12 grew old", "4,500  Sprite #12 grew up"]
    );
}

#[test]
fn the_event_log_says_who_laid_an_egg_hatched_and_turned_female() {
    // M2 design §10.
    let world = garden(pack());
    let (mother, father) = (EntityId(12), EntityId(13));
    let (egg, other_egg) = (EntityId(20), EntityId(21));
    let first = on_ticks(vec![
        EventKind::TurnedFemale { id: mother },
        EventKind::Laid {
            egg,
            by: mother,
            with: Some(father),
        },
        EventKind::Laid {
            egg: other_egg,
            by: mother,
            with: None,
        },
    ]);
    assert_eq!(
        logged(&world, &first),
        [
            "3  Sprite #12 laid an egg alone",
            "2  Sprite #12 laid an egg (with Sprite #13)",
            "1  Sprite #12 turned female",
        ]
    );
    let then = on_ticks(vec![
        EventKind::Hatched {
            id: egg,
            parents: vec![mother, father],
        },
        EventKind::Hatched {
            id: other_egg,
            parents: vec![mother],
        },
        // An egg is never thrown (§10.4): 103 is its type.
        EventKind::CommandRejected {
            command: Command::Throw {
                from: Pos { x: 1, y: 1 },
                toward: terra_sim::Dir::E,
                tiles: 3,
            },
            reason: Rejection::NotThrowable { item_type: 103 },
        },
    ]);
    assert_eq!(
        logged(&world, &then),
        [
            "3  Couldn't throw the egg: it can only be put down",
            "2  Sprite #21 hatched (Sprite #12's)",
            "1  Sprite #20 hatched (Sprite #12 and Sprite #13's)",
        ]
    );
}

/// Where the egg `egg` lies.
fn egg_tile(world: &World, egg: EntityId) -> Pos {
    world
        .egg(egg)
        .and_then(|egg| egg.pos())
        .expect("the egg on the map")
}

/// Runs `world` until the egg `egg` has hatched (M2 design §10.4).
fn hatch(world: &mut World, egg: EntityId) {
    while world.sprite(egg).is_none() {
        assert!(world.tick() < 2_000, "the egg hatches");
        world.step();
    }
}

#[test]
fn an_egg_is_drawn_as_its_theme_says_and_selected_its_card_says_when_it_hatches() {
    // M2 design §10.7: `o` in CP437, `0` in ASCII, where `o` is the ball;
    // selected, in reverse video. In a corner, clear of the Cursor in the
    // middle.
    let (mut world, egg) = a_lone_egg(&[]);
    let at = egg_tile(&world, egg);
    for (theme, glyph) in [(Theme::cp437(), 'o'), (Theme::ascii(), '0')] {
        let mut app = app_for(&world, theme, 100, 30);
        let screen = render(&app, &world, 100, 30);
        let drawn = &screen[cell(&world, at.x, at.y)];
        assert_eq!(drawn.symbol(), glyph.to_string());
        assert!(!drawn.modifier.contains(Modifier::REVERSED));
        // A click selects it; then the Cursor moves away.
        app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
        assert_eq!(app.selection(), Some(Selection::Living(egg)));
        app.apply(Action::Point(pointer_on(&world, 8, 0)), &world);
        let screen = render(&app, &world, 100, 30);
        let drawn = &screen[cell(&world, at.x, at.y)];
        assert_eq!(drawn.symbol(), glyph.to_string());
        assert!(drawn.modifier.contains(Modifier::REVERSED));
        // Every sprite tab shows the card, just after it's laid.
        for tab in [Tab::Body, Tab::Brain, Tab::Chem, Tab::Genome] {
            open(&mut app, &world, tab);
            let text = lines(&render(&app, &world, 100, 30)).join("\n");
            assert!(text.contains("egg · hatches in 750 ticks"), "{text}");
            assert!(text.contains("parents Sprite #1"), "{text}");
        }
    }
    world.step();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    let text = lines(&render(&app, &world, 100, 30)).join("\n");
    assert!(text.contains("egg · hatches in 749 ticks"), "{text}");
}

#[test]
fn an_egg_held_past_its_time_waits_and_its_hatchling_stays_selected() {
    // M2 design §10.4, §10.7: held when it's due, it hatches on the tick
    // it's put down, and the selection goes on to the hatchling.
    let (mut world, egg) = a_lone_egg(&[]);
    let at = egg_tile(&world, egg);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let frame = |app: &mut App, world: &mut World| {
        for command in app.take_commands() {
            world.submit(command);
        }
        let events = world.step();
        app.record(&events, world);
    };
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    frame(&mut app, &mut world);
    let held = world.cursor().holds().map(|held| held.id());
    assert_eq!(held, Some(egg), "picked up");
    let due = world.egg(egg).expect("the egg").hatches();
    while world.tick() <= due {
        frame(&mut app, &mut world);
    }
    assert!(world.egg(egg).is_some(), "it waits");
    let text = lines(&render(&app, &world, 100, 30)).join("\n");
    assert!(text.contains("egg · hatches once it's put down"), "{text}");
    // Put down where no sprite stands, it hatches at once.
    let free = (0..10)
        .map(|x| Pos { x, y: 0 })
        .find(|&pos| world.sprite_at(pos).is_none())
        .expect("a free tile");
    app.apply(
        Action::left_click(pointer_on(&world, free.x, free.y)),
        &world,
    );
    frame(&mut app, &mut world);
    let hatchling = world.sprite(egg).expect("it hatched");
    assert_eq!(hatchling.pos(), free);
    assert_eq!(app.selection(), Some(Selection::Living(egg)));
    let text = lines(&render(&app, &world, 100, 30)).join("\n");
    assert!(text.contains("young ·"), "{text}");
}

#[test]
fn g_saves_a_selected_egg_s_genome_but_f_won_t_follow_it_and_tab_passes_it_by() {
    // M2 design §10.7: an egg is clicked on to select it.
    let (world, egg) = a_lone_egg(&[]);
    let at = egg_tile(&world, egg);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    assert_eq!(app.selection(), Some(Selection::Living(egg)));
    // `F`, with the Cursor on the egg, where no sprite stands.
    app.apply(Action::Follow { at: None }, &world);
    assert_eq!(app.followed(), None);
    assert_eq!(app.refusal(), Some("No sprite here to follow"));
    // `G` saves the genome it hatches with.
    let folder = std::env::temp_dir().join(format!("terra-ui-egg-genome-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    app.set_genome_folder(folder.clone());
    app.apply(Action::ExportGenome, &world);
    let file = folder.join(format!("sprite-{}-tick-{}.ron", egg.0, world.tick()));
    let saved = std::fs::read_to_string(&file).expect("the genome saved");
    let genome = world.egg(egg).expect("the egg").genome();
    assert_eq!(saved, genome.to_ron(world.data()));
    // `Tab` and `Shift+Tab` go from sprite to sprite: here, to the mother
    // alone.
    for action in [
        Action::SelectNext,
        Action::SelectNext,
        Action::SelectPrevious,
    ] {
        app.apply(action, &world);
        let mother = Selection::Living(EntityId(1));
        assert_eq!(app.selection(), Some(mother), "{action:?}");
    }
}

#[test]
fn the_body_tab_says_how_near_a_male_is_to_turning_female_once_grown() {
    // M2 design §10.7: `turning 50%` with half the feminiser it takes;
    // never in a young one (§10.1).
    let pack = pack();
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "feminiser", value: 0.4),
    ])"#;
    let genome = terra_sim::Genome::from_ron(genome, &pack).expect("a valid genome");
    let (grown, young) = (Pos { x: 2, y: 3 }, Pos { x: 6, y: 1 });
    let world = WorldBuilder::grass(10, 5)
        .sprites_with(vec![(grown, Some(genome.clone())), (young, Some(genome))])
        .newborns(&[young])
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        inspector_line(&app, &world, "male"),
        "male · fertile · turning 50%"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(inspector_line(&app, &world, "male"), "male", "young");
}

#[test]
fn paused_an_item_picked_up_leaves_the_map_and_one_put_down_shows_where_it_lands() {
    // Design §6.5, #203: a click waits for the next tick, and the map, as
    // the Cursor's marks do, shows what it will do.
    let (mut world, egg) = a_lone_egg(&[]);
    let (at, to) = paused_egg_moves(&world);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    let screen = render(&app, &world, 100, 30);
    assert_ne!(screen[cell(&world, at.x, at.y)].symbol(), "o", "picked up");
    let status = status_line(&app, &world);
    assert!(!before_hints(&status).contains("· egg"), "{status}");
    app.apply(Action::left_click(pointer_on(&world, to.x, to.y)), &world);
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[cell(&world, to.x, to.y)].symbol(), "o", "put down");
    assert_ne!(screen[cell(&world, at.x, at.y)].symbol(), "o", "moved");
    let status = status_line(&app, &world);
    assert!(before_hints(&status).contains("· egg (laid)"), "{status}");
    // When time moves, the world does what the map showed.
    for command in app.take_commands() {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, &world);
    assert_eq!(world.egg(egg).and_then(|egg| egg.pos()), Some(to));
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[cell(&world, to.x, to.y)].symbol(), "o");
}

/// Where a lone egg lies, and a free tile to put it down on.
fn paused_egg_moves(world: &World) -> (Pos, Pos) {
    let egg = world.eggs().next().expect("an egg").id();
    let at = egg_tile(world, egg);
    let to = (0..10)
        .map(|x| Pos { x, y: 0 })
        .find(|&pos| pos != at && world.sprite_at(pos).is_none() && world.object_at(pos).is_none())
        .expect("a free tile");
    (at, to)
}

#[test]
fn paused_a_click_acts_on_an_item_where_the_map_shows_it() {
    // Design §6.5, #203: with a pick-up and a put-down waiting, the egg is
    // where it was put down, for Grab and Select alike.
    let (world, egg) = a_lone_egg(&[]);
    let (at, to) = paused_egg_moves(&world);
    let moved = || {
        let mut app = app_for(&world, Theme::cp437(), 100, 30);
        app.apply(Action::Mode(CursorMode::Grab), &world);
        app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
        app.apply(Action::left_click(pointer_on(&world, to.x, to.y)), &world);
        app
    };
    let mut app = moved();
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    assert_eq!(
        app.refusal(),
        Some("Nothing here to grab"),
        "it's gone from here"
    );
    app.apply(Action::left_click(pointer_on(&world, to.x, to.y)), &world);
    assert_eq!(app.grip(&world), Some(Grip::Holds(egg)), "picked up again");
    let mut app = moved();
    app.apply(Action::Mode(CursorMode::Select), &world);
    app.apply(Action::left_click(pointer_on(&world, to.x, to.y)), &world);
    assert_eq!(app.selection(), Some(Selection::Living(egg)));
}

#[test]
fn paused_a_thrown_item_shows_where_it_s_thrown_from() {
    // Design §2.5, §6.5: a throw puts the item down where it's thrown from,
    // and it rolls on from there when time moves.
    let (world, mut app) = holding_world(&[(Pos { x: 12, y: 2 }, "ball")], Pos { x: 12, y: 2 });
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
    app.apply(Action::Point(pointer_on(&world, 15, 2)), &world);
    app.apply(
        Action::Release {
            button: Button::Right,
            at: Some(pointer_on(&world, 15, 2)),
        },
        &world,
    );
    assert!(app.grip(&world).is_none(), "the throw is queued");
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[cell(&world, 12, 2)].symbol(), "○");
}

#[test]
fn paused_the_leash_runs_over_ground_as_the_map_shows_it() {
    // Design §6.5, #203: the berry picked up from (3, 2) and put down on
    // (4, 2), both waiting, then the sprite on (1, 2) taken hold of. The
    // leash's dots run over (3, 2) and stop at the berry on (4, 2).
    let world = WorldBuilder::grass(12, 5)
        .objects(&[(Pos { x: 3, y: 2 }, "berry")])
        .starters(&[Pos { x: 1, y: 2 }])
        .scripts(&[(Pos { x: 1, y: 2 }, ScriptedAction::Rest)])
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    for x in [3, 4, 1] {
        app.apply(Action::left_click(pointer_on(&world, x, 2)), &world);
    }
    app.apply(Action::Point(pointer_on(&world, 6, 2)), &world);
    let row: String = map_rows(&render(&app, &world, 100, 30), &world)[1 + 2]
        .chars()
        .skip(1)
        .take(12)
        .collect();
    assert_eq!(row, ".☺··•→.←....");
}

#[test]
fn leading_a_young_sprite_the_cursor_s_mark_shows_it_small() {
    // M2 design §9.6: the bottom left mark shows what the Cursor has hold
    // of, as the map draws it. Away from the map's edges, so the marks are
    // drawn.
    let at = Pos { x: 4, y: 2 };
    let world = WorldBuilder::grass(10, 5)
        .starters(&[at])
        .newborns(&[at])
        .seed(7)
        .build();
    for (theme, glyph) in [(Theme::cp437(), "ö"), (Theme::ascii(), "a")] {
        let mut app = app_for(&world, theme, 100, 30);
        let on = pointer_on(&world, at.x, at.y);
        app.apply(Action::Mode(CursorMode::Grab), &world);
        app.apply(Action::Point(on), &world);
        app.apply(Action::left_click(on), &world);
        let screen = render(&app, &world, 100, 30);
        // The Cursor's centre is up and left of the pointer.
        assert_eq!(screen[(on.x - 2, on.y)].symbol(), glyph);
    }
}

/// A world whose one sprite, by `physiology`'s rules, the built-in one's
/// changed as `changes` say, has laid an egg alone, and the egg. She may
/// breed alone at once (M2 design §15.7).
fn a_lone_egg(changes: &[(&str, &str)]) -> (World, EntityId) {
    let pack = pack_changing(&[LONE, ("physiology.ron", changes)]);
    let genome = terra_sim::Genome::from_ron(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: 7.0),
            Trait(trait: "sense_radius", value: 9.0),
            Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "hunger"),
            Emitter(locus: Chem("hydration"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "thirst"),
            InitialConcentration(chem: "feminiser", value: 1.0),
            InitialConcentration(chem: "longing", value: 1.0),
            Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
        ])"#,
        &pack,
    )
    .expect("a valid genome");
    let sprites = vec![(Pos { x: 1, y: 4 }, Some(genome))];
    let mut world = WorldBuilder::grass(10, 5)
        .sprites_with(sprites)
        .data(pack)
        .seed(7)
        .build();
    let egg = (0..50)
        .flat_map(|_| world.step())
        .find_map(|event| match event.kind {
            EventKind::Laid { egg, .. } => Some(egg),
            _ => None,
        })
        .expect("she laid an egg");
    (world, egg)
}

/// A keen female and a keen male, his genome with `his` genes too, side by
/// side by a thornbush, by `pack`'s rules, once they have had their first
/// egg and become partners: the world, her and him.
fn a_pair(pack: DataPack, his: &str) -> (World, EntityId, EntityId) {
    let keen = |extra: &str| {
        let text = format!(
            r#"(format: 1, genes: [
                Trait(trait: "speed", value: 10.0),
                Trait(trait: "sense_radius", value: 10.0),
                AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
                Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
                InitialConcentration(chem: "receptivity", value: 1.0),
                {extra}
            ])"#
        );
        terra_sim::Genome::from_ron(&text, &pack).expect("a valid genome")
    };
    let hers = r#"InitialConcentration(chem: "feminiser", value: 1.0),"#;
    let sprites = vec![
        (Pos { x: 1, y: 0 }, Some(keen(hers))),
        (Pos { x: 2, y: 0 }, Some(keen(his))),
    ];
    let mut world = WorldBuilder::grass(6, 2)
        .objects(&[(Pos { x: 4, y: 1 }, "thornbush")])
        .sprites_with(sprites)
        .data(pack)
        .seed(7)
        .build();
    let paired = (0..100)
        .flat_map(|_| world.step())
        .find_map(|event| match event.kind {
            EventKind::Paired { a, b } => Some((a, b)),
            _ => None,
        });
    let (she, he) = paired.expect("they had an egg");
    (world, she, he)
}

/// The screen, as text, with `id` clicked on and the Body tab open.
fn body_of(world: &World, id: EntityId) -> String {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    let tile = world.sprite(id).expect("alive").pos();
    app.apply(Action::left_click(pointer_on(world, tile.x, tile.y)), world);
    open(&mut app, world, Tab::Body);
    lines(&render(&app, world, 100, 30)).join("\n")
}

#[test]
fn the_body_tab_names_a_sprite_s_partner_after_their_first_egg() {
    // M2 design §15.8: `partner Sprite #2`, after the parents' line.
    let (world, she, he) = a_pair(pack(), "");
    let text = body_of(&world, she);
    assert!(
        text.contains(&format!("partner Sprite #{}", he.0)),
        "{text}"
    );
    assert!(!text.contains("· died"), "{text}");
}

#[test]
fn the_body_tab_says_when_a_sprite_s_partner_has_died() {
    // M2 design §15.8: `partner Kit #20 · died`, until she pairs again.
    // Once paired, he hits the thornbush, whose thorns kill at once.
    let his = r#"AttentionInstinct(input: "paired", category: "bush", weight: 2.0),
        Instinct(inputs: [("paired", false), ("attended_bush", false)], verb: Hit, weight: 2.0),"#;
    let (mut world, she, he) = a_pair(pack_changing(&[LETHAL_THORNS]), his);
    while world.sprite(he).is_some() {
        assert!(world.tick() < 200, "he died");
        world.step();
    }
    let text = body_of(&world, she);
    assert!(
        text.contains(&format!("partner Sprite #{} · died", he.0)),
        "{text}"
    );
}

/// The built-in physiology's copy and change, as its file begins it.
const COPY_AND_CHANGE: &str =
    "copy_and_change: (doubled: 0.00029, lost: 0.00029, changed: 0.00029,";

#[test]
fn the_genome_tab_of_a_sprite_with_parents_says_first_what_s_new_in_it() {
    // M2 design §12: a lone egg, every gene value copied wrongly and each
    // copy of a gene flawed half the time. Its mother, whom the world
    // began with, has no such list.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.5,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.0, lost: 0.0, changed: 0.0,",
        ),
    ]);
    hatch(&mut world, egg);
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Genome);
    let text = |app: &App| inspector_on(app, &world, 40).1;
    assert_eq!(text(&app)[0], "TRAITS", "the mother has no list");
    select(&mut app, &world, egg);
    let rows = text(&app);
    assert_eq!(rows[0], "FROM SPRITE #1 ALONE");
    let view = world.sprite(egg).expect("the hatchling");
    let changed = view.copying_errors().len();
    assert!(changed >= 3, "{changed} genes changed");
    let list = rows
        .iter()
        .position(|row| row.is_empty())
        .expect("a blank line");
    assert_eq!(rows[list + 1], "TRAITS");
    let listed = rows[1..list].join(" ");
    assert_eq!(listed.matches(", was ").count(), changed, "{listed}");
    let flaws = view.flaws();
    // A lone egg's copies are its mother's doubled, but copying flaws each
    // on its own, so it has broken genes and hidden flaws.
    let broken = flaws.iter().filter(|&&flawed| flawed == 2).count();
    let carried = flaws.iter().filter(|&&flawed| flawed == 1).count();
    assert!(broken > 0 && carried > 0, "{flaws:?}");
    let after = |label: &str| {
        listed
            .split(label)
            .nth(1)
            .map(|rest| rest.split(':').next().unwrap_or(rest).to_string())
            .unwrap_or_default()
    };
    assert_eq!(
        after("broken: ").matches('·').count() + 1,
        broken,
        "{listed}"
    );
    assert_eq!(
        after("carries flaws: ").matches('·').count() + 1,
        carried,
        "{listed}"
    );
    for name in ["speed", "low energy → hunger", "always → breed_alone"] {
        assert!(listed.contains(name), "{name:?} in {listed:?}");
    }
    // Each broken gene shows its health where it sits, as one old age
    // weakened does.
    let genes = rows[list + 1..].join(" ");
    assert_eq!(genes.matches("health .5").count(), broken, "{genes}");
}

#[test]
fn the_genome_tab_lists_the_genes_copying_changed_doubled_and_lost() {
    // M2 design §13: each changed gene with what it was, or what it's a
    // copy of; then the genes doubled, and those lost.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.3, lost: 0.3, changed: 0.3,",
        ),
    ]);
    hatch(&mut world, egg);
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    select(&mut app, &world, egg);
    open(&mut app, &world, Tab::Genome);
    let (_, rows) = inspector_on(&app, &world, 40);
    let list = rows
        .iter()
        .position(|row| row.is_empty())
        .expect("a blank line");
    let listed = rows[1..list].join(" ");
    let changes = world.sprite(egg).expect("the hatchling").copy_changes();
    let count = |kind: fn(&CopyChangeView) -> bool| changes.iter().filter(|c| kind(c)).count();
    let changed = count(|c| matches!(c, CopyChangeView::Changed { .. }));
    let copies = count(|c| matches!(c, CopyChangeView::Changed { on_copy: true, .. }));
    let doubled = count(|c| matches!(c, CopyChangeView::Doubled { .. }));
    let lost = count(|c| matches!(c, CopyChangeView::Lost { .. }));
    assert!(changed > 0 && doubled > 0 && lost > 0, "{changes:?}");
    assert_eq!(listed.matches("new: ").count(), changed, "{listed}");
    assert_eq!(listed.matches(", from ").count(), copies, "{listed}");
    assert_eq!(
        listed.matches(", was ").count(),
        changed - copies,
        "{listed}"
    );
    let after = |label: &str| {
        listed
            .split(label)
            .nth(1)
            .map(|rest| rest.split(':').next().unwrap_or(rest).to_string())
            .unwrap_or_default()
    };
    assert_eq!(
        after("doubled: ").matches('·').count() + 1,
        doubled,
        "{listed}"
    );
    assert_eq!(after("lost: ").matches('·').count() + 1, lost, "{listed}");
}

/// What's new in `id`, a hatchling selected with `Tab`, as the top of its
/// Genome tab lists it (M2 design §12.6), up to the blank row: each line
/// whole, though the tab wraps a long one onto rows indented further.
fn whats_new(world: &World, id: EntityId) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 40);
    select(&mut app, world, id);
    open(&mut app, world, Tab::Genome);
    let mut said: Vec<String> = Vec::new();
    for row in inspector_rows(&app, world, 40).1 {
        if row.is_empty() {
            return said;
        }
        match row.strip_prefix("   ") {
            Some(more) => {
                let line = said.last_mut().expect("a line it goes on");
                line.push(' ');
                line.push_str(more);
            }
            None => said.push(row.trim_start().to_string()),
        }
    }
    panic!("no blank row after {said:#?}");
}

#[test]
fn the_genome_tab_of_a_pair_s_young_says_it_is_from_them_both() {
    // M2 design §12.6: `FROM PIP #12 AND BO #7`, the one that laid it
    // first.
    let (mut world, she, he) = a_pair(pack(), "");
    let egg = world.eggs().next().expect("their egg").id();
    assert_eq!(world.egg(egg).expect("the egg").parents(), [she, he]);
    hatch(&mut world, egg);
    let from = format!("FROM SPRITE #{} AND SPRITE #{}", she.0, he.0);
    assert_eq!(whats_new(&world, egg)[0], from);
}

#[test]
fn the_genome_tab_says_of_a_gene_copying_changed_only_the_numbers_that_differ() {
    // M2 design §12.6: `+.00512 past .515, was +.00428`. Copying changes
    // each gene value seven times in ten, so a gene may change in each of
    // its numbers or in only one of two. Each changed gene's line says the
    // gene as it is, then what it was: the numbers that read differently,
    // or all of them if none does.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 0.7,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.0, lost: 0.0, changed: 0.0,",
        ),
    ]);
    hatch(&mut world, egg);
    let listed = whats_new(&world, egg);
    let view = world.sprite(egg).expect("the hatchling");
    let (genes, errors) = (view.genes(), view.copying_errors());
    // Where it's from, then a line for each changed gene, in genome order.
    assert_eq!(listed.len(), errors.len() + 1, "{listed:#?}");
    let mut some_alike = false;
    for ((index, was), line) in errors.iter().zip(&listed[1..]) {
        let (now_said, was_said) = line
            .split_once(", was ")
            .unwrap_or_else(|| panic!("{line:?}"));
        let (now, was) = (numbers_of(&genes[*index].0), numbers_of(was));
        let all = |values: &[f32]| values.iter().map(|&v| reads(v)).collect::<Vec<_>>();
        assert_eq!(numbers_in(now_said), all(&now), "{line:?}");
        let differ: Vec<String> = was
            .iter()
            .zip(&now)
            .filter(|&(&was, &now)| reads(was) != reads(now))
            .map(|(&was, _)| reads(was))
            .collect();
        some_alike |= !differ.is_empty() && differ.len() < was.len();
        let shown = if differ.is_empty() { all(&was) } else { differ };
        assert_eq!(numbers_in(was_said), shown, "{line:?}");
    }
    assert!(
        some_alike,
        "a gene with a number copying left alone: {listed:#?}"
    );
}

/// A gene's numbers in the order the Genome tab writes them, for the kinds
/// of gene `a_lone_egg`'s mother has: an emitter's gain, then its
/// threshold unless it's 0.
fn numbers_of(gene: &GeneView) -> Vec<f32> {
    match *gene {
        GeneView::Trait { value, .. } | GeneView::InitialConcentration { value, .. } => {
            vec![value]
        }
        GeneView::Emitter {
            gain,
            threshold: 0.0,
            ..
        } => vec![gain],
        GeneView::Emitter {
            gain, threshold, ..
        } => vec![gain, threshold],
        GeneView::Instinct { weight, .. } => vec![weight],
        ref other => panic!("no such gene in the mother: {other:?}"),
    }
}

/// How a gene's value reads to 3 significant figures, as the Genome tab
/// writes it: `.00428` and `4.28e-3` read alike.
fn reads(value: f32) -> String {
    format!("{value:.2e}")
}

/// The numbers written in `said`, as they read.
fn numbers_in(said: &str) -> Vec<String> {
    said.split(char::is_whitespace)
        .filter_map(|word| word.replace(',', "").parse::<f32>().ok())
        .map(reads)
        .collect()
}

#[test]
fn the_genome_tab_says_what_s_new_in_its_order() {
    // M2 design §12.6, §13.2: where its genes came from; each gene
    // changed; the genes doubled; those lost; each gene copying changed a
    // number of; the genes broken; those that carry a flaw. Everything
    // copying can do, it does often here.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.5,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.3, lost: 0.3, changed: 0.3,",
        ),
    ]);
    hatch(&mut world, egg);
    let listed = whats_new(&world, egg);
    // Each kind of line, in the order the tab lists them; a gene copying
    // changed a number of is named first, as the tab writes the gene.
    let starts = [
        "FROM ",
        "new: ",
        "doubled: ",
        "lost: ",
        "",
        "broken: ",
        "carries flaws: ",
    ];
    let kinds: Vec<usize> = listed
        .iter()
        .map(|line| {
            let named = starts
                .iter()
                .position(|&start| !start.is_empty() && line.starts_with(start));
            named.unwrap_or(4)
        })
        .collect();
    assert!(
        (0..starts.len()).all(|kind| kinds.contains(&kind)),
        "{kinds:?}: {listed:#?}"
    );
    assert!(kinds.is_sorted(), "{kinds:?}: {listed:#?}");
}

#[test]
fn a_broken_gene_counts_in_the_body_tab_s_genes_but_not_its_wear() {
    // M2 design §12.6: the Body tab's `genes .97` counts a broken gene,
    // whose health is .5; §12.4: it isn't wear. Every copy of every gene
    // that can weaken is flawed, so each is broken.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 1.0,"),
    ]);
    hatch(&mut world, egg);
    let view = world.sprite(egg).expect("the hatchling");
    assert!(view.flaws().contains(&2), "{:?}", view.flaws());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    select(&mut app, &world, egg);
    open(&mut app, &world, Tab::Body);
    assert_eq!(inspector_line(&app, &world, "wear"), "wear .00 · genes .50");
}

#[test]
fn the_event_log_says_what_s_new_in_a_hatchling_with_a_gene_changed() {
    // M2 design §13.2: the genes copying changed in what they do, not
    // those doubled or lost: nothing changed, every gene doubled or lost,
    // and every gene changed.
    for (doubled_lost, changed, new) in [
        ("0.0", "0.0", false),
        ("0.5", "0.0", false),
        ("0.0", "1.0", true),
    ] {
        let rules = format!(
            "copy_and_change: (doubled: {doubled_lost}, lost: {doubled_lost}, changed: {changed},"
        );
        let (mut world, egg) = a_lone_egg(&[
            ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
            ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
            (COPY_AND_CHANGE, rules.as_str()),
        ]);
        let mut app = app_for(&world, Theme::cp437(), 100, 30);
        loop {
            let events = world.step();
            app.record(&events, &world);
            let hatched =
                |event: &Event| matches!(event.kind, EventKind::Hatched { id, .. } if id == egg);
            if events.iter().any(hatched) {
                break;
            }
            assert!(world.tick() < 2_000, "it hatched");
        }
        let screen = lines(&render(&app, &world, 100, 30));
        let line = screen[25..28]
            .iter()
            .map(|row| inside(row).to_string())
            .find(|row| row.contains("hatched"))
            .expect("its hatching in the log");
        let hatched = format!("Sprite #{} hatched (Sprite #1's)", egg.0);
        if new {
            let changes = world.sprite(egg).expect("the hatchling").copy_changes();
            assert!(!changes.is_empty());
            let said = format!("{hatched}, with something new: ");
            let (_, rest) = line.split_once(&said).expect(&line);
            let named = ["low energy", "low hydration", "always"];
            assert!(named.iter().any(|name| rest.starts_with(name)), "{line}");
        } else {
            let changes = world.sprite(egg).expect("the hatchling").copy_changes();
            let doubled_or_lost = |change: &CopyChangeView| {
                matches!(
                    change,
                    CopyChangeView::Doubled { .. } | CopyChangeView::Lost { .. }
                )
            };
            assert_eq!(changes.is_empty(), doubled_lost == "0.0", "{changes:?}");
            assert!(changes.iter().all(doubled_or_lost), "{changes:?}");
            assert!(line.ends_with(&hatched), "{line}");
        }
    }
}

#[test]
fn the_map_view_alone_fills_the_frame_with_tiles_and_nothing_else() {
    // A 40×16 field of grass with rock at (5, 2) and water at (34, 13): a
    // 30×12 view centred on the map has them in its corners, with no border,
    // top bar, panels or status line round it (#241).
    let mut rows = vec![".".repeat(40); 16];
    rows[2].replace_range(5..6, "#");
    rows[13].replace_range(34..35, "~");
    let drawing: Vec<&str> = rows.iter().map(String::as_str).collect();
    let world = drawn_world(&drawing);
    let areas = ui::map_view_alone_areas(Size::new(30, 12));
    assert_eq!(areas.tiles, Rect::new(0, 0, 30, 12));
    let app = App::new(world.map(), Theme::ascii(), 7, areas);
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    terminal
        .draw(|frame| ui::render_map_view_alone(frame, &app, &world))
        .unwrap();
    let shown = lines(terminal.backend().buffer());
    assert_eq!(shown[0], format!("#{}", ".".repeat(29)));
    assert_eq!(shown[11], format!("{}~", ".".repeat(29)));
}

/// One 100×30 frame of `world`, drawn with only the screen's `parts`, and
/// its app fitted to them.
fn render_parts(world: &World, parts: ui::Parts) -> Vec<String> {
    let screen = Size::new(100, 30);
    let app = App::new(
        world.map(),
        Theme::ascii(),
        7,
        ui::areas_with(screen, parts),
    );
    let mut terminal = Terminal::new(TestBackend::new(screen.width, screen.height)).unwrap();
    terminal
        .draw(|frame| ui::render_with(frame, &app, world, parts))
        .unwrap();
    lines(terminal.backend().buffer())
}

#[test]
fn a_picture_without_the_event_log_gives_its_rows_to_the_map_view_and_inspector() {
    // #291: the renderer's clips can leave the event log out.
    let parts = ui::Parts {
        event_log: false,
        ..ui::Parts::ALL
    };
    let areas = ui::areas_with(Size::new(100, 30), parts);
    assert_eq!(areas.event_log, None);
    assert_eq!(areas.tiles, Rect::new(1, 2, 52, 26));
    let inspector = areas.inspector.expect("the inspector");
    assert_eq!((inspector.y, inspector.bottom()), (1, 29));
    let shown = render_parts(&generated_world(), parts);
    assert!(
        shown.iter().all(|line| !line.contains(" Events ")),
        "{shown:#?}"
    );
    // The map view's bottom border sits on the status line.
    assert!(shown[28].starts_with('└'), "{}", shown[28]);
    assert!(shown[28].ends_with('┘'), "{}", shown[28]);
}

#[test]
fn a_picture_without_the_inspector_gives_its_columns_to_the_map_view() {
    // #291.
    let parts = ui::Parts {
        inspector: false,
        ..ui::Parts::ALL
    };
    let areas = ui::areas_with(Size::new(100, 30), parts);
    assert_eq!(areas.inspector, None);
    assert_eq!(areas.tiles, Rect::new(1, 2, 98, 21));
    let shown = render_parts(&generated_world(), parts);
    assert!(shown[1].starts_with("┌─ Map ─"), "{}", shown[1]);
    assert!(shown[1].ends_with('┐'), "{}", shown[1]);
    assert_eq!(shown[1].chars().count(), 100, "{}", shown[1]);
}

#[test]
fn a_picture_without_the_top_bar_and_status_line_starts_and_ends_on_panels() {
    // #291: the map view and the inspector start on the top row, and the
    // event log ends on the bottom one.
    let parts = ui::Parts {
        top_bar: false,
        status_line: false,
        ..ui::Parts::ALL
    };
    let areas = ui::areas_with(Size::new(100, 30), parts);
    assert_eq!(areas.tiles, Rect::new(1, 1, 52, 23));
    assert_eq!(areas.event_log, Some(Rect::new(0, 25, 100, 5)));
    assert_eq!(areas.overlay, Some(Rect::new(0, 0, 100, 30)));
    let shown = render_parts(&generated_world(), parts);
    assert!(shown[0].starts_with("┌─ Map ─"), "{}", shown[0]);
    assert!(shown[25].starts_with("┌─ Events ─"), "{}", shown[25]);
    assert!(shown[29].starts_with('└'), "{}", shown[29]);
}

#[test]
fn a_picture_without_any_other_part_is_the_map_view_in_its_border() {
    // #291: the map view's border runs round the whole picture.
    let parts = ui::Parts {
        top_bar: false,
        inspector: false,
        event_log: false,
        status_line: false,
    };
    let areas = ui::areas_with(Size::new(100, 30), parts);
    assert_eq!(areas.tiles, Rect::new(1, 1, 98, 28));
    assert_eq!((areas.inspector, areas.event_log), (None, None));
    let shown = render_parts(&generated_world(), parts);
    assert!(shown[0].starts_with("┌─ Map ─"), "{}", shown[0]);
    assert!(shown[0].ends_with('┐'), "{}", shown[0]);
    assert!(shown[29].starts_with('└'), "{}", shown[29]);
    assert!(shown[29].ends_with('┘'), "{}", shown[29]);
}

#[test]
fn the_game_draws_every_part_of_the_screen() {
    // #291: leaving parts out is for pictures; the game's own screen has
    // the top bar, the map view and the inspector side by side, the event
    // log and the status line.
    let world = generated_world();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let shown = lines(&render(&app, &world, 100, 30));
    assert!(shown[0].starts_with(" Terra Sprites"), "{}", shown[0]);
    assert!(shown[1].starts_with("┌─ Map ─"), "{}", shown[1]);
    assert_eq!(shown[1].matches('┌').count(), 2, "{}", shown[1]);
    assert!(shown[24].starts_with("┌─ Events ─"), "{}", shown[24]);
    assert!(!shown[29].starts_with('└'), "{}", shown[29]);
}
