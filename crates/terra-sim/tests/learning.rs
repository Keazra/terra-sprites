//! Learning (design §5.6): step 4 consumes reward and punishment, and
//! learns what things are worth and its habits.

use terra_sim::{
    DataPack, EntityId, Event, EventKind, Genome, Learned, Map, Pos, Scenario, ScriptedAction,
    Thing, World,
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
        .map(|m| (m.learned, m.amount))
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
    let sprite = world.sprites().next().expect("the sprite");
    assert!(
        close(sprite.felt(), 0.5),
        "it felt the relief: {}",
        sprite.felt()
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

/// The lessons in `events`, as `(learned, good)`.
fn lessons(events: &[Event]) -> Vec<(Learned, bool)> {
    events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::LearnedMilestone { learned, good, .. } => Some((learned.clone(), *good)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_sprite_that_keeps_biting_a_thornbush_learns_each_lesson_once() {
    // Design v16 §5.6: a lesson is a learned value's first time half a
    // point from nothing. The first prick makes the new thornbush −.8 bad,
    // and new things −.8 too; later pricks teach nothing new.
    let thornbush = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(thornbush, "thornbush")],
        at(1, 1),
        r#"Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),"#,
        &[
            ScriptedAction::Eat { at: thornbush },
            ScriptedAction::Eat { at: thornbush },
            ScriptedAction::Eat { at: thornbush },
            ScriptedAction::Rest,
        ],
    );
    let learned: Vec<(Learned, bool)> = (0..8).flat_map(|_| lessons(&world.step())).collect();
    let thorns_bad = Learned::Bad {
        thing: "thornbush".into(),
    };
    assert_eq!(
        learned,
        [(thorns_bad, false), (Learned::NewThings, false)],
        "each once"
    );
}

/// A world drawn from `rows` with sprites of `genes` on each of `sprites`,
/// each doing its part of `script`.
fn scene(rows: &[&str], sprites: &[(Pos, &str)], script: &[(Pos, ScriptedAction)]) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&(pos, genes)| (pos, Some(genome(genes, &data))))
        .collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: script,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// What sprite `id` has learned, as `(learned, value)`.
fn memory_of(world: &World, id: EntityId) -> Vec<(Learned, f32)> {
    let sprite = world.sprite(id).expect("the sprite");
    sprite
        .memory()
        .into_iter()
        .map(|m| (m.learned, m.amount))
        .collect()
}

/// A sprite whose every hit it feels punishes it by .5.
const HIT_HURTS: &str =
    r#"Emitter(locus: Locus("was_hit"), mode: Level, gain: 0.5, chem: "punishment"),"#;

#[test]
fn a_sprite_that_is_hit_fears_its_attacker_and_learns_nothing_bad_of_sprites() {
    // Design v18 §5.6: a hurt done to it teaches fear of whoever did it,
    // fear_rate (1) × the hit's punishment (.5), and no badness or worth.
    let (me, attacker) = (at(1, 1), at(2, 1));
    let mut world = scene(
        &["....", "....", "...."],
        &[(me, HIT_HURTS), (attacker, "")],
        &[
            (me, ScriptedAction::Rest),
            (attacker, ScriptedAction::Hit { at: me }),
            (attacker, ScriptedAction::Rest),
        ],
    );
    let (me, attacker) = (
        world.sprite_at(me).expect("me").id(),
        world.sprite_at(attacker).expect("the attacker").id(),
    );
    world.step();
    world.step();
    let fear = Learned::Fear {
        thing: Thing::Sprite(attacker),
    };
    assert_eq!(
        memory_of(&world, me),
        [(fear, -0.5)],
        "frightening, and nothing else"
    );
}

/// A bored sprite whose play with another sprite halves its boredom.
const BORED: &str = r#"InitialConcentration(chem: "boredom", value: 1.0),
    Emitter(locus: Locus("played_social"), mode: Level, gain: -0.5, chem: "boredom"),"#;

#[test]
fn playing_with_a_sprite_teaches_what_that_one_is_worth_not_sprites_in_general() {
    // Design v18 §5.6: boredom falls by .5, its relief, and the sprite
    // played with is the thing touched: .5 × individual_rate_good (.8).
    let (me, partner) = (at(1, 1), at(2, 1));
    let mut world = scene(
        &["....", "....", "...."],
        &[(me, BORED), (partner, "")],
        &[
            (me, ScriptedAction::Play { at: partner }),
            (me, ScriptedAction::Rest),
            (partner, ScriptedAction::Rest),
        ],
    );
    let (me, partner) = (
        world.sprite_at(me).expect("me").id(),
        world.sprite_at(partner).expect("the partner").id(),
    );
    world.step();
    world.step();
    let worth = |thing: Thing| Learned::Worth {
        thing,
        need: Some("boredom".into()),
    };
    let memory = memory_of(&world, me);
    let value = |learned: &Learned| memory.iter().find(|(l, _)| l == learned).map(|&(_, v)| v);
    assert!(
        value(&worth(Thing::Sprite(partner))).is_some_and(|v| close(v, 0.4)),
        "{memory:?}"
    );
    assert_eq!(
        value(&worth("sprite".into())),
        None,
        "nothing learned about sprites in general: {memory:?}"
    );
}

#[test]
fn sprites_in_general_are_feared_only_once_several_have_hurt_it() {
    // Design v18 §5.6: sprites in general are the mean over the sprites it
    // remembers, at no strength with one, half with two and in full from
    // generalise (3). Each bully's hit punishes by .5, so each is −.5.
    let me = at(2, 2);
    let bullies = [at(3, 2), at(1, 2), at(2, 1)];
    for (n, expected) in [(1, None), (2, Some(-0.25)), (3, Some(-0.5))] {
        let mut sprites = vec![(me, HIT_HURTS)];
        let mut script = vec![(me, ScriptedAction::Rest); 4];
        for (i, &bully) in bullies[..n].iter().enumerate() {
            sprites.push((bully, ""));
            // Ten ticks apart, since a tick has one attacker; then it rests
            // out the run, so it hits only once.
            script.extend(vec![(bully, ScriptedAction::Rest); i]);
            script.push((bully, ScriptedAction::Hit { at: me }));
            script.extend(vec![(bully, ScriptedAction::Rest); 3]);
        }
        let mut world = scene(&["....."; 5], &sprites, &script);
        let me = world.sprite_at(me).expect("me").id();
        for _ in 0..25 {
            world.step();
        }
        let memory = memory_of(&world, me);
        let in_general = Learned::Fear {
            thing: "sprite".into(),
        };
        let value = memory
            .iter()
            .find(|(l, _)| *l == in_general)
            .map(|&(_, v)| v);
        assert_eq!(
            value.map(|v| (v * 100.0).round() / 100.0),
            expected,
            "{n} bullies: {memory:?}"
        );
    }
}

/// Whether sprite `id` remembers anything about sprite `other`.
fn remembers(world: &World, id: EntityId, other: EntityId) -> bool {
    memory_of(world, id)
        .iter()
        .any(|(learned, _)| match learned {
            Learned::Worth { thing, .. } | Learned::Bad { thing } | Learned::Fear { thing } => {
                *thing == Thing::Sprite(other)
            }
            _ => false,
        })
}

#[test]
fn a_sprite_whose_fear_has_faded_to_almost_nothing_is_forgotten() {
    // Design v18 §5.6: fear of .5 fading by .01 a tick is under
    // forget_below (.01) after about 390 ticks, and then forgotten.
    let (me, attacker) = (at(1, 1), at(2, 1));
    let fading = format!(r#"{HIT_HURTS} BrainParam(param: "fear_fade", value: 0.01),"#);
    let mut script = vec![(me, ScriptedAction::Rest); 60];
    script.push((attacker, ScriptedAction::Hit { at: me }));
    script.extend(vec![(attacker, ScriptedAction::Rest); 60]);
    let mut world = scene(
        &["....", "....", "...."],
        &[(me, &fading), (attacker, "")],
        &script,
    );
    let (me, attacker) = (
        world.sprite_at(me).expect("me").id(),
        world.sprite_at(attacker).expect("the attacker").id(),
    );
    world.step();
    world.step();
    assert!(remembers(&world, me, attacker), "hit, and remembered");
    for _ in 0..500 {
        world.step();
    }
    assert!(
        !remembers(&world, me, attacker),
        "{:?}",
        memory_of(&world, me)
    );
}

#[test]
fn a_sprite_that_dies_is_forgotten() {
    // Design v18 §5.6. After its hit the attacker bites a thornbush until
    // the pricks kill it; fear doesn't fade in these genomes.
    let (me, attacker, thornbush) = (at(1, 1), at(2, 1), at(3, 1));
    let mut script = vec![(me, ScriptedAction::Rest); 50];
    script.push((attacker, ScriptedAction::Hit { at: me }));
    script.extend(vec![(attacker, ScriptedAction::Eat { at: thornbush }); 100]);
    let data = builtin();
    let sprites = [
        (me, Some(genome(HIT_HURTS, &data))),
        (attacker, Some(genome("", &data))),
    ];
    let scenario = Scenario {
        map: Map::from_ascii(&["....", "....", "...."], &data).expect("valid drawing"),
        objects: &[(thornbush, "thornbush")],
        sprites: &sprites,
        scripted: &script,
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    let (me, attacker) = (
        world.sprite_at(me).expect("me").id(),
        world.sprite_at(attacker).expect("the attacker").id(),
    );
    world.step();
    world.step();
    assert!(remembers(&world, me, attacker), "hit, and remembered");
    let mut ticks = 0;
    while world.sprite(attacker).is_some() {
        world.step();
        ticks += 1;
        assert!(ticks < 500, "the attacker should have died of its pricks");
    }
    world.step();
    assert!(
        !remembers(&world, me, attacker),
        "{:?}",
        memory_of(&world, me)
    );
}
