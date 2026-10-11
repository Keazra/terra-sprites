//! Life stages and ageing (M2 design §9): sprites grow up at an age their
//! genes set, and a gene may switch on as they do.

mod common;
use common::{at, builtin, genome};

use terra_sim::{
    Event, EventKind, Learned, LifeStage, Map, Part, Pos, Scenario, ScriptedAction, Thing, Verb,
    World,
};

/// A newborn sprite of `genes` alone in a small field with `objects`,
/// doing `script` first.
fn world(genes: &str, objects: &[(Pos, &str)], script: &[ScriptedAction]) -> World {
    let data = builtin();
    let map = Map::from_ascii(&["......", "......", "......"], &data).expect("valid drawing");
    let sprite = at(1, 1);
    let sprites = [(sprite, Some(genome(genes, &data)))];
    let scripted: Vec<_> = script.iter().map(|&s| (sprite, s)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
        newborns: &[sprite],
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
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
