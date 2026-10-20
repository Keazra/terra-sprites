//! The population record (M2 design §14): the world counts its sprites by
//! life stage, and its eggs, every 100 ticks over its whole run, with the
//! layings, the hatchings, the sprites the Cursor spawned and the deaths
//! between counts, and the oldest and newest generations alive (#208); it's
//! world state, so a save keeps it, and it squeezes its counts to stay
//! small.

mod common;
use common::{
    ALONE, FEMALE, WorldBuilder, at, default_world, family_in, laid, lone, replayed, run_for, until,
};

use std::collections::BTreeMap;

use terra_sim::{
    Command, EventKind, Generations, LifeStage, PopulationChanges, PopulationCount, World,
};

/// The eggs and sprites in `world` now, by life stage, in the order a count
/// keeps them: eggs, young, adults, old.
fn stages(world: &World) -> [u32; 4] {
    let mut n = [world.eggs().count() as u32, 0, 0, 0];
    for sprite in world.sprites() {
        n[match sprite.life_stage() {
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

/// A grown female of the first generation, who may lay alone at once (M2
/// design §15.7).
fn mother() -> World {
    let female = format!("{FEMALE}{ALONE}");
    family_in(lone(), &[(at(0, 0), &female)], &[], 3)
}

/// A world of grass with no sprites, which steps quickly.
fn empty_world() -> World {
    WorldBuilder::new(&["....", "...."]).build()
}

/// One newborn starter sprite on a single tile of grass, with nothing to
/// eat or drink: it dies of thirst at about tick 4,000.
fn doomed_sprite() -> World {
    WorldBuilder::new(&["."])
        .starters(&[at(0, 0)])
        .newborns(&[at(0, 0)])
        .build()
}

#[test]
fn a_new_world_counts_its_first_sprites_then_every_100_ticks() {
    let mut world = default_world(7);
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
    assert_eq!(first.generations, generations(1, 1), "all of the first");
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
    assert_eq!(count.generations, None, "no sprites, no generations");
    assert_eq!(all_changes(&world).deaths(), 1, "counted once");
    let all_died: u64 = world.deaths_by_cause().map(|(_, n)| n).sum();
    assert_eq!(all_died, 1);
}

#[test]
fn an_egg_is_counted_as_an_egg_and_its_laying_and_hatching_once_each() {
    let mut world = mother();
    until(&mut world, 3_000, |event| {
        matches!(event.kind, EventKind::Laid { .. }).then_some(())
    })
    .expect("she lays alone");
    let now = world.population_now();
    assert_eq!((now.eggs, now.sprites(), now.with_eggs()), (1, 1, 2));
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
fn an_egg_the_cursor_holds_is_counted_as_one_on_the_map_is() {
    // M2 design §14.1: the eggs are counted whether on the map or held by
    // the Cursor.
    let mut world = mother();
    let egg = laid(&mut world, 3_000).expect("she lays alone").egg;
    world.submit(Command::PickUp { item: egg });
    world.step();
    assert_eq!(
        world.egg(egg).map(|egg| egg.pos()),
        Some(None),
        "the Cursor holds it"
    );
    assert_eq!(world.population_now().eggs, 1);
    while !world.tick().is_multiple_of(100) {
        world.step();
    }
    let last = world.population().last().expect("a count");
    assert_eq!(last.tick, world.tick());
    assert_eq!(last.eggs, 1, "a count taken while it's held");
}

/// The generations `oldest` to `newest`, as a count keeps them.
fn generations(oldest: u32, newest: u32) -> Option<Generations> {
    Some(Generations { oldest, newest })
}

#[test]
fn a_count_keeps_the_oldest_and_newest_generations_of_its_sprites() {
    // #208: a mother of the first generation, laying alone. Her egg is of
    // the second, but an egg isn't a sprite until it hatches (M2 design
    // §10.4).
    let mut world = mother();
    until(&mut world, 3_000, |event| {
        matches!(event.kind, EventKind::Laid { .. }).then_some(())
    })
    .expect("she lays alone");
    assert_eq!(world.population()[0].generations, generations(1, 1));
    assert_eq!(
        world.population_now().generations,
        generations(1, 1),
        "the egg isn't counted"
    );
    until(&mut world, 1_000, |event| {
        matches!(event.kind, EventKind::Hatched { .. }).then_some(())
    })
    .expect("it hatches");
    assert_eq!(world.population_now().generations, generations(1, 2));
    while !world.tick().is_multiple_of(100) {
        world.step();
    }
    let last = world.population().last().expect("a count");
    assert_eq!(last.tick, world.tick());
    assert_eq!(
        last.generations,
        generations(1, 2),
        "and the count keeps them"
    );
}

#[test]
fn a_replay_plays_the_record_back_with_its_generations() {
    // #208: the generations are in the record, which is world state, so a
    // replay checks them with the rest of it (design §2.7).
    let mut world = mother();
    world.start_recording();
    run_for(&mut world, 3_000);
    assert!(
        world
            .population()
            .iter()
            .any(|count| count.generations == generations(1, 2)),
        "her hatchling was counted"
    );
    let playback = replayed(&world);
    assert_eq!(playback.world().population(), world.population());
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
    let mut world = default_world(3);
    run_for(&mut world, 350);
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
    run_for(&mut world, 100);
    assert_eq!(ticks(&world), [61, 100]);
}

/// A save of schema 1 from before the record kept generations (the golden
/// save `lineage.rs` describes), written by the build that merged PR #201
/// on tick 105: its record has counts at ticks 0 and 100, and once it's
/// loaded, its sprites are of the second and third generations.
const GOLDEN_V1_BEFORE_LINEAGE: &[u8] = include_bytes!("golden/save-v1-before-lineage.tspr");

#[test]
fn a_save_from_before_generations_were_counted_has_none_in_its_counts_then() {
    let mut world = World::load(GOLDEN_V1_BEFORE_LINEAGE).expect("the save loads");
    let counted = |world: &World| -> Vec<(u64, Option<Generations>)> {
        world
            .population()
            .iter()
            .map(|count| (count.tick, count.generations))
            .collect()
    };
    assert_eq!(counted(&world), [(0, None), (100, None)]);
    assert_eq!(world.population_now().generations, generations(2, 3));
    while world.tick() < 200 {
        world.step();
    }
    let [.., (tick, after)] = counted(&world)[..] else {
        panic!("no counts");
    };
    assert_eq!(tick, 200);
    assert!(after.is_some(), "the counts after the load have them");
}

#[test]
fn a_long_run_squeezes_the_record_and_keeps_the_whole_run() {
    // 512 counts take it to tick 51,100, where it keeps every other one and
    // counts every 200 ticks; at 512 again, at tick 102,200, every 400.
    let mut world = empty_world();
    run_for(&mut world, 110_000);
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
