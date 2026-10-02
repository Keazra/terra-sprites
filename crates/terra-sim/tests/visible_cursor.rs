//! The visible Cursor (design v29 §6.5, slice 11d): sprites see a visible
//! Cursor as a thing of its own, go to it or back away from it, and learn
//! about it.

use terra_sim::{
    Command, DataPack, EventKind, Genome, Learned, Map, Outcome, Pos, Progress, Scenario,
    ScriptedAction, Target, Thing, Verb, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A sprite's genome: `genes`, with learning that doesn't fade, so what is
/// learned reads exactly.
fn genome(genes: &str, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: 10.0),
            Trait(trait: "sense_radius", value: 10.0),
            {genes}
            BrainParam(param: "worth_fade_good", value: 0.0),
            BrainParam(param: "worth_fade_bad", value: 0.0),
            BrainParam(param: "habit_fade", value: 0.0),
            BrainParam(param: "habit_fade_bad", value: 0.0),
            BrainParam(param: "fear_fade", value: 0.0),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// Its eye always goes to the Cursor, and it goes over to whatever it
/// looks at.
const DRAWN_TO_THE_CURSOR: &str = r#"
    AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
    Instinct(inputs: [("always", false)], verb: Approach, weight: 3.0),
"#;

/// A world drawn from `rows`, with one sprite of `genes` on `sprite`.
fn world(rows: &[&str], sprite: Pos, genes: &str) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(sprite, Some(genome(genes, &data)))];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

const OPEN: [&str; 5] = [
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
];

/// Shows the Cursor to sprites, on `tile`.
fn show_cursor(world: &mut World, tile: Pos) {
    world.submit(Command::MoveCursor { tile });
    world.submit(Command::ShowCursor { visible: true });
}

fn the_sprite(world: &World) -> terra_sim::SpriteView<'_> {
    world.sprites().next().expect("the sprite")
}

/// What the one sprite's attention scored, by what it scored.
fn seen(world: &World) -> Vec<Thing> {
    the_sprite(world)
        .explain()
        .map(|e| e.attention.into_iter().map(|(thing, _)| thing).collect())
        .unwrap_or_default()
}

#[test]
fn a_hidden_cursor_is_not_seen() {
    let mut world = world(&OPEN, at(1, 2), DRAWN_TO_THE_CURSOR);
    world.submit(Command::MoveCursor { tile: at(5, 2) });
    world.step();
    assert!(!seen(&world).contains(&Thing::Cursor), "{:?}", seen(&world));
    assert!(!world.cursor().visible());
}

#[test]
fn a_visible_cursor_is_seen_as_the_cursor_and_a_sprite_goes_over_to_it() {
    let mut world = world(&OPEN, at(1, 2), DRAWN_TO_THE_CURSOR);
    show_cursor(&mut world, at(6, 2));
    world.step();
    assert!(world.cursor().visible());
    assert_eq!(seen(&world), [Thing::Cursor]);
    let action = the_sprite(&world).action().expect("an action");
    assert_eq!(action.verb, Verb::Approach);
    assert_eq!(action.target, Some(Target::Cursor));
    // It's light, so a sprite may stand under it: it walks until it's on
    // the Cursor's tile or beside it.
    for _ in 0..20 {
        world.step();
    }
    let pos = the_sprite(&world).pos();
    assert!(pos.x >= 5 && pos.x <= 7, "it stopped at {pos:?}");
}

#[test]
fn a_sprite_going_over_to_the_cursor_follows_it_when_it_moves() {
    let mut world = world(&OPEN, at(1, 2), DRAWN_TO_THE_CURSOR);
    show_cursor(&mut world, at(9, 0));
    world.step();
    world.submit(Command::MoveCursor { tile: at(9, 4) });
    // It gets there: beside the Cursor's new tile, or under it.
    let mut got_there = false;
    for _ in 0..30 {
        world.step();
        let pos = the_sprite(&world).pos();
        got_there |= pos.x >= 8 && pos.y >= 3;
    }
    assert!(got_there, "it ended at {:?}", the_sprite(&world).pos());
}

#[test]
fn hiding_the_cursor_ends_an_action_aimed_at_it() {
    let mut world = world(&OPEN, at(1, 2), DRAWN_TO_THE_CURSOR);
    show_cursor(&mut world, at(9, 2));
    world.step();
    world.submit(Command::ShowCursor { visible: false });
    let events = world.step();
    let ended = events.iter().find_map(|event| match &event.kind {
        EventKind::ActionEnded {
            outcome, action, ..
        } => Some((*outcome, action.target)),
        _ => None,
    });
    assert_eq!(ended, Some((Outcome::Failed, Some(Target::Cursor))));
    assert!(!seen(&world).contains(&Thing::Cursor), "{:?}", seen(&world));
}

#[test]
fn a_sprite_backs_away_from_the_cursor() {
    let mut world = world(
        &OPEN,
        at(5, 2),
        r#"AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
           Instinct(inputs: [("always", false)], verb: Retreat, weight: 3.0),"#,
    );
    show_cursor(&mut world, at(6, 2));
    for _ in 0..4 {
        world.step();
    }
    let action = the_sprite(&world).action().expect("an action");
    assert_eq!(action.verb, Verb::Retreat);
    assert!(
        the_sprite(&world).pos().x < 5,
        "{:?}",
        the_sprite(&world).pos()
    );
}

#[test]
fn trying_to_eat_the_cursor_is_fruitless() {
    // It's light: the built-in Cursor's verb table has nothing in it.
    let mut world = world(
        &OPEN,
        at(5, 2),
        r#"AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
           Instinct(inputs: [("always", false)], verb: Eat, weight: 3.0),"#,
    );
    show_cursor(&mut world, at(6, 2));
    world.step();
    let action = the_sprite(&world).action().expect("an action");
    assert_eq!(action.verb, Verb::Eat);
    assert_eq!(action.target, Some(Target::Cursor));
    assert!(action.attempted);
    assert_eq!(action.progress, Progress::Ended(Outcome::Failed));
}

#[test]
fn the_cursor_is_learned_about_as_the_cursor() {
    // A hungry sprite that tries to eat the Cursor learns that eating it
    // doesn't work: a habit about the Cursor.
    let mut world = world(
        &OPEN,
        at(5, 2),
        r#"InitialConcentration(chem: "hunger", value: 1.0),
           AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
           Instinct(inputs: [("hunger", false)], verb: Eat, weight: 3.0),"#,
    );
    show_cursor(&mut world, at(6, 2));
    world.step();
    world.step();
    let memory = the_sprite(&world).memory();
    let habit = memory.iter().find(|m| {
        m.learned
            == Learned::Habit {
                thing: Thing::Cursor,
                verb: Verb::Eat,
            }
    });
    assert!(habit.is_some_and(|m| m.amount < 0.0), "{memory:?}");
}

/// What the one sprite has learned `learned` is, or 0.
fn value_of(world: &World, learned: &Learned) -> f32 {
    the_sprite(world)
        .memory()
        .into_iter()
        .find(|m| m.learned == *learned)
        .map_or(0.0, |m| m.amount)
}

fn likes_the_cursor() -> Learned {
    Learned::Worth {
        thing: Thing::Cursor,
        need: None,
    }
}

fn fears_the_cursor() -> Learned {
    Learned::Fear {
        thing: Thing::Cursor,
    }
}

/// A sprite that tries to eat a berry it stands on at tick 0, then rests.
fn eating_a_berry(berry: Pos) -> World {
    let data = builtin();
    let map = Map::from_ascii(&OPEN, &data).expect("valid drawing");
    let sprites = [(berry, Some(genome("", &data)))];
    let scripted = [
        (berry, ScriptedAction::Eat { at: berry }),
        (berry, ScriptedAction::Rest),
    ];
    let scenario = Scenario {
        map,
        objects: &[(berry, "berry")],
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// Pets the one sprite, or shocks it if `shock`, the tick after it ate.
fn touch_after_eating(world: &mut World, shock: bool) {
    world.step();
    let sprite = the_sprite(world).id();
    world.submit(if shock {
        Command::Correct {
            sprite,
            amplified: true,
        }
    } else {
        Command::Reward {
            sprite,
            amplified: true,
            reach_back: 3,
        }
    });
    world.step();
}

#[test]
fn a_pet_from_a_visible_cursor_teaches_the_sprite_to_like_it_and_what_it_did() {
    let mut world = eating_a_berry(at(5, 2));
    show_cursor(&mut world, at(5, 2));
    touch_after_eating(&mut world, false);
    assert!(value_of(&world, &likes_the_cursor()) > 0.0);
    let berries_good = Learned::Worth {
        thing: "berry".into(),
        need: None,
    };
    assert!(value_of(&world, &berries_good) > 0.0);
    assert_eq!(value_of(&world, &fears_the_cursor()), 0.0);
}

#[test]
fn a_shock_from_a_visible_cursor_teaches_the_sprite_to_fear_it_and_what_it_did() {
    let mut world = eating_a_berry(at(5, 2));
    show_cursor(&mut world, at(5, 2));
    touch_after_eating(&mut world, true);
    assert!(value_of(&world, &fears_the_cursor()) < 0.0);
    let berries_bad = Learned::Bad {
        thing: "berry".into(),
    };
    assert!(value_of(&world, &berries_bad) < 0.0);
    assert_eq!(value_of(&world, &likes_the_cursor()), 0.0);
}

#[test]
fn a_hidden_cursor_s_touch_teaches_nothing_about_the_cursor() {
    for shock in [false, true] {
        let mut world = eating_a_berry(at(5, 2));
        world.submit(Command::MoveCursor { tile: at(5, 2) });
        touch_after_eating(&mut world, shock);
        let about_the_cursor = the_sprite(&world).memory().into_iter().any(|m| {
            matches!(
                m.learned,
                Learned::Worth {
                    thing: Thing::Cursor,
                    ..
                } | Learned::Fear {
                    thing: Thing::Cursor
                }
            )
        });
        assert!(!about_the_cursor, "{:?}", the_sprite(&world).memory());
    }
}

/// A sprite that looks at the Cursor, and does as it feels about it.
const WATCHING_THE_CURSOR: &str = r#"
    AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
    BrainParam(param: "tau_base", value: 0.05),
"#;

#[test]
fn a_sprite_that_fears_the_cursor_backs_away_from_it_and_one_that_likes_it_goes_over() {
    // Liked, anything that goes to it counts: a thing worth having draws a
    // sprite to it, and it tries things on it (design §5.5).
    let going_over = [
        Verb::Approach,
        Verb::Eat,
        Verb::Drink,
        Verb::Hit,
        Verb::Play,
    ];
    for (shock, verbs) in [(true, &[Verb::Retreat][..]), (false, &going_over)] {
        // It rests while it's taught, so each touch is about the Cursor
        // alone: resting tries nothing on anything.
        let data = builtin();
        let map = Map::from_ascii(&OPEN, &data).expect("valid drawing");
        let sprites = [(at(2, 2), Some(genome(WATCHING_THE_CURSOR, &data)))];
        let scripted = [(at(2, 2), ScriptedAction::Rest); 2];
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted: &scripted,
        };
        let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
        show_cursor(&mut world, at(2, 2));
        let sprite = the_sprite(&world).id();
        // Taught over and over, so the feeling counts.
        for _ in 0..20 {
            world.submit(if shock {
                Command::Correct {
                    sprite,
                    amplified: true,
                }
            } else {
                Command::Reward {
                    sprite,
                    amplified: true,
                    reach_back: 3,
                }
            });
            world.step();
        }
        // Then it sits a little way off, and the sprite chooses for itself.
        world.submit(Command::MoveCursor { tile: at(5, 2) });
        let mut chosen = Vec::new();
        for _ in 0..40 {
            for event in world.step() {
                if let EventKind::ActionStarted { verb, .. } = event.kind {
                    chosen.push(verb);
                }
            }
        }
        let share =
            chosen.iter().filter(|v| verbs.contains(v)).count() as f32 / chosen.len() as f32;
        assert!(
            share > 0.5,
            "shock {shock}: {chosen:?} {:?}",
            the_sprite(&world).memory()
        );
    }
}

/// A sprite, resting through its first 20 ticks, shocked in each of them
/// by a visible Cursor on it, so it fears the Cursor fully.
fn afraid_of_the_cursor() -> World {
    afraid_of_the_cursor_in(&OPEN, 20)
}

/// As `afraid_of_the_cursor`, in a world drawn from `rows`, the sprite on
/// (2, 2) resting through its first `rests` ticks.
fn afraid_of_the_cursor_in(rows: &[&str], rests: usize) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(at(2, 2), Some(genome("", &data)))];
    let scripted = vec![(at(2, 2), ScriptedAction::Rest); rests];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &scripted,
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    show_cursor(&mut world, at(2, 2));
    let sprite = the_sprite(&world).id();
    for _ in 0..20 {
        world.submit(Command::Correct {
            sprite,
            amplified: true,
        });
        world.step();
    }
    world.step();
    world
}

#[test]
fn a_sprite_gets_used_to_a_feared_cursor_that_stays_on_it_and_does_nothing() {
    // Design v29 §5.6: it comes to see it can't do anything about it.
    let mut world = afraid_of_the_cursor();
    let feared = value_of(&world, &fears_the_cursor());
    assert!(feared < -0.9, "{feared}");
    for _ in 0..100 {
        world.step();
    }
    let calmer = value_of(&world, &fears_the_cursor());
    assert!(calmer > feared / 2.0, "{feared} then {calmer}");
}

#[test]
fn a_feared_cursor_far_off_or_hidden_leaves_the_fear_as_it_was() {
    for hide in [false, true] {
        let mut world = afraid_of_the_cursor();
        let feared = value_of(&world, &fears_the_cursor());
        world.submit(if hide {
            Command::ShowCursor { visible: false }
        } else {
            Command::MoveCursor { tile: at(10, 4) }
        });
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(value_of(&world, &fears_the_cursor()), feared, "hide {hide}");
    }
}

#[test]
fn a_feared_cursor_the_sprite_cannot_reach_leaves_the_fear_as_it_was() {
    // Design v29 §3.6: a sprite sees the Cursor only if its flood reaches
    // it, so one beyond a wall of rock, however near, calms nothing. The
    // way round the wall is far outside its flood, and it rests throughout.
    let mut walled = vec!["....#......"; 28];
    walled.extend(["...........", "..........."]);
    let mut world = afraid_of_the_cursor_in(&walled, 130);
    let feared = value_of(&world, &fears_the_cursor());
    world.submit(Command::MoveCursor { tile: at(5, 2) });
    for _ in 0..100 {
        world.step();
    }
    assert_eq!(value_of(&world, &fears_the_cursor()), feared);
}

/// A sprite whose pricks punish it, resting beside a thornbush two tiles
/// east, taken hold of and shoved into it by the Cursor, visible or not,
/// the shove sent but not yet run.
fn about_to_be_shoved_into_a_thornbush(visible: bool) -> World {
    about_to_be_shoved_into("thornbush", visible, "")
}

/// As `about_to_be_shoved_into_a_thornbush`, into an object of type `into`,
/// the sprite's genome having `genes` too.
fn about_to_be_shoved_into(into: &str, visible: bool, genes: &str) -> World {
    let data = builtin();
    let map = Map::from_ascii(&OPEN, &data).expect("valid drawing");
    let genes = format!(
        r#"Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),{genes}"#
    );
    let sprites = [(at(1, 2), Some(genome(&genes, &data)))];
    let scripted = [(at(1, 2), ScriptedAction::Rest); 10];
    let scenario = Scenario {
        map,
        objects: &[(at(3, 2), into)],
        sprites: &sprites,
        scripted: &scripted,
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    world.submit(Command::MoveCursor { tile: at(1, 2) });
    world.submit(Command::ShowCursor { visible });
    let sprite = the_sprite(&world).id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    world.submit(Command::Shove {
        toward: terra_sim::Dir::E,
        tiles: 3,
    });
    world
}

/// As `about_to_be_shoved_into_a_thornbush`, four ticks after the shove.
fn shoved_into_a_thornbush(visible: bool) -> World {
    let mut world = about_to_be_shoved_into_a_thornbush(visible);
    for _ in 0..4 {
        world.step();
    }
    world
}

#[test]
fn a_shove_into_a_thornbush_by_a_visible_cursor_teaches_fear_of_the_cursor_too() {
    // Design v29 §5.6: as a hit teaches fear of the hitter. The thornbush is
    // bad either way.
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let seen = shoved_into_a_thornbush(true);
    assert!(value_of(&seen, &thorns_bad) < 0.0);
    assert!(value_of(&seen, &fears_the_cursor()) < 0.0);
    let unseen = shoved_into_a_thornbush(false);
    assert!(value_of(&unseen, &thorns_bad) < 0.0);
    assert_eq!(value_of(&unseen, &fears_the_cursor()), 0.0);
}

#[test]
fn a_crash_teaches_fear_of_the_cursor_only_in_its_own_tick() {
    // Design v29 §5.6: a zap from a hidden Cursor just after the crash
    // teaches nothing about the Cursor, however recent the crash.
    let mut world = about_to_be_shoved_into_a_thornbush(true);
    let mut crashed = false;
    for _ in 0..4 {
        world.step();
        if value_of(&world, &fears_the_cursor()) < 0.0 {
            crashed = true;
            break;
        }
    }
    assert!(crashed, "the crash taught no fear of the Cursor");
    let feared = value_of(&world, &fears_the_cursor());
    let sprite = the_sprite(&world).id();
    world.submit(Command::ShowCursor { visible: false });
    world.submit(Command::Correct {
        sprite,
        amplified: true,
    });
    world.step();
    assert_eq!(value_of(&world, &fears_the_cursor()), feared);
}

/// The ticks stepped after the shove until the sprite crashed, counting the
/// tick of the crash.
fn ticks_to_crash(world: &mut World) -> usize {
    for ticks in 1..=6 {
        let crashed = world
            .step()
            .iter()
            .any(|event| matches!(event.kind, EventKind::Crashed { .. }));
        if crashed {
            return ticks;
        }
    }
    panic!("the sprite never crashed");
}

#[test]
fn a_crash_s_fear_of_the_cursor_does_not_wear_off_in_the_tick_it_is_learned() {
    // Design v29 §5.6: the sprite gets used to a Cursor that does nothing,
    // and in the tick of the crash it did something.
    let fear_after_crash = |calming: f32| {
        let genes = format!(r#"BrainParam(param: "cursor_calming", value: {calming:?}),"#);
        let mut world = about_to_be_shoved_into("thornbush", true, &genes);
        ticks_to_crash(&mut world);
        value_of(&world, &fears_the_cursor())
    };
    let feared = fear_after_crash(0.0);
    assert!(feared < 0.0);
    assert_eq!(fear_after_crash(0.1), feared);
}

#[test]
fn a_hidden_cursor_s_zap_in_the_tick_of_a_crash_teaches_no_fear_of_the_cursor() {
    // Design v29 §5.6: shoved into a bush that doesn't hurt, by a visible
    // Cursor, and zapped by a hidden one as it crashes. The pain is the
    // zap's, and the sprite couldn't see where that came from.
    let crash = ticks_to_crash(&mut about_to_be_shoved_into("berry_bush", true, ""));
    let mut world = about_to_be_shoved_into("berry_bush", true, "");
    for _ in 1..crash {
        world.step();
    }
    let sprite = the_sprite(&world).id();
    world.submit(Command::ShowCursor { visible: false });
    world.submit(Command::Correct {
        sprite,
        amplified: true,
    });
    let crashed = world
        .step()
        .iter()
        .any(|event| matches!(event.kind, EventKind::Crashed { .. }));
    assert!(crashed);
    assert_eq!(value_of(&world, &fears_the_cursor()), 0.0);
}
