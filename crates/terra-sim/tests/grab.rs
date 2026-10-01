//! Grab mode's commands (design v23 §2.5, §6.5): the Cursor leads sprites
//! and holds items, one thing at a time.

use terra_sim::{
    Blocker, Command, DataPack, Emptied, EntityId, Event, EventKind, Grip, Map, Outcome, Pos,
    Rejection, Removal, Scenario, ScriptedAction, Terrain, World,
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

/// The reason the world gave for refusing `command`, if it did.
fn refused(events: &[Event], command: Command) -> Option<Rejection> {
    events.iter().find_map(|e| match e.kind {
        EventKind::CommandRejected { command: c, reason } if c == command => Some(reason),
        _ => None,
    })
}

#[test]
fn a_sprite_that_is_gone_cant_be_taken_hold_of() {
    let mut world = world(&["....."], &[], &[], &[]);
    let command = Command::TakeHold {
        sprite: EntityId(99),
    };
    world.submit(command);
    let events = world.step();
    assert_eq!(refused(&events, command), Some(Rejection::Gone));
    assert_eq!(world.cursor().leads(), None);
}

#[test]
fn a_cursor_leading_one_sprite_cant_take_hold_of_another() {
    let mut world = world(&["....."], &[], &[at(0, 0), at(4, 0)], &[]);
    let (first, second) = (sprite_on(&world, at(0, 0)), sprite_on(&world, at(4, 0)));
    let command = Command::TakeHold { sprite: second };
    world.submit(Command::TakeHold { sprite: first });
    world.submit(command);
    let events = world.step();
    assert_eq!(
        refused(&events, command),
        Some(Rejection::Busy(Grip::Leads(first)))
    );
    assert_eq!(world.cursor().leads(), Some(first));
}

#[test]
fn a_led_sprite_walks_to_the_cursor_at_its_own_pace_and_chooses_nothing() {
    let mut world = world(&["........"], &[], &[at(0, 0)], &[]);
    let id = sprite_on(&world, at(0, 0));
    world.submit(Command::TakeHold { sprite: id });
    world.submit(Command::MoveCursor { tile: at(5, 0) });
    let mut started = Vec::new();
    let mut arrived_after = None;
    for tick in 1..=40 {
        for event in world.step() {
            if let EventKind::ActionStarted { id: who, verb } = event.kind
                && who == id
            {
                started.push(verb);
            }
        }
        let pos = world.sprite(id).expect("the sprite").pos();
        if pos == at(5, 0) && arrived_after.is_none() {
            arrived_after = Some(tick);
        }
    }
    // Five tiles take a walker of speed 12 or less more than a tick.
    assert!(arrived_after.is_some_and(|t| t > 1), "{arrived_after:?}");
    assert_eq!(world.sprite(id).expect("the sprite").pos(), at(5, 0));
    assert_eq!(started, Vec::new(), "it chose nothing while led");
}

/// Leads the sprite on `from` towards `tile` for `ticks` ticks, and says
/// where it ends up.
fn lead(world: &mut World, from: Pos, tile: Pos, ticks: u32) -> Pos {
    let id = sprite_on(world, from);
    world.submit(Command::TakeHold { sprite: id });
    world.submit(Command::MoveCursor { tile });
    for _ in 0..ticks {
        world.step();
    }
    assert_eq!(world.cursor().leads(), Some(id), "still led");
    world.sprite(id).expect("the sprite").pos()
}

#[test]
fn a_led_sprite_goes_as_close_as_it_can_to_a_tile_it_cant_stand_on() {
    let mut world = world(&["....#"], &[], &[at(0, 0)], &[]);
    assert_eq!(lead(&mut world, at(0, 0), at(4, 0), 30), at(3, 0));
}

#[test]
fn a_led_sprite_stops_beside_a_sprite_on_the_cursors_tile() {
    let rests = [(at(4, 0), ScriptedAction::Rest); 6];
    let mut world = world(&["......"], &[], &[at(0, 0), at(4, 0)], &rests);
    assert_eq!(lead(&mut world, at(0, 0), at(4, 0), 30), at(3, 0));
}

#[test]
fn a_led_sprite_held_up_by_another_waits_and_never_gives_up() {
    // The sprite resting in the corridor blocks the way to the Cursor.
    let rests = [(at(2, 0), ScriptedAction::Rest); 8];
    let mut world = world(&["....."], &[], &[at(0, 0), at(2, 0)], &rests);
    assert_eq!(lead(&mut world, at(0, 0), at(4, 0), 40), at(1, 0));
}

#[test]
fn a_pet_reaches_a_led_sprite() {
    let mut world = world(&["....."], &[], &[at(0, 0)], &[]);
    let id = sprite_on(&world, at(0, 0));
    world.submit(Command::TakeHold { sprite: id });
    world.step();
    world.submit(Command::Reward {
        sprite: id,
        amplified: false,
        reach_back: 3,
    });
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::Rewarded {
                id,
                amplified: false
            }),
        "{events:?}"
    );
}

#[test]
fn let_go_a_sprite_chooses_for_itself_again_at_once() {
    let mut world = world(&["....."], &[], &[at(0, 0)], &[]);
    let id = sprite_on(&world, at(0, 0));
    world.submit(Command::TakeHold { sprite: id });
    world.step();
    world.submit(Command::LetGo);
    let events = world.step();
    assert!(
        events
            .iter()
            .any(|e| e.kind == EventKind::LetGo { sprite: id }),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ActionStarted { id: who, .. } if who == id)),
        "it chose something that tick: {events:?}"
    );
    assert_eq!(world.cursor().leads(), None);
}

#[test]
fn letting_go_with_no_sprite_led_is_refused() {
    let mut world = world(&["....."], &[], &[], &[]);
    world.submit(Command::LetGo);
    let events = world.step();
    assert_eq!(
        refused(&events, Command::LetGo),
        Some(Rejection::NotLeading)
    );
}

/// The built-in data pack, with `changes` made to `file`'s text.
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
fn a_led_sprite_that_dies_empties_the_cursor() {
    // Born without water and injured whole by its first thirsty tick.
    let data = builtin_changing(
        "physiology.ron",
        &[
            (
                "newborn: (energy: 1.0, hydration: 1.0,",
                "newborn: (energy: 1.0, hydration: 0.0,",
            ),
            ("dehydration: 0.0011", "dehydration: 1.0"),
        ],
    );
    let map = Map::from_ascii(&["..."], &data).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(at(0, 0), None)],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    let id = sprite_on(&world, at(0, 0));
    world.submit(Command::TakeHold { sprite: id });
    let mut events = Vec::new();
    for _ in 0..10 {
        events.extend(world.step());
    }
    assert!(world.sprite(id).is_none(), "it died: {events:?}");
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::CursorEmptied {
                reason: Emptied::Died { sprite: id }
            }),
        "{events:?}"
    );
    assert_eq!(world.cursor().leads(), None);
}

/// The object on `pos`.
fn object_on(world: &World, pos: Pos) -> EntityId {
    world.object_at(pos).expect("an object there").id()
}

#[test]
fn picking_up_an_item_takes_it_off_the_map_into_the_cursor() {
    let mut world = world(&["....."], &[(at(2, 0), "berry")], &[], &[]);
    let berry = object_on(&world, at(2, 0));
    world.submit(Command::PickUp { item: berry });
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::PickedUp {
                item: berry,
                object_type: "berry".into()
            }),
        "{events:?}"
    );
    assert!(world.object_at(at(2, 0)).is_none(), "off the map");
    assert!(world.objects().all(|o| o.id() != berry), "not on the map");
    let held = world.cursor().holds().expect("an item held");
    assert_eq!((held.id(), held.type_name()), (berry, "berry"));
}

#[test]
fn a_fixture_is_rooted_and_cant_be_picked_up() {
    let mut world = world(&["....."], &[(at(2, 0), "berry_bush")], &[], &[]);
    let bush = object_on(&world, at(2, 0));
    let command = Command::PickUp { item: bush };
    world.submit(command);
    let events = world.step();
    assert_eq!(refused(&events, command), Some(Rejection::Rooted));
    assert_eq!(world.object_at(at(2, 0)).map(|o| o.id()), Some(bush));
    assert!(world.cursor().holds().is_none());
}

#[test]
fn an_item_that_is_gone_cant_be_picked_up() {
    let mut world = world(&["....."], &[], &[], &[]);
    let command = Command::PickUp { item: EntityId(99) };
    world.submit(command);
    let events = world.step();
    assert_eq!(refused(&events, command), Some(Rejection::Gone));
}

/// A world of `rows` and `objects` whose Cursor holds the item on `from`.
fn holding(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[Pos],
    from: Pos,
) -> (World, EntityId) {
    let mut world = world(rows, objects, sprites, &[]);
    let item = object_on(&world, from);
    world.submit(Command::PickUp { item });
    world.step();
    assert_eq!(world.cursor().holds().map(|h| h.id()), Some(item));
    (world, item)
}

#[test]
fn a_held_item_is_put_down_on_a_tile_at_rest() {
    let (mut world, ball) = holding(&["....."], &[(at(0, 0), "ball")], &[], at(0, 0));
    world.submit(Command::PutDown { tile: at(3, 0) });
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::PutDown {
                item: ball,
                object_type: "ball".into(),
                pos: at(3, 0)
            }),
        "{events:?}"
    );
    assert!(world.cursor().holds().is_none());
    assert_eq!(world.object_at(at(3, 0)).map(|o| o.id()), Some(ball));
}

#[test]
fn an_item_may_be_put_down_under_a_sprite() {
    let rests = [(at(3, 0), ScriptedAction::Rest); 2];
    let mut world = world(&["....."], &[(at(0, 0), "berry")], &[at(3, 0)], &rests);
    let berry = object_on(&world, at(0, 0));
    world.submit(Command::PickUp { item: berry });
    world.submit(Command::PutDown { tile: at(3, 0) });
    world.step();
    assert_eq!(world.object_at(at(3, 0)).map(|o| o.id()), Some(berry));
}

#[test]
fn an_item_isnt_put_down_where_it_cant_go_and_the_refusal_names_what_is_in_the_way() {
    let (mut world, _) = holding(
        &["..=#"],
        &[(at(0, 0), "ball"), (at(1, 0), "berry")],
        &[],
        at(0, 0),
    );
    // A berry's and a ball's stable object type IDs in the built-in pack.
    let (berry, ball) = (2, 4);
    assert_eq!(world.data().object_type_name(berry), Some("berry"));
    assert_eq!(world.data().object_type_name(ball), Some("ball"));
    let cases = [
        (at(1, 0), Blocker::Object(berry)),
        (at(2, 0), Blocker::Terrain(Terrain::DeepWater)),
        (at(3, 0), Blocker::Terrain(Terrain::Rock)),
    ];
    for (tile, blocker) in cases {
        let command = Command::PutDown { tile };
        world.submit(command);
        let events = world.step();
        let reason = Rejection::InTheWay {
            item_type: ball,
            blocker,
        };
        assert_eq!(refused(&events, command), Some(reason));
        assert!(world.cursor().holds().is_some(), "still held");
    }
}

#[test]
fn putting_down_with_nothing_held_is_refused() {
    let mut world = world(&["....."], &[], &[], &[]);
    let command = Command::PutDown { tile: at(1, 0) };
    world.submit(command);
    let events = world.step();
    assert_eq!(refused(&events, command), Some(Rejection::NotHolding));
}

#[test]
fn a_rolling_ball_picked_up_stops_rolling() {
    // The kick lands on tick 1, and the ball rolls from tick 2.
    let kicker = at(1, 1);
    let script = [
        (kicker, ScriptedAction::Play { at: at(2, 1) }),
        (kicker, ScriptedAction::Rest),
        (kicker, ScriptedAction::Rest),
    ];
    let lane = ["...........", "...........", "..........."];
    let mut world = world(&lane, &[(at(2, 1), "ball")], &[kicker], &script);
    let ball = object_on(&world, at(2, 1));
    world.step();
    world.step();
    assert_eq!(
        world.object_at(at(3, 1)).map(|o| o.id()),
        Some(ball),
        "rolling"
    );
    world.submit(Command::PickUp { item: ball });
    world.submit(Command::PutDown { tile: at(3, 1) });
    for _ in 0..4 {
        world.step();
    }
    assert_eq!(
        world.object_at(at(3, 1)).map(|o| o.id()),
        Some(ball),
        "at rest"
    );
}

/// The built-in pack, but a berry lasts 3 ticks and always sprouts when it
/// expires, wherever it is.
fn short_lived_sprouting_berries() -> DataPack {
    builtin_changing(
        "objects.ron",
        &[
            (
                r#"stages: [(name: "fresh", ticks: (1500, 2500), next: Expire)],"#,
                r#"stages: [(name: "fresh", ticks: (3, 3), next: Expire)],"#,
            ),
            (
                r#"if: [Fertility(Ge, 0.5), DensityBelow("berry_bush", 4, 3), KeepsPathsOpen, Chance(0.1)],"#,
                "if: [],",
            ),
        ],
    )
}

#[test]
fn a_held_berry_expires_and_empties_the_cursor_but_never_sprouts() {
    let data = short_lived_sprouting_berries();
    let map = Map::from_ascii(&["....."], &data).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[(at(2, 0), "berry")],
        sprites: &[],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    let berry = object_on(&world, at(2, 0));
    world.submit(Command::PickUp { item: berry });
    let mut events = Vec::new();
    for _ in 0..6 {
        events.extend(world.step());
    }
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::CursorEmptied {
                reason: Emptied::Removed {
                    item: berry,
                    object_type: "berry".into(),
                    reason: Removal::Expired
                }
            }),
        "{events:?}"
    );
    assert!(world.cursor().holds().is_none());
    assert_eq!(world.objects().count(), 0, "it didn't sprout: {events:?}");
}

#[test]
fn a_held_items_location_conditions_are_false() {
    // Anywhere on the map, this berry destroys itself on its first turn.
    let data = builtin_changing(
        "objects.ron",
        &[(
            r#"if: [Fertility(Ge, 0.5), DensityBelow("berry_bush", 4, 3), KeepsPathsOpen, Chance(0.1)],
          do: [ReplaceWith("berry_bush")]),"#,
            r#"if: [], do: []),
         (trigger: Every(1), if: [Fertility(Ge, 0.0)], do: [DestroySelf]),"#,
        )],
    );
    let map = Map::from_ascii(&["....."], &data).expect("valid drawing");
    let objects = [(at(2, 0), "berry"), (at(4, 0), "berry")];
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &[],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    let held = object_on(&world, at(2, 0));
    world.submit(Command::PickUp { item: held });
    for _ in 0..3 {
        world.step();
    }
    assert!(
        world.object_at(at(4, 0)).is_none(),
        "the one on the map went"
    );
    assert_eq!(world.cursor().holds().map(|h| h.id()), Some(held));
}

#[test]
fn a_sprite_heading_for_an_item_that_is_picked_up_gives_up_as_it_is_gone() {
    let eater = at(0, 0);
    let script = [(eater, ScriptedAction::Eat { at: at(5, 0) })];
    let mut world = world(&["......"], &[(at(5, 0), "berry")], &[eater], &script);
    let (sprite, berry) = (sprite_on(&world, eater), object_on(&world, at(5, 0)));
    world.step();
    world.submit(Command::PickUp { item: berry });
    let events = world.step();
    let ended = events.iter().find_map(|e| match &e.kind {
        EventKind::ActionEnded {
            id,
            outcome,
            action,
            ..
        } if *id == sprite => Some((*outcome, action.target_gone)),
        _ => None,
    });
    assert_eq!(ended, Some((Outcome::Failed, true)), "{events:?}");
}

#[test]
fn a_grab_and_a_let_go_queued_together_both_apply_on_the_next_tick() {
    let mut world = world(&["....."], &[(at(3, 0), "ball")], &[at(0, 0)], &[]);
    let (sprite, ball) = (sprite_on(&world, at(0, 0)), object_on(&world, at(3, 0)));
    world.submit(Command::TakeHold { sprite });
    world.submit(Command::LetGo);
    world.submit(Command::PickUp { item: ball });
    world.submit(Command::PutDown { tile: at(4, 0) });
    let events = world.step();
    let rejected = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::CommandRejected { .. }))
        .count();
    assert_eq!(rejected, 0, "{events:?}");
    assert!(world.cursor().leads().is_none() && world.cursor().holds().is_none());
    assert_eq!(world.object_at(at(4, 0)).map(|o| o.id()), Some(ball));
}

#[test]
fn the_cursor_cant_be_moved_off_the_map() {
    let mut world = world(&["....."], &[], &[], &[]);
    let command = Command::MoveCursor { tile: at(5, 0) };
    world.submit(command);
    let events = world.step();
    assert_eq!(refused(&events, command), Some(Rejection::OffTheMap));
}

#[test]
fn where_the_cursor_is_is_part_of_the_worlds_state() {
    // Saves and replays carry it (design v23 §2.8).
    let rows = ["....."];
    let (mut moved, mut still) = (world(&rows, &[], &[], &[]), world(&rows, &[], &[], &[]));
    moved.submit(Command::MoveCursor { tile: at(2, 0) });
    moved.step();
    still.step();
    assert_ne!(moved.state_hash(), still.state_hash());
}
