//! The UI state (design §6.8): everything the screen shows that isn't the world.

use ratatui::layout::Size;
use terra_sim::{Map, Pos};

use crate::clock::Clock;
use crate::theme::Theme;

/// How close to the viewport's edge the cursor may come before it scrolls, in tiles.
const SCROLL_MARGIN: u16 = 3;

/// The UI state. Rendering reads it; input changes it.
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
    /// How many tiles the map view shows.
    view: Size,
}

impl App {
    /// A new UI for `map`, with the cursor at the map's centre. `view` is how
    /// many tiles the map view shows.
    pub fn new(map: &Map, theme: Theme, seed: u64, view: Size) -> App {
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
                x: centred(cursor.x, view.width, map.width()),
                y: centred(cursor.y, view.height, map.height()),
            },
            map_size: Size::new(map.width(), map.height()),
            view,
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

    /// Refits the viewport to a map view of `view` tiles, as after a resize: it
    /// stays within the wall and keeps the cursor in view, moving as little as it can.
    pub fn fit_viewport(&mut self, view: Size) {
        self.view = view;
        let map = self.map_size;
        self.viewport = Pos {
            x: contain(self.viewport.x, self.cursor.x, view.width, map.width),
            y: contain(self.viewport.y, self.cursor.y, view.height, map.height),
        };
    }

    /// Puts the cursor on `pos`, a tile in view, as a click does. The viewport
    /// doesn't scroll.
    pub fn place_cursor(&mut self, pos: Pos) {
        self.cursor = pos;
    }

    /// Moves the cursor by `(dx, dy)` tiles, stopping at the wall. The viewport
    /// scrolls just enough to keep the cursor away from its edge.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (map, view) = (self.map_size, self.view);
        self.cursor = Pos {
            x: step_within(self.cursor.x, dx, map.width),
            y: step_within(self.cursor.y, dy, map.height),
        };
        self.viewport = Pos {
            x: follow(self.viewport.x, self.cursor.x, view.width, map.width),
            y: follow(self.viewport.y, self.cursor.y, view.height, map.height),
        };
    }
}

/// A viewport origin kept within the map, so the view never shows past the wall.
/// A map no bigger than the view is shown from its start.
fn clamp_origin(origin: i32, view: u16, len: u16) -> u16 {
    let furthest = (i32::from(len) - i32::from(view)).max(0);
    origin.clamp(0, furthest) as u16
}

/// A viewport origin moved as little as possible to keep `cursor` inside a
/// view of `view` tiles, with no margin.
fn contain(origin: u16, cursor: u16, view: u16, len: u16) -> u16 {
    if view == 0 {
        return origin;
    }
    let cursor = i32::from(cursor);
    let origin = i32::from(origin).clamp(cursor + 1 - i32::from(view), cursor);
    clamp_origin(origin, view, len)
}

/// A viewport origin moved as little as possible to keep `cursor` at least
/// `SCROLL_MARGIN` tiles inside a view of `view` tiles (less, in a tiny view).
fn follow(origin: u16, cursor: u16, view: u16, len: u16) -> u16 {
    if view == 0 {
        return origin;
    }
    let margin = i32::from(SCROLL_MARGIN.min((view - 1) / 2));
    let (cursor, view_len) = (i32::from(cursor), i32::from(view));
    let origin = i32::from(origin).clamp(cursor + margin + 1 - view_len, cursor - margin);
    clamp_origin(origin, view, len)
}

/// `coord + delta`, kept within `0..len`.
fn step_within(coord: u16, delta: i32, len: u16) -> u16 {
    (i32::from(coord) + delta).clamp(0, i32::from(len) - 1) as u16
}
