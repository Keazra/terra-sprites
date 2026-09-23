use std::time::Duration;

use terra_tui::clock::{Clock, Speed};

/// Advances `clock` through `total` time in 40 ms frames, never running out of
/// frame budget. Returns how many ticks ran.
fn run_for(clock: &mut Clock, total: Duration) -> u64 {
    let frame = Duration::from_millis(40);
    let mut ticks = 0;
    let mut remaining = total;
    while remaining >= frame {
        clock.advance(frame, || ticks += 1, || false);
        remaining -= frame;
    }
    ticks
}

#[test]
fn at_1x_one_second_of_frames_runs_ten_ticks() {
    let mut clock = Clock::new();
    assert_eq!(run_for(&mut clock, Duration::from_secs(1)), 10);
}

#[test]
fn faster_steps_through_every_speed_and_stops_at_max() {
    let mut clock = Clock::new();
    let mut seen = vec![clock.speed()];
    for _ in 0..6 {
        clock.faster();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [X1, X2, X4, X8, X16, Max, Max]);
}

#[test]
fn the_tick_rate_follows_the_speed() {
    let mut clock = Clock::new();
    clock.faster();
    clock.faster();
    assert_eq!(clock.speed(), Speed::X4);
    assert_eq!(run_for(&mut clock, Duration::from_secs(1)), 40);
}

#[test]
fn slower_steps_back_down_and_stops_at_1x() {
    let mut clock = Clock::new();
    for _ in 0..5 {
        clock.faster();
    }
    let mut seen = vec![clock.speed()];
    for _ in 0..6 {
        clock.slower();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [Max, X16, X8, X4, X2, X1, X1]);
}

#[test]
fn a_paused_clock_runs_nothing_and_builds_no_backlog() {
    let mut clock = Clock::new();
    clock.toggle_pause();
    assert!(clock.is_paused());
    assert_eq!(run_for(&mut clock, Duration::from_secs(1)), 0);

    clock.toggle_pause();
    assert!(!clock.is_paused());
    assert_eq!(run_for(&mut clock, Duration::from_secs(1)), 10);
}

#[test]
fn stepping_while_paused_runs_exactly_one_tick() {
    let mut clock = Clock::new();
    clock.toggle_pause();
    clock.step_once();
    assert_eq!(run_for(&mut clock, Duration::from_secs(1)), 1);
    assert_eq!(
        run_for(&mut clock, Duration::from_secs(1)),
        0,
        "a step is used once"
    );
}

#[test]
fn stepping_while_running_does_nothing_extra() {
    let mut clock = Clock::new();
    clock.step_once();
    assert_eq!(run_for(&mut clock, Duration::from_secs(1)), 10);
}

#[test]
fn a_frame_cut_short_by_the_budget_drops_its_backlog() {
    let mut clock = Clock::new();
    for _ in 0..4 {
        clock.faster();
    }
    assert_eq!(clock.speed(), Speed::X16); // 160 ticks per second

    // A slow frame: a whole second is due, but the budget runs out after 3 ticks.
    let ticks = std::cell::Cell::new(0);
    let ran = clock.advance(
        Duration::from_secs(1),
        || ticks.set(ticks.get() + 1),
        || ticks.get() >= 3,
    );
    assert_eq!(ran, 3);

    // The next 40 ms frame runs its own 6 ticks (6.4 due), not the 157 left over.
    assert_eq!(run_for(&mut clock, Duration::from_millis(40)), 6);
}

#[test]
fn at_max_speed_a_frame_runs_until_its_budget_is_spent() {
    let mut clock = Clock::new();
    for _ in 0..5 {
        clock.faster();
    }
    assert_eq!(clock.speed(), Speed::Max);

    let ticks = std::cell::Cell::new(0);
    let ran = clock.advance(
        Duration::from_millis(40),
        || ticks.set(ticks.get() + 1),
        || ticks.get() >= 500,
    );
    assert_eq!(ran, 500);
}
