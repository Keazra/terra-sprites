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
        (
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Action::Quit),
        ),
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
fn wasd_and_the_arrows_scroll_the_view_one_tile() {
    let one = |dx, dy| Some(Action::Scroll { dx, dy });
    let cases = [
        (KeyCode::Char('w'), one(0, -1)),
        (KeyCode::Char('a'), one(-1, 0)),
        (KeyCode::Char('s'), one(0, 1)),
        (KeyCode::Char('d'), one(1, 0)),
        (KeyCode::Up, one(0, -1)),
        (KeyCode::Left, one(-1, 0)),
        (KeyCode::Down, one(0, 1)),
        (KeyCode::Right, one(1, 0)),
    ];
    for (code, expected) in cases {
        assert_eq!(Keys::new().action_for(press(code)), expected, "{code:?}");
    }
}

#[test]
fn shift_scrolls_five_tiles() {
    let five = |dx, dy| Some(Action::Scroll { dx, dy });
    let shifted = |code| KeyEvent::new(code, KeyModifiers::SHIFT);
    let cases = [
        (shifted(KeyCode::Up), five(0, -5)),
        (shifted(KeyCode::Left), five(-5, 0)),
        (shifted(KeyCode::Down), five(0, 5)),
        (shifted(KeyCode::Right), five(5, 0)),
        // Terminals report Shift+w as `W`, with or without the Shift modifier.
        (shifted(KeyCode::Char('W')), five(0, -5)),
        (press(KeyCode::Char('A')), five(-5, 0)),
        (shifted(KeyCode::Char('S')), five(0, 5)),
        (press(KeyCode::Char('D')), five(5, 0)),
    ];
    for (key, expected) in cases {
        assert_eq!(Keys::new().action_for(key), expected, "{key:?}");
    }
}

#[test]
fn holding_a_scroll_key_keeps_scrolling() {
    let mut keys = Keys::with_release_reporting(true);
    let right = Some(Action::Scroll { dx: 1, dy: 0 });
    assert_eq!(keys.action_for(press(KeyCode::Char('d'))), right);
    assert_eq!(
        keys.action_for(press(KeyCode::Char('d'))),
        right,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Char('d'), KeyEventKind::Repeat)),
        right,
        "held via repeat"
    );
}

#[test]
fn escape_and_y_answer_the_quit_prompt_and_q_no_longer_quits() {
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Esc)),
        Some(Action::Escape)
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('y'))),
        Some(Action::Yes)
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('q'))),
        Some(Action::OtherKey)
    );
}

#[test]
fn holding_escape_counts_once_so_it_cannot_confirm_its_own_prompt() {
    let mut keys = Keys::with_release_reporting(true);
    assert_eq!(keys.action_for(press(KeyCode::Esc)), Some(Action::Escape));
    assert_eq!(
        keys.action_for(press(KeyCode::Esc)),
        None,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Esc, KeyEventKind::Repeat)),
        None,
        "held via repeat"
    );
}

#[test]
fn any_other_key_is_reported_so_it_can_cancel_a_prompt() {
    for code in [
        KeyCode::Char('k'),
        KeyCode::Char('n'),
        KeyCode::Enter,
        KeyCode::F(5),
    ] {
        assert_eq!(
            Keys::new().action_for(press(code)),
            Some(Action::OtherKey),
            "{code:?}"
        );
    }
}

#[test]
fn the_mouse_points_while_moving_and_clicks_with_the_left_button() {
    use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mouse = |kind| MouseEvent {
        kind,
        column: 12,
        row: 7,
        modifiers: KeyModifiers::NONE,
    };
    let action = |kind| terra_tui::input::mouse_action(mouse(kind));
    let point = Some(Action::Point { column: 12, row: 7 });
    assert_eq!(action(MouseEventKind::Moved), point);
    assert_eq!(action(MouseEventKind::Drag(MouseButton::Left)), point);
    assert_eq!(
        action(MouseEventKind::Down(MouseButton::Left)),
        Some(Action::Click { column: 12, row: 7 })
    );
    for kind in [
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::ScrollDown,
    ] {
        assert_eq!(action(kind), None, "{kind:?}");
    }
}
