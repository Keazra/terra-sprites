//! Grab mode's commands (design v23 §2.5, §6.5): the Cursor leads sprites
//! and holds items, one thing at a time.

use terra_sim::{
    Command, DataPack, EntityId, EventKind, Map, Outcome, Pos, Scenario, ScriptedAction, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A world drawn from `rows`, with `objects`, and a starter sprite on each
/// of `sprites`, doing `script`, by the tile it starts on.
fn world(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[Pos],
    script: &[(Pos, ScriptedAction)],
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<_>)> = sprites.iter().map(|&pos| (pos, None)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: script,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// The sprite on `pos`.
fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
}

#[test]
fn taking_hold_of_a_sprite_leads_it_and_leaves_it_where_it_stands() {
    let mut world = world(
        &[".....", ".....", "....."],
        &[],
        &[at(2, 1)],
        &[(at(2, 1), ScriptedAction::Rest)],
    );
    let id = sprite_on(&world, at(2, 1));
    world.submit(Command::TakeHold { sprite: id });
    let events = world.step();
    assert!(
        events
            .iter()
            .any(|e| e.kind == EventKind::TookHold { sprite: id }),
        "{events:?}"
    );
    assert_eq!(world.cursor().leads(), Some(id));
    assert_eq!(world.sprite_at(at(2, 1)).map(|s| s.id()), Some(id));
}

#[test]
fn taking_hold_of_a_sprite_pulls_it_away_from_what_it_was_doing() {
    let mut world = world(
        &["........"],
        &[],
        &[at(0, 0)],
        &[(
            at(0, 0),
            ScriptedAction::Wander {
                destination: at(7, 0),
            },
        )],
    );
    let id = sprite_on(&world, at(0, 0));
    world.step();
    world.submit(Command::TakeHold { sprite: id });
    let events = world.step();
    let ended = events.iter().find_map(|e| match e.kind {
        EventKind::ActionEnded {
            id: who, outcome, ..
        } if who == id => Some(outcome),
        _ => None,
    });
    assert_eq!(ended, Some(Outcome::PulledAway), "{events:?}");
}
