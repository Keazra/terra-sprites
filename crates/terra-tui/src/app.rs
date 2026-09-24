//! The UI state (design §6.8): everything the screen shows that isn't the world.

use ratatui::layout::{Position, Rect, Size};
use terra_sim::{Map, Pos};

use crate::clock::Clock;
use crate::input::Action;
use crate::theme::Theme;

/// Whether the game carries on after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

/// The UI state. Rendering reads it; actions change it.
pub struct App {
    pub clock: Clock,
    pub theme: Theme,
    /// The seed the world was made from, shown so a map can be made again.
    pub seed: u64,
    cursor: Pos,
    /// The top-left tile of the viewport.
    viewport: Pos,
    /// The map's size, in tiles.
    map_size: Size,
    /// Where on screen the map view draws its tiles.
    tiles: Rect,
    /// The screen cell under the mouse pointer, while that is one of the map view's tiles.
    pointer: Option<Position>,
    quit_prompt: bool,
}

impl App {
    /// A new UI for `map`, with the cursor at the map's centre and the viewport
    /// centred on it. `tiles` is where on screen the map view draws its tiles.
    pub fn new(map: &Map, theme: Theme, seed: u64, tiles: Rect) -> App {
        let cursor = Pos {
            x: map.width() / 2,
            y: map.height() / 2,
        };
        let centred = |cursor: u16, view: u16, len: u16| {
            clamp_origin(i32::from(cursor) - i32::from(view / 2), view, len)
        };
        App {
            clock: Clock::new(),
            theme,
            seed,
            cursor,
            viewport: Pos {
                x: centred(cursor.x, tiles.width, map.width()),
                y: centred(cursor.y, tiles.height, map.height()),
            },
            map_size: Size::new(map.width(), map.height()),
            tiles,
            pointer: None,
            quit_prompt: false,
        }
    }

    /// The tile the cursor is on.
    pub fn cursor(&self) -> Pos {
        self.cursor
    }

    /// The top-left tile of the viewport: the part of the map the map view shows.
    pub fn viewport(&self) -> Pos {
        self.viewport
    }

    /// Whether "Quit? (y/n)" is waiting for an answer.
    pub fn quit_prompt_open(&self) -> bool {
        self.quit_prompt
    }

    /// Refits the viewport to where the map view now draws its tiles, as after
    /// a resize. The viewport stays within the wall.
    pub fn fit_viewport(&mut self, tiles: Rect) {
        self.tiles = tiles;
        self.scroll(0, 0);
    }

    /// Carries out an action, and says whether the game carries on.
    pub fn apply(&mut self, action: Action) -> Flow {
        if self.quit_prompt {
            match action {
                Action::Yes | Action::Escape | Action::Quit => return Flow::Quit,
                // The mouse carries on as usual and doesn't answer the prompt.
                Action::Point { .. } | Action::Click { .. } => {}
                // Any other key cancels the prompt, and does nothing else.
                _ => {
                    self.quit_prompt = false;
                    return Flow::Continue;
                }
            }
        }
        match action {
            Action::TogglePause => self.clock.toggle_pause(),
            Action::StepOnce => self.clock.step_once(),
            Action::Faster { held: false } => self.clock.faster(),
            Action::Faster { held: true } => self.clock.faster_held(),
            Action::Slower { held: false } => self.clock.slower(),
            Action::Slower { held: true } => self.clock.slower_held(),
            Action::Scroll { dx, dy } => self.scroll(dx, dy),
            // In Select mode (the only mode so far), a click just points.
            Action::Point { column, row } | Action::Click { column, row } => {
                self.point(Position::new(column, row));
            }
            Action::Escape => self.quit_prompt = true,
            Action::Yes | Action::OtherKey => {}
            Action::Quit => return Flow::Quit,
        }
        Flow::Continue
    }

    /// Scrolls the viewport, stopping at the wall. A still pointer then points
    /// at whatever tile has moved under it.
    fn scroll(&mut self, dx: i32, dy: i32) {
        let map = self.map_size;
        self.viewport = Pos {
            x: clamp_origin(i32::from(self.viewport.x) + dx, self.tiles.width, map.width),
            y: clamp_origin(
                i32::from(self.viewport.y) + dy,
                self.tiles.height,
                map.height,
            ),
        };
        if let Some(cell) = self.pointer {
            self.point(cell);
        }
    }

    /// Puts the cursor on the tile at screen cell `cell`. Off the map view's
    /// tiles, the cursor stays where it was.
    fn point(&mut self, cell: Position) {
        if !self.tiles.contains(cell) {
            self.pointer = None;
            return;
        }
        self.pointer = Some(cell);
        let map = self.map_size;
        self.cursor = Pos {
            x: (self.viewport.x + (cell.x - self.tiles.x)).min(map.width - 1),
            y: (self.viewport.y + (cell.y - self.tiles.y)).min(map.height - 1),
        };
    }
}

/// A viewport origin kept within the map, so the view never shows past the wall.
/// A map no bigger than the view is shown from its start.
fn clamp_origin(origin: i32, view: u16, len: u16) -> u16 {
    let furthest = (i32::from(len) - i32::from(view)).max(0);
    origin.clamp(0, furthest) as u16
}
