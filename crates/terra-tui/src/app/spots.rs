//! What the pointer is over, and what a click on it does (design §6.1):
//! the spots of the frame last drawn, which drawing hands the app each
//! frame.

use ratatui::layout::Position;
use terra_sim::{EntityId, Pos, World};

use super::App;
use crate::family_tree::Known;
use crate::spots::{Spot, Spots};

impl App {
    /// Keeps `spots`, the frame just drawn's, in place of the last's
    /// (design §6.1). Drawing calls it once it's done.
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

    /// Whether a left click at `at` lands on a spot that takes one, rather
    /// than on the map.
    pub(super) fn clicks_spot(&self, at: Position) -> bool {
        self.spot_at(at).is_some_and(|spot| spot.takes_a_click())
    }

    /// A left click on the spot at `at` (design §6.1): the filters go to the
    /// next, a tab opens, a memory heading opens or shuts its block (design
    /// v40 §6.1), and a line of the log selects the sprite or egg it's
    /// about while it's in the world, as `Tab` does (§6.5).
    pub(super) fn click_spot(&mut self, at: Position, world: &World) {
        self.point(at);
        let Some(spot) = self.spot_at(at) else {
            return;
        };
        match spot {
            Spot::Filters => self.next_filter(),
            Spot::Tab(tab) => self.open(tab),
            Spot::Memory(which) => self.toggle_lesson(which),
            Spot::LogLine(id) => {
                if let Some(pos) = in_the_world(world, id) {
                    self.select_in_view(id, pos);
                }
            }
            Spot::Chemical { .. } | Spot::Gene(_) | Spot::PopUp => {}
        }
    }
}

/// Where sprite or egg `id` is, if it's in the world, living or laid:
/// `None` once it has died or been forgotten, and `Some(None)` for an egg
/// that's held, so has no tile of its own (M2 design §10.7).
pub(crate) fn in_the_world(world: &World, id: EntityId) -> Option<Option<Pos>> {
    match Known::of(world, id) {
        Known::Living(sprite) => Some(Some(sprite.pos())),
        Known::Egg(egg) => Some(egg.pos()),
        Known::Dead(_) | Known::Forgotten => None,
    }
}
