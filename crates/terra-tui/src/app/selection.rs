//! The selected sprite and the inspector (design §6.1): selecting, the
//! open tab and how far it's scrolled, and the Brain tab's open memories
//! (v40).

use std::collections::BTreeSet;

use ratatui::layout::{Position, Rect};
use terra_sim::{EntityId, World};

use super::{App, Direction, Selection, Tab};
use crate::inspector;

impl App {
    /// The sprite the inspector shows, if one is selected.
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// The open inspector tab.
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// How many lines the open tab is scrolled down.
    pub fn tab_scroll(&self) -> usize {
        self.tab_scroll
    }

    /// Selects the sprite `id`. From the World tab, that opens Body; and
    /// another sprite than before shows its tab from the top.
    pub(super) fn select(&mut self, id: EntityId) {
        self.select_as(Selection::Living(id));
    }

    /// Selects `selection`, living or dead, as `select` does.
    pub(super) fn select_as(&mut self, selection: Selection) {
        let id = selection.id();
        let another = self.selection.map(Selection::id) != Some(id);
        if self.tab == Tab::World {
            self.open(Tab::Body);
        } else if another {
            self.tab_scroll = 0;
        }
        if another {
            self.observed.clear();
            self.open_lessons.clear();
        }
        self.selection = Some(selection);
    }

    /// Opens `tab`, from the top.
    pub(super) fn open(&mut self, tab: Tab) {
        self.tab = tab;
        self.tab_scroll = 0;
    }

    /// The rows a tab has in the inspector: a page.
    pub(super) fn inspector_rows(&self) -> usize {
        self.inspector
            .map_or(0, |area| usize::from(inspector::tab_area(area).height))
    }

    /// Scrolls the open tab by `lines` from where it's shown, down being
    /// positive, stopping at the top and where its last line comes into view.
    pub(super) fn scroll_tab(&mut self, lines: i32, world: &World) {
        let (length, rows) = (inspector::lines(self, world).len(), self.inspector_rows());
        let from = inspector::first_shown(self.tab_scroll, length, rows);
        let furthest = inspector::first_shown(usize::MAX, length, rows);
        let scrolled = from as i64 + i64::from(lines);
        self.tab_scroll = scrolled.clamp(0, furthest as i64) as usize;
    }

    /// Selects the sprite with the next or previous ID, wrapping around, and
    /// centres the viewport on it if it's out of view. With nothing selected,
    /// it starts from the lowest or highest ID.
    pub(super) fn select_along(&mut self, world: &World, direction: Direction) {
        let ids: Vec<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
        let current = self.selection.map(Selection::id);
        let chosen = match direction {
            Direction::Next => current
                .and_then(|current| ids.iter().find(|&&id| id > current))
                .or(ids.first()),
            Direction::Previous => current
                .and_then(|current| ids.iter().rev().find(|&&id| id < current))
                .or(ids.last()),
        };
        let Some(&id) = chosen else {
            return;
        };
        self.select(id);
        let pos = world.sprite(id).expect("a sprite just listed").pos();
        if self.cell_of(pos).is_none() {
            self.centre_on(pos);
        }
    }

    /// The inspector, its title row included, if the screen has room for it.
    pub(crate) fn inspector_bounds(&self) -> Option<Rect> {
        self.inspector
    }

    /// Which memory block is open on the Brain tab.
    pub(crate) fn lessons_open(&self) -> &BTreeSet<inspector::MemoryOpen> {
        &self.open_lessons
    }

    /// Opens or shuts the memory block under `at`, if that line opens one.
    /// A click on a lesson line, or off the Brain tab, does nothing.
    pub(super) fn toggle_lesson(&mut self, at: Position, world: &World) -> bool {
        let Some(which) = inspector::lesson_at(self, world, at) else {
            return false;
        };
        if !self.open_lessons.remove(&which) {
            self.open_lessons.insert(which);
        }
        true
    }
}
