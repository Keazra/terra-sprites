//! Lineage (M2 design §16): each sprite and egg knows its generation and
//! where it came from, the world remembers each sprite that dies, with its
//! parents and how it died, and finds anyone's young, living or dead. The
//! record is world state, saved, hashed and replayed, but nothing in the
//! world reads it.

mod common;
use common::{
    ALONE, FEMALE, NO_COPYING_ERRORS, at, builtin_changing, family, family_in, hatch, laid, until,
};

use terra_sim::{
    Command, DeathCause, EntityId, EventKind, LifeStage, Origin, Playback, Sex, World,
};

/// The changes to `physiology.ron` that make thirst kill within a few
/// dozen ticks, so a sprite with no water dies soon after it hatches.
const PARCHED: [(&str, &str); 2] = [
    ("hydration_loss: 0.00033", "hydration_loss: 0.03"),
    ("dehydration: 0.0011", "dehydration: 0.5"),
];

/// A female who lays alone, with no water anywhere, so she and her young
/// die of thirst soon after they're born, her eggs copied exactly. Returns
/// the world and her ID.
fn mother_of(seed: u64) -> (World, EntityId) {
    let changes: Vec<_> = NO_COPYING_ERRORS.into_iter().chain(PARCHED).collect();
    let data = builtin_changing("physiology.ron", &changes);
    let mother = format!("{ALONE}{FEMALE}");
    let world = family_in(data, &[(at(2, 1), &mother)], &[], seed);
    let id = world.sprite_at(at(2, 1)).expect("the mother").id();
    (world, id)
}

/// Steps `world` until the sprite `id` dies, returning the tick it died on.
fn died(world: &mut World, id: EntityId, ticks: u64) -> u64 {
    let died = until(world, ticks, |event| match event.kind {
        EventKind::Died { id: dead, .. } if dead == id => Some(event.tick),
        _ => None,
    });
    died.expect("it died")
}

#[test]
fn the_first_sprites_are_the_first_generation_and_their_young_the_next() {
    let (mut world, mother) = mother_of(1);
    let sprite = world.sprite(mother).unwrap();
    assert_eq!((sprite.generation(), sprite.origin()), (1, Origin::First));
    let egg = laid(&mut world, 100).expect("an egg").egg;
    assert_eq!(world.egg(egg).unwrap().generation(), 2);
    hatch(&mut world, egg);
    let hatchling = world.sprite(egg).unwrap();
    assert_eq!(
        (hatchling.generation(), hatchling.origin()),
        (2, Origin::Hatched)
    );
}

#[test]
fn a_sprite_the_cursor_places_is_the_first_generation_and_says_so() {
    let mut world = family(&[], &[], 1);
    world.submit(Command::SpawnSprite {
        tile: at(4, 2),
        genome: None,
    });
    world.step();
    let sprite = world.sprite_at(at(4, 2)).expect("the placed sprite");
    assert_eq!((sprite.generation(), sprite.origin()), (1, Origin::Placed));
}

#[test]
fn a_sprite_that_dies_is_remembered_with_its_parents_and_how_it_died() {
    let (mut world, mother) = mother_of(1);
    let egg = laid(&mut world, 100).expect("an egg").egg;
    world.submit(Command::Rename {
        sprite: egg,
        name: "Pebble".into(),
    });
    hatch(&mut world, egg);
    let hatched = world.tick() - 1;
    assert!(
        world.dead(egg).is_none(),
        "alive, so not remembered as dead"
    );
    let tick = died(&mut world, egg, 100);
    let dead = world.dead(egg).expect("remembered");
    assert_eq!(dead.id(), egg);
    assert_eq!(dead.name(), Some("Pebble"));
    assert_eq!(dead.parents(), [mother]);
    assert_eq!((dead.generation(), dead.origin()), (2, Origin::Hatched));
    assert_eq!(dead.cause(), DeathCause::Dehydration);
    assert_eq!(dead.life_stage(), LifeStage::Young);
    assert_eq!(dead.sex(), Sex::Male);
    assert_eq!(dead.died(), tick);
    assert_eq!(dead.age(), tick - hatched);
}

#[test]
fn a_sprite_s_young_are_its_eggs_its_living_young_and_its_dead_young() {
    let (mut world, mother) = mother_of(1);
    let egg = laid(&mut world, 100).expect("an egg").egg;
    assert_eq!(world.young(mother), [egg], "an egg");
    died(&mut world, mother, 100);
    hatch(&mut world, egg);
    assert_eq!(world.young(mother), [egg], "a living young sprite");
    died(&mut world, egg, 100);
    assert_eq!(world.young(mother), [egg], "a dead one");
    assert_eq!(world.young(egg), [], "the young have none");
}

#[test]
fn the_first_sprites_and_their_young_are_remembered_alike() {
    let (mut world, mother) = mother_of(1);
    let egg = laid(&mut world, 100).expect("an egg").egg;
    died(&mut world, mother, 100);
    let dead = world.dead(mother).expect("remembered");
    assert_eq!((dead.generation(), dead.origin()), (1, Origin::First));
    assert_eq!(dead.parents(), []);
    assert_eq!(
        (dead.sex(), dead.life_stage(), dead.cause()),
        (Sex::Female, LifeStage::Adult, DeathCause::Dehydration)
    );
    hatch(&mut world, egg);
    died(&mut world, egg, 100);
    let remembered: Vec<EntityId> = world.dead_sprites().map(|dead| dead.id()).collect();
    assert_eq!(remembered, [mother, egg]);
}

#[test]
fn the_dead_are_saved_and_loaded() {
    let (mut world, _) = mother_of(1);
    let egg = laid(&mut world, 100).expect("an egg").egg;
    hatch(&mut world, egg);
    died(&mut world, egg, 100);
    let mut loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    let dead = loaded.dead(egg).expect("remembered after a load");
    assert_eq!(
        (dead.parents(), dead.cause(), dead.generation()),
        (
            world.dead(egg).unwrap().parents(),
            DeathCause::Dehydration,
            2
        )
    );
    for _ in 0..100 {
        world.step();
        loaded.step();
    }
    assert_eq!(loaded.state_hash(), world.state_hash());
}

#[test]
fn the_record_of_the_dead_is_hashed() {
    // The record is world state (M2 design §16). Two worlds alike but for
    // the name of a sprite that has died differ only in what they remember
    // of it.
    let named = |name: &str| {
        let (mut world, _) = mother_of(1);
        let egg = laid(&mut world, 100).expect("an egg").egg;
        world.submit(Command::Rename {
            sprite: egg,
            name: name.into(),
        });
        hatch(&mut world, egg);
        died(&mut world, egg, 100);
        world
    };
    let (pebble, kit) = (named("Pebble"), named("Kit"));
    assert_eq!(pebble.tick(), kit.tick());
    assert_ne!(pebble.state_hash(), kit.state_hash());
}

#[test]
fn the_dead_replay_exactly() {
    let (mut world, _) = mother_of(2);
    world.start_recording();
    let mut deaths = 0;
    for _ in 0..2_000 {
        deaths += world
            .step()
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Died { .. }))
            .count();
    }
    assert!(deaths >= 1, "a hatchling died");
    let mut playback = Playback::new(&world.replay().unwrap()).expect("the replay plays");
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
    let remembered = |world: &World| world.dead_sprites().map(|d| d.id()).collect::<Vec<_>>();
    assert_eq!(remembered(playback.world()), remembered(&world));
}

/// A save of schema 1 from before generations and eggs as items (the
/// golden save `save.rs` describes): sprites 9 and 10, each with an egg
/// laid alone, 11 and 12.
const GOLDEN_V1_EGGS: &[u8] = include_bytes!("golden/save-v1-eggs.tspr");

#[test]
fn a_save_from_before_generations_numbers_them_from_its_sprites() {
    let mut world = World::load(GOLDEN_V1_EGGS).expect("the save loads");
    let sprites: Vec<_> = world.sprites().map(|s| (s.id(), s.generation())).collect();
    assert_eq!(sprites, [(EntityId(9), 1), (EntityId(10), 1)]);
    let eggs: Vec<_> = world.eggs().map(|e| (e.id(), e.generation())).collect();
    assert_eq!(eggs, [(EntityId(11), 2), (EntityId(12), 2)]);
    assert_eq!(world.dead_sprites().count(), 0, "it remembers no dead");
    hatch(&mut world, EntityId(12));
    assert_eq!(world.sprite(EntityId(12)).unwrap().generation(), 2);
}
