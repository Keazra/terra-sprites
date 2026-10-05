//! Maps keys and the mouse to UI actions (design §6.5–6.6), telling held keys
//! from fresh presses.

use std::collections::HashSet;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;

use crate::app::CursorMode;

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
    /// A click on this screen cell, amplified if Ctrl was held (design v21
    /// §6.5).
    Click {
        at: Position,
        button: Button,
        amplified: bool,
    },
    /// `Q` or `E`: what a left or right click does, where the Cursor is,
    /// amplified if Shift was held (design v21 §6.5).
    Press {
        button: Button,
        amplified: bool,
    },
    /// A mouse button let go of over this screen cell, or the key that
    /// stands for it with no cell: in Grab mode, letting go of the right
    /// button sends a throw or a shove (design v25 §6.5).
    Release {
        button: Button,
        at: Option<Position>,
    },
    /// `F`, or a middle click on this screen cell: following, the Cursor
    /// stops; otherwise it follows the sprite where the Cursor is, or else
    /// the selected sprite, leaving the selection as it is (design v26 §6.5).
    Follow {
        at: Option<Position>,
    },
    /// Pick a cursor mode (`Z` Select, `X` Train, `C` Grab); `C` in Grab
    /// mode opens the Place menu (design v28 §6.5).
    Mode(CursorMode),
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
    /// Back out of whatever is open, let go of what the Cursor has hold of,
    /// or ask to quit (`Esc`).
    Back,
    /// Say yes to a prompt (`y`).
    Confirm,
    /// Switch the detail view on or off (`v`).
    ToggleDetail,
    /// Switch to the next colour mode for sprites (`b`, design §6.3).
    CycleColours,
    /// Have the view follow the selected sprite, or stop (`T`, design v21
    /// §6.1).
    Track,
    /// Switch the event log to its next filter (`m`, design §6.1).
    CycleEventFilter,
    /// Open or close the help screen (`?`, design §6.1).
    Help,
    /// Open or close the sprite list (`l`, design §6.1).
    SpriteList,
    /// Show the Cursor to sprites in the current cursor mode, or hide it
    /// from them (`h`, design v29 §6.5).
    ToggleVisible,
    /// Cancel a prompt: any key with no job of its own.
    Dismiss,
    /// Pick the menu item with this number, from 1 (`1`–`9`, design v28).
    Pick(u8),
    /// Take what's chosen: the highlighted menu item, or the name typed
    /// (`Enter`).
    Enter,
    /// Name the selected sprite (`r`, design §6.5).
    Rename,
    /// Export the selected sprite's genome to a file (`g`, design §6.1).
    ExportGenome,
    /// Save the world as the quicksave (`F5`, design §6.7).
    Quicksave,
    /// Load the quicksave (`F9`).
    Quickload,
    /// Save the world by a name typed (`Ctrl+S`).
    SaveAs,
    /// Pick a save to load from a list (`Ctrl+O`).
    OpenSaves,
    /// Pick a theme from the built-in ones and the themes folder (`Ctrl+T`,
    /// design v34 §6.7).
    OpenThemes,
    /// In a replay, take it over and play on from where it is (`Ctrl+R`,
    /// design v36 §2.7).
    TakeOver,
    /// While naming, a letter typed.
    Type(char),
    /// While naming, the last letter rubbed out (`Backspace`).
    Erase,
    /// While naming, another random name to start from (`Tab`).
    AnotherName,
    /// Quit at once (`Ctrl+C`).
    Quit,
}

/// A mouse button, or the key that stands for it (design v21 §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// The left button, or `Q`.
    Left,
    /// The right button, or `E`.
    Right,
}

impl Action {
    /// A plain left click on `at`.
    pub fn left_click(at: Position) -> Action {
        Action::Click {
            at,
            button: Button::Left,
            amplified: false,
        }
    }

    /// A plain right click on `at`.
    pub fn right_click(at: Position) -> Action {
        Action::Click {
            at,
            button: Button::Right,
            amplified: false,
        }
    }

    /// A middle click on `at`: what `F` does, there (design v26 §6.5).
    pub fn middle_click(at: Position) -> Action {
        Action::Follow { at: Some(at) }
    }
}

/// How far Shift scrolls the viewport, in tiles (design §6.5).
const SHIFT_STEP: i32 = 5;

/// Turns key events into actions, remembering enough to recognise held keys.
///
/// A key is *held* when the terminal reports it as repeating, or when it is
/// pressed again without a release in between. The second rule is only trusted
/// where the terminal reports releases (Windows always does, as does a terminal
/// with the kitty keyboard protocol on; others are trusted once a release
/// arrives), otherwise every press would look held.
#[derive(Debug)]
pub struct Keys {
    releases_reported: bool,
    down: HashSet<KeyCode>,
}

impl Keys {
    /// A tracker assuming what this platform's terminals do without the kitty
    /// keyboard protocol: Windows reports releases.
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

    /// The action for a key event while the player types a name (design v28
    /// §6.5): letters are typed rather than acting, `Backspace` rubs one out,
    /// `Tab` offers another random name, `Enter` takes it and `Esc` gives up.
    /// Only presses count, and `Ctrl+C` still quits. Releases are still
    /// tracked, so the `r` that opened the prompt counts as let go after it,
    /// and a key already down when the prompt opened types nothing until it's
    /// let go: holding `r` doesn't type over the offered name.
    pub fn typed_action(&mut self, key: KeyEvent) -> Option<Action> {
        let physical = physical_key(key.code);
        if key.kind == KeyEventKind::Release {
            self.releases_reported = true;
            self.down.remove(&physical);
            return None;
        }
        if self.releases_reported && self.down.contains(&physical) {
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return matches!(key.code, KeyCode::Char('c' | 'C')).then_some(Action::Quit);
        }
        match key.code {
            KeyCode::Char(c) => Some(Action::Type(c)),
            KeyCode::Backspace => Some(Action::Erase),
            KeyCode::Tab => Some(Action::AnotherName),
            KeyCode::Enter => Some(Action::Enter),
            KeyCode::Esc => Some(Action::Back),
            _ => None,
        }
    }

    /// The action for a key event, if it has one. Of key releases, only
    /// `E`'s acts: it's the right button let go (design v25 §6.5).
    pub fn action_for(&mut self, key: KeyEvent) -> Option<Action> {
        // Terminals using the kitty keyboard protocol report these as keys of
        // their own (design v27 §6.6); they only change other keys.
        if matches!(
            key.code,
            KeyCode::Modifier(_) | KeyCode::CapsLock | KeyCode::NumLock | KeyCode::ScrollLock
        ) {
            return None;
        }
        let physical = physical_key(key.code);
        if key.kind == KeyEventKind::Release {
            self.releases_reported = true;
            self.down.remove(&physical);
            return (physical == KeyCode::Char('e')).then_some(Action::Release {
                button: Button::Right,
                at: None,
            });
        }
        let pressed_again = !self.down.insert(physical);
        let held = key.kind == KeyEventKind::Repeat || (self.releases_reported && pressed_again);
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(match key.code {
                KeyCode::Char('c' | 'C') => Action::Quit,
                // A held key would save or open the list again and again.
                KeyCode::Char('s' | 'S') if !held => Action::SaveAs,
                KeyCode::Char('o' | 'O') if !held => Action::OpenSaves,
                KeyCode::Char('t' | 'T') if !held => Action::OpenThemes,
                KeyCode::Char('r' | 'R') if !held => Action::TakeOver,
                KeyCode::Char('s' | 'S' | 'o' | 'O' | 't' | 'T' | 'r' | 'R') => return None,
                _ => Action::Dismiss,
            });
        }
        // Only Shift makes a scroll key jump: a capital letter may come from Caps Lock.
        let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
            SHIFT_STEP
        } else {
            1
        };
        let scroll = |dx: i32, dy: i32| Some(Action::Scroll { dx, dy });
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        // `Q` and `E` act once per press, like a click (design v21 §6.5).
        let press = |button| {
            (!held).then_some(Action::Press {
                button,
                amplified: shift,
            })
        };
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
            // Likewise, a held Esc would let go, leave the mode, ask to quit
            // and cancel its own question in a blur.
            KeyCode::Esc => (!held).then_some(Action::Back),
            KeyCode::Char('.') => Some(Action::StepOnce),
            KeyCode::Char('+' | '=') => Some(Action::Faster { held }),
            KeyCode::Char('-') => Some(Action::Slower { held }),
            KeyCode::Char('y') => Some(Action::Confirm),
            KeyCode::Char('z') => Some(Action::Mode(CursorMode::Select)),
            KeyCode::Char('x') => Some(Action::Mode(CursorMode::Train)),
            // `C` again in Grab mode opens the Place menu, so a held `C` mustn't.
            KeyCode::Char('c') => (!held).then_some(Action::Mode(CursorMode::Grab)),
            KeyCode::Char('q') => press(Button::Left),
            KeyCode::Char('e') => press(Button::Right),
            // A held `f` would flicker Follow on and off.
            KeyCode::Char('f') => (!held).then_some(Action::Follow { at: None }),
            // A held `v` would flicker the detail view on and off.
            KeyCode::Char('v') => (!held).then_some(Action::ToggleDetail),
            // Likewise a held `h`, whether sprites see the Cursor.
            KeyCode::Char('h') => (!held).then_some(Action::ToggleVisible),
            KeyCode::Char('r') => (!held).then_some(Action::Rename),
            KeyCode::Char('b') => (!held).then_some(Action::CycleColours),
            // A held `t` would flicker tracking on and off.
            KeyCode::Char('t') => (!held).then_some(Action::Track),
            KeyCode::Char('m') => (!held).then_some(Action::CycleEventFilter),
            KeyCode::Char('?') => (!held).then_some(Action::Help),
            KeyCode::Char('l') => (!held).then_some(Action::SpriteList),
            KeyCode::Char('g') => (!held).then_some(Action::ExportGenome),
            KeyCode::Char(digit @ '1'..='9') => (!held).then(|| Action::Pick(digit as u8 - b'0')),
            KeyCode::Enter => (!held).then_some(Action::Enter),
            KeyCode::F(5) => (!held).then_some(Action::Quicksave),
            KeyCode::F(9) => (!held).then_some(Action::Quickload),
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
/// points there; a left or right button press clicks, amplified with Ctrl
/// (design v21 §6.5), a middle press is what `F` does (design v26 §6.5), and
/// the wheel turns.
pub fn mouse_action(event: MouseEvent) -> Option<Action> {
    let cell = Position::new(event.column, event.row);
    let click = |button| Action::Click {
        at: cell,
        button,
        amplified: event.modifiers.contains(KeyModifiers::CONTROL),
    };
    Some(match event.kind {
        MouseEventKind::Down(MouseButton::Left) => click(Button::Left),
        MouseEventKind::Down(MouseButton::Right) => click(Button::Right),
        MouseEventKind::Down(MouseButton::Middle) => Action::middle_click(cell),
        MouseEventKind::Up(MouseButton::Right) => Action::Release {
            button: Button::Right,
            at: Some(cell),
        },
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
