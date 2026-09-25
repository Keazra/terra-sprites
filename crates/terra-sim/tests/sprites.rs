//! Sprites with a body (design §3.2, §4): the first population, and life and
//! death without food or water.

use std::collections::BTreeSet;

use terra_sim::{
    ChemicalKind, DataPack, DeathCause, EmitterMode, EntityId, EventKind, Expression, GeneView,
    Genome, Map, Pos, Scenario, Traits, World, WorldConfig,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

#[test]
fn a_new_world_places_the_preset_s_sprites_after_its_objects_on_empty_walkable_tiles() {
    let data = builtin();
    let world = World::new(WorldConfig::builtin(&data), data, 3);
    let sprites: Vec<_> = world.sprites().collect();
    assert_eq!(sprites.len(), 30, "the built-in preset's count");
    let last_object = world.objects().map(|o| o.id()).max().expect("objects");
    let mut tiles = BTreeSet::new();
    for sprite in &sprites {
        assert!(sprite.id() > last_object, "sprites come after the objects");
        let pos = sprite.pos();
        let terrain = world.data().terrain(world.map().terrain(pos));
        assert!(terrain.step_cost().is_some(), "{pos:?} is walkable");
        assert!(world.object_at(pos).is_none(), "{pos:?} holds no object");
        assert!(tiles.insert(pos), "one sprite per tile");
    }
}

#[test]
fn the_first_population_starts_between_60_and_100_percent_full() {
    let data = builtin();
    let world = World::new(WorldConfig::builtin(&data), data, 3);
    let mut energies = BTreeSet::new();
    for sprite in world.sprites() {
        for chemical in ["energy", "hydration"] {
            let level = sprite.chemical(chemical).expect("a chemical");
            assert!((0.6..=1.0).contains(&level), "{chemical} at {level}");
        }
        energies.insert(sprite.chemical("energy").expect("energy").to_bits());
    }
    assert!(energies.len() > 20, "drawn, not all the same");
}

#[test]
fn a_preset_may_set_20_to_100_sprites_or_leave_them_out() {
    let data = builtin();
    let with =
        |sprites: &str| WorldConfig::from_ron(&format!("(width: 64, height: 64{sprites})"), &data);
    for bad in [", sprites: 19", ", sprites: 101"] {
        assert!(with(bad).is_err(), "{bad}");
    }
    let config = with(", sprites: 20").expect("valid preset");
    assert_eq!(World::new(config, data.clone(), 1).sprites().count(), 20);
    let config = with("").expect("valid preset");
    assert_eq!(
        World::new(config, data, 1).sprites().count(),
        0,
        "none unless asked for"
    );
}

/// The built-in pack's files with `file` changed: each `(from, to)` replaced.
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
    let sources: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    DataPack::from_sources(&sources).expect("a valid test pack")
}

/// A 9×9 field of grass with one newborn sprite in the middle, made from
/// `genome`, or the starter genome with spawn variation.
fn lone_sprite(data: DataPack, genome: Option<&str>) -> World {
    let map = Map::from_ascii(&["........."; 9], &data).expect("valid drawing");
    let genome = genome.map(|text| Genome::from_ron(text, &data).expect("a valid genome"));
    let sprites = [(Pos { x: 4, y: 4 }, genome)];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// A death: the tick it happened in, and the `Died` event's fields.
#[derive(Debug)]
struct Death {
    tick: u64,
    id: EntityId,
    cause: DeathCause,
    age: u64,
}

/// Steps `world` until a sprite dies, for at most `limit` ticks.
fn first_death(world: &mut World, limit: u64) -> Death {
    for _ in 0..limit {
        for event in world.step() {
            if let EventKind::Died { id, cause, age } = event.kind {
                return Death {
                    tick: event.tick,
                    id,
                    cause,
                    age,
                };
            }
        }
    }
    panic!("no sprite died within {limit} ticks");
}

#[test]
fn a_motionless_sprite_gets_hungry_and_thirsty_then_dies_of_dehydration() {
    let mut world = lone_sprite(builtin(), None);
    let id = world.sprites().next().expect("the sprite").id();
    for _ in 0..3_900 {
        world.step();
    }
    let sprite = world.sprites().next().expect("alive at tick 3,900");
    assert!(sprite.chemical("thirst").expect("thirst") > 0.9, "parched");
    assert!(
        sprite.chemical("hunger").expect("hunger") > 0.1,
        "getting hungry"
    );
    assert!(sprite.chemical("pain").expect("pain") > 0.0, "hurting");

    // Water lasts about 3,000 ticks, then injury kills in about 1,000 (design v7).
    let death = first_death(&mut world, 1_000);
    assert!((3_900..4_500).contains(&death.tick), "{death:?}");
    assert_eq!(death.id, id);
    assert_eq!(death.cause, DeathCause::Dehydration);
    assert_eq!(death.age, death.tick, "born on tick 0");
    assert_eq!(world.sprites().count(), 0, "gone by the end of the tick");
}

#[test]
fn a_motionless_sprite_that_never_thirsts_dies_of_starvation() {
    let data = builtin_changing(
        "physiology.ron",
        &[("hydration_loss: 0.00033", "hydration_loss: 0.0")],
    );
    let mut world = lone_sprite(data, None);
    // Energy lasts about 6,000 ticks at sense radius 10, then injury kills in about 1,000.
    let death = first_death(&mut world, 9_000);
    assert_eq!(death.cause, DeathCause::Starvation);
    assert!((6_000..8_000).contains(&death.age), "{death:?}");
}

#[test]
fn a_sprite_that_never_hungers_or_thirsts_dies_of_old_age() {
    let data = builtin_changing(
        "physiology.ron",
        &[
            ("basal: 0.0001,", "basal: 0.0,"),
            ("per_sense_tile: 0.0000067,", "per_sense_tile: 0.0,"),
            ("hydration_loss: 0.00033", "hydration_loss: 0.0"),
        ],
    );
    let genome = r#"(format: 1, genes: [Trait(trait: "lifespan", value: 20000.0)])"#;
    let mut world = lone_sprite(data, Some(genome));
    // Past its lifespan, a sprite dies within about 2,000 ticks.
    let death = first_death(&mut world, 23_000);
    assert_eq!(death.cause, DeathCause::OldAge);
    assert!((21_500..22_500).contains(&death.age), "{death:?}");
}

/// Object types for testing where solid objects may go.
const SOLIDS: &str = r#"[
    (id: 1, name: "shrub", category: BerryBush, tags: [Solid, Fixture]),
    (id: 2, name: "seed", category: Berry,
     stages: [(name: "dormant", ticks: (2, 2), next: Expire)],
     rules: [(trigger: OnExpire, do: [ReplaceWith("shrub")])]),
    (id: 3, name: "creeper", category: Thornbush, tags: [Solid, Fixture],
     rules: [(trigger: Every(1), do: [SpreadTo("creeper", 1, [])])]),
    (id: 4, name: "spawner", category: Ball,
     rules: [(trigger: Every(1), do: [SpawnNearby("shrub", 1)])]),
    (id: 5, name: "pebble", category: Ball),
]"#;

/// A 5×5 field of grass holding `objects` and newborn starter sprites on `sprites`.
fn field(objects: &[(Pos, &str)], sprites: &[Pos]) -> Result<World, terra_sim::ScenarioError> {
    let objects_ron = include_str!("../../../data/objects.ron");
    let data = builtin_changing("objects.ron", &[(objects_ron, SOLIDS)]);
    let map = Map::from_ascii(&["....."; 5], &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites.iter().map(|&pos| (pos, None)).collect();
    World::from_scenario(
        Scenario {
            map,
            objects,
            sprites: &sprites,
        },
        data,
        1,
    )
}

fn type_at(world: &World, pos: Pos) -> Option<String> {
    world.object_at(pos).map(|o| o.type_name().to_string())
}

const MIDDLE: Pos = Pos { x: 2, y: 2 };

#[test]
fn a_sprite_can_share_a_tile_with_an_item_but_not_a_solid_object() {
    assert!(field(&[(MIDDLE, "pebble")], &[MIDDLE]).is_ok());
    assert!(field(&[(MIDDLE, "shrub")], &[MIDDLE]).is_err());
}

#[test]
fn an_item_under_a_sprite_is_not_replaced_by_a_solid_object() {
    let mut control = field(&[(MIDDLE, "seed")], &[]).expect("valid");
    let mut world = field(&[(MIDDLE, "seed")], &[Pos { x: 0, y: 0 }, MIDDLE]).expect("valid");
    for _ in 0..5 {
        control.step();
        world.step();
    }
    assert_eq!(
        type_at(&control, MIDDLE).as_deref(),
        Some("shrub"),
        "without a sprite it sprouts"
    );
    assert_eq!(
        type_at(&world, MIDDLE),
        None,
        "the seed expired where it lay"
    );
}

#[test]
fn solid_objects_never_spread_or_spawn_onto_a_sprite() {
    for spreader in ["creeper", "spawner"] {
        let beside = Pos { x: 3, y: 2 };
        let mut world = field(&[(MIDDLE, spreader)], &[beside]).expect("valid");
        for _ in 0..200 {
            world.step();
        }
        assert_eq!(type_at(&world, beside), None, "{spreader}");
        let around = (1..=3)
            .flat_map(|y| (1..=3).map(move |x| Pos { x, y }))
            .filter(|&pos| pos != MIDDLE && pos != beside);
        for pos in around {
            assert!(type_at(&world, pos).is_some(), "{spreader} filled {pos:?}");
        }
    }
}

#[test]
fn the_first_population_starts_with_no_false_fall_in_energy_or_hydration() {
    // A starter genome that turns any real fall in energy or hydration into reward.
    let starter = r#"(format: 1, genes: [
        Emitter(locus: Chem("energy"), mode: Fall, threshold: 0.02, gain: 1.0, chem: "reward"),
        Emitter(locus: Chem("hydration"), mode: Fall, threshold: 0.02, gain: 1.0, chem: "reward"),
    ])"#;
    let starter_ron = include_str!("../../../data/genomes/starter.ron");
    let data = builtin_changing("genomes/starter.ron", &[(starter_ron, starter)]);
    let config =
        WorldConfig::from_ron("(width: 48, height: 48, sprites: 20)", &data).expect("valid preset");
    let mut world = World::new(config, data, 5);
    world.step();
    for sprite in world.sprites() {
        assert_eq!(sprite.chemical("reward"), Some(0.0), "{:?}", sprite.id());
    }
}

#[test]
fn a_sprite_shows_its_age_and_its_traits_as_its_body_has_them() {
    // Speed past its range is clamped to 12, and sense radius, with no gene,
    // takes the middle of its range, 10 (design §4.8).
    let genome = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 50.0),
        Trait(trait: "lifespan", value: 61234.5),
    ])"#;
    let mut world = lone_sprite(builtin(), Some(genome));
    assert_eq!(world.sprites().next().expect("the sprite").age(), 0);
    for _ in 0..12 {
        world.step();
    }
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.age(), 12);
    assert_eq!(
        sprite.traits(),
        Traits {
            speed: 12.0,
            sense_radius: 10.0,
            lifespan: 61234.5
        }
    );
}

#[test]
fn a_sprite_lists_its_chemicals_in_pack_order_with_their_kinds() {
    use ChemicalKind::{Drive, Hormone, LearningSignal, Physical};
    let world = lone_sprite(builtin(), None);
    let sprite = world.sprites().next().expect("the sprite");
    let listed: Vec<(String, ChemicalKind)> = sprite
        .chemicals()
        .map(|c| (c.name.to_string(), c.kind))
        .collect();
    let mut expected: Vec<(String, ChemicalKind)> = [
        ("energy", Physical),
        ("hydration", Physical),
        ("stamina", Physical),
        ("food", Physical),
        ("water", Physical),
        ("injury", Physical),
        ("hunger", Drive),
        ("thirst", Drive),
        ("pain", Drive),
        ("tiredness", Drive),
        ("boredom", Drive),
        ("loneliness", Drive),
        ("crowdedness", Drive),
        ("reward", LearningSignal),
        ("punishment", LearningSignal),
    ]
    .into_iter()
    .map(|(name, kind)| (name.to_string(), kind))
    .collect();
    expected.extend((0..16).map(|n| (format!("h{n}"), Hormone)));
    assert_eq!(listed, expected);
}

#[test]
fn each_chemical_s_change_is_its_level_now_less_its_level_one_tick_ago() {
    let mut world = lone_sprite(builtin(), None);
    let newborn = world.sprites().next().expect("the sprite");
    assert!(
        newborn.chemicals().all(|c| c.change == 0.0),
        "nothing has changed at birth"
    );
    for _ in 0..50 {
        world.step();
    }
    let levels = |world: &World| -> Vec<f32> {
        let sprite = world.sprites().next().expect("the sprite");
        sprite.chemicals().map(|c| c.level).collect()
    };
    let before = levels(&world);
    world.step();
    let sprite = world.sprites().next().expect("the sprite");
    for (chemical, before) in sprite.chemicals().zip(before) {
        assert_eq!(
            chemical.change,
            chemical.level - before,
            "{}",
            chemical.name
        );
    }
    // At rest, hydration falls by physiology's 0.00033 a tick (Appendix B).
    let hydration = sprite
        .chemicals()
        .find(|c| c.name == "hydration")
        .expect("hydration");
    assert!(
        (hydration.change + 0.00033).abs() < 1e-6,
        "{}",
        hydration.change
    );
}

#[test]
fn a_sprite_shows_each_gene_by_name_with_how_it_is_expressed() {
    let genome = r#"(format: 1, genes: [
        HalfLife(chem: "pain", ticks: 20),
        HalfLife(chem: "pain", ticks: 30),
        Emitter(locus: Locus("nearby_sprites"), mode: Level, invert: true, threshold: 0.5, gain: 0.0005, chem: "loneliness"),
        Emitter(locus: Chem("hunger"), mode: Fall, threshold: 0.02, gain: 1.0, chem: "reward"),
        Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food"),
        Reaction(reactants: [("h0", 2)], products: [("h1", 1), ("h2", 1)], rate: 0.1),
        Receptor(chem: "reward", threshold: 0.2, gain: 0.5, target: "learning_rate_mod"),
        InitialConcentration(chem: "boredom", value: 0.2),
        Trait(trait: "sense_radius", value: 9.5),
        Gene(type: 900, version: 1, payload: "c0ffee"),
    ])"#;
    let world = lone_sprite(builtin(), Some(genome));
    let sprite = world.sprites().next().expect("the sprite");
    use Expression::{Expressed, Flagged, Unexpressed, Unknown};
    assert_eq!(
        sprite.genes(),
        [
            (
                GeneView::HalfLife {
                    chem: "pain",
                    ticks: 20
                },
                Expressed
            ),
            (
                GeneView::HalfLife {
                    chem: "pain",
                    ticks: 30
                },
                Unexpressed
            ),
            (
                GeneView::Emitter {
                    locus: "nearby_sprites",
                    mode: EmitterMode::Level,
                    invert: true,
                    threshold: 0.5,
                    gain: 0.0005,
                    chem: "loneliness"
                },
                Expressed
            ),
            (
                GeneView::Emitter {
                    locus: "hunger",
                    mode: EmitterMode::Fall,
                    invert: false,
                    threshold: 0.02,
                    gain: 1.0,
                    chem: "reward"
                },
                Expressed
            ),
            (
                GeneView::Emitter {
                    locus: "ate",
                    mode: EmitterMode::Level,
                    invert: false,
                    threshold: 0.0,
                    gain: 0.3,
                    chem: "food"
                },
                Flagged(
                    "writes food, but only physiology and verbs change a physical chemical".into()
                )
            ),
            (
                GeneView::Reaction {
                    reactants: vec![("h0", 2)],
                    products: vec![("h1", 1), ("h2", 1)],
                    rate: 0.1
                },
                Expressed
            ),
            (
                GeneView::Receptor {
                    chem: "reward",
                    threshold: 0.2,
                    gain: 0.5,
                    target: "learning_rate_mod"
                },
                Expressed
            ),
            (
                GeneView::InitialConcentration {
                    chem: "boredom",
                    value: 0.2
                },
                Expressed
            ),
            (
                GeneView::Trait {
                    name: "sense_radius",
                    value: 9.5
                },
                Expressed
            ),
            (
                GeneView::Unknown {
                    type_id: 900,
                    version: 1,
                    bytes: 3
                },
                Unknown
            ),
        ]
    );
}

#[test]
fn the_world_counts_deaths_by_cause() {
    let data = builtin_changing(
        "physiology.ron",
        &[("hydration_loss: 0.00033", "hydration_loss: 0.5")],
    );
    let mut world = lone_sprite(data, None);
    let counts = |world: &World| {
        [
            DeathCause::Starvation,
            DeathCause::Dehydration,
            DeathCause::OldAge,
        ]
        .map(|cause| world.deaths(cause))
    };
    assert_eq!(counts(&world), [0, 0, 0]);
    let death = first_death(&mut world, 3_000);
    assert_eq!(death.cause, DeathCause::Dehydration);
    assert_eq!(counts(&world), [0, 1, 0]);
    world.step();
    assert_eq!(counts(&world), [0, 1, 0], "the count outlives the sprite");
}
