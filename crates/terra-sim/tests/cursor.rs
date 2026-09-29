//! The Cursor's touch (design v21 §2.5, §4.6, §5.6): Reward and Correct
//! commands, applied at step 1 of the next tick, and what a sprite learns
//! from them.

use terra_sim::{
    Command, DataPack, EntityId, EventKind, Genome, Map, Pos, Scenario, ScriptedAction, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A genome of `genes` and nothing else that would move a level, so what
/// the Cursor does reads exactly.
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
            BrainParam(param: "fear_fade", value: 0.0),
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

/// A sprite resting alone in a field, and its ID.
fn resting_sprite() -> (World, EntityId) {
    let world = world(
        &[".....", ".....", "....."],
        &[],
        at(2, 1),
        "",
        &[ScriptedAction::Rest, ScriptedAction::Rest],
    );
    let id = world.sprites().next().expect("the sprite").id();
    (world, id)
}

fn pet(sprite: EntityId) -> Command {
    Command::Reward {
        sprite,
        amplified: false,
        reach_back: 3,
    }
}

#[test]
fn a_pet_gives_the_sprite_a_good_feeling_of_a_half_on_the_next_tick() {
    // Design v21 §4.6: a pet injects reward .5 at step 1, and learning takes
    // it in at step 4 of the same tick.
    let (mut world, id) = resting_sprite();
    world.submit(pet(id));
    let events = world.step();
    assert!(
        events.iter().any(|e| e.kind
            == EventKind::Rewarded {
                id,
                amplified: false
            }),
        "{events:?}"
    );
    let sprite = world.sprite(id).expect("the sprite");
    assert!(close(sprite.felt(), 0.5), "{}", sprite.felt());
}
