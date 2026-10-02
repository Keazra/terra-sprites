//! The visible Cursor on screen (design v29 §6.5): `H` shows the Cursor to
//! sprites in the current cursor mode, or hides it, and while it's visible
//! every move onto a new tile tells the world.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Position, Rect};
use terra_sim::{Command, DataPack, Map, Pos, Scenario, World};
use terra_tui::app::{App, Areas, CursorMode, Flow};
use terra_tui::input::{Action, Keys};
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
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
/// tile's cell is its position, its first commands taken.
fn app(world: &World) -> App {
    let areas = Areas {
        tiles: Rect::new(0, 0, 10, 6),
        inspector: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), 1, areas);
    app.take_commands();
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

/// Points at `tile`: the pointer one down and right of it (design v27 §6.5).
fn point(app: &mut App, world: &World, tile: Pos) {
    apply(
        app,
        world,
        Action::Point(Position::new(tile.x + 1, tile.y + 1)),
    );
}

/// Hands the app's commands to the world and runs a tick, as a frame does.
fn tick(app: &mut App, world: &mut World) {
    for command in app.take_commands() {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, world);
}

#[test]
fn every_mode_starts_with_the_cursor_hidden_and_moving_it_tells_the_world_nothing() {
    let world = field(&[]);
    let mut app = app(&world);
    for mode in CursorMode::ALL {
        apply(&mut app, &world, Action::Mode(mode));
        assert!(!app.visible(), "{mode:?}");
    }
    point(&mut app, &world, at(3, 3));
    assert_eq!(app.take_commands(), []);
}

#[test]
fn h_shows_the_cursor_where_it_is_and_again_hides_it() {
    let world = field(&[]);
    let mut app = app(&world);
    point(&mut app, &world, at(3, 3));
    apply(&mut app, &world, Action::ToggleVisible);
    assert!(app.visible());
    assert_eq!(
        app.take_commands(),
        [
            Command::MoveCursor { tile: at(3, 3) },
            Command::ShowCursor { visible: true }
        ]
    );
    apply(&mut app, &world, Action::ToggleVisible);
    assert!(!app.visible());
    assert_eq!(
        app.take_commands(),
        [Command::ShowCursor { visible: false }]
    );
}

#[test]
fn while_visible_each_move_onto_a_new_tile_tells_the_world() {
    let world = field(&[]);
    let mut app = app(&world);
    point(&mut app, &world, at(3, 3));
    apply(&mut app, &world, Action::ToggleVisible);
    app.take_commands();
    point(&mut app, &world, at(4, 3));
    point(&mut app, &world, at(4, 3));
    point(&mut app, &world, at(5, 2));
    assert_eq!(
        app.take_commands(),
        [
            Command::MoveCursor { tile: at(4, 3) },
            Command::MoveCursor { tile: at(5, 2) }
        ]
    );
}

#[test]
fn each_mode_keeps_its_own_switch() {
    // Design v29 §6.5: visible in Train, the Cursor hides on going to
    // Select, and shows again on coming back.
    let world = field(&[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    apply(&mut app, &world, Action::ToggleVisible);
    app.take_commands();
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    assert!(!app.visible());
    assert_eq!(
        app.take_commands(),
        [Command::ShowCursor { visible: false }]
    );
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    assert_eq!(app.take_commands(), []);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    assert!(app.visible());
    let commands = app.take_commands();
    assert_eq!(
        commands.last(),
        Some(&Command::ShowCursor { visible: true })
    );
}

#[test]
fn a_visible_cursor_following_a_sprite_tells_the_world_as_the_sprite_walks() {
    let mut world = field(&[at(2, 2)]);
    let mut app = app(&world);
    let sprite = world.sprite_at(at(2, 2)).expect("a sprite").id();
    point(&mut app, &world, at(2, 2));
    apply(&mut app, &world, Action::Follow { at: None });
    apply(&mut app, &world, Action::ToggleVisible);
    let mut told = Vec::new();
    for _ in 0..40 {
        for command in app.take_commands() {
            if let Command::MoveCursor { tile } = command {
                told.push(tile);
            }
            world.submit(command);
        }
        let events = world.step();
        app.record(&events, &world);
    }
    let now = world.sprite(sprite).expect("the sprite").pos();
    assert_eq!(told.first(), Some(&at(2, 2)));
    tick(&mut app, &mut world);
    assert_eq!(world.cursor().tile(), Some(now), "told {told:?}");
}

/// The status line of a 100×30 screen.
fn status_line(app: &App, world: &World) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..100).map(|x| buffer[(x, 29)].symbol()).collect()
}

#[test]
fn the_status_line_says_when_sprites_can_see_the_cursor() {
    let world = field(&[]);
    let areas = ui::areas(ratatui::layout::Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), Theme::cp437(), 1, areas);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    assert!(!status_line(&app, &world).contains("seen"));
    apply(&mut app, &world, Action::ToggleVisible);
    assert!(
        status_line(&app, &world).contains("│ TRAIN · seen"),
        "{}",
        status_line(&app, &world)
    );
}

#[test]
fn h_toggles_whether_sprites_can_see_the_cursor_and_held_does_not_flicker() {
    let mut keys = Keys::new();
    for h in ['h', 'H'] {
        let press = KeyEvent::new(KeyCode::Char(h), KeyModifiers::NONE);
        assert_eq!(keys.action_for(press), Some(Action::ToggleVisible));
    }
    let held =
        KeyEvent::new_with_kind(KeyCode::Char('h'), KeyModifiers::NONE, KeyEventKind::Repeat);
    assert_eq!(keys.action_for(held), None);
}

/// The selected sprite's observed lines, newest first.
fn observed(app: &App) -> Vec<String> {
    app.observed().map(|o| o.line.clone()).collect()
}

#[test]
fn a_touch_from_a_visible_cursor_is_told_as_coming_from_the_cursor() {
    // Design v29 §6.1: a sprite that can see the Cursor knows where the
    // touch came from.
    for (visible, line) in [
        (false, "Felt a gentle touch out of nowhere"),
        (true, "Felt a gentle touch from the Cursor"),
    ] {
        let mut world = field(&[at(2, 2)]);
        let mut app = app(&world);
        apply(&mut app, &world, Action::SelectNext);
        apply(&mut app, &world, Action::Mode(CursorMode::Train));
        if visible {
            apply(&mut app, &world, Action::ToggleVisible);
        }
        apply(&mut app, &world, Action::Follow { at: None });
        apply(
            &mut app,
            &world,
            Action::Press {
                button: terra_tui::input::Button::Left,
                amplified: false,
            },
        );
        tick(&mut app, &mut world);
        assert_eq!(observed(&app).first().map(String::as_str), Some(line));
    }
}

#[test]
fn being_led_and_shoved_by_a_visible_cursor_is_told_as_the_cursor_s_doing() {
    for (visible, how) in [(false, "out of nowhere"), (true, "by the Cursor")] {
        let mut world = field(&[at(2, 2)]);
        let mut app = app(&world);
        apply(&mut app, &world, Action::SelectNext);
        apply(&mut app, &world, Action::Mode(CursorMode::Grab));
        if visible {
            apply(&mut app, &world, Action::ToggleVisible);
        }
        point(&mut app, &world, at(2, 2));
        apply(
            &mut app,
            &world,
            Action::Click {
                at: Position::new(3, 3),
                button: terra_tui::input::Button::Left,
                amplified: false,
            },
        );
        tick(&mut app, &mut world);
        world.submit(Command::Shove {
            toward: terra_sim::Dir::E,
            tiles: 2,
        });
        for _ in 0..4 {
            tick(&mut app, &mut world);
        }
        let lines = observed(&app);
        assert!(
            lines.contains(&format!("Was pulled along {how}")),
            "{lines:?}"
        );
        assert!(lines.contains(&format!("Was shoved {how}")), "{lines:?}");
    }
}
