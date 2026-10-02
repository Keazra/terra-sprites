//! Throwing and shoving on screen (design v25 §6.5): in Grab mode the right
//! button, or `E`, held down, aims what the Cursor has hold of, grabbing
//! what's under it first if it's empty, by pulling back from the Cursor, and
//! letting go sends it.

use ratatui::layout::{Position, Rect};
use terra_sim::{Command, DataPack, Dir, EntityId, Map, Pos, Scenario, ScriptedAction, World};
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

/// Has the Cursor follow the sprite on `tile`, with a middle click (design
/// v26 §6.5).
fn follow(app: &mut App, world: &World, tile: Pos) {
    apply(app, world, Action::middle_click(cell(tile)));
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
fn a_right_click_with_nothing_held_led_or_there_sends_nothing_and_says_so() {
    // Design v25 §6.5: a bush is rooted to the ground.
    let world = field(&[(at(6, 2), "thornbush")], &[]);
    for (tile, why) in [
        (at(4, 2), "Nothing here to throw or shove"),
        (
            at(6, 2),
            "Can't throw the thornbush: it's rooted to the ground",
        ),
    ] {
        let mut app = grab_app(&world);
        click(&mut app, &world, tile, Button::Right);
        assert_eq!(app.take_commands(), Vec::new());
        assert_eq!(app.status_mark(), StatusMark::Rejected);
        assert_eq!(app.refusal(), Some(why));
        assert!(!app.aiming());
    }
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
    assert_eq!(app.cursor(), at(3, 5), "sent, it's back on the pointer");
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
fn a_shove_goes_the_nearest_of_the_8_directions_to_the_way_pulled_from_the_cursor() {
    // Design v25 §6.5: pressed beside the led sprite, the Cursor goes onto
    // it, and the pull is the pointer's from there. Pulled 4 east and 1
    // south, a shove goes west; 2 east and 1 south is nearer north-west.
    for (to, toward, tiles) in [(at(9, 6), Dir::W, 4), (at(7, 6), Dir::NW, 2)] {
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
    assert_eq!(app.cursor(), at(3, 5), "the Cursor is back on the pointer");
    point(&mut app, &world, at(2, 5));
    assert_eq!(app.cursor(), at(2, 5), "and follows it again");
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
fn following_a_sprite_a_throw_starts_at_its_feet_pulled_from_there() {
    // Design v25 §6.5: following, the Cursor sits on its sprite, wherever
    // the click lands, and the pull is the pointer's from the Cursor.
    let mut world = field(&[(at(2, 2), "berry")], &[at(8, 5)]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(2, 2));
    follow(&mut app, &world, at(8, 5));
    assert!(app.followed().is_some());
    click(&mut app, &world, at(12, 5), Button::Right);
    point(&mut app, &world, at(14, 5));
    let_go(&mut app, &world, at(14, 5));
    let throw = Command::Throw {
        from: at(8, 5),
        toward: Dir::W,
        tiles: 6,
    };
    assert_eq!(app.take_commands(), vec![throw]);
}

#[test]
fn stopping_follow_while_aiming_leaves_the_aim_as_it_was() {
    // Design v26 §6.5: `F` works in every mode, but while aiming the
    // Cursor stays where aiming began, and the pointer pulls.
    let mut world = field(&[(at(2, 2), "berry")], &[at(8, 5)]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(2, 2));
    follow(&mut app, &world, at(8, 5));
    click(&mut app, &world, at(12, 5), Button::Right);
    point(&mut app, &world, at(14, 5));
    apply(&mut app, &world, Action::Follow { at: None });
    assert_eq!(app.followed(), None);
    assert_eq!(app.cursor(), at(8, 5), "still where aiming began");
    let_go(&mut app, &world, at(14, 5));
    let throw = Command::Throw {
        from: at(8, 5),
        toward: Dir::W,
        tiles: 6,
    };
    assert_eq!(app.take_commands(), vec![throw]);
}

#[test]
fn a_queued_throw_leaves_the_marks_empty_and_a_refused_one_says_why() {
    // Design v25 §6.5: the marks follow the queue; a throw from where the
    // ball can't go is refused as putting it down there would be.
    let mut world = field(&[(at(5, 5), "ball"), (at(9, 5), "berry")], &[]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(5, 5));
    point(&mut app, &world, at(9, 5));
    click(&mut app, &world, at(9, 5), Button::Right);
    point(&mut app, &world, at(7, 5));
    let_go(&mut app, &world, at(7, 5));
    assert_eq!(
        app.status_marks(&world),
        [StatusMark::Grab, StatusMark::Empty],
        "queued"
    );
    tick(&mut app, &mut world);
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(
        app.refusal(),
        Some("Couldn't throw the ball: a berry is there")
    );
    assert!(world.cursor().holds().is_some(), "still held");
}

/// What the selected sprite was seen to go through, newest first, once the
/// Cursor took hold of the sprite on (5, 5) and shoved it east 3 tiles,
/// in a field with `objects` and another sprite on each of `others`.
fn shove_seen(objects: &[(Pos, &str)], others: &[Pos]) -> Vec<String> {
    let mut sprites = vec![(at(5, 5), None)];
    sprites.extend(others.iter().map(|&pos| (pos, None)));
    // Resting whenever they choose, so they stay where they are.
    let rests: Vec<(Pos, ScriptedAction)> = sprites
        .iter()
        .flat_map(|&(pos, _)| [(pos, ScriptedAction::Rest); 20])
        .collect();
    let map = Map::from_ascii(&["...................."; 12], &pack()).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &rests,
    };
    let mut world = World::from_scenario(scenario, pack(), 1).expect("valid scenario");
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::SelectNext);
    lead(&mut app, &mut world, at(5, 5));
    point(&mut app, &world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(2, 5));
    let_go(&mut app, &world, at(2, 5));
    for _ in 0..4 {
        tick(&mut app, &mut world);
    }
    app.observed().take(2).map(|o| o.line.clone()).collect()
}

#[test]
fn the_observed_list_tells_of_a_shove_out_of_nowhere_and_what_it_crashed_into() {
    // Design v25 §6.1: let go with a push, it was pulled along, then shoved.
    let pulled = "Was pulled along out of nowhere";
    assert_eq!(shove_seen(&[], &[]), ["Was shoved out of nowhere", pulled]);
    let thorns = [(at(8, 5), "thornbush")];
    assert_eq!(
        shove_seen(&thorns, &[]),
        [
            "Was shoved out of nowhere, into a thornbush, and got hurt",
            pulled
        ]
    );
    let seen = shove_seen(&[], &[at(8, 5)]);
    assert!(
        seen[0].starts_with("Was shoved out of nowhere, into Sprite #"),
        "{seen:?}"
    );
}

#[test]
fn a_right_press_on_a_sprite_with_the_cursor_empty_takes_hold_of_it_and_aims_a_shove() {
    // Design v25 §6.5, from the owner's build check: hold the right button
    // over a thing to grab it and aim, pull, and let go.
    let mut world = field(&[], &[at(5, 5)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    assert!(app.aiming());
    point(&mut app, &world, at(3, 5));
    let_go(&mut app, &world, at(3, 5));
    let shove = Command::Shove {
        toward: Dir::E,
        tiles: 2,
    };
    assert_eq!(sent(&mut app), vec![Command::TakeHold { sprite }, shove]);
    // Both apply at the next tick, and it slides a tile of the 2.
    world.submit(Command::TakeHold { sprite });
    world.submit(shove);
    world.step();
    let slide = world.sprite(sprite).and_then(|s| s.slide());
    assert_eq!(slide, Some((Dir::E, 1)));
}

#[test]
fn a_right_press_on_an_item_with_the_cursor_empty_picks_it_up_and_throws_it_from_there() {
    // Design v25 §6.5. Both apply at the next tick, and the ball rolls a
    // tile from where it lay.
    let mut world = field(&[(at(5, 5), "ball")], &[]);
    let mut app = grab_app(&world);
    let ball = object_on(&world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(5, 7));
    let_go(&mut app, &world, at(5, 7));
    let throw = Command::Throw {
        from: at(5, 5),
        toward: Dir::N,
        tiles: 2,
    };
    assert_eq!(
        app.take_commands(),
        vec![Command::PickUp { item: ball }, throw]
    );
    world.submit(Command::PickUp { item: ball });
    world.submit(throw);
    world.step();
    assert_eq!(world.object_at(at(5, 4)).map(|o| o.id()), Some(ball));
}

#[test]
fn leading_a_right_press_puts_the_cursor_on_the_sprite_which_stands_still_while_aimed() {
    // Design v25 §6.5: a led sprite walks towards the Cursor, so with the
    // Cursor on it, it stands still, though it was heading for the pointer.
    let mut world = field(&[], &[at(5, 5)]);
    let mut app = grab_app(&world);
    let sprite = lead(&mut app, &mut world, at(5, 5));
    point(&mut app, &world, at(9, 5));
    click(&mut app, &world, at(9, 5), Button::Right);
    assert_eq!(app.cursor(), at(5, 5), "on the sprite");
    for _ in 0..10 {
        tick(&mut app, &mut world);
    }
    let still = world.sprite(sprite).map(|s| s.pos());
    assert_eq!(still, Some(at(5, 5)), "it stood still");
    assert_eq!(app.cursor(), at(5, 5));
    assert!(app.aiming());
}

#[test]
fn a_sprite_aimed_while_it_slides_slides_on_and_the_cursor_rides_along() {
    // Design v25 §6.5: shoved 3 tiles east, it's taken hold of again a
    // tile into its slide, and stands still where the slide ends.
    let mut world = field(&[], &[at(5, 5)]);
    let mut app = grab_app(&world);
    let sprite = lead(&mut app, &mut world, at(5, 5));
    point(&mut app, &world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(2, 5));
    let_go(&mut app, &world, at(2, 5));
    tick(&mut app, &mut world);
    assert_eq!(
        world.sprite(sprite).and_then(|s| s.slide()),
        Some((Dir::E, 2))
    );
    point(&mut app, &world, at(6, 5));
    click(&mut app, &world, at(6, 5), Button::Right);
    for _ in 0..8 {
        tick(&mut app, &mut world);
    }
    let still = world.sprite(sprite).map(|s| s.pos());
    assert_eq!(still, Some(at(8, 5)), "where its slide ended");
    assert_eq!(app.cursor(), at(8, 5), "the Cursor rode along");
    assert!(app.aiming());
}

#[test]
fn holding_an_item_a_right_press_on_a_sprite_aims_the_item() {
    // Design v25 §6.5: the press aims what the Cursor has hold of, whatever
    // is under the pointer.
    let mut world = field(&[(at(2, 2), "ball")], &[at(5, 5)]);
    let mut app = grab_app(&world);
    hold(&mut app, &mut world, at(2, 2));
    point(&mut app, &world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(5, 6));
    let_go(&mut app, &world, at(5, 6));
    let throw = Command::Throw {
        from: at(5, 5),
        toward: Dir::N,
        tiles: 1,
    };
    assert_eq!(app.take_commands(), vec![throw]);
}

#[test]
fn cancelling_an_aim_the_press_grabbed_for_leaves_it_in_the_cursor_s_grip() {
    // Design v25 §6.5: Esc, or letting go with the pointer on the Cursor.
    for pulled in [true, false] {
        let mut world = field(&[], &[at(5, 5)]);
        let mut app = grab_app(&world);
        let sprite = sprite_on(&world, at(5, 5));
        click(&mut app, &world, at(5, 5), Button::Right);
        if pulled {
            point(&mut app, &world, at(3, 5));
            apply(&mut app, &world, Action::Back);
        } else {
            let_go(&mut app, &world, at(5, 5));
        }
        assert!(!app.aiming());
        tick(&mut app, &mut world);
        assert_eq!(world.cursor().leads(), Some(sprite), "pulled: {pulled}");
    }
}

#[test]
fn following_with_the_cursor_empty_a_right_press_anywhere_aims_the_followed_sprite() {
    // Design v25 §6.5: the press grabs as a left click would.
    let world = field(&[], &[at(8, 5)]);
    let mut app = grab_app(&world);
    let sprite = sprite_on(&world, at(8, 5));
    follow(&mut app, &world, at(8, 5));
    click(&mut app, &world, at(12, 5), Button::Right);
    point(&mut app, &world, at(10, 5));
    let_go(&mut app, &world, at(10, 5));
    let shove = Command::Shove {
        toward: Dir::W,
        tiles: 2,
    };
    assert_eq!(sent(&mut app), vec![Command::TakeHold { sprite }, shove]);
}

#[test]
fn the_aim_is_cancelled_when_the_led_sprite_dies() {
    // Design v25 §6.5. Newborns here start with no energy and starve fast,
    // dying a few ticks in.
    let text = DataPack::builtin_sources()
        .iter()
        .find(|(path, _)| *path == "physiology.ron")
        .map(|(_, text)| {
            text.replace("newborn: (energy: 1.0,", "newborn: (energy: 0.0,")
                .replace("starvation: 0.0011,", "starvation: 0.3,")
        })
        .expect("physiology.ron");
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| {
            (
                path,
                if path == "physiology.ron" {
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
        objects: &[],
        sprites: &[(at(5, 5), None)],
        scripted: &[(at(5, 5), ScriptedAction::Rest)],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
    let mut app = grab_app(&world);
    lead(&mut app, &mut world, at(5, 5));
    click(&mut app, &world, at(5, 5), Button::Right);
    point(&mut app, &world, at(3, 5));
    for _ in 0..5 {
        tick(&mut app, &mut world);
    }
    assert!(world.sprites().next().is_none(), "it starved");
    assert!(!app.aiming());
    let_go(&mut app, &world, at(3, 5));
    assert_eq!(sent(&mut app), Vec::new());
}
