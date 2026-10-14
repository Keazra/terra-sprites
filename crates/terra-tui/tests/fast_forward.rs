//! Fast-forward (M2 design §16): a Lab world run with no screen for hours
//! of play ends exactly as the game would have left it, reporting how it
//! went a tenth of the way at a time: the life stages, the deepest
//! generation and the deaths by cause.

use std::collections::BTreeMap;
use std::time::Duration;

use terra_sim::{
    DataPack, DeathCause, EntityId, EventKind, LifeStage, Map, Mode, Pos, Scenario, World,
    WorldConfig, gene_numbers, middle_numbers,
};
use terra_tui::fast_forward::{
    self, FastForward, HEADER, HOUR, Moved, Row, Tally, hours, save_name, spoken,
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

/// Two sprites on a field with no food or water: one hurts the other to
/// death, and the other dies of thirst, within the hour.
fn a_deadly_field() -> World {
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
    World::from_scenario(scenario, data, 1).expect("valid scenario")
}

#[test]
fn the_rows_count_every_death_by_its_cause() {
    let mut run = FastForward::new(a_deadly_field(), 1);
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

/// The cells of a table, without the mark at its start that tells a
/// spreadsheet it's UTF-8.
fn cells(table: &str) -> Vec<Vec<String>> {
    let table = table.strip_prefix('\u{feff}').expect("marked as UTF-8");
    table
        .lines()
        .map(|line| line.split(',').map(str::to_string).collect())
        .collect()
}

#[test]
fn the_population_table_has_a_row_for_each_count_and_one_for_the_end() {
    // Counts are taken every 100 ticks at first (M2 design §14), so the
    // run stops between two.
    let mut run = FastForward::new(lab_world(7), 2);
    run.run_to(HOUR + 50);
    let world = run.world();
    let table = cells(&fast_forward::population_table(world));
    assert_eq!(
        table[0],
        [
            "Hours",
            "Tick",
            "Eggs",
            "Young",
            "Adults",
            "Old",
            "Laid",
            "Hatched",
            "Spawned",
            "Died",
            "Starvation",
            "Dehydration",
            "Old age"
        ]
    );
    let counts = world.population();
    assert_eq!(
        table.len(),
        1 + counts.len() + 1,
        "the header, the counts and now"
    );
    let first = &counts[0];
    assert_eq!(
        table[1][..6],
        [
            "0",
            "0",
            &first.eggs.to_string(),
            &first.young.to_string(),
            &first.adults.to_string(),
            &first.old.to_string()
        ]
    );
    assert_eq!(table[2][..2], ["0.02", "100"]);
    let now = world.population_now();
    let last = table.last().unwrap();
    assert_eq!(last[..2], ["1.01", "4550"]);
    assert_eq!(last[3], now.young.to_string());
    let laid: u32 = table[1..]
        .iter()
        .map(|row| row[6].parse::<u32>().unwrap())
        .sum();
    let laid_events = counts.iter().map(|c| c.since.laid).sum::<u32>() + now.since.laid;
    assert_eq!(laid, laid_events);
}

#[test]
fn a_population_table_that_ends_on_a_count_ends_there() {
    let mut run = FastForward::new(lab_world(7), 1);
    run_through(&mut run);
    let world = run.world();
    let table = cells(&fast_forward::population_table(world));
    assert_eq!(table.len(), 1 + world.population().len());
    assert_eq!(table.last().unwrap()[..2], ["1", "4500"]);
}

#[test]
fn the_population_table_names_each_cause_of_death_the_world_has_seen() {
    let mut run = FastForward::new(a_deadly_field(), 1);
    run_through(&mut run);
    let world = run.world();
    let table = cells(&fast_forward::population_table(world));
    let hurt = table[0]
        .iter()
        .position(|h| h.starts_with("Hurt by "))
        .expect("a hurt column");
    let thirst = table[0].iter().position(|h| h == "Dehydration").unwrap();
    let total = |column: usize| -> u64 {
        table[1..]
            .iter()
            .map(|row| row[column].parse::<u64>().unwrap())
            .sum()
    };
    let died: BTreeMap<DeathCause, u64> = world.deaths_by_cause().collect();
    assert_eq!(total(thirst), died[&DeathCause::Dehydration]);
    let hurt_by: u64 = died
        .iter()
        .filter(|(c, _)| matches!(c, DeathCause::HurtBy(_)))
        .map(|(_, n)| n)
        .sum();
    assert_eq!(total(hurt), hurt_by);
}

#[test]
fn the_genes_table_starts_with_the_starter_then_has_a_row_an_hour() {
    let world = lab_world(7);
    let starter = gene_numbers(&world.data().starter().genes(world.data()));
    let at_start = middle_numbers(&world, &starter).expect("sprites alive");
    let mut run = FastForward::new(world, 2);
    run_through(&mut run);
    let table = cells(&run.genes().table());
    let names: Vec<&str> = starter.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(table[0][..3], ["Hours", "Tick", "Sprites"]);
    assert_eq!(table[0][3..], names);
    assert_eq!(table[1][..3], ["starter", "", ""]);
    assert_eq!(table[1][3], starter[0].1.to_string());
    let hours: Vec<&str> = table[2..].iter().map(|row| row[0].as_str()).collect();
    assert_eq!(hours, ["0", "1", "2"]);
    assert_eq!(table[2][1], "0");
    assert_eq!(table[2][2], lab_world(7).sprites().count().to_string());
    let middle = |n: usize| at_start[n].map_or(String::new(), |v| v.to_string());
    assert_eq!(table[2][3..6], [middle(0), middle(1), middle(2)]);
}

#[test]
fn a_run_stopped_between_hours_ends_its_genes_table_where_it_stopped() {
    let mut run = FastForward::new(lab_world(7), 2);
    run.run_to(HOUR + 300);
    run.finish();
    let table = cells(&run.genes().table());
    let ticks: Vec<&str> = table[2..].iter().map(|row| row[1].as_str()).collect();
    assert_eq!(ticks, ["0", "4500", "4800"]);
    run.finish();
    assert_eq!(cells(&run.genes().table()).len(), 5, "only once");
}

#[test]
fn the_numbers_that_moved_most_come_first_halving_as_far_as_doubling() {
    let starter = [
        ("a".to_string(), 1.0),
        ("b".to_string(), 2.0),
        ("never set".to_string(), 0.0),
        ("lost".to_string(), 4.0),
        ("c".to_string(), 1.0),
        ("still".to_string(), 3.0),
    ];
    let now = [Some(1.5), Some(1.0), Some(5.0), None, Some(2.0), Some(3.05)];
    let moved = fast_forward::moved_most(&starter, &now, 5);
    let names: Vec<&str> = moved.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["b", "c", "a"], "only those a tenth or more from it");
    assert_eq!(
        moved[0],
        Moved {
            name: "b".into(),
            starter: 2.0,
            now: 1.0
        }
    );
    assert_eq!(
        moved[0].line(10),
        "  b                  2 → 1         down 50%"
    );
    assert_eq!(
        moved[1].line(10),
        "  c                  1 → 2         up 100%"
    );
    assert_eq!(fast_forward::moved_most(&starter, &now, 1).len(), 1);
}
