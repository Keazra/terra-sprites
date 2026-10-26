//! What the pointer is over, and what a click on it does (#304): the
//! spots of the frame last drawn, which drawing hands the app each frame.

use ratatui::layout::Position;
use terra_sim::{EntityId, World};

use super::App;
use crate::family_tree::Known;
use crate::spots::{Spot, Spots};

impl App {
    /// Keeps `spots`, the frame just drawn's, in place of the last's
    /// (#304). Drawing calls it once it's done.
    pub(crate) fn keep_spots(&self, spots: Spots) {
        *self.spots.borrow_mut() = spots;
    }

    /// The last screen cell the pointer was on, if it has been on one.
    pub(crate) fn mouse(&self) -> Option<Position> {
        self.mouse
    }

    /// The spot the last frame drew at screen cell `cell`, if any.
    pub(crate) fn spot_at(&self, cell: Position) -> Option<Spot> {
        self.spots.borrow().at(cell).cloned()
    }

    /// What the pointer is over, as the last frame drew it, if it's over
    /// a spot.
    pub(crate) fn pointed_spot(&self) -> Option<Spot> {
        self.spot_at(self.mouse?)
    }

    /// The sprite the event log's line under the pointer is about, which
    /// the map lights up (#304).
    pub(crate) fn pointed_event(&self) -> Option<EntityId> {
        match self.pointed_spot()? {
            Spot::Event(id) => Some(id),
            _ => None,
        }
    }

    /// Whether a left click at `at` lands on a spot that takes a click: the
    /// event log's filters, an inspector tab, or a line of the log.
    pub(super) fn clicks_spot(&self, at: Position) -> bool {
        matches!(
            self.spot_at(at),
            Some(Spot::Filters | Spot::Tab(_) | Spot::Event(_))
        )
    }

    /// A left click on the spot at `at`: the filters go to the next, a tab
    /// opens (design §6.1), and a line of the log selects the sprite it's
    /// about while it lives, as the Tab key does (#304).
    pub(super) fn click_spot(&mut self, at: Position, world: &World) {
        match self.spot_at(at) {
            Some(Spot::Filters) => self.next_filter(),
            Some(Spot::Tab(tab)) => self.open(tab),
            Some(Spot::Event(id)) => {
                let pos = match Known::of(world, id) {
                    Known::Living(sprite) => Some(sprite.pos()),
                    Known::Egg(egg) => egg.pos(),
                    Known::Dead(_) | Known::Forgotten => return,
                };
                self.select(id);
                if let Some(pos) = pos.filter(|&pos| self.cell_of(pos).is_none()) {
                    self.centre_on(pos);
                }
            }
            _ => {}
        }
    }
}
