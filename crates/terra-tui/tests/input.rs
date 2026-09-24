use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use terra_tui::input::{Action, Keys};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
}

#[test]
fn time_control_keys_map_to_their_actions() {
    let cases = [
        (press(KeyCode::Char(' ')), Some(Action::TogglePause)),
        (press(KeyCode::Char('.')), Some(Action::StepOnce)),
        (
            press(KeyCode::Char('+')),
            Some(Action::Faster { held: false }),
        ),
        (
            press(KeyCode::Char('=')),
            Some(Action::Faster { held: false }),
        ), // `+` without Shift
        (
            press(KeyCode::Char('-')),
            Some(Action::Slower { held: false }),
        ),
        (press(KeyCode::Char('q')), Some(Action::Quit)),
        (
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Action::Quit),
        ),
        (press(KeyCode::Char('x')), None),
    ];
    for (key, expected) in cases {
        assert_eq!(Keys::new().action_for(key), expected, "{key:?}");
    }
}

#[test]
fn key_releases_are_ignored() {
    // Windows reports releases as well as presses; acting on both would double every action.
    let release = kind(KeyCode::Char(' '), KeyEventKind::Release);
    assert_eq!(Keys::new().action_for(release), None);
}

#[test]
fn a_key_the_terminal_reports_as_repeating_is_held() {
    let mut keys = Keys::new();
    assert_eq!(
        keys.action_for(press(KeyCode::Char('-'))),
        Some(Action::Slower { held: false })
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Char('-'), KeyEventKind::Repeat)),
        Some(Action::Slower { held: true })
    );
}

#[test]
fn where_releases_are_reported_a_second_press_without_a_release_is_held() {
    // Windows reports auto-repeat as more presses, but always reports releases.
    let mut keys = Keys::with_release_reporting(true);
    let minus = KeyCode::Char('-');
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: false })
    );
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: true })
    );
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: true })
    );
    assert_eq!(keys.action_for(kind(minus, KeyEventKind::Release)), None);
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: false })
    );
}

#[test]
fn where_releases_are_not_reported_repeated_presses_stay_fresh() {
    // Without releases, a hold can't be told from taps, so never guess "held".
    let mut keys = Keys::with_release_reporting(false);
    for _ in 0..3 {
        assert_eq!(
            keys.action_for(press(KeyCode::Char('-'))),
            Some(Action::Slower { held: false })
        );
    }
}

#[test]
fn seeing_a_release_turns_on_held_detection() {
    let mut keys = Keys::with_release_reporting(false);
    let plus = KeyCode::Char('+');
    keys.action_for(press(plus));
    keys.action_for(kind(plus, KeyEventKind::Release)); // this terminal reports releases after all
    assert_eq!(
        keys.action_for(press(plus)),
        Some(Action::Faster { held: false })
    );
    assert_eq!(
        keys.action_for(press(plus)),
        Some(Action::Faster { held: true })
    );
}

#[test]
fn holding_space_toggles_pause_only_once() {
    let mut keys = Keys::with_release_reporting(true);
    let space = KeyCode::Char(' ');
    assert_eq!(keys.action_for(press(space)), Some(Action::TogglePause));
    assert_eq!(
        keys.action_for(press(space)),
        None,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(space, KeyEventKind::Repeat)),
        None,
        "held via repeat"
    );
}

#[test]
fn holding_step_keeps_stepping() {
    let mut keys = Keys::with_release_reporting(true);
    let dot = KeyCode::Char('.');
    assert_eq!(keys.action_for(press(dot)), Some(Action::StepOnce));
    assert_eq!(
        keys.action_for(press(dot)),
        Some(Action::StepOnce),
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(dot, KeyEventKind::Repeat)),
        Some(Action::StepOnce),
        "held via repeat"
    );
}

#[test]
fn a_plus_released_as_equals_is_still_released() {
    // Releasing Shift before the key makes Windows report the release as `=`.
    let mut keys = Keys::with_release_reporting(true);
    assert_eq!(
        keys.action_for(press(KeyCode::Char('+'))),
        Some(Action::Faster { held: false })
    );
    keys.action_for(kind(KeyCode::Char('='), KeyEventKind::Release));
    assert_eq!(
        keys.action_for(press(KeyCode::Char('+'))),
        Some(Action::Faster { held: false })
    );
}

#[test]
fn arrows_and_hjkl_move_the_cursor_one_tile() {
    let one = |dx, dy| Some(Action::MoveCursor { dx, dy });
    let cases = [
        (KeyCode::Left, one(-1, 0)),
        (KeyCode::Right, one(1, 0)),
        (KeyCode::Up, one(0, -1)),
        (KeyCode::Down, one(0, 1)),
        (KeyCode::Char('h'), one(-1, 0)),
        (KeyCode::Char('l'), one(1, 0)),
        (KeyCode::Char('k'), one(0, -1)),
        (KeyCode::Char('j'), one(0, 1)),
    ];
    for (code, expected) in cases {
        assert_eq!(Keys::new().action_for(press(code)), expected, "{code:?}");
    }
}

#[test]
fn shift_moves_the_cursor_five_tiles() {
    let five = |dx, dy| Some(Action::MoveCursor { dx, dy });
    let shifted = |code| KeyEvent::new(code, KeyModifiers::SHIFT);
    let cases = [
        (shifted(KeyCode::Left), five(-5, 0)),
        (shifted(KeyCode::Right), five(5, 0)),
        (shifted(KeyCode::Up), five(0, -5)),
        (shifted(KeyCode::Down), five(0, 5)),
        // Terminals report Shift+h as `H`, with or without the Shift modifier.
        (shifted(KeyCode::Char('H')), five(-5, 0)),
        (press(KeyCode::Char('L')), five(5, 0)),
        (shifted(KeyCode::Char('K')), five(0, -5)),
        (press(KeyCode::Char('J')), five(0, 5)),
    ];
    for (key, expected) in cases {
        assert_eq!(Keys::new().action_for(key), expected, "{key:?}");
    }
}

#[test]
fn holding_a_cursor_key_keeps_moving() {
    let mut keys = Keys::with_release_reporting(true);
    let right = Some(Action::MoveCursor { dx: 1, dy: 0 });
    assert_eq!(keys.action_for(press(KeyCode::Right)), right);
    assert_eq!(
        keys.action_for(press(KeyCode::Right)),
        right,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Right, KeyEventKind::Repeat)),
        right,
        "held via repeat"
    );
}

#[test]
fn a_left_click_is_a_click_at_that_cell_and_other_mouse_events_are_ignored() {
    use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mouse = |kind| MouseEvent {
        kind,
        column: 12,
        row: 7,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        terra_tui::input::mouse_action(mouse(MouseEventKind::Down(MouseButton::Left))),
        Some(Action::Click { column: 12, row: 7 })
    );
    for kind in [
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Moved,
        MouseEventKind::ScrollDown,
    ] {
        assert_eq!(
            terra_tui::input::mouse_action(mouse(kind)),
            None,
            "{kind:?}"
        );
    }
}
