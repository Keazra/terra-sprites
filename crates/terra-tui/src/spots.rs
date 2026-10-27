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
/// something about, pointing at lights up, or a click lands on (design
/// §6.1).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Spot {
    /// An inspector tab's name, in its title row, which takes a click and
    /// says nothing, since a tab's name says what it shows (Fauxpaw,
    /// 2026-10-27).
    Tab(Tab),
    /// The event log's filters, at the right of its title row.
    Filters,
    /// A chemical's name in the inspector: a drive's on the Body tab, or a
    /// chemical's on the Chem tab. By its name in the data pack.
    Chemical { name: String, kind: ChemicalKind },
    /// A gene's name on the Genome tab, the words of its line but its
    /// numbers, or an unknown gene's whole line, by its place in the
    /// genome.
    Gene(usize),
    /// A line of the Brain tab that opens or shuts a memory block (design
    /// v40 §6.1).
    Memory(MemoryOpen),
    /// The name in a line of the event log of the sprite or egg it's
    /// about, the first it names, while it's in the world: pointing at it
    /// lights its sprite up on the map, and a click selects it. It says
    /// nothing, since its name says whom the line is about (Fauxpaw,
    /// 2026-10-27).
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

    /// Whether it's drawn in the theme's spot style, unless it keeps a look
    /// of its own (design §6.1, §6.2): what a tooltip says something about,
    /// and the name of a log line's sprite, which pointing at lights the
    /// sprite up on the map. Not a tab's name, a memory heading, which is a
    /// heading, nor a pop-up's box.
    pub(crate) fn coloured(&self) -> bool {
        !matches!(self, Spot::Tab(_) | Spot::Memory(_) | Spot::PopUp)
    }

    /// Whether the pointer on it lights it up (design §6.1): all but a
    /// tab's name, which keeps the look it had, and a pop-up's box.
    pub(crate) fn lights_up(&self) -> bool {
        !matches!(self, Spot::Tab(_) | Spot::PopUp)
    }
}

/// The spots of one frame, in the order they were drawn, each with whether
/// it's marked in the theme's spot style; and the pop-ups' boxes, each with
/// whether it fades in and out (design §6.9).
#[derive(Debug, Default)]
pub(crate) struct Spots(Vec<(Rect, Spot, bool)>, Vec<(Rect, bool)>);

impl Spots {
    /// Notes that `spot` is drawn over `area`, marked in the theme's spot
    /// style if it's of a kind drawn in it (design §6.2). A pop-up's box is
    /// noted with `note_pop_up`.
    pub(crate) fn note(&mut self, area: Rect, spot: Spot) {
        debug_assert_ne!(spot, Spot::PopUp, "a pop-up is noted with note_pop_up");
        self.noted(area, spot, true);
    }

    /// Notes that `spot` is drawn over `area` in a look of its own, which
    /// it keeps until the pointer is on it (design §6.2): what's minor
    /// stays muted, as a gene with no effect does.
    pub(crate) fn note_unmarked(&mut self, area: Rect, spot: Spot) {
        self.noted(area, spot, false);
    }

    fn noted(&mut self, area: Rect, spot: Spot, marked: bool) {
        if !area.is_empty() {
            self.0.push((area, spot, marked));
        }
    }

    /// Notes a pop-up's box over `area`, over whatever is drawn beneath it,
    /// which `fades` in as it opens and out as it closes, as all but the
    /// debug overlay do (design §6.9).
    pub(crate) fn note_pop_up(&mut self, area: Rect, fades: bool) {
        if !area.is_empty() {
            self.noted(area, Spot::PopUp, false);
            self.1.push((area, fades));
        }
    }

    /// The spot drawn at screen cell `cell`, if any: the one drawn last
    /// there, so a pop-up's before what's under it.
    pub(crate) fn at(&self, cell: Position) -> Option<&Spot> {
        self.0
            .iter()
            .rev()
            .find(|(area, ..)| area.contains(cell))
            .map(|(_, spot, _)| spot)
    }

    /// The screen cells where `spot` is drawn and nothing drawn later is
    /// over it: all of it, a gene's name on each of its lines.
    pub(crate) fn cells_of<'a>(&'a self, spot: &'a Spot) -> impl Iterator<Item = Position> + 'a {
        self.0
            .iter()
            .filter(move |(_, noted, _)| noted == spot)
            .flat_map(|(area, ..)| area.positions())
            .filter(move |&cell| self.at(cell) == Some(spot))
    }

    /// Every spot noted to be marked in the theme's spot style, in the
    /// order they were drawn: those of a kind drawn in it, but what keeps
    /// a look of its own.
    pub(crate) fn marked(&self) -> impl Iterator<Item = &Spot> {
        self.0
            .iter()
            .filter(|(_, spot, marked)| *marked && spot.coloured())
            .map(|(_, spot, _)| spot)
    }

    /// Where each of the pop-ups is drawn, which no tooltip covers (design
    /// §6.1).
    pub(crate) fn pop_ups(&self) -> impl Iterator<Item = Rect> + '_ {
        self.1.iter().map(|&(area, _)| area)
    }

    /// Where each of the pop-ups that fade in and out is drawn (design
    /// §6.9).
    pub(crate) fn fading(&self) -> impl Iterator<Item = Rect> + '_ {
        self.1
            .iter()
            .filter(|&&(_, fades)| fades)
            .map(|&(area, _)| area)
    }

    /// Whether a pop-up is drawn over screen cell `cell`.
    pub(crate) fn under_a_pop_up(&self, cell: Position) -> bool {
        self.pop_ups().any(|area| area.contains(cell))
    }

    /// Whether the pop-up drawn last over screen cell `cell`, if any, is
    /// one that fades.
    pub(crate) fn fades_at(&self, cell: Position) -> bool {
        self.1
            .iter()
            .rev()
            .find(|(area, _)| area.contains(cell))
            .is_some_and(|&(_, fades)| fades)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_drawn_later_comes_before_what_is_under_it() {
        let mut spots = Spots::default();
        spots.note(Rect::new(0, 0, 10, 1), Spot::Filters);
        spots.note_pop_up(Rect::new(4, 0, 3, 3), true);
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
        spots.note_pop_up(Rect::new(3, 0, 2, 2), true);
        let cells: Vec<Position> = spots.cells_of(&Spot::Gene(2)).collect();
        assert_eq!(
            cells,
            [(0, 0), (1, 0), (2, 0), (1, 1), (2, 1)].map(Position::from)
        );
    }

    #[test]
    fn only_what_is_coloured_and_keeps_no_look_of_its_own_is_marked() {
        let mut spots = Spots::default();
        spots.note(Rect::new(0, 0, 4, 1), Spot::Gene(2));
        spots.note_unmarked(Rect::new(0, 1, 4, 1), Spot::Gene(3));
        spots.note_pop_up(Rect::new(0, 2, 4, 1), true);
        let marked: Vec<&Spot> = spots.marked().collect();
        assert_eq!(marked, [&Spot::Gene(2)]);
        assert_eq!(spots.at(Position::new(1, 1)), Some(&Spot::Gene(3)));
        assert_eq!(spots.pop_ups().collect::<Vec<_>>(), [Rect::new(0, 2, 4, 1)]);
    }

    #[test]
    fn of_the_pop_ups_all_but_the_debug_overlay_fade() {
        // Design §6.9: the debug overlay is drawn over whatever's open.
        let mut spots = Spots::default();
        spots.note_pop_up(Rect::new(0, 0, 6, 4), true);
        spots.note_pop_up(Rect::new(4, 0, 4, 2), false);
        assert_eq!(spots.pop_ups().count(), 2);
        assert_eq!(spots.fading().collect::<Vec<_>>(), [Rect::new(0, 0, 6, 4)]);
        assert!(spots.fades_at(Position::new(1, 1)));
        assert!(!spots.fades_at(Position::new(5, 1)), "under the overlay");
        assert!(!spots.fades_at(Position::new(9, 9)), "under no pop-up");
        assert!(spots.under_a_pop_up(Position::new(5, 1)));
        assert!(!spots.under_a_pop_up(Position::new(9, 9)));
    }

    #[test]
    fn nothing_is_noted_where_nothing_is_drawn() {
        let mut spots = Spots::default();
        spots.note(Rect::new(3, 3, 0, 1), Spot::Filters);
        assert_eq!(spots.at(Position::new(3, 3)), None);
    }
}
