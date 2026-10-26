//! What each part of the screen is, where it's drawn: its spots (#304).
//! Drawing notes them as it goes, and the list is made afresh each frame,
//! so the status line says what the pointer is over, and a click lands on
//! what the player saw under it, never on where it was a frame or a layout
//! before. A spot noted later comes first, so a pop-up, drawn over the
//! panels, takes the pointer before what's under it.

use ratatui::layout::{Position, Rect};
use terra_sim::{ChemicalKind, EntityId};

use crate::app::Tab;

/// Something on screen the pointer can be on, which the status line can
/// say something about (design §6.1, #304).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Spot {
    /// An inspector tab's name, in its title row.
    Tab(Tab),
    /// The event log's filters, at the right of its title row.
    Filters,
    /// A chemical's line in the inspector, or its cell where several share
    /// a line: a drive's bar on the Body tab, or a chemical on the Chem
    /// tab. By its name in the data pack.
    Chemical { name: String, kind: ChemicalKind },
    /// A gene's lines on the Genome tab, by its place in the genome.
    Gene(usize),
    /// A line of the event log, by the sprite it's about: the first it
    /// names.
    Event(EntityId),
    /// A pop-up's box, over whatever is drawn beneath it.
    PopUp,
}

/// The spots of one frame, in the order they were drawn.
#[derive(Debug, Default)]
pub(crate) struct Spots(Vec<(Rect, Spot)>);

impl Spots {
    /// Notes that `spot` is drawn over `area`.
    pub(crate) fn note(&mut self, area: Rect, spot: Spot) {
        if !area.is_empty() {
            self.0.push((area, spot));
        }
    }

    /// The spot drawn at screen cell `cell`, if any: the one drawn last
    /// there, so a pop-up's before what's under it.
    pub(crate) fn at(&self, cell: Position) -> Option<&Spot> {
        self.0
            .iter()
            .rev()
            .find(|(area, _)| area.contains(cell))
            .map(|(_, spot)| spot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_drawn_later_comes_before_what_is_under_it() {
        let mut spots = Spots::default();
        spots.note(Rect::new(0, 0, 10, 1), Spot::Filters);
        spots.note(Rect::new(4, 0, 3, 3), Spot::PopUp);
        assert_eq!(spots.at(Position::new(1, 0)), Some(&Spot::Filters));
        assert_eq!(spots.at(Position::new(5, 0)), Some(&Spot::PopUp));
        assert_eq!(spots.at(Position::new(5, 2)), Some(&Spot::PopUp));
        assert_eq!(spots.at(Position::new(8, 2)), None);
    }

    #[test]
    fn nothing_is_noted_where_nothing_is_drawn() {
        let mut spots = Spots::default();
        spots.note(Rect::new(3, 3, 0, 1), Spot::Filters);
        assert_eq!(spots.at(Position::new(3, 3)), None);
    }
}
