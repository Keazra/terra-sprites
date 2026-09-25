//! Sprites with a body (design §3.2, §4): the first population, and life and
//! death without food or water.

use std::collections::BTreeSet;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, Genome, Map, Pos, Scenario, World, WorldConfig,
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

/// A death: the tick it happened in, who died, and the `Died` event's fields.
#[derive(Debug)]
struct Death {
    tick: u64,
    id: EntityId,
    name: String,
    cause: DeathCause,
    age: u64,
}

/// Steps `world` until a sprite dies, for at most `limit` ticks.
fn first_death(world: &mut World, limit: u64) -> Death {
    for _ in 0..limit {
        for event in world.step() {
            if let EventKind::Died {
                id,
                name,
                cause,
                age,
            } = event.kind
            {
                return Death {
                    tick: event.tick,
                    id,
                    name,
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
    let (id, name) = {
        let sprite = world.sprites().next().expect("the sprite");
        (sprite.id(), sprite.name())
    };
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
    assert_eq!((death.id, death.name.as_str()), (id, name.as_str()));
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
