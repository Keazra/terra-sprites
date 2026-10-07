//! Lab scenarios (design §7.1): a world, run for any seed, with what
//! happened in each window counted.

mod common;
use common::builtin;

use std::collections::BTreeMap;

use terra_sim::{
    CursorTouch, DeathCause, Event, EventKind, LabError, LabRun, LabScenario, Learned, Map,
    Outcome, Pos, Scenario, Verb, Window, Without, World, report,
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
    windows: [(0, 1000), (2000, 3000)],
)"#;

#[test]
fn a_window_must_lie_within_the_run() {
    let data = builtin();
    let past = ARENA.replace("(2000, 3000)", "(2000, 3001)");
    assert_eq!(
        LabScenario::from_ron(&past, &data).err(),
        Some(LabError::BadWindow {
            from: 2000,
            to: 3001
        })
    );
    let backwards = ARENA.replace("(2000, 3000)", "(2000, 2000)");
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

/// What `events` add to a window's counts, counted here by hand.
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
            EventKind::Died { cause, .. } => *window.deaths.entry(*cause).or_insert(0) += 1,
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
    let run = lab.run(data.clone(), 5);

    // The same world, drawn by hand, stepped here: objects in reading
    // order, then the sprite.
    let rows = ["........", "........", "......~~", "......~~"];
    let map = Map::from_ascii(&rows, &data).expect("valid drawing");
    let objects = [
        (Pos { x: 4, y: 1 }, "thornbush"),
        (Pos { x: 4, y: 2 }, "berry_bush"),
    ];
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &[(Pos { x: 1, y: 1 }, None)],
        scripted: &[],
    };
    let mut world = World::from_scenario(scenario, data.clone(), 5).expect("a valid scenario");
    let mut windows = [(0, 1000), (2000, 3000)].map(|(from, to)| Window {
        from,
        to,
        applied: BTreeMap::new(),
        deaths: BTreeMap::new(),
        touches: BTreeMap::new(),
        lessons: BTreeMap::new(),
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
    assert_eq!(spans, [(0, 1000), (2000, 3000)]);
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
