//! The map view (design §6.1, M2 design §11): where the viewport is and
//! how it scrolls, which tile or what past the wall a screen cell shows,
//! tiles one cell wide or two, where the pointer points, the view tracking
//! the selected sprite, and the inspector beside it or over it.

use std::time::Duration;

use ratatui::layout::{Position, Rect};
use terra_sim::{Pos, World};

use super::{App, Areas, Origin, Screen, Selection};
use crate::input::Action;
use crate::space::Place;
use crate::spots::Spot;

impl App {
    /// Where the viewport starts: the part of the map, and of what's past
    /// its wall, the map view shows.
    pub fn viewport(&self) -> Origin {
        self.viewport
    }

    /// How far the stars have drifted, as time at 1× (M2 design §11.3).
    pub fn drift(&self) -> Duration {
        self.drift
    }

    /// What the map view draws at screen cell `cell`: a tile, the glass or
    /// space; or `None` off the map view's tiles.
    pub fn place_at(&self, cell: Position) -> Option<Place> {
        let area = self.tile_area;
        area.contains(cell)
            .then(|| self.place_from(area.as_position(), cell))
    }

    /// What the map view, drawing from screen cell `corner`, draws at
    /// screen cell `cell`, which may be past the cells the app has fitted
    /// its view to, as in a frame drawn just after the terminal grew.
    pub(crate) fn place_from(&self, corner: Position, cell: Position) -> Place {
        Place::at(
            self.viewport.x + i32::from((cell.x - corner.x) / self.tile_width()),
            self.viewport.y + i32::from(cell.y - corner.y),
            self.map_size,
        )
    }

    /// How many screen cells wide a tile is drawn: two while zoomed, so
    /// tiles are square, and one otherwise (M2 design §11.7, #305).
    pub fn tile_width(&self) -> u16 {
        if self.zoom { 2 } else { 1 }
    }

    /// Whether tiles are drawn two cells wide (`o`, M2 design §11.7).
    pub fn zoomed(&self) -> bool {
        self.zoom
    }

    /// How many tiles the map view shows across, a tile cut in half at its
    /// right edge included.
    pub(super) fn view_columns(&self) -> u16 {
        self.tile_area.width.div_ceil(self.tile_width())
    }

    /// `o`: tiles two cells wide, or back to one, keeping the tile at the
    /// middle of the view where it is (M2 design §11.7, #305).
    pub(super) fn toggle_zoom(&mut self) {
        let middle = Origin {
            x: self.viewport.x + i32::from(self.view_columns() / 2),
            y: self.viewport.y,
        };
        self.zoom = !self.zoom;
        self.viewport = Origin {
            x: middle.x - i32::from(self.view_columns() / 2),
            y: middle.y,
        };
        self.settle();
        self.tell_player(if self.zoom {
            "Tiles two cells wide, so they're square".into()
        } else {
            "Tiles one cell wide".into()
        });
    }

    /// The tile drawn at screen cell `cell`, if the map view draws one there.
    pub fn tile_at(&self, cell: Position) -> Option<Pos> {
        self.place_at(cell)?.tile()
    }

    /// What the pointer on screen cell `cell` points at, as `pointed_xy`
    /// says, if it's over the map view: a tile, the glass or space.
    pub(super) fn pointed_place(&self, cell: Position) -> Option<Place> {
        let (x, y) = self.pointed_xy(cell)?;
        Some(Place::at(x, y, self.map_size))
    }

    /// Where the pointer on screen cell `cell` points (design v27 §6.5), in
    /// the map's tile coordinates, which run on past the wall, if it's over
    /// the map view: the tile up and to the left of the one under it, so the
    /// pointer's arrow rests on the Cursor's corner rather than hiding it.
    /// On the map view's top row it stays in that row, and on its left
    /// column in that column; and the cell just right of or below the map
    /// view, the inspector's left column or the event log's title row,
    /// points at the last column or row, so every tile in view can be
    /// pointed at. Past the wall, at the glass or space, it points at no
    /// tile (M2 design §11.4). Where the tile it points at is under a
    /// pop-up's box, it points at nothing (design §6.1): on the box's left
    /// edge, it still points at the tile beside it.
    pub(super) fn pointed_xy(&self, cell: Position) -> Option<(i32, i32)> {
        let area = self.tile_area;
        let reach = Rect::new(
            area.x,
            area.y,
            area.width.saturating_add(1),
            area.height.saturating_add(1),
        );
        if area.is_empty() || !reach.contains(cell) {
            return None;
        }
        // Over the inspector where it's drawn over the map, it points at
        // nothing on the map (#305).
        if self.covers_map(cell) {
            return None;
        }
        let pointed = Position::new(
            cell.x.saturating_sub(1).max(area.x),
            cell.y.saturating_sub(1).max(area.y),
        );
        if self.spot_at(pointed) == Some(Spot::PopUp) {
            return None;
        }
        let (col, row) = ((pointed.x - area.x) / self.tile_width(), pointed.y - area.y);
        Some((
            self.viewport.x + i32::from(col),
            self.viewport.y + i32::from(row),
        ))
    }

    /// Whether `action` is the wheel over the map view, its glass and space
    /// included, which changes the cursor mode there (M2 design §11.4).
    pub(super) fn pointed_at_map(&self, action: &Action) -> bool {
        match *action {
            Action::Wheel { at, .. } => {
                self.pointed_place(at).is_some() && !self.on_the_inspector(at)
            }
            _ => false,
        }
    }

    /// The screen cell where `tile` is drawn, its first while tiles are two
    /// cells wide, if it is in view.
    pub fn cell_of(&self, tile: Pos) -> Option<Position> {
        let (area, origin) = (self.tile_area, self.viewport);
        let col = u16::try_from(i32::from(tile.x) - origin.x).ok()?;
        let row = u16::try_from(i32::from(tile.y) - origin.y).ok()?;
        let x = col.checked_mul(self.tile_width())?;
        (x < area.width && row < area.height).then(|| Position::new(area.x + x, area.y + row))
    }

    /// Refits the app to where its panels are now drawn, as after a resize.
    pub fn fit(&mut self, areas: Areas) {
        self.tile_area = areas.tiles;
        self.inspector_room = areas.inspector;
        self.pops_over = areas.pops_over;
        self.event_log_area = areas.event_log;
        self.overlay = areas.overlay;
        self.settle();
    }

    /// Where the help screen and the sprite list are drawn, if the screen
    /// has room for the game.
    pub fn overlay(&self) -> Option<Rect> {
        self.overlay
    }

    /// Where the inspector is drawn just now, if it is: filling the screen,
    /// beside the map view, or over it while it's open there (design §6.1,
    /// #305).
    pub fn inspector_area(&self) -> Option<Rect> {
        match self.screen {
            Screen::Inspector => self.overlay,
            _ if self.pops_over && !self.inspector_open => None,
            _ => self.inspector_room,
        }
    }

    /// Whether the inspector is open over the map view, where it pops
    /// over it on a screen under 100 columns (#305).
    pub fn inspector_open(&self) -> bool {
        self.pops_over && self.inspector_open
    }

    /// Whether the inspector would pop over the map view, and is shut
    /// (#305).
    pub fn inspector_shut(&self) -> bool {
        self.pops_over && !self.inspector_open && self.inspector_room.is_some()
    }

    /// Whether screen cell `cell` is on the inspector, as it's drawn just
    /// now.
    pub(super) fn on_the_inspector(&self, cell: Position) -> bool {
        self.inspector_area()
            .is_some_and(|area| area.contains(cell))
    }

    /// Whether the inspector is drawn over the map view at screen cell
    /// `cell`, so the map beneath it can't be pointed at there (#305).
    fn covers_map(&self, cell: Position) -> bool {
        let over = self.screen == Screen::Inspector || self.inspector_open();
        over && self.on_the_inspector(cell)
    }

    /// `i`: the inspector, bigger (design §6.1, #305). Where it pops over
    /// the map view and is shut, it opens there; otherwise it fills the
    /// screen, and `i` again, or `Esc`, takes it back. With no room for it
    /// at all, it's refused.
    pub(super) fn bigger_inspector(&mut self) {
        if self.inspector_room.is_none() {
            return self.refuse("No room for the inspector".into());
        }
        self.on_inspector = true;
        if self.pops_over && !self.inspector_open {
            self.inspector_open = true;
        } else {
            self.end_aim();
            self.screen = Screen::Inspector;
        }
    }

    /// `T`: the view follows the selected sprite, or stops (design v21
    /// §6.1). With none selected, it's refused.
    pub(super) fn toggle_tracking(&mut self, world: &World) {
        if self.tracking {
            self.tracking = false;
            return self.tell_player("Stopped tracking".into());
        }
        let Some(Selection::Living(id)) = self.selection else {
            return self.refuse("Select a sprite to track it".into());
        };
        self.tracking = true;
        self.tell_player(format!("Tracking {}", self.names.label(id)));
        self.track_selected(world);
    }

    /// Whether the view follows the selected sprite (design v21 §6.1).
    pub fn tracking(&self) -> bool {
        self.tracking
    }

    /// While tracking, centres the view on the selected sprite, as far as
    /// the margin past the wall allows (M2 design §11.1).
    pub(super) fn track_selected(&mut self, world: &World) {
        let tracked = match self.selection {
            // An egg, where it lies (M2 design §10.7).
            Some(Selection::Living(id)) if self.tracking => match world.sprite(id) {
                Some(sprite) => Some(sprite.pos()),
                None => world.egg(id).and_then(|egg| egg.pos()),
            },
            _ => None,
        };
        if let Some(pos) = tracked {
            self.centre_on(pos);
        }
    }

    /// Scrolls the viewport so that `tile` is at its centre, as far as the
    /// margin past the wall allows (M2 design §11.1).
    pub(super) fn centre_on(&mut self, tile: Pos) {
        self.viewport = Origin {
            x: i32::from(tile.x) - i32::from(self.view_columns() / 2),
            y: i32::from(tile.y) - i32::from(self.tile_area.height / 2),
        };
        self.settle();
    }

    pub(super) fn scroll(&mut self, dx: i32, dy: i32) {
        self.viewport = Origin {
            x: self.viewport.x.saturating_add(dx),
            y: self.viewport.y.saturating_add(dy),
        };
        self.settle();
    }

    /// Keeps the viewport within its margin past the wall, and the cursor on
    /// whatever tile a still pointer points at.
    fn settle(&mut self) {
        let (map, area) = (self.map_size, self.tile_area);
        self.viewport = Origin {
            x: clamp_origin(self.viewport.x, self.view_columns(), map.width),
            y: clamp_origin(self.viewport.y, area.height, map.height),
        };
        if let Some(cell) = self.pointer {
            self.point(cell);
        }
    }

    /// Notes the tile the pointer on screen cell `cell` points at as the
    /// pointer's, and puts the Cursor there, within the leash, unless it
    /// follows a sprite. Where it points at no tile, both stay on their last
    /// tile: past the wall, where the Cursor can't reach and isn't drawn,
    /// or off the map view, where it's drawn there again (M2 design
    /// §11.4). Where the pointer is on screen is noted as the pointer
    /// moves, whatever's open (`apply`), not here, since the view settling
    /// points again from the last tile pointed at.
    pub(super) fn point(&mut self, cell: Position) {
        let xy = self.pointed_xy(cell);
        self.pointer = xy.map(|_| cell);
        // While the player aims, the pointer pulls, past the wall too (M2
        // design §11.4).
        if let (Some(aim), Some(xy)) = (self.aim.as_mut(), xy) {
            aim.to = xy;
        }
        match xy.map(|(x, y)| Place::at(x, y, self.map_size)) {
            Some(Place::Tile(tile)) => {
                self.pointed = tile;
                self.pointer_past_wall = false;
                // While the player aims, the Cursor stays on what it aims,
                // and the pointer pulls (design v25 §6.5).
                if self.followed().is_none() && self.aim.is_none() {
                    self.cursor = self.within_leash(tile);
                }
            }
            Some(Place::Glass(_) | Place::Space) => self.pointer_past_wall = true,
            None => self.pointer_past_wall = false,
        }
    }
}

/// A viewport origin kept within a quarter of the view, `view` tiles long,
/// past the wall at either end of a map `len` tiles long. A map that fits in
/// the view with its glass, a tile at either end, is shown in the middle
/// of it instead, and doesn't scroll (M2 design §11.1).
fn clamp_origin(origin: i32, view: u16, len: u16) -> i32 {
    let (view, len) = (i32::from(view), i32::from(len));
    if len + 2 <= view {
        return -((view - len) / 2);
    }
    let margin = view / 4;
    origin.clamp(-margin, (len - view + margin).max(-margin))
}
