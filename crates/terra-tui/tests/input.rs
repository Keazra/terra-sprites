use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use terra_tui::input::{Action, action_for};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn time_control_keys_map_to_their_actions() {
    let cases = [
        (press(KeyCode::Char(' ')), Some(Action::TogglePause)),
        (press(KeyCode::Char('.')), Some(Action::StepOnce)),
        (press(KeyCode::Char('+')), Some(Action::Faster)),
        (press(KeyCode::Char('=')), Some(Action::Faster)), // `+` without Shift
        (press(KeyCode::Char('-')), Some(Action::Slower)),
        (press(KeyCode::Char('q')), Some(Action::Quit)),
        (
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Action::Quit),
        ),
        (press(KeyCode::Char('x')), None),
    ];
    for (key, expected) in cases {
        assert_eq!(action_for(key), expected, "{key:?}");
    }
}

#[test]
fn key_releases_are_ignored() {
    // Windows reports releases as well as presses; acting on both would double every action.
    let release = KeyEvent::new_with_kind(
        KeyCode::Char(' '),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    );
    assert_eq!(action_for(release), None);
}
