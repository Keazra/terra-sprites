//! Throwing and shoving on screen (design v25 §6.5): in Grab mode the right
//! button, or `E`, held down, aims what the Cursor has hold of by pulling
//! back, and letting go sends it.

use ratatui::layout::{Position, Rect};
use terra_sim::{Command, DataPack, Dir, EntityId, Map, Pos, Scenario, World};
use terra_tui::app::{App, Areas, CursorMode, Flow, StatusMark};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// An all-grass world, 20×12, with `objects`, and starter sprites on
/// `sprites`.
fn field(objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    let rows = vec!["...................."; 12];
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
        tiles: Rect::new(0, 0, 20, 12),
        inspector: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), 1, areas);
    apply(&mut app, world, Action::Mode(CursorMode::Grab));
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

fn cell(tile: Pos) -> Position {
    Position::new(tile.x, tile.y)
}

fn click(app: &mut App, world: &World, tile: Pos, button: Button) {
    let action = Action::Click {
        at: cell(tile),
        button,
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
fn a_right_click_with_nothing_held_or_led_sends_nothing_and_says_so() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Right);
    assert_eq!(app.take_commands(), Vec::new());
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(app.refusal(), Some("Nothing to throw or shove"));
}

/// Makes the Cursor hold the item on `tile`: a click, and a tick.
fn hold(app: &mut App, world: &mut World, tile: Pos) -> EntityId {
    let item = object_on(world, tile);
    click(app, world, tile, Button::Left);
    tick(app, world);
    assert!(world.cursor().holds().is_some());
    item
}

/// Points at `tile`, as a mouse move, or a drag, does.
fn point(app: &mut App, world: &World, tile: Pos) {
    apply(app, world, Action::Point(cell(tile)));
}

/// Lets go of the right button over `tile`.
fn let_go(app: &mut App, world: &World, tile: Pos) {
    let action = Action::Release {
        button: Button::Right,
        at: Some(cell(tile)),
    };
    apply(app, world, action);
}

#[test]
fn pulling_back_and_letting_go_throws_the_other_way_as_far_as_the_pull() {
    let mut world = field(&[(at(5, 5), "ball")], &[]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(3, 5));
    assert_eq!(
        app.cursor(),
        at(5, 5),
        "the Cursor stays where it was pressed"
    );
    let_go(&mut app, &world, at(3, 5));
    let throw = Command::Throw {
        from: at(5, 5),
        toward: Dir::E,
        tiles: 2,
    };
    assert_eq!(app.take_commands(), vec![throw]);
}

/// The commands the app's clicks sent, leaving out where the Cursor is,
/// which it tells the world while it leads.
fn sent(app: &mut App) -> Vec<Command> {
    let commands = app.take_commands().into_iter();
    commands
        .filter(|c| !matches!(c, Command::MoveCursor { .. }))
        .collect()
}

/// Makes the Cursor lead the sprite on `tile`: a click, and a tick.
fn lead(app: &mut App, world: &mut World, tile: Pos) -> EntityId {
    let sprite = sprite_on(world, tile);
    click(app, world, tile, Button::Left);
    tick(app, world);
    assert_eq!(world.cursor().leads(), Some(sprite));
    sprite
}

#[test]
fn a_shove_goes_the_nearest_of_the_8_directions_to_the_way_pulled() {
    // Pulled 3 east and 1 south, a shove goes west; 2 east and 1 south is
    // nearer north-west (design v25 §6.5).
    for (to, toward, tiles) in [(at(9, 6), Dir::W, 3), (at(8, 6), Dir::NW, 2)] {
        let mut world = field(&[], &[at(5, 5)]);
        let mut app = grab_app(&world);
        lead(&mut app, &mut world, at(5, 5));
        app.take_commands();
        click(&mut app, &world, at(6, 5), Button::Right);
        point(&mut app, &world, to);
        let_go(&mut app, &world, to);
        assert_eq!(
            sent(&mut app),
            vec![Command::Shove { toward, tiles }],
            "pulled to {to:?}"
        );
    }
}

#[test]
fn letting_go_where_aiming_began_sends_nothing_and_esc_cancels_in_grab_mode() {
    let mut world = field(&[(at(5, 5), "ball")], &[]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    let_go(&mut app, &world, at(5, 5));
    assert_eq!(app.take_commands(), Vec::new(), "no pull");

    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(3, 5));
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.mode(), CursorMode::Grab, "Esc cancels the aim first");
    assert_eq!(app.cursor(), at(5, 5));
    point(&mut app, &world, at(2, 5));
    assert_eq!(
        app.cursor(),
        at(2, 5),
        "the Cursor follows the pointer again"
    );
    let_go(&mut app, &world, at(2, 5));
    assert_eq!(app.take_commands(), Vec::new(), "cancelled");
}

#[test]
fn the_aim_is_cancelled_when_what_the_cursor_held_is_gone() {
    // A berry that lasts 3 ticks expires in the Cursor while the player aims.
    let text = DataPack::builtin_sources()
        .iter()
        .find(|(path, _)| *path == "objects.ron")
        .map(|(_, text)| {
            text.replace(
                "ticks: (1500, 2500), next: Expire",
                "ticks: (3, 3), next: Expire",
            )
        })
        .expect("objects.ron");
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| {
            (
                path,
                if path == "objects.ron" {
                    text.as_str()
                } else {
                    builtin
                },
            )
        })
        .collect();
    let data = DataPack::from_sources(&sources).expect("a valid pack");
    let map = Map::from_ascii(&["...................."; 12], &data).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[(at(5, 5), "berry")],
        sprites: &[],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(3, 5));
    for _ in 0..4 {
        tick(&mut app, &mut world);
    }
    assert!(world.cursor().holds().is_none(), "it expired");
    point(&mut app, &world, at(2, 5));
    assert_eq!(app.cursor(), at(2, 5), "no longer aiming");
    let_go(&mut app, &world, at(2, 5));
    assert_eq!(app.take_commands(), Vec::new());
}

#[test]
fn e_aims_from_the_cursor_and_letting_it_go_or_pressing_it_again_sends() {
    // A terminal that doesn't report a key let go sends a second press
    // instead (design v25 §6.5).
    let press = Action::Press {
        button: Button::Right,
        amplified: false,
    };
    let release = Action::Release {
        button: Button::Right,
        at: None,
    };
    for second in [release, press] {
        let mut world = field(&[(at(5, 5), "ball")], &[]);
        let mut app = grab_app(&world);
        hold(&mut app, &mut world, at(5, 5));
        apply(&mut app, &world, press);
        point(&mut app, &world, at(5, 8));
        apply(&mut app, &world, second);
        let throw = Command::Throw {
            from: at(5, 5),
            toward: Dir::N,
            tiles: 3,
        };
        assert_eq!(app.take_commands(), vec![throw], "{second:?}");
    }
}

#[test]
fn locked_on_to_a_sprite_a_throw_starts_at_its_feet() {
    // Design v25 §6.5: locked on, the Cursor sits on its sprite, wherever
    // the click lands.
    let mut world = field(&[(at(2, 2), "berry")], &[at(8, 5)]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(2, 2));
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    click(&mut app, &world, at(8, 5), Button::Right);
    assert!(app.locked().is_some());
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    click(&mut app, &world, at(12, 5), Button::Right);
    point(&mut app, &world, at(14, 5));
    let_go(&mut app, &world, at(14, 5));
    let throw = Command::Throw {
        from: at(8, 5),
        toward: Dir::W,
        tiles: 2,
    };
    assert_eq!(app.take_commands(), vec![throw]);
}
