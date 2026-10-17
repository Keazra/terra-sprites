//! The view past the wall (M2 design §11): space round the map's glass,
//! the stars going by in it, and the pointer over it, where the Cursor
//! can't reach.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    Command, CursorTouch, DataPack, Dir, EntityId, Grip, Map, Pos, Scenario, ScriptedAction, World,
};
use terra_tui::app::{App, CursorMode, Flow};
use terra_tui::input::{Action, Button};
use terra_tui::space::Place;
use terra_tui::theme::Theme;
use terra_tui::ui;

/// The screen every test draws: the smallest the game allows, with a map
/// view of 52×21 tiles from screen cell (1, 2).
const SCREEN: Size = Size::new(100, 30);

/// The map view's tiles on `SCREEN`.
const TILES: Rect = Rect::new(1, 2, 52, 21);

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

/// An all-grass world `width`×`height`, with starter sprites on `sprites`.
fn field(width: usize, height: usize, sprites: &[Pos]) -> World {
    field_with(width, height, &[], sprites)
}

/// An all-grass world `width`×`height`, with `objects`, and starter sprites
/// on `sprites` that rest, so they go only where they're led.
fn field_with(width: usize, height: usize, objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    let row = ".".repeat(width);
    let map = Map::from_ascii(&vec![row.as_str(); height], &pack()).expect("valid drawing");
    let scripted: Vec<(Pos, ScriptedAction)> = sprites
        .iter()
        .flat_map(|&pos| [(pos, ScriptedAction::Rest); 50])
        .collect();
    let sprites: Vec<(Pos, _)> = sprites.iter().map(|&pos| (pos, None)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
        newborns: &[],
    };
    World::from_scenario(scenario, pack(), 1).expect("valid scenario")
}

/// Hands the app's commands to the world and runs a tick, as a frame does.
fn tick(app: &mut App, world: &mut World) {
    for command in app.take_commands() {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, world);
}

/// A 10×6 field with a sprite on (4, 3). It fits in the map view with its
/// glass, so it's drawn in the middle: tile (x, y) on screen cell
/// (22 + x, 9 + y), with the glass a cell out all round.
fn small_field() -> (World, EntityId) {
    small_field_with_sprite_on(Pos { x: 4, y: 3 })
}

/// The small field, with its sprite on `sprite` instead.
fn small_field_with_sprite_on(sprite: Pos) -> (World, EntityId) {
    let world = field(10, 6, &[sprite]);
    let id = world.sprite_at(sprite).expect("the sprite").id();
    (world, id)
}

fn app(world: &World) -> App {
    let mut app = App::new(world.map(), Theme::cp437(), 1, ui::areas(SCREEN));
    app.take_commands();
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

/// The screen cell of the small field's `tile`.
fn cell(tile: Pos) -> Position {
    Position::new(22 + tile.x, 9 + tile.y)
}

/// Where the pointer rests to point at the small field's `tile`: a cell
/// down and right of it (design v27 §6.5).
fn on(tile: Pos) -> Position {
    let at = cell(tile);
    Position::new(at.x + 1, at.y + 1)
}

/// Where the pointer rests to point at (`x`, `y`) round the small field,
/// counted on past its wall: (-1, `y`) is the glass, and further out is
/// space.
fn past(x: i32, y: i32) -> Position {
    let at = |offset: i32| u16::try_from(offset).expect("on screen");
    Position::new(at(23 + x), at(10 + y))
}

/// A cell over space, well clear of the small field and its glass.
const SPACE: Position = Position::new(6, 5);

fn render(app: &App, world: &World) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(SCREEN.width, SCREEN.height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// The map view's tiles as drawn, a string a row.
fn map_view(screen: &Buffer) -> Vec<String> {
    (TILES.top()..TILES.bottom())
        .map(|y| {
            (TILES.left()..TILES.right())
                .map(|x| screen[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// The cells of the map view with a star of the default theme drawn on.
fn stars(screen: &Buffer) -> Vec<Position> {
    TILES
        .positions()
        .filter(|&at| matches!(screen[at].symbol(), "·" | "∙" | "*"))
        .collect()
}

fn status_line(screen: &Buffer) -> String {
    (0..SCREEN.width)
        .map(|x| screen[(x, SCREEN.height - 1)].symbol())
        .collect()
}

#[test]
fn stars_are_drawn_only_on_space() {
    // M2 design §11.3: never over the map or its glass.
    let (world, _) = small_field();
    let mut app = app(&world);
    // With the pointer over space, the Cursor and its `·` mark aren't drawn.
    apply(&mut app, &world, Action::Point(SPACE));
    for _ in 0..8 {
        let screen = render(&app, &world);
        let stars = stars(&screen);
        assert!(stars.len() > 20, "plenty of them: {}", stars.len());
        for at in stars {
            assert_eq!(app.place_at(at), Some(Place::Space), "{at:?}");
            let fg = screen[at].fg;
            assert!(
                matches!(fg, Color::DarkGray | Color::Gray | Color::White),
                "{fg:?}"
            );
        }
        app.animate(Duration::from_millis(700));
    }
    // A map that fills the map view shows no space, so no stars: the only
    // `·` is the Cursor's mark, beside it.
    let world = field(120, 60, &[]);
    let app = self::app(&world);
    let cursor = app.cell_of(app.cursor()).expect("in view");
    let beside = |at: &Position| at.x.abs_diff(cursor.x) <= 1 && at.y.abs_diff(cursor.y) <= 1;
    let stars = stars(&render(&app, &world));
    assert!(stars.iter().all(beside), "{stars:?}");
}

#[test]
fn the_stars_stand_still_while_paused_even_through_a_step() {
    // M2 design §11.3.
    let (world, _) = small_field();
    let mut app = app(&world);
    app.animate(Duration::from_secs(2));
    let before = map_view(&render(&app, &world));
    apply(&mut app, &world, Action::TogglePause);
    apply(&mut app, &world, Action::StepOnce);
    app.animate(Duration::from_secs(10));
    assert_eq!(map_view(&render(&app, &world)), before, "paused");
    apply(&mut app, &world, Action::TogglePause);
    app.animate(Duration::from_secs(1));
    assert_ne!(map_view(&render(&app, &world)), before, "running again");
}

#[test]
fn the_stars_drift_with_the_speed_and_as_far_at_16x_as_at_4x() {
    // M2 design §11.3: no quicker than at 4×, so 16× and Max don't blur.
    let (world, _) = small_field();
    let after = |faster: usize| {
        let mut app = app(&world);
        // A hand-made world is a Story world, which `+` takes no faster
        // than 4× (M2 design §16), so the clock is sped up directly.
        for _ in 0..faster {
            app.clock.faster();
        }
        app.animate(Duration::from_secs(3));
        map_view(&render(&app, &world))
    };
    let (at_1x, at_2x, at_4x) = (after(0), after(1), after(2));
    assert_ne!(at_1x, at_2x);
    assert_ne!(at_2x, at_4x);
    assert_eq!(after(3), at_4x, "8×");
    assert_eq!(after(4), at_4x, "16×");
    assert_eq!(after(5), at_4x, "Max");
}

#[test]
fn over_space_the_cursor_is_not_drawn_and_the_status_line_says_space() {
    // M2 design §11.4: it stays on its last tile, the sprite, out of sight,
    // and shows again when the pointer comes back.
    let (world, id) = small_field();
    let mut app = app(&world);
    let sprite = Pos { x: 4, y: 3 };
    assert_eq!(app.cell_of(sprite), Some(cell(sprite)));
    apply(&mut app, &world, Action::Point(on(sprite)));
    let cell = cell(sprite);
    // The 3×3 grid round the sprite, a row at a time, whether the sprite is
    // in reverse video, as the Cursor's target, and the status line.
    let drawn = |app: &App| {
        let screen = render(app, &world);
        let grid: String = (cell.y - 1..=cell.y + 1)
            .flat_map(|y| (cell.x - 1..=cell.x + 1).map(move |x| (x, y)))
            .map(|at| screen[at].symbol().to_string())
            .collect();
        let reversed = screen[cell].modifier.contains(Modifier::REVERSED);
        (grid, reversed, status_line(&screen))
    };
    let (grid, reversed, status) = drawn(&app);
    assert_eq!(
        (grid.as_str(), reversed),
        ("♦↓·→☺←·↑♦", true),
        "on the sprite"
    );
    assert!(status.starts_with(&format!(" (4,3) grass · Sprite #{}", id.0)));
    for over in [SPACE, Position::new(21, 9), Position::new(33, 16)] {
        apply(&mut app, &world, Action::Point(over));
        assert!(app.out_of_reach(), "{over:?}");
        assert_eq!(app.cursor(), sprite, "on its last tile");
        let (grid, reversed, status) = drawn(&app);
        assert_eq!((grid.as_str(), reversed), ("....☺....", false), "{over:?}");
        assert!(status.starts_with(" space │ SELECT"), "{status}");
    }
    apply(&mut app, &world, Action::Point(on(sprite)));
    assert_eq!(drawn(&app).0, "♦↓·→☺←·↑♦", "back");
}

#[test]
fn a_withdrawn_cursor_is_out_of_reach_until_the_pointer_comes_back() {
    // #241: a picture of the game with no player in it, such as the
    // renderer's clips, withdraws the Cursor: out of reach, as a pointer
    // over space takes it, with no pointer anywhere, so the status line
    // says nothing of what it's over (M2 design §11.4). Ticks leave it
    // there.
    let (mut world, _) = small_field();
    let mut app = app(&world);
    let sprite = Pos { x: 4, y: 3 };
    apply(&mut app, &world, Action::Point(on(sprite)));
    app.withdraw_cursor();
    tick(&mut app, &mut world);
    assert!(app.out_of_reach());
    let screen = render(&app, &world);
    let at = cell(sprite);
    // Grass round the sprite, which rests, with no arms or marks.
    let round: String = (at.y - 1..=at.y + 1)
        .flat_map(|y| (at.x - 1..=at.x + 1).map(move |x| (x, y)))
        .filter(|&(x, y)| (x, y) != (at.x, at.y))
        .map(|cell| screen[cell].symbol().to_string())
        .collect();
    assert_eq!(round, "........");
    assert!(!screen[at].modifier.contains(Modifier::REVERSED));
    assert!(app.cursor_withdrawn());
    let status = status_line(&screen);
    assert!(status.starts_with(" SELECT"), "{status}");
    apply(&mut app, &world, Action::Point(on(sprite)));
    assert!(!app.out_of_reach(), "back over the map");
    assert!(!app.cursor_withdrawn());
}

#[test]
fn over_space_a_click_q_or_e_does_nothing_in_any_mode() {
    // M2 design §11.4, every input that reaches the rule
    // (docs/agents/how-we-work.md): a click of each button, plain or
    // amplified, `Q` and `E`, and a middle click or `F`, with the Cursor
    // left on a sprite, which each would act on.
    let (world, _) = small_field();
    let sprite = Pos { x: 4, y: 3 };
    let mut inputs = Vec::new();
    for button in [Button::Left, Button::Right] {
        for amplified in [false, true] {
            inputs.push(Action::Click {
                at: SPACE,
                button,
                amplified,
            });
            inputs.push(Action::Press { button, amplified });
        }
    }
    inputs.push(Action::middle_click(SPACE));
    inputs.push(Action::Follow { at: None });
    for mode in CursorMode::ALL {
        for input in &inputs {
            let mut app = app(&world);
            apply(&mut app, &world, Action::Mode(mode));
            apply(&mut app, &world, Action::Point(on(sprite)));
            apply(&mut app, &world, Action::Point(SPACE));
            apply(&mut app, &world, *input);
            let what = format!("{input:?} in {mode:?}");
            assert_eq!(app.take_commands(), [], "{what}");
            assert_eq!(app.selection(), None, "{what}");
            assert_eq!(app.followed(), None, "{what}");
            assert!(!app.aiming(), "{what}");
        }
    }
}

#[test]
fn following_a_sprite_a_click_over_space_and_q_still_reach_it() {
    // M2 design §11.4: the Cursor stays on its sprite, drawn, wherever the
    // pointer is.
    let (world, id) = small_field();
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    let sprite = Pos { x: 4, y: 3 };
    apply(&mut app, &world, Action::middle_click(on(sprite)));
    assert_eq!(app.followed(), Some(id));
    apply(&mut app, &world, Action::Point(SPACE));
    assert!(!app.out_of_reach(), "following, it isn't out of reach");
    let screen = render(&app, &world);
    assert!(!status_line(&screen).contains("space"));
    let cell = cell(sprite);
    assert_eq!(
        screen[(cell.x, cell.y - 1)].symbol(),
        "▼",
        "drawn, clamping it"
    );
    let pet = CursorTouch::Pet.command(id, app.clock.reach_back());
    apply(&mut app, &world, Action::left_click(SPACE));
    assert_eq!(
        app.take_commands(),
        std::slice::from_ref(&pet),
        "a click over space"
    );
    apply(&mut app, &world, Action::Point(Position::new(21, 12)));
    apply(&mut app, &world, Action::left_click(Position::new(21, 12)));
    assert_eq!(
        app.take_commands(),
        std::slice::from_ref(&pet),
        "a click on the glass"
    );
    let press = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, press);
    assert_eq!(app.take_commands(), [pet], "Q");
    // `F` over space stops following, as anywhere.
    apply(&mut app, &world, Action::middle_click(SPACE));
    assert_eq!(app.followed(), None);
    assert!(app.out_of_reach(), "now the pointer's out of reach");
    apply(&mut app, &world, Action::left_click(SPACE));
    assert_eq!(app.take_commands(), [], "and a click does nothing");
}

#[test]
fn f_over_space_follows_the_selected_sprite_not_the_one_the_cursor_was_on() {
    // M2 design §11.4: over space, `F` reaches no tile.
    let world = field(10, 6, &[Pos { x: 4, y: 3 }, Pos { x: 8, y: 1 }]);
    let selected = world.sprite_at(Pos { x: 8, y: 1 }).expect("a sprite").id();
    for input in [Action::Follow { at: None }, Action::middle_click(SPACE)] {
        let mut app = app(&world);
        apply(&mut app, &world, Action::left_click(on(Pos { x: 8, y: 1 })));
        apply(&mut app, &world, Action::Point(on(Pos { x: 4, y: 3 })));
        apply(&mut app, &world, Action::Point(SPACE));
        apply(&mut app, &world, input);
        assert_eq!(app.followed(), Some(selected), "{input:?}");
    }
}

#[test]
fn out_of_sight_the_cursor_draws_no_leash() {
    // M2 design §11.4. The led sprite is on (2, 3); the Cursor, four tiles
    // east of it, its arrow on (5, 3), leaves a leash on (3, 3) and (4, 3).
    let (world, _) = small_field_with_sprite_on(Pos { x: 2, y: 3 });
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 2, y: 3 })));
    apply(&mut app, &world, Action::Point(on(Pos { x: 6, y: 3 })));
    let leash = |app: &App| {
        let screen = render(app, &world);
        [3, 4].map(|x| screen[cell(Pos { x, y: 3 })].symbol().to_string())
    };
    assert_eq!(leash(&app), ["·", "·"], "led");
    apply(&mut app, &world, Action::Point(SPACE));
    assert_eq!(leash(&app), [".", "."], "out of sight");
}

#[test]
fn the_cursor_s_arrows_and_marks_never_cover_the_glass() {
    // M2 design §11.4: on the map's corner tile, the arrows and marks that
    // would fall on the glass aren't drawn.
    let (world, _) = small_field();
    let mut app = app(&world);
    apply(&mut app, &world, Action::Point(on(Pos { x: 0, y: 0 })));
    let screen = render(&app, &world);
    let corner = cell(Pos { x: 0, y: 0 });
    let row = |y: u16| -> String {
        (corner.x - 1..=corner.x + 1)
            .map(|x| screen[(x, y)].symbol())
            .collect()
    };
    assert_eq!(
        [row(corner.y - 1), row(corner.y), row(corner.y + 1)],
        ["╔══", "║.←", "║↑♦"]
    );
}

#[test]
fn leading_a_sprite_over_space_the_cursor_stays_and_the_sprite_is_not_led_on() {
    // M2 design §11.4. The sprite is on (1, 3); pointed at (9, 3), the
    // Cursor waits at the leash's end, (6, 3), for it to catch up.
    let mut world = field(10, 6, &[Pos { x: 1, y: 3 }]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 1, y: 3 })));
    tick(&mut app, &mut world);
    apply(&mut app, &world, Action::Point(on(Pos { x: 9, y: 3 })));
    assert_eq!(app.cursor(), Pos { x: 6, y: 3 });
    apply(&mut app, &world, Action::Point(SPACE));
    for _ in 0..40 {
        tick(&mut app, &mut world);
        assert_eq!(app.cursor(), Pos { x: 6, y: 3 }, "it stays");
    }
    assert!(matches!(app.grip(&world), Some(Grip::Leads(_))));
    let led = world.sprites().next().expect("the sprite").pos();
    assert!(led.x <= 6, "led no further than the Cursor: {led:?}");
}

#[test]
fn an_item_held_over_space_stays_held_until_the_pointer_comes_back() {
    // M2 design §11.4: over space, a click, `Q` or `E` does nothing, so the
    // ball the Cursor holds isn't put down on its last tile, (7, 3), or
    // aimed; back over the map, `Q` puts it down where the pointer is.
    let mut world = field_with(10, 6, &[(Pos { x: 5, y: 3 }, "ball")], &[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 5, y: 3 })));
    tick(&mut app, &mut world);
    assert!(matches!(app.grip(&world), Some(Grip::Holds(_))));
    apply(&mut app, &world, Action::Point(on(Pos { x: 7, y: 3 })));
    apply(&mut app, &world, Action::Point(SPACE));
    for button in [Button::Left, Button::Right] {
        let click = Action::Click {
            at: SPACE,
            button,
            amplified: false,
        };
        let press = Action::Press {
            button,
            amplified: false,
        };
        for input in [click, press] {
            apply(&mut app, &world, input);
            assert_eq!(app.take_commands(), [], "{input:?}");
            assert!(!app.aiming(), "{input:?}");
        }
    }
    for _ in 0..10 {
        tick(&mut app, &mut world);
    }
    assert!(matches!(app.grip(&world), Some(Grip::Holds(_))), "held");
    assert_eq!(app.cursor(), Pos { x: 7, y: 3 }, "on its last tile");
    apply(&mut app, &world, Action::Point(on(Pos { x: 8, y: 2 })));
    let q = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, q);
    let put_down = Command::PutDown {
        tile: Pos { x: 8, y: 2 },
    };
    assert_eq!(app.take_commands(), [put_down]);
}

#[test]
fn an_aim_sent_over_space_leaves_the_cursor_where_it_was_out_of_reach() {
    // M2 design §11.4: "an aim that ends over space, sent or not, leaves
    // the Cursor where it was". The ball is aimed from (5, 3), pulled
    // across (2, 3) and on into space, and thrown from there.
    let mut world = field_with(10, 6, &[(Pos { x: 5, y: 3 }, "ball")], &[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 5, y: 3 })));
    tick(&mut app, &mut world);
    apply(
        &mut app,
        &world,
        Action::right_click(on(Pos { x: 5, y: 3 })),
    );
    apply(&mut app, &world, Action::Point(on(Pos { x: 2, y: 3 })));
    apply(&mut app, &world, Action::Point(SPACE));
    apply(&mut app, &world, Action::right_click(SPACE));
    let sent = app.take_commands();
    assert!(
        matches!(sent[..], [Command::Throw { from, .. }] if from == Pos { x: 5, y: 3 }),
        "{sent:?}"
    );
    assert!(!app.aiming());
    assert!(app.out_of_reach());
    assert_eq!(app.cursor(), Pos { x: 5, y: 3 }, "not the last tile pulled");
    for _ in 0..5 {
        tick(&mut app, &mut world);
        assert_eq!(app.cursor(), Pos { x: 5, y: 3 }, "it stays");
    }
}

#[test]
fn aiming_the_pull_goes_on_past_the_wall_and_a_right_click_or_e_over_space_sends_it() {
    // M2 design §11.4: the Cursor sits on what the player aims, so it isn't
    // out of reach, and the pointer pulls on past the wall. The ball is on
    // (0, 3), against the wall: pulled back over the glass to (-4, 3), in
    // space, it's thrown 4 tiles east. A right click sends an aim where the
    // terminal doesn't say when the button's let go (design v25 §6.5).
    let throw = Command::Throw {
        from: Pos { x: 0, y: 3 },
        toward: Dir::E,
        tiles: 4,
    };
    let sends = [
        Action::right_click(past(-4, 3)),
        Action::Press {
            button: Button::Right,
            amplified: false,
        },
    ];
    for send in sends {
        let mut world = field_with(10, 6, &[(Pos { x: 0, y: 3 }, "ball")], &[]);
        let mut app = app(&world);
        apply(&mut app, &world, Action::Mode(CursorMode::Grab));
        apply(&mut app, &world, Action::left_click(on(Pos { x: 0, y: 3 })));
        tick(&mut app, &mut world);
        apply(
            &mut app,
            &world,
            Action::right_click(on(Pos { x: 0, y: 3 })),
        );
        apply(&mut app, &world, Action::Point(past(-1, 3)));
        assert_eq!(app.aim_line(&world).len(), 1, "over the glass");
        apply(&mut app, &world, Action::Point(past(-4, 3)));
        assert!(!app.out_of_reach(), "{send:?}");
        assert_eq!(app.aim_line(&world).len(), 4, "over space");
        apply(&mut app, &world, send);
        assert_eq!(
            app.take_commands(),
            std::slice::from_ref(&throw),
            "{send:?}"
        );
    }
}

#[test]
fn aiming_a_click_over_space_reaches_what_is_aimed() {
    // M2 design §11.4: while the player aims, the Cursor isn't out of
    // reach, so a left click over space reaches what it aims, as `Q` does:
    // it puts the ball down where it's aimed from.
    let mut world = field_with(10, 6, &[(Pos { x: 5, y: 3 }, "ball")], &[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 5, y: 3 })));
    tick(&mut app, &mut world);
    apply(
        &mut app,
        &world,
        Action::right_click(on(Pos { x: 5, y: 3 })),
    );
    apply(&mut app, &world, Action::left_click(SPACE));
    let put_down = Command::PutDown {
        tile: Pos { x: 5, y: 3 },
    };
    assert_eq!(app.take_commands(), [put_down]);
}

#[test]
fn leading_the_followed_sprite_over_space_q_does_nothing() {
    // M2 design §11.4: Follow steps aside while the Cursor leads its sprite,
    // so the Cursor follows the pointer, and over space it's out of reach.
    let mut world = field(10, 6, &[Pos { x: 4, y: 3 }]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 4, y: 3 })));
    tick(&mut app, &mut world);
    apply(
        &mut app,
        &world,
        Action::middle_click(on(Pos { x: 4, y: 3 })),
    );
    tick(&mut app, &mut world);
    apply(&mut app, &world, Action::Point(SPACE));
    assert!(app.out_of_reach());
    app.take_commands();
    let q = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, q);
    assert_eq!(app.take_commands(), [], "a grab click would let go");
    assert!(matches!(app.grip(&world), Some(Grip::Leads(_))));
}

#[test]
fn the_wheel_over_space_cycles_the_modes() {
    // M2 design §11.4: as anywhere over the map view (design v22 §6.5).
    let (world, _) = small_field();
    let mut app = app(&world);
    let wheel = Action::Wheel {
        at: SPACE,
        notches: 1,
    };
    apply(&mut app, &world, wheel);
    assert_eq!(app.mode(), CursorMode::Train);
}

#[test]
fn off_the_map_view_after_space_the_cursor_shows_on_its_last_tile_again() {
    // M2 design §11.4: out of reach only while the pointer is over the
    // glass or space. Off the map view, over the inspector, the Cursor stays
    // on its last tile, drawn, as it always has, and `Q` reaches it.
    let (world, id) = small_field();
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    apply(&mut app, &world, Action::Point(on(Pos { x: 4, y: 3 })));
    apply(&mut app, &world, Action::Point(SPACE));
    assert!(app.out_of_reach());
    let inspector = Position::new(70, 10);
    assert!(!TILES.contains(inspector));
    apply(&mut app, &world, Action::Point(inspector));
    assert!(!app.out_of_reach());
    let status = status_line(&render(&app, &world));
    assert!(status.starts_with(" (4,3) grass · Sprite #"), "{status}");
    let q = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, q);
    let pet = CursorTouch::Pet.command(id, app.clock.reach_back());
    assert_eq!(app.take_commands(), [pet]);
}

#[test]
fn over_space_esc_still_ends_an_aim_then_puts_the_held_item_down() {
    // M2 design §11.4 leaves `Esc` as it is (design v25, v33 §6.5): first
    // it ends the aim, then it puts the ball down where the Cursor stayed,
    // on (5, 3), where it was aimed from, not on (2, 3), the last tile the
    // pull crossed: once the aim ends past the wall, the Cursor can't move.
    let mut world = field_with(10, 6, &[(Pos { x: 5, y: 3 }, "ball")], &[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 5, y: 3 })));
    tick(&mut app, &mut world);
    apply(
        &mut app,
        &world,
        Action::right_click(on(Pos { x: 5, y: 3 })),
    );
    apply(&mut app, &world, Action::Point(on(Pos { x: 2, y: 3 })));
    apply(&mut app, &world, Action::Point(SPACE));
    apply(&mut app, &world, Action::Back);
    assert!(!app.aiming());
    assert_eq!(app.take_commands(), [], "nothing thrown");
    assert!(app.out_of_reach());
    assert_eq!(app.cursor(), Pos { x: 5, y: 3 });
    apply(&mut app, &world, Action::Back);
    let put_down = Command::PutDown {
        tile: Pos { x: 5, y: 3 },
    };
    assert_eq!(app.take_commands(), [put_down]);
}

#[test]
fn over_space_esc_ends_a_shove_s_aim_and_the_led_sprite_stands_still() {
    // M2 design §11.4: a shove aimed and then called off with the pointer
    // over space leaves the Cursor on the sprite, out of reach, so the
    // sprite isn't led after (6, 3), the last tile the pull crossed.
    let mut world = field(10, 6, &[Pos { x: 4, y: 3 }]);
    let id = world
        .sprite_at(Pos { x: 4, y: 3 })
        .expect("the sprite")
        .id();
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::left_click(on(Pos { x: 4, y: 3 })));
    tick(&mut app, &mut world);
    apply(
        &mut app,
        &world,
        Action::right_click(on(Pos { x: 4, y: 3 })),
    );
    apply(&mut app, &world, Action::Point(on(Pos { x: 6, y: 3 })));
    apply(&mut app, &world, Action::Point(SPACE));
    apply(&mut app, &world, Action::Back);
    assert!(!app.aiming());
    assert!(app.out_of_reach());
    for _ in 0..20 {
        tick(&mut app, &mut world);
        assert_eq!(app.cursor(), Pos { x: 4, y: 3 }, "it stays");
    }
    assert!(matches!(app.grip(&world), Some(Grip::Leads(_))));
    let led = world.sprite(id).expect("the sprite").pos();
    assert_eq!(led, Pos { x: 4, y: 3 }, "it stands still");
}
