//! The map view (design §6.1, M2 design §11): where the viewport is and
//! how it scrolls, which tile or what past the wall a screen cell shows,
//! tiles one cell wide or two, where the pointer points, the view tracking
//! the selected sprite, and the inspector beside it or over it. The map
//! view is drawn from where the pan has reached (design §6.9), and the
//! pointer points at what's drawn there.

use std::time::Duration;

use ratatui::layout::{Position, Rect};
use terra_sim::{Pos, World};

use super::{App, Areas, Origin, Selection};
use crate::input::Action;
use crate::space::Place;
use crate::spots::Spot;

impl App {
    /// Where the viewport starts: the part of the map, and of what's past
    /// its wall, the map view shows.
    pub fn viewport(&self) -> Origin {
        self.viewport
    }

    /// Where the map view is drawn from just now: where the pan has
    /// reached while the view pans to a sprite, and the viewport otherwise
    /// (design §6.9). Whatever's drawn on the map, and what the pointer
    /// points at, go by this.
    pub(crate) fn shown_viewport(&self) -> Origin {
        self.motion
            .shown_from(self.viewport, self.running_for, self.clock.speed())
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
            self.column_from(corner, cell),
            self.shown_viewport().y + i32::from(cell.y - corner.y),
            self.map_size,
        )
    }

    /// Which column of tiles, past the wall too, the map view, drawing from
    /// screen cell `corner`, draws at screen cell `cell` (M2 design §11.7).
    pub(crate) fn column_from(&self, corner: Position, cell: Position) -> i32 {
        self.shown_viewport().x + i32::from((cell.x - corner.x) / self.tile_width())
    }

    /// How many screen cells wide a tile is drawn: two while zoomed, so
    /// tiles are square, and one otherwise (M2 design §11.7, #305).
    pub fn tile_width(&self) -> u16 {
        if self.zoom { 2 } else { 1 }
    }

    /// How many tiles across the player sees, a tile cut in half at the
    /// edge included: the map view's, less those under the inspector while
    /// it's open over it (design §6.1, #305). The view is centred, kept
    /// past the wall and saved by these.
    pub(super) fn seen_columns(&self) -> u16 {
        let width = match self.inspector {
            InspectorRoom::Over { area, open: true } => area.x.saturating_sub(self.tile_area.x),
            _ => self.tile_area.width,
        };
        width.min(self.tile_area.width).div_ceil(self.tile_width())
    }

    /// `o`: tiles two cells wide, or back to one, keeping the tile at the
    /// middle of the view where it is (M2 design §11.7, #305).
    pub(super) fn toggle_zoom(&mut self) {
        let middle = Origin {
            x: self.viewport.x + i32::from(self.seen_columns() / 2),
            y: self.viewport.y,
        };
        self.zoom = !self.zoom;
        self.viewport = Origin {
            x: middle.x - i32::from(self.seen_columns() / 2),
            y: middle.y,
        };
        // It jumps rather than pans (design §6.9).
        self.motion.stop_pan();
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
    /// the map view: the tile whose cells are up a row and a cell to the
    /// left, so the pointer's arrow rests on the Cursor's corner rather than
    /// hiding it; with tiles two cells wide, either of the tile's cells
    /// right of that (M2 design §11.7). On the map view's top row it stays
    /// in that row, and on its left column in that column; and the cell just
    /// right of or below the map view, the inspector's left column or the
    /// event log's title row, points at the last column or row, so every
    /// tile in view can be pointed at. Over the inspector where it's drawn
    /// over the map, and past the wall, at the glass or space, it points at
    /// no tile (design §6.1, M2 design §11.4). Where the tile it points at
    /// is under a pop-up's box, it points at nothing (design §6.1): on the
    /// box's left edge, it still points at the tile beside it.
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
        let pointed = Position::new(
            cell.x.saturating_sub(1).max(area.x),
            cell.y.saturating_sub(1).max(area.y),
        );
        if self.hidden_by_inspector(cell, pointed) {
            return None;
        }
        if self.spot_at(pointed) == Some(Spot::PopUp) {
            return None;
        }
        let (col, row) = ((pointed.x - area.x) / self.tile_width(), pointed.y - area.y);
        let shown = self.shown_viewport();
        Some((shown.x + i32::from(col), shown.y + i32::from(row)))
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
    /// cells wide, if it is in view: where the pan has reached while the
    /// view pans (design §6.9).
    pub fn cell_of(&self, tile: Pos) -> Option<Position> {
        self.cell_from(self.shown_viewport(), tile)
    }

    /// Whether `tile` is out of sight once the view has got where it's
    /// going, after any pan: out of view, or under the inspector open over
    /// the map view (design §6.1, #305).
    pub(super) fn out_of_sight(&self, tile: Pos) -> bool {
        self.cell_from(self.viewport, tile).is_none()
            || self.under_the_inspector_from(self.viewport, tile)
    }

    /// The screen cell where `tile` is drawn by a map view drawn from
    /// `origin`, as `cell_of` says.
    fn cell_from(&self, origin: Origin, tile: Pos) -> Option<Position> {
        let area = self.tile_area;
        let col = u16::try_from(i32::from(tile.x) - origin.x).ok()?;
        let row = u16::try_from(i32::from(tile.y) - origin.y).ok()?;
        let x = col.checked_mul(self.tile_width())?;
        (x < area.width && row < area.height).then(|| Position::new(area.x + x, area.y + row))
    }

    /// Refits the app to where its panels are now drawn, as after a resize.
    pub fn fit(&mut self, areas: Areas) {
        // On a screen of a new size, the view jumps (design §6.9).
        if areas.tiles != self.tile_area {
            self.motion.stop_pan();
        }
        self.tile_area = areas.tiles;
        self.inspector = InspectorRoom::of(&areas, self.inspector);
        // With no room for the inspector, it can't fill the screen either
        // (design §6.1, #305).
        self.full_screen &= self.inspector != InspectorRoom::Nowhere;
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
        if self.full_screen {
            self.overlay
        } else {
            self.inspector.shown()
        }
    }

    /// Whether the inspector fills the screen (`i`, design §6.1, #305),
    /// under whatever's opened from it.
    pub fn full_screen(&self) -> bool {
        self.full_screen
    }

    /// Whether the inspector is open over the map view, where it pops
    /// over it on a screen under 100 columns (design §6.1, #305).
    pub fn inspector_open(&self) -> bool {
        matches!(self.inspector, InspectorRoom::Over { open: true, .. })
    }

    /// Whether the inspector is shut, on a screen where it pops over the
    /// map view (design §6.1, #305).
    pub fn inspector_shut(&self) -> bool {
        matches!(self.inspector, InspectorRoom::Over { open: false, .. })
    }

    /// Whether screen cell `cell` is on the inspector, as it's drawn just
    /// now.
    pub(crate) fn on_the_inspector(&self, cell: Position) -> bool {
        self.inspector_area()
            .is_some_and(|area| area.contains(cell))
    }

    /// Whether the inspector hides the map view from a pointer on screen
    /// cell `cell` that points at cell `pointed`, so it points at nothing
    /// on the map (design §6.1, #305): filling the screen, it hides all of
    /// it, and open over the map view, where it's on either.
    fn hidden_by_inspector(&self, cell: Position, pointed: Position) -> bool {
        self.full_screen
            || self.inspector_open()
                && (self.on_the_inspector(cell) || self.on_the_inspector(pointed))
    }

    /// Whether `tile` is drawn under the inspector, open over the map view
    /// (design §6.1, #305).
    pub(super) fn under_the_inspector(&self, tile: Pos) -> bool {
        self.under_the_inspector_from(self.shown_viewport(), tile)
    }

    /// Whether `tile` is drawn under the inspector, open over the map view,
    /// by a map view drawn from `origin`: the one rule `under_the_inspector`
    /// and `out_of_sight` go by (design §6.1, #305).
    fn under_the_inspector_from(&self, origin: Origin, tile: Pos) -> bool {
        self.inspector_open()
            && self
                .cell_from(origin, tile)
                .is_some_and(|cell| self.on_the_inspector(cell))
    }

    /// Opens the inspector over the map view, or shuts it, where it pops
    /// over it, keeping the view to what's seen beside it (design §6.1,
    /// #305).
    pub(super) fn open_inspector(&mut self, now: bool) {
        self.inspector.open(now);
        self.settle();
    }

    /// `i`: the inspector, bigger (design §6.1, #305). Where it pops over
    /// the map view and is shut, it opens there; otherwise it fills the
    /// screen, and `i` again, or `Esc`, takes it back. With no room for it
    /// at all, it's refused.
    pub(super) fn bigger_inspector(&mut self) {
        if self.inspector == InspectorRoom::Nowhere {
            return self.refuse("No room for the inspector".into());
        }
        self.on_inspector = true;
        if self.inspector_shut() {
            self.open_inspector(true);
        } else {
            self.end_aim();
            self.full_screen = true;
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
            self.pan_to(pos);
        }
    }

    /// Scrolls the viewport so that `tile` is at its centre, as far as the
    /// margin past the wall allows (M2 design §11.1). The view jumps there,
    /// as a new world or a load starts it.
    pub(super) fn centre_on(&mut self, tile: Pos) {
        self.viewport = self.centred_on(tile);
        self.settle();
    }

    /// Centres the view on the sprite or egg on `tile`, as `centre_on`
    /// does, but the map view pans there when it's more than 2 tiles away,
    /// from wherever it's drawn just now (design §6.9).
    pub(super) fn pan_to(&mut self, tile: Pos) {
        let (was, drawn) = (self.viewport, self.shown_viewport());
        self.viewport = self.centred_on(tile);
        self.keep_in_margin();
        let to = self.viewport;
        self.motion.centred((was, drawn, to), self.running_for);
        self.repoint();
    }

    /// The viewport with `tile` at its centre.
    fn centred_on(&self, tile: Pos) -> Origin {
        Origin {
            x: i32::from(tile.x) - i32::from(self.seen_columns() / 2),
            y: i32::from(tile.y) - i32::from(self.tile_area.height / 2),
        }
    }

    pub(super) fn scroll(&mut self, dx: i32, dy: i32) {
        self.viewport = Origin {
            x: self.viewport.x.saturating_add(dx),
            y: self.viewport.y.saturating_add(dy),
        };
        // Scrolling jumps, a pan under way included (design §6.9).
        self.motion.stop_pan();
        self.settle();
    }

    /// Keeps the viewport within its margin past the wall, and the cursor on
    /// whatever tile a still pointer points at.
    fn settle(&mut self) {
        self.keep_in_margin();
        self.repoint();
    }

    /// Keeps the viewport within its margin past the wall (M2 design
    /// §11.1).
    fn keep_in_margin(&mut self) {
        let (map, area) = (self.map_size, self.tile_area);
        let ((left, right), (top, bottom)) =
            (self.origins_across(), origins(area.height, map.height));
        self.viewport = Origin {
            x: self.viewport.x.clamp(left, right),
            y: self.viewport.y.clamp(top, bottom),
        };
    }

    /// Puts the Cursor on whatever tile a still pointer points at, but not
    /// while the view pans: only once the pan ends, so the world never
    /// hears of the tiles between (design §6.9).
    pub(super) fn repoint(&mut self) {
        if self.motion.pans(self.running_for, self.clock.speed()) {
            return;
        }
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

    /// The first and last of the viewport's origins across (M2 design
    /// §11.1). While the inspector is open over the map view, the part seen
    /// beside it goes as far past the wall as it would alone, and the whole
    /// view's as before, so opening it never moves the view; unless the map
    /// fits in the part seen, which shows it in its middle (design §6.1,
    /// #305).
    fn origins_across(&self) -> (i32, i32) {
        let len = self.map_size.width;
        let (whole, seen) = (
            self.tile_area.width.div_ceil(self.tile_width()),
            self.seen_columns(),
        );
        let (left, right) = origins(seen, len);
        if seen == whole || left == right {
            return (left, right);
        }
        let (whole_left, whole_right) = origins(whole, len);
        (left.min(whole_left), right.max(whole_right))
    }
}

/// The first and last viewport origins, a view `view` tiles long goes a
/// quarter of itself past the wall at either end of a map `len` tiles long.
/// A map that fits in the view with its glass, a tile at either end, is
/// shown in the middle of it instead, and doesn't scroll (M2 design §11.1).
fn origins(view: u16, len: u16) -> (i32, i32) {
    let (view, len) = (i32::from(view), i32::from(len));
    if len + 2 <= view {
        let middle = -((view - len) / 2);
        return (middle, middle);
    }
    let margin = view / 4;
    (-margin, (len - view + margin).max(-margin))
}

/// Where the inspector goes as the screen is laid out (design §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InspectorRoom {
    /// No room for it: the map view is drawn alone (design §6.1, #305).
    Nowhere,
    /// Beside the map view, its title row included.
    Beside(Rect),
    /// Over the map view's right side, on a screen under 100 columns,
    /// drawn only while it's open (design §6.1, #305).
    Over { area: Rect, open: bool },
}

impl InspectorRoom {
    /// The room `areas` lay out for it, open over the map view if it was
    /// open over it in `was`.
    pub(super) fn of(areas: &Areas, was: InspectorRoom) -> InspectorRoom {
        let open = matches!(was, InspectorRoom::Over { open: true, .. });
        match areas.inspector {
            None => InspectorRoom::Nowhere,
            Some(area) if areas.pops_over => InspectorRoom::Over { area, open },
            Some(area) => InspectorRoom::Beside(area),
        }
    }

    /// Where it's drawn while it's shown, if there's room for it.
    pub(super) fn room(self) -> Option<Rect> {
        match self {
            InspectorRoom::Nowhere => None,
            InspectorRoom::Beside(area) | InspectorRoom::Over { area, .. } => Some(area),
        }
    }

    /// Whether it pops over the map view rather than sitting beside it.
    pub(super) fn pops_over(self) -> bool {
        matches!(self, InspectorRoom::Over { .. })
    }

    /// Where it's drawn, if it's shown: beside the map view, or over it
    /// while it's open.
    fn shown(self) -> Option<Rect> {
        match self {
            InspectorRoom::Beside(area) | InspectorRoom::Over { area, open: true } => Some(area),
            _ => None,
        }
    }

    /// Opens it over the map view, or shuts it, where it pops over it.
    pub(super) fn open(&mut self, now: bool) {
        if let InspectorRoom::Over { open, .. } = self {
            *open = now;
        }
    }
}
