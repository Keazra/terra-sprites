//! Learning (design §5.6): step 4 consumes reward and punishment, and
//! learns what things are worth and its habits.

use terra_sim::{DataPack, Genome, Learned, Map, Pos, Scenario, ScriptedAction, World};

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
            // No fading, so what is learned reads exactly.
            BrainParam(param: "worth_fade_good", value: 0.0),
            BrainParam(param: "worth_fade_bad", value: 0.0),
            BrainParam(param: "habit_fade", value: 0.0),
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

/// What the one sprite has learned, as `(learned, value)`.
fn memory(world: &World) -> Vec<(Learned, f32)> {
    let sprite = world.sprites().next().expect("the sprite");
    sprite
        .memory()
        .into_iter()
        .map(|m| (m.learned, m.value))
        .collect()
}

/// What the one sprite has learned `learned` is worth, or 0.
fn value_of(world: &World, learned: &Learned) -> f32 {
    memory(world)
        .into_iter()
        .find(|(l, _)| l == learned)
        .map_or(0.0, |(_, value)| value)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A sprite's genome that is hungry and whose eating halves its hunger, as
/// the starter genome's `ate` emitter does.
const HUNGRY: &str = r#"InitialConcentration(chem: "hunger", value: 1.0),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),"#;

#[test]
fn a_hungry_sprite_that_eats_from_a_bush_learns_the_bush_is_good_for_hunger() {
    // Design v16 §5.6: hunger falls by .5, its relief, and the bush it bit
    // is the thing touched: .5 × worth_rate_good (.5).
    let bush = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(bush, "berry_bush")],
        at(1, 1),
        HUNGRY,
        &[ScriptedAction::Eat { at: bush }, ScriptedAction::Rest],
    );
    world
        .start_object(bush, "mature", &[("fruit", 3)])
        .expect("a bush with fruit");
    world.step();
    assert_eq!(
        memory(&world),
        [],
        "nothing learned before the bite is felt"
    );
    world.step();
    let good_for_hunger = Learned::Worth {
        thing: "berry_bush".into(),
        need: Some("hunger".into()),
    };
    assert!(
        close(value_of(&world, &good_for_hunger), 0.25),
        "{:?}",
        memory(&world)
    );
}

#[test]
fn a_prick_makes_only_the_thornbush_touched_bad_not_what_was_looked_at_before() {
    // Design v16 §5.6: bad goes to the thing touched. The sprite walks past
    // a berry bush to bite a thornbush; each prick punishes it by 1, so bad
    // is −(.8 × 1) at worth_rate_bad's .8.
    let (bush, thornbush) = (at(2, 1), at(4, 1));
    let mut world = world(
        &["......", "......", "......"],
        &[(bush, "berry_bush"), (thornbush, "thornbush")],
        at(1, 1),
        r#"Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),"#,
        &[
            ScriptedAction::Approach { at: bush },
            ScriptedAction::Eat { at: thornbush },
            ScriptedAction::Rest,
        ],
    );
    for _ in 0..6 {
        world.step();
    }
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    let bush_bad = Learned::Bad {
        thing: "berry_bush".into(),
    };
    assert!(
        close(value_of(&world, &thorns_bad), -0.8),
        "{:?}",
        memory(&world)
    );
    assert_eq!(value_of(&world, &bush_bad), 0.0, "looked at, never touched");
}

#[test]
fn a_reward_makes_the_thing_touched_good_in_general() {
    // A pet-like reward of .5 as it plays with a ball: .5 × .5.
    let ball = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(ball, "ball")],
        at(1, 1),
        r#"Emitter(locus: Locus("played"), mode: Level, gain: 0.5, chem: "reward"),"#,
        &[ScriptedAction::Play { at: ball }, ScriptedAction::Rest],
    );
    world.step();
    world.step();
    let balls_good = Learned::Worth {
        thing: "ball".into(),
        need: None,
    };
    assert!(
        close(value_of(&world, &balls_good), 0.25),
        "{:?}",
        memory(&world)
    );
}
