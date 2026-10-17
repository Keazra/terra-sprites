//! Life stages and ageing (M2 design §9): sprites grow up at an age their
//! genes set, and a gene may switch on as they do.

mod common;
use common::{
    AGES_AT_ONCE, BARREN, WorldBuilder, at, builtin, builtin_changing, close, genome, starting_with,
};

use terra_sim::{
    DataPack, Event, EventKind, Learned, LifeStage, Part, Pos, ScriptedAction, Thing, Verb, World,
};

/// A newborn sprite of `genes` alone in a small field with `objects`,
/// doing `script` first.
fn world(genes: &str, objects: &[(Pos, &str)], script: &[ScriptedAction]) -> World {
    world_in(builtin(), genes, objects, script)
}

/// The same, in `data`.
fn world_in(
    data: DataPack,
    genes: &str,
    objects: &[(Pos, &str)],
    script: &[ScriptedAction],
) -> World {
    let sprite = at(1, 1);
    let genome = genome(genes, &data);
    let scripted: Vec<_> = script.iter().map(|&s| (sprite, s)).collect();
    WorldBuilder::new(&["......", "......", "......"])
        .data(data)
        .objects(objects)
        .sprites_with(vec![(sprite, Some(genome))])
        .scripts(&scripted)
        .newborns(&[sprite])
        .build()
}

/// A newborn sprite of `genes` alone in a small field, whose body never
/// needs anything, so it lives until old age harms it. Its calls cost
/// nothing too (M2 design §15.1).
fn carefree(genes: &str) -> World {
    carefree_changing(genes, &[])
}

/// The same, with `changes` to its physiology too.
fn carefree_changing(genes: &str, changes: &[(&str, &str)]) -> World {
    let mut all = vec![
        ("basal: 0.0001,", "basal: 0.0,"),
        ("per_sense_tile: 0.0000067,", "per_sense_tile: 0.0,"),
        ("hydration_loss: 0.00033", "hydration_loss: 0.0"),
        ("growth: 0.3,", "growth: 0.0,"),
        ("calls: (cost: 0.002,", "calls: (cost: 0.0,"),
    ];
    all.extend_from_slice(changes);
    world_in(builtin_changing("physiology.ron", &all), genes, &[], &[])
}

/// Steps `world` until its tick is `tick`, returning every event.
fn run_to(world: &mut World, tick: u64) -> Vec<Event> {
    let mut events = Vec::new();
    while world.tick() < tick {
        events.extend(world.step());
    }
    events
}

#[test]
fn a_sprite_grows_up_at_the_age_its_genes_set() {
    let mut world = world(r#"Trait(trait: "grows_up", value: 1500.0),"#, &[], &[]);
    let events = run_to(&mut world, 1500);
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.age(), 1500);
    assert_eq!(sprite.life_stage(), LifeStage::Young);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::GrewUp { .. })),
        "not grown up yet"
    );
    let events = world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.life_stage(), LifeStage::Adult);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.kind == EventKind::GrewUp { id: sprite.id() })
            .count(),
        1,
        "{events:?}"
    );
}

#[test]
fn an_instinct_that_switches_on_at_growing_up_joins_without_touching_what_was_learned() {
    // The sprite bites the thornbush once while young, and learns it's bad.
    // On growing up an instinct to rest switches on: it rests from then on,
    // and what it learned about thornbushes is just as it was.
    let mut world = world(
        r#"Trait(trait: "grows_up", value: 1500.0),
           Emitter(locus: Locus("pricked"), mode: Level, gain: 0.5, chem: "punishment"),
           Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
           Instinct(inputs: [("always", false)], verb: Rest, weight: 0.9, from: Adult),"#,
        &[(at(2, 1), "thornbush")],
        &[ScriptedAction::Eat { at: at(2, 1) }],
    );
    run_to(&mut world, 1500);
    let young = world.sprites().next().expect("the sprite");
    let learned = young.memory();
    assert!(
        learned
            .iter()
            .any(|m| matches!(&m.learned, Learned::Bad { thing: Thing::ObjectType(t) } if t == "thornbush")
                && m.amount.abs() > 0.1),
        "it learned thornbushes are bad: {learned:?}"
    );
    assert_ne!(young.action().map(|a| a.verb), Some(Verb::Rest));

    run_to(&mut world, 1502);
    let adult = world.sprites().next().expect("the sprite");
    assert_eq!(
        adult.memory(),
        learned,
        "growing up leaves learning as it was"
    );
    assert_eq!(adult.action().map(|a| a.verb), Some(Verb::Rest));
    let explanation = adult.explain().expect("it has decided");
    assert!(
        explanation
            .contributions
            .iter()
            .any(|c| c.part == Part::Concept(vec![("always", false)]) && c.amount > 0.5),
        "the new instinct drives the rest: {explanation:?}"
    );
}

#[test]
fn a_gene_that_switches_on_at_growing_up_works_from_that_tick_s_chemistry() {
    // M2 design §9.1: its stage changes at the start of step 3, before its
    // chemistry runs, so a feeling that switches on rises that tick.
    let mut world = world_in(
        builtin_changing("physiology.ron", &AGES_AT_ONCE),
        r#"Trait(trait: "grows_up", value: 3.0),
           Emitter(locus: Locus("always"), mode: Level, gain: 0.1, chem: "longing", from: Adult),"#,
        &[],
        &[],
    );
    run_to(&mut world, 3);
    let young = world.sprites().next().expect("the sprite");
    assert_eq!(young.life_stage(), LifeStage::Young);
    assert_eq!(young.chemical("longing"), Some(0.0));
    let events = world.step();
    let adult = world.sprites().next().expect("the sprite");
    assert!(events.contains(&Event {
        tick: 3,
        kind: EventKind::GrewUp { id: adult.id() }
    }));
    let longing = adult.chemical("longing").expect("longing");
    assert!(close(longing, 0.1), "{longing}");
}

#[test]
fn a_worn_sprite_s_wear_is_saved_and_hashed() {
    // M2 design §9.4: wear is world state. A newborn this hungry is
    // starving from the start, so a hard life wears it.
    let mut world = world_in(starting_with(0.1, 1.0), "", &[], &[]);
    run_to(&mut world, 50);
    let wear = world.sprites().next().expect("the sprite").wear();
    assert!(wear > 0.005, "{wear}");
    let mut loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    assert_eq!(loaded.sprites().next().expect("the sprite").wear(), wear);
    run_to(&mut world, 100);
    run_to(&mut loaded, 100);
    assert_eq!(loaded.state_hash(), world.state_hash());
}

#[test]
fn a_grown_up_sprite_loads_grown_up_with_its_switched_genes_working() {
    let mut world = world(
        r#"Trait(trait: "grows_up", value: 1500.0),
           Instinct(inputs: [("always", false)], verb: Rest, weight: 0.9, from: Adult),"#,
        &[],
        &[],
    );
    run_to(&mut world, 1501);
    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    let sprite = loaded.sprites().next().expect("the sprite");
    assert_eq!(sprite.life_stage(), LifeStage::Adult);
    let mut loaded = loaded;
    let events = loaded.step();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, EventKind::GrewUp { .. })),
        "it grew up once, before the save"
    );
}

#[test]
fn a_sprite_grows_old_at_the_age_its_genes_set() {
    let mut world = carefree(
        r#"Trait(trait: "grows_up", value: 1500.0),
           Trait(trait: "grows_old", value: 6000.0),"#,
    );
    let events = run_to(&mut world, 6001);
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.life_stage(), LifeStage::Old);
    let id = sprite.id();
    let grew_old: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::GrewOld { id })
        .collect();
    assert_eq!(grew_old.len(), 1, "{grew_old:?}");
    assert_eq!(grew_old[0].tick, 6000);
    assert_eq!(sprite.wear(), 0.0, "an easy life, alone with nothing");
}

#[test]
fn a_sprite_whose_genes_have_it_grow_old_before_it_grows_up_does_both_at_once() {
    let mut world = carefree(
        r#"Trait(trait: "grows_up", value: 13500.0),
           Trait(trait: "grows_old", value: 6000.0),"#,
    );
    let events = run_to(&mut world, 13_501);
    let id = world.sprites().next().expect("the sprite").id();
    let stages: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::GrewUp { .. } | EventKind::GrewOld { .. }))
        .map(|e| (e.tick, e.kind.clone()))
        .collect();
    assert_eq!(
        stages,
        [
            (13_500, EventKind::GrewUp { id }),
            (13_500, EventKind::GrewOld { id })
        ]
    );
}

#[test]
fn an_old_sprite_s_weakened_genes_are_saved_and_hashed() {
    // M2 design §9.4: gene health is world state.
    // Its speed and sense radius genes come first, and weaken.
    let mut world = carefree_changing(
        r#"Trait(trait: "grows_up", value: 1500.0),
           Trait(trait: "grows_old", value: 6000.0),"#,
        &[("ticks: 3600,", "ticks: 500,")],
    );
    run_to(&mut world, 6_400);
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.life_stage(), LifeStage::Old);
    assert!(sprite.vigour() < 1.0, "its genes have weakened");
    assert!(sprite.gene_health()[..2].iter().any(|&h| h < 1.0));
    assert!(
        sprite.gene_health()[2..].iter().all(|&h| h == 1.0),
        "settings don't weaken: {:?}",
        sprite.gene_health()
    );
    let before = world.state_hash();
    let mut loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), before);
    let loaded_sprite = loaded.sprites().next().expect("the sprite");
    assert_eq!(loaded_sprite.gene_health(), sprite.gene_health());
    assert_eq!(loaded_sprite.traits(), sprite.traits());
    run_to(&mut world, 6_600);
    run_to(&mut loaded, 6_600);
    assert_eq!(loaded.state_hash(), world.state_hash());
}

#[test]
fn a_starter_sprite_grows_up_at_about_an_hour_and_grows_old_at_about_five() {
    // M2 design §2, §9.3: about 4,500 and 22,500 ticks for the starter
    // genome, at the ages its own genes say.
    let data = builtin_changing(
        "physiology.ron",
        &[
            ("basal: 0.0001,", "basal: 0.0,"),
            ("per_sense_tile: 0.0000067,", "per_sense_tile: 0.0,"),
            ("per_step: 0.0002,", "per_step: 0.0,"),
            ("hydration_loss: 0.00033", "hydration_loss: 0.0"),
            ("growth: 0.3,", "growth: 0.0,"),
            BARREN,
        ],
    );
    let sprite = at(1, 1);
    let mut world = WorldBuilder::new(&["......", "......", "......"])
        .data(data)
        .sprites_with(vec![(sprite, None)])
        .newborns(&[sprite])
        .build();
    // Spawn variation moves growing up by up to a tenth, and growing old by
    // up to two fifths, so lives differ (`trait_variation`).
    let traits = world.sprites().next().expect("the sprite").traits();
    for (time, aim, spread) in [
        (traits.grows_up, 4_500.0, 0.1),
        (traits.grows_old, 22_500.0, 0.4),
    ] {
        let within = aim * (1.0 - spread)..=aim * (1.0 + spread);
        assert!(within.contains(&time), "{time} for {aim}");
    }
    let id = world.sprites().next().expect("the sprite").id();
    let events = run_to(&mut world, traits.grows_old.ceil() as u64 + 10);
    let stages: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::GrewUp { .. } | EventKind::GrewOld { .. }))
        .map(|e| (e.tick, e.kind.clone()))
        .collect();
    let tick = |time: f32| time.ceil() as u64;
    assert_eq!(
        stages,
        [
            (tick(traits.grows_up), EventKind::GrewUp { id }),
            (tick(traits.grows_old), EventKind::GrewOld { id })
        ]
    );
}
