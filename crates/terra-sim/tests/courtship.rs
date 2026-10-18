//! Calls, getting to know each other, courtship, partners and kin (M2
//! design §15): a call carries round walls but not through rock; a hearer
//! knows who called and from where, and may go there; sprites get to know
//! the ones they see and play with; a dance is felt by the one danced for;
//! a pair mates only once she's receptive, and becomes partners; close kin
//! know each other; and a female breeds alone only long after she last saw
//! or heard a male.

mod common;
use common::{
    ALONE, FEMALE, KEEN, LONE, WorldBuilder, at, builtin, builtin_changing, builtin_changing_files,
    hatch, id_at, laid, pulse_in_h1, until, world_in,
};

use terra_sim::{
    Command, DataPack, EntityId, EventKind, Outcome, Pos, Progress, ScriptedAction, Sex, Verb,
    World,
};

/// Calls all the time, and does nothing else.
const CALLER: &str = r#"
    Instinct(inputs: [("always", false)], verb: Call, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.02),
"#;

/// Rests all the time, and does nothing else.
const RESTER: &str = r#"
    Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.02),
"#;

/// The same, with the built-in pack.
fn world(rows: &[&str], sprites: &[(Pos, &str)]) -> World {
    world_in(builtin(), rows, sprites)
}

/// Whether the sprite `id` has heard `caller` call, from `from`.
fn heard(world: &World, id: EntityId, caller: EntityId, from: Pos) -> bool {
    let sprite = world.sprite(id).expect("the hearer");
    sprite.heard().contains(&(caller, from))
}

/// A row of grass `width` tiles long.
fn row(width: usize) -> String {
    ".".repeat(width)
}

#[test]
fn a_call_carries_30_tiles_over_grass_and_no_further() {
    // M2 design §15.1: grass soaks up 10 a tile and a call carries 300.
    let field = row(40);
    let rows = [field.as_str()];
    let (caller, near, far) = (at(0, 0), at(30, 0), at(31, 0));
    let mut world = world(&rows, &[(caller, CALLER), (near, RESTER), (far, RESTER)]);
    let (caller, near, far) = (
        id_at(&world, caller),
        id_at(&world, near),
        id_at(&world, far),
    );
    world.step();
    assert!(heard(&world, near, caller, at(0, 0)), "30 tiles away");
    assert!(world.sprite(far).unwrap().heard().is_empty(), "31 away");
}

#[test]
fn a_call_carries_21_tiles_on_the_diagonal() {
    // A diagonal step costs 14/10 of a straight one, as walking does.
    let rows: Vec<String> = (0..23).map(|_| row(23)).collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (caller, near, far) = (at(0, 0), at(21, 21), at(22, 22));
    let mut world = world(&rows, &[(caller, CALLER), (near, RESTER), (far, RESTER)]);
    let (caller, near, far) = (
        id_at(&world, caller),
        id_at(&world, near),
        id_at(&world, far),
    );
    world.step();
    assert!(heard(&world, near, caller, at(0, 0)));
    assert!(world.sprite(far).unwrap().heard().is_empty());
}

#[test]
fn a_call_goes_round_a_wall_through_a_gap_but_not_through_rock() {
    // M2 design §15.1: rock stops sound, and a call doesn't cut a corner
    // past it. Two tiles apart across the wall, the hearer is reached only
    // round its end: down and up beside it, 10 a tile each way, and 40 to
    // cross below it, so a wall 14 tiles long takes 300 and one of 15
    // takes 320, past the call's reach.
    let walls = |long: usize| -> Vec<&str> {
        (0..16)
            .map(|y| if y < long { ".#." } else { "..." })
            .collect()
    };
    let (caller, hearer) = (at(0, 0), at(2, 0));
    let shorter = walls(14);
    let mut round = world(&shorter, &[(caller, CALLER), (hearer, RESTER)]);
    round.step();
    let (from, them) = (id_at(&round, caller), id_at(&round, hearer));
    assert!(heard(&round, them, from, caller), "round the end");

    let longer = walls(15);
    let mut too_far = world(&longer, &[(caller, CALLER), (hearer, RESTER)]);
    too_far.step();
    let them = id_at(&too_far, hearer);
    assert!(
        too_far.sprite(them).unwrap().heard().is_empty(),
        "too far round"
    );
}

#[test]
fn a_call_cuts_no_corner_past_a_tile_that_stops_sound() {
    // M2 design §15.1: with rock on one side of the diagonal and sand
    // that soaks up 300 on the other, the call can't step across the
    // corner, and the way round through the sand is too far. With grass
    // in place of the rock it steps across.
    let data = builtin_changing(
        "terrain.ron",
        &[(
            "fertility: 0.0, drinkable: false, allows_fixtures: true,  sound: 10",
            "fertility: 0.0, drinkable: false, allows_fixtures: true,  sound: 300",
        )],
    );
    let (caller, hearer) = (at(0, 0), at(1, 1));
    let mut past_rock = world_in(
        data.clone(),
        &[".#", ":."],
        &[(caller, CALLER), (hearer, RESTER)],
    );
    past_rock.step();
    let them = id_at(&past_rock, hearer);
    assert!(past_rock.sprite(them).unwrap().heard().is_empty());

    let mut past_grass = world_in(data, &["..", ":."], &[(caller, CALLER), (hearer, RESTER)]);
    past_grass.step();
    let (from, them) = (id_at(&past_grass, caller), id_at(&past_grass, hearer));
    assert!(heard(&past_grass, them, from, caller));
}

#[test]
fn a_call_costs_the_caller_energy() {
    // M2 design §15.1: `calls.cost`, 0.002, on top of what resting costs.
    let rows = ["....."];
    let mut calling = world(&rows, &[(at(0, 0), CALLER)]);
    let mut resting = world(&rows, &[(at(0, 0), RESTER)]);
    calling.step();
    resting.step();
    let energy = |world: &World| world.sprites().next().unwrap().chemical("energy").unwrap();
    let paid = energy(&resting) - energy(&calling);
    assert!((paid - 0.002).abs() < 0.0002, "it paid {paid}");
}

#[test]
fn a_caller_calls_once_and_stands_listening_10_ticks_in_all() {
    // M2 design §15.1: one call, on the action's first tick, heard on the
    // next; the action, call and listening, takes `calls.listen` ticks in
    // all, where it stands.
    let field = row(10);
    let rows = [field.as_str()];
    let hears = format!("{RESTER}{}", pulse_in_h1("heard"));
    let mut world = world(&rows, &[(at(0, 0), CALLER), (at(5, 0), &hears)]);
    let (caller, hearer) = (id_at(&world, at(0, 0)), id_at(&world, at(5, 0)));
    let (mut started, mut ended, mut heard) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..40 {
        for event in world.step() {
            match event.kind {
                EventKind::ActionStarted {
                    id,
                    verb: Verb::Call,
                } if id == caller => started.push(event.tick),
                EventKind::ActionEnded {
                    id,
                    verb: Verb::Call,
                    outcome: Outcome::Applied,
                    ..
                } if id == caller => ended.push(event.tick),
                _ => {}
            }
        }
        if world.sprite(hearer).unwrap().chemical("h1") == Some(1.0) {
            heard.push(world.tick() - 1);
        }
        assert_eq!(world.sprite(caller).unwrap().pos(), at(0, 0), "it stood");
    }
    assert!(started.len() >= 3, "it called again and again: {started:?}");
    let calls: Vec<u64> = started.iter().map(|t| t + 1).filter(|&t| t < 40).collect();
    assert_eq!(heard, calls, "one call each, at {started:?}");
    for (start, end) in started.iter().zip(&ended) {
        let ticks = end - start + 1;
        assert_eq!(ticks, 10, "started {started:?}, ended {ended:?}");
    }
}

#[test]
fn a_heard_call_is_known_for_150_ticks_and_then_forgotten() {
    // M2 design §15.1: `calls.heard_for`. The caller calls once, and the
    // Cursor then holds it where it stands, so it calls no more.
    let field = row(10);
    let rows = [field.as_str()];
    let mut world = world(&rows, &[(at(0, 0), CALLER), (at(5, 0), RESTER)]);
    let (caller, hearer) = (id_at(&world, at(0, 0)), id_at(&world, at(5, 0)));
    world.step();
    assert!(heard(&world, hearer, caller, at(0, 0)), "heard on tick 0");
    world.submit(Command::TakeHold { sprite: caller });
    world.submit(Command::MoveCursor { tile: at(0, 0) });
    while world.tick() < 151 {
        let knows = heard(&world, hearer, caller, at(0, 0));
        assert!(knows, "forgotten too soon, at tick {}", world.tick());
        world.step();
    }
    assert!(world.sprite(hearer).unwrap().heard().is_empty());
}

/// Goes up to the sprite it looks at while that one is of the other sex,
/// and otherwise rests.
const OTHER_SEX: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("target_other_sex", false)], verb: Approach, weight: 1.0),
    Instinct(inputs: [("always", false)], verb: Rest, weight: 0.5),
    BrainParam(param: "tau_base", value: 0.02),
    BrainParam(param: "curiosity", value: 0.0),
"#;

#[test]
fn a_caller_heard_but_out_of_sight_shows_its_body_language() {
    // M2 design §15.1: read as if in sight, so he goes to her call, 20
    // tiles off, as she's of the other sex, and not to a male's.
    let field = row(30);
    let rows = [field.as_str()];
    let goes = |caller: &str| {
        let mut world = world(&rows, &[(at(0, 0), caller), (at(20, 0), OTHER_SEX)]);
        let hearer = id_at(&world, at(20, 0));
        (0..30).any(|_| {
            world.step();
            let action = world.sprite(hearer).unwrap().action();
            action.is_some_and(|a| a.verb == Verb::Approach && a.heard == Some(at(0, 0)))
        })
    };
    assert!(goes(&format!("{CALLER}{FEMALE}")), "he went to her call");
    assert!(!goes(CALLER), "he went to a male's call");
}

/// A row of grass 30 tiles long and 5 deep, with berry bushes across it
/// on the tenth column, all but one if `gap`.
fn walled(gap: bool) -> (Vec<String>, Vec<(Pos, &'static str)>) {
    let rows = (0..5).map(|_| row(30)).collect();
    let skip = usize::from(gap);
    let bushes = (0..5)
        .skip(skip)
        .map(|y| (at(9, y), "berry_bush"))
        .collect();
    (rows, bushes)
}

#[test]
fn a_sprite_with_no_way_to_a_call_never_sets_off_and_forgets_it() {
    // M2 design §15.1: objects don't stop sound, so the call is heard
    // across a row of bushes; with no gap there's no way there, and the
    // call is forgotten at once, long before `heard_for`. With a gap, it
    // sets off. The caller calls once.
    let trip = |gap: bool| {
        let (rows, bushes) = walled(gap);
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let mut world = WorldBuilder::new(&rows)
            .objects(&bushes)
            .sprites(&[(at(1, 2), CALLER), (at(25, 2), ANSWERS)])
            .build();
        let (caller, answerer) = (id_at(&world, at(1, 2)), id_at(&world, at(25, 2)));
        world.step();
        assert!(
            heard(&world, answerer, caller, at(1, 2)),
            "heard through bushes"
        );
        world.submit(Command::TakeHold { sprite: caller });
        world.submit(Command::MoveCursor { tile: at(1, 2) });
        let mut set_off = false;
        for _ in 0..10 {
            world.step();
            let action = world.sprite(answerer).unwrap().action();
            set_off |= action.is_some_and(|a| a.heard == Some(at(1, 2)));
        }
        let knows = !world.sprite(answerer).unwrap().heard().is_empty();
        (set_off, knows)
    };
    let (set_off, knows) = trip(false);
    assert!(!set_off && !knows, "set off {set_off}, knows {knows}");
    let (set_off, knows) = trip(true);
    assert!(
        set_off && knows,
        "through the gap: set off {set_off}, knows {knows}"
    );
}

#[test]
fn a_caller_that_calls_again_from_elsewhere_turns_the_trip_there() {
    // M2 design §15.1: on the way to the call, the Cursor leads the caller
    // off, out of sight, and lets it go to call again from there.
    let field = row(30);
    let rows = vec![field.as_str(); 16];
    let (first, second) = (at(1, 1), at(1, 14));
    let mut world = world(&rows, &[(first, CALLER), (at(28, 1), ANSWERS)]);
    let (caller, answerer) = (id_at(&world, first), id_at(&world, at(28, 1)));
    let heading = |world: &World| {
        let action = world.sprite(answerer).unwrap().action();
        action.filter(|a| a.remembered).and_then(|a| a.heard)
    };
    for _ in 0..3 {
        world.step();
    }
    assert_eq!(heading(&world), Some(first), "on its way by ear");
    world.submit(Command::TakeHold { sprite: caller });
    world.submit(Command::MoveCursor { tile: second });
    while world.sprite(caller).unwrap().pos() != second {
        world.step();
        assert_eq!(heading(&world), Some(first), "still on its way");
    }
    world.submit(Command::LetGo);
    let mut turned = false;
    for _ in 0..5 {
        for event in world.step() {
            let restarted =
                matches!(event.kind, EventKind::ActionStarted { id, .. } if id == answerer);
            assert!(!restarted, "a new action, not the trip turned");
        }
        turned |= heading(&world) == Some(second);
    }
    assert!(turned, "it heads for where the caller called again");
}

/// Goes up to whatever sprite it looks at.
const ANSWERS: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("attended_sprite", false)], verb: Approach, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.02),
"#;

#[test]
fn a_sprite_that_hears_a_call_out_of_sight_goes_to_where_it_came_from() {
    // M2 design §15.1: the caller is offered at the edge of sight, and
    // going to it is a trip to the call's tile; once the caller is in
    // sight, 10 tiles off, it's an ordinary action aimed at it.
    let field = row(30);
    let rows = [field.as_str(); 3];
    let (from, start) = (at(1, 1), at(25, 1));
    let mut world = world(&rows, &[(from, CALLER), (start, ANSWERS)]);
    let (caller, answerer) = (id_at(&world, from), id_at(&world, start));
    let mut by_ear = false;
    let mut came = false;
    for _ in 0..200 {
        world.step();
        let sprite = world.sprite(answerer).expect("the answerer");
        let action = sprite.action().expect("an action");
        if action.verb == Verb::Approach && action.heard == Some(from) {
            assert!(action.remembered, "a trip");
            by_ear = true;
        }
        let beside = sprite.pos().x <= from.x + 1 && sprite.pos().y.abs_diff(from.y) <= 1;
        if beside && action.progress == Progress::Ended(Outcome::Applied) {
            came = true;
            break;
        }
    }
    assert!(by_ear, "it set off by ear");
    assert!(came, "it came to the caller");
    assert!(world.sprite(caller).is_some());
}

#[test]
fn a_sprite_that_finds_no_caller_where_the_call_came_from_forgets_the_call() {
    // M2 design §15.1: got there with the caller still out of sight, it
    // has lost the trail. The call is known long enough to get there.
    let data = builtin_changing("physiology.ron", &[("heard_for: 150,", "heard_for: 1000,")]);
    let rows: Vec<String> = (0..30).map(|_| row(31)).collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (from, start) = (at(1, 1), at(30, 1));
    let mut world = world_in(data, &rows, &[(from, CALLER), (start, ANSWERS)]);
    let (caller, answerer) = (id_at(&world, from), id_at(&world, start));
    world.step();
    assert!(heard(&world, answerer, caller, from));
    // The Cursor leads the caller away, as far from where it called as
    // the answerer is, so it's never in sight.
    world.submit(Command::TakeHold { sprite: caller });
    world.submit(Command::MoveCursor { tile: at(1, 29) });
    let mut got_there = false;
    for _ in 0..600 {
        world.step();
        let sprite = world.sprite(answerer).expect("the answerer");
        if sprite.pos() == from {
            got_there = true;
            break;
        }
    }
    assert!(got_there, "it went where the call came from");
    world.step();
    let sprite = world.sprite(answerer).expect("the answerer");
    assert!(sprite.heard().is_empty(), "it forgot the call");
    let away = world.sprite(caller).unwrap().pos().y - from.y;
    assert!(away > 14, "the caller is out of sight, {away} tiles away");
}

#[test]
fn a_male_that_hears_a_female_call_counts_her_near_and_stays_male() {
    // M2 design §15.1, round 1 Q4: hearing counts for sex by neighbours.
    // He turns female after about 800 ticks with no female adult sensed.
    let turning = r#"
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
        Emitter(locus: Locus("female_near"), mode: Level, invert: true, threshold: 0.5, gain: 0.003, chem: "feminiser", from: Adult),
        Emitter(locus: Locus("female_near"), mode: Level, gain: -0.01, chem: "feminiser"),
        HalfLife(chem: "feminiser", ticks: 600),
    "#;
    let field = row(30);
    let rows = [field.as_str()];
    let sex_after = |her_genes: &str| {
        let female = format!("{FEMALE}{her_genes}");
        let mut world = world(&rows, &[(at(0, 0), &female), (at(25, 0), turning)]);
        let him = id_at(&world, at(25, 0));
        for _ in 0..2_000 {
            world.step();
        }
        world.sprite(him).expect("him").sex()
    };
    assert_eq!(sex_after(CALLER), Sex::Male, "he heard her");
    assert_eq!(
        sex_after(RESTER),
        Sex::Female,
        "he never knew she was there"
    );
}

#[test]
fn a_female_that_hears_a_male_call_cannot_breed_alone() {
    // M2 design §15.7: a male heard counts as one seen, here for 1,000
    // ticks.
    let soon = builtin_changing(
        "physiology.ron",
        &[("alone_after: 4500,", "alone_after: 1000,")],
    );
    let field = row(30);
    let rows = [field.as_str()];
    let female = format!("{ALONE}{FEMALE}");
    let first_alone = |his_genes: &str| {
        let sprites = [(at(0, 0), female.as_str()), (at(25, 0), his_genes)];
        let mut world = world_in(soon.clone(), &rows, &sprites);
        laid(&mut world, 1_500).map(|laying| laying.tick)
    };
    assert_eq!(first_alone(CALLER), None, "she heard him");
    assert!(first_alone(RESTER).is_some_and(|tick| tick >= 1_000));
}

#[test]
fn a_sprite_gets_to_know_one_it_sees_and_better_one_it_plays_with() {
    // M2 design §15.2: `befriend_rate` (0.0013) a tick in sight, from
    // `forget_below` (0.01) on the first; a play adds `befriend_play` (0.1)
    // to both. Out of sight, it never knows the other.
    let field = row(30);
    let rows = [field.as_str()];
    let sprites = [(at(0, 0), RESTER), (at(3, 0), RESTER), (at(25, 0), RESTER)];
    let mut seeing = world(&rows, &sprites);
    let [a, b, far] = [at(0, 0), at(3, 0), at(25, 0)].map(|pos| id_at(&seeing, pos));
    for _ in 0..100 {
        seeing.step();
    }
    let knows =
        |world: &World, id: EntityId, other: EntityId| world.sprite(id).unwrap().familiarity(other);
    let seen = knows(&seeing, a, b);
    assert!((0.13..0.15).contains(&seen), "{seen}");
    assert!(
        (knows(&seeing, b, a) - seen).abs() < 1e-6,
        "both see the other"
    );
    assert_eq!(knows(&seeing, a, far), 0.0);
    let known = seeing.sprite(a).unwrap().known_sprites();
    assert_eq!(known.iter().map(|&(id, _)| id).collect::<Vec<_>>(), [b]);

    let player = r#"
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("attended_sprite", false)], verb: Play, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.02),
    "#;
    let mut playing = world(&rows, &[(at(0, 0), player), (at(1, 0), RESTER)]);
    let (a, b) = (id_at(&playing, at(0, 0)), id_at(&playing, at(1, 0)));
    let plays = until(&mut playing, 100, |event| match event.kind {
        EventKind::ActionEnded {
            verb: Verb::Play,
            outcome: Outcome::Applied,
            ..
        } => Some(event.tick),
        _ => None,
    });
    assert!(plays.is_some(), "they played");
    let ticks = playing.tick() as f32;
    let by_sight = 0.01 + (ticks - 1.0) * 0.0013;
    for (id, other) in [(a, b), (b, a)] {
        let known = knows(&playing, id, other);
        assert!(known >= by_sight + 0.09, "{known}");
    }
}

#[test]
fn a_sprite_known_only_by_sight_is_forgotten_once_out_of_sight_it_fades() {
    // M2 design §15.2: familiarity fades by `befriend_fade`, here fast,
    // while the other walks off out of sight.
    let fading = r#"
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.02),
        BrainParam(param: "befriend_fade", value: 0.01),
    "#;
    let field = row(30);
    let rows = [field.as_str()];
    // It walks off in legs it can see the end of.
    let away = [12, 21, 29].map(|x| {
        (
            at(3, 0),
            ScriptedAction::Wander {
                destination: at(x, 0),
            },
        )
    });
    let mut world = WorldBuilder::new(&rows)
        .sprites(&[(at(0, 0), fading), (at(3, 0), RESTER)])
        .scripts(&away)
        .build();
    let (a, b) = (id_at(&world, at(0, 0)), id_at(&world, at(3, 0)));
    world.step();
    assert!(world.sprite(a).unwrap().familiarity(b) > 0.0, "seen");
    for _ in 0..200 {
        world.step();
    }
    assert_eq!(world.sprite(b).unwrap().pos(), at(29, 0), "it walked off");
    assert_eq!(world.sprite(a).unwrap().familiarity(b), 0.0, "forgotten");
}

/// Dances for whatever sprite it looks at, and feels a dance: being danced
/// for raises its receptivity, and dancing its `h1`.
const DANCER: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("attended_sprite", false)], verb: Court, weight: 1.0),
    Emitter(locus: Locus("courted"), mode: Level, gain: 0.2, chem: "receptivity"),
    Emitter(locus: Locus("danced"), mode: Level, gain: 0.2, chem: "h1"),
    BrainParam(param: "tau_base", value: 0.02),
"#;

/// Rests, and being danced for raises its receptivity.
const DANCED_FOR: &str = r#"
    Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    Emitter(locus: Locus("courted"), mode: Level, gain: 0.2, chem: "receptivity"),
    BrainParam(param: "tau_base", value: 0.02),
"#;

#[test]
fn a_dance_is_felt_by_the_one_danced_for_and_the_dancer() {
    // M2 design §15.3: `courted` on the one danced for, `danced` on the
    // dancer.
    let rows = ["......"];
    let mut world = world(&rows, &[(at(1, 0), DANCER), (at(2, 0), DANCED_FOR)]);
    let (dancer, danced_for) = (id_at(&world, at(1, 0)), id_at(&world, at(2, 0)));
    let danced = until(&mut world, 20, |event| match event.kind {
        EventKind::ActionEnded {
            id,
            verb: Verb::Court,
            outcome: Outcome::Applied,
            ..
        } if id == dancer => Some(()),
        _ => None,
    });
    assert!(danced.is_some(), "it danced");
    // The pulses go live at the next step 3.
    world.step();
    let level = |id, chem| world.sprite(id).unwrap().chemical(chem).unwrap();
    assert!(level(danced_for, "receptivity") > 0.0, "courted");
    assert!(level(dancer, "h1") > 0.0, "danced");
}

#[test]
fn an_adult_never_dances_for_a_young_sprite_nor_a_young_one_at_all() {
    // M2 design §5, §15.3: Court is offered only between adults.
    let rows = ["......"];
    let mut world = WorldBuilder::new(&rows)
        .sprites(&[(at(1, 0), DANCER), (at(2, 0), DANCER)])
        .newborns(&[at(2, 0)])
        .build();
    for _ in 0..200 {
        for event in world.step() {
            assert!(
                !matches!(
                    event.kind,
                    EventKind::ActionStarted {
                        verb: Verb::Court,
                        ..
                    }
                ),
                "a dance with a young sprite"
            );
        }
    }
}

/// An instinct to mate with whatever sprite it sees, with no receptivity.
const EAGER: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
"#;

#[test]
fn a_pair_lays_only_once_she_is_receptive() {
    // M2 design §15.3: her receptivity at least `breeding.receptive`.
    let rows = ["......"];
    let female = format!("{EAGER}{FEMALE}");
    let mut world = world(&rows, &[(at(1, 0), &female), (at(2, 0), EAGER)]);
    let mut tries = Vec::new();
    for _ in 0..200 {
        let events = world.step();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Laid { .. })),
            "an egg with no courtship"
        );
        tries.extend(events.iter().filter_map(|e| match e.kind {
            EventKind::ActionEnded {
                verb: Verb::Mate,
                outcome,
                ..
            } => Some(outcome),
            _ => None,
        }));
    }
    assert!(tries.contains(&Outcome::Failed), "{tries:?}");
}

/// The built-in pack, but eggs cost nothing and a parent may breed again
/// 100 ticks after an egg.
fn often() -> DataPack {
    builtin_changing(
        "physiology.ron",
        &[
            ("cost: 0.3,", "cost: 0.0,"),
            ("rest: 1500,", "rest: 100,"),
            LONE,
        ],
    )
}

#[test]
fn a_pair_that_has_an_egg_are_partners_and_are_told_once() {
    // M2 design §15.4.
    let rows = ["......", "......"];
    let female = format!("{KEEN}{FEMALE}");
    let mut world = world_in(often(), &rows, &[(at(1, 0), &female), (at(2, 0), KEEN)]);
    let (she, he) = (id_at(&world, at(1, 0)), id_at(&world, at(2, 0)));
    let mut paired = 0;
    let mut eggs = 0;
    for _ in 0..600 {
        for event in world.step() {
            match event.kind {
                EventKind::Paired { a, b } => {
                    assert_eq!((a, b), (she, he));
                    paired += 1;
                }
                EventKind::Laid { .. } => eggs += 1,
                _ => {}
            }
        }
    }
    assert!(eggs >= 2, "{eggs} eggs");
    assert_eq!(paired, 1, "told once, on the first egg");
    assert_eq!(world.sprite(she).unwrap().partner(), Some((he, false)));
    assert_eq!(world.sprite(he).unwrap().partner(), Some((she, false)));
}

#[test]
fn a_sprite_that_has_an_egg_with_another_pairs_with_it_and_frees_its_partner() {
    // M2 design §15.4: pairing again frees the old partner, with a
    // `parted` line.
    let rows = [".....", "....."];
    let female = format!("{KEEN}{FEMALE}");
    let mut world = world_in(
        often(),
        &rows,
        &[(at(1, 0), KEEN), (at(2, 0), &female), (at(3, 0), KEEN)],
    );
    let she = id_at(&world, at(2, 0));
    let mut partners = Vec::new();
    let mut parted = Vec::new();
    for _ in 0..3000 {
        for event in world.step() {
            match event.kind {
                EventKind::Paired { a, b } => {
                    assert_eq!(a, she);
                    partners.push(b);
                }
                EventKind::Parted { a, b } => parted.push((a, b)),
                _ => {}
            }
        }
        if partners.len() == 2 {
            break;
        }
    }
    let [first, second] = partners[..] else {
        panic!("she paired with {partners:?}");
    };
    assert_ne!(first, second);
    assert_eq!(parted, [(she, first)]);
    assert_eq!(world.sprite(she).unwrap().partner(), Some((second, false)));
    assert_eq!(world.sprite(second).unwrap().partner(), Some((she, false)));
    assert_eq!(world.sprite(first).unwrap().partner(), None);
}

#[test]
fn partners_saved_and_loaded_are_the_same_world() {
    let rows = ["......"];
    let female = format!("{KEEN}{FEMALE}");
    let mut world = world(&rows, &[(at(1, 0), &female), (at(2, 0), KEEN)]);
    laid(&mut world, 100).expect("an egg");
    world.step();
    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    let she = id_at(&world, at(1, 0));
    assert_eq!(
        loaded.sprite(she).unwrap().partner(),
        world.sprite(she).unwrap().partner()
    );
    assert!(world.sprite(she).unwrap().partner().is_some());
}

#[test]
fn a_mother_her_young_and_their_brothers_and_sisters_are_kin() {
    // M2 design §15.5.
    let female = format!("{ALONE}{FEMALE}");
    let rows = ["........", "........", "........", "........"];
    let mut world = world_in(often(), &rows, &[(at(2, 1), &female), (at(6, 3), RESTER)]);
    let (mother, stranger) = (id_at(&world, at(2, 1)), id_at(&world, at(6, 3)));
    let first = laid(&mut world, 50).expect("an egg").egg;
    let second = laid(&mut world, 300).expect("another egg").egg;
    hatch(&mut world, second);
    let first = world.sprite(first).expect("the first hatched");
    assert!(first.is_kin(mother) && first.is_kin(second));
    assert!(world.sprite(mother).unwrap().is_kin(second));
    assert!(!first.is_kin(stranger) && !first.is_kin(first.id()));
}

/// The built-in pack as a save from before courtship has it: no courtship
/// loci or inputs, so no calls or courtship, and breeding alone gated on
/// longing (M2 design §10.3). Its chemicals are the built-in list, which
/// is enough: with the loci gone, courtship is off.
fn before_courtship() -> DataPack {
    let starter = include_str!("../../../data/genomes/starter.ron");
    let courting: Vec<String> = starter
        .lines()
        .filter(|line| {
            [
                "courted",
                "danced",
                "receptivity",
                "grief",
                "lost_partner",
                "target_familiar",
                "target_partner",
                "target_kin",
                "target_condition",
                "paired",
                "male_near\", true",
                "Call",
            ]
            .iter()
            .any(|word| line.contains(word))
        })
        .map(str::to_string)
        .collect();
    let courting: Vec<(&str, &str)> = courting.iter().map(|line| (line.as_str(), "")).collect();
    let inputs: Vec<String> = include_str!("../../../data/brain_io.ron")
        .lines()
        .filter(|line| (74..=87).any(|id| line.contains(&format!("(id: {id}, "))))
        .map(str::to_string)
        .collect();
    let inputs: Vec<(&str, &str)> = inputs.iter().map(|line| (line.as_str(), "")).collect();
    let loci: Vec<String> = include_str!("../../../data/loci.ron")
        .lines()
        .filter(|line| {
            [10, 11, 12, 44, 45, 46, 47]
                .iter()
                .any(|id| line.contains(&format!("(id: {id}, ")))
        })
        .map(str::to_string)
        .collect();
    let loci: Vec<(&str, &str)> = loci.iter().map(|line| (line.as_str(), "")).collect();
    builtin_changing_files(&[
        ("genomes/starter.ron", &courting),
        ("brain_io.ron", &inputs),
        ("loci.ron", &loci),
        (
            "physiology.ron",
            &[
                ("alone_after: 4500,", "breeds_alone: 0.5,"),
                ("receptive: 0.5,", ""),
                ("apart: 4500,", ""),
                (
                    "calls: (cost: 0.002, reach: 300, heard_for: 150, listen: 10),",
                    "",
                ),
            ],
        ),
    ])
}

#[test]
fn a_pack_from_before_courtship_lays_with_no_dance_and_has_no_partners_nor_calls() {
    // M2 design §15: a save keeps the pack it was made with (M1 design
    // §2.8). Court and Call are never offered, mating needs no
    // receptivity, and an egg makes no partners.
    let rows = ["......"];
    let female = format!("{EAGER}{FEMALE}");
    // It would dance for anyone it looks at, or call: an old pack has no
    // loci for being danced for.
    let both = r#"
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("attended_sprite", false)], verb: Court, weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Call, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.02),
    "#;
    let mut world = world_in(
        before_courtship(),
        &rows,
        &[(at(1, 0), &female), (at(2, 0), EAGER), (at(4, 0), both)],
    );
    let she = id_at(&world, at(1, 0));
    let mut laid_eggs = 0;
    for _ in 0..300 {
        for event in world.step() {
            match event.kind {
                EventKind::ActionStarted {
                    verb: Verb::Court | Verb::Call,
                    ..
                } => panic!("a dance or a call in a pack without them"),
                EventKind::Paired { .. } => panic!("partners in a pack without them"),
                EventKind::Laid { .. } => laid_eggs += 1,
                _ => {}
            }
        }
    }
    assert!(laid_eggs > 0, "they laid with no dance");
    assert_eq!(world.sprite(she).unwrap().partner(), None);
}

#[test]
fn a_pack_from_before_courtship_lets_a_longing_female_breed_alone_with_a_male_in_sight() {
    // M2 design §10.3: such a pack gates breeding alone on longing.
    let rows = ["........", "........", "........", "........"];
    let female = format!("{ALONE}{FEMALE}");
    let mut world = world_in(
        before_courtship(),
        &rows,
        &[(at(2, 1), &female), (at(5, 2), RESTER)],
    );
    let laying = laid(&mut world, 50).expect("she laid alone");
    assert_eq!(laying.with, None);
}
