//! Fights and flight (design §3.6, §5.8): a hit records its attacker, and
//! a hit sprite turns to it while it feels the hit; pain leads to backing
//! away, and some hits are hit back. Driven through hand-made worlds.

use terra_sim::{
    DataPack, EntityId, EventKind, Genome, Map, Outcome, Pos, Scenario, ScriptedAction, Target,
    Verb, World,
};

const STARTER: &str = include_str!("../../../data/genomes/starter.ron");

/// Makes a sprite's genome from the data pack.
type MakeGenome = fn(&DataPack) -> Genome;

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
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `sprites` made as given, placed in the
/// order given, each starting on the scripted actions given for its tile.
fn scene(
    rows: &[&str],
    sprites: &[(Pos, MakeGenome)],
    scripted: &[(Pos, ScriptedAction)],
) -> World {
    scene_with(rows, &[], sprites, scripted, 1)
}

/// `scene`, with `objects`, and the world's RNG seeded with `seed`.
fn scene_with(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[(Pos, MakeGenome)],
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
    let sprites = [
        (bystander, walker as MakeGenome),
        (me, watcher),
        (attacker, walker),
    ];
    let mut world = scene(&["....."], &sprites, &script);
    let me = id_at(&world, me);
    world.step();
    assert_eq!(
        attends(&world, me),
        Some(bystander),
        "before the hit is felt"
    );
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
        let sprites = [(me, starter as MakeGenome)];
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
    // About one sprite in three hits back. The instincts alone make it one
    // in four, but learning punishes the wander the hit interrupted, so
    // hitting back clears the switch margin a little more often (design
    // v15 change 10).
    assert!(
        2 * backed_away >= 3 * hit_back && hit_back >= 3,
        "backed away {backed_away}, hit back {hit_back}, of 40"
    );
}

#[test]
fn a_crowded_sprite_mostly_backs_away_and_sometimes_hits_a_neighbour() {
    let me = at(3, 3);
    let neighbours = [at(2, 2), at(4, 2), at(2, 4), at(4, 4)];
    let (mut backed_away, mut hit) = (0, 0);
    for seed in 0..100 {
        let data = builtin();
        let text = STARTER.replace(
            "    ],
)",
            "        InitialConcentration(chem: \"crowdedness\", value: 0.6),
    ],
)",
        );
        let crowded = Genome::from_ron(&text, &data).expect("a valid genome");
        let mut sprites = vec![(me, Some(crowded))];
        sprites.extend(neighbours.map(|pos| (pos, Some(walker(&data)))));
        let script = neighbours.map(|pos| (pos, ScriptedAction::Rest));
        let scenario = Scenario {
            map: Map::from_ascii(&["........"; 8], &data).expect("valid drawing"),
            objects: &[],
            sprites: &sprites,
            scripted: &script,
        };
        let mut world = World::from_scenario(scenario, data, seed).expect("a valid scenario");
        let id = id_at(&world, me);
        match next_choice(&mut world, id, 1) {
            Some((Verb::Retreat, Some(Target::Sprite(_)))) => backed_away += 1,
            Some((Verb::Hit, Some(Target::Sprite(_)))) => hit += 1,
            _ => {}
        }
    }
    assert!(
        backed_away > 5 * hit && hit >= 3,
        "backed away {backed_away}, hit {hit}, of 100"
    );
}

#[test]
fn a_sprite_cornered_by_its_attacker_turns_on_it() {
    // At the end of a corridor, with the attacker in the only way out.
    let (me, attacker) = (at(0, 0), at(1, 0));
    let mut turned = 0;
    for seed in 0..10 {
        let script = [
            (attacker, ScriptedAction::Hit { at: me }),
            (attacker, ScriptedAction::Rest),
        ];
        let sprites = [(me, starter as MakeGenome), (attacker, walker)];
        let mut world = scene_with(&["......"], &[], &sprites, &script, seed);
        let (id, them) = (id_at(&world, me), id_at(&world, attacker));
        // What the sprite does next, once a retreat of its has been cornered.
        let mut cornered = false;
        let mut next = None;
        for _ in 0..20 {
            for event in world.step() {
                match event.kind {
                    EventKind::ActionEnded {
                        id: who,
                        verb: Verb::Retreat,
                        outcome: Outcome::Blocked,
                        ..
                    } if who == id => cornered = true,
                    EventKind::ActionStarted { id: who, verb } if who == id && cornered => {
                        let action = world.sprite(id).and_then(|s| s.action());
                        next = Some((verb, action.and_then(|a| a.target)));
                    }
                    _ => {}
                }
            }
            if next.is_some() {
                break;
            }
        }
        if next == Some((Verb::Hit, Some(Target::Sprite(them)))) {
            turned += 1;
        }
    }
    assert!(turned >= 8, "turned on the attacker in {turned} of 10");
}

/// A watcher that feels every hit as a punishment of 1.
fn fearful_watcher(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
        BrainParam(param: "tau_base", value: 0.05),
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
        Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

#[test]
fn a_feared_sprite_catches_the_eye_over_a_nearer_stranger() {
    // Design v18 §5.3. The bully hits and walks off to stand 3 tiles away;
    // the stranger stands beside the watcher. Nearness alone favours the
    // stranger (salience .5 against .4); fear of the bully, 1 × vigilance
    // (.8) × what's left of it at that distance (.6), adds .48.
    let (stranger, me, bully) = (at(0, 0), at(1, 0), at(2, 0));
    let script = [
        (stranger, ScriptedAction::Rest),
        (bully, ScriptedAction::Hit { at: me }),
        (
            bully,
            ScriptedAction::Wander {
                destination: at(4, 0),
            },
        ),
        (bully, ScriptedAction::Rest),
    ];
    let sprites = [
        (stranger, walker as MakeGenome),
        (me, fearful_watcher),
        (bully, walker),
    ];
    let mut world = scene(&["......."], &sprites, &script);
    let (me, bully) = (id_at(&world, me), id_at(&world, bully));
    for _ in 0..12 {
        world.step();
    }
    let bully_at = world.sprite(bully).expect("the bully").pos();
    assert_eq!(bully_at, at(4, 0), "the bully has walked off");
    assert_eq!(attends(&world, me), Some(bully_at));
}

/// A sprite that attends to sprites, would rather rest a little, and feels
/// every hit as a punishment of 1: so fear alone would make it back away.
fn skittish(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
        BrainParam(param: "tau_base", value: 0.05),
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 0.3),
        Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

#[test]
fn a_sprite_backs_away_from_one_it_fears_once_the_hit_is_no_longer_felt() {
    // Design v18 §5.5. The bully hits at tick 0 and stays beside it. While
    // the hit is felt, at tick 1, fear is quiet and the sprite keeps
    // resting; from tick 2, flight (.8) × its fear (1) beats resting's .3 by
    // more than the switch margin (.2), and it backs away from the bully.
    let (me, bully) = (at(1, 0), at(2, 0));
    let script = [
        (bully, ScriptedAction::Hit { at: me }),
        (bully, ScriptedAction::Rest),
    ];
    let sprites = [(me, skittish as MakeGenome), (bully, walker)];
    let mut world = scene(&["......"], &sprites, &script);
    let (me, bully) = (id_at(&world, me), id_at(&world, bully));
    world.step();
    world.step();
    let resting = world.sprite(me).and_then(|s| s.action()).map(|a| a.verb);
    assert_eq!(resting, Some(Verb::Rest), "quiet while the hit is felt");
    assert_eq!(
        next_choice(&mut world, me, 1),
        Some((Verb::Retreat, Some(Target::Sprite(bully))))
    );
}

#[test]
fn a_sprite_does_not_back_away_from_one_it_fears_that_is_far_off() {
    // Design v18 §5.5: fear pulls only within fear_reach (half of sight).
    // The bully hits, then walks off to stand 7 tiles away, out of reach of
    // fear's pull, and the sprite goes on resting.
    let (me, bully) = (at(1, 0), at(2, 0));
    let mut script = vec![
        (bully, ScriptedAction::Hit { at: me }),
        (
            bully,
            ScriptedAction::Wander {
                destination: at(8, 0),
            },
        ),
    ];
    // It stays there for the rest of the test.
    script.extend(vec![(bully, ScriptedAction::Rest); 5]);
    let sprites = [(me, skittish as MakeGenome), (bully, walker)];
    let mut world = scene(&[".........."], &sprites, &script);
    let me = id_at(&world, me);
    for _ in 0..12 {
        world.step();
    }
    assert_eq!(world.sprite(me).expect("me").pos(), at(1, 0), "still there");
    let choice = next_choice(&mut world, me, 20).map(|(verb, _)| verb);
    assert_eq!(choice, Some(Verb::Rest));
}

#[test]
fn a_starter_sprite_hit_by_a_bully_backs_away_from_it_far_more_than_from_a_stranger() {
    // Design v18 §5.5, the bully arena of slice 9c's prototype (#62): a
    // starter sprite fears the one that hit it, not sprites in general, so
    // it backs away from the bully far more than from a stranger. On v17's
    // brain the two were about even (4 and 3.5 in 400 ticks).
    let (me, bully, stranger) = (at(5, 3), at(6, 3), at(9, 5));
    let (mut from_bully, mut from_stranger) = (0, 0);
    for seed in 1..=5 {
        let script = [
            (me, ScriptedAction::Rest),
            (bully, ScriptedAction::Hit { at: me }),
        ];
        let sprites = [
            (me, starter as MakeGenome),
            (bully, starter),
            (stranger, starter),
        ];
        let mut world = scene_with(&["............"; 8], &[], &sprites, &script, seed);
        let (id, them, other) = (
            id_at(&world, me),
            id_at(&world, bully),
            id_at(&world, stranger),
        );
        for _ in 0..400 {
            for event in world.step() {
                if let EventKind::ActionEnded {
                    id: who,
                    verb: Verb::Retreat,
                    action,
                    ..
                } = event.kind
                    && who == id
                {
                    match action.target {
                        Some(Target::Sprite(t)) if t == them => from_bully += 1,
                        Some(Target::Sprite(t)) if t == other => from_stranger += 1,
                        _ => {}
                    }
                }
            }
        }
    }
    assert!(
        from_bully > 3 * from_stranger,
        "backed away from the bully {from_bully} times, from the stranger {from_stranger}"
    );
}

/// A skittish sprite in the middle of a 14×3 field, hit in turn by the
/// first `bullies` of three sprites around it, ten ticks apart, which then
/// walk off beyond fear's reach; a stranger rests beside it. What it
/// chooses first once its own rests are over.
fn hurt_by(bullies: usize) -> (Option<(Verb, Option<Target>)>, EntityId) {
    let me = at(6, 1);
    let spots = [at(5, 1), at(7, 1), at(6, 0)];
    let far = [at(0, 0), at(13, 0), at(13, 2)];
    let stranger = at(6, 2);
    let mut script = vec![(me, ScriptedAction::Rest); 6];
    script.extend(vec![(stranger, ScriptedAction::Rest); 12]);
    let mut sprites = vec![(me, skittish as MakeGenome), (stranger, walker)];
    for (i, (&bully, &away)) in spots.iter().zip(&far).enumerate().take(bullies) {
        script.extend(vec![(bully, ScriptedAction::Rest); i]);
        script.push((bully, ScriptedAction::Hit { at: me }));
        script.push((bully, ScriptedAction::Wander { destination: away }));
        script.extend(vec![(bully, ScriptedAction::Rest); 10]);
        sprites.push((bully, walker));
    }
    let mut world = scene(&[".............."; 3], &sprites, &script);
    let (me, stranger) = (id_at(&world, me), id_at(&world, stranger));
    for _ in 0..59 {
        world.step();
    }
    (next_choice(&mut world, me, 5), stranger)
}

#[test]
fn a_sprite_hurt_by_three_different_sprites_backs_away_from_a_stranger() {
    // Design v18 §5.6: three bullies at −1 each make sprites in general
    // −1, and a stranger is judged by them: flight (.8) beats resting (.3).
    // One bully leaves sprites in general alone, and it goes on resting.
    let (choice, stranger) = hurt_by(3);
    assert_eq!(
        choice,
        Some((Verb::Retreat, Some(Target::Sprite(stranger))))
    );
    let (choice, _) = hurt_by(1);
    assert_eq!(choice.map(|(verb, _)| verb), Some(Verb::Rest));
}
