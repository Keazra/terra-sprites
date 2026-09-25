use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    DataPack, DeathCause, EntityId, Event, EventKind, Map, Removal, Scenario, World, WorldConfig,
};
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
    let areas = ui::areas(Size::new(width, height), world.map());
    App::new(world.map(), theme, 7, areas)
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
        ui::areas(Size::new(40, 8), small.map()).tiles,
        Rect::new(1, 2, 10, 4),
        "a small map gets a shrunk map view"
    );
    let row = ".".repeat(30);
    let big = drawn_world(&vec![row.as_str(); 20]);
    assert_eq!(
        ui::areas(Size::new(20, 8), big.map()).tiles,
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
    app.apply(Action::Point(Position::new(1, 2)), &world);
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
    app.apply(Action::Scroll { dx: -100, dy: -100 }, &world);
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
    app.apply(Action::Point(Position::new(1, 2)), &world); // the view's top-left tile
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
    app.apply(Action::Point(Position::new(18, 4)), &world);
    assert_eq!(app.cursor(), terra_sim::Pos { x: 23, y: 10 });
    app.apply(Action::Point(Position::new(0, 4)), &world);
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
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
        app.apply(Action::Point(Position::new(1 + x, 2 + y)), &world);
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
    app.apply(Action::Back, &world);
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
    app.apply(Action::Scroll { dx: 100, dy: 100 }, &world); // tiles (12, 16) to (29, 19): the bottom-right corner
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
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &[],
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
    cp437.apply(Action::Click(Position::new(1 + 7, 2 + 1)), &world);
    cp437.apply(Action::Point(Position::new(1, 2 + 4)), &world); // the cursor out of the way
    let screen = render(&cp437, &world, 40, 9);
    assert_eq!(lines(&screen)[3], "║.'..♠..☻..║", "the selected sprite");
    assert_eq!(screen[(3, 5)].symbol(), "☺", "the other sprite");

    // In ascii, `&` is already the berry bush, so the selected sprite is `@` in reverse video.
    let mut ascii = pointing_at(&world, Theme::ascii(), 40, 9, 7, 1);
    ascii.apply(Action::Click(Position::new(1 + 7, 2 + 1)), &world);
    ascii.apply(Action::Point(Position::new(1, 2 + 4)), &world);
    let screen = render(&ascii, &world, 40, 9);
    assert_eq!(screen[(8, 3)].symbol(), "@");
    assert!(screen[(8, 3)].modifier.contains(Modifier::REVERSED));
    assert!(!screen[(3, 5)].modifier.contains(Modifier::REVERSED));
}

/// An app on `world` with the cursor pointed at tile `(x, y)` of a small map,
/// whose tiles are drawn from screen cell (1, 2).
fn pointing_at(world: &World, theme: Theme, width: u16, height: u16, x: u16, y: u16) -> App {
    let mut app = app_for(world, theme, width, height);
    app.apply(Action::Point(Position::new(1 + x, 2 + y)), world);
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
    assert_eq!(lines(&render(&app, &world, 60, 9))[8], expected);
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
    assert!(bar.ends_with("│ seed 7 │ sprites 2"), "{bar}");
    assert!(
        top_bar(&default_app()).ends_with("│ sprites 30"),
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
        inspector[1], "┌─ Body Chem Genome [World] ─────────────────┐",
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
fn a_narrow_terminal_leaves_the_inspector_out_and_gives_the_map_view_the_width() {
    let row = ".".repeat(200);
    let world = drawn_world(&vec![row.as_str(); 20]);
    assert_eq!(
        ui::areas(Size::new(99, 30), world.map()).tiles.width,
        97,
        "below 100 columns, the map view takes it all"
    );
    assert_eq!(
        ui::areas(Size::new(100, 30), world.map()).tiles.width,
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

fn died(tick: u64, id: u64, cause: DeathCause, age: u64) -> Event {
    Event {
        tick,
        kind: EventKind::Died {
            id: EntityId(id),
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
    app.record(&[died(4_012, 31, DeathCause::Dehydration, 4_012)]);
    app.record(&[
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
    ]);
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
fn the_event_log_leaves_object_events_out_and_keeps_the_latest_100() {
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
    app.record(&[spawned]);
    assert_eq!(app.event_log().count(), 0);
    for tick in 0..150 {
        app.record(&[died(tick, tick, DeathCause::Starvation, tick)]);
    }
    let ticks: Vec<u64> = app.event_log().map(|event| event.tick).collect();
    assert_eq!(ticks.len(), 100);
    assert_eq!((ticks[0], ticks[99]), (149, 50), "newest first");
}

#[test]
fn a_screen_under_30_rows_has_no_event_log() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 29);
    let screen = lines(&render(&app, &world, 100, 29));
    assert!(screen.iter().all(|line| !line.contains("Events")));
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
        ("Body", "┌─ [Body] Chem Genome World ─────────────────┐"),
        ("Chem", "┌─ Body [Chem] Genome World ─────────────────┐"),
        ("Genome", "┌─ Body Chem [Genome] World ─────────────────┐"),
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
    let title = format!("┌─ Sprite #{id} ── [Body] Chem Genome World ─");
    assert!(top.starts_with(&title), "{top:?}");
    assert!(top.ends_with("─┐"), "{top:?}");
    app.apply(Action::PreviousTab, &world);
    let (top, _) = inspector(&app, &world);
    assert!(
        top.starts_with(&format!("┌─ Sprite #{id} ── Body Chem Genome [World] ─")),
        "{top:?}"
    );
}

#[test]
fn when_the_selected_sprite_dies_its_tabs_say_how_and_at_what_age() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    app.record(&[died(4_012, id.0, DeathCause::Dehydration, 4_012)]);
    for tab in ["Body", "Chem", "Genome"] {
        let (top, text) = inspector(&app, &world);
        assert!(
            top.starts_with(&format!("┌─ Sprite #{} ──", id.0)),
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

/// A 10×5 field of grass with one sprite at (2, 3), made from `genome`, and
/// an app on it that has selected the sprite and stepped the world `ticks`
/// times.
fn one_sprite(genome: &str, ticks: u32) -> (World, App) {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let genome = terra_sim::Genome::from_ron(genome, &pack).expect("a valid genome");
    let sprites = [(terra_sim::Pos { x: 2, y: 3 }, Some(genome))];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
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
    let (world, app) = one_sprite(WORKED_GENOME, 1);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..13],
        [
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
    assert!(text[13..].iter().all(String::is_empty), "{text:?}");
}

#[test]
fn the_chem_tab_lists_every_chemical_with_its_level_and_change_per_tick() {
    let (world, mut app) = one_sprite(WORKED_GENOME, 1);
    app.apply(Action::NextTab, &world);
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
            "reward        .00",
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
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
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
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
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
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
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
    app.apply(
        Action::Wheel {
            at: Position::new(3, 3),
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
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
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
    };
    let world = World::from_scenario(scenario, pack.clone(), 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3), drawn at cell (3, 5).
    app.apply(Action::Click(Position::new(3, 5)), &world);
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
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
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
    assert_eq!(body[7], "boredom     ░░░░░░░░░░ .00", "the Body tab");
    app.apply(Action::NextTab, &world);
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
    app.apply(Action::NextTab, &world);
    app.apply(Action::NextTab, &world);
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
    app.record(&[died(4_012, id.0, DeathCause::Dehydration, 12_345_678)]);
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
    // The first sprite is at (2, 3), drawn at cell (3, 5).
    app.apply(Action::Click(Position::new(3, 5)), &world);
    let rows = right_part(&render(&app, &world, 100, 12), 46);
    assert!(rows[1].contains("[Body]"), "{:?}", rows[1]);
    assert!(inside(&rows[2]).starts_with("age "), "{:?}", rows[2]);
}

#[test]
fn chemical_names_show_with_spaces_for_underscores() {
    // "boredom" renamed "bored_ness" in the pack and the starter genome.
    let files: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let renamed = match path {
                "chemicals.ron" | "genomes/starter.ron" => {
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
    };
    let world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let (_, body) = inspector(&app, &world);
    assert!(body[7].starts_with("bored ness  "), "{:?}", body[7]);
    app.apply(Action::NextTab, &world);
    let (_, chem) = inspector(&app, &world);
    assert!(chem[11].starts_with("bored ness  "), "{:?}", chem[11]);
}
