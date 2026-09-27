//! Learning (design §5.6): step 4 consumes reward and punishment, credits
//! them back along the trace, and moves the sprite's links.

use terra_sim::{DataPack, Genome, Map, Pos, Scenario, ScriptedAction, World};

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
