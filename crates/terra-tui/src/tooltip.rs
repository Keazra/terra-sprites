//! Tooltips (design §6.1, #304): what the pointer is over, said in a box
//! below it and to its left, and the spot under it lit up. What the event
//! log's filters do, a chemical's name and what its data says it is, what
//! a gene does, and which sprite a line of the log is about. Whatever a
//! tooltip can explain is drawn in the theme's spot style, so the player
//! can see what to point at.

use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use terra_sim::{ChemicalKind, World};

use crate::app::{App, Screen, Selection, in_the_world};
use crate::controls;
use crate::inspector;
use crate::spots::{Spot, Spots};
use crate::text::{BOUND, capitalised, display_name, fitted, wrap};
use crate::ui::{blank, draw_border};

/// The widest a tooltip is, its border included.
const BOX_WIDTH: u16 = 40;

/// What a spot says it is (design §6.1): its name, on its tooltip's top
/// edge, what it is, and what a click or a key does with it, if anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Explained {
    pub(crate) name: String,
    pub(crate) about: String,
    pub(crate) hint: Option<String>,
}

/// What `spot` says it is, if anything. A tab's name says nothing, since
/// it says what the tab shows (Fauxpaw, 2026-10-27), a memory heading says
/// nothing, since it's a heading, and a pop-up's box says nothing, so the
/// pointer on one explains nothing beneath it.
pub(crate) fn explained(spot: &Spot, app: &App, world: &World) -> Option<Explained> {
    match spot {
        Spot::Filters => Some(Explained {
            name: "Event filter".into(),
            about:
                "Which events the log shows: all of them, the selected sprite's, or the major ones"
                    .into(),
            hint: Some(format!(
                "click, or {}, for the next",
                controls::EVENT_FILTER.written
            )),
        }),
        Spot::Chemical { name, kind } => {
            let about = world
                .data()
                .chemical_about(name)
                .unwrap_or_else(|| kind_words(*kind));
            Some(Explained {
                name: display_name(name),
                about: capitalised(about),
                hint: None,
            })
        }
        // Its name, and what it does, numbers and all: "An emitter: hunger
        // rises while energy is below .5 ..."
        Spot::Gene(index) => {
            let id = app.selection()?.id();
            let sprite = world.sprite(id)?;
            let (gene, _) = sprite.genes().into_iter().nth(*index)?;
            Some(Explained {
                name: inspector::gene_name(&gene).replace(BOUND, " "),
                about: inspector::gene_about(&gene),
                hint: None,
            })
        }
        Spot::LogLine(id) => {
            let selected = app.selection().map(Selection::id) == Some(*id);
            let (about, hint) = match in_the_world(world, *id) {
                Some(_) if selected => ("The selected sprite, whom this line is about", None),
                Some(_) => ("Whom this line is about", Some("click to select it".into())),
                None => ("Whom this line is about, who has died", None),
            };
            Some(Explained {
                name: app.names().label(*id),
                about: about.into(),
                hint,
            })
        }
        Spot::Tab(_) | Spot::Memory(_) | Spot::PopUp => None,
    }
}

/// What a chemical of `kind` is, for one whose data pack doesn't say
/// (design §6.1).
fn kind_words(kind: ChemicalKind) -> &'static str {
    match kind {
        ChemicalKind::Physical => "part of its body",
        ChemicalKind::Drive => "a drive, an urge it feels",
        ChemicalKind::LearningSignal => "a feeling learning feeds on",
        ChemicalKind::Hormone => "a hormone its genes may use",
    }
}

/// Draws whatever a tooltip can explain in the theme's spot style, where
/// nothing drawn later is over it (design §6.1, §6.2), but what keeps a
/// look of its own: what's minor stays muted, so a gene with no effect
/// still reads as one until it's pointed at.
pub(crate) fn mark_spots(buf: &mut Buffer, app: &App, spots: &Spots) {
    let look = app.theme.text();
    for spot in spots.marked() {
        for cell in spots.cells_of(spot) {
            buf[cell].set_style(look.spot);
        }
    }
}

/// Where the pointer is, for its tooltip and what it lights up (design
/// §6.1): only on the world's own screen, the map with the inspector and
/// the event log, where a click or a key does what a tooltip says, and only
/// while no key has been pressed since it moved, which puts them away until
/// it moves again.
pub(crate) fn pointer(app: &App) -> Option<Position> {
    app.shown_pointer()
        .filter(|_| app.screen() == Screen::Normal)
}

/// Lights up the spot the pointer is on, all of it, a gene's name on each
/// of its lines, in the theme's spot style in reverse video (design §6.1,
/// §6.2), except where a pop-up is over it. A tab's name and a pop-up's
/// box aren't lit.
pub(crate) fn light_up_spot(buf: &mut Buffer, app: &App, spots: &Spots) {
    let Some(spot) = pointer(app).and_then(|cell| spots.at(cell)) else {
        return;
    };
    if !spot.lights_up() {
        return;
    }
    let lit = app.theme.text().spot.add_modifier(Modifier::REVERSED);
    for cell in spots.cells_of(spot) {
        buf[cell].set_style(lit);
    }
}

/// Draws the tooltip saying what the pointer is over, if it's over
/// something that says, within `room` (design §6.1): below the pointer and
/// to its left, since what's pointed at is mostly at the screen's right;
/// to its right where there's no room at its left, and above it where
/// there's none below. It never covers the cell the pointer is on, nor a
/// pop-up.
pub(crate) fn draw(buf: &mut Buffer, room: Rect, app: &App, world: &World, spots: &Spots) {
    let Some(pointer) = pointer(app) else {
        return;
    };
    let Some(said) = spots
        .at(pointer)
        .and_then(|spot| explained(spot, app, world))
    else {
        return;
    };
    let look = app.theme.text();
    // A space each side, inside the border.
    let width = usize::from(BOX_WIDTH.min(room.width).saturating_sub(4));
    let mut lines: Vec<Line> = wrap("", "", &said.about, width)
        .into_iter()
        .map(Line::raw)
        .collect();
    if let Some(hint) = &said.hint {
        lines.extend(
            wrap("", "", hint, width)
                .into_iter()
                .map(|line| Line::from(Span::styled(line, look.muted))),
        );
    }
    let widest = lines
        .iter()
        .map(Line::width)
        .chain([said.name.chars().count() + 3])
        .max()
        .unwrap_or(0);
    let size = (
        (widest as u16 + 4).min(BOX_WIDTH).min(room.width),
        (lines.len() as u16 + 2).min(room.height),
    );
    let pop_ups: Vec<Rect> = spots.pop_ups().collect();
    let Some(area) = beside(pointer, size, room, &pop_ups) else {
        return;
    };
    // Its name on its top edge, between the corner and a dash at each end
    // and a space each side, ending `...` where it's too long.
    let name = fitted(&said.name, usize::from(area.width).saturating_sub(6));
    blank(buf, area);
    draw_border(buf, area, &format!(" {name} "));
    let inner = area.inner(Margin::new(1, 1));
    for (row, line) in (inner.y..inner.bottom()).zip(&lines) {
        buf.set_line(inner.x + 1, row, line, inner.width.saturating_sub(2));
    }
}

/// Where a tooltip of `size` (its width, then its height) goes beside the
/// pointer at `pointer`, within `room` and off each of `pop_ups`: below and
/// to the left, or where that won't do, to the right, then above and to
/// the left or right. `None` if it can't go beside it without covering it
/// or a pop-up.
fn beside(
    pointer: Position,
    (width, height): (u16, u16),
    room: Rect,
    pop_ups: &[Rect],
) -> Option<Rect> {
    let left = pointer.x.checked_sub(width).filter(|&x| x >= room.x);
    let right = Some(pointer.x + 1).filter(|&x| x + width <= room.right());
    let below = Some(pointer.y + 1).filter(|&y| y + height <= room.bottom());
    let above = pointer.y.checked_sub(height).filter(|&y| y >= room.y);
    [(left, below), (right, below), (left, above), (right, above)]
        .into_iter()
        .filter_map(|(x, y)| Some(Rect::new(x?, y?, width, height)))
        .find(|area| !pop_ups.iter().any(|pop_up| pop_up.intersects(*area)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tooltip_goes_below_and_left_of_the_pointer_where_it_fits() {
        let room = Rect::new(0, 1, 100, 28);
        assert_eq!(
            beside(Position::new(60, 5), (20, 4), room, &[]),
            Some(Rect::new(40, 6, 20, 4))
        );
    }

    #[test]
    fn near_the_left_and_the_bottom_it_goes_right_and_above() {
        let room = Rect::new(0, 1, 100, 28);
        assert_eq!(
            beside(Position::new(10, 27), (20, 4), room, &[]),
            Some(Rect::new(11, 23, 20, 4))
        );
    }

    #[test]
    fn it_keeps_off_a_pop_up_going_where_it_would_have_gone_without() {
        let room = Rect::new(0, 1, 100, 28);
        let pointer = Position::new(60, 5);
        // Below and to the left is the debug overlay's, say.
        let pop_up = Rect::new(30, 5, 25, 10);
        assert_eq!(
            beside(pointer, (20, 4), room, &[pop_up]),
            Some(Rect::new(61, 6, 20, 4)),
            "to the right"
        );
        let wide = Rect::new(30, 5, 60, 10);
        assert_eq!(
            beside(pointer, (20, 4), room, &[wide]),
            Some(Rect::new(40, 1, 20, 4)),
            "above"
        );
        assert_eq!(
            beside(pointer, (20, 4), room, &[Rect::new(0, 1, 100, 28)]),
            None
        );
    }

    #[test]
    fn it_never_covers_the_pointer() {
        let room = Rect::new(0, 1, 30, 6);
        for x in 0..30 {
            for y in 1..7 {
                let pointer = Position::new(x, y);
                if let Some(area) = beside(pointer, (20, 4), room, &[]) {
                    assert!(!area.contains(pointer), "{pointer:?} {area:?}");
                    assert_eq!(area.intersection(room), area, "{pointer:?}");
                }
            }
        }
    }
}
