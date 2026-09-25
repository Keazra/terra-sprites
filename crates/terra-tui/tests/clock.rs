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
fn at_1x_eight_seconds_of_frames_run_ten_ticks() {
    // 1× is 1.25 ticks a second, slow enough to watch a sprite walk.
    let mut clock = Clock::new();
    assert_eq!(run_for(&mut clock, Duration::from_secs(8)), 10);
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
    assert_eq!(run_for(&mut clock, Duration::from_secs(2)), 10);
}

#[test]
fn slower_halves_the_speed_down_to_an_eighth_and_stops() {
    let mut clock = Clock::new();
    for _ in 0..5 {
        clock.faster();
    }
    let mut seen = vec![clock.speed()];
    for _ in 0..9 {
        clock.slower();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(
        seen,
        [Max, X16, X8, X4, X2, X1, Half, Quarter, Eighth, Eighth]
    );
}

#[test]
fn a_paused_clock_runs_nothing_and_builds_no_backlog() {
    let mut clock = Clock::new();
    clock.toggle_pause();
    assert!(clock.is_paused());
    assert_eq!(run_for(&mut clock, Duration::from_secs(8)), 0);

    clock.toggle_pause();
    assert!(!clock.is_paused());
    assert_eq!(run_for(&mut clock, Duration::from_secs(8)), 10);
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
    assert_eq!(run_for(&mut clock, Duration::from_secs(8)), 10);
}

#[test]
fn a_frame_cut_short_by_the_budget_drops_its_backlog() {
    let mut clock = Clock::new();
    for _ in 0..4 {
        clock.faster();
    }
    assert_eq!(clock.speed(), Speed::X16); // 20 ticks per second

    // A slow frame: a whole second is due, but the budget runs out after 3 ticks.
    let ticks = std::cell::Cell::new(0);
    let ran = clock.advance(
        Duration::from_secs(1),
        || ticks.set(ticks.get() + 1),
        || ticks.get() >= 3,
    );
    assert_eq!(ran, 3);

    // The next 200 ms of frames run their own 4 ticks, not the 17 left over.
    assert_eq!(run_for(&mut clock, Duration::from_millis(200)), 4);
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

#[test]
fn an_eighth_speed_runs_exactly_ten_ticks_in_64_seconds() {
    let mut clock = Clock::new();
    for _ in 0..3 {
        clock.slower();
    }
    assert_eq!(clock.speed(), Speed::Eighth); // 5/32 of a tick a second
    assert_eq!(run_for(&mut clock, Duration::from_secs(64)), 10);
}

#[test]
fn faster_climbs_back_up_from_an_eighth() {
    let mut clock = Clock::new();
    for _ in 0..3 {
        clock.slower();
    }
    let mut seen = vec![clock.speed()];
    for _ in 0..3 {
        clock.faster();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [Eighth, Quarter, Half, X1]);
}

#[test]
fn holding_slower_stops_at_1x_until_pressed_again() {
    let mut clock = Clock::new();
    for _ in 0..4 {
        clock.faster();
    }
    assert_eq!(clock.speed(), Speed::X16);

    let mut seen = vec![];
    for _ in 0..6 {
        clock.slower_held();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [X8, X4, X2, X1, X1, X1]);

    clock.slower();
    assert_eq!(clock.speed(), Half);
}

#[test]
fn holding_faster_stops_at_1x_until_pressed_again() {
    let mut clock = Clock::new();
    for _ in 0..3 {
        clock.slower();
    }
    assert_eq!(clock.speed(), Speed::Eighth);

    let mut seen = vec![];
    for _ in 0..5 {
        clock.faster_held();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [Quarter, Half, X1, X1, X1]);

    clock.faster();
    assert_eq!(clock.speed(), X2);
}

#[test]
fn holding_continues_past_1x_once_a_fresh_press_has_crossed_it() {
    let mut clock = Clock::new();
    clock.slower(); // a fresh press crosses 1×
    clock.slower_held();
    clock.slower_held();
    assert_eq!(clock.speed(), Speed::Eighth);

    let mut clock = Clock::new();
    clock.faster(); // a fresh press crosses 1×
    clock.faster_held();
    assert_eq!(clock.speed(), Speed::X4);
}

#[test]
fn holding_faster_stops_at_16x_so_max_takes_a_fresh_press() {
    let mut clock = Clock::new();
    clock.faster(); // a fresh press crosses 1×
    assert_eq!(clock.speed(), Speed::X2);

    let mut seen = vec![];
    for _ in 0..5 {
        clock.faster_held();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [X4, X8, X16, X16, X16]);

    clock.faster();
    assert_eq!(clock.speed(), Max);
}

#[test]
fn holding_slower_from_max_passes_16x_and_stops_at_1x() {
    let mut clock = Clock::new();
    for _ in 0..5 {
        clock.faster();
    }
    assert_eq!(clock.speed(), Speed::Max);

    let mut seen = vec![];
    for _ in 0..6 {
        clock.slower_held();
        seen.push(clock.speed());
    }
    use Speed::*;
    assert_eq!(seen, [X16, X8, X4, X2, X1, X1]);
}
