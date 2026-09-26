//! Fights (design §3.6, §5.8): a hit records its attacker, and a hit sprite
//! turns to it while it feels the hit. Driven through hand-made worlds.

use terra_sim::{DataPack, EntityId, Genome, Map, Pos, Scenario, ScriptedAction, World};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A genome with only traits, so nothing but a script decides what it does.
fn walker(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A sprite that always rests, and always attends to sprites: so what it
/// attends to is always its Sprite candidate.
fn watcher(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
        BrainParam(param: "tau_base", value: 0.05),
        AttentionInstinct(input: "always", category: Sprite, weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `sprites` made as given, placed in the
/// order given, each starting on the scripted actions given for its tile.
fn scene(
    rows: &[&str],
    sprites: &[(Pos, fn(&DataPack) -> Genome)],
    scripted: &[(Pos, ScriptedAction)],
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&(pos, genome)| (pos, Some(genome(&data))))
        .collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// The ID of the sprite on `pos`.
fn id_at(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
}

/// Where the sprite `id` attends to.
fn attends(world: &World, id: EntityId) -> Option<Pos> {
    world.sprite(id).expect("the sprite").attending_to()
}

#[test]
fn a_hit_sprite_turns_to_its_attacker_while_it_feels_the_hit() {
    // Both others stand beside the watcher; the bystander, placed first, has
    // the lower ID, so it's the nearest sprite until the attacker hits.
    let (bystander, me, attacker) = (at(0, 0), at(1, 0), at(2, 0));
    // The attacker hits once, then rests.
    let script = [
        (bystander, ScriptedAction::Rest),
        (attacker, ScriptedAction::Hit { at: me }),
        (attacker, ScriptedAction::Rest),
    ];
    let sprites = [(bystander, walker as fn(&DataPack) -> Genome), (me, watcher), (attacker, walker)];
    let mut world = scene(&["....."], &sprites, &script);
    let me = id_at(&world, me);
    world.step();
    assert_eq!(attends(&world, me), Some(bystander), "before the hit is felt");
    world.step();
    assert_eq!(attends(&world, me), Some(attacker), "feeling the hit");
    world.step();
    assert_eq!(attends(&world, me), Some(bystander), "the hit has passed");
}
