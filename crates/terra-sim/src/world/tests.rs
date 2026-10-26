//! The world's unit tests: saves it refuses, the invariant checks (design
//! §7.1), and the parts of a tick that need its insides.

use super::*;
use crate::action::{Action, Outcome, ScriptedAction, Walk};
use crate::ageing::Switch;
use crate::brain::Brain;
use crate::cursor::Grip;
use crate::genome::Gene;
use crate::learning::{Signals, Touch, TraceEntry, TypeMemory};
use crate::map::Dir;
use crate::mutation::{CopyChange, CopyingError};
use crate::objects::Roll;
use crate::registry::{ChemId, Verb};
use crate::test_util::{
    WorldBuilder, at, broken_presets, builtin, builtin_changing, id_at, lone, wander_to,
};

/// A 7×5 field of grass, with a pool of shallow water at (5, 3), and a
/// berry bush at (2, 2), made from seed 7, for sprites to be added to.
fn field() -> WorldBuilder<'static> {
    let rows = &[".......", ".......", ".......", ".....~.", "......."];
    let bush = &[(Pos { x: 2, y: 2 }, "berry_bush")];
    WorldBuilder::new(rows).objects(bush).seed(7)
}

/// The field with a bush, before any step.
fn field_with_a_bush() -> World {
    field().build()
}

/// A genome of only a speed trait of `value`, read with `data`.
fn speed(value: f32, data: &DataPack) -> Genome {
    let text = format!(r#"(format: 1, genes: [Trait(trait: "speed", value: {value:?})])"#);
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A save whose checksum holds but whose world doesn't fit its pack, as
/// a build with a bug might write, is refused rather than crashing the
/// next tick (design §2.8).
#[test]
fn a_save_that_doesnt_fit_its_pack_is_refused() {
    let damage: [fn(&mut WorldState, &DataPack); 11] = [
        |state, _| the_bush(state).kind = 127,
        |state, _| {
            the_sprite(state).body.chems.pop();
        },
        // A flood whose way back from a tile it reached leads nowhere.
        |state, _| {
            let flood = the_sprite(state).flood.as_mut().expect("a flood");
            flood.damage_a_step(8);
        },
        |state, _| {
            let flood = the_sprite(state).flood.as_mut().expect("a flood");
            flood.damage_a_step(u8::MAX);
        },
        // A concept of an input the pack doesn't have.
        |state, _| the_sprite(state).brain.concepts[0] = vec![(999, false)],
        // Something learned about an object type the pack doesn't have.
        |state, data| {
            let experience = &mut the_sprite(state).brain.experience;
            let nothing = TypeMemory::new(data.needs().count());
            experience.types.insert(Subject::ObjectType(9999), nothing);
        },
        // A motive for a need the pack doesn't have.
        |state, _| {
            let trace = &mut the_sprite(state).brain.trace;
            trace.back_mut().expect("a trace entry").motive = Some(999);
        },
        // A gene copying changed that was of a chemical the pack
        // doesn't have, which the Genome tab would name.
        |state, _| {
            let sprite = the_sprite(state);
            let (gene, ticks) = sprite
                .genome
                .genes
                .iter()
                .enumerate()
                .find_map(|(index, gene)| match *gene {
                    Gene::HalfLife { ticks, .. } => Some((index, ticks)),
                    _ => None,
                })
                .expect("a half-life gene");
            let was = Gene::HalfLife {
                chem: ChemId(999),
                ticks: ticks * 2,
            };
            sprite.copying_errors = vec![CopyingError { gene, was }];
        },
        // A gene lost in copying that was of a chemical the pack doesn't
        // have.
        |state, _| {
            let was = Gene::HalfLife {
                chem: ChemId(999),
                ticks: 100,
            };
            the_sprite(state).copy_changes = vec![CopyChange::Lost {
                was,
                switch: Switch::default(),
            }];
        },
        // A sprite born, or a trace entry made, after the save.
        |state, _| {
            let tick = state.tick;
            the_sprite(state).born = tick as i64 + 1;
        },
        |state, _| {
            let tick = state.tick;
            let trace = &mut the_sprite(state).brain.trace;
            trace.back_mut().expect("a trace entry").tick = tick + 1;
        },
    ];
    for (n, damage) in damage.into_iter().enumerate() {
        let mut world = field_with_a_bush();
        world.submit(Command::SpawnSprite {
            tile: Pos { x: 1, y: 1 },
            genome: None,
        });
        for _ in 0..10 {
            world.step();
        }
        let sprite = the_sprite(&mut world.state);
        sprite.brain.trace.push_back(TraceEntry {
            tick: 10,
            verb: None,
            subject: None,
            motive: Some(0),
        });
        assert!(World::load(&world.save()).is_ok(), "damage {n}");
        damage(&mut world.state, &world.data);
        match World::load(&world.save()) {
            Err(LoadError::Damaged(_)) => {}
            Err(err) => panic!("damage {n} refused for the wrong reason: {err}"),
            Ok(_) => panic!("damage {n} loaded"),
        }
    }
}

/// A save whose record of the dead doesn't hold together (M2 design
/// §17.1) is refused, as a build with a bug might write it.
#[test]
fn a_save_whose_record_of_the_dead_is_damaged_is_refused() {
    let damage: [fn(&mut WorldState); 4] = [
        // A sprite remembered as dead that's still in the world.
        |state| {
            let (id, sprite) = state.sprites.iter().next().expect("a sprite");
            let record = Record::of(sprite, DeathCause::OldAge, state.tick);
            state.dead.remember(id, record);
        },
        // One whose parent is younger than itself.
        |state| {
            let (_, sprite) = state.sprites.iter().next().expect("a sprite");
            let mut record = Record::of(sprite, DeathCause::OldAge, state.tick);
            record.parents = vec![EntityId(state.next_id)];
            record.generation = 2;
            state.dead.remember(EntityId(state.next_id - 1), record);
        },
        // One in its parent's generation.
        |state| {
            let (_, sprite) = state.sprites.iter().next().expect("a sprite");
            let record = Record::of(sprite, DeathCause::OldAge, state.tick);
            state.dead.remember(EntityId(0), record.clone());
            let mut young = record;
            young.parents = vec![EntityId(0)];
            state.dead.remember(EntityId(state.next_id - 1), young);
        },
        // One in the generation of its parent, who lives.
        |state| {
            let (parent, sprite) = state.sprites.iter().next().expect("a sprite");
            let mut record = Record::of(sprite, DeathCause::OldAge, state.tick);
            record.parents = vec![parent];
            state.dead.remember(EntityId(state.next_id - 1), record);
        },
    ];
    for (n, damage) in damage.into_iter().enumerate() {
        let mut world = field_with_a_bush();
        world.submit(Command::SpawnSprite {
            tile: Pos { x: 1, y: 1 },
            genome: None,
        });
        world.submit(Command::SpawnSprite {
            tile: Pos { x: 4, y: 1 },
            genome: None,
        });
        world.step();
        let gone = world.state.sprites.iter().last().expect("a sprite").0;
        world.state.sprites.remove(gone);
        assert!(World::load(&world.save()).is_ok(), "damage {n}");
        damage(&mut world.state);
        match World::load(&world.save()) {
            Err(LoadError::Damaged(_)) => {}
            Err(err) => panic!("damage {n} refused for the wrong reason: {err}"),
            Ok(_) => panic!("damage {n} loaded"),
        }
    }
}

/// A save whose preset breaks a rule a preset file must keep, as a
/// damaged file might hold it, is refused (design §2.8): a world
/// recorded from its first tick is generated again from its preset
/// (design §2.7).
#[test]
fn a_save_whose_preset_breaks_a_rule_is_refused() {
    let data = builtin();
    let config = WorldConfig::builtin(&data);
    let world = World::new(config.clone(), data, 7);
    assert!(World::load(&world.save()).is_ok());
    for (n, preset) in broken_presets(&config).into_iter().enumerate() {
        let bytes = save::write(&save::Contents {
            seed: world.seed,
            config: Some(preset),
            pack: world.data.sources().to_vec(),
            state: &world.state,
            view: None,
        });
        match World::load(&bytes) {
            Err(LoadError::Damaged(why)) => assert!(why.contains("preset"), "{n}: {why}"),
            Err(err) => panic!("preset {n} refused for the wrong reason: {err}"),
            Ok(_) => panic!("preset {n} loaded"),
        }
    }
}

/// Two sprites, a step on, with the record of the dead checked as it
/// stands.
fn two_sprites_a_step_on() -> World {
    let mut world = field_with_a_bush();
    for x in [1, 4] {
        world.submit(Command::SpawnSprite {
            tile: Pos { x, y: 1 },
            genome: None,
        });
    }
    world.step();
    assert_eq!(world.check_invariants(), Ok(()));
    world
}

/// A sprite or an object the world remembers as dead is caught on any
/// tick, not only one when a sprite died (design §7.1).
#[test]
fn something_in_the_world_remembered_as_dead_is_caught_on_any_tick() {
    let mut world = two_sprites_a_step_on();
    let (id, sprite) = world.state.sprites.iter().next().expect("a sprite");
    let record = Record::of(sprite, DeathCause::OldAge, world.state.tick);
    world.state.dead.remember(id, record.clone());
    assert!(
        world
            .check_invariants()
            .is_err_and(|broken| broken.0.contains("remembered as dead"))
    );
    world.state.dead = RecordOfTheDead::default();
    let bush = world.object_at(Pos { x: 2, y: 2 }).expect("the bush").id();
    world.state.dead.remember(bush, record);
    assert!(
        world
            .check_invariants()
            .is_err_and(|broken| broken.0.contains("remembered as dead"))
    );
}

/// A record of the dead found broken is found broken again, however
/// often it's checked (design §7.1).
#[test]
fn a_broken_record_of_the_dead_stays_caught() {
    let mut world = two_sprites_a_step_on();
    let (_, sprite) = world.state.sprites.iter().next().expect("a sprite");
    let mut record = Record::of(sprite, DeathCause::OldAge, world.state.tick);
    record.parents = vec![EntityId(world.state.next_id)];
    record.generation = 2;
    let gone = EntityId(world.state.next_id - 1);
    world.state.sprites.remove(gone);
    world.state.dead.remember(gone, record);
    *world.state.deaths.entry(DeathCause::OldAge).or_insert(0) += 1;
    for check in 1..=2 {
        assert!(
            world
                .check_invariants()
                .is_err_and(|broken| broken.0.contains("younger than itself")),
            "check {check}"
        );
    }
}

/// The world's only sprite.
fn the_sprite(state: &mut WorldState) -> &mut Sprite {
    let (id, _) = state.sprites.iter().next().expect("a sprite");
    state.sprites.get_mut(id).expect("the sprite")
}

/// Sprite `id` of `world`, to change.
fn sprite_mut(world: &mut World, id: EntityId) -> &mut Sprite {
    world.state.sprites.get_mut(id).expect("a sprite")
}

/// The berry bush on (2, 2) of the field with a bush, to change.
fn the_bush(state: &mut WorldState) -> &mut Object {
    let id = state.objects.at(Pos { x: 2, y: 2 }).expect("the bush");
    state.objects.get_mut(id).expect("the bush")
}

/// Runs step 4 (design §2.4) for every sprite of `world` but those
/// `dying`, with none of them in company.
fn step_4(world: &mut World, dying: &[EntityId]) {
    let mut events = Vec::new();
    learning::run(
        &mut world.state,
        &world.data,
        dying,
        &BTreeMap::new(),
        &mut events,
    );
}

/// Puts an object of type `name` on `pos` without checking the placement rules.
fn force_place(world: &mut World, name: &str, pos: Pos) {
    let kind = world.data.object_type_named(name).expect("a built-in type");
    let object = new_object(&world.data, kind, pos);
    world.state.add_object(object, &world.data);
}

/// Puts a berry on `pos`, and gives its ID.
fn force_berry(world: &mut World, pos: Pos) -> EntityId {
    force_place(world, "berry", pos);
    world.state.objects.at(pos).expect("the berry")
}

#[test]
#[should_panic(expected = "at the end of tick 0: EntityId(1) is at or above the ID counter")]
fn in_debug_builds_a_broken_invariant_fails_on_the_tick_it_is_found() {
    let mut world = field_with_a_bush();
    world.state.next_id = 1;
    world.step();
}

/// Sets the object on `pos` rolling east with `left` tiles to go.
fn set_rolling(world: &mut World, pos: Pos, left: u16) {
    let id = world.state.objects.at(pos).expect("an object there");
    let roll = Roll { dir: Dir::E, left };
    world.state.objects.get_mut(id).expect("the object").roll = Some(roll);
}

#[test]
fn a_rolling_item_with_tiles_to_go_passes_the_invariant_checks() {
    let mut world = field_with_a_bush();
    force_place(&mut world, "ball", Pos { x: 4, y: 1 });
    set_rolling(&mut world, Pos { x: 4, y: 1 }, 3);
    assert_eq!(world.check_invariants(), Ok(()));
}

#[test]
fn a_rolling_solid_object_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    set_rolling(&mut world, Pos { x: 2, y: 2 }, 3);
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_roll_with_no_tiles_to_go_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    force_place(&mut world, "ball", Pos { x: 4, y: 1 });
    set_rolling(&mut world, Pos { x: 4, y: 1 }, 0);
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_valid_world_passes_its_invariant_checks() {
    assert_eq!(field_with_a_bush().check_invariants(), Ok(()));
}

#[test]
fn a_solid_object_on_terrain_that_does_not_allow_fixtures_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    force_place(&mut world, "thornbush", Pos { x: 3, y: 3 });
    assert_eq!(world.check_invariants(), Ok(()), "side by side is fine");
    force_place(&mut world, "thornbush", Pos { x: 5, y: 3 });
    assert!(world.check_invariants().is_err(), "in the pool");
}

#[test]
fn an_id_the_counter_has_not_reached_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    world.state.next_id = 1;
    assert!(world.check_invariants().is_err());
}

#[test]
fn an_id_counter_that_goes_back_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    let berry = force_berry(&mut world, Pos { x: 5, y: 1 });
    world.state.objects.remove(berry);
    assert_eq!(world.check_invariants(), Ok(()));
    // Every object left is below the counter, but the berry's ID could now be reused.
    world.state.next_id -= 1;
    assert!(world.check_invariants().is_err());
    assert!(world.check_invariants().is_err(), "and it stays caught");
}

/// The field with a bush, plus starter sprites at (5, 1) and (5, 2).
fn field_with_sprites() -> (World, EntityId, EntityId) {
    let world = field().starters(&[at(5, 1), at(5, 2)]).build();
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    (world, ids[0], ids[1])
}

#[test]
fn valid_sprites_pass_the_invariant_checks() {
    assert_eq!(field_with_sprites().0.check_invariants(), Ok(()));
}

#[test]
fn a_sprite_the_tile_index_has_elsewhere_breaks_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    sprite_mut(&mut world, first).pos = Pos { x: 0, y: 0 };
    assert!(world.check_invariants().is_err());
}

#[test]
fn two_sprites_on_one_tile_break_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    sprite_mut(&mut world, first).pos = Pos { x: 5, y: 2 };
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_sprite_on_a_solid_object_breaks_an_invariant() {
    let (mut world, _, _) = field_with_sprites();
    force_place(&mut world, "thornbush", Pos { x: 5, y: 1 });
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_cursor_leading_and_holding_as_commands_leave_it_passes_the_invariant_checks() {
    let (mut world, first, _) = field_with_sprites();
    world.submit(Command::TakeHold { sprite: first });
    world.step();
    assert_eq!(world.check_invariants(), Ok(()));
    let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
    world.submit(Command::LetGo);
    world.submit(Command::PickUp { item: berry });
    world.step();
    assert_eq!(world.check_invariants(), Ok(()));
}

#[test]
fn a_cursor_leading_a_sprite_that_is_not_there_breaks_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    world.submit(Command::TakeHold { sprite: first });
    world.step();
    world.state.sprites.remove(first);
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_sprite_s_age_sensor_reaches_1_as_broken_genes_bring_its_old_age_sooner() {
    // M2 design §9.3, §12.4: the sensor and the life stage agree on
    // when it grows old.
    let (mut world, first, _) = field_with_sprites();
    let sprite = sprite_mut(&mut world, first);
    sprite.flaws = vec![2; sprite.genome.genes.len()];
    world.step();
    let view = world.sprite(first).expect("a sprite");
    assert!(view.grows_old_at() < view.traits().grows_old * 0.7);
    let sprite = world.state.sprites.get(first).expect("a sprite");
    let age = sprite.body.loci[world.data.physiology().indices.age];
    let ticks = age * view.grows_old_at();
    let expected = sprite.age(world.state.tick) as f32;
    assert!(
        (ticks - expected).abs() <= 2.0,
        "{ticks} ticks, not {expected}"
    );
}

#[test]
fn an_id_both_a_sprite_and_an_object_breaks_an_invariant() {
    // M2 design §10.4: a hatchling takes its egg's ID, and the egg is
    // gone.
    let (mut world, first, _) = field_with_sprites();
    let berry = world.data.real_object_type("berry").expect("berries");
    let object = new_object(&world.data, berry, Pos { x: 0, y: 0 });
    let tick = world.state.tick;
    world.state.objects.place(first, object, &world.data, tick);
    let broken = world.check_invariants().expect_err("one ID, two things");
    assert!(broken.0.contains("both"), "{broken:?}");
}

#[test]
fn a_copying_error_in_a_gene_the_genome_lacks_breaks_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    let sprite = sprite_mut(&mut world, first);
    let gene = sprite.genome.genes.len() - 1;
    let was = sprite.genome.genes[gene].clone();
    sprite.copying_errors = vec![CopyingError { gene, was }];
    assert_eq!(world.check_invariants(), Ok(()), "its last gene");
    let sprite = sprite_mut(&mut world, first);
    sprite.copying_errors[0].gene += 1;
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_copying_error_that_changed_what_a_gene_does_breaks_an_invariant() {
    // M2 design §12.1: copying changes only a gene's values.
    let (mut world, first, _) = field_with_sprites();
    let sprite = sprite_mut(&mut world, first);
    let gene = sprite.genome.genes.len() - 1;
    let was = sprite.genome.genes[gene].clone();
    let other = sprite
        .genome
        .genes
        .iter()
        .find(|other| std::mem::discriminant(*other) != std::mem::discriminant(&was))
        .expect("a gene of another kind")
        .clone();
    sprite.copying_errors = vec![CopyingError { gene, was: other }];
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_copy_change_that_doesnt_fit_the_genome_breaks_an_invariant() {
    // M2 design §13: a doubled gene does what the gene before it does;
    // a changed one differs from what it was.
    let (mut world, first, _) = field_with_sprites();
    let sprite = sprite_mut(&mut world, first);
    let last = sprite.genome.genes.len() - 1;
    let gene = sprite.genome.genes[last].clone();
    let switch = sprite.genome.switch(last);
    let other = sprite
        .genome
        .genes
        .iter()
        .find(|other| std::mem::discriminant(*other) != std::mem::discriminant(&gene))
        .expect("a gene of another kind")
        .clone();
    let fits = [CopyChange::Changed {
        gene: last,
        was: other,
        switch,
        on_copy: false,
    }];
    let not = [
        CopyChange::Doubled { gene: 0 },
        CopyChange::Doubled { gene: last + 1 },
        CopyChange::Doubled { gene: 1 },
        CopyChange::Changed {
            gene: last,
            was: gene.clone(),
            switch,
            on_copy: false,
        },
        CopyChange::Changed {
            gene: last,
            was: gene,
            switch: Switch {
                from: LifeStage::Old,
                until: None,
            },
            on_copy: true,
        },
    ];
    sprite.copy_changes = fits.to_vec();
    assert_eq!(world.check_invariants(), Ok(()));
    for change in not {
        let sprite = sprite_mut(&mut world, first);
        sprite.copy_changes = vec![change.clone()];
        assert!(world.check_invariants().is_err(), "{change:?}");
    }
}

#[test]
fn more_than_two_flawed_copies_of_a_gene_or_flaws_of_genes_it_lacks_break_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    let sprite = sprite_mut(&mut world, first);
    sprite.flaws = vec![1, 2];
    assert_eq!(world.check_invariants(), Ok(()), "two at most");
    let sprite = sprite_mut(&mut world, first);
    sprite.flaws = vec![3];
    assert!(world.check_invariants().is_err());
    let sprite = sprite_mut(&mut world, first);
    sprite.flaws = vec![0; sprite.genome.genes.len() + 1];
    assert!(world.check_invariants().is_err(), "a gene too many");
}

#[test]
fn a_sprite_led_by_no_cursor_breaks_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    sprite_mut(&mut world, first).lead = Some(Walk::default());
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_held_item_the_cursor_does_not_hold_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
    world.state.objects.pick_up(berry);
    assert!(world.check_invariants().is_err());
}

#[test]
fn an_object_not_scheduled_for_its_next_turn_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    world.step();
    the_bush(&mut world.state).stage_ends = 5;
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_held_item_in_a_stage_its_type_lacks_breaks_an_invariant() {
    // Gemini's review of PR #89: a held item's life goes on, so its
    // stage and counters are checked as on the map (design §3.5.2).
    let mut world = field_with_a_bush();
    let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
    world.state.objects.pick_up(berry);
    world.state.cursor.take(Grip::Holds(berry));
    assert_eq!(world.check_invariants(), Ok(()));
    world.state.objects.get_mut(berry).expect("the berry").stage = Some(9);
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_cursor_holding_an_item_on_the_map_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    let berry = force_berry(&mut world, Pos { x: 0, y: 0 });
    world.state.cursor.grip = Some(Grip::Holds(berry));
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_level_outside_0_to_1_breaks_an_invariant() {
    let (mut world, _, second) = field_with_sprites();
    sprite_mut(&mut world, second).body.chems[0] = 1.5;
    assert!(world.check_invariants().is_err());
}

#[test]
fn levels_a_tick_ago_that_do_not_match_the_chemicals_break_an_invariant() {
    // As a body loaded from a save would have, if loading didn't fill them in.
    let (mut world, first, _) = field_with_sprites();
    let body = &mut sprite_mut(&mut world, first).body;
    body.chems_before_tick.clear();
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_locus_that_is_not_a_number_breaks_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    sprite_mut(&mut world, first).body.loci[1] = f32::NAN;
    assert!(world.check_invariants().is_err());
}

#[test]
fn a_sprite_id_the_counter_has_not_reached_breaks_an_invariant() {
    let (mut world, _, second) = field_with_sprites();
    world.state.next_id = second.0;
    world.checked_next_id.set(second.0);
    assert!(world.check_invariants().is_err());
}

#[test]
fn the_state_hash_covers_every_sprite_s_chemistry() {
    let (mut world, _, second) = field_with_sprites();
    let before = world.state_hash();
    sprite_mut(&mut world, second).body.chems[5] = 0.5;
    assert_ne!(world.state_hash(), before);
}

#[test]
fn nearby_sprites_counts_the_others_within_3_tiles_over_4_capped_at_1() {
    let tiles = [
        at(3, 3),
        at(0, 0),
        at(6, 6),
        at(3, 4),
        at(4, 4),
        at(2, 2),
        at(7, 3),
        at(8, 8),
    ];
    let mut world = WorldBuilder::new(&["........."; 9])
        .starters(&tiles)
        .build();
    world.step();
    let index = world.data.physiology().indices.nearby_sprites;
    let reading = |x, y| {
        let pos = Pos { x, y };
        let id = world.state.sprites.at(pos).expect("a sprite");
        let (_, sprite) = world
            .state
            .sprites
            .iter()
            .find(|&(i, _)| i == id)
            .expect("a sprite");
        sprite.body.loci[index]
    };
    assert_eq!(reading(3, 3), 1.0, "5 others within 3 tiles, capped");
    assert_eq!(
        reading(7, 3),
        0.5,
        "(4, 4) and (6, 6); (3, 3) is 4 tiles away"
    );
    assert_eq!(reading(8, 8), 0.25, "(6, 6) only");
}

/// A row of grass `length` tiles long with one walker of speed 10, a
/// step a tick, at its west end, starting on `scripted`.
fn row_with_a_walker(length: usize, scripted: &[ScriptedAction]) -> (World, EntityId) {
    let row = ".".repeat(length);
    let start = at(0, 0);
    let world = WorldBuilder::new(&[row.as_str()])
        .sprites_made(&[(start, |data| speed(10.0, data))])
        .script(start, scripted)
        .build();
    let id = world.sprites().next().expect("the walker").id();
    (world, id)
}

#[test]
fn a_sprite_that_stays_put_makes_its_flood_again_every_8_ticks() {
    let (mut world, id) = row_with_a_walker(3, &[ScriptedAction::Rest; 3]);
    for tick in 0..24 {
        world.step();
        let flood = world.state.sprites.get(id).expect("alive").flood.as_ref();
        assert_eq!(flood.expect("a flood").made, tick / 8 * 8, "tick {tick}");
    }
}

#[test]
fn a_wander_ends_as_failed_once_its_destination_can_no_longer_be_reached() {
    let destination = Pos { x: 5, y: 0 };
    let (mut world, id) = row_with_a_walker(6, &[ScriptedAction::Wander { destination }]);
    world.step();
    force_place(&mut world, "thornbush", destination);
    let ended: Vec<(u64, EventKind)> = world
        .step()
        .into_iter()
        .filter(|e| matches!(e.kind, EventKind::ActionEnded { .. }))
        .map(|e| (e.tick, e.kind))
        .collect();
    let failed: Vec<(u64, Verb, Outcome)> = ended
        .into_iter()
        .map(|(tick, kind)| match kind {
            EventKind::ActionEnded {
                id: who,
                verb,
                outcome,
                ..
            } if who == id => (tick, verb, outcome),
            _ => unreachable!("only endings"),
        })
        .collect();
    assert_eq!(failed, [(1, Verb::Wander, Outcome::Failed)]);
}

#[test]
fn two_sprites_never_swap_diagonally_past_a_bush_that_grew_since_their_floods() {
    // Speed 4: 40 tenths a tick against a diagonal's 140, so neither
    // steps for the first ticks, and both floods still show the diagonal
    // when the bush grows.
    let (a, b) = (at(0, 0), at(1, 1));
    let mut world = WorldBuilder::new(&["..", ".."])
        .sprites_made(&[(a, |data| speed(4.0, data)), (b, |data| speed(4.0, data))])
        .scripts(&[(a, wander_to(b)), (b, wander_to(a))])
        .build();
    world.step();
    force_place(&mut world, "thornbush", Pos { x: 1, y: 0 });
    let waiting = |world: &World| -> Vec<(u32, u32)> {
        world
            .state
            .sprites
            .iter()
            .map(|(_, s)| {
                (
                    s.action.as_ref().expect("wandering").walk.blocked_ticks,
                    s.move_points,
                )
            })
            .collect()
    };
    world.step();
    assert_eq!(waiting(&world), [(0, 80); 2], "80 tenths: not yet blocked");
    world.step();
    world.step();
    assert_eq!(
        waiting(&world),
        [(1, 140); 2],
        "blocked, banked up to the step"
    );
    assert!(
        world.sprite_at(a).is_some() && world.sprite_at(b).is_some(),
        "no swap"
    );
}

#[test]
fn a_dying_sprite_takes_no_part_in_a_swap() {
    // Head-on in a corridor. The east sprite (speed 10) banks a step's
    // points on the first tick while the west one (speed 5) can't yet
    // pay for a swap; then the east one starves to death.
    let (west, east) = (at(1, 0), at(2, 0));
    let mut world = WorldBuilder::new(&["...."])
        .sprites_made(&[
            (west, |data| speed(5.0, data)),
            (east, |data| speed(10.0, data)),
        ])
        .scripts(&[(west, wander_to(at(3, 0))), (east, wander_to(at(0, 0)))])
        .build();
    let ids: Vec<EntityId> = world.sprites().map(|s| s.id()).collect();
    world.step();
    let indices = world.data.physiology().indices;
    let dying = sprite_mut(&mut world, ids[1]);
    dying.body.chems[indices.energy] = 0.0;
    dying.body.chems[indices.injury] = 1.0;
    let events = world.step();
    let about_dying: Vec<&EventKind> = events
        .iter()
        .map(|e| &e.kind)
        .filter(|k| matches!(k, EventKind::ActionEnded { id, .. } if *id == ids[1]))
        .collect();
    assert!(about_dying.is_empty(), "{about_dying:?}");
    assert!(world.sprite(ids[1]).is_none(), "it died");
    assert_eq!(world.sprite(ids[0]).expect("alive").pos(), west, "no swap");
}

#[test]
fn a_blocked_sprite_finds_its_way_round_to_another_of_its_target_s_goal_tiles() {
    // Design §3.7: blocked re-planning looks for a path to the action's
    // goal tiles. A sprite rests on the one in front of the bush, at the
    // end of the corridor; the only other goal tiles are round the long
    // loop, dearer than the corridor even with its occupied-tile
    // penalty, so the walker heads down the corridor and is blocked.
    let rows = [".........", ".#######.", ".........", "#########"];
    let (walker, rester, bush) = (at(0, 2), at(6, 2), at(7, 2));
    let mut world = WorldBuilder::new(&rows)
        .objects(&[(bush, "berry_bush")])
        .sprites_made(&[
            (walker, |data| speed(10.0, data)),
            (rester, |data| speed(10.0, data)),
        ])
        .script(rester, &[ScriptedAction::Rest; 5])
        .build();
    let id = world.sprite_at(walker).expect("the walker").id();
    let target = Target::Object(world.state.objects.at(bush).expect("the bush"));
    sprite_mut(&mut world, id).action =
        Some(Action::new(Verb::Eat, None, Some((target, None)), true, 0));
    let mut ended = None;
    for _ in 0..40 {
        for event in world.step() {
            if let EventKind::ActionEnded {
                id: who, outcome, ..
            } = event.kind
                && who == id
            {
                ended.get_or_insert(outcome);
            }
        }
        if ended.is_some() {
            break;
        }
    }
    let pos = world.sprite(id).expect("alive").pos();
    assert_ne!(ended, Some(Outcome::Blocked), "at {pos:?}");
    assert_eq!(pos, Pos { x: 8, y: 1 }, "round the loop, beside the bush");
}

#[test]
fn a_change_of_mind_aims_at_nothing_out_of_reach() {
    // Design §5.5, #173: a verb switched to mid-action is aimed at the
    // action's target, so while that's out of reach, as a retreat's can
    // be, only what needs no target is offered. A hungry sprite backing
    // away from a bush beyond its reach doesn't change its mind to eat
    // that bush, nor the one in reach beside it.
    let data = builtin();
    let starter = include_str!("../../../../data/genomes/starter.ron");
    let hungry = r#"        InitialConcentration(chem: "hunger", value: 0.9),"#;
    let text = starter.replace("    ],\n)", &format!("{hungry}\n    ],\n)"));
    let genome = Genome::from_ron(&text, &data).expect("a valid genome");
    let (far, near) = (at(28, 1), at(3, 0));
    let mut world = WorldBuilder::new(&[".............................."; 3])
        .data(data)
        .objects(&[(far, "berry_bush"), (near, "berry_bush")])
        .sprites_with(vec![(at(1, 1), Some(genome))])
        .build();
    let id = world.sprites().next().expect("the sprite").id();
    let target = Target::Object(world.state.objects.at(far).expect("the far bush"));
    let kind = world.state.type_of(&world.data, target);
    let backing_away = Action::new(Verb::Retreat, None, Some((target, kind)), false, 0);
    sprite_mut(&mut world, id).action = Some(backing_away);
    world.step();
    let action = world.sprite(id).expect("the sprite").action();
    let action = action.expect("an action");
    assert_eq!((action.verb, action.target), (Verb::Wander, None));
}

#[test]
fn a_sprite_makes_its_flood_again_on_each_new_tile() {
    // Design §3.6: on a new tile, or 8 ticks after the last flood.
    let (mut world, id) = row_with_a_walker(
        6,
        &[ScriptedAction::Wander {
            destination: Pos { x: 5, y: 0 },
        }],
    );
    world.step();
    for tick in 1..4 {
        world.step();
        let sprite = world.state.sprites.get(id).expect("alive");
        assert_eq!(sprite.pos.x, tick as u16 + 1);
        let flood = sprite.flood.as_ref().expect("a flood");
        assert_eq!(
            (flood.made, flood.origin.x),
            (tick, tick as u16),
            "tick {tick}"
        );
    }
}

#[test]
fn with_learning_off_familiarity_still_grows() {
    // Design §5.6: a control run learns no worth or habit, but what a
    // sprite attends to still grows familiar.
    let data = builtin();
    let genes = r#"(format: 1, genes: [
        AttentionInstinct(input: "always", category: "toy", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    ])"#;
    let genome = Genome::from_ron(genes, &data).expect("a valid genome");
    let mut world = WorldBuilder::new(&["....."])
        .data(data)
        .objects(&[(at(3, 0), "ball")])
        .sprites_with(vec![(at(0, 0), Some(genome))])
        .build();
    world.state.learning = false;
    let id = world.sprites().next().expect("the sprite").id();
    for _ in 0..50 {
        world.step();
    }
    let brain = &world.state.sprites.get(id).expect("alive").brain;
    let ball = brain
        .experience
        .types
        .get(&Subject::ObjectType(4))
        .expect("it attended to the ball");
    assert!(ball.familiarity > 0.0);
    assert!(ball.worth.iter().all(|&w| w == 0.0) && ball.habits.iter().all(|&h| h == 0.0));
    assert!(ball.taught_habits.iter().all(|&h| h == 0.0));
}

#[test]
fn with_learning_off_sprites_grow_no_more_familiar_with_each_other() {
    // M2 design §15.2: unlike familiarity with object types (design
    // §5.6), getting to know a sprite is learning, so a control run
    // knows no sprite by sight or by playing with it. With learning on,
    // the same two know each other.
    let knowing = |learning: bool| {
        let data = builtin();
        let genes = |genes: &str| {
            let text = format!(
                r#"(format: 1, genes: [{genes}
                BrainParam(param: "tau_base", value: 0.02),
            ])"#
            );
            Genome::from_ron(&text, &data).expect("a valid genome")
        };
        let player = genes(
            r#"AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
            Instinct(inputs: [("attended_sprite", false)], verb: Play, weight: 1.0),"#,
        );
        let rester = genes(r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#);
        let mut world = WorldBuilder::new(&["....."])
            .data(data)
            .sprites_with(vec![(at(0, 0), Some(player)), (at(1, 0), Some(rester))])
            .build();
        world.state.learning = learning;
        let [a, b] = [0, 1].map(|x| world.sprite_at(Pos { x, y: 0 }).expect("a sprite").id());
        let mut played = false;
        for _ in 0..50 {
            played |= world.step().iter().any(|event| {
                matches!(
                    event.kind,
                    EventKind::ActionEnded {
                        verb: Verb::Play,
                        outcome: Outcome::Applied,
                        ..
                    }
                )
            });
        }
        assert!(played, "they played");
        let knows = |id: EntityId, other| world.sprite(id).expect("alive").familiarity(other);
        [knows(a, b), knows(b, a)]
    };
    assert_eq!(knowing(false), [0.0, 0.0]);
    assert!(knowing(true).iter().all(|&known| known > 0.1));
}

#[test]
fn an_old_save_s_egg_with_no_egg_type_or_nowhere_to_lie_hatches_as_it_loads() {
    // M2 design §10.4: an egg from before eggs were items, a sprite that
    // hatches on tick 750, turned into an egg item as it loads; with no
    // type to become, or with an item on the only tile there is, it
    // hatches at once, keeping its ID.
    let hatches = |data: DataPack, objects: &[(Pos, &str)]| {
        let tile = at(0, 0);
        let mut world = WorldBuilder::new(&["."])
            .data(data)
            .objects(objects)
            .starters(&[tile])
            .newborns(&[tile])
            .build();
        let id = world.sprites().next().expect("the egg").id();
        let egg = sprite_mut(&mut world, id);
        egg.hatches = Some(750);
        world.state.old_eggs_to_items(&world.data);
        let as_egg = world.egg(id).map(|egg| egg.hatches());
        (world.sprite(id).map(|sprite| sprite.age()), as_egg)
    };
    let data = builtin();
    assert_eq!(hatches(data.clone(), &[]), (None, Some(750)), "an egg");
    let berry = [(Pos { x: 0, y: 0 }, "berry")];
    assert_eq!(hatches(data, &berry), (Some(0), None), "nowhere to lie");
    let changes = [("next: Hatch", "next: Expire")];
    let no_egg = builtin_changing("objects.ron", &changes);
    assert_eq!(hatches(no_egg, &[]), (Some(0), None), "no egg type");
}

/// The built-in pack's water, as sprites learn about it (`objects.ron`).
const WATER: Subject = Subject::ObjectType(100);

/// Rests, and does nothing else.
const RESTS: &str = r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#;

/// Thirsty from the start, and drinks.
const DRINKS: &str = r#"InitialConcentration(chem: "thirst", value: 1.0),
    Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),"#;

/// Rests, its eye drawn to a parent showing it something (M2 design
/// §18.1) and to nothing by chance, with `genes` as well.
fn looks_to_a_parent_showing(genes: &str) -> String {
    format!(
        r#"{RESTS}
        AttentionInstinct(input: "always", category: "sprite", whom: Showing, weight: 1.0),
        BrainParam(param: "curiosity", value: 0.0),
        BrainParam(param: "tau_att_base", value: 0.05),
        {genes}"#
    )
}

/// A genome of `genes`, read with `data`.
fn genome_of(genes: &str, data: &DataPack) -> Genome {
    let text = format!("(format: 2, genes: [{genes}])");
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// Teaches the sprite `id` that water is good for thirst, as if it had
/// drunk.
fn know_water(world: &mut World, id: EntityId) {
    let needs = world.data.need_places().len();
    let thirst = world.data.needs().position(|need| need == "thirst");
    let thirst = thirst.expect("thirst is a need");
    let brain = &mut sprite_mut(world, id).brain;
    let known = brain.experience.learn_about(WATER, needs);
    known.worth[thirst] = 0.6;
    known.touched = true;
}

#[test]
fn a_parent_drinking_draws_its_young_s_eye_until_the_young_knows_water() {
    // M2 design §18.1: an attention instinct about a parent showing it
    // something draws its eye to its mother drinking for her thirst,
    // knowing water is good for thirst, before its nearer father, while
    // it doesn't know that; once it does, the nearer parent draws it.
    let data = builtin();
    let looks = looks_to_a_parent_showing(r#"BrainParam(param: "attention_margin", value: 0.0),"#);
    let (mother, father, young) = (at(1, 0), at(5, 1), at(4, 1));
    let mut world = WorldBuilder::new(&["~.....", "~....."])
        .sprites_with(vec![
            (mother, Some(genome_of(DRINKS, &data))),
            (father, Some(genome_of(RESTS, &data))),
            (young, Some(genome_of(&looks, &data))),
        ])
        .newborns(&[young])
        .data(data)
        .build();
    let [mother_id, father_id, young_id] = [mother, father, young].map(|pos| id_at(&world, pos));
    know_water(&mut world, mother_id);
    let parents = vec![mother_id, father_id];
    sprite_mut(&mut world, young_id).parents = parents;
    let looks_at = |world: &World| world.sprite(young_id).expect("young").attending_to();
    world.step();
    assert_eq!(looks_at(&world), Some(mother), "its mother is drinking");
    know_water(&mut world, young_id);
    world.step();
    assert_eq!(looks_at(&world), Some(father), "it knows water");
}

#[test]
fn a_parent_whose_target_is_gone_shows_its_young_nothing() {
    // M2 design §18.1: the young, first in turn, weighs its mother while
    // her Eat, which ate the berry, is still her latest action, aimed at
    // a berry that's gone. She knows berries, but one that's gone shows
    // it nothing, and isn't looked up.
    let data = builtin();
    let looks = format!(
        r#"{RESTS}
        AttentionInstinct(input: "always", category: "sprite", whom: Showing, weight: 1.0)"#
    );
    let (young, mother, berry) = (at(3, 1), at(1, 0), at(0, 0));
    let mut world = WorldBuilder::new(&["....", "...."])
        .objects(&[(berry, "berry")])
        .sprites_with(vec![
            (young, Some(genome_of(&looks, &data))),
            (mother, Some(genome_of(RESTS, &data))),
        ])
        .scripts(&[(mother, ScriptedAction::Eat { at: berry })])
        .newborns(&[young])
        .data(data)
        .build();
    let (young_id, mother_id) = (id_at(&world, young), id_at(&world, mother));
    let needs = world.data.need_places().len();
    let berry = world.data.real_object_type("berry").expect("berries");
    let berry_type = world.data.object_types()[berry].id;
    let brain = &mut sprite_mut(&mut world, mother_id).brain;
    let known = brain
        .experience
        .learn_about(Subject::ObjectType(berry_type), needs);
    known.worth[0] = 0.6;
    known.touched = true;
    sprite_mut(&mut world, young_id).parents = vec![mother_id];
    while world.state.objects.iter().next().is_some() {
        world.step();
    }
    let showing = |world: &World| {
        crate::family::relation(
            &world.state,
            &world.data,
            young_id,
            Target::Sprite(mother_id),
        )
        .showing
    };
    assert!(!showing(&world), "the berry is gone");
    world.step();
}

#[test]
fn a_parent_heard_calling_out_of_sight_shows_its_young_nothing() {
    // M2 design §18.1: a young sprite *sees* its parent showing it
    // something. Its mother drinks, knowing water is good for thirst,
    // out of its sight, and it heard her call: she draws its eye as a
    // call does, but not as showing it something, so the ball it sees
    // draws it more.
    let data = builtin();
    let rows = [format!("~{}", ".".repeat(29)), ".".repeat(30)];
    let rows = rows.each_ref().map(String::as_str);
    let seeing_6 = |genes: &str| {
        let genes = format!(r#"Trait(trait: "sense_radius", value: 6.0), {genes}"#);
        genome_of(&genes, &data)
    };
    let looks = looks_to_a_parent_showing(
        r#"AttentionInstinct(input: "always", category: "toy", weight: 0.5),
        BrainParam(param: "salience_gain", value: 0.0),"#,
    );
    let (mother, young_tile, ball) = (at(5, 0), at(20, 1), at(22, 1));
    let mut world = WorldBuilder::new(&rows)
        .objects(&[(ball, "ball")])
        .sprites_with(vec![
            (mother, Some(seeing_6(DRINKS))),
            (young_tile, Some(seeing_6(&looks))),
        ])
        .newborns(&[young_tile])
        .data(data)
        .build();
    let (mother_id, young_id) = (id_at(&world, mother), id_at(&world, young_tile));
    know_water(&mut world, mother_id);
    let tick = world.state.tick;
    let young = sprite_mut(&mut world, young_id);
    young.parents = vec![mother_id];
    young.heard.push(crate::courtship::Heard {
        caller: mother_id,
        from: mother,
        tick,
    });
    world.step();
    let action = world.sprite(mother_id).expect("mother").action();
    let action = action.expect("her action");
    assert_eq!(action.verb, Verb::Drink, "she goes to drink");
    let walking = matches!(action.progress, crate::action::Progress::Walking { .. });
    assert!(walking, "{:?}", action.progress);
    let showing = crate::family::relation(
        &world.state,
        &world.data,
        young_id,
        Target::Sprite(mother_id),
    );
    assert!(!showing.showing, "she's out of its sight");
    let looks_at = world.sprite(young_id).expect("young").attending_to();
    assert_eq!(looks_at, Some(ball));
}

#[test]
fn a_world_nobody_trains_that_doesn_t_watch_is_taught_nothing() {
    // Design §5.6, #187: only the Cursor's Reward and Correct teach a
    // taught habit, and watching passes one on (M2 design §18.2), so
    // sprites nobody trains that don't learn by watching learn as they
    // did before.
    let data = builtin_changing(
        "physiology.ron",
        &[(
            r#""watch_rate":        (range: (0.0, 1.0),"#,
            r#""watch_rate":        (range: (0.0, 0.0),"#,
        )],
    );
    let config = WorldConfig::builtin(&data);
    let mut world = World::new(config, data, 7);
    for _ in 0..2_000 {
        world.step();
    }
    let types = || {
        let brains = world.state.sprites.iter().map(|(_, sprite)| &sprite.brain);
        brains.flat_map(|brain| brain.experience.types.values())
    };
    assert!(
        types().any(|known| known.habits.iter().any(|&h| h != 0.0)),
        "it learns"
    );
    assert!(types().all(|known| known.taught_habits.iter().all(|&h| h == 0.0)));
}

#[test]
fn a_sprite_with_no_stamina_left_still_walks() {
    // Design §3.7: low stamina only raises tiredness.
    let (mut world, id) = row_with_a_walker(
        6,
        &[ScriptedAction::Wander {
            destination: Pos { x: 5, y: 0 },
        }],
    );
    let stamina = world.data.physiology().indices.stamina;
    let sprite = sprite_mut(&mut world, id);
    sprite.body.start_at(stamina, 0.0);
    for _ in 0..5 {
        world.step();
    }
    let sprite = world.state.sprites.get(id).expect("the walker");
    assert_eq!(sprite.pos, Pos { x: 5, y: 0 });
    assert_eq!(sprite.body.chems[stamina], 0.0, "spent, and none came back");
}

#[test]
fn a_blocked_sprite_banks_points_only_up_to_its_next_step() {
    // Design §3.7: its next step, onto the shallow water a resting
    // sprite stands on, costs 250.
    let (walker, rester) = (at(0, 0), at(1, 0));
    let mut world = WorldBuilder::new(&[".~.."])
        .sprites_made(&[
            (walker, |data| speed(10.0, data)),
            (rester, |data| speed(10.0, data)),
        ])
        .scripts(&[
            (walker, wander_to(at(3, 0))),
            (rester, ScriptedAction::Rest),
        ])
        .build();
    let id = world.sprite_at(walker).expect("the walker").id();
    sprite_mut(&mut world, id).move_points = 1_000;
    world.step();
    let sprite = world.state.sprites.get(id).expect("the walker");
    assert_eq!(sprite.pos, walker, "held up");
    assert_eq!(sprite.move_points, 250, "one shallow-water step's worth");
}

#[test]
fn a_sprite_arriving_keeps_at_most_one_step_s_worth_of_points() {
    let (mut world, id) = row_with_a_walker(
        6,
        &[ScriptedAction::Wander {
            destination: Pos { x: 1, y: 0 },
        }],
    );
    sprite_mut(&mut world, id).move_points = 1_000;
    world.step();
    let walker = world.state.sprites.get(id).expect("the walker");
    assert_eq!(walker.pos, Pos { x: 1, y: 0 });
    assert_eq!(walker.move_points, 100, "one grass step's worth");
}

#[test]
fn a_counter_above_its_maximum_breaks_an_invariant() {
    let mut world = field_with_a_bush();
    the_bush(&mut world.state).counters[0] = 7;
    assert!(world.check_invariants().is_err());
}

/// A sprite of `genes` on (1, 1) of the field with a bush, doing
/// `script` first.
fn field_with_one(genes: &str, script: &[ScriptedAction]) -> (World, EntityId) {
    let data = builtin();
    let text = format!("(format: 1, genes: [{genes}])");
    let genome = Genome::from_ron(&text, &data).expect("a valid genome");
    let tile = at(1, 1);
    let world = field()
        .data(data)
        .sprites_with(vec![(tile, Some(genome))])
        .script(tile, script)
        .build();
    let id = world.sprites().next().expect("the sprite").id();
    (world, id)
}

/// The ticks of the entries in sprite `id`'s trace, oldest first.
fn trace_ticks(world: &World, id: EntityId) -> Vec<u64> {
    let sprite = world.state.sprites.get(id).expect("the sprite");
    sprite.brain.trace.iter().map(|entry| entry.tick).collect()
}

#[test]
fn a_deciding_sprite_adds_one_trace_entry_every_tick_even_while_its_action_continues() {
    // It rests for 10 ticks: one choice, then 4 ticks carrying on.
    let (mut world, id) = field_with_one(
        r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0)"#,
        &[],
    );
    let started = (0..5)
        .flat_map(|_| world.step())
        .filter(|e| matches!(e.kind, EventKind::ActionStarted { .. }))
        .count();
    assert_eq!(started, 1, "one rest, still going");
    assert_eq!(trace_ticks(&world, id), [0, 1, 2, 3, 4]);
}

#[test]
fn a_sprite_on_a_scripted_action_adds_no_trace_entries() {
    let (mut world, id) = field_with_one("", &[ScriptedAction::Rest]);
    for _ in 0..5 {
        world.step();
    }
    assert_eq!(trace_ticks(&world, id), [] as [u64; 0]);
}

/// A trace entry at `tick`, choosing Eat.
fn eat_entry(tick: u64) -> TraceEntry {
    TraceEntry {
        tick,
        verb: Some(Verb::Eat),
        subject: None,
        motive: None,
    }
}

#[test]
fn a_sprite_marked_dying_learns_nothing_at_step_4() {
    let (mut world, first, second) = field_with_sprites();
    let reward = world.data.physiology().indices.reward;
    for id in [first, second] {
        let entry = eat_entry(0);
        let sprite = sprite_mut(&mut world, id);
        sprite.brain.trace.push_back(entry);
        sprite.body.chems[reward] = 0.5;
    }
    world.state.tick = 1;
    step_4(&mut world, &[second]);
    let sprite = |id| world.state.sprites.get(id).expect("a sprite");
    assert_eq!(sprite(first).brain.felt, 0.5, "the living one learns");
    assert_eq!(sprite(first).body.chems[reward], 0.0);
    assert_eq!(sprite(second).brain.felt, 0.0, "the dying one doesn't");
    assert_eq!(
        sprite(second).body.chems[reward],
        0.5,
        "nor uses up its reward"
    );
}

#[test]
fn with_learning_switched_off_step_4_uses_up_reward_and_learns_nothing() {
    // Design v16 §5.6, for a lab scenario's control run (§7.1).
    let (mut world, first, _) = field_with_sprites();
    world.state.learning = false;
    let indices = world.data.physiology().indices;
    let bush = world.data.category_named("bush").expect("a category");
    let sprite = sprite_mut(&mut world, first);
    sprite.brain.touched = Some(Touch {
        tick: 0,
        verb: Some(Verb::Eat),
        subject: Subject::Category(bush),
        sprite: None,
        novelty: 1.0,
        by_cursor: false,
        place: None,
    });
    sprite.body.chems[indices.reward] = 0.5;
    step_4(&mut world, &[]);
    let sprite = world.state.sprites.get(first).expect("a sprite");
    assert_eq!(sprite.brain.felt, 0.5, "felt, and used up");
    assert_eq!(sprite.body.chems[indices.reward], 0.0);
    assert_eq!(sprite.brain.memory(&world.data), [], "learned nothing");
}

#[test]
fn with_learning_switched_off_the_cursor_s_touch_lands_and_teaches_nothing() {
    // Design v21 §5.6: a control run's sprites still feel a pet, and its
    // reach back is used up with it.
    let (mut world, first, _) = field_with_sprites();
    world.state.learning = false;
    let bush = world.data.category_named("bush").expect("a category");
    sprite_mut(&mut world, first).brain.touched = Some(Touch {
        tick: 0,
        verb: Some(Verb::Eat),
        subject: Subject::Category(bush),
        sprite: None,
        novelty: 1.0,
        by_cursor: false,
        place: None,
    });
    world.submit(Command::Reward {
        sprite: first,
        amplified: false,
        reach_back: 10,
    });
    world.apply_commands(&mut Vec::new());
    step_4(&mut world, &[]);
    let sprite = world.state.sprites.get(first).expect("a sprite");
    assert_eq!(sprite.brain.felt, 0.5, "felt, and used up");
    assert_eq!(sprite.brain.reach_back, None, "used up");
    assert_eq!(sprite.brain.memory(&world.data), [], "learned nothing");
}

#[test]
fn the_state_hash_covers_what_every_sprite_has_learned() {
    let (mut world, _, second) = field_with_sprites();
    let entry = eat_entry(0);
    let before = world.state_hash();
    let data = world.data.clone();
    let brain = &mut sprite_mut(&mut world, second).brain;
    brain.trace.push_back(entry);
    let traced = world.state_hash();
    assert_ne!(traced, before, "the trace is hashed");
    let brain = &mut sprite_mut(&mut world, second).brain;
    let bush = data.category_named("bush").expect("a category");
    brain.touched = Some(Touch {
        tick: 0,
        verb: Some(Verb::Eat),
        subject: Subject::Category(bush),
        sprite: None,
        novelty: 1.0,
        by_cursor: false,
        place: None,
    });
    let hunger = |brain: &mut Brain, level| {
        let needs = [vec![level], vec![0.0; data.need_places().len() - 1]].concat();
        Signals {
            relief: brain.relief(&needs, &data),
            needs,
            ..Default::default()
        }
    };
    let signals = hunger(brain, 1.0);
    brain.learn(0, &signals, 1.0, &data);
    let touched = world.state_hash();
    assert_ne!(touched, traced, "what it touched is hashed");
    let brain = &mut sprite_mut(&mut world, second).brain;
    let signals = hunger(brain, 0.5);
    brain.learn(1, &signals, 1.0, &data);
    let learned = world.state_hash();
    assert_ne!(learned, touched, "what it learned is hashed");
    let brain = &mut sprite_mut(&mut world, second).brain;
    let water = Target::Water(Pos { x: 0, y: 0 });
    let place = (
        Subject::Category(data.water_category()),
        water,
        Pos { x: 0, y: 0 },
    );
    brain.experience.remember_place(
        place.0,
        place.1,
        place.2,
        crate::physiology::RememberedPlaces {
            held: 8,
            per_kind: 8,
            merge: 5,
        },
    );
    assert_ne!(
        world.state_hash(),
        learned,
        "the places it remembers are hashed"
    );
}

/// Sprite `id`'s brain in `world` remembers `n` places on tiles of its
/// own, row by row, each its own place.
fn remember_places(world: &mut World, id: EntityId, n: u16) {
    let water = Subject::Category(world.data.water_category());
    let width = world.state.map.width();
    let brain = &mut sprite_mut(world, id).brain;
    for n in 0..n {
        let at = Pos {
            x: n % width,
            y: n / width,
        };
        brain.experience.remember_place(
            water,
            Target::Water(at),
            at,
            crate::physiology::RememberedPlaces {
                held: 100,
                per_kind: 100,
                merge: 0,
            },
        );
    }
}

#[test]
fn a_remembered_place_off_the_map_or_too_many_or_out_of_range_breaks_an_invariant() {
    // M2 design §7.
    let (mut world, first, _) = field_with_sprites();
    remember_places(&mut world, first, 1);
    assert_eq!(world.check_invariants(), Ok(()));
    let breaks: [fn(&mut World, EntityId); 4] = [
        |world, id| remember_places(world, id, 9),
        |world, id| sprite_mut(world, id).brain.experience.places[0].at = Pos { x: 999, y: 0 },
        |world, id| sprite_mut(world, id).brain.experience.places[0].recall = 1.5,
        |world, id| sprite_mut(world, id).brain.experience.places[0].recall = f32::NAN,
    ];
    for (i, broken) in breaks.into_iter().enumerate() {
        let (mut world, first, _) = field_with_sprites();
        remember_places(&mut world, first, 1);
        broken(&mut world, first);
        assert!(world.check_invariants().is_err(), "break {i}");
    }
}

/// A world 40 tiles wide whose one sprite remembers water on each tile
/// of the top row in `xs`, as no limits would stop it.
fn remembering_water_on(xs: &[u16]) -> World {
    let (water, land) = ("~".repeat(40), ".".repeat(40));
    let mut world = WorldBuilder::new(&[&water, &land])
        .starters(&[at(0, 1)])
        .seed(7)
        .build();
    let id = world.sprites().next().expect("the sprite").id();
    let water = Subject::Category(world.data.water_category());
    let brain = &mut sprite_mut(&mut world, id).brain;
    for &x in xs {
        let at = Pos { x, y: 0 };
        let loose = crate::physiology::RememberedPlaces {
            held: 100,
            per_kind: 100,
            merge: 0,
        };
        brain
            .experience
            .remember_place(water, Target::Water(at), at, loose);
    }
    world
}

/// A sprite going to a ball at (36, 2), its eye on it, very thirsty,
/// that remembers water at (0, 2) beyond a wall whose one gap a bush
/// stands in, with `genes` and `objects` as well; and the ball. Its next
/// step 5 turns its eye to the water (M2 design §7.4).
fn going_to_a_ball_with_water_beyond_a_wall(
    genes: &str,
    objects: &[(Pos, &str)],
) -> (World, EntityId, EntityId) {
    let data = builtin();
    // A wall with a gap that a bush stands in.
    let (wall, gap) = (
        format!("~{}#{}", ".".repeat(19), ".".repeat(19)),
        format!("~{}", ".".repeat(39)),
    );
    let rows = [&wall, &wall, &gap, &wall, &wall].map(String::as_str);
    let genome = Genome::from_ron(
        &format!(
            r#"(format: 1, genes: [
                Trait(trait: "speed", value: 10.0),
                Trait(trait: "sense_radius", value: 6.0),
                Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),
                Instinct(inputs: [("always", false)], verb: Approach, weight: 0.3),
                BrainParam(param: "tau_base", value: 0.05),
                BrainParam(param: "salience_gain", value: 0.0),
                BrainParam(param: "curiosity", value: 0.0),
                {genes}
            ])"#
        ),
        &data,
    )
    .expect("a valid genome");
    let mut objects = objects.to_vec();
    objects.extend([
        (Pos { x: 20, y: 2 }, "berry_bush"),
        (Pos { x: 36, y: 2 }, "ball"),
    ]);
    let mut world = WorldBuilder::new(&rows)
        .data(data)
        .objects(&objects)
        .sprites_with(vec![(at(30, 2), Some(genome))])
        .seed(7)
        .build();
    let id = world.sprites().next().expect("the sprite").id();
    let ball = world.object_at(Pos { x: 36, y: 2 }).expect("the ball").id();
    let ball_type = world.data.object_type_id("ball");
    let toy = world.state.category_of(&world.data, Target::Object(ball));
    let at = Pos { x: 0, y: 2 };
    let water = world.state.subject_of(&world.data, Target::Water(at));
    let needs = world.data.needs().count();
    let quench = world.data.needs().position(|n| n == "thirst");
    let (quench, thirst) = (quench.expect("thirst"), thirst_chem(&world));
    // A step first, for its flood.
    world.step();
    let tick = world.state.tick;
    let sprite = world.state.sprites.get_mut(id).expect("the sprite");
    let limits = world.data.physiology().remembered_places;
    sprite
        .brain
        .experience
        .remember_place(water, Target::Water(at), at, limits);
    let known = sprite.brain.experience.learn_about(water, needs);
    known.worth[quench] = 1.0;
    known.touched = true;
    sprite.body.chems[thirst] = 1.0;
    let target = Some((Target::Object(ball), ball_type));
    let mut events = Vec::new();
    let goal = Some(Pos { x: 35, y: 2 });
    crate::action::start(
        sprite,
        id,
        Verb::Approach,
        goal,
        target,
        false,
        tick,
        &mut events,
    );
    sprite.brain.attended = Some(toy);
    (world, id, ball)
}

/// Where the built-in pack's thirst is among its chemicals.
fn thirst_chem(world: &World) -> usize {
    let chemicals = world.data.chemicals();
    let thirst = chemicals.iter().position(|c| c.name == "thirst");
    thirst.expect("thirst")
}

#[test]
fn a_remembered_place_with_no_way_there_leaves_what_the_sprite_is_doing_be() {
    // M2 design §7.4: going to a ball, very thirsty, its eye turns to
    // water it remembers beyond a wall. With no way there, it forgets
    // the water and goes on to the ball, and decides nothing new.
    let (mut world, id, ball) = going_to_a_ball_with_water_beyond_a_wall("", &[]);
    let at = Pos { x: 0, y: 2 };
    let events = world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.remembered_places(), [], "it forgot the water");
    let ended = events
        .iter()
        .any(|e| matches!(e.kind, EventKind::ActionEnded { id: ended, .. } if ended == id));
    assert!(!ended, "{events:?}");
    let action = sprite.action().expect("its action");
    assert_eq!(action.verb, Verb::Approach);
    assert_eq!(action.target, Some(Target::Object(ball)));
    assert_ne!(sprite.attending_to(), Some(at), "no decision about it");
}

#[test]
fn a_remembered_place_with_no_way_there_leaves_the_sprite_s_eye_where_it_was() {
    // M2 design §7.4: a place it can't get to never interrupts anything.
    // A bush in sight draws its eye a little more than the ball, but by
    // less than `attention_margin` (.2), so its eye stays on the ball,
    // the tick after the water took it too.
    let (mut world, id, ball) = going_to_a_ball_with_water_beyond_a_wall(
        r#"AttentionInstinct(input: "always", category: "bush", weight: 0.1),"#,
        &[(Pos { x: 33, y: 1 }, "berry_bush")],
    );
    world.step();
    assert_eq!(world.sprites().next().expect("it").remembered_places(), []);
    // No longer thirsty, it has no reason to change what it's doing.
    let thirst = thirst_chem(&world);
    sprite_mut(&mut world, id).body.chems[thirst] = 0.0;
    let events = world.step();
    let ended = events
        .iter()
        .any(|e| matches!(e.kind, EventKind::ActionEnded { id: ended, .. } if ended == id));
    assert!(!ended, "{events:?}");
    let sprite = world.sprites().next().expect("the sprite");
    let action = sprite.action().expect("its action");
    assert_eq!(action.verb, Verb::Approach);
    assert_eq!(action.target, Some(Target::Object(ball)));
}

#[test]
fn a_remembered_place_with_no_way_there_leaves_its_eye_on_what_it_is_doing_that_tick() {
    // M2 design §7.4, §18.1: carrying on, its latest step 5 is that
    // tick's, with its eye on the ball, so it watches as it would have.
    let (mut world, id, ball) = going_to_a_ball_with_water_beyond_a_wall("", &[]);
    let tick = world.state.tick;
    world.step();
    let toy = world.state.category_of(&world.data, Target::Object(ball));
    let brain = &world.state.sprites.get(id).expect("the sprite").brain;
    assert_eq!(brain.attended, Some(toy));
    let eye = brain.snapshot.as_ref().expect("a step 5");
    assert_eq!(eye.tick, tick, "its latest step 5 is this tick's");
    assert_eq!(eye.attended, Some(toy));
    assert_eq!(eye.target, Some(Target::Object(ball)));
    assert_eq!(eye.verb, Some(Verb::Approach));
    let entry = brain.trace.back().expect("a trace entry");
    assert_eq!((entry.tick, entry.verb), (tick, Some(Verb::Approach)));
}

#[test]
fn an_idle_sprite_with_no_way_to_a_remembered_place_still_takes_its_step_5() {
    // M2 design §7.4, §18.1: doing nothing, it turns its eye to water it
    // remembers beyond the wall. With no way there, it decides nothing,
    // its eye goes back where it was, and its latest step 5 is that
    // tick's, so it watches as it would have.
    let (mut world, id, ball) = going_to_a_ball_with_water_beyond_a_wall("", &[]);
    sprite_mut(&mut world, id).action = None;
    let tick = world.state.tick;
    world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.remembered_places(), [], "it forgot the water");
    assert!(sprite.action().is_none(), "it decided nothing");
    let toy = world.state.category_of(&world.data, Target::Object(ball));
    let brain = &world.state.sprites.get(id).expect("the sprite").brain;
    assert_eq!(brain.attended, Some(toy));
    let eye = brain.snapshot.as_ref().expect("a step 5");
    assert_eq!(eye.tick, tick, "its latest step 5 is this tick's");
    assert_eq!((eye.attended, eye.verb), (Some(toy), None));
}

#[test]
fn a_sprite_lets_its_last_step_5_go_when_its_stage_changes() {
    // M2 design §9.2, §18.1: the concepts its last step 5 was taken with
    // may be gone, so until it takes another it learns nothing by
    // watching, and one watching it is shown nothing.
    let (mut world, id, _) = going_to_a_ball_with_water_beyond_a_wall("", &[]);
    world.step();
    let tick = world.state.tick;
    let sprite = world.state.sprites.get_mut(id).expect("the sprite");
    assert!(sprite.brain.snapshot.is_some(), "it took a step 5");
    let grew = sprite.grow(tick + 100_000, &world.data);
    assert_eq!(grew, Some((LifeStage::Adult, LifeStage::Old)));
    assert!(sprite.brain.snapshot.is_none());
}

/// A world drawn from `row`, 40 tiles long, three times, with `objects`,
/// and a sprite on (35, 1) that sees 6 tiles, with `genes`, that knows
/// what `subject` (its type, or the water's) is as `learn` makes it, and
/// remembers `target` on `at`, as well as `recall`; and the sprite.
fn remembering(
    row: &str,
    objects: &[(Pos, &str)],
    genes: &str,
    target: impl Fn(&World) -> Target,
    recall: f32,
    learn: impl Fn(&mut TypeMemory),
) -> (World, EntityId) {
    let data = builtin();
    let genome = Genome::from_ron(
        &format!(
            r#"(format: 1, genes: [
                Trait(trait: "sense_radius", value: 6.0),
                BrainParam(param: "salience_gain", value: 0.0),
                BrainParam(param: "curiosity", value: 0.0),
                {genes}
            ])"#
        ),
        &data,
    )
    .expect("a valid genome");
    let mut world = WorldBuilder::new(&[row; 3])
        .data(data)
        .objects(objects)
        .sprites_with(vec![(at(35, 1), Some(genome))])
        .build();
    let id = world.sprites().next().expect("the sprite").id();
    let target = target(&world);
    let (at, _) = world.state.whereabouts(&world.data, target).expect("it");
    let subject = world.state.subject_of(&world.data, target);
    let needs = world.data.needs().count();
    let limits = world.data.physiology().remembered_places;
    let brain = &mut sprite_mut(&mut world, id).brain;
    brain.experience.remember_place(subject, target, at, limits);
    brain.experience.places[0].recall = recall;
    let known = brain.experience.learn_about(subject, needs);
    known.touched = true;
    learn(known);
    (world, id)
}

#[test]
fn a_remembered_place_is_not_offered_while_one_of_its_kind_is_in_sight() {
    // M2 design §7.3: it looks at the water it sees, not the water it
    // remembers, even when it has learned water is bad, which draws the
    // eye less the fainter the water is remembered.
    let row = format!("~{}~", ".".repeat(38));
    let (mut world, id) = remembering(
        &row,
        &[],
        r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#,
        |_| Target::Water(Pos { x: 0, y: 1 }),
        0.5,
        |known| known.bad = -0.5,
    );
    world.step();
    let brain = &world.state.sprites.get(id).expect("the sprite").brain;
    let eye = brain.snapshot.as_ref().and_then(|eye| eye.target);
    assert!(
        matches!(eye, Some(Target::Water(pos)) if pos.x == 39),
        "{eye:?}"
    );
}

#[test]
fn a_remembered_place_whose_thing_is_gone_is_not_offered() {
    // M2 design §7.3: out of sight, it hasn't seen the bush is gone, so
    // it still remembers it, but its eye doesn't turn there, hungry as
    // it is, and it doesn't find there's no way to it and forget it.
    let row = ".".repeat(40);
    let (mut world, id) = remembering(
        &row,
        &[(Pos { x: 2, y: 1 }, "berry_bush")],
        r#"InitialConcentration(chem: "hunger", value: 1.0),
           Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),"#,
        |world| {
            let bush = world.object_at(Pos { x: 2, y: 1 }).expect("the bush");
            Target::Object(bush.id())
        },
        1.0,
        |known| known.worth[0] = 0.8,
    );
    let bush = world.state.objects.at(Pos { x: 2, y: 1 }).expect("it");
    world.state.objects.remove(bush);
    world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.remembered_places().len(), 1, "it hasn't seen");
    let brain = &world.state.sprites.get(id).expect("the sprite").brain;
    let eye = brain.snapshot.as_ref().expect("a step 5");
    assert_eq!(eye.target, None, "{eye:?}");
}

#[test]
fn too_many_places_of_a_kind_or_two_too_near_break_an_invariant() {
    // M2 design §7.2: 3 of a kind at most, none within 5 of another.
    assert_eq!(remembering_water_on(&[0, 6, 12]).check_invariants(), Ok(()));
    assert!(
        remembering_water_on(&[0, 6, 12, 18])
            .check_invariants()
            .is_err()
    );
    assert!(remembering_water_on(&[0, 5]).check_invariants().is_err());
}

#[test]
fn a_save_from_before_the_limits_on_a_kind_loads_within_them() {
    // M2 design §7.2: a save from before `per_kind` and the merge rule
    // may hold 8 places of water, some near each other.
    let world = remembering_water_on(&[0, 2, 10, 20, 30, 36]);
    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.check_invariants(), Ok(()));
    let sprite = loaded.sprites().next().expect("the sprite");
    let places: Vec<Pos> = sprite.remembered_places().iter().map(|p| p.at).collect();
    assert_eq!(places.len(), 3, "{places:?}");
}

#[test]
fn a_learned_link_past_one_or_not_a_number_breaks_an_invariant() {
    for broken in [1.5, f32::NAN] {
        let (mut world, first, _) = field_with_sprites();
        assert_eq!(world.check_invariants(), Ok(()));
        let brain = &mut sprite_mut(&mut world, first).brain;
        brain.break_a_link_for_test(broken);
        assert!(world.check_invariants().is_err(), "{broken}");
    }
}

#[test]
fn a_learned_value_out_of_its_range_or_not_a_number_breaks_an_invariant() {
    // Design v16 §5.6: worth and good 0 to 1, bad −1 to 0, the rest −1 to 1.
    fn known(brain: &mut Brain) -> &mut TypeMemory {
        brain.experience.learn_about(Subject::ObjectType(1), 1)
    }
    let breaks: [fn(&mut Brain); 8] = [
        |brain| known(brain).worth[0] = -0.1,
        |brain| known(brain).good = 1.5,
        |brain| known(brain).bad = 0.2,
        |brain| known(brain).habits[0] = f32::NAN,
        |brain| known(brain).taught_habits[0] = 1.5,
        // What it took on by watching (M2 design §19.7).
        |brain| known(brain).watched_bad = 0.2,
        |brain| known(brain).watched_habits[0] = 1.5,
        |brain| known(brain).familiarity = 2.0,
    ];
    for (i, broken) in breaks.into_iter().enumerate() {
        let (mut world, first, _) = field_with_sprites();
        broken(&mut sprite_mut(&mut world, first).brain);
        assert!(world.check_invariants().is_err(), "break {i}");
    }
}

#[test]
fn a_felt_value_that_is_not_a_number_breaks_an_invariant() {
    let (mut world, first, _) = field_with_sprites();
    sprite_mut(&mut world, first).brain.felt = f32::NAN;
    assert!(world.check_invariants().is_err());
}

/// Sprites that rest and do nothing else, on `tiles` of a row of grass
/// `length` long, made with `data`, before any step.
fn resters(data: DataPack, length: usize, tiles: &[u16]) -> (World, Vec<EntityId>) {
    let row = ".".repeat(length);
    let genes = r#"(format: 2, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.02),
    ])"#;
    let genome = Genome::from_ron(genes, &data).expect("a valid genome");
    let sprites = tiles.iter().map(|&x| (at(x, 0), Some(genome.clone())));
    let world = WorldBuilder::new(&[row.as_str()])
        .sprites_with(sprites.collect())
        .data(data)
        .build();
    let ids = tiles.iter().map(|&x| id_at(&world, at(x, 0))).collect();
    (world, ids)
}

/// Makes `a` and `b` partners, as an egg of theirs would.
fn make_partners(world: &mut World, a: EntityId, b: EntityId) -> Vec<Event> {
    let mut events = Vec::new();
    courtship::pair(&mut world.state, &world.data, a, b, &mut events);
    events
}

#[test]
fn partners_who_go_long_enough_neither_seeing_nor_hearing_the_other_part() {
    // M2 design §15.4, with `breeding.apart` 50 ticks: 50 tiles apart,
    // out of sight, resting and never calling, they part, told once.
    let data = builtin_changing("physiology.ron", &[("apart: 4500,", "apart: 50,")]);
    let (mut world, ids) = resters(data, 51, &[0, 50]);
    let (a, b) = (ids[0], ids[1]);
    let paired = make_partners(&mut world, a, b);
    assert!(matches!(
        paired[..],
        [Event {
            kind: EventKind::Paired { .. },
            ..
        }]
    ));
    let mut parted = Vec::new();
    for _ in 0..60 {
        for event in world.step() {
            if let EventKind::Parted { a, b } = event.kind {
                parted.push((event.tick, a, b));
            }
        }
    }
    assert_eq!(parted, [(50, a, b)]);
    for id in [a, b] {
        assert_eq!(world.sprite(id).unwrap().partner(), None);
    }
    // Side by side, they never part.
    let data = builtin_changing("physiology.ron", &[("apart: 4500,", "apart: 50,")]);
    let (mut world, ids) = resters(data, 5, &[0, 1]);
    make_partners(&mut world, ids[0], ids[1]);
    for _ in 0..60 {
        let events = world.step();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Parted { .. }))
        );
    }
    assert_eq!(
        world.sprite(ids[0]).unwrap().partner(),
        Some((ids[1], false))
    );
}

#[test]
fn a_sprite_whose_partner_dies_is_told_and_names_it_until_it_pairs_again() {
    // M2 design §15.4, §15.8.
    let (mut world, ids) = resters(builtin(), 5, &[0, 1, 3]);
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    make_partners(&mut world, a, b);
    let injury = world.data.physiology().indices.injury;
    sprite_mut(&mut world, b).body.chems[injury] = 1.0;
    let events = world.step();
    let lost: Vec<_> = events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::LostPartner { id, partner } => Some((id, partner)),
            _ => None,
        })
        .collect();
    assert_eq!(lost, [(a, b)]);
    assert_eq!(world.sprite(a).unwrap().partner(), Some((b, true)));
    let lost_partner = world
        .data
        .physiology()
        .indices
        .courtship
        .expect("the built-in pack courts")
        .lost_partner;
    world.step();
    let felt = world.state.sprites.get(a).expect("a").body.loci[lost_partner];
    assert!(felt > 0.0, "it felt the loss");
    // A new partner frees the name, with no parting from the dead.
    let events = make_partners(&mut world, a, c);
    let kinds: Vec<_> = events.into_iter().map(|e| e.kind).collect();
    assert_eq!(kinds, [EventKind::Paired { a, b: c }]);
    assert_eq!(world.sprite(a).unwrap().partner(), Some((c, false)));
}

#[test]
fn a_partner_lost_out_of_sight_ends_the_sense_of_it_away_but_is_not_found() {
    // M2 design §15.6: `found_family` is for finding the one it missed,
    // not for losing it.
    let (mut world, ids) = resters(builtin(), 30, &[0, 20]);
    let (a, b) = (ids[0], ids[1]);
    make_partners(&mut world, a, b);
    world.step();
    let families = world.data.physiology().indices.families;
    let families = families.expect("the built-in pack has families");
    let locus = |world: &World, at: usize| world.state.sprites.get(a).expect("a").body.loci[at];
    assert_eq!(
        locus(&world, families.partner_away),
        1.0,
        "b is out of sight"
    );
    let injury = world.data.physiology().indices.injury;
    sprite_mut(&mut world, b).body.chems[injury] = 1.0;
    for _ in 0..3 {
        world.step();
        assert_eq!(locus(&world, families.found), 0.0, "nothing found");
    }
    assert_eq!(
        locus(&world, families.partner_away),
        0.0,
        "no partner to miss"
    );
}

#[test]
fn a_parent_remembers_its_egg_until_it_hatches() {
    // M2 design §15.6: the place an egg lay is forgotten once it has
    // hatched, as well as once it's found gone.
    let data = lone();
    let genes = r#"(format: 2, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
        Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
        InitialConcentration(chem: "longing", value: 1.0),
        InitialConcentration(chem: "feminiser", value: 1.0),
    ])"#;
    let genome = Genome::from_ron(genes, &data).expect("a valid genome");
    let mut world = WorldBuilder::new(&["......"])
        .data(data)
        .sprites_with(vec![(at(0, 0), Some(genome))])
        .build();
    let mother = id_at(&world, at(0, 0));
    let remembered = |world: &World| -> Vec<EntityId> {
        let sprite = world.state.sprites.get(mother).expect("the mother");
        sprite.eggs.iter().map(|known| known.egg).collect()
    };
    // It looks for family before an egg is laid in the tick, so it
    // notes the egg from the next.
    let mut laid = None;
    let mut noted = None;
    for _ in 0..2_000 {
        if let Some(egg) = noted {
            assert_eq!(remembered(&world), [egg], "remembered until it hatches");
        }
        noted = laid;
        for event in world.step() {
            match event.kind {
                EventKind::Laid { egg, .. } => laid = Some(egg),
                EventKind::Hatched { id, .. } if Some(id) == laid => {
                    world.step();
                    assert_eq!(remembered(&world), [], "forgotten once hatched");
                    return;
                }
                _ => {}
            }
        }
    }
    panic!("the egg never hatched");
}
