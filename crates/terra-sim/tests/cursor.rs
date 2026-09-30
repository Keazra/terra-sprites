//! The Cursor's touch (design v21 §2.5, §4.6, §5.6): Reward and Correct
//! commands, applied at step 1 of the next tick, and what a sprite learns
//! from them.

use terra_sim::{
    Command, DataPack, EntityId, EventKind, Genome, Learned, Map, Pos, Rejection, Scenario,
    ScriptedAction, Thing, Verb, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A genome of `genes` and nothing else that would move a level, so what
/// the Cursor does reads exactly.
fn genome(genes: &str, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: 10.0),
            Trait(trait: "sense_radius", value: 10.0),
            {genes}
            // No fading, so what is learned reads exactly.
            BrainParam(param: "worth_fade_good", value: 0.0),
            BrainParam(param: "worth_fade_bad", value: 0.0),
            BrainParam(param: "habit_fade", value: 0.0),
            BrainParam(param: "habit_fade_bad", value: 0.0),
            BrainParam(param: "fear_fade", value: 0.0),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `objects`, and one sprite of `genes` on
/// `sprite` doing `script`.
fn world(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprite: Pos,
    genes: &str,
    script: &[ScriptedAction],
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(sprite, Some(genome(genes, &data)))];
    let scripted: Vec<(Pos, ScriptedAction)> = script.iter().map(|&s| (sprite, s)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// A sprite resting alone in a field, and its ID.
fn resting_sprite() -> (World, EntityId) {
    let world = world(
        &[".....", ".....", "....."],
        &[],
        at(2, 1),
        "",
        &[ScriptedAction::Rest, ScriptedAction::Rest],
    );
    let id = world.sprites().next().expect("the sprite").id();
    (world, id)
}

fn pet(sprite: EntityId) -> Command {
    Command::Reward {
        sprite,
        amplified: false,
        reach_back: 3,
    }
}

#[test]
fn a_pet_gives_the_sprite_a_good_feeling_of_a_half_on_the_next_tick() {
    // Design v21 §4.6: a pet injects reward .5 at step 1, and learning takes
    // it in at step 4 of the same tick.
    let (mut world, id) = resting_sprite();
    world.submit(pet(id));
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::Rewarded {
                id,
                amplified: false
            }),
        "{events:?}"
    );
    let sprite = world.sprite(id).expect("the sprite");
    assert!(close(sprite.felt(), 0.5), "{}", sprite.felt());
}

#[test]
fn a_hug_gives_the_sprite_a_full_good_feeling() {
    // Design v21 §4.6: amplified, a Reward is a hug, reward 1.
    let (mut world, id) = resting_sprite();
    world.submit(Command::Reward {
        sprite: id,
        amplified: true,
        reach_back: 3,
    });
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::Rewarded {
                id,
                amplified: true
            }),
        "{events:?}"
    );
    let sprite = world.sprite(id).expect("the sprite");
    assert!(close(sprite.felt(), 1.0), "{}", sprite.felt());
}

fn zap(sprite: EntityId) -> Command {
    Command::Correct {
        sprite,
        amplified: false,
    }
}

#[test]
fn a_zap_gives_a_bad_feeling_of_a_half_and_some_pain_but_no_injury() {
    // Design v21 §4.6: a zap injects punishment .5 and pain .3, and never
    // injures.
    let (mut world, id) = resting_sprite();
    world.submit(zap(id));
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::Corrected {
                id,
                amplified: false
            }),
        "{events:?}"
    );
    let sprite = world.sprite(id).expect("the sprite");
    assert!(close(sprite.felt(), -0.5), "{}", sprite.felt());
    assert!(close(sprite.chemical("pain").expect("pain"), 0.3));
    assert_eq!(sprite.chemical("injury"), Some(0.0));
}

#[test]
fn a_shock_gives_a_full_bad_feeling_and_more_pain_but_no_injury() {
    // Design v21 §4.6: amplified, a Correct is a shock: punishment 1, pain .6.
    let (mut world, id) = resting_sprite();
    world.submit(Command::Correct {
        sprite: id,
        amplified: true,
    });
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::Corrected {
                id,
                amplified: true
            }),
        "{events:?}"
    );
    let sprite = world.sprite(id).expect("the sprite");
    assert!(close(sprite.felt(), -1.0), "{}", sprite.felt());
    assert!(close(sprite.chemical("pain").expect("pain"), 0.6));
    assert_eq!(sprite.chemical("injury"), Some(0.0));
}

#[test]
fn a_pet_and_a_zap_each_reach_the_sprite_as_their_own_sense() {
    // Design v21 §4.6: a pet pulses `petted`, a zap `shocked`, which genes
    // can react to. Here each raises its own hormone at step 3.
    let world_with_senses = || {
        let world = world(
            &[".....", ".....", "....."],
            &[],
            at(2, 1),
            r#"Emitter(locus: Locus("petted"), mode: Level, gain: 0.2, chem: "h0"),
               Emitter(locus: Locus("shocked"), mode: Level, gain: 0.2, chem: "h1"),"#,
            &[ScriptedAction::Rest],
        );
        let id = world.sprites().next().expect("the sprite").id();
        (world, id)
    };
    let (mut petted, id) = world_with_senses();
    petted.submit(pet(id));
    petted.step();
    let sprite = petted.sprite(id).expect("the sprite");
    assert_eq!(
        (sprite.chemical("h0"), sprite.chemical("h1")),
        (Some(0.2), Some(0.0))
    );
    let (mut zapped, id) = world_with_senses();
    zapped.submit(zap(id));
    zapped.step();
    let sprite = zapped.sprite(id).expect("the sprite");
    assert_eq!(
        (sprite.chemical("h0"), sprite.chemical("h1")),
        (Some(0.0), Some(0.2))
    );
}

#[test]
fn commands_wait_for_the_next_tick_then_apply_in_the_order_sent_and_levels_stop_at_full() {
    // Design v21 §2.5: each command applies on its own, in order, at step 1
    // of the next tick, and three pets in a tick give one full dose.
    let (mut world, id) = resting_sprite();
    for _ in 0..3 {
        world.submit(pet(id));
    }
    world.submit(zap(id));
    let events = world.step();
    let cursor: Vec<&EventKind> = events
        .iter()
        .map(|e| &e.kind)
        .filter(|k| matches!(k, EventKind::Rewarded { .. } | EventKind::Corrected { .. }))
        .collect();
    let petted = EventKind::Rewarded {
        id,
        amplified: false,
    };
    let zapped = EventKind::Corrected {
        id,
        amplified: false,
    };
    assert_eq!(cursor, [&petted, &petted, &petted, &zapped]);
    let sprite = world.sprite(id).expect("the sprite");
    assert!(close(sprite.felt(), 1.0 - 0.5), "{}", sprite.felt());
    // Nothing waits for the tick after.
    let events = world.step();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::Rewarded { .. })),
        "{events:?}"
    );
}

#[test]
fn a_command_for_a_sprite_that_is_gone_is_refused_and_touches_no_one() {
    // Design v21 §2.5: a sprite that died has left the world, so it's gone,
    // as one that never was is.
    let (mut world, id) = resting_sprite();
    let nobody = EntityId(999);
    world.submit(pet(nobody));
    world.submit(zap(nobody));
    let events = world.step();
    let refused: Vec<&EventKind> = events
        .iter()
        .map(|e| &e.kind)
        .filter(|k| matches!(k, EventKind::CommandRejected { .. }))
        .collect();
    assert_eq!(
        refused,
        [
            &EventKind::CommandRejected {
                command: pet(nobody),
                reason: Rejection::Gone
            },
            &EventKind::CommandRejected {
                command: zap(nobody),
                reason: Rejection::Gone
            },
        ]
    );
    assert!(
        !events.iter().any(|e| matches!(
            e.kind,
            EventKind::Rewarded { .. } | EventKind::Corrected { .. }
        )),
        "{events:?}"
    );
    assert_eq!(world.sprite(id).expect("the sprite").felt(), 0.0);
}

/// A sprite of `genes` that kicks the ball beside it at tick 0, then rests
/// until tick 50, trying nothing else, and its ID.
fn kicker(genes: &str) -> (World, EntityId) {
    let ball = at(3, 1);
    let mut script = vec![ScriptedAction::Play { at: ball }];
    script.extend([ScriptedAction::Rest; 5]);
    let world = world(
        &["......", "......", "......"],
        &[(ball, "ball")],
        at(2, 1),
        genes,
        &script,
    );
    let id = world.sprites().next().expect("the sprite").id();
    (world, id)
}

/// What sprite `id` has learned `learned` is worth, or 0.
fn value_of(world: &World, id: EntityId, learned: &Learned) -> f32 {
    let sprite = world.sprite(id).expect("the sprite");
    sprite
        .memory()
        .into_iter()
        .find(|m| &m.learned == learned)
        .map_or(0.0, |m| m.amount)
}

fn balls_good() -> Learned {
    Learned::Worth {
        thing: "ball".into(),
        need: None,
    }
}

fn kicking_balls() -> Learned {
    Learned::Habit {
        thing: "ball".into(),
        verb: Verb::Play,
    }
}

/// Runs `world` until its tick is `tick`.
fn run_to(world: &mut World, tick: u64) {
    while world.tick() < tick {
        world.step();
    }
}

#[test]
fn a_pet_within_its_reach_back_makes_the_last_thing_tried_and_that_habit_good() {
    // Design v21 §5.6: the kick at tick 0 is 6 ticks old when the pet lands,
    // past the touch window's 3 but within its reach back of 10. So balls
    // are good, .5 × worth_rate_good (.5), and so is kicking them, at full
    // weight: .5 × habit_rate (.3).
    let (mut world, id) = kicker("");
    run_to(&mut world, 6);
    world.submit(Command::Reward {
        sprite: id,
        amplified: false,
        reach_back: 10,
    });
    world.step();
    assert!(close(value_of(&world, id, &balls_good()), 0.25));
    assert!(close(value_of(&world, id, &kicking_balls()), 0.15));
}

/// Kicks the ball at tick 0, and pets the sprite at `tick` with `reach_back`.
/// Returns what it then thinks of balls, and of kicking them.
fn pet_after_a_kick(tick: u64, reach_back: u16) -> (f32, f32) {
    let (mut world, id) = kicker("");
    run_to(&mut world, tick);
    world.submit(Command::Reward {
        sprite: id,
        amplified: false,
        reach_back,
    });
    world.step();
    (
        value_of(&world, id, &balls_good()),
        value_of(&world, id, &kicking_balls()),
    )
}

#[test]
fn a_pet_after_its_reach_back_has_passed_teaches_nothing_of_the_last_try() {
    // Design v21 §5.6: 6 ticks after the kick, a reach back of 5 misses it.
    assert_eq!(pet_after_a_kick(6, 5), (0.0, 0.0));
    assert_eq!(pet_after_a_kick(5, 5), (0.25, 0.15), "just within");
}

#[test]
fn a_reach_back_below_the_touch_window_or_past_the_longest_is_taken_as_that_bound() {
    // Design v21 §2.5: at least touch_window (3), at most max_reach_back (40).
    assert_eq!(pet_after_a_kick(3, 0), (0.25, 0.15), "0 is taken as 3");
    assert_eq!(
        pet_after_a_kick(40, 1000),
        (0.25, 0.15),
        "1,000 is taken as 40"
    );
    assert_eq!(pet_after_a_kick(41, 1000), (0.0, 0.0));
}

/// A tired sprite whose resting eases its tiredness by .05 a tick.
const TIRED_RESTER: &str = r#"InitialConcentration(chem: "tiredness", value: 1.0),
    Emitter(locus: Locus("resting"), mode: Level, gain: -0.05, chem: "tiredness"),"#;

#[test]
fn in_a_tick_the_cursor_touches_a_sprite_relief_still_looks_back_only_the_touch_window() {
    // Design v21 §5.6: the pet at tick 6 reaches back to the kick, but the
    // relief of resting that tick doesn't: the kick is past the touch window.
    let (mut world, id) = kicker(TIRED_RESTER);
    let balls_restful = Learned::Worth {
        thing: "ball".into(),
        need: Some("tiredness".into()),
    };
    run_to(&mut world, 6);
    let before = value_of(&world, id, &balls_restful);
    assert!(before > 0.0, "resting just after the kick taught it");
    world.submit(Command::Reward {
        sprite: id,
        amplified: false,
        reach_back: 10,
    });
    world.step();
    assert_eq!(value_of(&world, id, &balls_restful), before);
    assert!(close(value_of(&world, id, &balls_good()), 0.25));
}

/// A world drawn from `rows` with sprites of `genes` on each of `sprites`,
/// each doing its part of `script`.
fn scene(rows: &[&str], sprites: &[(Pos, &str)], script: &[(Pos, ScriptedAction)]) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&(pos, genes)| (pos, Some(genome(genes, &data))))
        .collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: script,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

#[test]
fn a_zap_while_the_sprite_was_just_hit_teaches_fear_of_the_hitter() {
    // Design v21 §5.6, v18 §5.6: with a `was_hit` pulse live, punishment
    // teaches fear of the attacker, a zap's too: the hit's .5 and the zap's
    // .5, at fear_rate 1. Nothing is bad.
    let (me, attacker) = (at(1, 1), at(2, 1));
    let mut world = scene(
        &["....", "....", "...."],
        &[
            (
                me,
                r#"Emitter(locus: Locus("was_hit"), mode: Level, gain: 0.5, chem: "punishment"),"#,
            ),
            (attacker, ""),
        ],
        &[
            (me, ScriptedAction::Rest),
            (attacker, ScriptedAction::Hit { at: me }),
            (attacker, ScriptedAction::Rest),
        ],
    );
    let (me, attacker) = (
        world.sprite_at(me).expect("me").id(),
        world.sprite_at(attacker).expect("the attacker").id(),
    );
    world.step();
    world.submit(zap(me));
    world.step();
    let memory: Vec<(Learned, f32)> = world
        .sprite(me)
        .expect("me")
        .memory()
        .into_iter()
        .map(|m| (m.learned, m.amount))
        .collect();
    let fear = Learned::Fear {
        thing: Thing::Sprite(attacker),
    };
    assert_eq!(memory, [(fear, -1.0)], "frightening, and nothing else");
}

#[test]
fn commands_waiting_for_the_next_tick_are_part_of_the_world_s_state() {
    // Design v21 §2.8: they're saved, so they're hashed.
    let (mut world, id) = resting_sprite();
    let before = world.state_hash();
    world.submit(pet(id));
    assert_ne!(world.state_hash(), before);
}

/// Kicks the ball at tick 0, and zaps the sprite at `tick`. Returns what it
/// then thinks of balls, and of kicking them.
fn zap_after_a_kick(tick: u64) -> (f32, f32) {
    let (mut world, id) = kicker("");
    run_to(&mut world, tick);
    world.submit(zap(id));
    world.step();
    let balls_bad = Learned::Bad {
        thing: "ball".into(),
    };
    (
        value_of(&world, id, &balls_bad),
        value_of(&world, id, &kicking_balls()),
    )
}

#[test]
fn a_zap_looks_back_only_the_touch_window_whatever_the_speed() {
    // Design v21 §5.6: a late shock mustn't land on the wrong thing, so a
    // Correct carries no reach back. 3 ticks after the kick the zap makes
    // balls bad, .5 × worth_rate_bad (.8), and kicking them, .5 × habit_rate
    // (.3) at full weight; 4 ticks after, it teaches nothing about them.
    let (bad, habit) = zap_after_a_kick(3);
    assert!(close(bad, -0.4) && close(habit, -0.15), "{bad}, {habit}");
    assert_eq!(zap_after_a_kick(4), (0.0, 0.0));
}

#[test]
fn a_zap_with_a_pet_in_the_same_tick_still_looks_back_only_the_touch_window() {
    // Design v21 §5.6: the pet reaches back to the kick 6 ticks before; the
    // zap, looking back only 3, teaches nothing about it.
    let (mut world, id) = kicker("");
    run_to(&mut world, 6);
    world.submit(Command::Reward {
        sprite: id,
        amplified: false,
        reach_back: 10,
    });
    world.submit(zap(id));
    world.step();
    let balls_bad = Learned::Bad {
        thing: "ball".into(),
    };
    assert!(close(value_of(&world, id, &balls_good()), 0.25));
    assert!(
        close(value_of(&world, id, &kicking_balls()), 0.15),
        "the zap took nothing off"
    );
    assert_eq!(value_of(&world, id, &balls_bad), 0.0);
}
