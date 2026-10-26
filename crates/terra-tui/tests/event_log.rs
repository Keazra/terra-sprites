//! The event log as drawn (design §6.1): what it says of each event,
//! newest first under the map, and how lines that read the same merge.

mod common;
use common::screen::{COPY_AND_CHANGE, a_lone_egg, app_for, garden, inside, render, touched};
use common::{
    HURT_ITSELF, UNHURT, acted, at, at_sprite, died, finished, lines, pack, pack_changing,
};

use terra_sim::{
    Blocker, Command, CopyChangeView, DeathCause, Emptied, EntityId, Event, EventKind, Grip,
    Learned, Outcome, Rejection, Removal, Target, Terrain, Thing, Verb, World,
};
use terra_tui::app::App;
use terra_tui::theme::Theme;

#[test]
fn the_event_log_lists_deaths_newest_first_under_the_map() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let id = EntityId(31);
    app.record(&[died(4_012, id, DeathCause::Dehydration, 4_012)], &world);
    app.record(
        &[
            Event {
                tick: 4_100,
                kind: EventKind::ObjectRemoved {
                    id: EntityId(3),
                    object_type: "berry".into(),
                    reason: Removal::Expired,
                },
            },
            died(4_100, EntityId(12), DeathCause::Starvation, 3_900),
            died(4_100, EntityId(13), DeathCause::OldAge, 66_000),
        ],
        &world,
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(screen[24].starts_with("┌─ Events ─"), "{:?}", screen[24]);
    assert_eq!(
        inside(&screen[25]),
        "4,100  Sprite #13 died (old age, age 66,000)"
    );
    assert_eq!(
        inside(&screen[26]),
        "4,100  Sprite #12 died (starvation, age 3,900)"
    );
    assert_eq!(
        inside(&screen[27]),
        "4,012  Sprite #31 died (dehydration, age 4,012)"
    );
    assert!(screen[28].starts_with('└'), "three lines of events");
}

#[test]
fn a_line_that_repeats_in_the_event_log_counts_up_within_cp437() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    // Two lines that read the same, in a row.
    let id = EntityId(31);
    app.record(&[died(4_012, id, DeathCause::Dehydration, 4_012)], &world);
    app.record(&[died(4_100, id, DeathCause::Dehydration, 4_012)], &world);
    let screen = lines(&render(&app, &world, 100, 30));
    assert_eq!(
        inside(&screen[25]),
        "4,100  Sprite #31 died (dehydration, age 4,012) x2"
    );
    assert!(screen[25].chars().all(terra_tui::cp437::contains));
}

#[test]
fn the_event_log_leaves_object_and_action_events_out_and_keeps_the_latest_100() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let spawned = Event {
        tick: 1,
        kind: EventKind::ObjectSpawned {
            id: EntityId(9),
            object_type: "berry".into(),
            pos: at(1, 1),
        },
    };
    app.record(&[spawned], &world);
    let id = EntityId(3);
    app.record(
        &[
            Event {
                tick: 1,
                kind: EventKind::ActionStarted {
                    id,
                    verb: Verb::Wander,
                },
            },
            finished(1, id, Verb::Wander, Outcome::Failed),
        ],
        &world,
    );
    assert_eq!(app.event_log().count(), 0, "nor action events");
    for tick in 0..150 {
        let starved = died(tick, EntityId(tick), DeathCause::Starvation, tick);
        app.record(&[starved], &world);
    }
    let ticks: Vec<u64> = app.event_log().map(|(event, _)| event.tick).collect();
    assert_eq!(ticks.len(), 100);
    assert_eq!((ticks[0], ticks[99]), (149, 50), "newest first");
}

#[test]
fn the_event_log_shows_every_play_and_hit_and_whatever_hurt_a_sprite() {
    use Outcome::{Applied, Interrupted};
    // Object types: berry_bush 1, thornbush 3, ball 4, sprite 101.
    let ball = Some((Target::Object(EntityId(40)), 4));
    let thorns = Some((Target::Object(EntityId(77)), 3));
    let bush = Some((Target::Object(EntityId(80)), 1));
    let sprite = |id| at_sprite(EntityId(id));
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let log = |app: &App| -> Vec<String> {
        let screen = lines(&render(app, &world, 100, 30));
        screen[25..28]
            .iter()
            .map(|row| inside(row).to_string())
            .collect()
    };
    app.record(
        &[
            acted(10, EntityId(4), Verb::Play, ball, Applied, UNHURT),
            acted(11, EntityId(5), Verb::Eat, bush, Applied, UNHURT),
            acted(12, EntityId(9), Verb::Play, sprite(2), Applied, UNHURT),
            acted(13, EntityId(7), Verb::Hit, sprite(12), Interrupted, UNHURT),
            acted(14, EntityId(7), Verb::Hit, sprite(12), Applied, UNHURT),
        ],
        &world,
    );
    assert_eq!(
        log(&app),
        [
            "14  Sprite #7 hit Sprite #12",
            "12  Sprite #9 played with Sprite #2",
            "10  Sprite #4 kicked a ball",
        ],
        "not eating that hurt nobody, nor what didn't get done"
    );
    app.record(
        &[
            acted(20, EntityId(3), Verb::Eat, thorns, Applied, HURT_ITSELF),
            acted(21, EntityId(3), Verb::Play, thorns, Applied, HURT_ITSELF),
            acted(22, EntityId(6), Verb::Hit, ball, Applied, UNHURT),
        ],
        &world,
    );
    assert_eq!(
        log(&app),
        [
            "22  Sprite #6 hit a ball",
            "21  Sprite #3 played with a thornbush and got hurt",
            "20  Sprite #3 tried to eat a thornbush and got hurt",
        ]
    );
}

/// The event log's rows, newest first, once an app on `world` has recorded a
/// lesson for each of `lessons`: (tick, sprite ID, what it learned, good).
fn logged_lessons(world: &World, lessons: Vec<(u64, u64, Learned, bool)>) -> Vec<String> {
    let watched = lessons
        .into_iter()
        .map(|(tick, id, learned, good)| (tick, id, learned, good, None))
        .collect();
    logged_watched(world, watched)
}

/// The same, each lesson with the sprite ID it was learned from watching,
/// if any (M2 design §18.4).
fn logged_watched(
    world: &World,
    lessons: Vec<(u64, u64, Learned, bool, Option<u64>)>,
) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    let events: Vec<Event> = lessons
        .into_iter()
        .map(|(tick, id, learned, good, watching)| Event {
            tick,
            kind: EventKind::LearnedMilestone {
                id: EntityId(id),
                learned,
                good,
                watching: watching.map(EntityId),
            },
        })
        .collect();
    app.record(&events, world);
    let screen = lines(&render(&app, world, 100, 30));
    screen[25..25 + events.len()]
        .iter()
        .map(|row| inside(row).to_string())
        .collect()
}

#[test]
fn the_event_log_says_what_a_sprite_learned_in_plain_words() {
    // Design v16 §6.1.
    let world = garden(pack());
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let water_for_thirst = Learned::Worth {
        thing: "water".into(),
        need: Some("thirst".into()),
    };
    let eating_balls = Learned::Habit {
        thing: "ball".into(),
        verb: Verb::Eat,
    };
    let log = logged_lessons(
        &world,
        vec![
            (30, 12, thorns_bad, false),
            (31, 12, water_for_thirst, true),
            (32, 9, eating_balls, false),
        ],
    );
    assert_eq!(
        log,
        [
            "32  Sprite #9 learned: eating balls is bad",
            "31  Sprite #12 learned: water is good for thirst",
            "30  Sprite #12 learned: thornbushes are bad",
        ]
    );
}

#[test]
fn the_event_log_says_whom_a_lesson_was_learned_from_watching() {
    // M2 design §18.4.
    let world = garden(pack());
    let water_for_thirst = Learned::Worth {
        thing: "water".into(),
        need: Some("thirst".into()),
    };
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let log = logged_watched(
        &world,
        vec![
            (30, 21, water_for_thirst, true, Some(12)),
            (31, 21, thorns_bad, false, Some(12)),
        ],
    );
    assert_eq!(
        log,
        [
            "31  Sprite #21 learned from watching Sprite #12: thornbushes are bad",
            "30  Sprite #21 learned from watching Sprite #12: water is good for thirst",
        ]
    );
}

#[test]
fn a_lesson_words_an_object_type_as_it_names_itself() {
    // Design v17 §3.5.1: thornbushes renamed brambles, and balls with no
    // plural, as if they were a thing you don't count.
    let world = garden(pack_changing(&[(
        "objects.ron",
        &[
            (r#"plural: "thornbushes""#, r#"plural: "brambles""#),
            (r#" plural: "balls","#, ""),
        ],
    )]));
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let ball_for_boredom = Learned::Worth {
        thing: "ball".into(),
        need: Some("boredom".into()),
    };
    let playing_with_balls = Learned::Habit {
        thing: "ball".into(),
        verb: Verb::Play,
    };
    let log = logged_lessons(
        &world,
        vec![
            (30, 12, thorns_bad, false),
            (31, 12, ball_for_boredom, true),
            (32, 9, playing_with_balls, true),
        ],
    );
    assert_eq!(
        log,
        [
            "32  Sprite #9 learned: playing with ball is good",
            "31  Sprite #12 learned: ball is good for boredom",
            "30  Sprite #12 learned: brambles are bad",
        ]
    );
}

#[test]
fn a_lesson_about_a_category_words_it_as_the_category_names_itself() {
    // Design v19 §3.5.5, §6.1: bushes are counted; fruit isn't, so it
    // keeps its name and takes "is".
    let world = garden(pack());
    let bushes_bad = Learned::Bad {
        thing: Thing::Category("bush".into()),
    };
    let fruit_for_hunger = Learned::Worth {
        thing: Thing::Category("fruit".into()),
        need: Some("hunger".into()),
    };
    let log = logged_lessons(
        &world,
        vec![
            (30, 12, bushes_bad, false),
            (31, 12, fruit_for_hunger, true),
        ],
    );
    assert_eq!(
        log,
        [
            "31  Sprite #12 learned: fruit is good for hunger",
            "30  Sprite #12 learned: bushes are bad",
        ]
    );
}

#[test]
fn the_event_log_says_when_sprites_in_general_turn_frightening() {
    // Design v18 §6.1.
    let world = garden(pack());
    let log = logged_lessons(
        &world,
        vec![(
            40,
            3,
            Learned::Fear {
                thing: Thing::Category("sprite".into()),
            },
            false,
        )],
    );
    assert_eq!(log, ["40  Sprite #3 learned: sprites are frightening"]);
}

/// The event log's first three rows once an app has recorded `events`.
fn logged(world: &World, events: &[Event]) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    for event in events {
        app.record(std::slice::from_ref(event), world);
    }
    let screen = lines(&render(&app, world, 100, 30));
    screen[25..28]
        .iter()
        .map(|row| inside(row).to_string())
        .collect()
}

#[test]
fn the_event_log_tells_the_player_what_they_did_through_the_cursor() {
    // Design v21 §6.1: spoken to the player.
    let world = garden(pack());
    let first = logged(
        &world,
        &[touched(1, "pet"), touched(2, "hug"), touched(3, "zap")],
    );
    assert_eq!(
        first,
        [
            "3  You zapped Sprite #12",
            "2  You hugged Sprite #12",
            "1  You petted Sprite #12",
        ]
    );
    let then = logged(
        &world,
        &[
            touched(4, "shock"),
            touched(5, "refused pet"),
            touched(6, "refused shock"),
        ],
    );
    assert_eq!(
        then,
        [
            "6  Couldn't shock Sprite #12: it's gone",
            "5  Couldn't pet Sprite #12: it's gone",
            "4  You shocked Sprite #12",
        ]
    );
}

/// `kinds` as events on ticks 1, 2, 3 and on.
fn on_ticks(kinds: Vec<EventKind>) -> Vec<Event> {
    (1..)
        .zip(kinds)
        .map(|(tick, kind)| Event { tick, kind })
        .collect()
}

#[test]
fn the_event_log_tells_the_player_what_they_grabbed_and_let_go() {
    // Design v23 §6.1: spoken to the player.
    let world = garden(pack());
    let (sprite, ball, berry) = (EntityId(12), EntityId(20), EntityId(21));
    let first = on_ticks(vec![
        EventKind::TookHold { sprite },
        EventKind::LetGo { sprite },
        EventKind::PickedUp {
            item: ball,
            object_type: "ball".into(),
        },
    ]);
    assert_eq!(
        logged(&world, &first),
        [
            "3  You picked up a ball",
            "2  You let go of Sprite #12",
            "1  You took hold of Sprite #12",
        ]
    );
    let then = on_ticks(vec![
        EventKind::PutDown {
            item: ball,
            object_type: "ball".into(),
            pos: at(1, 1),
        },
        EventKind::CursorEmptied {
            reason: Emptied::Removed {
                item: berry,
                object_type: "berry".into(),
                reason: Removal::Expired,
            },
        },
        // A led sprite's death has its own line, and needs no other.
        EventKind::CursorEmptied {
            reason: Emptied::Died { sprite },
        },
    ]);
    assert_eq!(
        logged(&world, &then)[..2],
        [
            "2  The berry you were holding expired",
            "1  You put the ball down",
        ]
    );
}

#[test]
fn the_event_log_says_why_a_grab_was_refused() {
    // Design v23 §6.1. A ball's stable type ID is 4, a berry's 2.
    let world = garden(pack());
    let (sprite, other) = (EntityId(12), EntityId(13));
    let refused = |command, reason| EventKind::CommandRejected { command, reason };
    let put_down = Command::PutDown { tile: at(1, 1) };
    let blocked = |blocker| Rejection::InTheWay {
        item_type: 4,
        blocker,
    };
    let first = on_ticks(vec![
        refused(Command::TakeHold { sprite }, Rejection::Gone),
        refused(
            Command::TakeHold { sprite },
            Rejection::Busy(Grip::Leads(other)),
        ),
        refused(put_down.clone(), blocked(Blocker::Object(2))),
    ]);
    assert_eq!(
        logged(&world, &first),
        [
            "3  Couldn't put the ball down: a berry is there",
            "2  Couldn't take hold of Sprite #12: you're already leading Sprite #13",
            "1  Couldn't take hold of Sprite #12: it's gone",
        ]
    );
    let then = on_ticks(vec![
        refused(put_down.clone(), blocked(Blocker::Terrain(Terrain::Rock))),
        refused(put_down, blocked(Blocker::Terrain(Terrain::DeepWater))),
        refused(Command::LetGo, Rejection::NotLeading),
    ]);
    assert_eq!(
        logged(&world, &then),
        [
            "3  Couldn't let go: you're not leading a sprite",
            "2  Couldn't put the ball down: it can't go in deep water",
            "1  Couldn't put the ball down: it can't go on rock",
        ]
    );
}

#[test]
fn the_same_line_again_merges_into_one_with_a_count() {
    // Design v21 §6.1: spam-clicking doesn't bury the log.
    let world = garden(pack());
    let mut events: Vec<Event> = (1..=10).map(|tick| touched(tick, "pet")).collect();
    events.push(touched(11, "zap"));
    assert_eq!(
        logged(&world, &events),
        [
            "11  You zapped Sprite #12",
            "10  You petted Sprite #12 x10",
            "",
        ]
    );
}

#[test]
fn lines_that_read_the_same_merge_though_their_events_differ() {
    // Design v21 §6.1: two different balls read the same, and so do pets
    // refused at different speeds.
    let world = garden(pack());
    let sprite = EntityId(12);
    let kick = |tick, ball| {
        let ball = Some((Target::Object(EntityId(ball)), 4));
        acted(tick, sprite, Verb::Play, ball, Outcome::Applied, UNHURT)
    };
    let refused = |tick, reach_back| Event {
        tick,
        kind: EventKind::CommandRejected {
            command: Command::Reward {
                sprite,
                amplified: false,
                reach_back,
            },
            reason: Rejection::Gone,
        },
    };
    let rows = logged(
        &world,
        &[kick(1, 40), kick(2, 41), refused(3, 3), refused(4, 20)],
    );
    assert_eq!(
        rows,
        [
            "4  Couldn't pet Sprite #12: it's gone x2",
            "2  Sprite #12 kicked a ball x2",
            "",
        ]
    );
}

#[test]
fn the_event_log_tells_of_throws_shoves_and_crashes_that_hurt() {
    // Design v25 §6.1: spoken to the player; a crash only if it hurt.
    let world = garden(pack());
    let (sprite, other, ball) = (EntityId(12), EntityId(13), EntityId(20));
    let events = on_ticks(vec![
        EventKind::Threw {
            item: ball,
            object_type: "ball".into(),
        },
        EventKind::Shoved { sprite },
        EventKind::Crashed {
            sprite,
            into: Thing::ObjectType("thornbush".into()),
            hurt: true,
        },
        EventKind::Crashed {
            sprite,
            into: Thing::Sprite(other),
            hurt: false,
        },
    ]);
    assert_eq!(
        logged(&world, &events),
        [
            "3  Sprite #12 was shoved into a thornbush and got hurt",
            "2  You shoved Sprite #12",
            "1  You threw the ball",
        ]
    );
}

#[test]
fn the_event_log_says_when_a_sprite_grows_up_and_grows_old() {
    // M2 design §9.6.
    let world = garden(pack());
    let id = EntityId(12);
    let rows = logged(
        &world,
        &[
            Event {
                tick: 4_500,
                kind: EventKind::GrewUp { id },
            },
            Event {
                tick: 13_500,
                kind: EventKind::GrewOld { id },
            },
        ],
    );
    assert_eq!(
        rows[..2],
        ["13,500  Sprite #12 grew old", "4,500  Sprite #12 grew up"]
    );
}

#[test]
fn the_event_log_says_who_laid_an_egg_hatched_and_turned_female() {
    // M2 design §10.
    let world = garden(pack());
    let (mother, father) = (EntityId(12), EntityId(13));
    let (egg, other_egg) = (EntityId(20), EntityId(21));
    let first = on_ticks(vec![
        EventKind::TurnedFemale { id: mother },
        EventKind::Laid {
            egg,
            by: mother,
            with: Some(father),
        },
        EventKind::Laid {
            egg: other_egg,
            by: mother,
            with: None,
        },
    ]);
    assert_eq!(
        logged(&world, &first),
        [
            "3  Sprite #12 laid an egg alone",
            "2  Sprite #12 laid an egg (with Sprite #13)",
            "1  Sprite #12 turned female",
        ]
    );
    let then = on_ticks(vec![
        EventKind::Hatched {
            id: egg,
            parents: vec![mother, father],
        },
        EventKind::Hatched {
            id: other_egg,
            parents: vec![mother],
        },
        // An egg is never thrown (§10.4): 103 is its type.
        EventKind::CommandRejected {
            command: Command::Throw {
                from: at(1, 1),
                toward: terra_sim::Dir::E,
                tiles: 3,
            },
            reason: Rejection::NotThrowable { item_type: 103 },
        },
    ]);
    assert_eq!(
        logged(&world, &then),
        [
            "3  Couldn't throw the egg: it can only be put down",
            "2  Sprite #21 hatched (Sprite #12's)",
            "1  Sprite #20 hatched (Sprite #12 and Sprite #13's)",
        ]
    );
}

#[test]
fn the_event_log_says_what_s_new_in_a_hatchling_with_a_gene_changed() {
    // M2 design §13.2: the genes copying changed in what they do, not
    // those doubled or lost: nothing changed, every gene doubled or lost,
    // and every gene changed.
    for (doubled_lost, changed, new) in [
        ("0.0", "0.0", false),
        ("0.5", "0.0", false),
        ("0.0", "1.0", true),
    ] {
        let rules = format!(
            "copy_and_change: (doubled: {doubled_lost}, lost: {doubled_lost}, changed: {changed},"
        );
        let (mut world, egg) = a_lone_egg(&[
            ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
            ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
            (COPY_AND_CHANGE, rules.as_str()),
        ]);
        let mut app = app_for(&world, Theme::cp437(), 100, 30);
        loop {
            let events = world.step();
            app.record(&events, &world);
            let hatched =
                |event: &Event| matches!(event.kind, EventKind::Hatched { id, .. } if id == egg);
            if events.iter().any(hatched) {
                break;
            }
            assert!(world.tick() < 2_000, "it hatched");
        }
        let screen = lines(&render(&app, &world, 100, 30));
        let line = screen[25..28]
            .iter()
            .map(|row| inside(row).to_string())
            .find(|row| row.contains("hatched"))
            .expect("its hatching in the log");
        let hatched = format!("Sprite #{} hatched (Sprite #1's)", egg.0);
        if new {
            let changes = world.sprite(egg).expect("the hatchling").copy_changes();
            assert!(!changes.is_empty());
            let said = format!("{hatched}, with something new: ");
            let (_, rest) = line.split_once(&said).expect(&line);
            let named = ["low energy", "low hydration", "always"];
            assert!(named.iter().any(|name| rest.starts_with(name)), "{line}");
        } else {
            let changes = world.sprite(egg).expect("the hatchling").copy_changes();
            let doubled_or_lost = |change: &CopyChangeView| {
                matches!(
                    change,
                    CopyChangeView::Doubled { .. } | CopyChangeView::Lost { .. }
                )
            };
            assert_eq!(changes.is_empty(), doubled_lost == "0.0", "{changes:?}");
            assert!(changes.iter().all(doubled_or_lost), "{changes:?}");
            assert!(line.ends_with(&hatched), "{line}");
        }
    }
}
