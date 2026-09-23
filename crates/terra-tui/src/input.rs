//! Maps keys to UI actions (design §6.5–6.6), telling held keys from fresh presses.

use std::collections::HashSet;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Something the player asked the UI to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TogglePause,
    StepOnce,
    /// `held` is true when the key is auto-repeating rather than freshly pressed.
    Faster {
        held: bool,
    },
    Slower {
        held: bool,
    },
    Quit,
}

/// Turns key events into actions, remembering enough to recognise held keys.
///
/// A key is *held* when the terminal reports it as repeating, or when it is
/// pressed again without a release in between. The second rule is only trusted
/// where the terminal reports releases (Windows always does; others are trusted
/// once a release arrives), otherwise every press would look held.
#[derive(Debug)]
pub struct Keys {
    releases_reported: bool,
    down: HashSet<KeyCode>,
}

impl Keys {
    /// A tracker assuming what this platform's terminals do: Windows reports releases.
    pub fn new() -> Keys {
        Keys::with_release_reporting(cfg!(windows))
    }

    /// A tracker told whether the terminal reports key releases.
    pub fn with_release_reporting(releases_reported: bool) -> Keys {
        Keys {
            releases_reported,
            down: HashSet::new(),
        }
    }

    /// The action for a key event, if it has one. Key releases never act.
    pub fn action_for(&mut self, key: KeyEvent) -> Option<Action> {
        let physical = physical_key(key.code);
        if key.kind == KeyEventKind::Release {
            self.releases_reported = true;
            self.down.remove(&physical);
            return None;
        }
        let pressed_again = !self.down.insert(physical);
        let held = key.kind == KeyEventKind::Repeat || (self.releases_reported && pressed_again);
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return (key.code == KeyCode::Char('c')).then_some(Action::Quit);
        }
        match key.code {
            // Holding space would flicker pause on and off, so only a fresh press toggles.
            KeyCode::Char(' ') => (!held).then_some(Action::TogglePause),
            KeyCode::Char('.') => Some(Action::StepOnce),
            KeyCode::Char('+' | '=') => Some(Action::Faster { held }),
            KeyCode::Char('-') => Some(Action::Slower { held }),
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }
}

/// Identifies the physical key behind a character, so a key pressed with Shift
/// and released after Shift (reported as the unshifted character) still counts
/// as released. Uses US-layout pairs for the keys the game binds.
fn physical_key(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char('+') => KeyCode::Char('='),
        KeyCode::Char('_') => KeyCode::Char('-'),
        KeyCode::Char('>') => KeyCode::Char('.'),
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    }
}

impl Default for Keys {
    fn default() -> Self {
        Keys::new()
    }
}
