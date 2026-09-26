//! The brain at work (design §5): sprites made from the starter genome eat
//! when hungry, drink when thirsty and wander when content, and change their
//! minds when a drive grows.

use terra_sim::{
    DataPack, EntityId, Event, EventKind, Genome, Map, Outcome, Pos, Scenario, Verb, World,
};

const STARTER: &str = include_str!("../../../data/genomes/starter.ron");

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// The starter genome, without spawn variation, with `extra` genes after its own.
fn starter_with(extra: &[&str], data: &DataPack) -> Genome {
    let genes: String = extra.iter().map(|g| format!("        {g},\n")).collect();
    let text = STARTER.replace("    ],\n)", &format!("{genes}    ],\n)"));
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `objects`, and a sprite on `pos` made
/// from `genome`; each bush in it starts mature with 6 fruit.
fn world(rows: &[&str], objects: &[(Pos, &str)], pos: Pos, genome: Genome, seed: u64) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(pos, Some(genome))];
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data, seed).expect("a valid scenario");
    for &(at, kind) in objects {
        if kind == "berry_bush" {
            world
                .start_object(at, "mature", &[("fruit", 6)])
                .expect("a bush");
        }
    }
    world
}

/// The actions sprite `id` ended in `events`, as `(verb, outcome)`.
fn ended(events: &[Event], id: EntityId) -> Vec<(Verb, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionEnded {
                id: who,
                verb,
                outcome,
                ..
            } if who == id => Some((verb, outcome)),
            _ => None,
        })
        .collect()
}

/// The verbs sprite `id` started in `events`.
fn started(events: &[Event], id: EntityId) -> Vec<Verb> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionStarted { id: who, verb } if who == id => Some(verb),
            _ => None,
        })
        .collect()
}

const FIELD: [&str; 7] = [
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
];

#[test]
fn a_hungry_sprite_walks_to_a_bush_and_eats() {
    for seed in 0..10 {
        let data = builtin();
        let hungry = starter_with(
            &[r#"InitialConcentration(chem: "hunger", value: 0.9)"#],
            &data,
        );
        let bush = at(8, 3);
        let mut world = world(&FIELD, &[(bush, "berry_bush")], at(1, 3), hungry, seed);
        let id = world.sprites().next().expect("the sprite").id();
        let ate = (0..100).any(|_| {
            let events = world.step();
            ended(&events, id).contains(&(Verb::Eat, Outcome::Applied))
        });
        assert!(ate, "seed {seed}: a hungry sprite eats");
        let fruit = world.object_at(bush).expect("the bush").counter("fruit");
        assert!(fruit < Some(6), "seed {seed}: {fruit:?}");
    }
}

#[test]
fn a_thirsty_sprite_walks_to_water_and_drinks() {
    let rows = [
        "...........",
        "...........",
        "........~~.",
        "........~~.",
        "...........",
    ];
    for seed in 0..10 {
        let data = builtin();
        let thirsty = starter_with(
            &[r#"InitialConcentration(chem: "thirst", value: 0.9)"#],
            &data,
        );
        let mut world = world(&rows, &[], at(1, 2), thirsty, seed);
        let id = world.sprites().next().expect("the sprite").id();
        let drank = (0..100).any(|_| {
            let events = world.step();
            ended(&events, id).contains(&(Verb::Drink, Outcome::Applied))
        });
        assert!(drank, "seed {seed}: a thirsty sprite drinks");
    }
}

#[test]
fn eating_and_drinking_relieve_hunger_and_thirst_the_next_tick() {
    let data = builtin();
    let needy = starter_with(
        &[
            r#"InitialConcentration(chem: "hunger", value: 0.9)"#,
            r#"InitialConcentration(chem: "thirst", value: 0.9)"#,
        ],
        &data,
    );
    let rows = ["........", "..~.....", "........"];
    let mut world = world(&rows, &[(at(4, 1), "berry_bush")], at(3, 1), needy, 1);
    let id = world.sprites().next().expect("the sprite").id();
    let mut relieved = Vec::new();
    for _ in 0..200 {
        let before = world.sprite(id).expect("the sprite");
        let (hunger, thirst) = (before.chemical("hunger"), before.chemical("thirst"));
        let events = world.step();
        for (verb, outcome) in ended(&events, id) {
            if outcome != Outcome::Applied || !matches!(verb, Verb::Eat | Verb::Drink) {
                continue;
            }
            let (drive, level) = match verb {
                Verb::Eat => ("hunger", hunger),
                _ => ("thirst", thirst),
            };
            let level = level.expect("a drive");
            world.step();
            let after = world.sprite(id).expect("the sprite").chemical(drive);
            let after = after.expect("a drive");
            // The pulse knocks the drive down by .5 at the next tick's step 3.
            assert!(after < level - 0.4, "{drive}: {level} to {after}");
            relieved.push(verb);
        }
        if relieved.contains(&Verb::Eat) && relieved.contains(&Verb::Drink) {
            return;
        }
    }
    panic!("it should both eat and drink: {relieved:?}");
}

#[test]
fn a_content_sprite_mostly_wanders() {
    let data = builtin();
    let content = starter_with(&[], &data);
    let mut world = world(&FIELD, &[(at(8, 3), "berry_bush")], at(1, 3), content, 3);
    let id = world.sprites().next().expect("the sprite").id();
    let mut verbs = Vec::new();
    for _ in 0..400 {
        verbs.extend(started(&world.step(), id));
    }
    let wanders = verbs.iter().filter(|&&v| v == Verb::Wander).count();
    assert!(verbs.len() >= 20, "{verbs:?}");
    assert!(
        wanders * 10 >= verbs.len() * 7,
        "{wanders} of {}: {verbs:?}",
        verbs.len()
    );
}

#[test]
fn a_growing_drive_interrupts_what_the_sprite_was_doing() {
    // Hunger climbs fast, from nothing, so eating soon outscores wandering by
    // more than the switch margin.
    let data = builtin();
    let starving = starter_with(
        &[r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.05, chem: "hunger")"#],
        &data,
    );
    let mut world = world(&FIELD, &[(at(9, 3), "berry_bush")], at(1, 3), starving, 2);
    let id = world.sprites().next().expect("the sprite").id();
    for _ in 0..40 {
        let events = world.step();
        let ends = ended(&events, id);
        if let Some(&(verb, _)) = ends.iter().find(|(_, o)| *o == Outcome::Interrupted) {
            assert_ne!(verb, Verb::Eat, "it changed its mind about something else");
            assert_eq!(started(&events, id), [Verb::Eat]);
            return;
        }
    }
    panic!("hunger should interrupt it");
}

#[test]
fn a_lone_sprite_with_a_bush_and_water_nearby_survives_10_000_ticks() {
    let rows = [
        "...............",
        "...............",
        "...........~~..",
        "...........~~..",
        "...............",
        "...............",
        "...............",
    ];
    for seed in [1, 2, 3] {
        let data = builtin();
        let map = Map::from_ascii(&rows, &data).expect("valid drawing");
        // The starter genome, with spawn variation, as the first population gets it.
        let sprites = [(at(7, 3), None)];
        let bush = at(3, 3);
        let scenario = Scenario {
            map,
            objects: &[(bush, "berry_bush")],
            sprites: &sprites,
            scripted: &[],
        };
        let mut world = World::from_scenario(scenario, data, seed).expect("a valid scenario");
        world
            .start_object(bush, "mature", &[("fruit", 6)])
            .expect("a bush");
        let id = world.sprites().next().expect("the sprite").id();
        let (mut meals, mut drinks) = (0, 0);
        for _ in 0..10_000 {
            let events = world.step();
            let died = events
                .iter()
                .find(|e| matches!(e.kind, EventKind::Died { .. }));
            assert!(died.is_none(), "seed {seed}: {died:?}");
            for ending in ended(&events, id) {
                match ending {
                    (Verb::Eat, Outcome::Applied) => meals += 1,
                    (Verb::Drink, Outcome::Applied) => drinks += 1,
                    _ => {}
                }
            }
        }
        let sprite = world.sprite(id).expect("alive");
        // Full energy lasts about 6,000 ticks and hydration about 3,000.
        assert!(
            meals > 0 && drinks > 0,
            "seed {seed}: it lived on its meals and drinks"
        );
        assert!(sprite.chemical("injury") < Some(0.5), "seed {seed}");
    }
}

#[test]
fn explain_gives_attention_scores_and_the_concepts_behind_the_verb() {
    let data = builtin();
    let genome = Genome::from_ron(
        r#"(format: 1, genes: [
            InitialConcentration(chem: "hunger", value: 0.8),
            BrainParam(param: "tau_base", value: 0.05),
            BrainParam(param: "tau_att_base", value: 0.05),
            AttentionInstinct(input: "hunger", category: Berry, weight: 1.0),
            Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
            Instinct(inputs: [("hunger", false), ("target_adjacent", true)], verb: Eat, weight: 0.5),
            Instinct(inputs: [("always", false)], verb: Eat, weight: -0.1),
            Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
        ])"#,
        &data,
    )
    .expect("a valid genome");
    let objects = [(at(8, 3), "berry_bush"), (at(8, 5), "berry")];
    let mut world = world(&FIELD, &objects, at(1, 3), genome, 1);
    let id = world.sprites().next().expect("the sprite").id();
    let sprite = world.sprite(id).expect("the sprite");
    assert!(sprite.explain().is_none(), "nothing decided before step 5");

    world.step();
    let sprite = world.sprite(id).expect("the sprite");
    let hunger = sprite.chemical("hunger").expect("hunger");
    let explained = sprite.explain().expect("step 5 ran");
    assert_eq!(explained.attended, Some("berry"));
    let categories: Vec<&str> = explained.attention.iter().map(|&(c, _)| c).collect();
    assert_eq!(categories, ["berry", "berry_bush"], "highest first");
    let (verb, score) = explained.decision.expect("a verb");
    assert_eq!(verb, Verb::Eat);
    // Largest first, whatever the sign: hunger, hunger and not there yet, always.
    let concepts: Vec<(Vec<(&str, bool)>, f32)> = explained
        .contributions
        .iter()
        .map(|c| (c.inputs.clone(), c.amount))
        .collect();
    assert_eq!(
        concepts,
        [
            (vec![("hunger", false)], hunger),
            (
                vec![("hunger", false), ("target_adjacent", true)],
                hunger * 0.5
            ),
            (vec![("always", false)], -0.1),
        ]
    );
    let total: f32 = concepts.iter().map(|(_, amount)| amount).sum();
    assert!((score - total).abs() < 1e-6, "{score} is the sum, {total}");
}

#[test]
fn attention_is_on_one_thing_the_map_can_show() {
    let data = builtin();
    let hungry = starter_with(
        &[r#"InitialConcentration(chem: "hunger", value: 0.9)"#],
        &data,
    );
    let bush = at(8, 3);
    let mut first = world(&FIELD, &[(bush, "berry_bush")], at(1, 3), hungry, 1);
    let id = first.sprites().next().expect("the sprite").id();
    assert_eq!(first.sprite(id).expect("the sprite").attending_to(), None);
    first.step();
    assert_eq!(
        first.sprite(id).expect("the sprite").attending_to(),
        Some(bush),
        "the bush, the only thing in reach"
    );

    // Water is many tiles, but attention is on one of them: the nearest.
    let rows = [
        "...........",
        "...........",
        "........~~.",
        "........~~.",
        "...........",
    ];
    let thirsty = starter_with(
        &[r#"InitialConcentration(chem: "thirst", value: 0.9)"#],
        &data,
    );
    let mut second = world(&rows, &[], at(1, 3), thirsty, 1);
    let id = second.sprites().next().expect("the sprite").id();
    second.step();
    let spot = second.sprite(id).expect("the sprite").attending_to();
    assert!(
        matches!(spot, Some(Pos { x: 8, y: 2..=3 })),
        "a nearest water tile: {spot:?}"
    );
}

#[test]
fn a_bored_sprite_goes_and_kicks_a_ball() {
    for seed in 0..10 {
        let data = builtin();
        let bored = starter_with(
            &[r#"InitialConcentration(chem: "boredom", value: 0.9)"#],
            &data,
        );
        let ball = at(8, 3);
        let mut world = world(&FIELD, &[(ball, "ball")], at(1, 3), bored, seed);
        let id = world.sprites().next().expect("the sprite").id();
        let kicked = (0..100).any(|_| {
            let events = world.step();
            ended(&events, id).contains(&(Verb::Play, Outcome::Applied))
        });
        assert!(kicked, "seed {seed}: a bored sprite kicks the ball");
    }
}

#[test]
fn a_lonely_sprite_goes_over_to_another_or_plays_with_it() {
    for seed in 0..10 {
        let data = builtin();
        let lonely = starter_with(
            &[r#"InitialConcentration(chem: "loneliness", value: 0.9)"#],
            &data,
        );
        let content = starter_with(&[], &data);
        let map = Map::from_ascii(&FIELD, &data).expect("valid drawing");
        let sprites = [(at(1, 3), Some(lonely)), (at(9, 3), Some(content))];
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted: &[],
        };
        let mut world = World::from_scenario(scenario, data, seed).expect("a valid scenario");
        let id = world.sprite_at(at(1, 3)).expect("the lonely sprite").id();
        let sought_company = (0..100).any(|_| {
            let events = world.step();
            ended(&events, id).iter().any(|&ending| {
                ending == (Verb::Approach, Outcome::Applied)
                    || ending == (Verb::Play, Outcome::Applied)
            })
        });
        assert!(sought_company, "seed {seed}: a lonely sprite seeks company");
    }
}
