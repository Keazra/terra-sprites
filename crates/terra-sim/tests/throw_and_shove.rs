//! Grab mode's right click (design v25 §2.5, §3.5.4, §6.5): the Cursor
//! throws a held item, which rolls, and shoves a led sprite, which slides
//! and may crash into what stops it.

use terra_sim::{
    Command, DataPack, DeathCause, Dir, EntityId, Event, EventKind, Map, Pos, Rejection, Scenario,
    ScriptedAction, Thing, World,
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
    world_in(builtin(), rows, objects, sprites, script)
}

/// A world like `world`'s, made with `data`.
fn world_in(
    data: DataPack,
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[Pos],
    script: &[(Pos, ScriptedAction)],
) -> World {
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

/// The sprite on `pos`.
fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
}

/// A world of `rows` and `objects` whose Cursor leads the sprite on `from`,
/// which rests whenever it chooses.
fn leading(rows: &[&str], objects: &[(Pos, &str)], from: Pos) -> (World, EntityId) {
    let rests = [(from, ScriptedAction::Rest); 20];
    let mut world = world(rows, objects, &[from], &rests);
    let sprite = sprite_on(&world, from);
    world.submit(Command::TakeHold { sprite });
    world.step();
    assert_eq!(world.cursor().leads(), Some(sprite));
    (world, sprite)
}

/// Where sprite `id` stands.
fn where_is(world: &World, id: EntityId) -> Pos {
    world.sprite(id).expect("the sprite").pos()
}

#[test]
fn a_shoved_sprite_is_let_go_and_slides_a_tile_a_tick_up_to_three() {
    let (mut world, sprite) = leading(&["..........."], &[], at(1, 0));
    world.submit(Command::Shove {
        toward: Dir::E,
        tiles: 10,
    });
    let events = world.step();
    let shoved = EventKind::Shoved { sprite };
    assert!(events.iter().any(|e| e.kind == shoved), "{events:?}");
    assert_eq!(world.cursor().leads(), None);
    assert_eq!(where_is(&world, sprite), at(2, 0), "a tile, from that tick");
    world.step();
    assert_eq!(where_is(&world, sprite), at(3, 0));
    world.step();
    assert_eq!(where_is(&world, sprite), at(4, 0), "a large thing goes 3");
    world.step();
    assert_eq!(where_is(&world, sprite), at(4, 0), "and rests there");
}

#[test]
fn a_slide_moves_after_the_rolling_items_so_a_ball_gets_there_first() {
    // Design v25 §2.4: at step 2 items roll, then sprites slide. The ball
    // rolls onto the tile first, and the sprite slides onto it, standing on
    // the ball; were it the other way round, the ball would bounce off it.
    let rests = [(at(0, 0), ScriptedAction::Rest); 20];
    let mut world = world(&["......"], &[(at(5, 0), "ball")], &[at(0, 0)], &rests);
    let (sprite, ball) = (
        sprite_on(&world, at(0, 0)),
        object_on(&world, at(5, 0)).unwrap(),
    );
    world.submit(Command::PickUp { item: ball });
    world.step();
    world.submit(Command::Throw {
        from: at(3, 0),
        toward: Dir::W,
        tiles: 2,
    });
    world.submit(Command::TakeHold { sprite });
    world.step();
    assert_eq!(object_on(&world, at(2, 0)), Some(ball), "a tile to go");
    world.submit(Command::Shove {
        toward: Dir::E,
        tiles: 1,
    });
    world.step();
    assert_eq!(object_on(&world, at(1, 0)), Some(ball));
    assert_eq!(where_is(&world, sprite), at(1, 0), "on the ball");
}

/// Whether `events` hold sprite `id` starting an action.
fn started(events: &[Event], id: EntityId) -> bool {
    events
        .iter()
        .any(|e| matches!(e.kind, EventKind::ActionStarted { id: who, .. } if who == id))
}

#[test]
fn a_sliding_sprite_chooses_nothing_until_its_slide_ends_and_then_chooses_at_once() {
    // Design v25 §2.4: its body carries on, but it decides nothing while it
    // slides; past its last tile, it chooses at that tick's step 5.
    let (mut world, sprite) = leading(&["..........."], &[], at(1, 0));
    let energy = |world: &World| world.sprite(sprite).unwrap().chemical("energy").unwrap();
    let before = energy(&world);
    world.submit(Command::Shove {
        toward: Dir::E,
        tiles: 3,
    });
    for tick in 1..=2 {
        let events = world.step();
        assert!(!started(&events, sprite), "tick {tick} of the slide");
    }
    assert!(energy(&world) < before, "its body carries on");
    let events = world.step();
    assert_eq!(where_is(&world, sprite), at(4, 0));
    assert!(started(&events, sprite), "it chooses as the slide ends");
}

/// A world of `rows`, `objects` and resting sprites on `sprites`, in which the
/// Cursor takes hold of the sprite on `from` and shoves it `tiles` tiles
/// `toward`; then four ticks more. Returns the world, the shoved sprite, and
/// the events from the shove on.
fn shoved(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[Pos],
    from: Pos,
    toward: Dir,
    tiles: u16,
) -> (World, EntityId, Vec<Event>) {
    shoved_in(builtin(), rows, objects, sprites, from, toward, tiles)
}

/// What `shoved` does, in a world made with `data`.
fn shoved_in(
    data: DataPack,
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[Pos],
    from: Pos,
    toward: Dir,
    tiles: u16,
) -> (World, EntityId, Vec<Event>) {
    let rests: Vec<(Pos, ScriptedAction)> = sprites
        .iter()
        .flat_map(|&pos| [(pos, ScriptedAction::Rest); 20])
        .collect();
    let mut world = world_in(data, rows, objects, sprites, &rests);
    let sprite = sprite_on(&world, from);
    world.submit(Command::TakeHold { sprite });
    world.step();
    world.submit(Command::Shove { toward, tiles });
    let mut events = Vec::new();
    for _ in 0..4 {
        events.extend(world.step());
    }
    (world, sprite, events)
}

/// What sprite `id` crashed into in `events`, and whether it hurt.
fn crashes(events: &[Event], id: EntityId) -> Vec<(Thing, bool)> {
    events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Crashed { sprite, into, hurt } if *sprite == id => {
                Some((into.clone(), *hurt))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn rock_deep_water_and_the_wall_stop_a_slide_without_a_crash() {
    for (rows, stops) in [
        (["...#.", "....."], at(2, 0)),
        (["...=.", "....."], at(2, 0)),
        (["...", "..."], at(2, 0)),
    ] {
        let (world, sprite, events) = shoved(&rows, &[], &[at(1, 0)], at(1, 0), Dir::E, 3);
        assert_eq!(where_is(&world, sprite), stops, "{rows:?}");
        assert_eq!(crashes(&events, sprite), [], "{rows:?}");
    }
}

#[test]
fn a_sprite_or_a_bush_ahead_stops_a_slide_with_a_crash() {
    let lane = ["......", "......"];
    let bush = [(at(3, 0), "berry_bush")];
    let (world, sprite, events) = shoved(&lane, &bush, &[at(1, 0)], at(1, 0), Dir::E, 3);
    assert_eq!(where_is(&world, sprite), at(2, 0));
    let into_bush = (Thing::ObjectType("berry_bush".into()), false);
    assert_eq!(crashes(&events, sprite), [into_bush]);

    let (world, sprite, events) = shoved(&lane, &[], &[at(1, 0), at(3, 0)], at(1, 0), Dir::E, 3);
    let other = sprite_on(&world, at(3, 0));
    assert_eq!(where_is(&world, sprite), at(2, 0));
    assert_eq!(crashes(&events, sprite), [(Thing::Sprite(other), false)]);
}

#[test]
fn a_slide_stopped_at_a_corner_crashes_into_the_bush_beside_it_east_or_west_first() {
    // Sliding NE from (1, 2), the corner's sides are (2, 2), east, and (1, 1),
    // north (design v25 §3.5.4).
    let field = [".....", ".....", ".....", "....."];
    let rocky = [".....", ".#...", ".....", "....."];
    let corner = |rows: &[&str], objects: &[(Pos, &str)]| {
        let (world, sprite, events) = shoved(rows, objects, &[at(1, 2)], at(1, 2), Dir::NE, 3);
        assert_eq!(where_is(&world, sprite), at(1, 2), "{objects:?}");
        crashes(&events, sprite)
    };
    let into = |name: &str, hurt| (Thing::ObjectType(name.into()), hurt);
    let north = [(at(1, 1), "berry_bush")];
    assert_eq!(corner(&field, &north), [into("berry_bush", false)]);
    let both = [(at(1, 1), "berry_bush"), (at(2, 2), "thornbush")];
    assert_eq!(
        corner(&field, &both),
        [into("thornbush", true)],
        "east first"
    );
    assert_eq!(corner(&rocky, &[]), [], "rock is no crash");
}

#[test]
fn a_shove_with_no_sprite_led_is_refused() {
    let (mut world, _) = holding(&["....."], &[(at(0, 0), "ball")], at(0, 0));
    let command = Command::Shove {
        toward: Dir::E,
        tiles: 3,
    };
    world.submit(command);
    let events = world.step();
    assert_eq!(refused(&events, command), Some(Rejection::NotLeading));
    assert!(world.cursor().holds().is_some(), "the ball is still held");
}

#[test]
fn a_sprite_taken_hold_of_mid_slide_slides_on_to_the_end_and_then_follows() {
    // Design v25 §6.5: taking hold of a sprite doesn't lift it, so it can't
    // end the slide.
    let (mut world, sprite) = leading(&["..........", ".........."], &[], at(1, 0));
    world.submit(Command::Shove {
        toward: Dir::E,
        tiles: 3,
    });
    world.step();
    world.submit(Command::TakeHold { sprite });
    // Behind it, so walking after the Cursor would undo the slide.
    world.submit(Command::MoveCursor { tile: at(0, 1) });
    world.step();
    assert_eq!(world.cursor().leads(), Some(sprite));
    assert_eq!(where_is(&world, sprite), at(3, 0), "still sliding");
    world.step();
    assert_eq!(where_is(&world, sprite), at(4, 0), "to the end");
    for _ in 0..20 {
        world.step();
    }
    assert_eq!(where_is(&world, sprite), at(0, 1), "then after the Cursor");
}

/// The built-in pack with `file` changed: each `(from, to)` replaced.
fn builtin_changing(file: &str, changes: &[(&str, &str)]) -> DataPack {
    let sources: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let mut text = text.to_string();
            if path == file {
                for &(from, to) in changes {
                    assert!(text.contains(from), "{from:?} is in {file}");
                    text = text.replace(from, to);
                }
            }
            (path, text)
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    DataPack::from_sources(&borrowed).expect("the changed pack is valid")
}

#[test]
fn a_crash_runs_the_thing_s_tags_so_a_thornbush_pricks_and_can_kill() {
    // Design v25 §3.5.4, v23 §3.5.6: a crash is a contact.
    let lane = ["......", "......"];
    let thorns = [(at(3, 0), "thornbush")];
    let (world, sprite, events) = shoved(&lane, &thorns, &[at(1, 0)], at(1, 0), Dir::E, 3);
    let thornbush = Thing::ObjectType("thornbush".into());
    assert_eq!(crashes(&events, sprite), [(thornbush, true)]);
    let injury = world.sprite(sprite).unwrap().chemical("injury").unwrap();
    assert!(
        injury > 0.029,
        "Thorny's crash, 0.03, less healing: {injury}"
    );

    let crash = r#"Crash: [Inject(Actor, "injury", 0.03)"#;
    let deadly = builtin_changing(
        "tags.ron",
        &[(crash, r#"Crash: [Inject(Actor, "injury", 1.0)"#)],
    );
    let (world, sprite, events) =
        shoved_in(deadly, &lane, &thorns, &[at(1, 0)], at(1, 0), Dir::E, 3);
    let died = events.iter().find_map(|e| match e.kind {
        EventKind::Died { id, cause, .. } if id == sprite => Some(cause),
        _ => None,
    });
    // The thornbush's stable object type ID.
    assert_eq!(died, Some(DeathCause::HurtBy(3)));
    assert_eq!(world.data().object_type_name(3), Some("thornbush"));
}

#[test]
fn a_sprite_crashed_into_feels_nothing() {
    // Design v25 §3.5.4: as a sprite a ball bounces off feels nothing.
    let lane = ["......", "......"];
    let (world, sprite, _) = shoved(&lane, &[], &[at(1, 0), at(3, 0)], at(1, 0), Dir::E, 3);
    let other = world.sprite_at(at(3, 0)).expect("the other sprite");
    assert_eq!(other.chemical("injury"), Some(0.0));
    assert_eq!(world.sprite(sprite).unwrap().chemical("injury"), Some(0.0));
}

#[test]
fn a_slide_passes_over_an_item_and_carries_on() {
    // Design v25 §3.5.4: an item doesn't stop it; a sprite can stand on one.
    let lane = ["......", "......"];
    let ball = [(at(3, 0), "ball")];
    let (world, sprite, events) = shoved(&lane, &ball, &[at(1, 0)], at(1, 0), Dir::E, 3);
    assert_eq!(where_is(&world, sprite), at(4, 0));
    assert_eq!(crashes(&events, sprite), []);
    assert!(
        world.object_at(at(3, 0)).is_some(),
        "the ball is where it was"
    );
}

#[test]
fn a_sliding_sprite_shoved_again_starts_a_fresh_slide() {
    // Design v25 §3.5.4: as a push on an item already rolling starts a
    // fresh roll (§3.5.2).
    let (mut world, sprite) = leading(&["........", "........"], &[], at(1, 0));
    world.submit(Command::Shove {
        toward: Dir::E,
        tiles: 3,
    });
    world.step();
    world.submit(Command::TakeHold { sprite });
    world.step();
    assert_eq!(where_is(&world, sprite), at(3, 0), "sliding still");
    world.submit(Command::Shove {
        toward: Dir::W,
        tiles: 2,
    });
    for _ in 0..4 {
        world.step();
    }
    assert_eq!(where_is(&world, sprite), at(1, 0), "2 back west, no more");
}

#[test]
fn a_diagonal_slide_meets_the_corner_before_the_tile_ahead() {
    // Design v25 §3.5.4: sliding NE from (1, 2), it passes the corner, (2, 2)
    // to the east and (1, 1) to the north, before it can reach (2, 1). So a
    // bush at the corner is what it crashes into, though a sprite or rock is
    // ahead; and rock at the corner stops it with no crash at all.
    let field = [".....", ".....", ".....", "....."];
    let rock_ahead = [".....", "..#..", ".....", "....."];
    let rocky_corner = [".....", ".#...", ".....", "....."];
    let berry_bush = (Thing::ObjectType("berry_bush".into()), false);
    let bush_north = [(at(1, 1), "berry_bush")];
    let (me, ahead) = (at(1, 2), at(2, 1));

    let (world, sprite, events) = shoved(&field, &bush_north, &[me, ahead], me, Dir::NE, 3);
    assert_eq!(where_is(&world, sprite), me);
    assert_eq!(
        crashes(&events, sprite),
        std::slice::from_ref(&berry_bush),
        "not the sprite"
    );

    let (_, sprite, events) = shoved(&rock_ahead, &bush_north, &[me], me, Dir::NE, 3);
    assert_eq!(
        crashes(&events, sprite),
        [berry_bush],
        "the bush, not the rock"
    );

    let (_, sprite, events) = shoved(&rocky_corner, &[], &[me, ahead], me, Dir::NE, 3);
    assert_eq!(crashes(&events, sprite), [], "rock at the corner");
}
