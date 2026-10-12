//! Breeding (M2 design §5, §10): mating happens only between adults,
//! enforced by the engine for any genome and data pack; a male and a female
//! lay an egg, or a female alone; it hatches into a young male; a lone adult
//! turns female; and births are world state, saved, hashed and replayed.

mod common;
use common::{WorldBuilder, at, builtin, builtin_changing_files, close, endings, genome};

use proptest::prelude::*;
use terra_sim::{
    Command, DataPack, Dir, EntityId, Event, EventKind, LifeStage, Outcome, Part, Playback, Pos,
    Rejection, Sex, Target, Thing, Verb, World,
};

/// A small field of grass with a sprite of `genes` on each tile of
/// `sprites`, those on `young` as newborns and the rest just grown up.
fn field(genes: &str, sprites: &[Pos], young: &[Pos], seed: u64) -> World {
    field_in(builtin(), genes, sprites, young, seed)
}

/// `field`, with the data pack `data`.
fn field_in(data: DataPack, genes: &str, sprites: &[Pos], young: &[Pos], seed: u64) -> World {
    let genome = genome(genes, &data);
    let placed = sprites
        .iter()
        .map(|&pos| (pos, Some(genome.clone())))
        .collect();
    WorldBuilder::new(&["........", "........", "........", "........"])
        .data(data)
        .sprites_with(placed)
        .newborns(young)
        .seed(seed)
        .build()
}

/// An instinct to mate with whatever sprite it sees, and nothing else.
const KEEN: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
"#;

/// A Mate action as it started: the life stage of the sprite mating, and
/// of the sprite it's aimed at, or `None` if it's aimed at no sprite.
type Mating = (LifeStage, Option<LifeStage>);

/// Steps `world` `ticks` times, returning each Mate action started. It
/// checks too that only a grown sprite starts to breed alone, and that
/// every egg is laid by a grown female, with a grown male if with anyone
/// (M2 design §10).
fn matings(world: &mut World, ticks: u64) -> Vec<Mating> {
    let mut found = Vec::new();
    for _ in 0..ticks {
        let events = world.step();
        for event in &events {
            if let EventKind::Laid { by, with, .. } = event.kind {
                let mother = world.sprite(by).expect("the mother");
                assert!(mother.life_stage().is_grown() && mother.sex() == Sex::Female);
                if let Some(father) = with.and_then(|id| world.sprite(id)) {
                    assert!(father.life_stage().is_grown() && father.sex() == Sex::Male);
                }
            }
            let EventKind::ActionStarted { id, verb } = event.kind else {
                continue;
            };
            // One that died in the same tick: the self-check saw its action.
            let Some(sprite) = world.sprite(id) else {
                continue;
            };
            match verb {
                Verb::BreedAlone => assert!(sprite.life_stage().is_grown()),
                Verb::Mate => {
                    let partner = match sprite.action().and_then(|a| a.target) {
                        Some(Target::Sprite(other)) => match world.sprite(other) {
                            Some(other) => Some(other.life_stage()),
                            None => continue,
                        },
                        _ => None,
                    };
                    found.push((sprite.life_stage(), partner));
                }
                _ => {}
            }
        }
    }
    found
}

#[test]
fn an_adult_keen_to_mate_mates_with_an_adult() {
    let mut world = field(KEEN, &[at(2, 1), at(3, 1)], &[], 1);
    let found = matings(&mut world, 20);
    assert!(!found.is_empty(), "no adult mated");
    assert!(
        found
            .iter()
            .all(|&(stage, partner)| stage == LifeStage::Adult && partner == Some(LifeStage::Adult)),
        "{found:?}"
    );
}

#[test]
fn an_adult_never_mates_with_a_young_sprite_nor_a_young_one_at_all() {
    // M2 design §5: Mate isn't offered unless both are adults, so the
    // instinct has nothing to choose.
    let mut world = field(KEEN, &[at(2, 1), at(3, 1)], &[at(3, 1)], 1);
    assert_eq!(matings(&mut world, 200), []);
}

/// A body that turns female at once, as an adult (M2 design §10).
const FEMALE: &str = r#"InitialConcentration(chem: "feminiser", value: 1.0),"#;

/// An instinct to breed alone whenever it can, and the longing it takes.
const ALONE: &str = r#"
    Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
    InitialConcentration(chem: "longing", value: 1.0),
"#;

/// An instinct to rest, so the sprites watched don't come to blows.
const REST: &str = r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#;

/// The starter genome's genes for turning female (M2 design §10): the
/// feminiser rises while no female adult is near, and fades. And it rests.
const TURNING: &str = r#"
    Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    Emitter(locus: Locus("female_near"), mode: Level, invert: true, threshold: 0.5, gain: 0.003, chem: "feminiser", from: Adult),
    HalfLife(chem: "feminiser", ticks: 600),
"#;

/// A small field of grass with a sprite of its own genes on each tile, the
/// ones on `young` as newborns and the rest just grown up.
fn family(sprites: &[(Pos, &str)], young: &[Pos], seed: u64) -> World {
    family_in(builtin(), sprites, young, seed)
}

/// The same, with `data`.
fn family_in(data: DataPack, sprites: &[(Pos, &str)], young: &[Pos], seed: u64) -> World {
    let placed = sprites
        .iter()
        .map(|&(pos, genes)| (pos, Some(genome(genes, &data))))
        .collect();
    WorldBuilder::new(&["........", "........", "........", "........"])
        .data(data)
        .sprites_with(placed)
        .newborns(young)
        .seed(seed)
        .build()
}

/// Steps `world` until `found` picks something out of a tick's events, or
/// `ticks` ticks have passed.
fn until<T>(
    world: &mut World,
    ticks: u64,
    mut found: impl FnMut(&Event) -> Option<T>,
) -> Option<T> {
    for _ in 0..ticks {
        if let Some(found) = world.step().iter().find_map(&mut found) {
            return Some(found);
        }
    }
    None
}

/// An egg laid: the egg, who laid it, with whom, and on which tick.
struct Laying {
    egg: EntityId,
    by: EntityId,
    with: Option<EntityId>,
    tick: u64,
}

/// Steps `world` until an egg is laid.
fn laid(world: &mut World, ticks: u64) -> Option<Laying> {
    until(world, ticks, |event| match event.kind {
        EventKind::Laid { egg, by, with } => Some(Laying {
            egg,
            by,
            with,
            tick: event.tick,
        }),
        _ => None,
    })
}

/// A sprite's energy.
fn energy(world: &World, id: EntityId) -> f32 {
    let sprite = world.sprite(id).expect("alive");
    sprite.chemical("energy").expect("energy")
}

#[test]
fn a_male_and_a_female_lay_an_egg_beside_her_and_split_its_cost() {
    let female = format!("{KEEN}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female), (at(3, 1), KEEN)], &[], 1);
    let (mother, father) = (
        world.sprite_at(at(2, 1)).unwrap().id(),
        world.sprite_at(at(3, 1)).unwrap().id(),
    );
    let mut before = (energy(&world, mother), energy(&world, father));
    let mut found = None;
    for _ in 0..100 {
        let events = world.step();
        found = events.iter().find_map(|event| match event.kind {
            EventKind::Laid { egg, by, with } => Some((egg, by, with)),
            _ => None,
        });
        if found.is_some() {
            break;
        }
        before = (energy(&world, mother), energy(&world, father));
    }
    let (egg, by, with) = found.expect("an egg was laid");
    assert_eq!((by, with), (mother, Some(father)));
    let egg = world.sprite(egg).expect("the egg");
    assert!(egg.is_egg());
    assert_eq!(egg.life_stage(), LifeStage::Egg);
    assert_eq!(egg.parents(), [mother, father]);
    let her = world.sprite(mother).unwrap().pos();
    let (dx, dy) = (egg.pos().x.abs_diff(her.x), egg.pos().y.abs_diff(her.y));
    assert_eq!(dx.max(dy), 1, "the egg is beside her");
    // An egg costs 0.3 of a full belly, half each (M2 design §10), give or
    // take a tick's metabolism.
    let paid = (
        before.0 - energy(&world, mother),
        before.1 - energy(&world, father),
    );
    assert!((paid.0 - 0.15).abs() < 0.01, "she paid {}", paid.0);
    assert!((paid.1 - 0.15).abs() < 0.01, "he paid {}", paid.1);
}

#[test]
fn two_males_or_two_females_lay_nothing_together() {
    // Two females may still each breed alone (M2 design §10).
    let female = format!("{KEEN}{FEMALE}");
    for genes in [KEEN, female.as_str()] {
        let mut world = family(&[(at(2, 1), genes), (at(3, 1), genes)], &[], 1);
        let mut tries = Vec::new();
        for _ in 0..300 {
            let events = world.step();
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event.kind, EventKind::Laid { with: Some(_), .. })),
                "a pair of one sex laid an egg"
            );
            tries.extend(endings(&events));
        }
        assert!(
            tries.contains(&(Verb::Mate, Outcome::Failed)),
            "they tried, fruitlessly: {tries:?}"
        );
    }
}

#[test]
fn a_female_breeds_alone_and_pays_it_all_but_a_male_cannot() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female), (at(5, 2), ALONE)], &[], 1);
    let (mother, male) = (
        world.sprite_at(at(2, 1)).unwrap().id(),
        world.sprite_at(at(5, 2)).unwrap().id(),
    );
    let Laying { egg, by, with, .. } = laid(&mut world, 50).expect("she laid an egg");
    assert_eq!((by, with), (mother, None));
    let egg = world.sprite(egg).unwrap();
    assert_eq!(egg.parents(), [mother]);
    assert_eq!(egg.genome(), world.sprite(mother).unwrap().genome());
    assert!(energy(&world, mother) < 0.72, "she paid the whole 0.3");
    // The male tries, and keeps failing.
    let mut tries = Vec::new();
    for _ in 0..100 {
        tries.extend(world.step().iter().filter_map(|event| match event.kind {
            EventKind::ActionEnded {
                id,
                verb: Verb::BreedAlone,
                outcome,
                ..
            } if id == male => Some(outcome),
            _ => None,
        }));
    }
    assert!(
        !tries.is_empty() && tries.iter().all(|&o| o == Outcome::Failed),
        "{tries:?}"
    );
    assert_eq!(world.sprite(male).unwrap().sex(), Sex::Male);
}

#[test]
fn an_egg_needs_nothing_and_hatches_after_750_ticks_as_a_young_male() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female)], &[], 1);
    let Laying {
        egg,
        by: mother,
        tick: laid_on,
        ..
    } = laid(&mut world, 50).expect("an egg");
    let view = world.sprite(egg).unwrap();
    assert_eq!(view.hatches(), Some(laid_on + 750));
    let pos = view.pos();
    let hatched = until(&mut world, 800, |event| match &event.kind {
        EventKind::Hatched { id, parents } if *id == egg => Some((event.tick, parents.clone())),
        _ => None,
    });
    let (tick, parents) = hatched.expect("it hatched");
    assert_eq!(tick, laid_on + 750);
    assert_eq!(parents, [mother]);
    let young = world.sprite(egg).expect("the hatchling");
    assert!(!young.is_egg());
    assert_eq!(young.life_stage(), LifeStage::Young);
    assert_eq!(young.sex(), Sex::Male);
    assert_eq!(world.tick() - young.age(), tick, "born as it hatched");
    assert_eq!(young.pos(), pos, "it hatched where it lay");
}

#[test]
fn an_egg_body_stays_as_it_was_laid() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let body = |world: &World| -> Vec<f32> {
        let egg = world.sprite(egg).unwrap();
        egg.chemicals().map(|c| c.level).collect()
    };
    let laid_with = body(&world);
    for _ in 0..700 {
        world.step();
        let egg_now = world.sprite(egg).unwrap();
        assert!(egg_now.is_egg());
        assert!(egg_now.action().is_none(), "an egg does nothing");
        assert_eq!(body(&world), laid_with, "an egg needs nothing");
    }
}

/// The commands a tick's `events` refused, and why.
fn refusals(events: Vec<Event>) -> Vec<(Command, Rejection)> {
    events
        .into_iter()
        .filter_map(|event| match event.kind {
            EventKind::CommandRejected { command, reason } => Some((command, reason)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_cursor_cant_touch_an_egg() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let commands = [
        Command::Reward {
            sprite: egg,
            amplified: false,
            reach_back: 10,
        },
        Command::Correct {
            sprite: egg,
            amplified: true,
        },
    ];
    for command in &commands {
        world.submit(command.clone());
    }
    let expected: Vec<_> = commands
        .iter()
        .map(|c| (c.clone(), Rejection::Egg))
        .collect();
    assert_eq!(refusals(world.step()), expected);
}

#[test]
fn the_cursor_leads_an_egg_where_it_goes_but_cant_shove_it() {
    // M2 design §10.4 (#190): the Cursor takes hold of an egg as it does a
    // sprite, and the egg goes where the Cursor goes, needing and doing
    // nothing on the way; but it's never shoved.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let body = |world: &World| -> Vec<f32> {
        let egg = world.sprite(egg).unwrap();
        egg.chemicals().map(|c| c.level).collect()
    };
    let laid_with = body(&world);
    world.submit(Command::TakeHold { sprite: egg });
    world.submit(Command::MoveCursor { tile: at(7, 3) });
    let mut took_hold = false;
    for _ in 0..100 {
        let events = world.step();
        took_hold |= events
            .iter()
            .any(|e| e.kind == EventKind::TookHold { sprite: egg });
        let carried = world.sprite(egg).unwrap();
        assert!(carried.is_egg());
        assert!(carried.action().is_none(), "an egg does nothing");
        assert_eq!(body(&world), laid_with, "an egg needs nothing");
    }
    assert!(took_hold);
    assert_eq!(world.sprite(egg).unwrap().pos(), at(7, 3));
    let shove = Command::Shove {
        toward: Dir::W,
        tiles: 3,
    };
    world.submit(shove.clone());
    assert_eq!(refusals(world.step()), [(shove, Rejection::Egg)]);
    assert_eq!(world.cursor().leads(), Some(egg), "still led");
    world.submit(Command::LetGo);
    for _ in 0..50 {
        world.step();
    }
    assert_eq!(world.cursor().leads(), None);
    assert_eq!(world.sprite(egg).unwrap().pos(), at(7, 3), "it lies still");
}

#[test]
fn an_egg_that_hatches_while_led_is_led_still() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    world.submit(Command::TakeHold { sprite: egg });
    world.submit(Command::MoveCursor { tile: at(7, 0) });
    let hatched = until(&mut world, 800, |event| match event.kind {
        EventKind::Hatched { id, .. } if id == egg => Some(()),
        _ => None,
    });
    assert!(hatched.is_some(), "it hatched");
    world.submit(Command::MoveCursor { tile: at(7, 3) });
    for _ in 0..100 {
        world.step();
    }
    assert_eq!(world.cursor().leads(), Some(egg));
    let young = world.sprite(egg).unwrap();
    assert_eq!(young.pos(), at(7, 3), "the hatchling follows the Cursor");
    assert!(young.action().is_none(), "and chooses nothing while led");
}

/// Hits whatever egg it sees, and nothing else.
const EGG_HITTER: &str = r#"
    AttentionInstinct(input: "always", category: "egg", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_egg", false)], verb: Hit, weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_egg", false), ("target_adjacent", true)], verb: Approach, weight: 2.0),
"#;

#[test]
fn hitting_an_egg_does_nothing() {
    // M2 design §10: an egg can't be hurt until something in the world is
    // meant to hurt eggs, so it's a fruitless try.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female), (at(6, 3), EGG_HITTER)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let body = |world: &World| -> Vec<f32> {
        let egg = world.sprite(egg).unwrap();
        egg.chemicals().map(|c| c.level).collect()
    };
    let laid_with = body(&world);
    let mut hits = Vec::new();
    for _ in 0..300 {
        hits.extend(
            endings(&world.step())
                .into_iter()
                .filter(|&(verb, _)| verb == Verb::Hit),
        );
    }
    assert!(!hits.is_empty(), "it hit the egg");
    assert!(hits.iter().all(|&(_, o)| o == Outcome::Failed), "{hits:?}");
    assert_eq!(body(&world), laid_with);
}

#[test]
fn a_lone_adult_turns_female_but_a_young_one_never_does() {
    // M2 design §10: no young sprite changes, whatever its hormones. This
    // one's feminiser stays full all along.
    let young = format!("{FEMALE}{REST}");
    let mut world = family(&[(at(1, 1), TURNING), (at(6, 2), &young)], &[at(6, 2)], 1);
    let (adult, child) = (
        world.sprite_at(at(1, 1)).unwrap().id(),
        world.sprite_at(at(6, 2)).unwrap().id(),
    );
    let turned = until(&mut world, 3_000, |event| match event.kind {
        EventKind::TurnedFemale { id } => Some(id),
        _ => None,
    });
    assert_eq!(turned, Some(adult));
    assert_eq!(world.sprite(adult).unwrap().sex(), Sex::Female);
    let child = world.sprite(child).unwrap();
    assert!(!child.life_stage().is_grown());
    assert_eq!(child.sex(), Sex::Male);
    assert!(close(child.chemical("feminiser").unwrap(), 1.0));
}

#[test]
fn a_male_that_sees_a_female_stays_male() {
    let female = format!("{FEMALE}{TURNING}");
    let mut world = family(&[(at(1, 1), &female), (at(6, 2), TURNING)], &[], 1);
    let male = world.sprite_at(at(6, 2)).unwrap().id();
    let turned: Vec<_> = (0..3_000)
        .flat_map(|_| world.step())
        .filter_map(|event| match event.kind {
            EventKind::TurnedFemale { id } => Some(id),
            _ => None,
        })
        .collect();
    assert!(!turned.contains(&male), "{turned:?}");
    assert_eq!(world.sprite(male).unwrap().sex(), Sex::Male);
}

/// Longing enough to breed alone, from the start.
const LONGS: &str = r#"InitialConcentration(chem: "longing", value: 1.0),"#;

/// The starter genome's genes for longing (M2 design §10): it builds while
/// no male adult is in sight, and falls fast while one is.
const LONGING: &str = r#"
    Emitter(locus: Locus("male_near"), mode: Level, invert: true, threshold: 0.5, gain: 0.001, chem: "longing", from: Adult),
    Emitter(locus: Locus("male_near"), mode: Level, gain: -0.01, chem: "longing"),
    HalfLife(chem: "longing", ticks: 3000),
"#;

/// The starter genome's way to breeding alone: arousal while fertile, and
/// longing, lead a female to it. And a weak habit of resting.
const LONGS_TO_BREED: &str = r#"
    Instinct(inputs: [("always", false)], verb: Rest, weight: 0.3),
    Emitter(locus: Locus("fertile"), mode: Level, threshold: 0.5, gain: 0.01, chem: "arousal"),
    Instinct(inputs: [("arousal", false), ("female", false), ("longing", false)], verb: BreedAlone, weight: 1.0),
"#;

#[test]
fn a_female_longs_for_a_male_only_while_she_sees_no_male_adult() {
    // M2 design §10. A young male, which can't mate, doesn't count.
    let female = format!("{FEMALE}{LONGING}{REST}");
    let longing = |young: &[Pos]| {
        let mut world = family(&[(at(1, 1), &female), (at(6, 2), REST)], young, 1);
        let her = world.sprite_at(at(1, 1)).unwrap().id();
        for _ in 0..2_000 {
            world.step();
        }
        let her = world.sprite(her).unwrap();
        assert_eq!(her.sex(), Sex::Female);
        her.chemical("longing").unwrap()
    };
    assert!(longing(&[at(6, 2)]) > 0.75, "{}", longing(&[at(6, 2)]));
    assert!(close(longing(&[]), 0.0), "{}", longing(&[]));
}

#[test]
fn the_starter_s_female_breeds_alone_only_after_long_without_a_male() {
    // M2 design §10: breeding alone is a lonely female's fallback. With an
    // adult male in sight she lays nothing alone; out of sight of one, she
    // does once her longing has built.
    let female = format!("{FEMALE}{LONGING}{LONGS_TO_BREED}");
    let laid_alone = |young: &[Pos]| {
        let mut world = family(&[(at(1, 1), &female), (at(6, 2), REST)], young, 1);
        let ticks: Vec<u64> = (0..3_000)
            .flat_map(|_| world.step())
            .filter_map(|event| match event.kind {
                EventKind::Laid { with: None, .. } => Some(event.tick),
                _ => None,
            })
            .collect();
        ticks
    };
    let watched = laid_alone(&[]);
    assert!(watched.is_empty(), "{watched:?}");
    let alone = laid_alone(&[at(6, 2)]);
    assert!(alone.first().is_some_and(|&tick| tick > 500), "{alone:?}");
}

#[test]
fn an_egg_saved_and_loaded_hatches_the_same() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female), (at(5, 2), KEEN)], &[], 4);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    world.step();
    let mut loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    let loaded_egg = loaded.sprite(egg).expect("the egg was saved");
    assert!(loaded_egg.is_egg());
    assert_eq!(loaded_egg.hatches(), world.sprite(egg).unwrap().hatches());
    for _ in 0..800 {
        assert_eq!(world.step(), loaded.step());
        assert_eq!(loaded.state_hash(), world.state_hash());
    }
    assert!(!loaded.sprite(egg).unwrap().is_egg(), "it hatched");
}

#[test]
fn a_birth_hashes_differently() {
    // Births are world state (M2 design §10): two worlds alike but for an
    // egg aren't the same world.
    let female = format!("{ALONE}{FEMALE}");
    let mut laying = family(&[(at(2, 1), &female)], &[], 1);
    let mut not = family(&[(at(2, 1), FEMALE)], &[], 1);
    laid(&mut laying, 50).expect("an egg");
    while not.tick() < laying.tick() {
        not.step();
    }
    assert_ne!(laying.state_hash(), not.state_hash());
}

#[test]
fn births_replay_exactly() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female), (at(5, 2), KEEN)], &[], 2);
    world.start_recording();
    let mut births = 0;
    for _ in 0..2_000 {
        births += world
            .step()
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Laid { .. } | EventKind::Hatched { .. }))
            .count();
    }
    assert!(births >= 2, "an egg was laid and hatched");
    let mut playback = Playback::new(&world.replay().unwrap()).expect("the replay plays");
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
}

/// Mates with the sprite it attends to while both are aroused and of the
/// other sex, as the starter genome does (M2 design §10), and arousal comes
/// fast with fertility. Otherwise it rests, and it hardly ever tries
/// anything at random.
const BODY_LANGUAGE: &str = r#"
    Emitter(locus: Locus("fertile"), mode: Level, threshold: 0.5, gain: 0.1, chem: "arousal"),
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("arousal", false), ("target_arousal", false), ("target_other_sex", false)], verb: Mate, weight: 2.0),
    Instinct(inputs: [("always", false)], verb: Rest, weight: 0.5),
    BrainParam(param: "tau_base", value: 0.05),
"#;

#[test]
fn a_sprite_reads_whether_the_one_it_watches_is_aroused_and_of_the_other_sex() {
    // M2 design §10: body language. A male and a female mate, while two
    // males never try.
    let female = format!("{BODY_LANGUAGE}{FEMALE}");
    let mut pair = family(&[(at(2, 1), &female), (at(3, 1), BODY_LANGUAGE)], &[], 1);
    let laying = laid(&mut pair, 200).expect("the pair laid an egg");
    assert!(laying.with.is_some(), "with a partner");

    let mut males = family(
        &[(at(2, 1), BODY_LANGUAGE), (at(3, 1), BODY_LANGUAGE)],
        &[],
        1,
    );
    for _ in 0..200 {
        let tried = males.step().iter().any(|event| {
            matches!(
                event.kind,
                EventKind::ActionStarted {
                    verb: Verb::Mate,
                    ..
                }
            )
        });
        assert!(!tried, "a male tried to mate with a male");
    }
}

#[test]
fn a_male_leaves_a_female_alone_while_she_shows_no_arousal() {
    // Without the emitter her arousal stays at 0, fertile as she is, so
    // the aroused male watching her never tries.
    let calm = BODY_LANGUAGE.replace(
        r#"Emitter(locus: Locus("fertile"), mode: Level, threshold: 0.5, gain: 0.1, chem: "arousal"),"#,
        "",
    );
    let female = format!("{calm}{FEMALE}");
    let mut world = family(&[(at(2, 1), &female), (at(3, 1), BODY_LANGUAGE)], &[], 1);
    let ended = (0..200)
        .flat_map(|_| endings(&world.step()))
        .collect::<Vec<_>>();
    assert!(
        !ended.iter().any(|&(verb, _)| verb == Verb::Mate),
        "{ended:?}"
    );
}

/// Goes after the sprite it watches while it's aroused, and it's aroused
/// from the start. Otherwise it wanders, and it hardly ever tries anything
/// at random.
const AROUSED: &str = r#"
    InitialConcentration(chem: "arousal", value: 1.0),
    AttentionInstinct(input: "arousal", category: "sprite", weight: 1.0),
    Instinct(inputs: [("arousal", false)], verb: Approach, weight: 2.0),
    Instinct(inputs: [("always", false)], verb: Wander, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.05),
"#;

/// Goes after the sprite it watches while that one shows arousal. Otherwise
/// it wanders, and it hardly ever tries anything at random.
const WATCHING: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("target_arousal", false)], verb: Approach, weight: 2.0),
    Instinct(inputs: [("always", false)], verb: Wander, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.05),
"#;

/// How many actions, in 200 ticks, the sprite on the left starts aimed at
/// the sprite on the right, and the one on the right at the one on the
/// left, each of its own genes, the one on the right young if `young`.
fn goes_after(left: &str, right: &str, young: bool) -> (usize, usize) {
    goes_after_in(builtin(), left, right, young)
}

/// The same, with `data`.
fn goes_after_in(data: DataPack, left: &str, right: &str, young: bool) -> (usize, usize) {
    let (a, b) = (at(1, 1), at(6, 2));
    let newborns: &[Pos] = if young { &[b] } else { &[] };
    let mut world = family_in(data, &[(a, left), (b, right)], newborns, 1);
    let ids = [a, b].map(|pos| world.sprite_at(pos).expect("a sprite").id());
    let mut counts = (0, 0);
    for _ in 0..200 {
        for event in world.step() {
            let EventKind::ActionStarted { id, .. } = event.kind else {
                continue;
            };
            let target = world
                .sprite(id)
                .and_then(|sprite| sprite.action())
                .and_then(|action| action.target);
            if (id, target) == (ids[0], Some(Target::Sprite(ids[1]))) {
                counts.0 += 1;
            } else if (id, target) == (ids[1], Some(Target::Sprite(ids[0]))) {
                counts.1 += 1;
            }
        }
    }
    counts
}

#[test]
fn an_aroused_adult_goes_after_an_adult_but_never_a_young_sprite() {
    // M2 design §5: the breeding drive works only between adults, whatever
    // the genes, so it reads 0 in a choice about a young sprite.
    let (to_adult, _) = goes_after(AROUSED, AROUSED, false);
    assert!(to_adult > 0, "it went after the adult");
    let (to_young, _) = goes_after(AROUSED, AROUSED, true);
    assert_eq!(to_young, 0, "it went after the young sprite");
}

#[test]
fn a_young_sprite_s_arousal_moves_it_to_nothing() {
    // M2 design §5: the drive reads 0 in a sprite not grown up.
    let (_, by_adult) = goes_after(AROUSED, AROUSED, false);
    assert!(by_adult > 0, "the adult went after the other");
    let (_, by_young) = goes_after(AROUSED, AROUSED, true);
    assert_eq!(by_young, 0, "the young sprite went after the adult");
}

/// A pack with no breeding: it has no sense of a male in sight, and with
/// it go the starter's genes that read one. Its drive is still a need.
fn without_breeding() -> DataPack {
    let male_near = r#"(id: 9,  name: "male_near",         kind: BodySensor),"#;
    let longing = |line: &str| {
        let starter = include_str!("../../../data/genomes/starter.ron");
        let found = starter.lines().find(|l| l.contains(line));
        found.expect("the starter's longing gene").to_string()
    };
    let (raise, drop) = (
        longing("invert: true, threshold: 0.5, gain: 0.001"),
        longing("gain: -0.01, chem: \"longing\""),
    );
    builtin_changing_files(&[
        ("loci.ron", &[(male_near, "")]),
        ("genomes/starter.ron", &[(&raise, ""), (&drop, "")]),
    ])
}

#[test]
fn the_drive_works_only_between_adults_in_a_pack_that_doesnt_breed_too() {
    // M2 design §5, whether or not the pack breeds.
    let world = family_in(without_breeding(), &[(at(1, 1), AROUSED)], &[], 1);
    let male = world.sprites().next().expect("a sprite");
    assert_eq!(male.turning(), None, "the pack has no breeding");
    let (to_adult, by_adult) = goes_after_in(without_breeding(), AROUSED, AROUSED, false);
    assert!(to_adult > 0 && by_adult > 0, "{to_adult} and {by_adult}");
    let (to_young, by_young) = goes_after_in(without_breeding(), AROUSED, AROUSED, true);
    assert_eq!((to_young, by_young), (0, 0));
}

#[test]
fn a_sprite_in_a_pack_without_breeding_is_never_fertile() {
    // M2 design §10: a save from before breeding keeps its pack, which has
    // none, so the Body tab mustn't call its sprites fertile.
    let fertile = |data| {
        let world = family_in(data, &[(at(1, 1), REST)], &[], 1);
        world.sprites().next().expect("a sprite").is_fertile()
    };
    assert!(fertile(builtin()), "a fed adult in a pack that breeds");
    assert!(!fertile(without_breeding()), "an adult in a pack without");
}

#[test]
fn a_young_sprite_shows_no_arousal() {
    // M2 design §5, §10: body language reads the drive's level in an
    // adult, and 0 in a young sprite, whatever its genes do with it.
    let shows = r#"InitialConcentration(chem: "arousal", value: 1.0),"#;
    let (to_adult, _) = goes_after(WATCHING, shows, false);
    assert!(to_adult > 0, "it went after the aroused adult");
    let (to_young, _) = goes_after(WATCHING, shows, true);
    assert_eq!(to_young, 0, "it went after the aroused young sprite");
}

#[test]
fn a_young_sprite_s_arousal_never_shows_on_screen() {
    // M2 design §5: whatever its genes, a sprite not grown up shows no
    // arousal, to the player as to other sprites. The front end reads its
    // chemicals through the view, which reads the drive as 0.
    let shows = r#"InitialConcentration(chem: "arousal", value: 1.0),"#;
    let world = family(&[(at(1, 1), shows), (at(3, 1), shows)], &[at(3, 1)], 1);
    for sprite in world.sprites() {
        let listed = sprite.chemicals().find(|c| c.name == "arousal");
        let listed = listed.map(|c| (c.level, c.change));
        let expected = if sprite.life_stage().is_grown() {
            1.0
        } else {
            0.0
        };
        assert_eq!(
            (sprite.chemical("arousal"), listed),
            (Some(expected), Some((expected, 0.0))),
            "a {:?} sprite",
            sprite.life_stage()
        );
    }
}

/// A state input a random gene may read.
fn state_input() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "always",
        "hunger",
        "thirst",
        "boredom",
        "loneliness",
        "crowdedness",
        "pain",
        "age",
        "nearby_sprites",
        "h3",
        "arousal",
        "arousal",
    ])
}

/// A random switch: all its life, or only before or after growing up.
fn switch() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["", ", until: Adult", ", from: Adult", ", from: Old"])
}

/// A random instinct, leaning to Mate.
fn instinct() -> impl Strategy<Value = String> {
    let verb = prop::sample::select(vec![
        "Mate",
        "Mate",
        "Mate",
        "BreedAlone",
        "BreedAlone",
        "Approach",
        "Play",
        "Hit",
        "Wander",
        "Rest",
    ]);
    let attended = prop::sample::select(vec![
        "",
        r#", ("attended_sprite", false)"#,
        r#", ("target_arousal", false)"#,
        r#", ("target_other_sex", false)"#,
    ]);
    (state_input(), any::<bool>(), attended, verb, -1.0f32..1.0, switch()).prop_map(
        |(input, negated, attended, verb, weight, switch)| {
            format!(
                r#"Instinct(inputs: [("{input}", {negated}){attended}], verb: {verb}, weight: {weight:?}{switch}),"#
            )
        },
    )
}

/// A random attention instinct, leaning to sprites.
fn attention() -> impl Strategy<Value = String> {
    let category = prop::sample::select(vec!["sprite", "sprite", "bush", "water", "toy"]);
    (state_input(), category, -1.0f32..1.0, switch()).prop_map(
        |(input, category, weight, switch)| {
            format!(
                r#"AttentionInstinct(input: "{input}", category: "{category}", weight: {weight:?}{switch}),"#
            )
        },
    )
}

/// A random genome of brain genes: each sprite grows up and old at random
/// ages, early enough, in `quick`'s world, that some cross a life stage
/// mid-run; some have a full feminiser, so they turn female if grown; and
/// some are aroused, whatever their age.
fn random_genes() -> impl Strategy<Value = String> {
    let aroused = prop::sample::select(vec![
        "",
        r#"InitialConcentration(chem: "arousal", value: 1.0),"#,
        r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.01, chem: "arousal"),"#,
    ]);
    (
        prop::collection::vec(instinct(), 1..8),
        prop::collection::vec(attention(), 0..4),
        50.0f32..300.0,
        200.0f32..900.0,
        prop::sample::select(vec!["", FEMALE]),
        aroused,
        prop::sample::select(vec!["", LONGS]),
    )
        .prop_map(|(instincts, attention, grows_up, grows_old, female, aroused, longs)| {
            format!(
                r#"Trait(trait: "grows_up", value: {grows_up:?}),
                   Trait(trait: "grows_old", value: {grows_old:?}),
                   Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0, from: Young),
                   Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 0.5, from: Young),
                   {female}
                   {aroused}
                   {longs}
                   {}
                   {}"#,
                attention.join("\n"),
                instincts.join("\n")
            )
        })
}

/// The built-in pack, but with sprites that may grow up and old within a
/// few hundred ticks and eggs that hatch in 100.
fn quick() -> DataPack {
    builtin_changing_files(&[(
        "physiology.ron",
        &[
            (
                "grows_old: (6000.0, 60000.0),",
                "grows_old: (100.0, 60000.0),",
            ),
            ("grows_up: (1500.0, 13500.0),", "grows_up: (10.0, 13500.0),"),
            ("hatches: 750,", "hatches: 100,"),
        ],
    )])
}

/// Whether, in its latest choice, the breeding drive moved a sprite not
/// grown up, or one attending to a sprite not grown up (M2 design §5): its
/// arousal, or the arousal it reads in another, adds nothing then. Says
/// which sprite and how, if it did.
fn drive_moved_a_child(world: &World) -> Option<String> {
    for sprite in world.sprites() {
        let Some(explanation) = sprite.explain() else {
            continue;
        };
        let attended = &explanation.attended;
        let on_a_child = match attended {
            Some(Thing::Sprite(other)) => world
                .sprite(*other)
                .is_some_and(|other| !other.life_stage().is_grown()),
            Some(Thing::ObjectType(name)) => name == "egg",
            _ => false,
        };
        if sprite.life_stage().is_grown() && !on_a_child {
            continue;
        }
        for c in &explanation.contributions {
            if let Part::Concept(inputs) = &c.part
                && inputs.iter().any(|&(input, negated)| {
                    !negated && (input == "arousal" || input == "target_arousal")
                })
            {
                return Some(format!(
                    "a {:?} sprite attending to {attended:?}: {inputs:?} added {}",
                    sprite.life_stage(),
                    c.amount
                ));
            }
        }
    }
    None
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn whatever_the_genome_only_adults_ever_breed_and_only_with_adults(
        genes in random_genes(),
        young in prop::collection::vec(any::<bool>(), 4),
        seed: u64,
    ) {
        // M2 design §5, G7: Mate and the breeding drive only between
        // adults. Every tick also runs the self-check, which fails on a
        // Mate action that isn't between adults.
        let tiles = [at(2, 1), at(3, 1), at(2, 2), at(3, 2)];
        let newborns: Vec<Pos> = tiles
            .iter()
            .zip(&young)
            .filter(|&(_, &young)| young)
            .map(|(&pos, _)| pos)
            .collect();
        let mut world = field_in(quick(), &genes, &tiles, &newborns, seed);
        for _ in 0..600 {
            for (stage, partner) in matings(&mut world, 1) {
                prop_assert!(stage.is_grown(), "a {stage:?} sprite mated");
                prop_assert!(
                    partner.is_some_and(LifeStage::is_grown),
                    "a sprite mated with {partner:?}"
                );
            }
            prop_assert_eq!(drive_moved_a_child(&world), None);
        }
    }
}
