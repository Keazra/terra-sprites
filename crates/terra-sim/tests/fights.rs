//! Fights and flight (design §3.6, §5.8): a hit records its attacker, and
//! a hit sprite turns to it while it feels the hit; pain leads to backing
//! away, and some hits are hit back. Driven through hand-made worlds.

use terra_sim::{
    DataPack, EntityId, EventKind, Genome, Map, Outcome, Pos, Scenario, ScriptedAction, Target,
    Verb, World,
};

const STARTER: &str = include_str!("../../../data/genomes/starter.ron");

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

/// The starter genome, without spawn variation.
fn starter(data: &DataPack) -> Genome {
    Genome::from_ron(STARTER, data).expect("a valid genome")
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
    scene_with(rows, &[], sprites, scripted, 1)
}

/// `scene`, with `objects`, and the world's RNG seeded with `seed`.
fn scene_with(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[(Pos, fn(&DataPack) -> Genome)],
    scripted: &[(Pos, ScriptedAction)],
    seed: u64,
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&(pos, genome)| (pos, Some(genome(&data))))
        .collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted,
    };
    World::from_scenario(scenario, data, seed).expect("a valid scenario")
}

/// The verb and target of the first action sprite `id` starts in the next
/// `ticks` ticks, if it starts one.
fn next_choice(world: &mut World, id: EntityId, ticks: u32) -> Option<(Verb, Option<Target>)> {
    for _ in 0..ticks {
        let events = world.step();
        let started = events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ActionStarted { id: who, .. } if who == id));
        if started {
            let action = world.sprite(id)?.action()?;
            return Some((action.verb, action.target));
        }
    }
    None
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

#[test]
fn a_sprite_pricked_by_a_thornbush_backs_away_from_it() {
    // It tries to eat the thornbush beside it, and gets pricked.
    let (me, thornbush) = (at(2, 2), at(3, 2));
    let rows = [".......", ".......", ".......", ".......", "......."];
    let mut backed_away = 0;
    for seed in 0..10 {
        let script = [(me, ScriptedAction::Eat { at: thornbush })];
        let sprites = [(me, starter as fn(&DataPack) -> Genome)];
        let mut world = scene_with(&rows, &[(thornbush, "thornbush")], &sprites, &script, seed);
        let id = id_at(&world, me);
        let bush = world.object_at(thornbush).expect("the thornbush").id();
        world.step();
        if next_choice(&mut world, id, 5) == Some((Verb::Retreat, Some(Target::Object(bush)))) {
            backed_away += 1;
        }
    }
    assert!(backed_away >= 8, "backed away in {backed_away} of 10");
}

#[test]
fn a_hit_sprite_mostly_backs_away_from_its_attacker_and_sometimes_hits_back() {
    let (me, attacker) = (at(3, 3), at(4, 3));
    let rows = ["........"; 8];
    let (mut backed_away, mut hit_back) = (0, 0);
    for seed in 0..40 {
        // The attacker hits once, as the sprite it hits sets off on its
        // first action, and rests. That sprite is the starter genome with
        // spawn variation.
        let script = [
            (attacker, ScriptedAction::Hit { at: me }),
            (attacker, ScriptedAction::Rest),
        ];
        let data = builtin();
        let sprites = [(me, None), (attacker, Some(walker(&data)))];
        let scenario = Scenario {
            map: Map::from_ascii(&rows, &data).expect("valid drawing"),
            objects: &[],
            sprites: &sprites,
            scripted: &script,
        };
        let mut world = World::from_scenario(scenario, data, seed).expect("a valid scenario");
        let (id, them) = (id_at(&world, me), id_at(&world, attacker));
        let hit = (0..40).any(|_| {
            world.step().iter().any(|e| {
                matches!(e.kind, EventKind::ActionEnded {
                    id: who,
                    verb: Verb::Hit,
                    outcome: Outcome::Applied,
                    ..
                } if who == them)
            })
        });
        assert!(hit, "seed {seed}: the attacker hits");
        match next_choice(&mut world, id, 1) {
            Some((Verb::Retreat, Some(Target::Sprite(who)))) if who == them => backed_away += 1,
            Some((Verb::Hit, Some(Target::Sprite(who)))) if who == them => hit_back += 1,
            _ => {}
        }
    }
    assert!(
        backed_away > 2 * hit_back && hit_back >= 3,
        "backed away {backed_away}, hit back {hit_back}, of 40"
    );
}
