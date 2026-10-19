//! Lab scenarios (design §7.1): a world, run for any seed, with what
//! happened in each window counted.

mod common;
use common::{WorldBuilder, at, barren, builtin};

use std::collections::BTreeMap;

use terra_sim::{
    Childhood, CursorTouch, DeathCause, Event, EventKind, LabError, LabRun, LabScenario, Learned,
    LifeStage, Outcome, Verb, Window, Without, World, WorldConfig, report,
};

const ARENA: &str = r#"(
    world: Drawn(
        rows: [
            "........",
            ".S..T...",
            "....B.~~",
            "......~~",
        ],
        key: {'S': Sprite, 'T': Object("thornbush"), 'B': Object("berry_bush")},
    ),
    ticks: 3000,
    windows: [(0, 1000), (1000, 3000)],
)"#;

#[test]
fn a_window_must_lie_within_the_run() {
    let data = builtin();
    let past = ARENA.replace("(1000, 3000)", "(1000, 3001)");
    assert_eq!(
        LabScenario::from_ron(&past, &data).err(),
        Some(LabError::BadWindow {
            from: 1000,
            to: 3001
        })
    );
    let backwards = ARENA.replace("(1000, 3000)", "(1000, 1000)");
    assert!(matches!(
        LabScenario::from_ron(&backwards, &data),
        Err(LabError::BadWindow { .. })
    ));
}

#[test]
fn a_key_must_name_an_object_type() {
    let data = builtin();
    let text = ARENA.replace("Object(\"thornbush\")", "Object(\"thistle\")");
    assert!(matches!(
        LabScenario::from_ron(&text, &data),
        Err(LabError::Scenario(_))
    ));
}

/// What `events` add to a window's counts, counted here by hand, in a world
/// whose one sprite is a starter: an adult, and not yet old.
fn tally(events: &[Event], window: &mut Window) {
    for event in events {
        match &event.kind {
            EventKind::ActionEnded {
                verb,
                outcome: Outcome::Applied,
                action,
                ..
            } => {
                *window
                    .applied
                    .entry((*verb, action.target_type))
                    .or_insert(0) += 1
            }
            EventKind::Died { cause, .. } => {
                *window.deaths.entry(*cause).or_insert(0) += 1;
                *window
                    .stage_deaths
                    .entry((LifeStage::Adult, *cause))
                    .or_insert(0) += 1;
            }
            EventKind::LearnedMilestone { learned, good, .. } => {
                *window.lessons.entry((learned.clone(), *good)).or_insert(0) += 1
            }
            _ => {}
        }
    }
}

#[test]
fn a_run_counts_applied_actions_and_deaths_in_each_window() {
    let data = builtin();
    let lab = LabScenario::from_ron(ARENA, &data).expect("a valid scenario");
    let run = lab.run(data.clone(), 1);

    // The same world, drawn by hand, stepped here: objects in reading
    // order, then the sprite.
    let rows = ["........", "........", "......~~", "......~~"];
    let objects = [(at(4, 1), "thornbush"), (at(4, 2), "berry_bush")];
    let mut world = WorldBuilder::new(&rows)
        .data(data)
        .objects(&objects)
        .starters(&[at(1, 1)])
        .build();
    let mut windows = [(0, 1000), (1000, 3000)].map(|(from, to)| Window {
        from,
        to,
        applied: BTreeMap::new(),
        deaths: BTreeMap::new(),
        touches: BTreeMap::new(),
        lessons: BTreeMap::new(),
        young: None,
        first: Childhood::default(),
        later: Childhood::default(),
        stage_deaths: BTreeMap::new(),
    });
    for _ in 0..3000 {
        let events = world.step();
        for window in &mut windows {
            let tick = world.tick() - 1;
            if (window.from..window.to).contains(&tick) {
                tally(&events, window);
            }
        }
    }
    assert_eq!(
        run,
        LabRun {
            windows: windows.to_vec(),
            control: None,
            without: None,
        }
    );
    assert!(
        run.windows[0].applied.values().sum::<u64>() > 0,
        "the sprite did something"
    );
    assert!(
        run.windows.iter().any(|w| !w.lessons.is_empty()),
        "and learned something"
    );
}

#[test]
fn the_report_lists_each_window_with_a_column_per_seed_and_the_median() {
    let data = builtin();
    // Object types: thornbush 3.
    let window = |eats: u64, wanders: u64, thorn_deaths: u64| Window {
        from: 0,
        to: 5000,
        applied: BTreeMap::from([
            ((Verb::Eat, Some(3)), eats),
            ((Verb::Wander, None), wanders),
        ]),
        deaths: BTreeMap::from([(DeathCause::HurtBy(3), thorn_deaths)]),
        touches: BTreeMap::new(),
        lessons: BTreeMap::new(),
        young: None,
        first: Childhood::default(),
        later: Childhood::default(),
        stage_deaths: BTreeMap::new(),
    };
    let runs = [
        (
            1,
            LabRun {
                windows: vec![window(23, 40, 1)],
                control: None,
                without: None,
            },
        ),
        (
            2,
            LabRun {
                windows: vec![window(9, 38, 0)],
                control: None,
                without: None,
            },
        ),
        (
            3,
            LabRun {
                windows: vec![window(12, 51, 0)],
                control: None,
                without: None,
            },
        ),
    ];
    assert_eq!(
        report(&runs, &data),
        "\
ticks 0 to 5,000              seed 1    seed 2    seed 3    median
  eat thornbush                   23         9        12        12
  wander                          40        38        51        40
  died: hurt by thornbush          1         0         0         0
"
    );
}

#[test]
fn the_report_lists_the_control_runs_after_the_learning_runs() {
    let data = builtin();
    let window = |eats| Window {
        from: 0,
        to: 5000,
        applied: BTreeMap::from([((Verb::Eat, Some(3)), eats)]),
        deaths: BTreeMap::new(),
        touches: BTreeMap::new(),
        lessons: BTreeMap::new(),
        young: None,
        first: Childhood::default(),
        later: Childhood::default(),
        stage_deaths: BTreeMap::new(),
    };
    let runs = [
        (
            1,
            LabRun {
                windows: vec![window(3)],
                control: Some(vec![window(30)]),
                without: Some(Without::Learning),
            },
        ),
        (
            2,
            LabRun {
                windows: vec![window(5)],
                control: Some(vec![window(24)]),
                without: Some(Without::Learning),
            },
        ),
    ];
    assert_eq!(
        report(&runs, &data),
        "ticks 0 to 5,000              seed 1    seed 2    median
  eat thornbush                    3         5         4

control, without learning
ticks 0 to 5,000              seed 1    seed 2    median
  eat thornbush                   30        24        27
"
    );
}

#[test]
fn a_scenario_can_ask_for_a_control_run_of_each_seed_without_learning() {
    // Design v16 §7.1: the learning run is unchanged, and a second run of
    // the same seed, with learning switched off, is counted in the same
    // windows.
    let data = builtin();
    let with_control = ARENA.replace(
        "ticks: 3000,",
        "ticks: 3000,
    control: NoLearning,",
    );
    let plain = LabScenario::from_ron(ARENA, &data).expect("a valid scenario");
    let controlled = LabScenario::from_ron(&with_control, &data).expect("a valid scenario");
    let (run, plain_run) = (controlled.run(data.clone(), 5), plain.run(data.clone(), 5));
    assert_eq!(run.windows, plain_run.windows);
    assert_eq!(run.without, Some(Without::Learning));
    let control = run.control.expect("a control run");
    let spans: Vec<(u64, u64)> = control.iter().map(|w| (w.from, w.to)).collect();
    assert_eq!(spans, [(0, 1000), (1000, 3000)]);
    assert!(
        control[0].applied.values().sum::<u64>() > 0,
        "its sprite lived too"
    );
}

/// A sprite with balls to play with, and a trainer that pets it for each
/// applied Play on a ball until tick 2,000.
const PLAYGROUND: &str = r#"(
    world: Drawn(
        rows: [
            "........",
            ".S..o...",
            "....o.~~",
            "..B...~~",
        ],
        key: {'S': Sprite, 'o': Object("ball"), 'B': Object("berry_bush")},
    ),
    ticks: 3000,
    trainer: (on: (Play, "ball"), give: Pet, until: 2000),
    control: NoTrainer,
    windows: [(0, 2000), (0, 3000)],
)"#;

#[test]
fn a_trainer_must_answer_an_object_type_and_a_control_without_it_needs_one() {
    let data = builtin();
    let unicorn = PLAYGROUND.replace("(Play, \"ball\")", "(Play, \"unicorn\")");
    assert_eq!(
        LabScenario::from_ron(&unicorn, &data).err(),
        Some(LabError::UnknownTarget("unicorn".into()))
    );
    let untrained = PLAYGROUND.replace(
        "trainer: (on: (Play, \"ball\"), give: Pet, until: 2000),",
        "",
    );
    assert_eq!(
        LabScenario::from_ron(&untrained, &data).err(),
        Some(LabError::NoTrainer)
    );
    // Design v21 §5.6: a zap or a shock always looks back the touch window.
    let reaching = PLAYGROUND.replace("give: Pet,", "give: Zap, reach_back: 20,");
    assert_eq!(
        LabScenario::from_ron(&reaching, &data).err(),
        Some(LabError::ReachBackOnCorrect)
    );
}

#[test]
fn a_trainer_pets_the_sprite_for_each_applied_play_on_a_ball_until_it_stops() {
    // Design v21 §7.1: each applied Play on a ball before tick 2,000 gets a
    // pet the tick after; later ones get none.
    let data = builtin();
    let lab = LabScenario::from_ron(PLAYGROUND, &data).expect("a valid scenario");
    let run = lab.run(data.clone(), 3);
    let plays = run.windows[0].applied_on(Verb::Play, "ball", &data);
    assert!(plays > 1, "the sprite played: {plays}");
    assert_eq!(
        run.windows[1].touches,
        BTreeMap::from([(CursorTouch::Pet, plays)])
    );
}

#[test]
fn a_trainer_waits_its_delay_before_each_touch() {
    // A delay as long as the run: no pet lands within it.
    let data = builtin();
    let late = PLAYGROUND.replace("give: Pet,", "give: Pet, delay: 3000,");
    let lab = LabScenario::from_ron(&late, &data).expect("a valid scenario");
    let run = lab.run(data.clone(), 3);
    assert!(run.windows[0].applied_on(Verb::Play, "ball", &data) > 0);
    assert_eq!(run.windows[1].touches, BTreeMap::new());
}

#[test]
fn a_trainer_s_control_run_is_the_same_seed_without_it() {
    // Design v21 §7.3: learning stays on; only the trainer is missing.
    let data = builtin();
    let lab = LabScenario::from_ron(PLAYGROUND, &data).expect("a valid scenario");
    let run = lab.run(data.clone(), 3);
    assert_eq!(run.without, Some(Without::Trainer));
    let control = run.control.expect("a control run");
    assert!(control.iter().all(|w| w.touches.is_empty()));
    assert!(control[0].applied.values().sum::<u64>() > 0, "it lived too");
}

#[test]
fn the_report_counts_the_cursor_s_touches_and_lessons_and_names_the_control() {
    let data = builtin();
    // Object types: ball 4.
    let window = |pets: u64, lessons: u64| Window {
        from: 0,
        to: 5000,
        applied: BTreeMap::from([((Verb::Play, Some(4)), 7)]),
        deaths: BTreeMap::new(),
        touches: if pets > 0 {
            BTreeMap::from([(CursorTouch::Pet, pets)])
        } else {
            BTreeMap::new()
        },
        lessons: if lessons > 0 {
            let balls_good = Learned::Worth {
                thing: "ball".into(),
                need: None,
            };
            BTreeMap::from([((balls_good, true), lessons)])
        } else {
            BTreeMap::new()
        },
        young: None,
        first: Childhood::default(),
        later: Childhood::default(),
        stage_deaths: BTreeMap::new(),
    };
    let runs = [(
        1,
        LabRun {
            windows: vec![window(7, 1)],
            control: Some(vec![window(0, 0)]),
            without: Some(Without::Trainer),
        },
    )];
    assert_eq!(
        report(&runs, &data),
        "ticks 0 to 5,000              seed 1    median
  play ball                        7         7
  given: pet                       7         7
  lesson: ball is good             1         1

control, without the trainer
ticks 0 to 5,000              seed 1    median
  play ball                        7         7
"
    );
}

#[test]
fn a_scenario_can_have_its_sprites_grow_old_at_another_age() {
    // M2 design §2: a scenario watching the same sprites for longer than a
    // life gives them a later old age; here, a sooner one. Starting just
    // grown up at 4,500 ticks, a starter sprite would grow old at tick
    // 9,000; at 6,000 it grows old at tick 1,500 and dies of it within 7,000.
    let data = barren();
    let whole = ARENA
        .replace("ticks: 3000", "ticks: 7000")
        .replace("windows: [(0, 1000), (1000, 3000)]", "windows: [(0, 7000)]");
    let old_age = |text: &str| {
        let lab = LabScenario::from_ron(text, &data).expect("a valid scenario");
        let run = lab.run(data.clone(), 1);
        run.windows[0].deaths.get(&DeathCause::OldAge).copied()
    };
    assert_eq!(old_age(&whole), None);
    let sooner = whole.replace("ticks: 7000", "grows_old: 6000.0,\n    ticks: 7000");
    assert_eq!(old_age(&sooner), Some(1));
}

/// Two grown sprites who each get a newborn of their own at tick 100, in a
/// world with room for no more, and a control run of orphans.
const FAMILIES: &str = r#"(
    world: Drawn(
        rows: [
            "..........",
            ".S......S.",
            "....~~....",
            "..........",
        ],
        key: {'S': Sprite},
    ),
    most_sprites: 4,
    ticks: 400,
    young: (at: 100),
    control: Orphans,
    windows: [(0, 100), (100, 400)],
)"#;

#[test]
fn a_control_run_of_orphans_needs_young_that_appear_within_the_run() {
    let data = builtin();
    let no_young = FAMILIES.replace("young: (at: 100),", "");
    assert_eq!(
        LabScenario::from_ron(&no_young, &data).err(),
        Some(LabError::NoYoung)
    );
    let late = FAMILIES.replace("(at: 100)", "(at: 400)");
    assert_eq!(
        LabScenario::from_ron(&late, &data).err(),
        Some(LabError::LateYoung(400))
    );
}

#[test]
fn a_scenario_can_give_each_sprite_a_newborn_of_its_own() {
    // M2 design §18.6: young with a parent, against orphans.
    let data = builtin();
    let lab = LabScenario::from_ron(FAMILIES, &data).expect("a valid scenario");
    let run = lab.run(data, 1);
    assert_eq!(run.windows[0].young, None, "none yet");
    let young = run.windows[1].young.as_ref().expect("the young");
    assert_eq!(young.young, 2);
    // What the young did is part of what everyone did.
    for (action, &count) in &young.applied {
        assert!(run.windows[1].applied[action] >= count);
    }
}

#[test]
fn a_control_run_of_orphans_takes_the_grown_ups_away_as_their_young_appear() {
    let data = builtin();
    let lab = LabScenario::from_ron(FAMILIES, &data).expect("a valid scenario");
    let run = lab.run(data, 1);
    assert_eq!(run.without, Some(Without::Parents));
    let control = run.control.expect("a control run");
    // Until the young appear, the control run is the same world.
    assert_eq!(control[0], run.windows[0]);
    let young = control[1].young.as_ref().expect("the young");
    assert_eq!(young.young, 2);
    // Once they have, the young are all that acts.
    assert!(run.windows[1].applied.values().sum::<u64>() > young.applied.values().sum::<u64>());
    assert_eq!(control[1].applied, young.applied);
}

#[test]
fn the_young_s_lessons_count_who_learned_each_and_the_median_ticks_to_it() {
    // M2 design §18.6: one that hasn't learned a lesson counts the ticks to
    // the window's end.
    let data = builtin();
    let long = FAMILIES
        .replace("ticks: 400", "ticks: 4100")
        .replace("(100, 400)", "(100, 4100)");
    let lab = LabScenario::from_ron(&long, &data).expect("a valid scenario");
    let run = lab.run(data.clone(), 1);
    let young = run.windows[1].young.as_ref().expect("the young");
    assert!(!young.learned.is_empty(), "they learned something");
    for (lesson, &knew) in &young.learned {
        let took = young.learned_after[lesson];
        assert!((1..=2).contains(&knew));
        assert!(took <= 4000);
        if knew == 1 {
            // Of two, the median is the mean of the one that learned it and
            // the one that didn't, which counts the whole window.
            assert!(took > 2000);
        }
    }
    let text = report(&[(1, run)], &data);
    assert!(text.contains("  young  "), "{text}");
    assert!(text.contains("young learned: "), "{text}");
    assert!(text.contains("ticks to learn: "), "{text}");
    assert!(text.contains("control, orphans"), "{text}");
}

#[test]
fn the_unattended_world_starts_with_newborns_only() {
    // M2 design §19.6: the default world, its first sprites newborns.
    let data = builtin();
    let config = WorldConfig::builtin(&data);
    let sprites = config.sprites();
    let world = World::unattended(config, data.clone(), 1);
    let first: Vec<_> = world.sprites().collect();
    assert_eq!(first.len(), sprites);
    assert!(
        first
            .iter()
            .all(|s| s.life_stage() == LifeStage::Young && s.age() == 0)
    );
    let lab = LabScenario::from_ron("(world: Unattended, ticks: 50, windows: [(0, 50)])", &data)
        .expect("a valid scenario");
    let run = lab.run(data, 1);
    assert_eq!(run.windows[0].first.grew_up, 0, "none grows up so soon");
}

#[test]
fn the_report_counts_who_grew_up_and_deaths_by_life_stage() {
    let data = builtin();
    let window = |grew_up: u64, died_young: u64| Window {
        from: 0,
        to: 6000,
        applied: BTreeMap::new(),
        deaths: BTreeMap::from([(DeathCause::Dehydration, died_young)]),
        touches: BTreeMap::new(),
        lessons: BTreeMap::new(),
        young: None,
        first: Childhood {
            grew_up,
            died_young,
        },
        later: Childhood::default(),
        stage_deaths: BTreeMap::from([((LifeStage::Young, DeathCause::Dehydration), died_young)]),
    };
    let runs = [
        (
            1,
            LabRun {
                windows: vec![window(6, 24)],
                control: None,
                without: None,
            },
        ),
        (
            2,
            LabRun {
                windows: vec![window(4, 26)],
                control: None,
                without: None,
            },
        ),
    ];
    assert_eq!(
        report(&runs, &data),
        "\
ticks 0 to 6,000              seed 1    seed 2    median
  died: dehydration               24        26        25
  grew up: first                   6         4         5
  died young: first               24        26        25
  died, young: dehydration        24        26        25
"
    );
}

#[test]
fn lone_newborns_can_take_after_the_hatched_sprites_of_another_world() {
    // M2 design §19.6: the long run's lone newborns, given the genes of the
    // hatched sprites alive at its end. A world with none gives none.
    let data = builtin();
    let config = WorldConfig::builtin(&data);
    let mut world = World::new(config.clone(), data.clone(), 1);
    assert!(World::unattended_like(config.clone(), data.clone(), 1, &world).is_none());
    while !world.sprites().any(|s| !s.parents().is_empty()) {
        assert!(world.tick() < 10_000, "an egg hatches");
        world.step();
    }
    let hatched: Vec<_> = world
        .sprites()
        .filter(|s| !s.parents().is_empty())
        .map(|s| s.genome().clone())
        .collect();
    let lone = World::unattended_like(config.clone(), data, 1, &world).expect("hatched sprites");
    let newborns: Vec<_> = lone.sprites().collect();
    assert_eq!(newborns.len(), config.sprites());
    for newborn in &newborns {
        assert_eq!(newborn.life_stage(), LifeStage::Young);
        assert!(
            hatched.contains(newborn.genome()),
            "takes after one of them"
        );
    }
}
