//! Grab mode on screen (design v23 §6.5): a click grabs what's under the
//! Cursor, taking hold of a sprite or picking up an item, and the next lets
//! go or puts it down.

use ratatui::layout::{Position, Rect};
use terra_sim::{Command, DataPack, EntityId, Map, Pos, Scenario, World};
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
