//! The step 2 engine: objects defined as data, living by their lifecycle rules
//! (design §3.5).

use terra_sim::{
    DataPack, EntityId, Event, EventKind, Map, Pos, Removal, Scenario, ScenarioError, World,
};

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// The built-in pack with `objects.ron` holding only `types`.
fn pack_with(types: &[&str]) -> DataPack {
    let objects = format!("[{}]", types.join(",\n"));
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            (
                path,
                if path == "objects.ron" {
                    &objects
                } else {
                    text
                },
            )
        })
        .collect();
    DataPack::from_sources(&sources).expect("valid test pack")
}

/// A world on a drawn map, with `objects` placed as `(x, y, type name)`.
fn try_scenario(
    data: &DataPack,
    rows: &[&str],
    objects: &[(u16, u16, &str)],
) -> Result<World, ScenarioError> {
    let map = Map::from_ascii(rows, data).expect("valid drawing");
    let objects: Vec<(Pos, &str)> = objects
        .iter()
        .map(|&(x, y, name)| (at(x, y), name))
        .collect();
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &[],
    };
    World::from_scenario(scenario, data.clone(), 7)
}

fn scenario(data: &DataPack, rows: &[&str], objects: &[(u16, u16, &str)]) -> World {
    try_scenario(data, rows, objects).expect("valid scenario")
}

/// A 7×5 field of grass.
const FIELD: [&str; 5] = [".......", ".......", ".......", ".......", "......."];

#[test]
fn a_scenario_places_objects_and_lists_them_in_ascending_id_order() {
    let world = scenario(
        &builtin(),
        &FIELD,
        &[(1, 1, "berry_bush"), (5, 2, "ball"), (0, 4, "berry")],
    );
    let listed: Vec<(EntityId, &str, Pos)> = world
        .objects()
        .map(|o| (o.id(), o.type_name(), o.pos()))
        .collect();
    assert_eq!(
        listed,
        [
            (EntityId(1), "berry_bush", at(1, 1)),
            (EntityId(2), "ball", at(5, 2)),
            (EntityId(3), "berry", at(0, 4)),
        ]
    );
    assert_eq!(world.object_at(at(5, 2)).map(|o| o.id()), Some(EntityId(2)));
    assert!(world.object_at(at(0, 0)).is_none());
    assert!(
        world.object_at(at(7, 3)).is_none(),
        "past the right wall, not the berry at (0, 4)"
    );
    assert!(world.object_at(at(0, 5)).is_none(), "below the bottom wall");
}

/// FIELD with a pool of shallow water at (1, 1) and a rock at (3, 1).
const POOL_AND_ROCK: [&str; 5] = [".......", ".~.#...", ".......", ".......", "......."];

#[test]
fn a_scenario_rejects_objects_that_break_the_placement_rules() {
    let pack = builtin();
    let cases: [&[(u16, u16, &str)]; 5] = [
        &[(1, 1, "berry_bush")],
        &[(3, 1, "berry")],
        &[(3, 2, "berry"), (3, 2, "ball")],
        &[(7, 1, "ball")], // past the right wall: not row 2's first tile
        &[(0, 5, "ball")], // below the bottom wall
    ];
    for objects in cases {
        let result = try_scenario(&pack, &POOL_AND_ROCK, objects);
        assert!(
            matches!(result, Err(ScenarioError::CantPlace { .. })),
            "{objects:?}: {:?}",
            result.err()
        );
    }
    for name in ["shrub", "water"] {
        let result = try_scenario(&pack, &FIELD, &[(3, 2, name)]);
        assert_eq!(
            result.err(),
            Some(ScenarioError::NotAnObjectType(name.into()))
        );
    }
}

#[test]
fn solid_objects_may_stand_side_by_side_and_items_in_shallow_water() {
    let objects = [
        (2, 2, "berry_bush"),
        (3, 2, "thornbush"),
        (2, 3, "berry_bush"),
        (1, 1, "berry"),
        (0, 0, "berry_bush"),
    ];
    assert!(try_scenario(&builtin(), &POOL_AND_ROCK, &objects).is_ok());
}

/// A type with two stages of fixed length: `a` for 3 ticks, then `b` for 2,
/// then it expires. `extra` adds fields.
fn sprout(extra: &str) -> String {
    format!(
        r#"(id: 1, name: "sprout", category: BerryBush,
            counters: {{"n": 100}},
            stages: [(name: "a", ticks: (3, 3), next: Stage("b")),
                     (name: "b", ticks: (2, 2), next: Expire)],
            {extra})"#
    )
}

/// The stage of the object on `pos`, or `None` if the tile is empty.
fn stage_at(world: &World, pos: Pos) -> Option<String> {
    world
        .object_at(pos)
        .map(|o| o.stage().expect("has stages").to_string())
}

#[test]
fn an_object_moves_through_its_stages_then_expires() {
    let mut world = scenario(&pack_with(&[&sprout("")]), &FIELD, &[(3, 2, "sprout")]);
    let mut seen = vec![stage_at(&world, at(3, 2))];
    let mut events = Vec::new();
    for _ in 0..6 {
        events = world.step();
        seen.push(stage_at(&world, at(3, 2)));
    }
    let a = Some("a".to_string());
    let b = Some("b".to_string());
    assert_eq!(
        seen,
        [a.clone(), a.clone(), a.clone(), a, b.clone(), b, None]
    );
    assert_eq!(
        events,
        [Event {
            tick: 5,
            kind: EventKind::ObjectRemoved {
                id: EntityId(1),
                object_type: "sprout".into(),
                reason: Removal::Expired,
            },
        }]
    );
}

/// A permanent type (no stages) with a counter `n` up to `max`. `extra` adds fields.
fn ticker(max: u16, extra: &str) -> String {
    format!(r#"(id: 2, name: "ticker", category: Ball, counters: {{"n": {max}}}, {extra})"#)
}

/// The counter `n` of the object on `pos`.
fn n_at(world: &World, pos: Pos) -> u16 {
    world
        .object_at(pos)
        .and_then(|o| o.counter("n"))
        .expect("an object with a counter n")
}

#[test]
fn every_fires_on_ticks_staggered_by_object_id() {
    let rule = r#"rules: [(trigger: Every(4), do: [AddCounter("n", 1)])]"#;
    let pack = pack_with(&[&ticker(100, rule)]);
    // Entity 1 fires when (tick + 1) % 4 == 0: ticks 3 and 7; entity 2 on ticks 2 and 6.
    let mut world = scenario(&pack, &FIELD, &[(1, 1, "ticker"), (5, 3, "ticker")]);
    let mut seen = Vec::new();
    for _ in 0..8 {
        world.step();
        seen.push((n_at(&world, at(1, 1)), n_at(&world, at(5, 3))));
    }
    assert_eq!(
        seen,
        [
            (0, 0),
            (0, 0),
            (0, 1),
            (1, 1),
            (1, 1),
            (1, 1),
            (1, 2),
            (2, 2)
        ]
    );
}

#[test]
fn add_counter_stays_between_zero_and_the_maximum() {
    let up = pack_with(&[&ticker(
        5,
        r#"rules: [(trigger: Every(1), do: [AddCounter("n", 3)])]"#,
    )]);
    let mut world = scenario(&up, &FIELD, &[(3, 2, "ticker")]);
    let mut seen = Vec::new();
    for _ in 0..3 {
        world.step();
        seen.push(n_at(&world, at(3, 2)));
    }
    assert_eq!(seen, [3, 5, 5]);

    let down = pack_with(&[&ticker(
        5,
        r#"rules: [(trigger: Every(1), do: [AddCounter("n", -2)])]"#,
    )]);
    let mut world = scenario(&down, &FIELD, &[(3, 2, "ticker")]);
    world.step();
    assert_eq!(n_at(&world, at(3, 2)), 0);
}

/// Steps `world` `ticks` times.
fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
}

/// A rule that counts, in `n`, the ticks on which `condition` holds.
fn count_when(condition: &str) -> String {
    format!(r#"rules: [(trigger: Every(1), if: [{condition}], do: [AddCounter("n", 1)])]"#)
}

#[test]
fn in_stage_holds_while_the_object_is_in_that_stage() {
    let pack = pack_with(&[&sprout(&count_when(r#"InStage("b")"#))]);
    let mut world = scenario(&pack, &FIELD, &[(3, 2, "sprout")]);
    run(&mut world, 5);
    assert_eq!(n_at(&world, at(3, 2)), 2, "stage b lasts ticks 3 and 4");
}

#[test]
fn counter_compares_a_counter_with_a_value() {
    // `n` counts up by one each tick; `m` counts the ticks on which n cmp 3 held.
    let expected = [
        ("Lt", 2),
        ("Le", 3),
        ("Eq", 1),
        ("Ne", 4),
        ("Ge", 3),
        ("Gt", 2),
    ];
    for (cmp, ticks) in expected {
        let counter = format!(
            r#"(id: 2, name: "ticker", category: Ball, counters: {{"n": 100, "m": 100}},
                rules: [(trigger: Every(1), do: [AddCounter("n", 1)]),
                        (trigger: Every(1), if: [Counter("n", {cmp}, 3)], do: [AddCounter("m", 1)])])"#
        );
        let mut world = scenario(&pack_with(&[&counter]), &FIELD, &[(3, 2, "ticker")]);
        run(&mut world, 5);
        let m = world.object_at(at(3, 2)).and_then(|o| o.counter("m"));
        assert_eq!(m, Some(ticks), "Counter(n, {cmp}, 3) as n goes 1 to 5");
    }
}

#[test]
fn fertility_compares_the_fertility_of_the_objects_tile() {
    let pack = pack_with(&[&ticker(100, &count_when("Fertility(Ge, 0.5)"))]);
    // Grass 1.0, dirt 0.5, sand 0.0.
    let mut world = scenario(
        &pack,
        &[".,:"],
        &[(0, 0, "ticker"), (1, 0, "ticker"), (2, 0, "ticker")],
    );
    world.step();
    let counts: Vec<u16> = (0..3).map(|x| n_at(&world, at(x, 0))).collect();
    assert_eq!(counts, [1, 1, 0]);
}

/// A permanent item type with no rules.
const PEBBLE: &str = r#"(id: 3, name: "pebble", category: Berry)"#;

#[test]
fn density_below_counts_objects_of_a_type_within_a_chebyshev_radius() {
    let cases = [
        (
            r#"DensityBelow("pebble", 1, 1)"#,
            0,
            "a pebble is 1 tile away",
        ),
        (
            r#"DensityBelow("pebble", 2, 2)"#,
            0,
            "two pebbles are within 2 tiles",
        ),
        (
            r#"DensityBelow("pebble", 2, 3)"#,
            1,
            "only two pebbles are within 2 tiles",
        ),
        (
            r#"DensityBelow("pebble", 0, 1)"#,
            1,
            "no pebble is on its own tile",
        ),
        (r#"DensityBelow("ticker", 0, 1)"#, 0, "it counts itself"),
    ];
    for (condition, expected, why) in cases {
        let pack = pack_with(&[&ticker(100, &count_when(condition)), PEBBLE]);
        let mut world = scenario(
            &pack,
            &FIELD,
            &[
                (3, 2, "ticker"),
                (4, 3, "pebble"),
                (1, 0, "pebble"),
                (6, 2, "pebble"),
            ],
        );
        world.step();
        assert_eq!(n_at(&world, at(3, 2)), expected, "{condition}: {why}");
    }
}

#[test]
fn chance_of_zero_never_holds_and_chance_of_one_always_does() {
    for (p, expected) in [("0.0", 0), ("1.0", 10)] {
        let pack = pack_with(&[&ticker(100, &count_when(&format!("Chance({p})")))]);
        let mut world = scenario(&pack, &FIELD, &[(3, 2, "ticker")]);
        run(&mut world, 10);
        assert_eq!(n_at(&world, at(3, 2)), expected, "Chance({p})");
    }
}

/// A world of one `ticker` whose rules are `rules`, on `rows`, after `ticks` steps.
fn ticker_world(rows: &[&str], pos: (u16, u16), rules: &str, ticks: u64) -> World {
    let pack = pack_with(&[&ticker(100, rules), PEBBLE]);
    let mut world = scenario(&pack, rows, &[(pos.0, pos.1, "ticker")]);
    run(&mut world, ticks);
    world
}

#[test]
fn conditions_stop_at_the_first_false_one_so_a_later_chance_draws_nothing() {
    let never = r#"Counter("n", Gt, 0)"#;
    let with_chance = format!(
        r#"rules: [(trigger: Every(1), if: [{never}, Chance(0.5)], do: [AddCounter("n", 1)])]"#
    );
    let without =
        format!(r#"rules: [(trigger: Every(1), if: [{never}], do: [AddCounter("n", 1)])]"#);
    let drawing = r#"rules: [(trigger: Every(1), if: [Chance(0.5)], do: [])]"#;
    let hash = |rules: &str| ticker_world(&FIELD, (3, 2), rules, 10).state_hash();
    assert_eq!(hash(&with_chance), hash(&without));
    assert_ne!(
        hash(drawing),
        hash(&without),
        "a Chance that runs does draw"
    );
}

/// The tiles holding a pebble.
fn pebbles(world: &World) -> Vec<Pos> {
    world
        .objects()
        .filter(|o| o.type_name() == "pebble")
        .map(|o| o.pos())
        .collect()
}

/// A rule that spawns a pebble once, on the ticker's first turn.
fn spawn_once(effect: &str) -> String {
    format!(
        r#"rules: [(trigger: Every(1), if: [Counter("n", Eq, 0)], do: [AddCounter("n", 1), {effect}])]"#
    )
}

#[test]
fn spawn_nearby_creates_an_object_on_a_tile_where_it_may_go() {
    let boxed_in = [
        "#####", //
        "#..##", //
        "#####", //
    ];
    let world = ticker_world(
        &boxed_in,
        (1, 1),
        &spawn_once(r#"SpawnNearby("pebble", 1)"#),
        1,
    );
    assert_eq!(pebbles(&world), [at(2, 1)], "the only free tile in reach");
}

#[test]
fn spawn_nearby_chooses_uniformly_among_the_candidates() {
    let mut chosen = std::collections::BTreeSet::new();
    for seed in 0..100 {
        let pack = pack_with(&[
            &ticker(100, &spawn_once(r#"SpawnNearby("pebble", 1)"#)),
            PEBBLE,
        ]);
        let map = Map::from_ascii(&FIELD, &pack).expect("valid drawing");
        let scenario = Scenario {
            map,
            objects: &[(at(3, 2), "ticker")],
            sprites: &[],
        };
        let mut world = World::from_scenario(scenario, pack, seed).expect("valid");
        world.step();
        chosen.extend(pebbles(&world));
    }
    let neighbours: std::collections::BTreeSet<Pos> = (1..=3)
        .flat_map(|y| (2..=4).map(move |x| at(x, y)))
        .filter(|&p| p != at(3, 2))
        .collect();
    assert_eq!(
        chosen, neighbours,
        "every free neighbour gets chosen, and nothing else"
    );
}

#[test]
fn spawn_nearby_with_no_candidates_does_nothing_and_draws_nothing() {
    let sealed = [
        "###", //
        "#.#", //
        "###", //
    ];
    let spawning = ticker_world(
        &sealed,
        (1, 1),
        &spawn_once(r#"SpawnNearby("pebble", 1)"#),
        3,
    );
    let not_spawning = ticker_world(&sealed, (1, 1), &spawn_once(r#"AddCounter("n", 0)"#), 3);
    assert!(pebbles(&spawning).is_empty());
    assert_eq!(spawning.state_hash(), not_spawning.state_hash());
}

#[test]
fn a_spawned_object_gets_the_next_id_and_is_reported() {
    let boxed_in = [
        "#####", //
        "#..##", //
        "#####", //
    ];
    let pack = pack_with(&[
        &ticker(100, &spawn_once(r#"SpawnNearby("pebble", 1)"#)),
        PEBBLE,
    ]);
    let mut world = scenario(&pack, &boxed_in, &[(1, 1, "ticker")]);
    let events = world.step();
    assert_eq!(
        events,
        [Event {
            tick: 0,
            kind: EventKind::ObjectSpawned {
                id: EntityId(2),
                object_type: "pebble".into(),
                pos: at(2, 1),
            },
        }]
    );
}

#[test]
fn spread_to_lands_within_the_square_where_placement_and_the_conditions_allow() {
    // Grass in columns 0–4, sand in 5–8; the ticker is at (4, 3) of 7 rows.
    let half_sand = [".....::::"; 7];
    let rules =
        r#"rules: [(trigger: Every(1), do: [SpreadTo("pebble", 2, [Fertility(Ge, 0.5)])])]"#;
    let world = ticker_world(&half_sand, (4, 3), rules, 60);
    let spread = pebbles(&world);
    assert!(!spread.is_empty(), "60 draws put down some pebbles");
    for pos in spread {
        assert!(
            (2..=6).contains(&pos.x) && (1..=5).contains(&pos.y),
            "{pos:?} is within 2 of (4, 3)"
        );
        assert!(pos.x <= 4, "{pos:?} is on fertile grass");
    }
}

#[test]
fn spread_to_draws_once_whatever_the_square_holds() {
    // A spread that never lands, over squares of different sizes: the draws
    // (and so the state) are the same each time, and differ from not spreading.
    let spread = |radius: u16, times: usize| {
        let effect = format!(r#"SpreadTo("pebble", {radius}, [Fertility(Gt, 2.0)])"#);
        let effects = vec![effect; times].join(", ");
        let rules = format!("rules: [(trigger: Every(1), do: [{effects}])]");
        ticker_world(&FIELD, (3, 2), &rules, 5).state_hash()
    };
    assert_eq!(
        spread(0, 1),
        spread(3, 1),
        "one draw, however big the square"
    );
    assert_ne!(spread(1, 1), spread(1, 2), "two spreads draw twice");
    let idle = ticker_world(&FIELD, (3, 2), "rules: [(trigger: Every(1), do: [])]", 5);
    assert_ne!(spread(1, 1), idle.state_hash(), "a spread draws");
}

/// A solid fixture with no rules unless `extra` adds some.
fn shrub(extra: &str) -> String {
    format!(
        r#"(id: 4, name: "shrub", category: Thornbush, tags: [Solid, Fixture],
            counters: {{"n": 100}},
            stages: [(name: "shoot", ticks: (5, 5), next: Stage("grown")),
                     (name: "grown", ticks: (5, 5), next: Expire)],
            {extra})"#
    )
}

/// An item that lasts 2 ticks, then runs `on_expire`.
fn seed(on_expire: &str) -> String {
    format!(
        r#"(id: 5, name: "seed", category: Berry,
            stages: [(name: "ripe", ticks: (2, 2), next: Expire)],
            rules: [(trigger: OnExpire, do: [{on_expire}])])"#
    )
}

#[test]
fn replace_with_puts_a_new_object_at_its_first_stage_on_the_same_tile() {
    let counting = count_when(r#"InStage("shoot")"#);
    let pack = pack_with(&[&seed(r#"ReplaceWith("shrub")"#), &shrub(&counting)]);
    let mut world = scenario(&pack, &FIELD, &[(3, 2, "seed")]);
    run(&mut world, 2);
    let events = world.step();
    assert_eq!(
        events,
        [
            Event {
                tick: 2,
                kind: EventKind::ObjectRemoved {
                    id: EntityId(1),
                    object_type: "seed".into(),
                    reason: Removal::Replaced,
                },
            },
            Event {
                tick: 2,
                kind: EventKind::ObjectSpawned {
                    id: EntityId(2),
                    object_type: "shrub".into(),
                    pos: at(3, 2),
                },
            },
        ]
    );
    let shrub = world.object_at(at(3, 2)).expect("the shrub");
    assert_eq!(shrub.stage(), Some("shoot"));
    assert_eq!(shrub.counter("n"), Some(0), "it first runs next tick");
    world.step();
    assert_eq!(n_at(&world, at(3, 2)), 1);
}

#[test]
fn a_replacement_that_cant_go_there_leaves_the_object_as_it_was() {
    // Shallow water doesn't allow fixtures, so a seed there can't become a shrub.
    let pack = pack_with(&[&seed(r#"ReplaceWith("shrub")"#), &shrub("")]);
    let mut world = scenario(&pack, &POOL_AND_ROCK, &[(1, 1, "seed")]);
    run(&mut world, 2);
    let events = world.step();
    assert!(
        matches!(
            events.as_slice(),
            [Event {
                kind: EventKind::ObjectRemoved {
                    reason: Removal::Expired,
                    ..
                },
                ..
            }]
        ),
        "the seed simply expires: {events:?}"
    );

    let every_tick =
        r#"rules: [(trigger: Every(1), do: [ReplaceWith("shrub"), AddCounter("n", 1)])]"#;
    let pack = pack_with(&[&ticker(100, every_tick), &shrub("")]);
    let mut world = scenario(&pack, &POOL_AND_ROCK, &[(1, 1, "ticker")]);
    world.step();
    assert_eq!(n_at(&world, at(1, 1)), 1, "the ticker stays and carries on");
}

#[test]
fn a_replacement_ends_the_replaced_objects_turn() {
    let rules =
        r#"rules: [(trigger: Every(1), do: [ReplaceWith("shrub"), SpawnNearby("pebble", 1)])]"#;
    let pack = pack_with(&[&ticker(100, rules), &shrub(""), PEBBLE]);
    let mut world = scenario(&pack, &FIELD, &[(3, 2, "ticker")]);
    world.step();
    let names: Vec<&str> = world.objects().map(|o| o.type_name()).collect();
    assert_eq!(names, ["shrub"], "no pebble after the replacement");
}

#[test]
fn destroy_self_removes_the_object_and_ends_its_turn() {
    let rules = r#"rules: [(trigger: Every(1), do: [DestroySelf, SpawnNearby("pebble", 1)])]"#;
    let pack = pack_with(&[&ticker(100, rules), PEBBLE]);
    let mut world = scenario(&pack, &FIELD, &[(3, 2, "ticker")]);
    let events = world.step();
    assert!(world.objects().next().is_none(), "no ticker and no pebble");
    assert_eq!(
        events,
        [Event {
            tick: 0,
            kind: EventKind::ObjectRemoved {
                id: EntityId(1),
                object_type: "ticker".into(),
                reason: Removal::Destroyed,
            },
        }]
    );
}

#[test]
fn on_stage_enter_fires_when_a_stage_begins_including_the_first() {
    let first = pack_with(&[&sprout(
        r#"rules: [(trigger: OnStageEnter("a"), do: [AddCounter("n", 1)])]"#,
    )]);
    let mut world = scenario(&first, &FIELD, &[(3, 2, "sprout")]);
    run(&mut world, 4);
    assert_eq!(n_at(&world, at(3, 2)), 1, "once, on its first turn");

    let second = pack_with(&[&sprout(
        r#"rules: [(trigger: OnStageEnter("b"), do: [AddCounter("n", 1)])]"#,
    )]);
    let mut world = scenario(&second, &FIELD, &[(3, 2, "sprout")]);
    run(&mut world, 3);
    assert_eq!(n_at(&world, at(3, 2)), 0, "stage b hasn't begun");
    world.step();
    assert_eq!(n_at(&world, at(3, 2)), 1, "stage b begins on tick 3");
}

#[test]
fn an_expiring_object_runs_only_its_on_expire_rules() {
    // `n` counts the turns; the pebble is dropped on expiry only if the Every
    // rule didn't also run on the expiring turn. Stages a and b last 5 turns.
    let rules = r#"rules: [(trigger: Every(1), do: [AddCounter("n", 1)]),
                           (trigger: OnExpire, if: [Counter("n", Eq, 5)], do: [SpawnNearby("pebble", 1)])]"#;
    let pack = pack_with(&[&sprout(rules), PEBBLE]);
    let mut world = scenario(&pack, &FIELD, &[(3, 2, "sprout")]);
    run(&mut world, 6);
    assert!(world.object_at(at(3, 2)).is_none(), "the sprout expired");
    assert_eq!(pebbles(&world).len(), 1);
}

#[test]
fn the_visual_state_is_the_first_visual_rule_that_matches_or_default() {
    let fields = r#"rules: [(trigger: Every(1), if: [InStage("b")], do: [AddCounter("n", 1)])],
                    visual: [(if: [InStage("a")], state: "young"),
                             (if: [Counter("n", Ge, 2)], state: "full")]"#;
    let mut world = scenario(&pack_with(&[&sprout(fields)]), &FIELD, &[(3, 2, "sprout")]);
    let mut seen = Vec::new();
    for _ in 0..5 {
        world.step();
        let object = world.object_at(at(3, 2)).expect("the sprout");
        seen.push(object.visual_state().to_string());
    }
    assert_eq!(seen, ["young", "young", "young", "default", "full"]);
}

#[test]
fn the_built_in_berry_bush_looks_like_a_seedling_then_bare_then_fruiting() {
    let mut world = scenario(&builtin(), &FIELD, &[(3, 2, "berry_bush")]);
    let look = |world: &World| {
        world
            .object_at(at(3, 2))
            .expect("the bush")
            .visual_state()
            .to_string()
    };
    assert_eq!(look(&world), "seedling");
    let mut looks = vec![look(&world)];
    for _ in 0..3_000 {
        world.step();
        if looks.last() != Some(&look(&world)) {
            looks.push(look(&world));
        }
    }
    assert_eq!(looks, ["seedling", "default", "fruiting"]);
}

#[test]
fn keeps_paths_open_holds_where_a_solid_object_would_not_split_the_open_tiles_around() {
    let pack = pack_with(&[&ticker(100, &count_when("KeepsPathsOpen")), PEBBLE]);
    let corridor = [
        "#####", //
        ".....", //
        "#####", //
    ];
    let mut plugging = scenario(&pack, &corridor, &[(2, 1, "ticker")]);
    plugging.step();
    assert_eq!(n_at(&plugging, at(2, 1)), 0, "it would plug the corridor");

    let mut open = scenario(&pack, &FIELD, &[(3, 2, "ticker")]);
    open.step();
    assert_eq!(n_at(&open, at(3, 2)), 1, "in open ground");
}
