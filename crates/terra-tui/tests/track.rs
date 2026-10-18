//! Track (`T`, design v21 §6.1): the view follows the selected sprite.

mod common;
use common::{LONE, LONE_MOTHER, QUICK, WorldBuilder, at, grow, map_app, pack, pointer_on};

use ratatui::layout::Rect;
use terra_sim::{Map, Pos, Scenario, ScriptedAction, World};
use terra_tui::app::{App, Origin, Selection};
use terra_tui::input::Action;

/// An app on `world` showing 20×10 tiles of it, with no panels.
fn small_view(world: &World) -> App {
    map_app(world, Rect::new(1, 2, 20, 10))
}

/// A 60×40 field with a starter sprite at (50, 30) wandering to (42, 30),
/// within its sight, and one at (5, 5), and an app showing 20×10 tiles of it.
fn field() -> (World, App) {
    let wander = ScriptedAction::Wander {
        destination: at(42, 30),
    };
    let world = WorldBuilder::grass(60, 40)
        .sprites(&[at(50, 30), at(5, 5)])
        .scripts(&[(at(50, 30), wander)])
        .build();
    let app = small_view(&world);
    (world, app)
}

/// Selects the sprite on `tile`, with `Tab`.
fn select(app: &mut App, world: &World, tile: Pos) {
    let id = world.sprite_at(tile).expect("a sprite").id();
    common::select(app, world, id);
}

/// Whether the viewport has `tile` at its centre.
fn centred_on(app: &App, tile: Pos) -> bool {
    let (x, y) = (i32::from(tile.x), i32::from(tile.y));
    app.viewport()
        == Origin {
            x: x - 10,
            y: y - 5,
        }
}

#[test]
fn t_has_the_view_follow_the_selected_sprite_as_it_walks() {
    let (mut world, mut app) = field();
    select(&mut app, &world, at(50, 30));
    app.apply(Action::Scroll { dx: -40, dy: -30 }, &world);
    app.apply(Action::Track, &world);
    assert!(app.tracking());
    assert!(centred_on(&app, at(50, 30)), "{:?}", app.viewport());
    assert_eq!(app.notice(), Some("Tracking Sprite #1"));
    let walker = |world: &World| world.sprites().next().expect("the walker").pos();
    for _ in 0..30 {
        let events = world.step();
        app.record(&events, &world);
        if walker(&world) == at(42, 30) {
            break;
        }
    }
    let now = walker(&world);
    assert_eq!(now, at(42, 30), "it walked");
    assert!(centred_on(&app, now), "{:?} for {now:?}", app.viewport());
}

#[test]
fn t_again_stops_tracking_and_so_does_scrolling() {
    let (world, mut app) = field();
    select(&mut app, &world, at(50, 30));
    app.apply(Action::Track, &world);
    app.apply(Action::Track, &world);
    assert!(!app.tracking());
    assert_eq!(app.notice(), Some("Stopped tracking"));

    app.apply(Action::Track, &world);
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    assert!(!app.tracking(), "scrolling by hand takes the view back");
    assert!(centred_on(&app, at(49, 30)), "scrolled one tile left");
}

#[test]
fn tracking_follows_whichever_sprite_is_selected() {
    let (world, mut app) = field();
    select(&mut app, &world, at(50, 30));
    app.apply(Action::Track, &world);
    select(&mut app, &world, at(5, 5));
    assert!(app.tracking());
    // Centred, with the view a quarter of its width past the wall (M2
    // design §11.1).
    assert!(centred_on(&app, at(5, 5)), "{:?}", app.viewport());
}

#[test]
fn track_stops_a_quarter_of_the_view_past_the_wall() {
    // M2 design §11.1: a sprite in the corner, at (1, 1), would be centred
    // from (-9, -4); the 20×10 view goes only 5 tiles past the wall
    // sideways, and 2 up.
    let row = ".".repeat(60);
    let map = Map::from_ascii(&vec![row.as_str(); 40], &pack()).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(at(1, 1), None)],
        scripted: &[],
        newborns: &[],
    };
    let world = World::from_scenario(scenario, pack(), 1).expect("valid scenario");
    let mut app = small_view(&world);
    select(&mut app, &world, at(1, 1));
    app.apply(Action::Track, &world);
    assert_eq!(app.viewport(), Origin { x: -5, y: -2 });
}

#[test]
fn t_tracks_an_egg_where_it_lies() {
    // M2 design §10.7: a mother who lays alone at once, on the field, and
    // her egg, clicked on to select it.
    let [quick, eggs] = QUICK;
    let world = grow(
        &[quick, eggs, LONE],
        (60, 40),
        &[(at(50, 30), LONE_MOTHER)],
        1,
    );
    let egg = world.eggs().next().expect("an egg");
    let (id, lies) = (egg.id(), egg.pos().expect("on the map"));
    let mut app = small_view(&world);
    // The view scrolled to the egg, and the pointer on it.
    let from = app.viewport();
    let (dx, dy) = (
        i32::from(lies.x) - from.x - 10,
        i32::from(lies.y) - from.y - 5,
    );
    app.apply(Action::Scroll { dx, dy }, &world);
    app.apply(Action::left_click(pointer_on(&app, lies)), &world);
    assert_eq!(app.selection(), Some(Selection::Living(id)));
    app.apply(Action::Scroll { dx: -40, dy: -25 }, &world);
    assert!(!centred_on(&app, lies), "scrolled away");
    app.apply(Action::Track, &world);
    assert!(app.tracking());
    assert_eq!(
        app.notice(),
        Some(format!("Tracking Sprite #{}", id.0).as_str())
    );
    assert!(centred_on(&app, lies), "{:?}", app.viewport());
}

#[test]
fn t_with_no_sprite_selected_says_so() {
    let (world, mut app) = field();
    app.apply(Action::Track, &world);
    assert!(!app.tracking());
    assert_eq!(app.refusal(), Some("Select a sprite to track it"));
}
