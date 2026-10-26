//! Maps keys and the mouse to UI actions (design §6.5–6.6), telling held keys
//! from fresh presses.

use std::collections::HashSet;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;

use crate::app::CursorMode;
use crate::listed::listed_enum;

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
    /// Open the command palette (`:`, #303).
    Palette,
    /// Open or close the sprite list (`l`, design §6.1).
    SpriteList,
    /// In the sprite list, start filtering it (`/`, design §6.1, #313).
    FilterList,
    /// Open or close the population graph (`p`, M2 design §14).
    Population,
    /// Open or close the family tree (`k`, M2 design §17.2).
    FamilyTree,
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
    /// Show or hide the debug overlay, how the game is running on this
    /// computer (`F3`, design §6.1, #311).
    DebugOverlay,
    /// While naming, a letter typed.
    Type(char),
    /// While naming, the last letter rubbed out (`Backspace`).
    Erase,
    /// While naming, another random name to start from (`Tab`); while
    /// filtering the sprite list, the next order.
    AnotherName,
    /// While typing, `?` let go of, where the terminal says so: in the New
    /// world box, the mode's explanation goes (M2 design §16).
    HelpReleased,
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

    /// Whether this is the pointer's doing (pointing, a click, a middle
    /// click or a turn of the wheel) or a button let go of, or the key that
    /// stands for it: none of these is a key pressed, so none answers a
    /// prompt (design v33 §6.6), and a screen too small to draw on takes
    /// none (M2 design §8.2).
    pub fn is_pointer(self) -> bool {
        matches!(
            self,
            Action::Point(_)
                | Action::Click { .. }
                | Action::Release { .. }
                | Action::Follow { at: Some(_) }
                | Action::Wheel { .. }
        )
    }
}

listed_enum! {
    /// What the terminal reports of keys, as far as `Keys` can tell (design
    /// §6.6): what the debug overlay says (design §6.1, #311), so a problem
    /// holding a key can be told from a picture.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum KeyReports {
        /// No release has arrived, so a held key is told from taps only by
        /// its repeats, where the terminal sends them.
        #[default]
        NoReleases => "no releases",
        /// Releases arrive.
        Releases => "releases reported",
        /// The kitty keyboard protocol is on: releases arrive, and a
        /// shifted key as the character it types.
        Kitty => "kitty protocol",
    }

    /// Each, the least reported first.
    pub const ALL;

    /// What the debug overlay says.
    pub fn label(self) -> &'static str;
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
    /// A shifted key arrives as its character without Shift, as with the
    /// kitty protocol's alternate keys, so a capital letter means Shift:
    /// Caps Lock alone sends the plain letter there.
    shifted_as_characters: bool,
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
            shifted_as_characters: false,
            down: HashSet::new(),
        }
    }

    /// A tracker for a terminal with the kitty keyboard protocol on, as the
    /// game asks for it (design §6.6): it reports releases, and a shifted
    /// key as the character it types, `?` or `W`, without Shift.
    pub fn kitty() -> Keys {
        Keys {
            releases_reported: true,
            shifted_as_characters: true,
            down: HashSet::new(),
        }
    }

    /// What the terminal reports of keys, as far as is known yet.
    pub fn reports(&self) -> KeyReports {
        match (self.shifted_as_characters, self.releases_reported) {
            (true, _) => KeyReports::Kitty,
            (false, true) => KeyReports::Releases,
            (false, false) => KeyReports::NoReleases,
        }
    }

    /// The action for a key event while the player types a name (design v28
    /// §6.5): letters are typed rather than acting, `Backspace` rubs one out,
    /// `Tab` offers another random name, `Enter` takes it and `Esc` gives up;
    /// the arrows pick from a list or a row beside it, and `Shift+Tab` goes
    /// back through the sprite list's orders while it's filtered (#313).
    /// `F3` still shows or hides the debug overlay (design §6.1).
    /// Only presses count, and `Ctrl+C` still quits, but letting go of `?`
    /// says so, as New world explains a mode while it's held (M2 design
    /// §16). Releases are still tracked, so the `r` that opened the prompt
    /// counts as let go after it, and a key already down when the prompt
    /// opened types nothing until it's let go: holding `r` doesn't type over
    /// the offered name.
    pub fn typed_action(&mut self, key: KeyEvent) -> Option<Action> {
        let physical = physical_key(key.code);
        if key.kind == KeyEventKind::Release {
            self.releases_reported = true;
            self.down.remove(&physical);
            return (physical == KeyCode::Char('/')).then_some(Action::HelpReleased);
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
            // Shift+Tab as `action_for` reads it, either way it's reported.
            KeyCode::BackTab => Some(Action::SelectPrevious),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Action::SelectPrevious)
            }
            KeyCode::Tab => Some(Action::AnotherName),
            KeyCode::Enter => Some(Action::Enter),
            KeyCode::Esc => Some(Action::Back),
            // `F3` types nothing, so it shows or hides the debug overlay
            // while typing too (design §6.1, #311), once a press: noted as
            // down, it does nothing pressed again with no release between.
            KeyCode::F(3) => {
                self.down.insert(physical);
                (key.kind != KeyEventKind::Repeat).then_some(Action::DebugOverlay)
            }
            // New world's presets and modes are picked while its seed is
            // typed (M2 design §8.3, §16); naming does nothing with them.
            KeyCode::Up => Some(Action::Scroll { dx: 0, dy: -1 }),
            KeyCode::Down => Some(Action::Scroll { dx: 0, dy: 1 }),
            KeyCode::Left => Some(Action::Scroll { dx: -1, dy: 0 }),
            KeyCode::Right => Some(Action::Scroll { dx: 1, dy: 0 }),
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
        // Only Shift makes a scroll key jump: a capital letter may come from
        // Caps Lock, except where a shifted key arrives as its character.
        let shift = key.modifiers.contains(KeyModifiers::SHIFT)
            || self.shifted_as_characters
                && matches!(key.code, KeyCode::Char(c) if c.is_ascii_uppercase());
        let step = if shift { SHIFT_STEP } else { 1 };
        let scroll = |dx: i32, dy: i32| Some(Action::Scroll { dx, dy });
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
            KeyCode::Char(':') => (!held).then_some(Action::Palette),
            KeyCode::Char('l') => (!held).then_some(Action::SpriteList),
            KeyCode::Char('/') => (!held).then_some(Action::FilterList),
            KeyCode::Char('p') => (!held).then_some(Action::Population),
            KeyCode::Char('k') => (!held).then_some(Action::FamilyTree),
            KeyCode::Char('g') => (!held).then_some(Action::ExportGenome),
            KeyCode::Char(digit @ '1'..='9') => (!held).then(|| Action::Pick(digit as u8 - b'0')),
            KeyCode::Enter => (!held).then_some(Action::Enter),
            // A held `F3` would flicker the debug overlay on and off.
            KeyCode::F(3) => (!held).then_some(Action::DebugOverlay),
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
        KeyCode::Char('?') => KeyCode::Char('/'),
        KeyCode::Char(':') => KeyCode::Char(';'),
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    }
}

impl Default for Keys {
    fn default() -> Self {
        Keys::new()
    }
}
