//! Remembered places (M2 design §7): a sprite remembers where
//! something that stays put eased a need, and goes back to it out of sight.

mod common;
use common::{at, builtin};

use terra_sim::{
    Command, DataPack, EventKind, Genome, Map, Outcome, Pos, RememberedPlace, Scenario,
    ScriptedAction, Target, Thing, Verb, World,
};

/// A thirsty sprite that sees 6 tiles: thirst creeps up all the time and
/// drops at a drink; thirst leads to drinking, and with nothing pressing it
/// wanders. Nothing it learns fades.
fn thirsty(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 6.0),
        Emitter(locus: Locus("always"), mode: Level, gain: 0.004, chem: "thirst"),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
        Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
        BrainParam(param: "tau_base", value: 0.05),
        BrainParam(param: "worth_fade_good", value: 0.0),
        BrainParam(param: "place_fade", value: 0.0),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A room 40 tiles long with water along its west wall.
const ROOM: [&str; 5] = [
    "~.......................................",
    "~.......................................",
    "~.......................................",
    "~.......................................",
    "~.......................................",
];

/// A thirsty sprite in `ROOM` on `start`, doing `script` first.
fn room(start: Pos, script: &[ScriptedAction]) -> World {
    let data = builtin();
    let map = Map::from_ascii(&ROOM, &data).expect("valid drawing");
    let sprites = [(start, Some(thirsty(&data)))];
    let scripted: Vec<(Pos, ScriptedAction)> = script.iter().map(|&s| (start, s)).collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// Runs `world` for up to `ticks` ticks, and returns where the sprite stood
/// when it set off to drink, for the first drink it made, if it made one.
fn first_drink(world: &mut World, ticks: u64) -> Option<Pos> {
    let mut set_off = None;
    for _ in 0..ticks {
        let before = world.sprites().next().expect("the sprite").pos();
        for event in world.step() {
            match event.kind {
                EventKind::ActionStarted {
                    verb: Verb::Drink, ..
                } => set_off = Some(before),
                EventKind::ActionEnded {
                    verb: Verb::Drink,
                    outcome: Outcome::Applied,
                    action,
                    ..
                } if matches!(action.target, Some(Target::Water(_))) => return set_off,
                _ => {}
            }
        }
    }
    None
}

/// Wanders along row 2 from column `from` to column `to`, 5 tiles at a
/// time, each within sight.
fn along(from: u16, to: u16) -> Vec<ScriptedAction> {
    let mut hops = Vec::new();
    let mut x = from;
    while x != to {
        x = if to > x {
            (x + 5).min(to)
        } else {
            x.saturating_sub(5).max(to)
        };
        hops.push(ScriptedAction::Wander {
            destination: at(x, 2),
        });
    }
    hops
}

/// A thirsty sprite drinks at the west wall, and wanders to the far end of
/// the room, out of sight of the water.
fn drank_and_wandered_off() -> World {
    let mut script = vec![ScriptedAction::Rest; 10];
    script.push(ScriptedAction::Drink { at: at(0, 2) });
    script.extend(along(1, 36));
    room(at(1, 2), &script)
}

#[test]
fn a_sprite_that_found_water_and_wandered_off_goes_back_to_it_when_thirsty() {
    let mut world = drank_and_wandered_off();
    // The scripted drink, then the walk away.
    assert_eq!(first_drink(&mut world, 200), Some(at(1, 2)));
    let set_off = first_drink(&mut world, 1_000).expect("it goes back and drinks");
    assert!(
        set_off.x > 7,
        "it set off from {set_off:?}, within sight of the water"
    );
}

#[test]
fn a_trip_out_of_sight_says_it_is_from_memory_until_the_place_is_in_reach() {
    let mut world = drank_and_wandered_off();
    first_drink(&mut world, 200);
    let mut from_memory = Vec::new();
    for _ in 0..1_000 {
        world.step();
        let sprite = world.sprites().next().expect("the sprite");
        if let Some(action) = sprite.action().filter(|a| a.verb == Verb::Drink) {
            from_memory.push((sprite.pos().x, action.remembered));
            if action.attempted {
                break;
            }
        }
    }
    // Its flood reaches 6 tiles, and the water's goal tiles are a tile out
    // from it. The action finds the water in reach at the next tick's 5.0,
    // a step after it is, so the tile between counts as neither.
    let (out, near): (Vec<_>, Vec<_>) = from_memory
        .iter()
        .filter(|&&(x, _)| x != 7)
        .partition(|&&(x, _)| x > 7);
    assert!(!out.is_empty(), "it walked from out of sight");
    assert!(
        out.iter().all(|&&(_, remembered)| remembered),
        "{from_memory:?}"
    );
    assert!(
        near.iter().all(|&&(_, remembered)| !remembered),
        "{from_memory:?}"
    );
}

#[test]
fn a_sprite_with_no_way_back_to_a_place_forgets_it_and_never_sets_off() {
    // M2 design §7.4: bushes grow across the room behind it.
    let mut world = drank_and_wandered_off();
    first_drink(&mut world, 200);
    while world.sprites().next().expect("the sprite").pos().x < 30 {
        world.step();
    }
    assert_eq!(places(&world).len(), 1);
    let bush = world
        .data()
        .object_type_id("berry_bush")
        .expect("a bush type");
    for y in 0..5 {
        world.submit(Command::Place {
            tile: at(10, y),
            object_type: bush,
        });
    }
    let mut set_off = false;
    for _ in 0..1_000 {
        for event in world.step() {
            if let EventKind::CommandRejected { reason, .. } = event.kind {
                panic!("a bush wasn't placed: {reason:?}");
            }
        }
        // Thirsty, it may try drinking from the bushes it sees, but never
        // heads for the water.
        let sprite = world.sprites().next().expect("the sprite");
        set_off |= sprite
            .action()
            .is_some_and(|a| matches!(a.target, Some(Target::Water(_))));
    }
    assert_eq!(places(&world), [], "it forgot the water");
    assert!(!set_off, "it never set off for water it can't get to");
}

/// `ROOM` walled across at column 20 but for gaps on `gaps`, with only a
/// corridor along row 2 east of it, from column 22, so a sprite there comes
/// back through the gap on row 2. Column 21 is open on rows 0 to 2, so a
/// gap on row 0 is a way round.
fn walled(gaps: &[usize]) -> Vec<String> {
    (0..ROOM.len())
        .map(|y| {
            ROOM[y]
                .char_indices()
                .map(|(x, tile)| match x {
                    20 if !gaps.contains(&y) => '#',
                    21 if y > 2 => '#',
                    22.. if y != 2 => '#',
                    _ => tile,
                })
                .collect()
        })
        .collect()
}

/// A sprite that only ever rests.
fn resting(data: &DataPack) -> Genome {
    genome(
        10.0,
        r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
           BrainParam(param: "tau_base", value: 0.05),"#,
        data,
    )
}

/// The thirsty sprite of `drank_and_wandered_off` in `rows`, run until it
/// sets off back to the water, when a resting sprite is put down in the gap
/// on row 2, in its way. Then up to `ticks` more, until a trip ends, with
/// how it ended and the most ticks in a row the trip stood still.
fn held_up_in(rows: &[String], ticks: u64) -> (World, Vec<Outcome>, u32) {
    let data = builtin();
    // Its eye isn't caught by the sprite in its way, so its mind stays on
    // the water.
    let thirsty = genome(
        10.0,
        r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.004, chem: "thirst"),
           Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
           Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),
           Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
           BrainParam(param: "tau_base", value: 0.05),
           BrainParam(param: "salience_gain", value: 0.0),
           BrainParam(param: "curiosity", value: 0.0),
           BrainParam(param: "vigilance", value: 0.0),"#,
        &data,
    );
    let resting = resting(&data);
    let mut script = vec![ScriptedAction::Rest; 10];
    script.push(ScriptedAction::Drink { at: at(0, 2) });
    script.extend(along(1, 36));
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mut world = world_in(data, &rows, &[], at(1, 2), thirsty, &script);
    assert_eq!(first_drink(&mut world, 200), Some(at(1, 2)));
    let on_a_trip = |world: &World| {
        let sprite = world.sprites().next().expect("the sprite");
        sprite.action().is_some_and(|a| a.remembered)
    };
    for _ in 0..1_000 {
        if on_a_trip(&world) {
            break;
        }
        world.step();
    }
    assert!(on_a_trip(&world), "it set off for the water");
    let here = world.sprites().next().expect("the sprite").pos();
    assert!(here.x > 22, "it set off from {here:?}, past the wall");
    world.submit(Command::SpawnSprite {
        tile: at(20, 2),
        genome: Some(resting),
    });
    let first = world.sprites().next().expect("the sprite").id();
    let mut ended = Vec::new();
    let (mut still, mut longest) = (0, 0);
    for _ in 0..ticks {
        let trip = on_a_trip(&world);
        let before = world.sprites().next().expect("the sprite").pos();
        for event in world.step() {
            match event.kind {
                EventKind::CommandRejected { reason, .. } => panic!("{reason:?}"),
                EventKind::ActionEnded { id, outcome, .. } if trip && id == first => {
                    ended.push(outcome);
                }
                _ => {}
            }
        }
        let sprites: Vec<Pos> = world.sprites().map(|s| s.pos()).collect();
        assert_eq!(sprites[1], at(20, 2), "the resting sprite stays put");
        still = if trip && sprites[0] == before {
            still + 1
        } else {
            0
        };
        longest = longest.max(still);
        if !ended.is_empty() || !on_a_trip(&world) {
            break;
        }
    }
    (world, ended, longest)
}

#[test]
fn a_trip_held_up_by_a_sprite_finds_the_way_round_it() {
    // M2 design §7.4: a second gap, two rows up, is a longer way round.
    // Held up, it plans again at once, rather than after `replan_after`
    // ticks standing still.
    let (world, ended, still) = held_up_in(&walled(&[0, 2]), 300);
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(ended, [], "the trip carries on");
    assert!(sprite.pos().x < 20, "it got round, to {:?}", sprite.pos());
    assert_eq!(still, 1, "it was held up for a tick");
}

#[test]
fn a_trip_with_no_way_round_a_sprite_ends_blocked_and_the_place_is_kept() {
    // M2 design §7.4: a sprite in the way soon moves, so it's no reason to
    // forget the water.
    let (world, ended, still) = held_up_in(&walled(&[2]), 300);
    assert_eq!(ended, [Outcome::Blocked]);
    assert_eq!(still, 2, "held up for a tick, it found no way the next");
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.pos(), at(21, 2), "it was held up beside the gap");
    assert_eq!(places(&world).len(), 1, "it still remembers the water");
}

#[test]
fn a_sprite_that_never_found_the_water_never_goes_to_it_from_out_of_sight() {
    // It may wander within sight of the water, and drink then.
    let mut world = room(at(36, 2), &[]);
    if let Some(set_off) = first_drink(&mut world, 1_000) {
        assert!(set_off.x <= 7, "it set off from {set_off:?}, out of sight");
    }
}

/// A genome of `speed` that sees 6 tiles, with `genes`, whose remembered
/// places never fade.
fn genome(speed: f32, genes: &str, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: {speed:?}),
            Trait(trait: "sense_radius", value: 6.0),
            {genes}
            BrainParam(param: "place_fade", value: 0.0),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// Hunger that creeps up all the time and drops at a meal.
const HUNGRY: &str = r#"
    Emitter(locus: Locus("always"), mode: Level, gain: 0.004, chem: "hunger"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),"#;

/// A world drawn from `rows` with `objects`, and one sprite of `genome` on
/// `start` doing `script`, in `data`.
fn world_in(
    data: DataPack,
    rows: &[&str],
    objects: &[(Pos, &str)],
    start: Pos,
    genome: Genome,
    script: &[ScriptedAction],
) -> World {
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(start, Some(genome))];
    let scripted: Vec<(Pos, ScriptedAction)> = script.iter().map(|&s| (start, s)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// The places the world's one sprite remembers.
fn places(world: &World) -> Vec<RememberedPlace> {
    world
        .sprites()
        .next()
        .expect("the sprite")
        .remembered_places()
}

/// Steps `world` `ticks` times.
fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
}

/// Rests, to grow hungry or thirsty, then does `then`.
fn after_a_while(then: &[ScriptedAction]) -> Vec<ScriptedAction> {
    let mut script = vec![ScriptedAction::Rest; 10];
    script.extend_from_slice(then);
    script.push(ScriptedAction::Rest);
    script
}

#[test]
fn a_bush_that_eased_hunger_is_remembered_where_it_stands_but_a_berry_on_the_ground_is_not() {
    // M2 design §7: only what stays put. The berry is eaten up, and items
    // get moved; a bush is fixed in place.
    let data = builtin();
    let (bush, berry) = (at(4, 2), at(2, 2));
    let hungry = genome(10.0, HUNGRY, &data);
    let objects = [(bush, "berry_bush"), (berry, "berry")];
    let eat = |at| after_a_while(&[ScriptedAction::Eat { at }]);
    let mut world = world_in(
        data.clone(),
        &[".......", ".......", ".......", ".......", "......."],
        &objects,
        at(3, 2),
        hungry.clone(),
        &eat(berry),
    );
    run(&mut world, 120);
    assert_eq!(
        places(&world),
        [],
        "the berry is learned about, not remembered where"
    );
    let mut world = world_in(
        data,
        &[".......", ".......", ".......", ".......", "......."],
        &objects,
        at(3, 2),
        hungry,
        &eat(bush),
    );
    world
        .start_object(bush, "mature", &[("fruit", 6)])
        .expect("the bush");
    run(&mut world, 120);
    let remembered = places(&world);
    assert_eq!(remembered.len(), 1, "{remembered:?}");
    assert_eq!(remembered[0].thing, Thing::from("berry_bush"));
    assert_eq!(remembered[0].at, bush);
    assert!(remembered[0].recall == 1.0, "{remembered:?}");
}

#[test]
fn a_drink_that_eases_nothing_leaves_no_place() {
    let data = builtin();
    let never_thirsty = genome(10.0, "", &data);
    let drink = [ScriptedAction::Drink { at: at(0, 2) }, ScriptedAction::Rest];
    let mut world = world_in(data, &ROOM, &[], at(1, 2), never_thirsty, &drink);
    run(&mut world, 20);
    assert_eq!(places(&world), []);
}

#[test]
fn a_sprite_sees_a_remembered_bush_is_gone_and_forgets_it() {
    // M2 design §7: a fig tree that dies of old age soon after the sprite
    // ate from it, while the sprite is away.
    let fig = r#"(id: 6, name: "fig_tree", plural: "fig trees",
         category: "bush", tags: [Solid, Fixture], size: Large, hardness: 1.0,
         counters: {"fruit": 6},
         stages: [(name: "fruiting", ticks: (150, 150), next: Expire)],
         verbs: { Eat: [RequireCounter("fruit", 1), AddCounter("fruit", -1),
                        Inject(Actor, "food", 0.3), Signal(Actor, "ate")] }),
    "#;
    let data = with_object(fig);
    let tree = at(2, 2);
    let hungry = genome(10.0, HUNGRY, &data);
    let mut script = after_a_while(&[ScriptedAction::Eat { at: tree }]);
    script.extend(along(3, 30));
    script.extend([ScriptedAction::Rest; 12]);
    // Then back, within sight of where the tree stood.
    script.extend(along(30, 6));
    let rows = ["...................................."; 5];
    let mut world = world_in(
        data,
        &rows,
        &[(tree, "fig_tree")],
        at(3, 2),
        hungry,
        &script,
    );
    world
        .start_object(tree, "fruiting", &[("fruit", 6)])
        .expect("the tree");
    run(&mut world, 200);
    assert!(world.object_at(tree).is_none(), "the tree has died");
    assert_eq!(places(&world).len(), 1, "it hasn't seen it's gone");
    run(&mut world, 100);
    assert_eq!(places(&world), []);
}

/// The built-in pack with the object type `entry` added.
fn with_object(entry: &str) -> DataPack {
    let sources: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let text = if path == "objects.ron" {
                text.replace(
                    "    // Pseudo types",
                    &format!("    {entry}\n    // Pseudo types"),
                )
            } else {
                text.to_string()
            };
            (path, text)
        })
        .collect();
    let sources: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    DataPack::from_sources(&sources).expect("a valid test pack")
}

#[test]
fn a_sprite_drinks_at_water_in_sight_rather_than_walk_to_water_it_remembers() {
    // Water at both ends of a long room: it drank at the west end, and is
    // thirsty again within sight of the east end.
    let rows = ["~.............................~"; 5];
    let data = builtin();
    let thirsty = thirsty(&data);
    let mut script = after_a_while(&[ScriptedAction::Drink { at: at(0, 2) }]);
    script.pop();
    script.extend(along(1, 26));
    // It waits there, in sight of the east end, until it's thirsty.
    script.extend([ScriptedAction::Rest; 7]);
    let mut world = world_in(data, &rows, &[], at(1, 2), thirsty, &script);
    assert_eq!(first_drink(&mut world, 200), Some(at(1, 2)));
    run(&mut world, 100);
    let here = world.sprites().next().expect("the sprite").pos();
    assert!(here.x >= 24, "{here:?} is out of sight of the east end");
    let mut drank_at = None;
    for _ in 0..1_000 {
        for event in world.step() {
            if let EventKind::ActionEnded {
                verb: Verb::Drink,
                outcome: Outcome::Applied,
                action,
                ..
            } = event.kind
            {
                drank_at = action.target;
            }
        }
        if drank_at.is_some() {
            break;
        }
    }
    assert!(
        matches!(drank_at, Some(Target::Water(pos)) if pos.x == 30),
        "{drank_at:?}"
    );
}

#[test]
fn a_trip_longer_than_the_action_timeout_gets_there_in_one_go() {
    // M2 design §7: a trip has as long as its walk takes, past the usual
    // 60 ticks. At 4 tenths of a tile a tick, 70 tiles take about 175.
    let rows =
        ["~..............................................................................."; 5];
    let data = builtin();
    let slow = genome(
        4.0,
        r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.002, chem: "thirst"),
           Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
           Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),
           BrainParam(param: "tau_base", value: 0.05),"#,
        &data,
    );
    let mut script = vec![ScriptedAction::Rest; 20];
    script.push(ScriptedAction::Drink { at: at(0, 2) });
    script.extend(along(1, 76));
    let mut world = world_in(data, &rows, &[], at(1, 2), slow, &script);
    assert_eq!(first_drink(&mut world, 300), Some(at(1, 2)));
    let set_off = first_drink(&mut world, 2_000).expect("it goes back and drinks");
    assert!(set_off.x >= 70, "it set off from {set_off:?}");
}

#[test]
fn remembered_places_are_saved_and_loaded() {
    // M2 design §7: they're world state.
    let mut world = drank_and_wandered_off();
    first_drink(&mut world, 200);
    // The relief is learned the tick after the drink.
    run(&mut world, 5);
    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(places(&loaded), places(&world));
    assert_eq!(places(&loaded).len(), 1);
    assert_eq!(loaded.state_hash(), world.state_hash());
}
