//! The debug overlay (`F3`, #311): how the game is running on this computer,
//! in a corner of the map view, for a screenshot when it feels slow.

mod common;
use common::{WorldBuilder, app_on, apply, at, lines, render, tick};

use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use terra_sim::World;
use terra_tui::app::{App, Screen};
use terra_tui::clock::Speed;
use terra_tui::debug_overlay::FrameTimes;
use terra_tui::input::{Action, KeyReports, Keys};

fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: KeyEventState::NONE,
    }
}

#[test]
fn f3_toggles_the_overlay_on_a_fresh_press_only() {
    let mut keys = Keys::with_release_reporting(true);
    assert_eq!(
        keys.action_for(key(KeyCode::F(3), KeyEventKind::Press)),
        Some(Action::DebugOverlay)
    );
    // A held `F3` would flicker it on and off.
    assert_eq!(
        keys.action_for(key(KeyCode::F(3), KeyEventKind::Repeat)),
        None
    );
}

#[test]
fn the_keys_say_whether_the_terminal_reports_releases() {
    assert_eq!(Keys::kitty().reports(), KeyReports::Kitty);
    assert_eq!(
        Keys::with_release_reporting(true).reports(),
        KeyReports::Releases
    );
    let mut keys = Keys::with_release_reporting(false);
    assert_eq!(keys.reports(), KeyReports::NoReleases);
    // A terminal not known to report releases is trusted once one arrives
    // (design v27 §6.6).
    keys.action_for(key(KeyCode::Char('w'), KeyEventKind::Press));
    keys.action_for(key(KeyCode::Char('w'), KeyEventKind::Release));
    assert_eq!(keys.reports(), KeyReports::Releases);
}

/// A 60×30 field of grass with a starter sprite on (2, 3): bigger than the
/// map view at 100×30, so it fills it.
fn field() -> World {
    WorldBuilder::grass(60, 30)
        .starters(&[at(2, 3)])
        .seed(7)
        .build()
}

/// The screen cells the overlay covers at 100×30, row by row: inside the
/// map view's border, at its top right, 29 wide and 6 high.
fn overlay_rows(app: &App, world: &World) -> Vec<String> {
    let screen = lines(&render(app, world));
    screen[2..8]
        .iter()
        .map(|row| row.chars().skip(24).take(29).collect())
        .collect()
}

/// Each of `lines` inside the overlay's border, as drawn.
fn boxed(lines: [&str; 4]) -> Vec<String> {
    let mut rows = vec![format!("┌─ Debug {} F3 ─┐", "─".repeat(14))];
    rows.extend(lines.iter().map(|line| format!("│{line:<27}│")));
    rows.push(format!("└{}┘", "─".repeat(27)));
    rows
}

/// `frames` frames of 40 ms, each running a tick until `ticks` have run,
/// and spending 1.2 ms on them and 3.4 ms drawing.
fn run_frames(app: &mut App, frames: u32, ticks: u32, keys: KeyReports) {
    for frame in 0..frames {
        let times = FrameTimes {
            elapsed: Duration::from_millis(40),
            ticks: u64::from(frame < ticks),
            simulating: Duration::from_micros(1_200),
            drawing: Duration::from_micros(3_400),
        };
        app.note_frame(times, keys);
    }
}

#[test]
fn f3_shows_the_overlay_in_the_map_view_s_top_right_corner_and_again_hides_it() {
    let world = field();
    let mut app = app_on(&world);
    let before = lines(&render(&app, &world));
    assert!(!app.debug_overlay(), "it starts hidden");

    apply(&mut app, &world, Action::DebugOverlay);
    assert!(app.debug_overlay());
    let rows = overlay_rows(&app, &world);
    assert_eq!(rows[0], boxed(["", "", "", ""])[0], "its top border");
    assert_eq!(rows[5], boxed(["", "", "", ""])[5], "its bottom border");
    let screen = lines(&render(&app, &world));
    assert!(
        screen[1].starts_with("┌─ Map ─"),
        "the map view's border stays: {}",
        screen[1]
    );
    assert_eq!(
        screen[2].chars().nth(53),
        Some('│'),
        "inside the map view's border"
    );

    apply(&mut app, &world, Action::DebugOverlay);
    assert!(!app.debug_overlay());
    assert_eq!(lines(&render(&app, &world)), before, "F3 again hides it");
}

#[test]
fn it_reads_a_second_of_frames_ticks_against_the_speed_asked_for() {
    let world = field();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::DebugOverlay);
    // 1× to 16×: 20 ticks a second.
    for _ in 0..4 {
        app.clock.faster();
    }
    // 25 frames of 40 ms are a second; 19 ticks ran in it.
    run_frames(&mut app, 25, 19, KeyReports::Releases);
    assert_eq!(
        overlay_rows(&app, &world),
        boxed([
            " ticks/s 19.0 of 20",
            " sim     1.2 of 25 ms",
            " draw    3.4 ms, 25 fps",
            " keys    releases reported",
        ])
    );
}

#[test]
fn until_a_second_of_frames_is_in_it_has_no_numbers() {
    let world = field();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::DebugOverlay);
    run_frames(&mut app, 24, 24, KeyReports::NoReleases);
    assert_eq!(
        overlay_rows(&app, &world),
        boxed([
            " ticks/s - of 1.25",
            " sim     - of 25 ms",
            " draw    -",
            " keys    no releases",
        ])
    );
}

#[test]
fn it_reads_again_each_second_from_that_second_s_frames_alone() {
    let world = field();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::DebugOverlay);
    run_frames(&mut app, 25, 25, KeyReports::Kitty);
    run_frames(&mut app, 25, 1, KeyReports::Kitty);
    assert_eq!(
        overlay_rows(&app, &world)[1],
        boxed([" ticks/s 1.0 of 1.25", "", "", ""])[1]
    );
    assert_eq!(
        overlay_rows(&app, &world)[4],
        boxed(["", "", "", " keys    kitty protocol"])[4]
    );
}

#[test]
fn paused_or_at_max_it_says_so_in_place_of_the_speed_asked_for() {
    let world = field();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::DebugOverlay);
    // ⅛×: 5/32 of a tick a second.
    for _ in 0..3 {
        app.clock.slower();
    }
    run_frames(&mut app, 25, 0, KeyReports::Releases);
    assert_eq!(
        overlay_rows(&app, &world)[1],
        boxed([" ticks/s 0.0 of 0.16", "", "", ""])[1]
    );

    app.clock.toggle_pause();
    assert_eq!(
        overlay_rows(&app, &world)[1],
        boxed([" ticks/s 0.0 paused", "", "", ""])[1]
    );

    app.clock.toggle_pause();
    while app.clock.speed() != Speed::Max {
        app.clock.faster();
    }
    let times = FrameTimes {
        elapsed: Duration::from_secs(1),
        ticks: 2_431,
        simulating: Duration::from_millis(25),
        drawing: Duration::from_millis(8),
    };
    app.note_frame(times, KeyReports::Releases);
    assert_eq!(
        overlay_rows(&app, &world)[1..4],
        boxed([
            " ticks/s 2,431 at Max",
            " sim     25.0 of 25 ms",
            " draw    8.0 ms, 1 fps",
            "",
        ])[1..4]
    );
}

#[test]
fn f3_shows_it_over_whatever_is_open_and_answers_no_prompt() {
    let world = field();
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Help);
    apply(&mut app, &world, Action::DebugOverlay);
    assert_eq!(app.screen(), Screen::Help, "help stays open");
    assert_eq!(overlay_rows(&app, &world)[0], boxed(["", "", "", ""])[0]);
    apply(&mut app, &world, Action::Back);

    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::QuitPrompt);
    apply(&mut app, &world, Action::DebugOverlay);
    assert_eq!(app.screen(), Screen::QuitPrompt, "the question still waits");
    assert!(!app.debug_overlay());
}

#[test]
fn the_overlay_never_touches_the_world() {
    let (mut shown, mut hidden) = (field(), field());
    let mut with = app_on(&shown);
    let mut without = app_on(&hidden);
    apply(&mut with, &shown, Action::DebugOverlay);
    for _ in 0..10 {
        run_frames(&mut with, 3, 3, KeyReports::Releases);
        tick(&mut with, &mut shown);
        tick(&mut without, &mut hidden);
    }
    assert_eq!(shown.state_hash(), hidden.state_hash());
}
