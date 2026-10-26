//! The map view (design §6.1, M2 design §11): where the viewport is and
//! how it scrolls, which tile or what past the wall a screen cell shows,
//! where the pointer points, and the view tracking the selected sprite.

use std::time::Duration;

use ratatui::layout::{Position, Rect};
use terra_sim::{Pos, World};

use super::{App, Areas, Origin, Selection};
use crate::input::Action;
use crate::space::Place;

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
            self.viewport.x + i32::from(cell.x - corner.x),
            self.viewport.y + i32::from(cell.y - corner.y),
            self.map_size,
        )
    }

    /// The tile drawn at screen cell `cell`, if the map view draws one there.
    pub fn tile_at(&self, cell: Position) -> Option<Pos> {
        self.place_at(cell)?.tile()
    }

    /// What the pointer on screen cell `cell` points at, as `pointed_spot`
    /// says, if it's over the map view: a tile, the glass or space.
    pub(super) fn pointed_place(&self, cell: Position) -> Option<Place> {
        let (x, y) = self.pointed_spot(cell)?;
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
    /// tile (M2 design §11.4).
    pub(super) fn pointed_spot(&self, cell: Position) -> Option<(i32, i32)> {
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
        let (col, row) = (
            cell.x.saturating_sub(1).max(area.x) - area.x,
            cell.y.saturating_sub(1).max(area.y) - area.y,
        );
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
                self.pointed_place(at).is_some() && !self.inspector.is_some_and(|i| i.contains(at))
            }
            _ => false,
        }
    }

    /// The screen cell where `tile` is drawn, if it is in view.
    pub fn cell_of(&self, tile: Pos) -> Option<Position> {
        let (area, origin) = (self.tile_area, self.viewport);
        let col = u16::try_from(i32::from(tile.x) - origin.x).ok()?;
        let row = u16::try_from(i32::from(tile.y) - origin.y).ok()?;
        (col < area.width && row < area.height).then(|| Position::new(area.x + col, area.y + row))
    }

    /// Refits the app to where its panels are now drawn, as after a resize.
    pub fn fit(&mut self, areas: Areas) {
        self.tile_area = areas.tiles;
        self.inspector = areas.inspector;
        self.event_log_area = areas.event_log;
        self.overlay = areas.overlay;
        self.settle();
    }

    /// Where the help screen and the sprite list are drawn, if the screen
    /// has room for the game.
    pub fn overlay(&self) -> Option<Rect> {
        self.overlay
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
        let area = self.tile_area;
        self.viewport = Origin {
            x: i32::from(tile.x) - i32::from(area.width / 2),
            y: i32::from(tile.y) - i32::from(area.height / 2),
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
            x: clamp_origin(self.viewport.x, area.width, map.width),
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
    /// §11.4).
    pub(super) fn point(&mut self, cell: Position) {
        self.mouse = Some(cell);
        let spot = self.pointed_spot(cell);
        self.pointer = spot.map(|_| cell);
        // While the player aims, the pointer pulls, past the wall too (M2
        // design §11.4).
        if let (Some(aim), Some(spot)) = (self.aim.as_mut(), spot) {
            aim.to = spot;
        }
        match spot.map(|(x, y)| Place::at(x, y, self.map_size)) {
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
