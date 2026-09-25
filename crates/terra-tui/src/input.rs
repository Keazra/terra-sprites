//! Maps keys and the mouse to UI actions (design §6.5–6.6), telling held keys
//! from fresh presses.

use std::collections::HashSet;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;

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
    /// Scroll the viewport by this many tiles.
    Scroll {
        dx: i32,
        dy: i32,
    },
    /// The mouse pointer is over this screen cell.
    Point(Position),
    /// A left click on this screen cell.
    Click(Position),
    /// Select the sprite with the next ID (`Tab`).
    SelectNext,
    /// Select the sprite with the previous ID (`Shift+Tab`).
    SelectPrevious,
    /// Open the next inspector tab (`]`).
    NextTab,
    /// Open the previous inspector tab (`[`).
    PreviousTab,
    /// Scroll the open inspector tab by this many pages (`PgDn` / `PgUp`).
    ScrollTab {
        pages: i32,
    },
    /// The mouse wheel turned this many notches, down being positive, with
    /// the pointer over this screen cell.
    Wheel {
        at: Position,
        notches: i32,
    },
    /// Back out of whatever is open, or ask to quit (`Esc`).
    Back,
    /// Say yes to a prompt (`y`).
    Confirm,
    /// Cancel a prompt: any key with no job of its own.
    Dismiss,
    /// Quit at once (`Ctrl+C`).
    Quit,
}

/// How far Shift scrolls the viewport, in tiles (design §6.5).
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
            let quit = key.code == KeyCode::Char('c');
            return Some(if quit { Action::Quit } else { Action::Dismiss });
        }
        // Only Shift makes a scroll key jump: a capital letter may come from Caps Lock.
        let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
            SHIFT_STEP
        } else {
            1
        };
        let scroll = |dx: i32, dy: i32| Some(Action::Scroll { dx, dy });
        let code = match key.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            other => other,
        };
        match code {
            KeyCode::Up | KeyCode::Char('w') => scroll(0, -step),
            KeyCode::Left | KeyCode::Char('a') => scroll(-step, 0),
            KeyCode::Down | KeyCode::Char('s') => scroll(0, step),
            KeyCode::Right | KeyCode::Char('d') => scroll(step, 0),
            // Holding space would flicker pause on and off, so only a fresh press toggles.
            KeyCode::Char(' ') => (!held).then_some(Action::TogglePause),
            // Likewise, a held Esc would answer its own "Quit?" prompt.
            KeyCode::Esc => (!held).then_some(Action::Back),
            KeyCode::Char('.') => Some(Action::StepOnce),
            KeyCode::Char('+' | '=') => Some(Action::Faster { held }),
            KeyCode::Char('-') => Some(Action::Slower { held }),
            KeyCode::Char('y') => Some(Action::Confirm),
            // Some terminals report Shift+Tab as its own key, others as Tab with Shift.
            KeyCode::BackTab => Some(Action::SelectPrevious),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Action::SelectPrevious)
            }
            KeyCode::Tab => Some(Action::SelectNext),
            KeyCode::Char(']') => Some(Action::NextTab),
            KeyCode::Char('[') => Some(Action::PreviousTab),
            KeyCode::PageDown => Some(Action::ScrollTab { pages: 1 }),
            KeyCode::PageUp => Some(Action::ScrollTab { pages: -1 }),
            _ => Some(Action::Dismiss),
        }
    }
}

/// The action for a mouse event. Every event says where the pointer is, so it
/// points there; a left-button press clicks, and the wheel turns.
pub fn mouse_action(event: MouseEvent) -> Option<Action> {
    let cell = Position::new(event.column, event.row);
    Some(match event.kind {
        MouseEventKind::Down(MouseButton::Left) => Action::Click(cell),
        MouseEventKind::ScrollDown => Action::Wheel {
            at: cell,
            notches: 1,
        },
        MouseEventKind::ScrollUp => Action::Wheel {
            at: cell,
            notches: -1,
        },
        _ => Action::Point(cell),
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
