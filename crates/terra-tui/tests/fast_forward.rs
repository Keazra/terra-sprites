//! Fast-forward (M2 design §16): a Lab world run with no screen for hours
//! of play ends exactly as the game would have left it, reporting how it
//! went a tenth of the way at a time: the life stages, the deepest
//! generation and the deaths by cause.

use std::collections::BTreeMap;
use std::time::Duration;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, LifeStage, Map, Mode, Pos, Scenario, World,
    WorldConfig,
};
use terra_tui::fast_forward::{
    self, FastForward, HEADER, HOUR, Row, Tally, hours, save_name, spoken,
};

/// A small generated Lab world.
fn lab_world(seed: u64) -> World {
    let data = DataPack::builtin().expect("valid pack");
    let config = WorldConfig::builtin(&data).sized(40, 32);
    World::new_in(config, data, seed, Mode::Lab)
}

/// Runs `run` to its end, taking each row as it's due.
fn run_through(run: &mut FastForward) -> Vec<Row> {
    let mut rows = Vec::new();
    while let Some(next) = run.next_row() {
        run.run_to(next);
        rows.push(run.take_row());
    }
    rows
}

#[test]
fn a_fast_forwarded_world_is_the_world_the_game_would_have() {
    let mut run = FastForward::new(lab_world(7), 1);
    run_through(&mut run);
    assert!(run.done());
    let mut played = lab_world(7);
    for _ in 0..HOUR {
        played.step();
    }
    assert_eq!(run.world().tick(), HOUR);
    assert_eq!(run.world().state_hash(), played.state_hash());
}

#[test]
fn it_reports_ten_rows_each_a_tenth_of_the_way_on_from_where_it_started() {
    let mut world = lab_world(3);
    for _ in 0..123 {
        world.step();
    }
    let mut run = FastForward::new(world, 1);
    let mut due = Vec::new();
    while let Some(next) = run.next_row() {
        due.push(next);
        run.run_to(next);
        assert_eq!(run.world().tick(), next);
        assert_eq!(run.take_row().ticks_run, next - 123);
    }
    let tenths: Vec<u64> = (1..=10).map(|n| 123 + n * 450).collect();
    assert_eq!(due, tenths);
}

#[test]
fn it_never_runs_past_its_end() {
    let mut run = FastForward::new(lab_world(3), 1);
    run.run_to(u64::MAX);
    assert_eq!(run.world().tick(), HOUR);
    assert!(run.done());
}

#[test]
fn a_row_counts_each_life_stage_and_the_eggs() {
    let mut run = FastForward::new(lab_world(7), 1);
    run.run_to(450);
    let world = run.world();
    let stage = |stage| world.sprites().filter(|s| s.life_stage() == stage).count();
    let (young, adults, old) = (
        stage(LifeStage::Young),
        stage(LifeStage::Adult),
        stage(LifeStage::Old),
    );
    let eggs = world.eggs().count();
    let row = run.take_row();
    assert_eq!(
        (row.young, row.adults, row.old, row.eggs),
        (young, adults, old, eggs)
    );
    assert!(row.adults > 0, "a new world starts with grown sprites");
}

#[test]
fn the_rows_count_every_death_by_its_cause() {
    // Two sprites on a field with no food or water: one hurts the other to
    // death, and the other dies of thirst, within the hour.
    let data = DataPack::builtin().expect("valid pack");
    let map = Map::from_ascii(&[".........."; 6], &data).expect("valid drawing");
    let sprites = [(Pos { x: 2, y: 2 }, None), (Pos { x: 6, y: 3 }, None)];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
        newborns: &[],
    };
    let world = World::from_scenario(scenario, data, 1).expect("valid scenario");
    let mut run = FastForward::new(world, 1);
    let rows = run_through(&mut run);
    let mut counted: BTreeMap<DeathCause, u64> = BTreeMap::new();
    for row in &rows {
        for (cause, n) in &row.died {
            *counted.entry(*cause).or_insert(0) += n;
        }
    }
    let died: BTreeMap<DeathCause, u64> = run.world().deaths_by_cause().collect();
    assert_eq!(died.values().sum::<u64>(), 2, "both died");
    assert_eq!(counted, died);
    let rows_with_deaths = rows.iter().filter(|row| !row.died.is_empty()).count();
    assert_eq!(rows_with_deaths, 2, "each in the row it died by");
}

fn id(n: u64) -> EntityId {
    EntityId(n)
}

#[test]
fn an_egg_is_a_generation_on_from_the_deeper_of_its_parents() {
    let mut tally = Tally::default();
    tally.note(&EventKind::Spawned {
        id: id(1),
        pos: Pos { x: 0, y: 0 },
    });
    tally.note(&EventKind::Laid {
        egg: id(2),
        by: id(1),
        with: None,
    });
    assert_eq!(tally.deepest(), 1);
    tally.note(&EventKind::Laid {
        egg: id(3),
        by: id(1),
        with: Some(id(2)),
    });
    assert_eq!(tally.deepest(), 2, "the deeper parent counts");
    tally.note(&EventKind::Laid {
        egg: id(4),
        by: id(1),
        with: None,
    });
    assert_eq!(tally.deepest(), 2, "a shallower line doesn't lower it");
}

#[test]
fn a_parent_the_tally_never_saw_counts_as_the_first_sprites() {
    let mut tally = Tally::default();
    tally.note(&EventKind::Laid {
        egg: id(9),
        by: id(7),
        with: None,
    });
    assert_eq!(tally.deepest(), 1);
}

#[test]
fn deaths_count_by_cause_until_they_are_taken() {
    let mut tally = Tally::default();
    let died = |n, cause| EventKind::Died {
        id: id(n),
        name: None,
        cause,
        age: 100,
    };
    tally.note(&died(1, DeathCause::OldAge));
    tally.note(&died(2, DeathCause::OldAge));
    tally.note(&died(3, DeathCause::Dehydration));
    assert_eq!(
        tally.take_deaths(),
        BTreeMap::from([(DeathCause::Dehydration, 1), (DeathCause::OldAge, 2)])
    );
    assert_eq!(tally.take_deaths(), BTreeMap::new());
}

#[test]
fn a_row_lines_up_under_the_header() {
    let row = Row {
        ticks_run: 20 * HOUR,
        young: 21,
        adults: 61,
        old: 13,
        eggs: 2,
        generation: 8,
        died: BTreeMap::from([
            (DeathCause::OldAge, 1_112),
            (DeathCause::Starvation, 3),
            (DeathCause::Dehydration, 3),
            (DeathCause::HurtBy(1), 1),
            (DeathCause::HurtBy(4), 1),
        ]),
    };
    assert_eq!(
        [HEADER, &row.line()],
        [
            " Hours  Sprites  Young  Adults  Old  Eggs  Generation   Died  Old age  Hunger  Thirst  Hurt",
            "    20       97     21      61   13     2           8  1,120    1,112       3       3     2",
        ]
    );
}

#[test]
fn hours_are_to_the_nearest_tenth() {
    assert_eq!(hours(200 * HOUR), "200");
    assert_eq!(hours(HOUR / 10), "0.1");
    assert_eq!(hours(HOUR * 3 / 2), "1.5");
    assert_eq!(hours(10 * HOUR + 100), "10", "rounded to a whole hour");
    assert_eq!(hours(1_234 * HOUR + HOUR / 2), "1,234.5");
}

#[test]
fn times_are_said_in_the_largest_units_that_fit() {
    assert_eq!(spoken(Duration::from_secs(45)), "45 seconds");
    assert_eq!(spoken(Duration::from_secs(60)), "1 minute");
    assert_eq!(spoken(Duration::from_secs(13 * 60 + 20)), "13 minutes");
    assert_eq!(spoken(Duration::from_secs(3_600)), "1 hour");
    assert_eq!(
        spoken(Duration::from_secs(2 * 3_600 + 5 * 60)),
        "2 hours 5 minutes"
    );
}

#[test]
fn the_report_ends_by_saying_how_far_it_got_and_how_fast() {
    let took = Duration::from_secs(31 * 60);
    assert_eq!(
        fast_forward::closing_line(false, 200 * HOUR, took, 1_483.6),
        "Done: 200 hours in 31 minutes, 1,484 ticks a second."
    );
    assert_eq!(
        fast_forward::closing_line(true, HOUR, Duration::from_secs(9), 500.0),
        "Stopped at 1 hour, after 9 seconds, 500 ticks a second."
    );
}

#[test]
fn a_runs_save_is_named_after_its_world_and_the_hours_it_ran() {
    assert_eq!(save_name("seed-7", 200 * HOUR), "seed-7-200h");
    assert_eq!(save_name("mira", 1_200 * HOUR), "mira-1200h");
    assert_eq!(save_name("mira", 143 * HOUR + HOUR / 2), "mira-143.5h");
}

#[test]
fn the_report_opens_by_saying_which_world_it_runs_and_for_how_long() {
    let world = lab_world(7);
    assert_eq!(
        fast_forward::opening_line(&world, None, "default", 200),
        "Fast-forwarding a new Lab world, seed 7, default preset, for 200 hours (900,000 ticks)."
    );
    assert_eq!(
        fast_forward::opening_line(&world, Some("mira"), "default", 1),
        "Fast-forwarding mira, a Lab world at tick 0, for 1 hour (4,500 ticks)."
    );
}
