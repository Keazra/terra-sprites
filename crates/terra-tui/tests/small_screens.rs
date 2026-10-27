//! Smaller screens step down rather than saying "Terminal too small", the
//! inspector fills the screen, and tiles can be drawn two cells wide
//! (design §6.1, M2 design §11.7, #305).

mod common;

use std::collections::BTreeSet;

use common::screen::inspector_on;
use common::*;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use terra_sim::{Pos, World};
use terra_tui::app::{App, CursorMode, MAP_HIDDEN, Screen, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::saves::{self, QUICKSAVE};
use terra_tui::theme::Theme;
use terra_tui::{controls, ui};

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
    let mut app = sized_app(&world, Theme::cp437(), size);
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
fn the_inspector_open_over_the_map_takes_clicks_and_the_wheel_off_the_map_beneath_it() {
    let world = field_world();
    let size = Size::new(80, 24);
    let mut app = sized_app(&world, Theme::cp437(), size);
    let far = sprite_on(&world, at(30, 8));
    let near = sprite_on(&world, at(5, 5));
    select(&mut app, &world, near);
    apply(&mut app, &world, Action::Track);
    apply(&mut app, &world, Action::Track);
    let pointer = pointer_on(&app, at(30, 8));
    let inspector = app.inspector_area();
    assert!(inspector.is_none(), "shut, it's drawn nowhere");
    apply(&mut app, &world, Action::Inspector);
    let inspector = app.inspector_area().expect("open");
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

/// Centres the view on the sprite on `tile`, with Track, scrolls it down
/// to the map view's last row and selects nothing, on a screen of `size`;
/// and says where `tile` is drawn.
fn on_the_last_row(app: &mut App, world: &World, size: Size, tile: Pos) -> Position {
    let tiles = ui::areas(size).tiles;
    select(app, world, sprite_on(world, tile));
    apply(app, world, Action::Track);
    let cell = app.cell_of(tile).expect("centred");
    let down = i32::from(tiles.bottom() - 1 - cell.y);
    apply(app, world, Action::Scroll { dx: 0, dy: -down });
    let cell = app.cell_of(tile).expect("in view");
    assert_eq!(cell.y, tiles.bottom() - 1, "on the map view's last row");
    // A click on empty grass, at the map view's top left.
    apply(
        app,
        world,
        Action::left_click(Position::new(tiles.x + 3, tiles.y + 3)),
    );
    assert_eq!(app.selection(), None);
    cell
}

#[test]
fn the_inspector_open_over_the_map_hides_it_from_the_pointer_below_it_too() {
    let world = WorldBuilder::grass(200, 80).starters(&[at(60, 40)]).build();
    let size = Size::new(80, 24);
    let mut app = sized_app(&world, Theme::cp437(), size);
    let cell = on_the_last_row(&mut app, &world, size, at(60, 40));
    apply(&mut app, &world, Action::Inspector);
    let inspector = app.inspector_area().expect("open");
    assert!(inspector.contains(cell), "the sprite is under it");
    // On the status line, the pointer points a row up and a cell left, at
    // the sprite under the inspector.
    let below = Position::new(cell.x + 1, cell.y + 1);
    assert!(!inspector.contains(below));
    apply(&mut app, &world, Action::left_click(below));
    assert_eq!(app.selection(), None, "the click reaches nothing");
}

#[test]
fn filling_the_screen_the_inspector_hides_the_map_and_the_event_log_from_the_pointer() {
    let world = WorldBuilder::grass(200, 80).starters(&[at(60, 40)]).build();
    // No event log, so the status line is just below the map view.
    let size = Size::new(100, 24);
    let mut app = sized_app(&world, Theme::cp437(), size);
    let cell = on_the_last_row(&mut app, &world, size, at(60, 40));
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
    apply(
        &mut app,
        &world,
        Action::left_click(Position::new(cell.x + 1, cell.y + 1)),
    );
    assert_eq!(app.selection(), None, "the hidden map takes no click");

    let mut app = app_on(&world);
    let filters = cell_of_text(&app, &world, SCREEN, "selected major");
    let filter = app.event_filter();
    apply(&mut app, &world, Action::Inspector);
    let screen = render(&app, &world);
    assert!(
        !row(&screen, filters.y).contains("selected major"),
        "hidden with the event log"
    );
    apply(&mut app, &world, Action::left_click(filters));
    assert_eq!(
        app.event_filter(),
        filter,
        "the hidden filters take no click"
    );
}

#[test]
fn open_over_the_map_the_inspector_leaves_the_view_where_it_was_and_centres_beside_it() {
    let world = field_world();
    let size = Size::new(80, 24);
    let tiles = ui::areas(size).tiles;
    let mut app = sized_app(&world, Theme::cp437(), size);
    to_the_corner(&mut app, &world);
    let before = app.cell_of(at(5, 5));
    apply(&mut app, &world, Action::Inspector);
    let inspector = app.inspector_area().expect("open");
    assert_eq!(
        app.cell_of(at(5, 5)),
        before,
        "opening it doesn't move the view"
    );

    // `Tab` and Track centre a sprite in the part of the view beside it.
    select(&mut app, &world, sprite_on(&world, at(30, 8)));
    let cell = app.cell_of(at(30, 8)).expect("in view");
    assert!(cell.x < inspector.x, "{cell:?} {inspector:?}");
    apply(&mut app, &world, Action::Track);
    let cell = app.cell_of(at(30, 8)).expect("in view");
    assert!(cell.x < inspector.x, "{cell:?} {inspector:?}");

    // The map's last column can be scrolled to beside it.
    apply(&mut app, &world, Action::Scroll { dx: 500, dy: 0 });
    let last = app.cell_of(at(199, 8)).expect("in view");
    assert!(last.x < inspector.x, "{last:?} {inspector:?}");
    // Shut, the view goes back to a quarter of itself past the wall.
    apply(&mut app, &world, Action::Back);
    assert!(!app.inspector_open());
    let last = app.cell_of(at(199, 8)).expect("in view");
    assert_eq!(last.x - tiles.x, tiles.width - 1 - tiles.width / 4);
}

#[test]
fn a_cursor_left_under_the_inspector_is_hidden_and_q_reaches_nothing_there() {
    let world = field_world();
    let size = Size::new(80, 24);
    let mut app = sized_app(&world, Theme::cp437(), size);
    to_the_corner(&mut app, &world);
    let far = sprite_on(&world, at(30, 8));
    let pointer = pointer_on(&app, at(30, 8));
    apply(&mut app, &world, Action::Point(pointer));
    assert_eq!(app.shown_cursor(), Some(at(30, 8)));
    apply(&mut app, &world, Action::Inspector);
    assert!(
        app.inspector_area()
            .is_some_and(|area| area.contains(app.cell_of(at(30, 8)).unwrap()))
    );
    assert_eq!(app.shown_cursor(), None, "under the inspector");
    let q = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, q);
    assert_eq!(app.selection(), None);

    apply(&mut app, &world, Action::Back);
    assert_eq!(app.shown_cursor(), Some(at(30, 8)), "shut, it's back");
    apply(&mut app, &world, q);
    assert_eq!(app.selection(), Some(Selection::Living(far)));
}

#[test]
fn shrunk_to_no_room_and_back_the_inspector_over_the_map_is_shut() {
    let world = field_world();
    let size = Size::new(80, 24);
    let mut app = sized_app(&world, Theme::cp437(), size);
    apply(&mut app, &world, Action::Inspector);
    assert!(app.inspector_open());
    app.fit(ui::areas(Size::new(80, 9)));
    assert!(!app.inspector_open() && !app.inspector_shut(), "no room");
    app.fit(ui::areas(size));
    assert!(app.inspector_shut(), "so Esc isn't spent closing it unseen");
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
    let mut app = sized_app(&world, Theme::cp437(), size);
    apply(&mut app, &world, Action::Inspector);
    assert_eq!(app.refusal(), Some("No room for the inspector"));
    assert!(!app.full_screen());
    assert_eq!(app.help_page_here().label(), "World", "still the map's");
}

#[test]
fn short_of_room_the_top_bar_keeps_the_time_and_the_help_key() {
    let world = field_world();
    let app = sized_app(&world, Theme::cp437(), Size::new(40, 12));
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
    let mut tall = sized_app(&world, Theme::cp437(), Size::new(100, 200));
    select(&mut tall, &world, id);
    open(&mut tall, &world, Tab::Genome);
    let (_, docked) = inspector_on(&tall, &world, 200);
    let docked: Vec<String> = docked.into_iter().filter(|line| !line.is_empty()).collect();

    let size = Size::new(140, 40);
    let mut app = sized_app(&world, Theme::cp437(), size);
    select(&mut app, &world, id);
    open(&mut app, &world, Tab::Genome);
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
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
    assert!(!app.full_screen(), "i again takes it back");
    apply(&mut app, &world, Action::Inspector);
    apply(&mut app, &world, Action::Back);
    assert!(!app.full_screen(), "and so does Esc");
    assert_eq!(app.screen(), Screen::Normal, "and doesn't ask to quit");
}

#[test]
fn filling_the_screen_the_inspector_takes_its_own_keys_and_time_s_but_not_the_map_s() {
    let world = field_world();
    let selected = sprite_on(&world, at(5, 5));
    // Each key from a fresh status line, with the Cursor on the other
    // sprite, which `Q`, `F` and `H` would reach.
    let full_screen = || {
        let mut app = sized_app(&world, Theme::cp437(), Size::new(100, 30));
        select(&mut app, &world, selected);
        point(&mut app, &world, at(30, 8));
        apply(&mut app, &world, Action::Inspector);
        app
    };
    let before = full_screen();
    for action in [
        Action::Mode(CursorMode::Grab),
        Action::Scroll { dx: 3, dy: 0 },
        Action::Zoom,
        Action::Track,
        Action::CycleColours,
        Action::CycleEventFilter,
        Action::Press {
            button: Button::Left,
            amplified: false,
        },
        Action::Follow { at: None },
        Action::ToggleVisible,
    ] {
        let mut app = full_screen();
        assert_eq!(app.refusal(), None);
        apply(&mut app, &world, action);
        assert_eq!(app.refusal(), Some(MAP_HIDDEN), "{action:?} says why");
        assert_eq!(app.mode(), CursorMode::Select, "{action:?}");
        assert_eq!(app.viewport(), before.viewport(), "{action:?}");
        assert_eq!(app.tile_width(), 1, "{action:?}");
        assert!(!app.tracking(), "{action:?}");
        assert_eq!(app.colour_mode(), before.colour_mode(), "{action:?}");
        assert_eq!(app.event_filter(), before.event_filter(), "{action:?}");
        assert_eq!(
            app.selection(),
            Some(Selection::Living(selected)),
            "{action:?}"
        );
        assert_eq!(app.followed(), None, "{action:?}");
        assert!(!app.visible(), "{action:?}");
        assert!(app.full_screen(), "{action:?}");
    }
    let mut app = full_screen();
    apply(&mut app, &world, Action::NextTab);
    assert_eq!(app.tab(), Tab::Brain);
    apply(&mut app, &world, Action::TogglePause);
    assert!(app.clock.is_paused());
    assert_eq!(app.refusal(), None);
    assert!(app.full_screen());
}

#[test]
fn filling_the_screen_the_palette_mutes_the_map_s_keys() {
    let world = field_world();
    let mut app = sized_app(&world, Theme::cp437(), Size::new(100, 30));
    let pressable = |app: &App, action: Action| {
        app.palette_rows()
            .iter()
            .find(|row| row.key.action == Some(action))
            .is_some_and(|row| controls::pressable(&row.key, app))
    };
    apply(&mut app, &world, Action::Palette);
    assert!(pressable(&app, Action::Zoom), "beside the map");
    apply(&mut app, &world, Action::Back);
    apply(&mut app, &world, Action::Inspector);
    apply(&mut app, &world, Action::Palette);
    for action in [Action::Zoom, Action::Track, Action::CycleEventFilter] {
        assert!(!pressable(&app, action), "{action:?}");
    }
    assert!(pressable(&app, Action::SpriteList), "the rest");
}

#[test]
fn shrunk_to_no_room_the_inspector_no_longer_fills_the_screen() {
    let world = field_world();
    let size = Size::new(100, 30);
    let mut app = sized_app(&world, Theme::cp437(), size);
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
    app.fit(ui::areas(Size::new(40, 20)));
    assert!(!app.full_screen(), "no room");
    let screen = render_on(&app, &world, Size::new(40, 20));
    assert!(shows_grass(&screen), "the map alone");
    app.fit(ui::areas(size));
    assert!(!app.full_screen(), "and grown again, the map");
}

#[test]
fn filling_the_screen_the_rest_of_the_keys_work_and_what_they_open_goes_back_to_it() {
    let world = field_world();
    let size = Size::new(100, 30);
    let mut app = sized_app(&world, Theme::cp437(), size);
    let (genomes, saves) = (scratch_folder("full-genome"), scratch_folder("full-save"));
    app.set_genome_folder(genomes.clone());
    app.set_save_folder(saves.clone());
    select(&mut app, &world, sprite_on(&world, at(5, 5)));
    open(&mut app, &world, Tab::Genome);
    apply(&mut app, &world, Action::Inspector);

    // `g` saves the genome it shows, and `F5` the world.
    apply(&mut app, &world, Action::ExportGenome);
    let notice = app.notice().expect("it says where");
    assert!(notice.starts_with("Saved Sprite #"), "{notice}");
    assert_eq!(std::fs::read_dir(&genomes).unwrap().count(), 1);
    apply(&mut app, &world, Action::Quicksave);
    assert_eq!(app.notice(), Some("Saved as quicksave"));

    // What a key opens is drawn over it, and goes back to it on closing.
    for (opens, open) in [
        (Action::Rename, Screen::Naming),
        (Action::SaveAs, Screen::SaveNaming),
        (Action::SpriteList, Screen::SpriteList),
        (Action::Population, Screen::Population),
        (Action::FamilyTree, Screen::FamilyTree),
        (Action::Palette, Screen::Palette),
        (Action::OpenSaves, Screen::LoadMenu),
        (Action::Help, Screen::Help),
    ] {
        apply(&mut app, &world, opens);
        assert_eq!(app.screen(), open, "{opens:?}");
        assert!(app.full_screen(), "{opens:?}: still beneath it");
        let screen = render_on(&app, &world, size);
        assert!(!shows_grass(&screen), "{opens:?}: no map beside it");
        apply(&mut app, &world, Action::Back);
        assert_eq!(app.screen(), Screen::Normal, "{opens:?} closes");
        assert!(app.full_screen(), "{opens:?}: back to the inspector");
        let screen = render_on(&app, &world, size);
        assert!(
            row(&screen, 1).contains("Genome"),
            "{opens:?}: its title row"
        );
    }
}

/// Whether `screen` shows any of the map's grass, which CP437 draws as a
/// green `.`.
fn shows_grass(screen: &Buffer) -> bool {
    screen
        .content
        .iter()
        .any(|cell| cell.symbol() == "." && cell.fg == Color::Green)
}

#[test]
fn a_load_keeps_square_tiles_and_the_inspector_open_over_the_map_and_the_view_as_saved() {
    let world = field_world();
    let size = Size::new(80, 24);
    let mut app = sized_app(&world, Theme::cp437(), size);
    let folder = scratch_folder("zoomed-load");
    app.set_save_folder(folder.clone());
    apply(&mut app, &world, Action::Zoom);
    apply(&mut app, &world, Action::Inspector);
    assert!(app.inspector_open());
    apply(&mut app, &world, Action::Scroll { dx: 21, dy: 7 });
    let (viewport, inspector) = (app.viewport(), app.inspector_area().expect("open"));
    apply(&mut app, &world, Action::Quicksave);
    let saved = std::fs::read(saves::path_for(&folder, QUICKSAVE)).expect("the quicksave");
    let middle = World::load(&saved)
        .expect("it loads")
        .view()
        .expect("its view");
    // Its middle is the middle of the tiles beside the inspector.
    let cell = app.cell_of(middle).expect("in view");
    let tiles = ui::areas(size).tiles;
    assert!(cell.x < inspector.x, "{cell:?} {inspector:?}");
    assert!(
        (inspector.x - tiles.x).abs_diff(2 * (cell.x - tiles.x)) <= 3,
        "{cell:?} {inspector:?}"
    );

    // Loaded from the full screen, it goes back to the map.
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
    apply(&mut app, &world, Action::Quickload);
    let loaded = app.take_loaded().expect("the quicksave loaded");
    assert_eq!(loaded.view(), Some(middle));
    assert_eq!(app.tile_width(), 2, "still square tiles");
    assert!(app.inspector_open(), "still open over the map");
    assert!(!app.full_screen(), "the map, to see the world loaded");
    assert_eq!(app.viewport(), viewport, "the view as it was saved");
}

#[test]
fn on_a_narrow_screen_i_opens_the_inspector_over_the_map_then_fills_the_screen() {
    let world = field_world();
    let mut app = sized_app(&world, Theme::cp437(), Size::new(80, 24));
    apply(&mut app, &world, Action::Inspector);
    assert!(app.inspector_open() && !app.full_screen());
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
    apply(&mut app, &world, Action::Back);
    assert!(app.inspector_open(), "back to it open over the map");
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
        let mut app = sized_app(&world, Theme::cp437(), size);
        let row = ui::areas(size).tiles.y + 1;
        assert_eq!(tiles_across(&app, row).len(), across, "{size:?}");
        apply(&mut app, &world, Action::Zoom);
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
    let mut app = sized_app(&world, Theme::cp437(), size);
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
    // An arrow over each of its cells, and under each.
    for x in [centre.x, centre.x + 1] {
        assert_eq!(screen[(x, centre.y - 1)].symbol(), arrows.down.to_string());
        assert_eq!(screen[(x, centre.y + 1)].symbol(), arrows.up.to_string());
    }
    // So the frame of light has two `═` above and two below while sprites
    // can see it.
    apply(&mut app, &world, Action::ToggleVisible);
    let screen = render(&app, &world);
    let frame = app.theme.visible_frame();
    for x in [centre.x, centre.x + 1] {
        assert_eq!(screen[(x, centre.y - 1)].symbol(), frame.down.to_string());
        assert_eq!(screen[(x, centre.y + 1)].symbol(), frame.up.to_string());
    }
    assert_eq!(
        screen[(centre.x + 2, centre.y)].symbol(),
        frame.left.to_string()
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
    let mut app = sized_app(&world, Theme::cp437(), size);
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
    assert!(app.full_screen());
}
