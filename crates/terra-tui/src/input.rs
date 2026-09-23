//! Maps keys to UI actions (design §6.5–6.6).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Something the player asked the UI to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TogglePause,
    StepOnce,
    Faster,
    Slower,
    Quit,
}

/// The action for a key event, if it has one. Key releases never act.
pub fn action_for(key: KeyEvent) -> Option<Action> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return (key.code == KeyCode::Char('c')).then_some(Action::Quit);
    }
    match key.code {
        KeyCode::Char(' ') => Some(Action::TogglePause),
        KeyCode::Char('.') => Some(Action::StepOnce),
        KeyCode::Char('+' | '=') => Some(Action::Faster),
        KeyCode::Char('-') => Some(Action::Slower),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}
