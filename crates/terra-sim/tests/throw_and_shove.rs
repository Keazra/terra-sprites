//! Grab mode's right click (design v25 §2.5, §3.5.4, §6.5): the Cursor
//! throws a held item, which rolls, and shoves a led sprite, which slides
//! and may crash into what stops it.

use terra_sim::{
    Command, DataPack, Dir, EntityId, Event, EventKind, Map, Pos, Rejection, Scenario,
    ScriptedAction, World,
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

/// The object on `pos`.
fn object_on(world: &World, pos: Pos) -> Option<EntityId> {
    world.object_at(pos).map(|o| o.id())
}

/// A world of `rows` and `objects` whose Cursor holds the item on `from`.
fn holding(rows: &[&str], objects: &[(Pos, &str)], from: Pos) -> (World, EntityId) {
    let mut world = world(rows, objects, &[], &[]);
    let item = object_on(&world, from).expect("an item there");
    world.submit(Command::PickUp { item });
    world.step();
    assert_eq!(world.cursor().holds().map(|h| h.id()), Some(item));
    (world, item)
}

/// The reason the world gave for refusing `command`, if it did.
fn refused(events: &[Event], command: Command) -> Option<Rejection> {
    events.iter().find_map(|e| match e.kind {
        EventKind::CommandRejected { command: c, reason } if c == command => Some(reason),
        _ => None,
    })
}

#[test]
fn a_thrown_item_is_put_down_where_the_throw_starts_and_rolls_from_that_tick() {
    let (mut world, ball) = holding(&["..........."], &[(at(0, 0), "ball")], at(0, 0));
    world.submit(Command::Throw {
        from: at(2, 0),
        toward: Dir::E,
        tiles: 3,
    });
    let events = world.step();
    let threw = EventKind::Threw {
        item: ball,
        object_type: "ball".into(),
    };
    assert!(events.iter().any(|e| e.kind == threw), "{events:?}");
    assert!(world.cursor().holds().is_none());
    assert_eq!(object_on(&world, at(3, 0)), Some(ball), "its first tile");
    world.step();
    world.step();
    assert_eq!(object_on(&world, at(5, 0)), Some(ball), "three tiles");
    world.step();
    assert_eq!(object_on(&world, at(5, 0)), Some(ball), "then at rest");
}

#[test]
fn a_throw_goes_at_least_a_tile_and_no_further_than_its_size_allows() {
    // A ball is medium, and the Cursor sends a medium thing up to 6 tiles
    // (design v25 §3.5.4, Appendix B).
    for (tiles, lands) in [(0, at(1, 0)), (6, at(6, 0)), (20, at(6, 0))] {
        let lane = ["...................."];
        let (mut world, ball) = holding(&lane, &[(at(19, 0), "ball")], at(19, 0));
        world.submit(Command::Throw {
            from: at(0, 0),
            toward: Dir::E,
            tiles,
        });
        for _ in 0..10 {
            world.step();
        }
        assert_eq!(object_on(&world, lands), Some(ball), "thrown {tiles}");
    }
}

#[test]
fn a_throw_is_refused_where_the_item_cant_go_and_with_nothing_held() {
    let rows = ["..#..", "....."];
    let (mut world, ball) = holding(&rows, &[(at(0, 0), "ball"), (at(1, 0), "berry")], at(0, 0));
    for from in [at(1, 0), at(2, 0)] {
        let command = Command::Throw {
            from,
            toward: Dir::E,
            tiles: 3,
        };
        world.submit(command);
        let events = world.step();
        assert!(
            matches!(refused(&events, command), Some(Rejection::InTheWay { .. })),
            "{events:?}"
        );
        assert_eq!(
            world.cursor().holds().map(|h| h.id()),
            Some(ball),
            "still held"
        );
    }
    let mut empty = world_without_hold();
    let command = Command::Throw {
        from: at(1, 0),
        toward: Dir::E,
        tiles: 3,
    };
    empty.submit(command);
    let events = empty.step();
    assert_eq!(refused(&events, command), Some(Rejection::NotHolding));
}

/// A plain world whose Cursor holds nothing.
fn world_without_hold() -> World {
    world(&["....."], &[], &[], &[])
}
