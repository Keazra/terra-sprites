//! Grab mode on screen (design v23 §6.5): a click grabs what's under the
//! Cursor, taking hold of a sprite or picking up an item, and the next lets
//! go or puts it down.

mod common;
use common::{
    Q, WorldBuilder, apply, at, click, field, follow, grab_app, object_on, point, sprite_on, tick,
};

use terra_sim::{Command, Grip, ScriptedAction, World};
use terra_tui::app::{App, CursorMode, Screen, StatusMark};
use terra_tui::input::{Action, Button};

#[test]
fn a_grab_click_on_a_sprite_takes_hold_of_it() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    let sprite = sprite_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::TakeHold { sprite }]);
}

#[test]
fn a_grab_click_on_an_item_picks_it_up() {
    let world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    let item = object_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::PickUp { item }]);
}

#[test]
fn a_sprite_is_grabbed_before_the_item_it_stands_on() {
    let world = field(&[(at(4, 2), "berry")], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    let sprite = sprite_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::TakeHold { sprite }]);
}

#[test]
fn a_grab_click_on_empty_ground_sends_nothing_and_says_so() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.take_commands(), Vec::new());
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(app.refusal(), Some("Nothing here to grab"));
}

#[test]
fn a_grab_click_on_a_bush_sends_nothing_and_says_it_is_rooted() {
    let world = field(&[(at(4, 2), "berry_bush")], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
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
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    click(&mut app, &world, at(7, 4), Button::Left);
    assert_eq!(app.take_commands(), vec![Command::LetGo]);
}

#[test]
fn holding_an_item_a_grab_click_puts_it_down_there() {
    let mut world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    click(&mut app, &world, at(7, 4), Button::Left);
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
    click(&mut app, &world, at(4, 2), Button::Left);
    click(&mut app, &world, at(6, 2), Button::Left);
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
    click(&mut app, &world, at(4, 2), Button::Left);
    assert_eq!(app.grip(&world), Some(Grip::Leads(mine)), "as queued");
    // Another command gets there first, so the world refuses the app's grab.
    world.submit(Command::TakeHold { sprite: other });
    tick(&mut app, &mut world);
    assert_eq!(app.grip(&world), Some(Grip::Leads(other)));
    assert_eq!(app.status_mark(), StatusMark::Rejected);
}

#[test]
fn while_leading_each_move_of_the_cursor_onto_a_new_tile_is_sent_once() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(4, 2));
    click(&mut app, &world, at(4, 2), Button::Left);
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
    click(&mut app, &world, at(4, 2), Button::Left);
    point(&mut app, &world, at(7, 3));
    let item = object_on(&world, at(4, 2));
    assert_eq!(app.take_commands(), vec![Command::PickUp { item }]);
}

#[test]
fn leading_the_followed_sprite_the_cursor_follows_the_pointer_and_follow_comes_back_after() {
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(4, 2));
    follow(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(0, 0));
    apply(&mut app, &world, Q);
    let commands = app.take_commands();
    let to_the_pointer = Command::MoveCursor { tile: at(0, 0) };
    assert_eq!(commands, vec![Command::TakeHold { sprite }, to_the_pointer]);
    for command in commands {
        world.submit(command.clone());
    }
    tick(&mut app, &mut world);
    point(&mut app, &world, at(8, 5));
    assert_eq!(
        (app.cursor(), app.followed()),
        (at(8, 5), None),
        "Follow waits"
    );
    apply(&mut app, &world, Q);
    tick(&mut app, &mut world);
    let there = world.sprite(sprite).expect("the sprite").pos();
    assert_eq!(
        (app.cursor(), app.followed()),
        (there, Some(sprite)),
        "back on it"
    );
}

#[test]
fn while_leading_a_cursor_following_another_walking_sprite_tells_the_world_where_it_goes() {
    let walker = at(5, 3);
    let wander = ScriptedAction::Wander {
        destination: at(9, 3),
    };
    let mut world = WorldBuilder::grass(10, 6)
        .starters(&[at(1, 1), walker])
        .scripts(&[(walker, wander)])
        .build();
    let mut app = grab_app(&world);
    let id = sprite_on(&world, walker);
    click(&mut app, &world, at(1, 1), Button::Left);
    follow(&mut app, &world, walker);
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
    let start = at(1, 1);
    let wander = ScriptedAction::Wander {
        destination: at(9, 1),
    };
    let mut world = WorldBuilder::grass(10, 6)
        .starters(&[start])
        .scripts(&[(start, wander)])
        .build();
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::SelectNext);
    tick(&mut app, &mut world);
    let now = world.sprites().next().expect("the sprite").pos();
    click(&mut app, &world, now, Button::Left);
    tick(&mut app, &mut world);
    click(&mut app, &world, at(5, 5), Button::Left);
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

/// A field with a ball on (1, 1) and a sprite resting on (4, 2) for a
/// while, so it stays put.
fn resting_sprite_and_ball() -> World {
    let resting = at(4, 2);
    WorldBuilder::grass(10, 6)
        .objects(&[(at(1, 1), "ball")])
        .starters(&[resting])
        .scripts(&[(resting, ScriptedAction::Rest); 5])
        .build()
}

#[test]
fn taking_hold_of_the_followed_sprite_puts_the_cursor_on_the_pointer_at_once() {
    // Design v23 §6.5: Follow waits, so a keyboard click lands where the
    // pointer is.
    let world = resting_sprite_and_ball();
    let mut app = grab_app(&world);
    follow(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(8, 5));
    assert_eq!(app.cursor(), at(4, 2), "following");
    apply(&mut app, &world, Q);
    assert_eq!(app.cursor(), at(8, 5));
}

#[test]
fn holding_an_item_while_following_a_sprite_a_click_puts_it_at_its_feet() {
    // Design v23 change 6: Follow steps aside only for the sprite it's on,
    // so holding a berry picked up with Follow off, the Cursor sits on the
    // followed sprite.
    let mut world = resting_sprite_and_ball();
    let mut app = grab_app(&world);
    click(&mut app, &world, at(1, 1), Button::Left);
    tick(&mut app, &mut world);
    follow(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(8, 5));
    assert_eq!(app.cursor(), at(4, 2), "on the followed sprite");
    apply(&mut app, &world, Q);
    assert_eq!(
        app.take_commands(),
        vec![Command::PutDown { tile: at(4, 2) }]
    );
}

#[test]
fn holding_an_item_while_following_a_sprite_a_mouse_click_anywhere_puts_it_at_its_feet() {
    // Design v26 §6.5: following, a click still reaches the sprite, wherever
    // it lands.
    let mut world = resting_sprite_and_ball();
    let mut app = grab_app(&world);
    click(&mut app, &world, at(1, 1), Button::Left);
    tick(&mut app, &mut world);
    follow(&mut app, &world, at(4, 2));
    click(&mut app, &world, at(8, 5), Button::Left);
    assert_eq!(
        app.take_commands(),
        vec![Command::PutDown { tile: at(4, 2) }]
    );
}

#[test]
fn leading_the_followed_sprite_follow_steps_aside_in_every_mode() {
    let world = resting_sprite_and_ball();
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(4, 2));
    follow(&mut app, &world, at(4, 2));
    apply(&mut app, &world, Q);
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    point(&mut app, &world, at(7, 4));
    assert_eq!((app.cursor(), app.followed()), (at(7, 4), None));
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    assert_eq!(app.followed(), None, "still aside in Select");
    assert_eq!(app.grip(&world), Some(Grip::Leads(sprite)));
}

/// A field 20 wide with a sprite resting on (1, 1), which the app in Grab
/// mode has just taken hold of, and a sprite resting on (12, 3).
fn leading_in_a_wide_field() -> (World, App) {
    let (led, other) = (at(1, 1), at(12, 3));
    let world = WorldBuilder::grass(20, 6)
        .starters(&[led, other])
        .scripts(&[(led, ScriptedAction::Rest); 9])
        .scripts(&[(other, ScriptedAction::Rest); 9])
        .build();
    let mut app = grab_app(&world);
    click(&mut app, &world, led, Button::Left);
    (world, app)
}

#[test]
fn leading_the_cursor_goes_no_further_than_the_leash_and_clicks_land_where_it_is() {
    // Design v23 change 14: 5 tiles in a square from the led sprite, on
    // (1, 1), the leash's end nearest the pointer.
    let (world, mut app) = leading_in_a_wide_field();
    app.take_commands();
    point(&mut app, &world, at(12, 3));
    assert_eq!(app.cursor(), at(6, 3));
    assert_eq!(
        app.take_commands(),
        vec![Command::MoveCursor { tile: at(6, 3) }]
    );
    // A pet clicked on the sprite past the leash lands on the Cursor's tile,
    // where there's no one.
    apply(&mut app, &world, Action::Mode(CursorMode::Train));
    click(&mut app, &world, at(12, 3), Button::Left);
    assert_eq!(app.take_commands(), Vec::new());
    assert_eq!(app.refusal(), Some("No sprite here to pet"));
}

#[test]
fn as_the_led_sprite_catches_up_the_cursor_moves_on_towards_the_pointer() {
    let (mut world, mut app) = leading_in_a_wide_field();
    point(&mut app, &world, at(11, 3));
    assert_eq!(app.cursor(), at(6, 3), "held back");
    for _ in 0..40 {
        tick(&mut app, &mut world);
    }
    assert_eq!(app.cursor(), at(11, 3), "there in the end");
}

#[test]
fn leading_one_sprite_following_another_the_cursor_waits_at_the_leash_s_end_towards_it() {
    // Design v23 change 6: Follow holds, so the led sprite is led towards
    // the followed one.
    let (world, mut app) = leading_in_a_wide_field();
    follow(&mut app, &world, at(12, 3));
    assert_eq!(app.cursor(), at(6, 3));
}

fn escape(app: &mut App, world: &World) {
    apply(app, world, Action::Back);
}

#[test]
fn leading_a_sprite_esc_lets_go_and_the_next_esc_returns_to_select() {
    // Design v33 §6.5: Esc lets go before it leaves the mode.
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    escape(&mut app, &world);
    assert_eq!(app.mode(), CursorMode::Grab, "letting go comes first");
    tick(&mut app, &mut world);
    assert_eq!(world.cursor().leads(), None);
    escape(&mut app, &world);
    assert_eq!(app.mode(), CursorMode::Select);
    assert_eq!(app.take_commands(), Vec::new());
}

#[test]
fn while_paused_a_second_esc_after_a_queued_let_go_returns_to_select_sending_nothing_more() {
    // The grip follows the queue (design v23 §6.5), so the first Esc's
    // `LetGo` already counts before time moves.
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    escape(&mut app, &world);
    escape(&mut app, &world);
    assert_eq!(app.mode(), CursorMode::Select);
    assert_eq!(app.take_commands(), vec![Command::LetGo], "sent once");
}

#[test]
fn holding_an_item_esc_puts_it_down_on_the_cursor_s_tile() {
    // Design v33 §6.5: where the Cursor is, as a click there would. The
    // pointer stays where it was, as with a key pressed.
    let mut world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    let ball = object_on(&world, at(4, 2));
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    point(&mut app, &world, at(7, 4));
    escape(&mut app, &world);
    assert_eq!(
        app.take_commands(),
        vec![Command::PutDown { tile: at(7, 4) }]
    );
    assert_eq!(app.grip(&world), None, "as the queue leaves it");
    world.submit(Command::PutDown { tile: at(7, 4) });
    let events = world.step();
    app.record(&events, &world);
    assert_eq!(world.object_at(at(7, 4)).map(|o| o.id()), Some(ball));
}

#[test]
fn holding_an_item_while_following_a_sprite_esc_puts_it_at_its_feet() {
    // The Cursor sits on the followed sprite (design v26 §6.5), so that's
    // where Esc puts the item down.
    let mut world = resting_sprite_and_ball();
    let mut app = grab_app(&world);
    click(&mut app, &world, at(1, 1), Button::Left);
    tick(&mut app, &mut world);
    follow(&mut app, &world, at(4, 2));
    point(&mut app, &world, at(8, 5));
    escape(&mut app, &world);
    assert_eq!(
        app.take_commands(),
        vec![Command::PutDown { tile: at(4, 2) }]
    );
}

#[test]
fn esc_lets_go_in_every_mode_before_it_asks_to_quit() {
    // What the Cursor holds or leads stays held in other modes (design v23
    // §6.5), so Esc lets go of it there too, first (design v33 §6.5).
    let mut world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    let ball = object_on(&world, at(4, 2));
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    point(&mut app, &world, at(6, 3));
    escape(&mut app, &world);
    assert_eq!(app.screen(), Screen::Normal, "no quit prompt yet");
    assert_eq!(app.grip(&world), None, "as the queue leaves it");
    tick(&mut app, &mut world);
    assert_eq!(world.object_at(at(6, 3)).map(|o| o.id()), Some(ball));
    escape(&mut app, &world);
    assert_eq!(app.screen(), Screen::QuitPrompt);
}

#[test]
fn esc_putting_an_item_down_where_it_cant_go_is_refused_with_why() {
    // As a click there would be (design v33 §6.5).
    let mut world = field(&[(at(4, 2), "ball"), (at(7, 4), "berry_bush")], &[]);
    let mut app = grab_app(&world);
    let ball = object_on(&world, at(4, 2));
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    point(&mut app, &world, at(7, 4));
    escape(&mut app, &world);
    tick(&mut app, &mut world);
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(
        app.refusal(),
        Some("Couldn't put the ball down: a berry bush is there")
    );
    assert_eq!(world.cursor().holds().map(|o| o.id()), Some(ball));
}
