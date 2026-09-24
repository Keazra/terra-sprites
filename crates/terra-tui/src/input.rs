//! Maps keys to UI actions (design §6.5–6.6), telling held keys from fresh presses.

use std::collections::HashSet;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

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
    /// Move the cursor by this many tiles.
    MoveCursor {
        dx: i32,
        dy: i32,
    },
    /// A left click on this screen cell.
    Click {
        column: u16,
        row: u16,
    },
    Quit,
}

/// How far Shift moves the cursor, in tiles (design §6.5).
const SHIFT_STEP: i32 = 5;

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
        let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
            SHIFT_STEP
        } else {
            1
        };
        let move_by = |dx: i32, dy: i32| Some(Action::MoveCursor { dx, dy });
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => move_by(-step, 0),
            KeyCode::Right | KeyCode::Char('l') => move_by(step, 0),
            KeyCode::Up | KeyCode::Char('k') => move_by(0, -step),
            KeyCode::Down | KeyCode::Char('j') => move_by(0, step),
            KeyCode::Char('H') => move_by(-SHIFT_STEP, 0),
            KeyCode::Char('L') => move_by(SHIFT_STEP, 0),
            KeyCode::Char('K') => move_by(0, -SHIFT_STEP),
            KeyCode::Char('J') => move_by(0, SHIFT_STEP),
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

/// The action for a mouse event, if it has one: only a left-button press acts.
pub fn mouse_action(event: MouseEvent) -> Option<Action> {
    (event.kind == MouseEventKind::Down(MouseButton::Left)).then_some(Action::Click {
        column: event.column,
        row: event.row,
    })
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
