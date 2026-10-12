//! Track (`T`, design v21 §6.1): the view follows the selected sprite.

use ratatui::layout::Rect;
use terra_sim::{DataPack, Map, Pos, Scenario, ScriptedAction, World};
use terra_tui::app::{App, Areas, Origin};
use terra_tui::input::Action;
use terra_tui::theme::Theme;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A 60×40 field with a starter sprite at (50, 30) wandering to (42, 30),
/// within its sight, and one at (5, 5), and an app showing 20×10 tiles of it.
fn field() -> (World, App) {
    let row = ".".repeat(60);
    let map = Map::from_ascii(&vec![row.as_str(); 40], &pack()).expect("valid drawing");
    let sprites = [(at(50, 30), None), (at(5, 5), None)];
    let scripted = [(
        at(50, 30),
        ScriptedAction::Wander {
            destination: at(42, 30),
        },
    )];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &scripted,
        newborns: &[],
    };
    let world = World::from_scenario(scenario, pack(), 1).expect("valid scenario");
    let areas = Areas {
        tiles: Rect::new(1, 2, 20, 10),
        inspector: None,
        event_log: None,
        overlay: None,
    };
    let app = App::new(world.map(), Theme::cp437(), 1, areas);
    (world, app)
}

/// Selects the sprite on `tile`, with `Tab`.
fn select(app: &mut App, world: &World, tile: Pos) {
    let id = world.sprite_at(tile).expect("a sprite").id();
    while app.selection().map(|s| s.id()) != Some(id) {
        app.apply(Action::SelectNext, world);
    }
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
    let areas = Areas {
        tiles: Rect::new(1, 2, 20, 10),
        inspector: None,
        event_log: None,
        overlay: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), 1, areas);
    select(&mut app, &world, at(1, 1));
    app.apply(Action::Track, &world);
    assert_eq!(app.viewport(), Origin { x: -5, y: -2 });
}

#[test]
fn t_with_no_sprite_selected_says_so() {
    let (world, mut app) = field();
    app.apply(Action::Track, &world);
    assert!(!app.tracking());
    assert_eq!(app.refusal(), Some("Select a sprite to track it"));
}
