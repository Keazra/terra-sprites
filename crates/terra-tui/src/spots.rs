//! What each part of the screen is, where it's drawn: its spots (design
//! §6.1, #304). The inspector, the event log and each pop-up note theirs
//! as they're drawn, and the list is made afresh each frame, so a tooltip
//! says what the pointer is over, and a click lands on what the player saw
//! under it, never on where it was a frame or a layout before.
//! A spot noted later comes first, so a pop-up, drawn over the panels,
//! takes the pointer before what's under it.

use ratatui::layout::{Position, Rect};
use terra_sim::{ChemicalKind, EntityId};

use crate::app::Tab;
use crate::inspector::MemoryOpen;

/// Something on screen the pointer can be on, which a tooltip can say
/// something about, or a click lands on (design §6.1).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Spot {
    /// An inspector tab's name, in its title row.
    Tab(Tab),
    /// The event log's filters, at the right of its title row.
    Filters,
    /// A chemical's name in the inspector: a drive's on the Body tab, or a
    /// chemical's on the Chem tab. By its name in the data pack.
    Chemical { name: String, kind: ChemicalKind },
    /// A gene's name on the Genome tab, the words of its line but its
    /// numbers, by its place in the genome.
    Gene(usize),
    /// A line of the Brain tab that opens or shuts a memory block (design
    /// v40 §6.1).
    Memory(MemoryOpen),
    /// The name in a line of the event log of the sprite or egg it's
    /// about: the first it names.
    LogLine(EntityId),
    /// A pop-up's box, over whatever is drawn beneath it.
    PopUp,
}

impl Spot {
    /// The spot of the chemical named `name` in the data pack.
    pub(crate) fn chemical(name: &str, kind: ChemicalKind) -> Spot {
        Spot::Chemical {
            name: name.to_string(),
            kind,
        }
    }

    /// Whether a left click on it is its own, rather than the map's under
    /// it: everything but what only says what it is. A pop-up's box takes
    /// the click, so it never lands on the map beneath (design §6.1).
    pub(crate) fn takes_a_click(&self) -> bool {
        !matches!(self, Spot::Chemical { .. } | Spot::Gene(_))
    }

    /// Whether it says what it is, in a tooltip, and so is drawn in the
    /// theme's spot style (design §6.1, §6.2): all but a memory heading,
    /// which is a heading, and a pop-up's box.
    pub(crate) fn explains(&self) -> bool {
        !matches!(self, Spot::Memory(_) | Spot::PopUp)
    }
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

    /// The screen cells where `spot` is drawn and nothing drawn later is
    /// over it: all of it, a gene's name on each of its lines.
    pub(crate) fn cells_of<'a>(&'a self, spot: &'a Spot) -> impl Iterator<Item = Position> + 'a {
        self.0
            .iter()
            .filter(move |(_, noted)| noted == spot)
            .flat_map(|(area, _)| area.positions())
            .filter(move |&cell| self.at(cell) == Some(spot))
    }

    /// Every spot noted, in the order they were drawn.
    pub(crate) fn all(&self) -> impl Iterator<Item = &Spot> {
        self.0.iter().map(|(_, spot)| spot)
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
    fn a_spot_s_cells_are_those_nothing_drawn_later_covers() {
        let mut spots = Spots::default();
        spots.note(Rect::new(0, 0, 4, 1), Spot::Gene(2));
        spots.note(Rect::new(1, 1, 2, 1), Spot::Gene(2));
        spots.note(Rect::new(3, 0, 2, 2), Spot::PopUp);
        let cells: Vec<Position> = spots.cells_of(&Spot::Gene(2)).collect();
        assert_eq!(
            cells,
            [(0, 0), (1, 0), (2, 0), (1, 1), (2, 1)].map(Position::from)
        );
    }

    #[test]
    fn nothing_is_noted_where_nothing_is_drawn() {
        let mut spots = Spots::default();
        spots.note(Rect::new(3, 3, 0, 1), Spot::Filters);
        assert_eq!(spots.at(Position::new(3, 3)), None);
    }
}
