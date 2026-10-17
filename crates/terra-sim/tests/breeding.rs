//! Breeding (M2 design §5, §10): mating happens only between adults,
//! enforced by the engine for any genome and data pack; a male and a female
//! lay an egg, or a female alone; it's an item until it hatches into a young
//! male (§10.4); a lone adult turns female; and births are world state,
//! saved, hashed and replayed.

mod common;
use common::{
    ALONE, FEMALE, KEEN, LONE, Laying, NO_COPYING_ERRORS, RECEPTIVE, WorldBuilder, at, builtin,
    builtin_changing, builtin_changing_files, close, endings, family, family_in, genome, id_at,
    laid, lone, until, value_of,
};

use proptest::prelude::*;
use terra_sim::{
    Command, DataPack, EntityId, EventKind, Learned, LifeStage, Outcome, Part, Playback, Pos,
    Rejection, ScriptedAction, Sex, Target, Thing, Verb, World,
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

/// A Mate or Court action as it started: the life stage of the sprite
/// mating or courting, and of the sprite it's aimed at, or `None` if it's
/// aimed at no sprite.
type Mating = (LifeStage, Option<LifeStage>);

/// Steps `world` `ticks` times, returning each Mate or Court action
/// started (M2 design §15.3). It
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
                Verb::Mate | Verb::Court => {
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

/// An instinct to rest, so the sprites watched don't come to blows.
const REST: &str = r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#;

/// The starter genome's genes for turning female (M2 design §10): the
/// feminiser rises while no female adult is near, and fades. And it rests.
const TURNING: &str = r#"
    Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    Emitter(locus: Locus("female_near"), mode: Level, invert: true, threshold: 0.5, gain: 0.003, chem: "feminiser", from: Adult),
    HalfLife(chem: "feminiser", ticks: 600),
"#;

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
    // An egg is an item, not a sprite (M2 design §10.4).
    assert!(world.sprite(egg).is_none(), "an egg isn't a sprite");
    let egg = world.egg(egg).expect("the egg");
    assert_eq!(egg.parents(), [mother, father]);
    let pos = egg.pos().expect("on the map");
    let item = world.object_at(pos).expect("an item on its tile");
    assert_eq!((item.id(), item.type_name()), (egg.id(), "egg"));
    assert!(item.is_egg() && !item.is_throwable());
    let her = world.sprite(mother).unwrap().pos();
    let (dx, dy) = (pos.x.abs_diff(her.x), pos.y.abs_diff(her.y));
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
    // Without copying errors (M2 design §12), the egg's genes are hers. In
    // a pack that lets her breed alone with him in sight (§15.7).
    let [no_errors, no_flaws] = NO_COPYING_ERRORS;
    let exact = builtin_changing("physiology.ron", &[no_errors, no_flaws, LONE]);
    let mut world = family_in(exact, &[(at(2, 1), &female), (at(5, 2), ALONE)], &[], 1);
    let (mother, male) = (
        world.sprite_at(at(2, 1)).unwrap().id(),
        world.sprite_at(at(5, 2)).unwrap().id(),
    );
    let Laying { egg, by, with, .. } = laid(&mut world, 50).expect("she laid an egg");
    assert_eq!((by, with), (mother, None));
    let egg = world.egg(egg).unwrap();
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
fn an_egg_hatches_after_750_ticks_as_a_young_male_where_it_lies() {
    // M2 design §10.4: the item is gone, and the young sprite it held
    // stands on its tile, with its ID, born as it hatched.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female)], &[], 1);
    let Laying {
        egg,
        by: mother,
        tick: laid_on,
        ..
    } = laid(&mut world, 50).expect("an egg");
    let view = world.egg(egg).unwrap();
    assert_eq!((view.laid(), view.hatches()), (laid_on, laid_on + 750));
    let pos = view.pos().expect("on the map");
    let hatched = until(&mut world, 800, |event| match &event.kind {
        EventKind::Hatched { id, parents } if *id == egg => Some((event.tick, parents.clone())),
        _ => None,
    });
    let (tick, parents) = hatched.expect("it hatched");
    assert_eq!(tick, laid_on + 750);
    assert_eq!(parents, [mother]);
    assert!(world.egg(egg).is_none(), "the egg is gone");
    assert!(world.object_at(pos).is_none(), "and its item with it");
    let young = world.sprite(egg).expect("the hatchling");
    assert_eq!(young.life_stage(), LifeStage::Young);
    assert_eq!(young.sex(), Sex::Male);
    assert_eq!(young.parents(), [mother]);
    assert_eq!(world.tick() - young.age(), tick, "born as it hatched");
    assert_eq!(young.pos(), pos, "it hatched where it lay");
}

#[test]
fn an_egg_lies_still_as_an_item_until_it_hatches() {
    // M2 design §10.4: an egg is no sprite, so it does nothing and needs
    // nothing, and a sprite's mother walks off it, as over any item.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let pos = world.egg(egg).unwrap().pos();
    for _ in 0..700 {
        world.step();
        assert!(world.sprite(egg).is_none(), "an egg isn't a sprite");
        assert_eq!(world.egg(egg).expect("the egg").pos(), pos);
        assert_eq!(world.sprites().count(), 1);
    }
}

#[test]
fn the_cursor_picks_up_and_puts_down_an_egg_but_never_throws_pets_or_zaps_it() {
    // M2 design §10.4, #196: an egg is an item, which the Cursor's Grab
    // mode holds as it does a berry; its type's data says it can't be
    // thrown; and pets and zaps are for sprites.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let pos = world.egg(egg).unwrap().pos().unwrap();
    let refused = |world: &mut World, commands: &[Command]| -> Vec<(Command, Rejection)> {
        for command in commands {
            world.submit(command.clone());
        }
        world
            .step()
            .into_iter()
            .filter_map(|event| match event.kind {
                EventKind::CommandRejected { command, reason } => Some((command, reason)),
                _ => None,
            })
            .collect()
    };
    let touches = [
        Command::Reward {
            sprite: egg,
            amplified: false,
            reach_back: 10,
        },
        Command::Correct {
            sprite: egg,
            amplified: true,
        },
        Command::TakeHold { sprite: egg },
    ];
    let expected: Vec<_> = touches
        .iter()
        .map(|c| (c.clone(), Rejection::Gone))
        .collect();
    assert_eq!(refused(&mut world, &touches), expected);
    assert_eq!(world.cursor().leads(), None);

    assert_eq!(refused(&mut world, &[Command::PickUp { item: egg }]), []);
    let held = world.cursor().holds().expect("it holds the egg");
    assert!(held.is_egg() && !held.is_throwable());
    assert_eq!(world.egg(egg).unwrap().pos(), None, "off the map");
    let throw = Command::Throw {
        from: pos,
        toward: terra_sim::Dir::E,
        tiles: 3,
    };
    let egg_type = world.data().object_type_id("egg").unwrap();
    assert_eq!(
        refused(&mut world, std::slice::from_ref(&throw)),
        [(
            throw,
            Rejection::NotThrowable {
                item_type: egg_type
            }
        )]
    );
    assert_eq!(world.cursor().holds().map(|h| h.id()), Some(egg));
    let down = at(6, 3);
    assert_eq!(refused(&mut world, &[Command::PutDown { tile: down }]), []);
    assert_eq!(world.egg(egg).unwrap().pos(), Some(down));
}

#[test]
fn an_egg_held_when_it_is_due_hatches_on_the_tick_it_is_put_down() {
    // #196: while the Cursor holds it, an egg waits.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    world.submit(Command::PickUp { item: egg });
    let due = world.egg(egg).unwrap().hatches();
    while world.tick() < due + 20 {
        let events = world.step();
        assert!(
            !events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Hatched { .. })),
            "it hatched in the Cursor"
        );
    }
    assert!(world.egg(egg).is_some());
    let free =
        |world: &World, tile| world.sprite_at(tile).is_none() && world.object_at(tile).is_none();
    let down = (0..8)
        .map(|x| at(x, 3))
        .find(|&tile| free(&world, tile))
        .unwrap();
    world.submit(Command::PutDown { tile: down });
    let put_down = world.tick();
    let events = world.step();
    assert!(
        events.iter().any(|event| event.tick == put_down
            && matches!(event.kind, EventKind::Hatched { id, .. } if id == egg)),
        "{events:?}"
    );
    assert_eq!(world.sprite(egg).unwrap().pos(), down);
    assert_eq!(world.cursor().holds().map(|h| h.id()), None);
}

#[test]
fn an_egg_with_a_sprite_on_it_waits_until_the_sprite_steps_off() {
    // #196: a sprite hatches on its egg's tile, so not under another. The
    // Cursor leads one onto the egg, keeps it there past the egg's time,
    // then leads it off.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female), (at(6, 3), REST)], &[], 1);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let resting = world.sprite_at(at(6, 3)).unwrap().id();
    let under = world.egg(egg).unwrap().pos().unwrap();
    world.submit(Command::TakeHold { sprite: resting });
    world.submit(Command::MoveCursor { tile: under });
    let due = world.egg(egg).unwrap().hatches();
    while world.tick() < due + 20 {
        world.step();
        assert!(world.egg(egg).is_some(), "it hatched under a sprite");
    }
    assert_eq!(world.sprite(resting).unwrap().pos(), under, "led onto it");
    let off = (0..8)
        .map(|x| at(x, 3))
        .find(|&tile| tile != under && world.sprite_at(tile).is_none())
        .unwrap();
    world.submit(Command::MoveCursor { tile: off });
    let hatched = until(&mut world, 100, |event| match event.kind {
        EventKind::Hatched { id, .. } if id == egg => Some(event.tick),
        _ => None,
    });
    assert!(hatched.is_some(), "it never hatched");
    assert_eq!(world.sprite(egg).unwrap().pos(), under);
}

/// Hits whatever egg it sees, and nothing else.
const EGG_HITTER: &str = r#"
    AttentionInstinct(input: "always", category: "egg", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_egg", false)], verb: Hit, weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_egg", false), ("target_adjacent", true)], verb: Approach, weight: 2.0),
"#;

#[test]
fn hitting_an_egg_does_nothing() {
    // M2 design §10.4: an egg can't be hurt until something in the world is
    // meant to hurt eggs, so it's a fruitless try, and it hatches as it
    // would have.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(
        lone(),
        &[(at(2, 1), &female), (at(6, 3), EGG_HITTER)],
        &[],
        1,
    );
    let egg = laid(&mut world, 50).expect("an egg").egg;
    let due = world.egg(egg).unwrap().hatches();
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
    assert_eq!(world.egg(egg).expect("the egg").hatches(), due);
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
fn a_female_breeds_alone_only_long_after_she_last_saw_a_male_adult_whatever_her_genes() {
    // M2 design §15.7: the body's rule, here after 1,000 ticks. With a male
    // adult in sight she lays nothing alone, however she longs; with none,
    // she can once that long has passed. A young male doesn't count.
    let soon = || {
        builtin_changing(
            "physiology.ron",
            &[("alone_after: 4500,", "alone_after: 1000,")],
        )
    };
    let female = format!("{ALONE}{FEMALE}");
    let laid_alone = |others: &[(Pos, &str)], young: &[Pos]| {
        let sprites: Vec<(Pos, &str)> = [(at(1, 1), female.as_str())]
            .into_iter()
            .chain(others.iter().copied())
            .collect();
        let mut world = family_in(soon(), &sprites, young, 1);
        let ticks: Vec<u64> = (0..1_500)
            .flat_map(|_| world.step())
            .filter_map(|event| match event.kind {
                EventKind::Laid { with: None, .. } => Some(event.tick),
                _ => None,
            })
            .collect();
        ticks
    };
    let watched = laid_alone(&[(at(6, 2), REST)], &[]);
    assert!(watched.is_empty(), "{watched:?}");
    let alone = laid_alone(&[], &[]);
    assert!(
        alone.first().is_some_and(|&tick| tick >= 1_000),
        "{alone:?}"
    );
    let with_young = laid_alone(&[(at(6, 2), REST)], &[at(6, 2)]);
    assert!(
        with_young.first().is_some_and(|&tick| tick >= 1_000),
        "{with_young:?}"
    );
}

#[test]
fn a_pack_from_before_eggs_were_items_lays_egg_items_that_hatch_when_it_says() {
    // #196: such a pack, which a save embeds, had eggs as sprites, its
    // `egg` a pseudo type, and said when they hatched in physiology.ron.
    // Its eggs are items now, which hatch as it said.
    let old = builtin_changing_files(&[
        (
            "objects.ron",
            &[(
                r#"(id: 103, name: "egg", plural: "eggs", category: "egg", size: Small, hardness: 1.0,
     stages: [(name: "laid", ticks: (750, 750), next: Hatch)],
     throwable: false,
     verbs: {}),"#,
                r#"(id: 103, name: "egg", plural: "eggs", category: "egg", pseudo: true, verbs: {}),"#,
            )],
        ),
        (
            "physiology.ron",
            &[
                ("breeding: (\n", "breeding: (\n        hatches: 300,\n"),
                LONE,
            ],
        ),
    ]);
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(old, &[(at(2, 1), &female)], &[], 1);
    let laying = laid(&mut world, 50).expect("an egg");
    let egg = world.egg(laying.egg).expect("an egg item");
    assert_eq!(egg.type_name(), "egg");
    assert_eq!(egg.hatches(), laying.tick + 300);
    let item = world.object_at(egg.pos().unwrap()).unwrap();
    assert!(!item.is_throwable());
}

#[test]
fn an_egg_saved_and_loaded_hatches_the_same() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female), (at(5, 2), KEEN)], &[], 4);
    let egg = laid(&mut world, 50).expect("an egg").egg;
    world.step();
    world.submit(Command::Rename {
        sprite: egg,
        name: "Pebble".into(),
    });
    world.step();
    let mut loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    let loaded_egg = loaded.egg(egg).expect("the egg was saved");
    let saved_egg = world.egg(egg).unwrap();
    assert_eq!(loaded_egg.hatches(), saved_egg.hatches());
    assert_eq!(loaded_egg.genome(), saved_egg.genome());
    assert_eq!(loaded_egg.name(), Some("Pebble"));
    for _ in 0..800 {
        assert_eq!(world.step(), loaded.step());
        assert_eq!(loaded.state_hash(), world.state_hash());
    }
    assert!(loaded.egg(egg).is_none(), "it hatched");
    assert_eq!(loaded.sprite(egg).unwrap().name(), Some("Pebble"));
}

#[test]
fn a_birth_hashes_differently() {
    // Births are world state (M2 design §10): two worlds alike but for an
    // egg aren't the same world.
    let female = format!("{ALONE}{FEMALE}");
    let mut laying = family_in(lone(), &[(at(2, 1), &female)], &[], 1);
    let mut not = family_in(lone(), &[(at(2, 1), FEMALE)], &[], 1);
    laid(&mut laying, 50).expect("an egg");
    while not.tick() < laying.tick() {
        not.step();
    }
    assert_ne!(laying.state_hash(), not.state_hash());
}

#[test]
fn births_replay_exactly() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(lone(), &[(at(2, 1), &female), (at(5, 2), KEEN)], &[], 2);
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
    let female = format!("{BODY_LANGUAGE}{FEMALE}{RECEPTIVE}");
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
    let reads_it = r#"(id: 80, name: "male_near",     reads: Locus("male_near")),"#;
    let calls = |line: &str| {
        let starter = include_str!("../../../data/genomes/starter.ron");
        let found = starter.lines().find(|l| l.contains(line));
        found.expect("the starter's call gene").to_string()
    };
    let female_calls = calls(r#"("male_near", true)], verb: Call"#);
    let longing = |line: &str| {
        let starter = include_str!("../../../data/genomes/starter.ron");
        let found = starter.lines().find(|l| l.contains(line));
        found.expect("the starter's longing gene").to_string()
    };
    let (raise, drop) = (
        longing("chem: \"longing\", from: Adult"),
        longing("gain: -0.01, chem: \"longing\""),
    );
    builtin_changing_files(&[
        ("loci.ron", &[(male_near, "")]),
        ("brain_io.ron", &[(reads_it, "")]),
        (
            "genomes/starter.ron",
            &[(&raise, ""), (&drop, ""), (&female_calls, "")],
        ),
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

/// What `id` has learned `thing` is worth for its breeding drive.
fn worth_for_the_drive(world: &World, id: EntityId, thing: Thing) -> f32 {
    let drive = Learned::Worth {
        thing,
        need: Some("arousal".into()),
    };
    value_of(world, id, &drive)
}

/// Aroused from the start, and cooling fast all the while, whatever it
/// does; its eye on the sprite it sees, which it courts.
const COOLING: &str = r#"
    InitialConcentration(chem: "arousal", value: 1.0),
    HalfLife(chem: "arousal", ticks: 10),
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false)], verb: Court, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "curiosity", value: 0.0),
"#;

/// Aroused from the start, until it has young.
const SPENT: &str = r#"
    InitialConcentration(chem: "arousal", value: 1.0),
    Emitter(locus: Locus("bred"), mode: Level, gain: -1.0, chem: "arousal"),
"#;

#[test]
fn the_breeding_drive_eased_teaches_worth_only_after_a_mate_and_only_of_the_partner() {
    // M2 design §5: the drive's fall while it plays with a ball, plays with
    // an adult or courts one teaches nothing of what either is worth for
    // the drive. Only a Mate's does, about the partner.
    let (me, ball, other) = (at(2, 1), at(2, 2), at(3, 1));
    let data = builtin();
    let sprites = vec![
        (me, Some(genome(COOLING, &data))),
        (other, Some(genome(REST, &data))),
    ];
    let mut world = WorldBuilder::new(&["........", "........", "........", "........"])
        .data(data)
        .objects(&[(ball, "ball")])
        .sprites_with(sprites)
        .scripts(&[
            (me, ScriptedAction::Play { at: ball }),
            (me, ScriptedAction::Play { at: other }),
        ])
        .build();
    let (me, other) = (id_at(&world, me), id_at(&world, other));
    let mut tries = Vec::new();
    for _ in 0..20 {
        tries.extend(endings(&world.step()));
    }
    for tried in [Verb::Play, Verb::Court] {
        assert!(tries.contains(&(tried, Outcome::Applied)), "{tries:?}");
    }
    assert_eq!(worth_for_the_drive(&world, me, "ball".into()), 0.0);
    assert_eq!(worth_for_the_drive(&world, me, Thing::Sprite(other)), 0.0);

    let female = format!("{KEEN}{FEMALE}{SPENT}");
    let male = format!("{KEEN}{SPENT}");
    let mut world = family(&[(at(2, 1), &female), (at(3, 1), &male)], &[], 1);
    let laying = laid(&mut world, 100).expect("an egg");
    let father = laying.with.expect("a pair's egg");
    world.step();
    world.step();
    for (id, partner) in [(laying.by, father), (father, laying.by)] {
        let worth = worth_for_the_drive(&world, id, Thing::Sprite(partner));
        assert!(worth > 0.0, "{worth}");
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
        "Court",
        "Court",
        "Call",
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
        r#", ("target_familiar", false)"#,
        r#", ("target_receptivity", false)"#,
    ]);
    (state_input(), any::<bool>(), attended, verb, -1.0f32..1.0, switch()).prop_map(
        |(input, negated, attended, verb, weight, switch)| {
            format!(
                r#"Instinct(inputs: [("{input}", {negated}){attended}], verb: {verb}, weight: {weight:?}{switch}),"#
            )
        },
    )
}

/// A random attention instinct, leaning to sprites, and about any of them or
/// only its family, kin, partner, parents or young (M2 design §15.6), the
/// grown-ups or a parent showing it something (§18.1).
fn attention() -> impl Strategy<Value = String> {
    let category = prop::sample::select(vec!["sprite", "sprite", "egg", "bush", "water", "toy"]);
    let whom = prop::sample::select(vec![
        "",
        ", whom: Family",
        ", whom: Kin",
        ", whom: Partner",
        ", whom: Parent",
        ", whom: Young",
        ", whom: Grown",
        ", whom: Showing",
    ]);
    (state_input(), category, whom, -1.0f32..1.0, switch()).prop_map(
        |(input, category, whom, weight, switch)| {
            format!(
                r#"AttentionInstinct(input: "{input}", category: "{category}"{whom}, weight: {weight:?}{switch}),"#
            )
        },
    )
}

/// A random genome of brain genes: each sprite grows up and old at random
/// ages, early enough, in `quick`'s world, that some cross a life stage
/// mid-run; some have a full feminiser, so they turn female if grown; some
/// are aroused, whatever their age; and each learns by watching at a
/// random rate (M2 design §18.1).
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
        0.0f32..1.0,
    )
        .prop_map(|(instincts, attention, grows_up, grows_old, female, aroused, longs, watch)| {
            format!(
                r#"Trait(trait: "grows_up", value: {grows_up:?}),
                   Trait(trait: "grows_old", value: {grows_old:?}),
                   BrainParam(param: "watch_rate", value: {watch:?}),
                   Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0, from: Young),
                   Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 0.5, from: Young),
                   Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Court, weight: 1.0, from: Young),
                   InitialConcentration(chem: "receptivity", value: 1.0),
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
    builtin_changing_files(&[
        (
            "physiology.ron",
            &[
                (
                    "grows_old: (6000.0, 60000.0),",
                    "grows_old: (100.0, 60000.0),",
                ),
                ("grows_up: (1500.0, 13500.0),", "grows_up: (10.0, 13500.0),"),
            ],
        ),
        (
            "objects.ron",
            &[(
                "ticks: (750, 750), next: Hatch",
                "ticks: (100, 100), next: Hatch",
            )],
        ),
    ])
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

/// Whether a sprite not grown up knows anything about mating (M2 design
/// §18.3): what anything is worth for the breeding drive, or a habit of
/// mating or courting. It never tries either itself, so it could only have
/// learned it by watching. Says which sprite and what, if one does.
fn a_child_knows_about_mating(world: &World) -> Option<String> {
    for sprite in world.sprites() {
        if sprite.life_stage().is_grown() {
            continue;
        }
        for memory in sprite.memory() {
            let about_mating = match &memory.learned {
                Learned::Worth { need, .. } => need.as_deref() == Some("arousal"),
                Learned::Habit { verb, .. } => matches!(verb, Verb::Mate | Verb::Court),
                _ => false,
            };
            if about_mating && memory.amount != 0.0 {
                return Some(format!("{:?} knows {memory:?}", sprite.id()));
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
        // M2 design §5, G7: Mate, Court and the breeding drive only
        // between adults, on a trip to a call too, and whomever an attention
        // instinct is about (§15.1, §15.6, §15.9), and nothing about it
        // learned by watching (§18.3). Every tick also runs the self-check,
        // which fails on a Mate or Court action that isn't between adults.
        // The last is out of the others' sight, but within a call's reach.
        let tiles = [at(2, 1), at(3, 1), at(2, 2), at(24, 2)];
        let newborns: Vec<Pos> = tiles
            .iter()
            .zip(&young)
            .filter(|&(_, &young)| young)
            .map(|(&pos, _)| pos)
            .collect();
        let data = quick();
        let genome = genome(&genes, &data);
        let field = ".".repeat(28);
        let rows = [field.as_str(); 4];
        let mut world = WorldBuilder::new(&rows)
            .data(data)
            .sprites_with(tiles.iter().map(|&pos| (pos, Some(genome.clone()))).collect())
            .newborns(&newborns)
            .seed(seed)
            .build();
        for _ in 0..600 {
            for (stage, partner) in matings(&mut world, 1) {
                prop_assert!(stage.is_grown(), "a {stage:?} sprite mated");
                prop_assert!(
                    partner.is_some_and(LifeStage::is_grown),
                    "a sprite mated with {partner:?}"
                );
            }
            prop_assert_eq!(drive_moved_a_child(&world), None);
            prop_assert_eq!(a_child_knows_about_mating(&world), None);
        }
    }
}
