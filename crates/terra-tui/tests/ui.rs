//! What the screen draws (design §6.1): the top bar, the map view and
//! what's on it, the Cursor and its marks, the status line, the screen
//! sizes the game draws on, and the parts a picture can leave out. The
//! inspector's tabs are tested in `inspector_tabs.rs`, and the event log in
//! `event_log.rs`.

mod common;
use common::screen::{
    HUNGRY_GENOME, SPEED_10, a_lone_egg, app_for, garden, garden_with_sprites, generated_world,
    inspector, one_sprite_among, one_sprite_doing, pointer_on, render, touched,
};
use common::{
    HURT_ITSELF, HURT_TARGET, UNHURT, WorldBuilder, acted, at, at_sprite, lines, open, pack,
    pack_changing, render_on, run, sized_app, status_line, tick,
};

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    Command, EntityId, Event, EventKind, Grip, Map, Outcome, Pos, ScriptedAction, Target, Verb,
    World,
};
use terra_tui::app::{App, CursorMode, Flow, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;
use terra_tui::ui;

/// A world on a drawn map, using the ascii legend.
fn drawn_world(rows: &[&str]) -> World {
    let map = Map::from_ascii(rows, &pack()).expect("valid drawing");
    World::from_map(map, pack(), 7).expect("one region")
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
    app.cell_of(at(x, y)).expect("in view")
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
    run(&mut world, 1_234);
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
    // M2 design §11.1: a small map no longer shrinks it. It has no frame,
    // so it runs to the screen's left edge (design §6.1).
    assert_eq!(ui::areas(Size::new(100, 30)).tiles, Rect::new(0, 1, 54, 24));
}

#[test]
fn a_small_map_is_drawn_in_the_middle_of_the_map_view_with_space_round_it() {
    // M2 design §11.1: the 10×4 map, in the 54×24 tiles of a 100×30 screen's
    // map view, has 22 columns of space on either side, its glass included,
    // and 10 rows above and below.
    let world = drawn_world(&SMALL_MAP);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    assert_eq!(app.cell_of(at(0, 0)), Some(Position::new(22, 11)));
    assert_eq!(app.cell_of(at(9, 3)), Some(Position::new(31, 14)));
    let screen = render(&app, &world, 100, 30);
    let corners = [(21, 10, "╔"), (32, 10, "╗"), (21, 15, "╚"), (32, 15, "╝")];
    for (x, y, pane) in corners {
        assert_eq!(screen[(x, y)].symbol(), pane, "({x}, {y})");
    }
    for (x, y) in [(20, 11), (33, 14), (22, 9), (31, 16), (0, 1), (53, 24)] {
        let star = screen[(x, y)].symbol();
        assert!(matches!(star, " " | "·" | "∙" | "*"), "space at ({x}, {y})");
    }
    // It doesn't scroll, in any direction.
    for (dx, dy) in [(-5, 0), (5, 0), (0, -5), (0, 5)] {
        app.apply(Action::Scroll { dx, dy }, &world);
        assert_eq!(app.cell_of(at(0, 0)), Some(Position::new(22, 11)));
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
        .objects(&[(at(4, 2), "berry")])
        .starters(&[at(1, 2)])
        .scripts(&[(at(1, 2), ScriptedAction::Rest)])
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
fn the_map_view_has_no_border_and_the_wall_is_glass() {
    // Design §6.1, M2 design §11.2: the glass shows where the wall is, one
    // tile outside the map, so the map view needs no frame: its tiles run
    // from edge to edge.
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Point(Position::new(30, 10)), &world);
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    for row in [&rows[0], &rows[23]] {
        assert_eq!(row, &".".repeat(54), "the top and bottom rows are tiles");
    }
    assert!(
        rows.iter()
            .all(|row| row.starts_with('.') && row.ends_with('.')),
        "and so are the left and right columns: {rows:#?}"
    );
    // Scrolled to the top-left corner, the view is a quarter of itself past
    // the wall: 13 columns and 6 rows, the glass in the last of each.
    app.apply(Action::Scroll { dx: -200, dy: -200 }, &world);
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    let glass: String = rows[5].chars().skip(12).take(6).collect();
    assert_eq!(glass, "╔═════");
    let side: String = rows[6].chars().skip(12).take(3).collect();
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
        (-200, -200, at(0, 0), "╔", ["═", "║"]),
        (200, -200, at(119, 0), "╗", ["═", "║"]),
        (-200, 200, at(0, 59), "╚", ["═", "║"]),
        (200, 200, at(119, 59), "╝", ["═", "║"]),
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
    app.apply(Action::Point(Position::new(0, 1)), &world); // the view's top-left tile
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    assert!(rows[0].starts_with(".←..."), "{}", rows[0]);
    assert!(rows[1].starts_with("↑♦..."), "{}", rows[1]);
    assert!(rows[2].starts_with("....."), "{}", rows[2]);
}

/// A field of grass bigger than the map view of a 100×30 screen, which
/// shows 54×24 of its tiles from cell (0, 1).
fn big_world() -> World {
    let row = ".".repeat(120);
    drawn_world(&vec![row.as_str(); 60])
}

/// The map view's rows on a 100×30 screen: the left 54 columns of rows 1
/// to 24.
fn map_view_rows(screen: &Buffer) -> Vec<String> {
    (1..25)
        .map(|y| (0..54).map(|x| screen[(x, y)].symbol()).collect())
        .collect()
}

#[test]
fn a_cursor_whose_target_is_out_of_view_is_not_drawn_at_all() {
    let world = big_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    // Put the cursor on the view's rightmost column, with the pointer on the
    // inspector's left column beside it, then move the pointer further
    // into the inspector, off the map view, and scroll left, so the cursor's
    // tile leaves the view.
    app.apply(Action::Point(Position::new(54, 12)), &world);
    assert_eq!(i32::from(app.cursor().x), app.viewport().x + 53);
    app.apply(Action::Point(Position::new(70, 12)), &world);
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    let rows = map_view_rows(&render(&app, &world, 100, 30));
    for row in &rows {
        assert_eq!(
            row,
            &".".repeat(54),
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
    let mut app = app_for(&world, Theme::cp437(), 100, 30); // 54×24 tiles
    // Tiles (79, 42) on: a quarter of the view, 13 and 6, past the wall.
    app.apply(Action::Scroll { dx: 200, dy: 200 }, &world);
    let screen = render(&app, &world, 140, 40); // room for 94×34 tiles
    let row = |y: u16| -> String { (0..42).map(|x| screen[(x, y)].symbol()).collect() };
    for y in 1..19 {
        assert_eq!(row(y), format!("{}║", ".".repeat(41)), "row {y}");
    }
    assert_eq!(row(19), format!("{}╝", "═".repeat(41)));
    for y in 20..35 {
        for x in 42..94 {
            assert!(
                matches!(screen[(x, y)].symbol(), " " | "·" | "∙" | "*"),
                "space at ({x}, {y})"
            );
        }
    }
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
    run(&mut world, 3);
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
    let sprite = world.sprite_at(at(7, 1)).expect("the sprite on the berry");
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
        54,
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

#[test]
fn a_100_by_30_screen_has_every_panel() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(screen[0].starts_with(" Terra Sprites"));
    assert!(screen[1].contains("Body Brain Chem Genome"));
    assert!(screen[25].contains("Events"));
}

#[test]
fn the_decision_marker_flashes_an_x_where_the_selected_sprite_is_heading() {
    let destination = at(7, 3);
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

#[test]
fn the_attention_marker_shades_the_one_thing_the_selected_sprite_attends_to() {
    // Away from the cursor, which starts on the map's centre, (5, 2).
    let (berry, bush) = (at(0, 1), at(8, 4));
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
    let (berry, bush) = (at(0, 1), at(8, 4));
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
fn a_hurt_sprite_takes_turns_with_a_red_bang_for_a_second() {
    for theme in [Theme::cp437(), Theme::ascii()] {
        let world = garden_with_sprites();
        let mut app = app_for(&world, theme, 100, 30);
        let biter = world.sprite_at(at(2, 3)).expect("a sprite");
        let other = world.sprite_at(at(7, 1)).expect("a sprite");
        let (biter, other) = (biter.id(), other.id());
        let biter_cell = app.cell_of(at(2, 3)).expect("in view");
        let other_cell = app.cell_of(at(7, 1)).expect("in view");
        let sprite_glyph = render(&app, &world, 100, 30)[biter_cell]
            .symbol()
            .to_string();
        // A thornbush bite, which hurts the biter, and a harmless kick.
        let thorns = Some((Target::Object(EntityId(77)), 3));
        let ball = Some((Target::Object(EntityId(40)), 4));
        app.record(
            &[
                acted(1, biter, Verb::Eat, thorns, Outcome::Applied, HURT_ITSELF),
                acted(1, other, Verb::Play, ball, Outcome::Applied, UNHURT),
            ],
            &world,
        );
        let look = |app: &App, cell| {
            let screen = render(app, &world, 100, 30);
            let cell = &screen[cell];
            (cell.symbol().to_string(), cell.fg)
        };
        // A quarter of a second the bang, a quarter the sprite, for a second.
        let bang = ("!".to_string(), Color::Red);
        assert_eq!(look(&app, biter_cell), bang);
        assert_eq!(
            look(&app, other_cell).0,
            sprite_glyph,
            "a kick hurts nobody"
        );
        app.animate(Duration::from_millis(250));
        assert_eq!(look(&app, biter_cell).0, sprite_glyph);
        app.animate(Duration::from_millis(250));
        assert_eq!(look(&app, biter_cell), bang);
        app.animate(Duration::from_millis(500));
        assert_eq!(look(&app, biter_cell).0, sprite_glyph, "and then it's over");
        app.animate(Duration::from_millis(250));
        assert_eq!(look(&app, biter_cell).0, sprite_glyph);

        // A hit hurts the sprite hit, not the hitter.
        let hit = acted(
            2,
            biter,
            Verb::Hit,
            at_sprite(other),
            Outcome::Applied,
            HURT_TARGET,
        );
        app.record(&[hit], &world);
        assert_eq!(look(&app, other_cell), bang);
        assert_eq!(look(&app, biter_cell).0, sprite_glyph);
    }
}

#[test]
fn the_status_line_names_train_mode_and_what_the_cursor_follows() {
    // Design v21 §6.1: tiles are drawn from screen cell (1, 2).
    let world = garden_with_sprites();
    let id = world.sprite_at(at(2, 3)).expect("a sprite").id();
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
    let sprite = world.sprite_at(at(2, 3)).expect("a sprite").id();
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

#[test]
fn a_pet_shows_a_heart_and_a_zap_a_yellow_double_bang_for_a_second() {
    // Design v21 §6.3: Pleased and Shocked, taking turns with the sprite.
    for (theme, heart, bang) in [(Theme::cp437(), "♥", "‼"), (Theme::ascii(), "+", "/")] {
        let world = garden_with_sprites();
        let mut app = app_for(&world, theme, 100, 30);
        let (petted, zapped) = (at(2, 3), at(7, 1));
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
        let look = |app: &App, cell| {
            let screen = render(app, &world, 100, 30);
            let cell = &screen[cell];
            (cell.symbol().to_string(), cell.fg)
        };
        assert_eq!(look(&app, petted_cell).0, heart);
        assert_eq!(look(&app, zapped_cell), (bang.to_string(), Color::Yellow));
        app.animate(Duration::from_millis(250));
        assert_eq!(look(&app, petted_cell).0, sprite_glyph);
        app.animate(Duration::from_millis(1000));
        assert_eq!(
            look(&app, petted_cell).0,
            sprite_glyph,
            "and then it's over"
        );
        assert_ne!(look(&app, zapped_cell).0, bang);
    }
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
    tick(&mut app, &mut world);
    assert!(world.cursor().holds().is_some());
    (world, app)
}

#[test]
fn a_steady_aim_line_runs_the_way_the_thing_will_go_as_far_as_it_can_over_empty_ground() {
    // Design v25 §6.5. A ball held on (12, 2) is pulled 7 tiles east: it'll
    // go west, but a ball goes at most 6 tiles. The berry on its way shows,
    // and the ball under the Cursor, where it'll be thrown from.
    let (world, mut app) = holding_world(&[(at(12, 2), "ball"), (at(8, 2), "berry")], at(12, 2));
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
    let (world, mut app) = holding_world(&[(at(12, 2), "ball")], at(12, 2));
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
    let (world, mut app) = holding_world(&[(at(12, 2), "ball")], at(12, 2));
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
    let status = lines(&render(&app, &world, 100, 30))[29].clone();
    assert!(
        status.ends_with("let go to throw  esc cancel"),
        "{status:?}"
    );
}

#[test]
fn a_short_aim_s_end_shows_even_beside_the_cursor() {
    // Design v25 §6.5: a ball pulled one tile east goes one tile west, onto
    // the Cursor's left arm; the end mark shows there, so it isn't lost.
    let (world, mut app) = holding_world(&[(at(12, 2), "ball")], at(12, 2));
    app.apply(Action::right_click(pointer_on(&world, 12, 2)), &world);
    app.apply(Action::Point(pointer_on(&world, 13, 2)), &world);
    let row: String = map_rows(&render(&app, &world, 100, 30), &world)[2 + 1]
        .chars()
        .skip(1)
        .take(20)
        .collect();
    assert_eq!(row, "...........°○←......");
}

/// Where the egg `egg` lies.
fn egg_tile(world: &World, egg: EntityId) -> Pos {
    world
        .egg(egg)
        .and_then(|egg| egg.pos())
        .expect("the egg on the map")
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
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::left_click(pointer_on(&world, at.x, at.y)), &world);
    tick(&mut app, &mut world);
    let held = world.cursor().holds().map(|held| held.id());
    assert_eq!(held, Some(egg), "picked up");
    let due = world.egg(egg).expect("the egg").hatches();
    while world.tick() <= due {
        tick(&mut app, &mut world);
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
    tick(&mut app, &mut world);
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
    tick(&mut app, &mut world);
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
    let (world, mut app) = holding_world(&[(at(12, 2), "ball")], at(12, 2));
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
        .objects(&[(at(3, 2), "berry")])
        .starters(&[at(1, 2)])
        .scripts(&[(at(1, 2), ScriptedAction::Rest)])
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
    let tile = at(4, 2);
    let world = WorldBuilder::grass(10, 5)
        .starters(&[tile])
        .newborns(&[tile])
        .seed(7)
        .build();
    for (theme, glyph) in [(Theme::cp437(), "ö"), (Theme::ascii(), "a")] {
        let mut app = app_for(&world, theme, 100, 30);
        let on = pointer_on(&world, tile.x, tile.y);
        app.apply(Action::Mode(CursorMode::Grab), &world);
        app.apply(Action::Point(on), &world);
        app.apply(Action::left_click(on), &world);
        let screen = render(&app, &world, 100, 30);
        // The Cursor's centre is up and left of the pointer.
        assert_eq!(screen[(on.x - 2, on.y)].symbol(), glyph);
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
    assert_eq!(areas.tiles, Rect::new(0, 1, 54, 28));
    let inspector = areas.inspector.expect("the inspector");
    assert_eq!((inspector.y, inspector.bottom()), (1, 29));
    let shown = render_parts(&generated_world(), parts);
    assert!(
        shown.iter().all(|line| !line.contains(" Events ")),
        "{shown:#?}"
    );
    // The line between the map view and the inspector runs down to the
    // status line.
    assert_eq!(shown[28].chars().nth(54), Some('│'), "{}", shown[28]);
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
    assert_eq!(areas.tiles, Rect::new(0, 1, 100, 24));
    let shown = render_parts(&generated_world(), parts);
    assert!(!shown[1].contains('│'), "{}", shown[1]);
    assert!(shown[25].starts_with("─ Events ─"), "{}", shown[25]);
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
    assert_eq!(areas.tiles, Rect::new(0, 0, 54, 26));
    assert_eq!(areas.event_log, Some(Rect::new(0, 26, 100, 4)));
    assert_eq!(areas.overlay, Some(Rect::new(0, 0, 100, 30)));
    let shown = render_parts(&generated_world(), parts);
    assert_eq!(shown[0].chars().nth(54), Some('│'), "{}", shown[0]);
    assert!(shown[26].starts_with("─ Events ─"), "{}", shown[26]);
}

#[test]
fn a_picture_without_any_other_part_is_the_map_view_alone() {
    // #291: the map view fills the whole picture; it has no border (design
    // §6.1).
    let parts = ui::Parts {
        top_bar: false,
        inspector: false,
        event_log: false,
        status_line: false,
    };
    let areas = ui::areas_with(Size::new(100, 30), parts);
    assert_eq!(areas.tiles, Rect::new(0, 0, 100, 30));
    assert_eq!((areas.inspector, areas.event_log), (None, None));
    let shown = render_parts(&generated_world(), parts);
    let lines = ['┌', '┐', '└', '┘', '│', '─'];
    assert!(shown.iter().all(|row| !row.contains(lines)), "{shown:#?}");
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
    assert!(shown[1].contains(" Body Brain Chem"), "{}", shown[1]);
    assert!(shown[25].starts_with("  Events "), "{}", shown[25]);
    assert!(!shown[29].trim().is_empty(), "{}", shown[29]);
}
