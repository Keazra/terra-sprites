//! The population graph (M2 design §14): `p` opens it over the panels, with
//! the world's population over its whole run stacked by life stage, and a
//! moment picked with `←` `→` or the pointer read under it, with the
//! generations alive then (#208); a small copy heads the World tab.

mod common;
use common::{
    LAYS_ALONE, LONE, Q, WorldBuilder, app_on, at, group, inspector_rows, pack_changing,
    pointer_on, render, render_on, row, run, sized_app,
};

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::{Color, Modifier};
use terra_sim::{DataPack, Pos, World};
use terra_tui::app::{App, CursorMode, Screen};
use terra_tui::input::Action;
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::theme::Theme;

/// The built-in pack, in which a female may lay alone at once (M2 design
/// §15.7).
fn lone() -> DataPack {
    pack_changing(&[LONE])
}

/// A field of grass with `young` newborn starter sprites along the top row
/// and `adults` grown ones along the next.
fn field(young: u16, adults: u16) -> World {
    let young: Vec<Pos> = (0..young).map(|x| at(x, 0)).collect();
    let adults: Vec<Pos> = (0..adults).map(|x| at(x, 1)).collect();
    WorldBuilder::grass(12, 4)
        .starters(&young)
        .starters(&adults)
        .newborns(&young)
        .seed(7)
        .build()
}

/// A single tile of grass with one newborn starter sprite, which dies of
/// thirst at about tick 4,450.
fn doomed() -> World {
    let tile = at(0, 0);
    WorldBuilder::grass(1, 1)
        .starters(&[tile])
        .newborns(&[tile])
        .build()
}

// At 100×30 the graph's box is rows 1 to 28, its inside rows 2 to 27 from
// column 1; the labels, up to "100%", take 7 columns, so the graph's first
// column is 8. Its 19 rows are 2 to 20, the axis 21, its ticks 22, the
// legend 24 and the three lines that read a moment 25 to 27.
const FIRST_COLUMN: u16 = 8;
const BOTTOM: u16 = 20;
const AXIS: u16 = 21;
const TICKS: u16 = 22;
const LEGEND: u16 = 24;

/// What the three lines under the graph say.
fn reading(buffer: &Buffer) -> [String; 3] {
    [25, 26, 27].map(|y| {
        let line = row(buffer, y);
        line.trim_start_matches('│')
            .trim_end_matches('│')
            .trim()
            .to_string()
    })
}

#[test]
fn p_opens_the_population_graph_over_the_panels_and_p_or_esc_closes_it() {
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    assert_eq!(app.screen(), Screen::Population);
    let screen = render(&app, &world);
    assert!(
        row(&screen, 0).starts_with(" Terra Sprites"),
        "the top bar stays"
    );
    let title = row(&screen, 1);
    assert!(
        title.starts_with("┌─ Population ── ticks 0 to 250 ─"),
        "{title}"
    );
    assert!(title.ends_with(" esc close ─┐"), "{title}");
    assert!(
        row(&screen, 2).starts_with("│ 100% ┤"),
        "{}",
        row(&screen, 2)
    );
    assert!(
        row(&screen, 11).starts_with("│  50% ┤"),
        "half way, in the middle of the middle row of 19: {}",
        row(&screen, 11)
    );
    assert!(
        row(&screen, AXIS).starts_with("│   0% └"),
        "{}",
        row(&screen, AXIS)
    );
    let legend = row(&screen, LEGEND);
    assert!(
        legend.starts_with("│ █ eggs  █ young  █ adults  █ old  as shares of each column"),
        "{legend}"
    );
    assert_eq!(
        reading(&screen),
        [
            "now, tick 250: 6 sprites · generation 1",
            "0 eggs (0%) · 2 young (33%) · 4 adults (67%) · 0 old (0%)",
            "since tick 200: none laid, hatched or died",
        ]
    );
    let status = row(&screen, 29);
    assert!(
        status.ends_with("←→ pick a moment  space pause  +/- speed  esc close"),
        "{status}"
    );
    app.apply(Action::Population, &world);
    assert_eq!(app.screen(), Screen::Normal, "p closes it");
    app.apply(Action::Population, &world);
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal, "and so does Esc");
}

/// A grown female of the first generation alone on a field of grass, who
/// lays alone at once (M2 design §10.4).
fn mother() -> World {
    WorldBuilder::grass(12, 4)
        .genes(&[(at(5, 2), LAYS_ALONE)])
        .data(lone())
        .build()
}

#[test]
fn the_eggs_are_at_the_bottom_of_each_column() {
    // M2 design §14.2: an egg is the youngest a sprite can be, so the eggs
    // are stacked under the young. A mother laying alone (§10.4) and her
    // egg are a column half egg, half adult.
    let mut world = mother();
    run(&mut world, 150);
    assert_eq!(world.eggs().count(), 1, "she laid one, not yet hatched");
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let screen = render(&app, &world);
    let look = Theme::cp437().population();
    let at = |y: u16| {
        let cell = &screen[(FIRST_COLUMN, y)];
        (cell.symbol().to_string(), cell.fg, cell.bg)
    };
    // Half of 38 halves each: 9 rows of egg, a row half egg and half
    // adult, then 9 of adult.
    assert_eq!(at(BOTTOM), ("█".into(), look.egg, Color::Reset));
    assert_eq!(at(BOTTOM - 8), ("█".into(), look.egg, Color::Reset));
    assert_eq!(at(BOTTOM - 9), ("▄".into(), look.egg, look.adult));
    assert_eq!(at(BOTTOM - 10), ("█".into(), look.adult, Color::Reset));
    assert_eq!(
        reading(&screen)[..2],
        [
            "now, tick 150: 1 sprite · generation 1",
            "1 egg (50%) · 0 young (0%) · 1 adult (50%) · 0 old (0%)",
        ]
    );
}

#[test]
fn nothing_drawn_beneath_shows_through_the_graph() {
    // The World tab's small graph has characters of two stages, drawn on a
    // background colour, and the Cursor beneath is in reverse video; the
    // full graph blanks them, colours and all.
    let mut world = field(12, 12);
    run(&mut world, 250);
    let mut app = app_on(&world);
    let beneath = render(&app, &world);
    let inside = || (1..29).flat_map(|y| (0..100).map(move |x| (x, y)));
    assert!(
        inside().any(|(x, y)| beneath[(x, y)].bg != Color::Reset),
        "something beneath has a background"
    );
    app.apply(Action::Population, &world);
    let screen = render(&app, &world);
    for (x, y) in inside() {
        let cell = &screen[(x, y)];
        if cell.symbol() == " " {
            assert_eq!(
                (cell.bg, cell.modifier),
                (Color::Reset, Modifier::empty()),
                "at ({x}, {y})"
            );
        }
    }
}

#[test]
fn each_column_is_split_by_share_from_the_bottom_in_the_theme_s_colours() {
    // M2 design §14.2: youngest to oldest from the bottom, eggs, young,
    // adults and old, each column full height. With no eggs, the 2 young
    // are a third of the column's 38 halves, 13 of them, the 4 adults the
    // 25 above, up to the top.
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let screen = render(&app, &world);
    let look = Theme::cp437().population();
    assert_eq!((look.young, look.adult), (Color::LightGreen, Color::Cyan));
    let at = |y: u16| {
        let cell = &screen[(FIRST_COLUMN, y)];
        (cell.symbol().to_string(), cell.fg, cell.bg)
    };
    assert_eq!(at(BOTTOM), ("█".into(), look.young, Color::Reset));
    assert_eq!(at(BOTTOM - 6), ("▄".into(), look.young, look.adult));
    assert_eq!(at(BOTTOM - 7), ("█".into(), look.adult, Color::Reset));
    assert_eq!(at(2), ("█".into(), look.adult, Color::Reset));
    // The pointer picks the column under it, past the labels.
    app.apply(Action::Point(Position::new(FIRST_COLUMN, 9)), &world);
    assert_eq!(app.graph_pick(), Some(100));
}

#[test]
fn the_graph_grows_from_the_left_then_squeezes_the_whole_run_in() {
    // A column is 100 ticks until the run is longer than the graph is wide.
    // The lone adult has no food or water, so it dies within the run.
    let mut world = field(0, 1);
    run(&mut world, 2_000);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let screen = render(&app, &world);
    let columns = |screen: &Buffer, y: u16| -> String {
        (FIRST_COLUMN..99)
            .map(|x| screen[(x, y)].symbol())
            .collect()
    };
    let one = columns(&screen, BOTTOM);
    assert_eq!(one.trim_end().chars().count(), 20, "{one}");
    let ticks = row(&screen, TICKS);
    assert!(ticks.starts_with("│      0 "), "{ticks}");
    assert!(
        ticks.ends_with(" 8,800   │"),
        "91 columns of 100 ticks, the last quarter marked at 8,800: {ticks}"
    );
    // The moment at tick 1,000, in the tenth column.
    app.apply(Action::Point(Position::new(FIRST_COLUMN + 9, 9)), &world);
    assert_eq!(app.graph_pick(), Some(1_000));
    run(&mut world, 58_000);
    let screen = render(&app, &world);
    let ticks = row(&screen, TICKS);
    assert!(ticks.starts_with("│      0 "), "the whole run: {ticks}");
    assert!(ticks.ends_with(" 58,000   │"), "{ticks}");
    let one = columns(&screen, BOTTOM);
    assert!(one.starts_with('█'), "its life is at the start: {one}");
    assert!(one.ends_with(' '), "and its death before the end: {one}");
    // Squeezed, the picked moment is in the second column, of 659 ticks.
    assert_eq!(screen[(FIRST_COLUMN + 1, AXIS)].symbol(), "▲");
    // Opened afresh, it reads now, in the last column; every column is
    // reached, so `←` picks the one before.
    app.apply(Action::Back, &world);
    app.apply(Action::Population, &world);
    let screen = render(&app, &world);
    assert_eq!(screen[(98, AXIS)].symbol(), "▲");
    assert!(
        reading(&screen)[0].starts_with("now, tick 60,000: 0 sprites"),
        "{:?}",
        reading(&screen)
    );
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    let screen = render(&app, &world);
    assert_eq!(screen[(98 - 1, AXIS)].symbol(), "▲");
}

#[test]
fn arrows_or_the_pointer_pick_a_moment_and_the_lines_under_the_graph_read_it() {
    // Counts at 0, 100 and 200; the columns show the counts at 100 and 200
    // and now.
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let left = Action::Scroll { dx: -1, dy: 0 };
    let right = Action::Scroll { dx: 1, dy: 0 };
    app.apply(left, &world);
    let screen = render(&app, &world);
    assert_eq!(screen[(FIRST_COLUMN + 1, AXIS)].symbol(), "▲");
    assert_eq!(
        reading(&screen),
        [
            "tick 200: 6 sprites · generation 1",
            "0 eggs (0%) · 2 young (33%) · 4 adults (67%) · 0 old (0%)",
            "since tick 100: none laid, hatched or died",
        ]
    );
    app.apply(left, &world);
    app.apply(left, &world);
    let screen = render(&app, &world);
    assert_eq!(
        screen[(FIRST_COLUMN, AXIS)].symbol(),
        "▲",
        "it stops at the first"
    );
    assert_eq!(
        reading(&screen)[2],
        "since tick 0: none laid, hatched or died"
    );
    app.apply(right, &world);
    app.apply(right, &world);
    app.apply(right, &world);
    let screen = render(&app, &world);
    assert_eq!(screen[(FIRST_COLUMN + 2, AXIS)].symbol(), "▲", "and at now");
    assert!(reading(&screen)[0].starts_with("now, tick 250:"));
    assert_eq!(app.graph_pick(), None, "which reads now as time runs");
    // The pointer over a column picks its moment; over one the run hasn't
    // reached, or the labels, it leaves the pick as it is.
    app.apply(Action::Point(Position::new(FIRST_COLUMN + 1, 9)), &world);
    assert_eq!(app.graph_pick(), Some(200));
    app.apply(Action::Point(Position::new(FIRST_COLUMN + 40, 9)), &world);
    app.apply(Action::Point(Position::new(3, 9)), &world);
    assert_eq!(app.graph_pick(), Some(200));
    let screen = render(&app, &world);
    assert!(reading(&screen)[0].starts_with("tick 200:"));
    // Closed and opened again, it reads now.
    app.apply(Action::Back, &world);
    app.apply(Action::Population, &world);
    assert_eq!(app.graph_pick(), None);
}

#[test]
fn the_pointer_picks_a_moment_and_leaves_the_cursor_where_it_is_on_the_map() {
    // M2 design §14.2: the pointer over the graph picks moments rather
    // than moving the Cursor on the map beneath.
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    let tile = at(5, 1);
    app.apply(Action::Point(pointer_on(&app, tile)), &world);
    assert_eq!(app.shown_cursor(), Some(tile));
    app.apply(Action::Population, &world);
    app.apply(Action::Point(Position::new(FIRST_COLUMN + 1, 9)), &world);
    assert_eq!(app.graph_pick(), Some(200));
    assert_eq!(app.shown_cursor(), Some(tile), "the pointer moved");
    app.apply(Action::left_click(Position::new(FIRST_COLUMN, 15)), &world);
    assert_eq!(app.graph_pick(), Some(100));
    assert_eq!(app.shown_cursor(), Some(tile), "a click");
    app.apply(Action::Population, &world);
    assert_eq!(app.shown_cursor(), Some(tile), "and closed");
}

#[test]
fn a_picked_moment_stays_picked_as_time_runs_and_now_moves_on() {
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let mut now = app_on(&world);
    now.apply(Action::Population, &world);
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    run(&mut world, 1_000);
    let screen = render(&app, &world);
    assert_eq!(screen[(FIRST_COLUMN + 1, AXIS)].symbol(), "▲");
    assert!(reading(&screen)[0].starts_with("tick 200:"));
    let screen = render(&now, &world);
    assert_eq!(screen[(FIRST_COLUMN + 12, AXIS)].symbol(), "▲");
    assert!(reading(&screen)[0].starts_with("now, tick 1,250:"));
}

#[test]
fn the_reading_says_which_generations_were_alive_then() {
    // #208: the mother is of the first generation, and her egg of the
    // second, but an egg isn't a sprite until it hatches (M2 design §10.4).
    let mut world = mother();
    run(&mut world, 150);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    assert_eq!(
        reading(&render(&app, &world))[0],
        "now, tick 150: 1 sprite · generation 1"
    );
    for _ in 0..2_000 {
        if world.sprites().count() == 2 {
            break;
        }
        world.step();
    }
    assert_eq!(world.sprites().count(), 2, "her egg hatched");
    let hatched = world.tick();
    run(&mut world, 100);
    assert_eq!(
        reading(&render(&app, &world))[0],
        format!(
            "now, tick {}: 2 sprites · generations 1 to 2",
            group(hatched + 100)
        )
    );
    // The first column reads the count at tick 100, before the hatching.
    app.apply(Action::Point(Position::new(FIRST_COLUMN, 9)), &world);
    assert_eq!(
        reading(&render(&app, &world))[0],
        "tick 100: 1 sprite · generation 1"
    );
}

/// A save of schema 1 from before the population record kept generations
/// (the golden save `terra-sim`'s `lineage.rs` describes), written by the
/// build that merged PR #201 on tick 105, with counts at ticks 0 and 100.
const GOLDEN_V1_BEFORE_LINEAGE: &[u8] =
    include_bytes!("../../terra-sim/tests/golden/save-v1-before-lineage.tspr");

#[test]
fn a_save_from_before_generations_reads_none_for_its_earlier_moments() {
    // #208: its counts have none, so their moments read no generations;
    // now, its sprites have theirs, numbered when it loaded (M2 design
    // §17.1).
    let world = World::load(GOLDEN_V1_BEFORE_LINEAGE).expect("the save loads");
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let now = reading(&render(&app, &world));
    assert_eq!(now[0], "now, tick 105: 4 sprites · generations 2 to 3");
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    let then = reading(&render(&app, &world));
    assert!(then[0].starts_with("tick 100: "), "{then:?}");
    assert!(!then[0].contains("generation"), "{then:?}");
}

#[test]
fn the_small_graph_reaches_past_most_sprites_when_the_cursor_spawned_past_it() {
    // M2 design §14.2: 101 sprites in a world that holds 100. The full
    // graph shows shares, so only the small one has a height in sprites.
    let sprites: Vec<Pos> = (0..101).map(|i| at(i % 12, i / 12)).collect();
    let world = WorldBuilder::grass(12, 9)
        .starters(&sprites)
        .seed(7)
        .build();
    let app = app_on(&world);
    // The graph's top row, under its title.
    let top = &inspector_rows(&app, &world, 30).1[1];
    assert!(top.starts_with(" 101 ┤"), "{top}");
}

#[test]
fn a_death_is_read_in_the_moment_it_happened() {
    let mut world = doomed();
    run(&mut world, 5_000);
    assert_eq!(world.sprites().count(), 0, "it died of thirst");
    let died = world
        .population()
        .iter()
        .position(|count| count.since.deaths() > 0)
        .expect("a count with the death");
    let count = &world.population()[died];
    // A column is 100 ticks, so the count at tick T is column T/100 - 1.
    let column = (count.tick / 100 - 1) as u16;
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let cell = Position::new(FIRST_COLUMN + column, 15);
    app.apply(Action::left_click(cell), &world);
    let screen = render(&app, &world);
    let tick = count.tick;
    assert_eq!(
        reading(&screen),
        [
            format!("tick {}: 0 sprites", group(tick)),
            "0 eggs · 0 young · 0 adults · 0 old".to_string(),
            format!("since tick {}: 1 died (dehydration 1)", group(tick - 100)),
        ]
    );
}

#[test]
fn time_runs_as_usual_while_the_graph_is_open() {
    let world = field(2, 4);
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let speed = app.clock.speed();
    app.apply(Action::TogglePause, &world);
    assert!(app.clock.is_paused());
    app.apply(Action::StepOnce, &world);
    let stepped = app.clock.advance(Duration::ZERO, || {}, || false);
    assert!(stepped > 0, "`.` steps while paused");
    app.apply(Action::Faster { held: false }, &world);
    assert_ne!(app.clock.speed(), speed);
    app.apply(Action::Slower { held: false }, &world);
    assert_eq!(app.clock.speed(), speed, "and `-` slows it again");
    app.apply(Action::TogglePause, &world);
    assert!(!app.clock.is_paused());
    assert_eq!(app.screen(), Screen::Population, "and it stays open");
}

#[test]
fn other_keys_do_nothing_while_the_graph_is_open() {
    // M2 design §14.2: only the graph's own keys and those for time.
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    let state = |app: &App| {
        (
            app.selection(),
            app.tab(),
            app.colour_mode(),
            app.mode(),
            app.tracking(),
            app.detail(),
            app.event_filter(),
            app.shown_cursor(),
        )
    };
    let before = state(&app);
    app.apply(Action::Population, &world);
    let keys = [
        Action::SelectNext,
        Action::SelectPrevious,
        Action::NextTab,
        Action::PreviousTab,
        Action::CycleColours,
        Action::Mode(CursorMode::Grab),
        Action::Track,
        Action::ToggleDetail,
        Action::CycleEventFilter,
        Q,
        Action::Rename,
        Action::Help,
        Action::SpriteList,
        Action::FamilyTree,
        Action::Enter,
    ];
    for key in keys {
        app.apply(key, &world);
        assert_eq!(app.screen(), Screen::Population, "{key:?}");
        assert_eq!(state(&app), before, "{key:?}");
    }
}

#[test]
fn the_world_tab_starts_with_a_small_graph() {
    let mut world = field(2, 4);
    run(&mut world, 250);
    let app = app_on(&world);
    let screen = render(&app, &world);
    // The inspector is the right-hand 46 columns, its inside from column 55.
    let inspector = |y: u16| -> String {
        (55..99)
            .map(|x| screen[(x, y)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    };
    assert_eq!(inspector(2), " population, ticks 0 to 250");
    assert!(inspector(3).starts_with(" 100 ┤··"), "{}", inspector(3));
    assert!(inspector(6).starts_with("  50 ┤"), "{}", inspector(6));
    assert!(inspector(9).starts_with("   0 └──"), "{}", inspector(9));
    assert_eq!(inspector(10), "");
    assert!(inspector(11).starts_with(" data pack"), "{}", inspector(11));
}

#[test]
fn a_loaded_world_shows_its_whole_graph() {
    let mut world = field(2, 4);
    run(&mut world, 1_250);
    let loaded = World::load(&world.save()).expect("the save loads");
    let mut app = app_on(&world);
    app.apply(Action::Population, &world);
    let (before, after) = (render(&app, &world), render(&app, &loaded));
    for y in 1..28 {
        assert_eq!(row(&after, y), row(&before, y), "row {y}");
    }
    assert!(row(&after, 1).starts_with("┌─ Population ── ticks 0 to 1,250 ─"));
}

/// A policy that shows everything but the population graph.
struct NoGraph;

impl InfoPolicy for NoGraph {
    fn can_view(&self, panel: Panel, _subject: Subject) -> bool {
        panel != Panel::Graph
    }
}

#[test]
fn the_policy_may_hide_the_graph() {
    // Design §6.4: the box stays, empty, and the World tab leaves it out.
    let mut world = field(2, 4);
    run(&mut world, 250);
    let mut app = app_on(&world);
    app.set_policy(NoGraph);
    let screen = render(&app, &world);
    let first: String = (55..99).map(|x| screen[(x, 2)].symbol()).collect();
    assert!(first.starts_with(" data pack"), "{first}");
    app.apply(Action::Population, &world);
    let screen = render(&app, &world);
    assert!(
        row(&screen, 1).starts_with("┌─ Population ───"),
        "{}",
        row(&screen, 1)
    );
    for y in 2..28 {
        assert_eq!(row(&screen, y).trim_matches(['│', ' ']), "", "row {y}");
    }
}

#[test]
fn a_column_narrower_than_the_time_between_counts_reads_the_count_it_shows() {
    // M2 design §14.3: squeezed, the record counts every 200 ticks, but
    // 700 columns wide, a column is still 100 ticks, so every other one
    // shows the same count as the one before.
    let mut world = field(0, 1);
    run(&mut world, 52_000);
    assert_eq!(world.population_every(), 200);
    let size = Size::new(700, 30);
    let mut app = sized_app(&world, Theme::cp437(), size);
    app.apply(Action::Population, &world);
    app.apply(Action::Point(Position::new(FIRST_COLUMN + 2, 9)), &world);
    assert_eq!(app.graph_pick(), Some(300));
    let screen = render_on(&app, &world, size);
    assert_eq!(screen[(FIRST_COLUMN + 2, AXIS)].symbol(), "▲");
    let [first, _, since] = reading(&screen);
    assert!(first.starts_with("tick 200: "), "{first}");
    assert_eq!(since, "since tick 0: none laid, hatched or died");

    // The first column shows the first count, and says so (§14.3).
    app.apply(Action::Point(Position::new(FIRST_COLUMN, 9)), &world);
    let screen = render_on(&app, &world, size);
    assert_eq!(screen[(FIRST_COLUMN, AXIS)].symbol(), "▲");
    let [first, _, since] = reading(&screen);
    assert!(first.starts_with("tick 0: 1 sprite"), "{first}");
    assert_eq!(since, "the first count");
}
