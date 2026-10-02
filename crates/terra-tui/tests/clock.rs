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
fn stepping_while_paused_at_1x_runs_one_tick() {
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

/// A paused clock at `speed`, reached from 1× with fresh presses.
fn paused_at(speed: Speed) -> Clock {
    let mut clock = Clock::new();
    clock.toggle_pause();
    let below_1x = matches!(speed, Speed::Eighth | Speed::Quarter | Speed::Half);
    for _ in 0..5 {
        if clock.speed() == speed {
            break;
        }
        if below_1x {
            clock.slower();
        } else {
            clock.faster();
        }
    }
    assert_eq!(clock.speed(), speed);
    clock
}

/// One frame whose budget runs out after `ticks` ticks. Returns how many ran.
fn advance_cut_after(clock: &mut Clock, ticks: u64) -> u64 {
    let ran = std::cell::Cell::new(0);
    clock.advance(
        Duration::from_millis(40),
        || ran.set(ran.get() + 1),
        || ran.get() >= ticks,
    )
}

#[test]
fn a_step_runs_one_seconds_worth_of_ticks_at_the_speed_and_at_least_one() {
    // Design v27 §6.6: 1× is 1.25 ticks a second, rounded down to 1; Max
    // steps as 16× does.
    use Speed::*;
    for (speed, ticks) in [
        (Eighth, 1),
        (Quarter, 1),
        (Half, 1),
        (X1, 1),
        (X2, 2),
        (X4, 5),
        (X8, 10),
        (X16, 20),
        (Max, 20),
    ] {
        let mut clock = paused_at(speed);
        clock.step_once();
        assert_eq!(
            run_for(&mut clock, Duration::from_secs(4)),
            ticks,
            "{speed:?}"
        );
    }
}

#[test]
fn a_step_cut_short_by_the_frame_budget_finishes_on_the_next_frames() {
    let mut clock = paused_at(Speed::X16);
    clock.step_once();
    assert_eq!(advance_cut_after(&mut clock, 3), 3);
    assert_eq!(run_for(&mut clock, Duration::from_secs(4)), 17);
}

#[test]
fn stepping_again_mid_step_tops_it_up_rather_than_adding_another() {
    // A held `.` repeats faster than a long step runs, so presses don't pile up.
    let mut clock = paused_at(Speed::X16);
    clock.step_once();
    assert_eq!(advance_cut_after(&mut clock, 3), 3);
    clock.step_once();
    clock.step_once();
    assert_eq!(run_for(&mut clock, Duration::from_secs(4)), 20);
}

#[test]
fn a_step_after_slowing_down_mid_step_runs_the_new_speed_s_step() {
    // The top bar shows the new speed, so the step must match it.
    let mut clock = paused_at(Speed::X16);
    clock.step_once();
    assert_eq!(advance_cut_after(&mut clock, 3), 3);
    for _ in 0..4 {
        clock.slower();
    }
    assert_eq!(clock.speed(), Speed::X1);
    clock.step_once();
    assert_eq!(run_for(&mut clock, Duration::from_secs(4)), 1);
}

#[test]
fn resuming_drops_what_is_left_of_a_step() {
    let mut clock = paused_at(Speed::X16);
    clock.step_once();
    advance_cut_after(&mut clock, 1);
    clock.toggle_pause();
    clock.toggle_pause();
    assert_eq!(run_for(&mut clock, Duration::from_secs(4)), 0);
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
