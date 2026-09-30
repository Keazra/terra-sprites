//! The Cursor on screen (design v21 §6.5): its modes, Train's pets and zaps,
//! and locking on.

use std::time::Duration;

use ratatui::layout::{Position, Rect};
use terra_sim::{
    Command, DataPack, DeathCause, EntityId, Event, EventKind, Map, Pos, Rejection, Scenario,
    ScriptedAction, World,
};
use terra_tui::app::{App, Areas, CursorMode, Flow, Screen, StatusMark};
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

fn pet(sprite: EntityId, reach_back: u64) -> Command {
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
fn the_wheel_cycles_the_cursor_modes_wrapping_round() {
    // Design v21 §6.5: a notch down is the next mode, up the previous; with
    // Select and Train alone, either way goes to the other.
    let world = field(&[]);
    let mut app = app(&world);
    let wheel = |notches| Action::Wheel {
        at: Position::new(4, 2),
        notches,
    };
    apply(&mut app, &world, wheel(1));
    assert_eq!(app.mode(), CursorMode::Train);
    apply(&mut app, &world, wheel(1));
    assert_eq!(app.mode(), CursorMode::Select, "wrapping round");
    apply(&mut app, &world, wheel(-1));
    assert_eq!(app.mode(), CursorMode::Train);
    apply(&mut app, &world, wheel(3));
    assert_eq!(app.mode(), CursorMode::Select, "a notch a mode");
}

#[test]
fn the_wheel_off_the_map_leaves_the_mode_alone() {
    // Design v22 §6.5: over the event log or the bars it does nothing. The
    // map view's tiles are drawn at cells (0, 0) to (9, 5).
    let world = field(&[]);
    let mut app = app(&world);
    let off_the_map = Action::Wheel {
        at: Position::new(20, 12),
        notches: 1,
    };
    apply(&mut app, &world, off_the_map);
    assert_eq!(app.mode(), CursorMode::Select);
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

/// Selects the sprite on `tile` and locks the Cursor on to it, in Select
/// mode: a left click, then a right click.
fn lock_on(app: &mut App, world: &World, tile: Pos) {
    click(app, world, tile, Button::Left);
    click(app, world, tile, Button::Right);
}

#[test]
fn in_select_mode_a_right_click_locks_the_cursor_on_to_the_selection_and_another_lets_go() {
    // Design v21 §6.5: wherever the click is, with a sprite selected.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.locked(), None, "selecting doesn't lock");
    click(&mut app, &world, at(8, 5), Button::Right);
    assert_eq!(app.locked(), Some(id));
    click(&mut app, &world, at(0, 0), Button::Right);
    assert_eq!(app.locked(), None);
}

#[test]
fn with_nothing_selected_a_right_click_on_a_sprite_selects_it_and_locks_on() {
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    click(&mut app, &world, at(1, 1), Button::Right);
    assert_eq!(
        (app.selection(), app.locked()),
        (None, None),
        "empty ground"
    );
    click(&mut app, &world, at(4, 2), Button::Right);
    assert_eq!(app.locked(), Some(id));
}

#[test]
fn locked_on_a_left_click_on_empty_ground_keeps_the_selection() {
    // Design v21 §6.5: a stray click can't lose a lock; unlocked, it clears.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    lock_on(&mut app, &world, at(4, 2));
    click(&mut app, &world, at(1, 1), Button::Left);
    assert_eq!(app.locked(), Some(id));
    click(&mut app, &world, at(1, 1), Button::Right);
    click(&mut app, &world, at(1, 1), Button::Left);
    assert_eq!(app.selection(), None);
}

#[test]
fn selecting_another_sprite_moves_the_lock_with_it() {
    // Design v21 §6.5: a left click on it, or Tab.
    let world = field(&[at(2, 2), at(6, 2)]);
    let (first, second) = (sprite_on(&world, at(2, 2)), sprite_on(&world, at(6, 2)));
    let mut app = app(&world);
    lock_on(&mut app, &world, at(2, 2));
    click(&mut app, &world, at(6, 2), Button::Left);
    assert_eq!(app.locked(), Some(second));
    apply(&mut app, &world, Action::SelectNext);
    assert_eq!(app.locked(), Some(first), "Tab wraps round to the first");
}

#[test]
fn locked_on_the_cursor_stays_on_its_sprite_whatever_the_pointer_does() {
    let world = field(&[at(4, 2)]);
    let mut app = app(&world);
    lock_on(&mut app, &world, at(4, 2));
    apply(&mut app, &world, Action::Point(Position::new(8, 5)));
    assert_eq!(app.cursor(), at(4, 2));
    click(&mut app, &world, at(4, 2), Button::Right);
    apply(&mut app, &world, Action::Point(Position::new(8, 5)));
    assert_eq!(
        app.cursor(),
        at(8, 5),
        "let go, it follows the pointer again"
    );
}

#[test]
fn locked_on_the_cursor_moves_with_its_sprite() {
    // Design v21 §6.5: the sprite walks off, and the Cursor goes with it.
    let data = pack();
    let map = Map::from_ascii(&[".........."; 6], &data).expect("valid drawing");
    let walk = ScriptedAction::Wander {
        destination: at(8, 2),
    };
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(at(1, 2), None)],
        scripted: &[(at(1, 2), walk)],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
    let mut app = app(&world);
    lock_on(&mut app, &world, at(1, 2));
    for _ in 0..3 {
        let events = world.step();
        app.record(&events, &world);
    }
    let now = world.sprites().next().expect("the sprite").pos();
    assert_ne!(now, at(1, 2), "it walked");
    assert_eq!(app.cursor(), now);
}

#[test]
fn the_locked_on_sprite_s_death_lets_go() {
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    lock_on(&mut app, &world, at(4, 2));
    let died = Event {
        tick: 0,
        kind: EventKind::Died {
            id,
            cause: DeathCause::Starvation,
            age: 5,
        },
    };
    app.record(&[died], &world);
    assert_eq!(app.locked(), None);
    apply(&mut app, &world, Action::Point(Position::new(8, 5)));
    assert_eq!(app.cursor(), at(8, 5), "it follows the pointer again");
}

#[test]
fn in_train_mode_the_locked_on_sprite_is_the_target_wherever_the_click_or_key_is() {
    // Design v21 §6.5.
    let world = field(&[at(2, 2), at(6, 2)]);
    let first = sprite_on(&world, at(2, 2));
    let mut app = app(&world);
    lock_on(&mut app, &world, at(2, 2));
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(6, 2), Button::Left);
    click(&mut app, &world, at(9, 5), Button::Right);
    let q = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, q);
    assert_eq!(
        app.take_commands(),
        [pet(first, 3), zap(first), pet(first, 3)]
    );
}

#[test]
fn q_and_e_act_where_the_cursor_is() {
    // Design v21 §6.5: under the pointer, or on the locked-on sprite.
    let world = field(&[at(4, 2), at(7, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Point(Position::new(4, 2)));
    let e = Action::Press {
        button: Button::Right,
        amplified: false,
    };
    apply(&mut app, &world, e);
    assert_eq!(app.locked(), Some(id), "E selected it and locked on");
    apply(&mut app, &world, Action::Point(Position::new(7, 2)));
    let q = Action::Press {
        button: Button::Left,
        amplified: false,
    };
    apply(&mut app, &world, q);
    assert_eq!(
        app.locked(),
        Some(id),
        "Q didn't select the sprite under the pointer"
    );
}

/// The Cursor touching sprite `id` on `tick`: a pet, hug, zap or shock, or
/// a pet refused.
fn touched(tick: u64, id: EntityId, what: &str) -> Event {
    let kind = match what {
        "pet" => EventKind::Rewarded {
            id,
            amplified: false,
        },
        "hug" => EventKind::Rewarded {
            id,
            amplified: true,
        },
        "zap" => EventKind::Corrected {
            id,
            amplified: false,
        },
        "shock" => EventKind::Corrected {
            id,
            amplified: true,
        },
        "refused pet" => EventKind::CommandRejected {
            command: pet(id, 3),
            reason: Rejection::Gone,
        },
        _ => unreachable!("{what}"),
    };
    Event { tick, kind }
}

#[test]
fn the_observed_list_tells_what_the_sprite_felt_not_where_it_came_from() {
    // Design v21 §6.1: the Cursor is invisible, so the touch came from
    // nowhere.
    let world = field(&[at(2, 2), at(6, 2)]);
    let (id, other) = (sprite_on(&world, at(2, 2)), sprite_on(&world, at(6, 2)));
    let mut app = app(&world);
    click(&mut app, &world, at(2, 2), Button::Left);
    let events = [
        touched(1, id, "pet"),
        touched(2, id, "hug"),
        touched(3, id, "zap"),
        touched(4, id, "shock"),
        touched(5, id, "shock"),
        touched(6, other, "pet"),
    ];
    app.record(&events, &world);
    let observed: Vec<(&str, u32)> = app.observed().map(|o| (o.line.as_str(), o.count)).collect();
    assert_eq!(
        observed,
        [
            ("Felt a jolt out of nowhere", 2),
            ("Felt a zap out of nowhere", 1),
            ("Felt a warm embrace out of nowhere", 1),
            ("Felt a gentle touch out of nowhere", 1),
        ]
    );
}

/// Moves the app's real-time clock on by `millis` milliseconds.
fn wait(app: &mut App, millis: u64) {
    app.animate(Duration::from_millis(millis));
}

#[test]
fn a_train_click_that_sends_a_command_flashes_sent_for_a_moment() {
    // Design v21 §6.5: about 0.3 s of real time. A Select click sends
    // nothing, so it doesn't flash.
    let world = field(&[at(4, 2)]);
    let mut app = app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.status_mark(), StatusMark::Idle);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 290);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 20);
    assert_eq!(app.status_mark(), StatusMark::Idle);
}

#[test]
fn the_world_s_report_flashes_applied_or_rejected() {
    // Design v21 §6.5: `☼` when a pet or zap applied, `?` when it was
    // refused.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    app.record(&[touched(1, id, "zap")], &world);
    assert_eq!(app.status_mark(), StatusMark::Applied);
    wait(&mut app, 310);
    assert_eq!(app.status_mark(), StatusMark::Idle);
    app.record(&[touched(2, id, "refused pet")], &world);
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    wait(&mut app, 310);
    assert_eq!(app.status_mark(), StatusMark::Idle);
}

#[test]
fn a_report_waits_until_sent_has_shown() {
    // Design v22 §6.5: at speed a command can apply in the frame it was
    // sent, so both flashes show, one after the other.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    app.record(&[touched(1, id, "pet")], &world);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 290);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 20);
    assert_eq!(app.status_mark(), StatusMark::Applied);
    wait(&mut app, 280);
    assert_eq!(app.status_mark(), StatusMark::Applied);
    wait(&mut app, 20);
    assert_eq!(app.status_mark(), StatusMark::Idle);
}

#[test]
fn a_new_click_flashes_sent_at_once_even_over_a_report() {
    // Design v21 §6.5: each click flashes `+` at once.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    app.record(&[touched(1, id, "pet")], &world);
    wait(&mut app, 100);
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.status_mark(), StatusMark::Sent);
}

#[test]
fn a_refusal_wins_over_a_command_applied_with_it() {
    // Design v22 §6.5: a refusal is what needs noticing, so `☼` doesn't
    // replace a `?` still to show or showing.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    let refused = touched(1, id, "refused pet");
    app.record(&[refused, touched(1, id, "pet")], &world);
    assert_eq!(app.status_mark(), StatusMark::Rejected, "in one tick");
    wait(&mut app, 100);
    app.record(&[touched(2, id, "pet")], &world);
    assert_eq!(app.status_mark(), StatusMark::Rejected, "while it shows");
}

#[test]
fn while_paused_sent_flashes_at_the_click_and_the_report_when_time_moves() {
    // Design v22 §6.5: the command waits for the next tick.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::TogglePause);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 5_000);
    assert_eq!(app.status_mark(), StatusMark::Idle, "paused, nothing yet");
    app.record(&[touched(1, id, "pet")], &world);
    assert_eq!(app.status_mark(), StatusMark::Applied, "time moved");
}

#[test]
fn a_train_click_with_nothing_to_act_on_flashes_rejected_at_once() {
    // Design v21 §6.5: it sends nothing, so there's no `+` to wait for,
    // even after a click that did send.
    let world = field(&[at(4, 2)]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    click(&mut app, &world, at(1, 1), Button::Right);
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(app.take_commands().len(), 1, "only the first click sent");
}

#[test]
fn a_flash_carries_on_through_a_change_of_mode() {
    // Design v22 §6.5.
    let world = field(&[at(4, 2)]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    assert_eq!(app.status_mark(), StatusMark::Sent);
}

#[test]
fn a_miss_then_a_quick_pet_shows_the_pet_applied() {
    // Design v22 §6.5: a refusal wins over a command applied with it, not
    // over a later click's.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(1, 1), Button::Left);
    assert_eq!(app.status_mark(), StatusMark::Rejected, "the miss");
    wait(&mut app, 130);
    click(&mut app, &world, at(4, 2), Button::Left);
    app.record(&[touched(1, id, "pet")], &world);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 300);
    assert_eq!(app.status_mark(), StatusMark::Applied, "the pet worked");
}

#[test]
fn a_refused_pet_then_a_quick_one_that_works_shows_the_second_applied() {
    // Design v22 §6.5: the marks follow the latest click, so its `+` puts
    // the first click's refusal, still waiting to show, behind it.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    wait(&mut app, 10);
    app.record(&[touched(1, id, "refused pet")], &world);
    wait(&mut app, 90);
    click(&mut app, &world, at(4, 2), Button::Left);
    wait(&mut app, 10);
    app.record(&[touched(2, id, "pet")], &world);
    wait(&mut app, 190);
    assert_eq!(app.status_mark(), StatusMark::Sent, "the second click's +");
    wait(&mut app, 110);
    assert_eq!(app.status_mark(), StatusMark::Applied, "the second pet");
}

#[test]
fn a_new_click_s_sent_isnt_cut_short_by_an_earlier_click_s_result() {
    // Design v22 §6.5: while paused, the first pet's `☼`, still waiting its
    // turn, would read as the second pet landing.
    let world = field(&[at(4, 2)]);
    let id = sprite_on(&world, at(4, 2));
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(4, 2), Button::Left);
    wait(&mut app, 10);
    app.record(&[touched(1, id, "pet")], &world);
    apply(&mut app, &world, Action::TogglePause);
    wait(&mut app, 90);
    click(&mut app, &world, at(4, 2), Button::Left);
    wait(&mut app, 250);
    assert_eq!(app.status_mark(), StatusMark::Sent);
    wait(&mut app, 100);
    assert_eq!(app.status_mark(), StatusMark::Idle, "paused: nothing yet");
}
