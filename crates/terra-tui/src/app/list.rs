//! The sprite list (design §6.1): its highlight, its order, its filter
//! (#313), and going to the sprite picked.

use ratatui::layout::{Margin, Position, Rect};
use terra_sim::{EntityId, World};

use super::{App, Draft, Flow, Screen, Selection, along, highlight_moved, typeable};
use crate::input::{Action, Button};
use crate::sprite_list::{self, SortBy};

impl App {
    /// Opens the sprite list, highlighting the selected sprite if it's
    /// listed, or else the first.
    pub(super) fn open_list(&mut self, world: &World) {
        self.screen = Screen::SpriteList;
        self.list_filter = None;
        let listed = sprite_list::order(self, world);
        self.list_choice = self
            .selection
            .map(Selection::id)
            .filter(|id| listed.contains(id))
            .or_else(|| listed.first().copied());
    }

    /// What an action does while the sprite list is open (design §6.1): the
    /// arrow keys or the wheel move the highlight, `Tab` changes the order,
    /// `Enter` or a click on a row goes to that sprite, `/` starts a filter
    /// (#313), and `l` or `Esc` closes the list.
    pub(super) fn apply_in_list(&mut self, action: Action, world: &World) -> Flow {
        let Some(action) = self.apply_in_filter(action, world) else {
            return Flow::Continue;
        };
        let ids = sprite_list::order(self, world);
        let choice = self.list_row(world);
        let moved = |by: i32| ids.get(highlight_moved(choice, by, ids.len())).copied();
        match action {
            Action::Scroll { dy, .. } => self.list_choice = moved(dy.signum()),
            Action::Wheel { at, notches } => {
                self.point(at);
                self.list_choice = moved(notches);
            }
            Action::SelectNext => self.list_sort = along(&SortBy::ALL, self.list_sort, 1),
            Action::SelectPrevious => self.list_sort = along(&SortBy::ALL, self.list_sort, -1),
            Action::Enter => {
                if let Some(&id) = ids.get(self.list_row(world)) {
                    self.go_to(id, world);
                }
            }
            Action::Click {
                at,
                button: Button::Left,
                ..
            } => {
                self.point(at);
                let row = self.list_row_at(at);
                if let Some(&id) = row.and_then(|row| ids.get(self.list_first(world) + row)) {
                    self.go_to(id, world);
                }
            }
            Action::Point(cell) => self.point(cell),
            Action::FilterList => self.list_filter = Some(Draft::offered(String::new())),
            Action::SpriteList | Action::Back => self.screen = Screen::Normal,
            _ => {}
        }
        Flow::Continue
    }

    /// What an action does while the sprite list's filter is typed (#313):
    /// characters CP437 can show are typed, up to the most it takes,
    /// `Backspace` rubs one out and `Esc` clears it, leaving the list open.
    /// Anything else, with `Tab` as the next order, is the list's to act
    /// on, and returned; so is any action with no filter.
    fn apply_in_filter(&mut self, action: Action, world: &World) -> Option<Action> {
        let Some(filter) = self.list_filter.as_mut() else {
            return Some(action);
        };
        match action {
            Action::Type(c) if typeable(c) => filter.type_char(c, sprite_list::MAX_FILTER_CHARS),
            Action::Erase => filter.erase(),
            Action::Back => {
                self.list_filter = None;
                return None;
            }
            Action::AnotherName => return Some(Action::SelectNext),
            // A character the screen can't show, or `?` let go of.
            Action::Type(_) | Action::HelpReleased => return None,
            other => return Some(other),
        }
        // The highlight stays on its sprite while it matches, or else goes to
        // the first that does.
        let ids = sprite_list::order(self, world);
        if !self.list_choice.is_some_and(|chosen| ids.contains(&chosen)) {
            self.list_choice = ids.first().copied();
        }
        None
    }

    /// Selects sprite `id`, centres the view on it and closes the sprite
    /// list.
    fn go_to(&mut self, id: EntityId, world: &World) {
        self.screen = Screen::Normal;
        self.list_filter = None;
        self.select(id);
        if let Some(sprite) = world.sprite(id) {
            self.centre_on(sprite.pos());
        }
        self.settle_cursor(world);
    }

    /// Which of the sprite list's rows `at` is on, from 0 for the first
    /// shown, if it's on one.
    fn list_row_at(&self, at: Position) -> Option<usize> {
        let inner = self.overlay?.inner(Margin::new(1, 1));
        // The heading takes the first row.
        let rows = Rect::new(
            inner.x,
            inner.y + 1,
            inner.width,
            inner.height.saturating_sub(1),
        );
        rows.contains(at).then(|| usize::from(at.y - rows.y))
    }

    /// What the sprite list is sorted by.
    pub fn list_sort(&self) -> SortBy {
        self.list_sort
    }

    /// What's typed into the sprite list's filter, while it's filtered
    /// (#313).
    pub fn list_filter(&self) -> Option<&str> {
        self.list_filter.as_ref().map(Draft::text)
    }

    /// Which row of the sprite list is highlighted, from 0, in its current
    /// order.
    pub fn list_row(&self, world: &World) -> usize {
        let ids = sprite_list::order(self, world);
        self.list_choice
            .and_then(|chosen| ids.iter().position(|&id| id == chosen))
            .unwrap_or(0)
    }

    /// How many sprites the sprite list shows at once: the rows inside its
    /// border, less the heading.
    pub fn list_rows(&self) -> usize {
        self.overlay.map_or(0, |area| {
            usize::from(area.inner(Margin::new(1, 1)).height.saturating_sub(1))
        })
    }

    /// The first row the sprite list shows: it scrolls so the highlighted
    /// sprite stays in view.
    pub fn list_first(&self, world: &World) -> usize {
        self.list_row(world)
            .saturating_sub(self.list_rows().max(1) - 1)
    }
}
