//! Learning (design §5.6): step 4 consumes reward and punishment, credits
//! them back along the trace, and moves the sprite's links.

use terra_sim::{
    DataPack, EntityId, Event, EventKind, Genome, Link, Map, Pos, Scenario, ScriptedAction, Verb,
    World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn genome(genes: &str, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: 10.0),
            Trait(trait: "sense_radius", value: 10.0),
            {genes}
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `objects`, and one sprite of `genes` on
/// `sprite` doing `script`.
fn world(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprite: Pos,
    genes: &str,
    script: &[ScriptedAction],
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(sprite, Some(genome(genes, &data)))];
    let scripted: Vec<(Pos, ScriptedAction)> = script.iter().map(|&s| (sprite, s)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

#[test]
fn learning_uses_up_reward_and_punishment_and_the_sprite_felt_the_difference() {
    // A bite of thornbush at tick 0 pricks; at tick 1 the prick raises
    // punishment by .3 and reward by .1, and learning takes both in.
    let thornbush = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(thornbush, "thornbush")],
        at(1, 1),
        r#"Emitter(locus: Locus("pricked"), mode: Level, gain: 0.3, chem: "punishment"),
           Emitter(locus: Locus("pricked"), mode: Level, gain: 0.1, chem: "reward"),"#,
        // A rest after, so it doesn't bite again.
        &[ScriptedAction::Eat { at: thornbush }, ScriptedAction::Rest],
    );
    world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.felt(), 0.0, "nothing felt before the bite lands");
    world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.chemical("punishment"), Some(0.0));
    assert_eq!(sprite.chemical("reward"), Some(0.0));
    assert!((sprite.felt() - -0.2).abs() < 1e-6, "{}", sprite.felt());
    world.step();
    let sprite = world.sprites().next().expect("the sprite");
    assert_eq!(sprite.felt(), 0.0, "a one-off, gone the tick after");
}

/// The lessons in `events`, as `(sprite, link, good)`.
fn lessons(events: &[Event]) -> Vec<(EntityId, Link, bool)> {
    events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::LearnedMilestone { id, link, good } => Some((*id, link.clone(), *good)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_sprite_that_keeps_biting_a_thornbush_learns_it_is_bad_once() {
    // Always hungry, it attends to the one thornbush and eats from it; each
    // prick punishes it.
    let thornbush = at(3, 1);
    let mut world = world(
        &["......", "......", "......"],
        &[(thornbush, "thornbush")],
        at(1, 1),
        r#"InitialConcentration(chem: "hunger", value: 1.0),
           Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
           AttentionInstinct(input: "hunger", category: Thornbush, weight: 1.0),
           Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
           BrainParam(param: "learning_rate", value: 0.2),"#,
        &[],
    );
    let id = world.sprites().next().expect("the sprite").id();
    let learned: Vec<(EntityId, Link, bool)> =
        (0..300).flat_map(|_| lessons(&world.step())).collect();
    let thorn_eat = Link::Decision {
        inputs: vec![("attended_thornbush".into(), false)],
        verb: Verb::Eat,
    };
    assert!(
        learned.contains(&(id, thorn_eat.clone(), false)),
        "{learned:?}"
    );
    let times = learned
        .iter()
        .filter(|(_, link, _)| *link == thorn_eat)
        .count();
    assert_eq!(times, 1, "announced once");
}

#[test]
fn a_sprite_rewarded_for_eating_learns_eating_and_looking_at_the_bush_are_good() {
    // Always hungry, beside a fruiting bush; every bite rewards it by 1.
    let bush = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(bush, "berry_bush")],
        at(1, 1),
        r#"InitialConcentration(chem: "hunger", value: 1.0),
           Emitter(locus: Locus("ate"), mode: Level, gain: 1.0, chem: "reward"),
           Instinct(inputs: [("hunger", false)], verb: Eat, weight: 0.1),
           BrainParam(param: "learning_rate", value: 0.5),
           BrainParam(param: "tau_base", value: 0.05),"#,
        &[],
    );
    world
        .start_object(bush, "mature", &[("fruit", 6)])
        .expect("a bush with fruit");
    let id = world.sprites().next().expect("the sprite").id();
    let learned: Vec<(EntityId, Link, bool)> =
        (0..100).flat_map(|_| lessons(&world.step())).collect();
    let eat = Link::Decision {
        inputs: vec![("hunger".into(), false)],
        verb: Verb::Eat,
    };
    let look = Link::Attention {
        input: "hunger".into(),
        category: "berry_bush".into(),
    };
    assert!(learned.contains(&(id, eat, true)), "{learned:?}");
    assert!(learned.contains(&(id, look, true)), "{learned:?}");
}
