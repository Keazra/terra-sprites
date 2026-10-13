//! The population record (M2 design §14): the world counts its sprites by
//! life stage every 100 ticks over its whole run, with the hatchings, the
//! sprites the Cursor spawned and the deaths between counts; it's world
//! state, so a save keeps it, and it squeezes its counts to stay small.

mod common;
use common::{ALONE, FEMALE, WorldBuilder, at, builtin, family, until};

use std::collections::BTreeMap;

use terra_sim::{
    Command, EventKind, LifeStage, PopulationChanges, PopulationCount, World, WorldConfig,
};

/// The sprites in `world` now, by life stage, in the order a count keeps
/// them: eggs, young, adults, old.
fn stages(world: &World) -> [u32; 4] {
    let mut n = [0; 4];
    for sprite in world.sprites() {
        n[match sprite.life_stage() {
            LifeStage::Egg => 0,
            LifeStage::Young => 1,
            LifeStage::Adult => 2,
            LifeStage::Old => 3,
        }] += 1;
    }
    n
}

fn stages_of(count: &PopulationCount) -> [u32; 4] {
    [count.eggs, count.young, count.adults, count.old]
}

fn ticks(world: &World) -> Vec<u64> {
    world.population().iter().map(|count| count.tick).collect()
}

/// Everything that happened in the record's stretches, and since its last
/// count.
fn all_changes(world: &World) -> PopulationChanges {
    let mut all = PopulationChanges::default();
    for count in world.population() {
        all.add(&count.since);
    }
    all.add(&world.population_now().since);
    all
}

/// A world of grass with no sprites, which steps quickly.
fn empty_world() -> World {
    WorldBuilder::new(&["....", "...."]).build()
}

/// One newborn starter sprite on a single tile of grass, with nothing to
/// eat or drink: it dies of thirst at about tick 4,000.
fn doomed_sprite() -> World {
    WorldBuilder::new(&["."])
        .sprites_with(vec![(at(0, 0), None)])
        .newborns(&[at(0, 0)])
        .build()
}

#[test]
fn a_new_world_counts_its_first_sprites_then_every_100_ticks() {
    let data = builtin();
    let mut world = World::new(WorldConfig::builtin(&data), data, 7);
    assert!(
        world.population().is_empty(),
        "nothing counted before it steps"
    );
    let mut seen = BTreeMap::new();
    for _ in 0..250 {
        if world.tick().is_multiple_of(100) {
            seen.insert(world.tick(), stages(&world));
        }
        world.step();
    }
    assert_eq!(ticks(&world), [0, 100, 200]);
    for count in world.population() {
        assert_eq!(
            stages_of(count),
            seen[&count.tick],
            "at tick {}",
            count.tick
        );
    }
    let first = &world.population()[0];
    assert_eq!(first.sprites(), 30, "the default world's first sprites");
    assert_eq!(
        first.since,
        PopulationChanges::default(),
        "nothing before the first"
    );
    let now = world.population_now();
    assert_eq!(now.tick, 250);
    assert_eq!(stages_of(&now), stages(&world));
}

#[test]
fn a_death_is_counted_in_the_stretch_it_happened_in() {
    let mut world = doomed_sprite();
    let died = until(&mut world, 6_000, |event| match event.kind {
        EventKind::Died { cause, .. } => Some((event.tick, cause)),
        _ => None,
    });
    let (tick, cause) = died.expect("it dies of thirst");
    // Step on to the next count, so the death is in a count's stretch.
    while !world.tick().is_multiple_of(100) {
        world.step();
    }
    let count = world
        .population()
        .iter()
        .find(|count| count.tick > tick)
        .expect("a count after the death");
    assert_eq!(count.since.died, BTreeMap::from([(cause, 1)]));
    assert_eq!(count.sprites(), 0);
    assert_eq!(all_changes(&world).deaths(), 1, "counted once");
    let all_died: u64 = world.deaths_by_cause().map(|(_, n)| n).sum();
    assert_eq!(all_died, 1);
}

#[test]
fn an_egg_is_counted_as_an_egg_and_its_laying_and_hatching_once_each() {
    let mut world = family(&[(at(0, 0), &format!("{FEMALE}{ALONE}"))], &[], 3);
    until(&mut world, 3_000, |event| {
        matches!(event.kind, EventKind::Laid { .. }).then_some(())
    })
    .expect("she lays alone");
    assert_eq!(world.population_now().eggs, 1);
    assert_eq!(all_changes(&world).laid, 1);
    until(&mut world, 1_000, |event| {
        matches!(event.kind, EventKind::Hatched { .. }).then_some(())
    })
    .expect("it hatches");
    let now = world.population_now();
    assert_eq!((now.eggs, now.young), (0, 1));
    let all = all_changes(&world);
    assert_eq!((all.laid, all.hatched), (1, 1));
}

#[test]
fn a_sprite_the_cursor_spawned_is_counted_as_spawned() {
    let mut world = empty_world();
    world.submit(Command::SpawnSprite {
        tile: at(1, 1),
        genome: None,
    });
    world.step();
    let now = world.population_now();
    assert_eq!(now.since.spawned, 1);
    assert_eq!(now.sprites(), 1);
}

#[test]
fn a_save_keeps_the_record() {
    let data = builtin();
    let mut world = World::new(WorldConfig::builtin(&data), data, 3);
    for _ in 0..350 {
        world.step();
    }
    let mut loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.population(), world.population());
    assert_eq!(loaded.population_now(), world.population_now());
    for _ in 0..100 {
        world.step();
        loaded.step();
    }
    assert_eq!(ticks(&loaded), [0, 100, 200, 300, 400]);
    assert_eq!(loaded.population(), world.population());
}

#[test]
fn a_save_from_before_the_record_begins_it_where_it_was_loaded() {
    // Written when the schema was 1, before the record (save.rs).
    let mut world = World::load(include_bytes!("golden/save-v1.tspr")).expect("it loads");
    assert!(world.population().is_empty());
    assert_eq!(world.tick(), 61);
    let sprites = stages(&world);
    world.step();
    assert_eq!(ticks(&world), [61]);
    assert_eq!(stages_of(&world.population()[0]), sprites);
    for _ in 0..100 {
        world.step();
    }
    assert_eq!(ticks(&world), [61, 100]);
}

#[test]
fn a_long_run_squeezes_the_record_and_keeps_the_whole_run() {
    // 512 counts take it to tick 51,100, where it keeps every other one and
    // counts every 200 ticks; at 512 again, at tick 102,200, every 400.
    let mut world = empty_world();
    for _ in 0..110_000 {
        world.step();
    }
    assert_eq!(world.population_every(), 400);
    let ticks = ticks(&world);
    assert!(ticks.len() < 512, "{} counts", ticks.len());
    assert_eq!(ticks[0], 0, "the first count stays");
    assert!(
        ticks.iter().all(|tick| tick.is_multiple_of(400)),
        "{ticks:?}"
    );
    assert!(ticks.windows(2).all(|pair| pair[1] - pair[0] == 400));
    assert_eq!(ticks.last(), Some(&110_000), "and counts up to now");
}
