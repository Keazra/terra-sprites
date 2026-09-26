use ratatui::layout::{Position, Rect};
use terra_sim::{
    ActionView, DataPack, DeathCause, EntityId, Event, EventKind, Hurt, Map, Outcome, Pos,
    Progress, Scenario, Verb, World,
};
use terra_tui::app::{App, Areas, Flow, Screen, Selection, Tab};
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::theme::Theme;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

/// An all-grass world, with starter sprites on `sprites`.
fn grass_with(width: usize, height: usize, sprites: &[Pos]) -> World {
    let row = ".".repeat(width);
    let rows = vec![row.as_str(); height];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let sprites: Vec<(Pos, _)> = sprites.iter().map(|&pos| (pos, None)).collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, pack(), 1).expect("valid scenario")
}

/// An all-grass world with no sprites.
fn grass(width: usize, height: usize) -> World {
    grass_with(width, height, &[])
}

/// Where the map view draws its tiles on screen: `width`×`height` tiles from cell (1, 2).
fn tile_area(width: u16, height: u16) -> Rect {
    Rect::new(1, 2, width, height)
}

fn app(world: &World, tile_area: Rect) -> App {
    App::new(world.map(), Theme::cp437(), 1, no_inspector(tile_area))
}

/// Panels with the map view's tiles on `tiles`, and no room for the inspector.
fn no_inspector(tiles: Rect) -> Areas {
    Areas {
        tiles,
        inspector: None,
    }
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn scroll(app: &mut App, world: &World, dx: i32, dy: i32) {
    assert_eq!(app.apply(Action::Scroll { dx, dy }, world), Flow::Continue);
}

fn point(app: &mut App, world: &World, column: u16, row: u16) {
    assert_eq!(
        app.apply(Action::Point(Position::new(column, row)), world),
        Flow::Continue
    );
}

fn click(app: &mut App, world: &World, column: u16, row: u16) {
    assert_eq!(
        app.apply(Action::Click(Position::new(column, row)), world),
        Flow::Continue
    );
}

/// The ID of the sprite on `pos`.
fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite").id()
}

#[test]
fn the_cursor_starts_at_the_centre_of_the_map() {
    assert_eq!(app(&grass(160, 96), tile_area(60, 20)).cursor(), at(80, 48));
    assert_eq!(app(&grass(5, 3), tile_area(5, 3)).cursor(), at(2, 1));
}

#[test]
fn the_viewport_starts_centred_on_the_cursor() {
    // The cursor starts at (80, 48); a 60×20 view centred on it starts at (50, 38).
    assert_eq!(
        app(&grass(160, 96), tile_area(60, 20)).viewport(),
        at(50, 38)
    );
}

#[test]
fn a_map_that_fits_in_the_view_is_shown_from_its_top_left_corner() {
    assert_eq!(app(&grass(20, 10), tile_area(20, 10)).viewport(), at(0, 0));
}

#[test]
fn scrolling_moves_the_viewport_and_stops_at_the_wall() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    assert_eq!(app.viewport(), at(70, 43));
    scroll(&mut app, &world, 1, 0);
    assert_eq!(app.viewport(), at(71, 43));
    scroll(&mut app, &world, -5, 2);
    assert_eq!(app.viewport(), at(66, 45));
    scroll(&mut app, &world, -1000, -1000);
    assert_eq!(app.viewport(), at(0, 0));
    scroll(&mut app, &world, 1000, 1000);
    assert_eq!(app.viewport(), at(140, 86));
}

#[test]
fn pointing_at_a_tile_puts_the_cursor_on_it() {
    // The view shows tiles (70, 43) to (89, 52), drawn from screen cell (1, 2).
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    point(&mut app, &world, 1, 2);
    assert_eq!(app.cursor(), at(70, 43));
    point(&mut app, &world, 20, 11);
    assert_eq!(app.cursor(), at(89, 52));
}

#[test]
fn pointing_outside_the_map_view_leaves_the_cursor_on_its_last_tile() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    point(&mut app, &world, 5, 5);
    assert_eq!(app.cursor(), at(74, 46));
    for (column, row) in [(0, 5), (21, 5), (5, 1), (5, 12)] {
        point(&mut app, &world, column, row);
        assert_eq!(app.cursor(), at(74, 46), "pointer at ({column}, {row})");
    }
}

#[test]
fn scrolling_under_a_still_pointer_moves_the_cursor_with_the_map() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    point(&mut app, &world, 11, 7);
    assert_eq!(app.cursor(), at(80, 48));
    scroll(&mut app, &world, 3, -1);
    assert_eq!(app.cursor(), at(83, 47));
}

#[test]
fn with_the_pointer_off_the_map_scrolling_leaves_the_cursor_on_its_tile() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    scroll(&mut app, &world, 3, 0); // no pointer yet
    assert_eq!(app.cursor(), at(80, 48));
    point(&mut app, &world, 11, 7); // over (83, 48) now
    point(&mut app, &world, 0, 7); // the pointer leaves the map view
    scroll(&mut app, &world, 3, 0);
    assert_eq!(app.cursor(), at(83, 48));
}

#[test]
fn a_click_puts_the_cursor_on_a_tile_without_scrolling() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    assert_eq!(
        app.apply(Action::Click(Position::new(1, 2)), &world),
        Flow::Continue
    );
    assert_eq!(app.cursor(), at(70, 43));
    assert_eq!(app.viewport(), at(70, 43));
}

#[test]
fn a_bigger_view_after_a_resize_still_stops_at_the_wall() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    scroll(&mut app, &world, 1000, 1000);
    assert_eq!(app.viewport(), at(140, 86));
    app.fit(no_inspector(tile_area(40, 20)));
    assert_eq!(app.viewport(), at(120, 76));
}

#[test]
fn time_actions_reach_the_clock() {
    let world = grass(40, 30);
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::Faster { held: false }, &world);
    assert_eq!(app.clock.speed(), Speed::X2);
    app.apply(Action::TogglePause, &world);
    assert!(app.clock.is_paused());
}

#[test]
fn escape_asks_to_quit_and_y_quits() {
    let world = grass(40, 30);
    let mut app = app(&world, tile_area(20, 10));
    assert_eq!(app.apply(Action::Back, &world), Flow::Continue);
    assert_eq!(app.screen(), Screen::QuitPrompt);
    assert_eq!(app.apply(Action::Confirm, &world), Flow::Quit);
}

#[test]
fn a_second_escape_quits() {
    let world = grass(40, 30);
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::Back, &world);
    assert_eq!(app.apply(Action::Back, &world), Flow::Quit);
}

#[test]
fn any_other_key_cancels_the_quit_prompt_and_does_nothing_else() {
    let world = grass(160, 96);
    let mut app = app(&world, tile_area(20, 10));
    for key in [
        Action::Dismiss,
        Action::TogglePause,
        Action::Scroll { dx: 1, dy: 0 },
    ] {
        app.apply(Action::Back, &world);
        assert_eq!(app.apply(key, &world), Flow::Continue, "{key:?}");
        assert_eq!(app.screen(), Screen::Normal, "{key:?} should cancel");
    }
    assert!(!app.clock.is_paused(), "space only cancelled the prompt");
    assert_eq!(
        app.viewport(),
        at(70, 43),
        "the scroll key only cancelled the prompt"
    );
    assert_eq!(
        app.apply(Action::Confirm, &world),
        Flow::Continue,
        "y with no prompt open"
    );
}

#[test]
fn moving_the_mouse_leaves_the_quit_prompt_open() {
    let world = grass(40, 30);
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::Back, &world);
    point(&mut app, &world, 3, 3);
    assert_eq!(app.screen(), Screen::QuitPrompt);
}

#[test]
fn ctrl_c_quits_at_once() {
    let world = grass(40, 30);
    let mut app = app(&world, tile_area(20, 10));
    assert_eq!(app.apply(Action::Quit, &world), Flow::Quit);
}

#[test]
fn clicking_a_sprite_selects_it_and_clicking_a_tile_with_none_clears_the_selection() {
    // The whole map fits in the view, so tile (x, y) is drawn at cell (1 + x, 2 + y).
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2)]);
    let mut app = app(&world, tile_area(20, 10));
    assert_eq!(app.selection(), None, "nothing is selected at first");
    click(&mut app, &world, 1 + 6, 2 + 2);
    assert_eq!(
        app.selection(),
        Some(Selection::Living(sprite_on(&world, at(6, 2))))
    );
    click(&mut app, &world, 1 + 3, 2 + 4);
    assert_eq!(
        app.selection(),
        Some(Selection::Living(sprite_on(&world, at(3, 4))))
    );
    click(&mut app, &world, 1 + 5, 2 + 5);
    assert_eq!(app.selection(), None);
}

#[test]
fn moving_the_pointer_over_a_sprite_does_not_select_it() {
    let world = grass_with(20, 10, &[at(6, 2)]);
    let mut app = app(&world, tile_area(20, 10));
    point(&mut app, &world, 1 + 6, 2 + 2);
    assert_eq!(app.selection(), None);
}

#[test]
fn tab_selects_sprites_in_id_order_and_wraps_around() {
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2), at(9, 9)]);
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    let mut app = app(&world, tile_area(20, 10));
    let mut tab = |action| {
        app.apply(action, &world);
        app.selection()
    };
    let living = |i: usize| Some(Selection::Living(ids[i]));
    assert_eq!(
        tab(Action::SelectNext),
        living(0),
        "from nothing, the lowest ID"
    );
    assert_eq!(tab(Action::SelectNext), living(1));
    assert_eq!(tab(Action::SelectNext), living(2));
    assert_eq!(tab(Action::SelectNext), living(0), "wrapping around");
    assert_eq!(tab(Action::SelectPrevious), living(2), "wrapping back");
    assert_eq!(tab(Action::SelectPrevious), living(1));
}

#[test]
fn shift_tab_with_nothing_selected_starts_from_the_highest_id() {
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2)]);
    let last = world.sprites().last().expect("sprites").id();
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectPrevious, &world);
    assert_eq!(app.selection(), Some(Selection::Living(last)));
}

#[test]
fn tab_with_no_sprites_selects_nothing() {
    let world = grass(20, 10);
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    app.apply(Action::SelectPrevious, &world);
    assert_eq!(app.selection(), None);
}

#[test]
fn tab_centres_the_view_on_a_sprite_out_of_view_and_leaves_it_alone_otherwise() {
    // The 20×10 view starts at (70, 43), showing tiles (70, 43) to (89, 52).
    let world = grass_with(160, 96, &[at(75, 45), at(150, 5)]);
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.viewport(), at(70, 43), "(75, 45) is in view");
    app.apply(Action::SelectNext, &world);
    // Centred on (150, 5): x from 140, and y from 0, stopped at the wall.
    assert_eq!(app.viewport(), at(140, 0));
}

#[test]
fn the_inspector_starts_on_the_world_tab_and_the_brackets_go_round_the_tabs() {
    let world = grass(20, 10);
    let mut app = app(&world, tile_area(20, 10));
    assert_eq!(app.tab(), Tab::World);
    let mut next = |action| {
        app.apply(action, &world);
        app.tab()
    };
    assert_eq!(next(Action::NextTab), Tab::Body, "wrapping around");
    assert_eq!(next(Action::NextTab), Tab::Brain);
    assert_eq!(next(Action::NextTab), Tab::Chem);
    assert_eq!(next(Action::NextTab), Tab::Genome);
    assert_eq!(next(Action::NextTab), Tab::World);
    assert_eq!(next(Action::PreviousTab), Tab::Genome);
    assert_eq!(next(Action::PreviousTab), Tab::Chem);
}

#[test]
fn selecting_a_sprite_from_the_world_tab_opens_body_and_from_another_tab_stays() {
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2)]);
    let mut app = app(&world, tile_area(20, 10));
    click(&mut app, &world, 1 + 3, 2 + 4);
    assert_eq!(app.tab(), Tab::Body, "by a click");

    let mut app = self::app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.tab(), Tab::Body, "by Tab");
    app.apply(Action::NextTab, &world);
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.tab(), Tab::Brain, "already on a sprite tab");
    click(&mut app, &world, 1 + 3, 2 + 4);
    assert_eq!(app.tab(), Tab::Brain);
}

#[test]
fn clearing_the_selection_leaves_the_tab_open() {
    let world = grass_with(20, 10, &[at(3, 4)]);
    let mut app = app(&world, tile_area(20, 10));
    click(&mut app, &world, 1 + 3, 2 + 4);
    click(&mut app, &world, 1 + 5, 2 + 5);
    assert_eq!((app.selection(), app.tab()), (None, Tab::Body));
}

fn died(id: EntityId, cause: DeathCause, age: u64) -> Event {
    Event {
        tick: age,
        kind: EventKind::Died { id, cause, age },
    }
}

#[test]
fn the_selection_remembers_how_the_selected_sprite_died() {
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2)]);
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    app.record(&[died(ids[1], DeathCause::Starvation, 90)], &world);
    assert_eq!(
        app.selection(),
        Some(Selection::Living(ids[0])),
        "another sprite's death"
    );
    app.record(&[died(ids[0], DeathCause::Dehydration, 4_012)], &world);
    assert_eq!(
        app.selection(),
        Some(Selection::Dead {
            id: ids[0],
            cause: DeathCause::Dehydration,
            age: 4_012
        })
    );
}

#[test]
fn after_the_selected_sprite_dies_tab_carries_on_from_its_id() {
    // The world here still has the dead sprite, as a real one wouldn't, so
    // Tab stepping past it shows it goes by ID.
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2), at(9, 9)]);
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    app.apply(Action::SelectNext, &world);
    app.record(&[died(ids[1], DeathCause::OldAge, 70_000)], &world);
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.selection(), Some(Selection::Living(ids[2])));

    app.record(&[died(ids[2], DeathCause::OldAge, 70_000)], &world);
    app.apply(Action::SelectPrevious, &world);
    assert_eq!(app.selection(), Some(Selection::Living(ids[1])));
}

#[test]
fn the_detail_view_starts_off_and_toggles_whatever_is_selected() {
    let world = grass_with(20, 10, &[at(3, 4)]);
    let mut app = app(&world, tile_area(20, 10));
    assert!(!app.detail());
    app.apply(Action::ToggleDetail, &world);
    assert!(app.detail());
    app.apply(Action::SelectNext, &world);
    assert!(app.detail(), "a new selection keeps it");
    app.apply(Action::ToggleDetail, &world);
    assert!(!app.detail());
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

/// The observed list, newest first, as `(line, count, tick)`.
fn observed(app: &App) -> Vec<(&str, u32, u64)> {
    app.observed()
        .map(|o| (o.line.as_str(), o.count, o.tick))
        .collect()
}

#[test]
fn the_observed_list_keeps_what_the_selected_sprite_finished_newest_first() {
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2)]);
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    let mut app = app(&world, tile_area(20, 10));
    app.record(&[finished(1, ids[0], Verb::Rest, Outcome::Applied)], &world);
    assert_eq!(observed(&app), [], "nothing is watched before a selection");
    app.apply(Action::SelectNext, &world);
    app.record(
        &[
            finished(5, ids[0], Verb::Wander, Outcome::Applied),
            finished(6, ids[1], Verb::Rest, Outcome::Applied),
            finished(9, ids[0], Verb::Wander, Outcome::Applied),
        ],
        &world,
    );
    app.record(
        &[
            finished(12, ids[0], Verb::Wander, Outcome::Applied),
            finished(20, ids[0], Verb::Rest, Outcome::Applied),
        ],
        &world,
    );
    assert_eq!(
        observed(&app),
        [("Rested", 1, 20), ("Wandered off", 3, 12)],
        "the same line in a row counts up, and keeps the latest tick"
    );
}

#[test]
fn selecting_another_sprite_starts_its_observed_list_afresh() {
    let world = grass_with(20, 10, &[at(3, 4), at(6, 2)]);
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    app.record(&[finished(5, ids[0], Verb::Rest, Outcome::Applied)], &world);
    // Clicking the selected sprite again keeps what was watched.
    click(&mut app, &world, 1 + 3, 2 + 4);
    assert_eq!(observed(&app).len(), 1);
    app.apply(Action::SelectNext, &world);
    assert_eq!(observed(&app), []);
    app.apply(Action::SelectPrevious, &world);
    assert_eq!(observed(&app), [], "coming back starts afresh too");
}

#[test]
fn the_observed_list_keeps_the_latest_500_lines() {
    let world = grass_with(20, 10, &[at(3, 4)]);
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app(&world, tile_area(20, 10));
    app.apply(Action::SelectNext, &world);
    for tick in 0..600 {
        // Alternating, so no two lines in a row read the same.
        let verb = if tick % 2 == 0 {
            Verb::Rest
        } else {
            Verb::Wander
        };
        app.record(&[finished(tick, id, verb, Outcome::Applied)], &world);
    }
    let list = observed(&app);
    assert_eq!(list.len(), 500);
    assert_eq!((list[0].2, list[499].2), (599, 100), "newest first");
}
