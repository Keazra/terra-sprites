//! The Cursor on screen (design v21 §6.5): its modes, Train's pets and zaps,
//! and locking on.

use ratatui::layout::{Position, Rect};
use terra_sim::{Command, DataPack, EntityId, Map, Pos, Scenario, World};
use terra_tui::app::{App, Areas, CursorMode, Flow, Screen};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

/// An all-grass world, 10×6, with starter sprites on `sprites`.
fn field(sprites: &[Pos]) -> World {
    let rows = vec![".........."; 6];
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

/// The app, with the map view's tiles drawn from screen cell (0, 0), so a
/// tile's cell is its position.
fn app(world: &World) -> App {
    let areas = Areas {
        tiles: Rect::new(0, 0, 10, 6),
        inspector: None,
    };
    App::new(world.map(), Theme::cp437(), 1, areas)
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

fn click(app: &mut App, world: &World, tile: Pos, button: Button) {
    let at = Position::new(tile.x, tile.y);
    let action = Action::Click {
        at,
        button,
        amplified: false,
    };
    apply(app, world, action);
}

/// The ID of the sprite on `pos`.
fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite").id()
}

fn pet(sprite: EntityId, reach_back: u16) -> Command {
    Command::Reward {
        sprite,
        amplified: false,
        reach_back,
    }
}

fn zap(sprite: EntityId) -> Command {
    Command::Correct {
        sprite,
        amplified: false,
    }
}

#[test]
fn z_and_x_pick_select_and_train_and_escape_goes_back_to_select() {
    let world = field(&[]);
    let mut app = app(&world);
    assert_eq!(app.mode(), CursorMode::Select);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    assert_eq!(app.mode(), CursorMode::Train);
    apply(&mut app, &world, Action::Back);
    assert_eq!(
        (app.mode(), app.screen()),
        (CursorMode::Select, Screen::Normal)
    );
    apply(&mut app, &world, Action::Back);
    assert_eq!(
        app.screen(),
        Screen::QuitPrompt,
        "from Select, Esc asks to quit"
    );
}

#[test]
fn in_train_mode_a_left_click_pets_the_sprite_under_it_and_a_right_click_zaps_it() {
    // Design v21 §6.5. At 1×, two seconds is 2.5 ticks: the reach back is 3.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    click(&mut app, &world, at(4, 2), Button::Right);
    assert_eq!(app.take_commands(), [pet(id, 3), zap(id)]);
    assert_eq!(app.take_commands(), [], "taken once");
}

#[test]
fn an_amplified_click_is_a_hug_or_a_shock() {
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    for button in [Button::Left, Button::Right] {
        let at = Position::new(4, 2);
        let amplified = true;
        apply(
            &mut app,
            &world,
            Action::Click {
                at,
                button,
                amplified,
            },
        );
    }
    let hug = Command::Reward {
        sprite: id,
        amplified: true,
        reach_back: 3,
    };
    let shock = Command::Correct {
        sprite: id,
        amplified: true,
    };
    assert_eq!(app.take_commands(), [hug, shock]);
}

#[test]
fn a_train_click_with_no_sprite_to_act_on_sends_nothing() {
    let world = field(&[at(4, 2)]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(1, 1), Button::Left);
    click(&mut app, &world, at(1, 1), Button::Right);
    assert_eq!(app.take_commands(), []);
}

#[test]
fn in_select_mode_clicks_send_nothing() {
    let world = field(&[at(4, 2)]);
    let mut app = app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.take_commands(), []);
}

#[test]
fn a_pet_reaches_back_two_seconds_of_the_player_s_time() {
    // Design v21 §6.5: 1.25 ticks a second at 1×, so 5 at 2×, 10 at 4×, 20
    // at 8× and 40 at 16×. While paused, the speed time resumes at.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    let mut reach = Vec::new();
    for _ in 0..4 {
        apply(&mut app, &world, Action::Faster { held: false });
        click(&mut app, &world, at(4, 2), Button::Left);
        let [Command::Reward { reach_back, .. }] = app.take_commands()[..] else {
            panic!("one pet");
        };
        reach.push(reach_back);
    }
    assert_eq!(reach, [5, 10, 20, 40]);
    apply(&mut app, &world, Action::TogglePause);
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.take_commands(), [pet(id, 40)]);
}
