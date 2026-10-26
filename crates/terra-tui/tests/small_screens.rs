//! Smaller screens step down rather than saying "Terminal too small", the
//! inspector fills the screen, and tiles can be drawn two cells wide
//! (design §6.1, M2 design §11.7, #305).

mod common;

use std::collections::BTreeSet;

use common::screen::inspector_on;
use common::*;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use terra_sim::World;
use terra_tui::app::{App, CursorMode, Screen, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;
use terra_tui::ui;

/// A field of grass bigger than any map view here, with sprites at (5, 5)
/// and (30, 8) and a ball at (7, 7).
fn field_world() -> World {
    WorldBuilder::grass(200, 80)
        .starters(&[at(5, 5), at(30, 8)])
        .objects(&[(at(7, 7), "ball")])
        .build()
}

/// A field of grass smaller than the map view, with a sprite at (5, 5).
fn small_world() -> World {
    WorldBuilder::grass(20, 10).starters(&[at(5, 5)]).build()
}

/// An app on `world` laid out for a screen of `size`, in the CP437 theme.
fn app_sized(world: &World, size: Size) -> App {
    sized_app(world, Theme::cp437(), size)
}

/// The last row of `screen`: the status line.
fn bottom(screen: &Buffer) -> String {
    row(screen, screen.area.height - 1)
}

/// The columns of tiles the map view draws on screen row `y`.
fn tiles_across(app: &App, y: u16) -> BTreeSet<u16> {
    (0..400)
        .filter_map(|x| app.tile_at(Position::new(x, y)).map(|tile| tile.x))
        .collect()
}

/// Scrolls the view as far up and left as it goes, so the map's top-left
/// corner is in view.
fn to_the_corner(app: &mut App, world: &World) {
    apply(app, world, Action::Scroll { dx: -500, dy: -500 });
}

#[test]
fn under_100_columns_the_inspector_is_shut_until_i_opens_it_over_the_map() {
    let world = field_world();
    let size = Size::new(80, 24);
    let areas = ui::areas(size);
    assert_eq!(areas.tiles.width, 80, "the map view takes the whole width");
    assert!(areas.pops_over);
    let mut app = app_sized(&world, size);
    select(&mut app, &world, sprite_on(&world, at(5, 5)));
    let shut = render_on(&app, &world, size);
    assert!(!row(&shut, 1).contains("Body"), "{}", row(&shut, 1));
    assert!(bottom(&shut).contains("i inspector"), "{}", bottom(&shut));

    apply(&mut app, &world, Action::Inspector);
    assert!(app.inspector_open());
    let open = render_on(&app, &world, size);
    let title = row(&open, 1);
    assert!(title.ends_with("Body  Brain Chem Genome World"), "{title}");
    assert!(
        title.starts_with(' ') && title.contains("Sprite #"),
        "{title}"
    );
    assert!(
        lines(&open).iter().any(|line| line.contains("hunger")),
        "the Body tab"
    );

    apply(&mut app, &world, Action::Back);
    assert!(!app.inspector_open(), "Esc closes it");
    let again = render_on(&app, &world, size);
    assert!(!row(&again, 1).contains("Body"));
}

#[test]
fn the_open_pop_over_takes_clicks_and_the_wheel_off_the_map_beneath_it() {
    let world = field_world();
    let size = Size::new(80, 24);
    let mut app = app_sized(&world, size);
    let far = sprite_on(&world, at(30, 8));
    let near = sprite_on(&world, at(5, 5));
    select(&mut app, &world, near);
    apply(&mut app, &world, Action::Track);
    apply(&mut app, &world, Action::Track);
    let cell = app.cell_of(at(30, 8)).expect("in view");
    let inspector = app.inspector_area();
    assert!(inspector.is_none(), "shut, it's drawn nowhere");
    apply(&mut app, &world, Action::Inspector);
    let inspector = app.inspector_area().expect("open");
    let pointer = Position::new(cell.x + 1, cell.y + 1);
    assert!(inspector.contains(pointer), "the far sprite is under it");
    apply(&mut app, &world, Action::left_click(pointer));
    assert_eq!(app.selection(), Some(Selection::Living(near)));
    apply(
        &mut app,
        &world,
        Action::Wheel {
            at: pointer,
            notches: 1,
        },
    );
    assert_eq!(app.mode(), CursorMode::Select, "the wheel scrolls the tab");

    apply(&mut app, &world, Action::Back);
    apply(&mut app, &world, Action::left_click(pointer));
    assert_eq!(
        app.selection(),
        Some(Selection::Living(far)),
        "shut, the click lands on the map"
    );
}

#[test]
fn under_30_rows_the_event_log_goes_and_with_too_few_for_the_inspector_the_map_is_alone() {
    let short = ui::areas(Size::new(120, 29));
    assert!(short.event_log.is_none() && short.inspector.is_some());
    for size in [Size::new(40, 12), Size::new(120, 9), Size::new(20, 5)] {
        let areas = ui::areas(size);
        assert!(
            areas.inspector.is_none() && areas.event_log.is_none(),
            "{size:?}"
        );
        assert_eq!(areas.tiles.width, size.width, "{size:?}");
        assert_eq!(
            areas.tiles.height,
            size.height - 2,
            "{size:?}: the bars stay"
        );
    }
    let world = field_world();
    let size = Size::new(40, 12);
    let mut app = app_sized(&world, size);
    apply(&mut app, &world, Action::Inspector);
    assert_eq!(app.refusal(), Some("No room for the inspector"));
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.help_page_here().label(), "World", "still the map's");
}

#[test]
fn short_of_room_the_top_bar_keeps_the_time_and_the_help_key() {
    let world = field_world();
    let app = app_sized(&world, Size::new(40, 12));
    let bar = row(&render_on(&app, &world, Size::new(40, 12)), 0);
    assert!(bar.starts_with(" tick 0 │ ► 1x"), "{bar}");
    assert!(bar.ends_with("? help"), "{bar}");
    assert!(
        !bar.contains("seed") && !bar.contains("Terra Sprites"),
        "{bar}"
    );
    let wide = row(&render_on(&app, &world, Size::new(100, 30)), 0);
    assert!(wide.starts_with(" Terra Sprites │ tick 0"), "{wide}");
}

#[test]
fn i_fills_the_screen_with_the_open_tab_in_columns_and_i_or_esc_takes_it_back() {
    let world = field_world();
    let id = sprite_on(&world, at(5, 5));
    // The Genome tab's lines, from a screen tall enough for them all.
    let mut tall = app_sized(&world, Size::new(100, 200));
    select(&mut tall, &world, id);
    open(&mut tall, &world, Tab::Genome);
    let (_, docked) = inspector_on(&tall, &world, 200);
    let docked: Vec<String> = docked.into_iter().filter(|line| !line.is_empty()).collect();

    let size = Size::new(140, 40);
    let mut app = app_sized(&world, size);
    select(&mut app, &world, id);
    open(&mut app, &world, Tab::Genome);
    apply(&mut app, &world, Action::Inspector);
    assert_eq!(app.screen(), Screen::Inspector);
    let screen = render_on(&app, &world, size);
    assert!(row(&screen, 1).contains("Genome"), "its title row");
    // Three columns of 45, two apart, read down each and on to the next.
    let mut shown = vec![];
    for column in 0..3 {
        let left = 1 + column * 47;
        for y in 2..size.height - 1 {
            let line: String = (left..left + 45).map(|x| screen[(x, y)].symbol()).collect();
            let line = line.trim().to_string();
            if !line.is_empty() {
                shown.push(line);
            }
        }
    }
    assert_eq!(shown[..], docked[..shown.len()], "its lines in order");
    let third: String = (95..140).map(|x| screen[(x, 2)].symbol()).collect();
    assert!(!third.trim().is_empty(), "on to a third column");
    assert!(
        bottom(&screen).contains("[ ] switch tabs"),
        "{}",
        bottom(&screen)
    );
    assert!(bottom(&screen).contains("esc close"), "{}", bottom(&screen));
    assert_eq!(app.help_page_here().label(), "Inspector");

    apply(&mut app, &world, Action::Inspector);
    assert_eq!(app.screen(), Screen::Normal, "i again takes it back");
    apply(&mut app, &world, Action::Inspector);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal, "and so does Esc");
}

#[test]
fn filling_the_screen_the_inspector_takes_its_own_keys_and_time_s_but_not_the_map_s() {
    let world = field_world();
    let mut app = app_sized(&world, Size::new(100, 30));
    select(&mut app, &world, sprite_on(&world, at(5, 5)));
    apply(&mut app, &world, Action::Inspector);
    let viewport = app.viewport();
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::Scroll { dx: 3, dy: 0 });
    assert_eq!(app.mode(), CursorMode::Select);
    assert_eq!(app.viewport(), viewport);
    apply(&mut app, &world, Action::NextTab);
    assert_eq!(app.tab(), Tab::Brain);
    apply(&mut app, &world, Action::TogglePause);
    assert!(app.clock.is_paused());
    assert_eq!(app.screen(), Screen::Inspector);
}

#[test]
fn on_a_narrow_screen_i_opens_the_pop_over_then_fills_the_screen() {
    let world = field_world();
    let mut app = app_sized(&world, Size::new(80, 24));
    apply(&mut app, &world, Action::Inspector);
    assert!(app.inspector_open() && app.screen() == Screen::Normal);
    apply(&mut app, &world, Action::Inspector);
    assert_eq!(app.screen(), Screen::Inspector);
    apply(&mut app, &world, Action::Back);
    assert!(app.inspector_open(), "back to the pop-over");
    apply(&mut app, &world, Action::Back);
    assert!(!app.inspector_open());
}

#[test]
fn zoomed_the_map_view_shows_47_by_34_tiles_at_140_by_40_and_27_by_24_at_100_by_30() {
    let world = field_world();
    for (size, across, down, zoomed) in [
        (Size::new(140, 40), 94, 34, 47),
        (Size::new(100, 30), 54, 24, 27),
    ] {
        let mut app = app_sized(&world, size);
        let row = ui::areas(size).tiles.y + 1;
        assert_eq!(tiles_across(&app, row).len(), across, "{size:?}");
        apply(&mut app, &world, Action::Zoom);
        assert!(app.zoomed());
        assert_eq!(app.tile_width(), 2);
        assert_eq!(tiles_across(&app, row).len(), zoomed, "{size:?}");
        assert_eq!(ui::areas(size).tiles.height, down, "{size:?}");
        apply(&mut app, &world, Action::Zoom);
        assert_eq!(tiles_across(&app, row).len(), across, "{size:?}: and back");
    }
}

#[test]
fn zoomed_terrain_repeats_in_a_tile_s_second_cell_and_a_sprite_or_an_object_leaves_it_blank() {
    let world = field_world();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Zoom);
    to_the_corner(&mut app, &world);
    let screen = render(&app, &world);
    let sprite = app.cell_of(at(5, 5)).expect("in view");
    assert_eq!(screen[sprite].symbol(), "☺");
    assert_eq!(screen[(sprite.x + 1, sprite.y)].symbol(), " ");
    let ball = app.cell_of(at(7, 7)).expect("in view");
    assert_eq!(screen[(ball.x + 1, ball.y)].symbol(), " ");
    let grass = app.cell_of(at(10, 10)).expect("in view");
    assert_eq!(screen[grass].symbol(), ".");
    assert_eq!(screen[(grass.x + 1, grass.y)].symbol(), ".");
    assert_eq!(app.cell_of(at(11, 10)).unwrap().x, grass.x + 2);
}

#[test]
fn zoomed_the_glass_meets_the_map_on_every_side() {
    let world = small_world();
    let size = Size::new(100, 30);
    let mut app = app_sized(&world, size);
    apply(&mut app, &world, Action::Zoom);
    let screen = render_on(&app, &world, size);
    let first = app.cell_of(at(0, 0)).expect("in view");
    let last = app.cell_of(at(19, 9)).expect("in view");
    let at_cell = |x: u16, y: u16| screen[(x, y)].symbol().to_string();
    assert_eq!(
        at_cell(first.x - 1, first.y),
        "║",
        "the left glass, by the map"
    );
    assert_eq!(at_cell(first.x - 2, first.y), " ");
    assert_eq!(at_cell(first.x - 1, first.y - 1), "╔");
    assert_eq!(at_cell(first.x, first.y - 1), "═");
    assert_eq!(at_cell(first.x + 1, first.y - 1), "═");
    assert_eq!(
        at_cell(last.x + 2, last.y),
        "║",
        "the right glass, by the map"
    );
    assert_eq!(at_cell(last.x + 3, last.y), " ");
    assert_eq!(at_cell(last.x + 2, last.y + 1), "╝");
}

#[test]
fn zoomed_a_click_on_either_cell_of_a_tile_lands_on_it() {
    let world = field_world();
    let id = sprite_on(&world, at(5, 5));
    for offset in [1, 2] {
        let mut app = app_on(&world);
        apply(&mut app, &world, Action::Zoom);
        to_the_corner(&mut app, &world);
        let cell = app.cell_of(at(5, 5)).expect("in view");
        // The pointer points one down and right of where it is (design
        // v27 §6.5).
        let pointer = Position::new(cell.x + offset, cell.y + 1);
        let under = Position::new(cell.x + offset - 1, cell.y);
        assert_eq!(app.tile_at(under), Some(at(5, 5)), "{offset}");
        apply(&mut app, &world, Action::left_click(pointer));
        assert_eq!(app.selection(), Some(Selection::Living(id)), "{offset}");
    }
}

#[test]
fn zoomed_the_cursor_frames_its_two_cell_tile() {
    let world = small_world();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Zoom);
    let screen = render(&app, &world);
    let centre = app.cell_of(app.cursor()).expect("in view");
    let arrows = app.theme.arrows();
    assert_eq!(
        screen[(centre.x - 1, centre.y)].symbol(),
        arrows.right.to_string()
    );
    assert_eq!(
        screen[(centre.x + 2, centre.y)].symbol(),
        arrows.left.to_string()
    );
    assert_eq!(
        screen[(centre.x, centre.y - 1)].symbol(),
        arrows.down.to_string()
    );
}

#[test]
fn zoom_keeps_the_tile_in_the_middle_of_the_view_and_tracking_keeps_up() {
    let world = field_world();
    let mut app = app_on(&world);
    let tiles = ui::areas(SCREEN).tiles;
    let middle = Position::new(tiles.x + tiles.width / 2, tiles.y + tiles.height / 2);
    let before = app.tile_at(middle).expect("a tile");
    apply(&mut app, &world, Action::Zoom);
    let after = app.tile_at(middle).expect("a tile");
    assert!(
        after.x.abs_diff(before.x) <= 1 && after.y == before.y,
        "{before:?} {after:?}"
    );

    select(&mut app, &world, sprite_on(&world, at(30, 8)));
    apply(&mut app, &world, Action::Track);
    let cell = app.cell_of(at(30, 8)).expect("tracked into view");
    assert!(cell.x.abs_diff(middle.x) <= 2, "{cell:?} {middle:?}");
}

#[test]
fn o_says_what_it_did() {
    let world = field_world();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Zoom);
    assert_eq!(
        app.notice(),
        Some("Tiles two cells wide, so they're square")
    );
    apply(&mut app, &world, Action::Zoom);
    assert_eq!(app.notice(), Some("Tiles one cell wide"));
}

#[test]
fn i_and_o_are_in_the_help() {
    let world = field_world();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Help);
    let shown = lines(&render(&app, &world)).join("\n");
    assert!(shown.contains("o          square tiles"), "{shown}");
    assert!(shown.contains("i          open, or full screen"), "{shown}");
}

#[test]
fn a_narrow_help_stacks_its_columns_rather_than_cutting_them() {
    let world = field_world();
    let size = Size::new(60, 20);
    let mut app = app_sized(&world, size);
    apply(&mut app, &world, Action::Inspector);
    apply(&mut app, &world, Action::Back);
    apply(&mut app, &world, Action::Help);
    let shown = lines(&render_on(&app, &world, size));
    let heading = |name: &str| {
        shown
            .iter()
            .position(|line| line.contains(name))
            .unwrap_or_else(|| panic!("{name} in {shown:#?}"))
    };
    assert!(
        heading("THE POINTER") > heading("INSPECTOR"),
        "one below the other"
    );
    assert!(
        shown
            .iter()
            .any(|line| line.contains("enter      the same, where it points")),
        "uncut: {shown:#?}"
    );
}

#[test]
fn the_title_screen_needs_60_by_20() {
    use terra_tui::args::Args;
    use terra_tui::start;
    use terra_tui::title::{self, Title};
    let draw = |size: Size| {
        let world = start::title_world(&Args::default(), 3, (size.width, size.height)).unwrap();
        let mut title = Title::new(world, Theme::cp437(), 1, size);
        title.animate(std::time::Duration::from_secs(10));
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(size.width, size.height))
                .unwrap();
        terminal.draw(|frame| title::render(frame, &title)).unwrap();
        lines(terminal.backend().buffer()).join("\n")
    };
    let fits = draw(Size::new(60, 20));
    assert!(
        fits.contains("1 New world") && fits.contains("4 Quit"),
        "{fits}"
    );
    let small = draw(Size::new(59, 20));
    assert!(small.contains("needs 60x20, this is 59x20"), "{small}");
}

#[test]
fn a_click_on_a_button_other_than_the_left_does_nothing_while_the_inspector_fills_the_screen() {
    let world = field_world();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Inspector);
    let before = app.selection();
    apply(
        &mut app,
        &world,
        Action::Click {
            at: Position::new(10, 10),
            button: Button::Right,
            amplified: false,
        },
    );
    assert_eq!(app.selection(), before);
    assert_eq!(app.screen(), Screen::Inspector);
}
