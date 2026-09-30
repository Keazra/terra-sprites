//! Grab mode on screen (design v23 §6.5): a click grabs what's under the
//! Cursor, taking hold of a sprite or picking up an item, and the next lets
//! go or puts it down.

use ratatui::layout::{Position, Rect};
use terra_sim::{Command, DataPack, EntityId, Grip, Map, Pos, Scenario, ScriptedAction, World};
use terra_tui::app::{App, Areas, CursorMode, Flow, StatusMark};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// An all-grass world, 10×6, with `objects`, and starter sprites on
/// `sprites`.
fn field(objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    let rows = vec![".........."; 6];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let sprites: Vec<(Pos, _)> = sprites.iter().map(|&pos| (pos, None)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, pack(), 1).expect("valid scenario")
}

/// The app in Grab mode, with the map view's tiles drawn from screen cell
/// (0, 0), so a tile's cell is its position.
fn grab_app(world: &World) -> App {
    let areas = Areas {
        tiles: Rect::new(0, 0, 10, 6),
        inspector: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), 1, areas);
    apply(&mut app, world, Action::Mode(CursorMode::Grab));
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

fn click(app: &mut App, world: &World, tile: Pos) {
    let action = Action::Click {
        at: Position::new(tile.x, tile.y),
        button: Button::Left,
        amplified: false,
    };
    apply(app, world, action);
}

/// Hands the app's commands to the world and runs a tick, as a frame does.
fn tick(app: &mut App, world: &mut World) {
    for command in app.take_commands() {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, world);
}

fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite").id()
}

fn object_on(world: &World, pos: Pos) -> EntityId {
    world.object_at(pos).expect("an object").id()
}

#[test]
fn a_grab_click_on_a_sprite_takes_hold_of_it() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    let sprite = sprite_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::TakeHold { sprite }]);
}

#[test]
fn a_grab_click_on_an_item_picks_it_up() {
    let world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    let item = object_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::PickUp { item }]);
}

#[test]
fn a_sprite_is_grabbed_before_the_item_it_stands_on() {
    let world = field(&[(at(4, 2), "berry")], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    let sprite = sprite_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::TakeHold { sprite }]);
}

#[test]
fn a_grab_click_on_empty_ground_sends_nothing_and_says_so() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    assert_eq!(app.take_commands(), Vec::new());
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(app.refusal(), Some("Nothing here to grab"));
}

#[test]
fn a_grab_click_on_a_bush_sends_nothing_and_says_it_is_rooted() {
    let world = field(&[(at(4, 2), "berry_bush")], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    assert_eq!(app.take_commands(), Vec::new());
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(
        app.refusal(),
        Some("Can't grab the berry bush: it's rooted to the ground")
    );
}

#[test]
fn leading_a_sprite_a_grab_click_lets_go() {
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    tick(&mut app, &mut world);
    click(&mut app, &world, at(7, 4));
    assert_eq!(app.take_commands(), vec![Command::LetGo]);
}

#[test]
fn holding_an_item_a_grab_click_puts_it_down_there() {
    let mut world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2));
    tick(&mut app, &mut world);
    click(&mut app, &world, at(7, 4));
    assert_eq!(
        app.take_commands(),
        vec![Command::PutDown { tile: at(7, 4) }]
    );
}

#[test]
fn while_paused_a_click_after_a_queued_grab_lets_go_and_both_apply_next_tick() {
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(4, 2));
    click(&mut app, &world, at(4, 2));
    click(&mut app, &world, at(6, 2));
    assert_eq!(
        app.take_commands(),
        vec![Command::TakeHold { sprite }, Command::LetGo]
    );
    assert_eq!(app.grip(&world), None, "as the queue leaves it");
    world.submit(Command::TakeHold { sprite });
    world.submit(Command::LetGo);
    let events = world.step();
    app.record(&events, &world);
    assert_eq!(world.cursor().leads(), None);
    assert_eq!(app.grip(&world), None);
}

#[test]
fn a_queued_grab_the_world_refuses_leaves_the_cursor_as_the_world_says() {
    let mut world = field(&[], &[at(4, 2), at(8, 4)]);
    let mut app = grab_app(&world);
    let (mine, other) = (sprite_on(&world, at(4, 2)), sprite_on(&world, at(8, 4)));
    click(&mut app, &world, at(4, 2));
    assert_eq!(app.grip(&world), Some(Grip::Leads(mine)), "as queued");
    // Another hand gets there first, so the world refuses the app's grab.
    world.submit(Command::TakeHold { sprite: other });
    tick(&mut app, &mut world);
    assert_eq!(app.grip(&world), Some(Grip::Leads(other)));
    assert_eq!(app.status_mark(), StatusMark::Rejected);
}

fn point(app: &mut App, world: &World, tile: Pos) {
    apply(app, world, Action::Point(Position::new(tile.x, tile.y)));
}

#[test]
fn while_leading_each_move_of_the_cursor_onto_a_new_tile_is_sent_once() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(4, 2));
    click(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(6, 2));
    point(&mut app, &world, at(6, 2));
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    point(&mut app, &world, at(7, 3));
    assert_eq!(
        app.take_commands(),
        vec![
            Command::TakeHold { sprite },
            Command::MoveCursor { tile: at(6, 2) },
            Command::MoveCursor { tile: at(7, 3) },
        ],
        "once a tile, in every mode"
    );
}

#[test]
fn not_leading_the_cursor_moves_without_telling_the_world() {
    let world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    point(&mut app, &world, at(6, 2));
    click(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(7, 3));
    let item = object_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::PickUp { item }]);
}

fn press(app: &mut App, world: &World) {
    let action = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(app, world, action);
}

/// Selects the sprite on `tile` and locks the Cursor on to it, in Select
/// mode, then goes back to Grab mode.
fn lock_on(app: &mut App, world: &World, tile: Pos) {
    apply(app, world, Action::Mode(CursorMode::Select));
    let action = Action::Click {
        at: Position::new(tile.x, tile.y),
        button: Button::Right,
        amplified: false,
    };
    apply(app, world, action);
    apply(app, world, Action::Mode(CursorMode::Grab));
}

#[test]
fn leading_the_locked_on_sprite_the_cursor_follows_the_pointer_and_the_lock_comes_back_after() {
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(4, 2));
    lock_on(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(0, 0));
    press(&mut app, &world);
    let commands = app.take_commands();
    assert_eq!(commands, vec![Command::TakeHold { sprite }]);
    for command in commands {
        world.submit(command);
    }
    tick(&mut app, &mut world);
    point(&mut app, &world, at(8, 5));
    assert_eq!(
        (app.cursor(), app.locked()),
        (at(8, 5), None),
        "the lock waits"
    );
    press(&mut app, &world);
    tick(&mut app, &mut world);
    let there = world.sprite(sprite).expect("the sprite").pos();
    assert_eq!(
        (app.cursor(), app.locked()),
        (there, Some(sprite)),
        "back on it"
    );
}

#[test]
fn holding_an_item_in_grab_mode_the_cursor_follows_the_pointer_though_locked_on() {
    let mut world = field(&[(at(1, 1), "ball")], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(1, 1));
    tick(&mut app, &mut world);
    lock_on(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(8, 5));
    assert_eq!(
        (app.cursor(), app.locked()),
        (at(8, 5), None),
        "the lock waits"
    );
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    assert_eq!(app.cursor(), at(4, 2), "in another mode, the lock holds");
}

#[test]
fn while_leading_a_cursor_locked_on_to_another_walking_sprite_tells_the_world_where_it_goes() {
    let rows = vec![".........."; 6];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let walker = at(5, 3);
    let wander = ScriptedAction::Wander {
        destination: at(9, 3),
    };
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(at(1, 1), None), (walker, None)],
        scripted: &[(walker, wander)],
    };
    let mut world = World::from_scenario(scenario, pack(), 1).expect("valid scenario");
    let mut app = grab_app(&world);
    let id = sprite_on(&world, walker);
    click(&mut app, &world, at(1, 1));
    lock_on(&mut app, &world, walker);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    for _ in 0..6 {
        tick(&mut app, &mut world);
        if world.sprite(id).expect("the walker").pos() != walker {
            break;
        }
    }
    let walked_to = world.sprite(id).expect("the walker").pos();
    assert_ne!(walked_to, walker, "it walked");
    assert_eq!(app.cursor(), walked_to);
    assert_eq!(
        app.take_commands(),
        vec![Command::MoveCursor { tile: walked_to }]
    );
}

#[test]
fn the_observed_list_tells_of_being_pulled_away_and_along_as_the_sprite_felt_it() {
    // Design v23 §6.1: the Cursor is invisible, so it comes out of nowhere.
    let rows = vec![".........."; 6];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let start = at(1, 1);
    let wander = ScriptedAction::Wander {
        destination: at(9, 1),
    };
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(start, None)],
        scripted: &[(start, wander)],
    };
    let mut world = World::from_scenario(scenario, pack(), 1).expect("valid scenario");
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::SelectNext);
    tick(&mut app, &mut world);
    let now = world.sprites().next().expect("the sprite").pos();
    click(&mut app, &world, now);
    tick(&mut app, &mut world);
    click(&mut app, &world, at(5, 5));
    tick(&mut app, &mut world);
    let observed: Vec<&str> = app.observed().map(|o| o.line.as_str()).collect();
    assert_eq!(
        observed,
        [
            "Was pulled along out of nowhere",
            "Wandered off, but was pulled away",
        ]
    );
}
