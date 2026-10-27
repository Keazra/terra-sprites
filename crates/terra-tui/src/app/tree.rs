//! The family tree's screen (M2 design §17): choosing a sprite on it, and
//! walking the tree to it.

use ratatui::layout::{Margin, Position};
use terra_sim::World;

use super::{App, Direction, Flow, Screen, Selection};
use crate::family_tree::{self, Step};
use crate::input::{Action, Button};

impl App {
    /// What an action does while the family tree is open (M2 design §17.3):
    /// the arrows or the pointer choose a sprite on it, and `Enter` or a
    /// click centres the tree on that one and selects it; `Tab` selects as
    /// ever, time runs as usual, `?` opens the help at the tree's page
    /// (#303), and `k` or `Esc` closes it.
    pub(super) fn apply_in_tree(&mut self, action: Action, world: &World) -> Flow {
        if self.apply_time(&action, world) {
            return Flow::Continue;
        }
        match action {
            Action::Scroll { dx, dy } => {
                let step = match (dx.signum(), dy.signum()) {
                    (-1, _) => Step::Left,
                    (1, _) => Step::Right,
                    (_, -1) => Step::Up,
                    (_, 1) => Step::Down,
                    _ => return Flow::Continue,
                };
                if let Some(tree) = self.tree(world)
                    && let Some(to) = tree.step(tree.chosen, step)
                {
                    tree.keep_in_view(&mut self.tree_scroll, to);
                    self.tree_choice = Some(to);
                }
            }
            // The pointer chooses a sprite on the tree; it leaves the Cursor
            // where it is on the map under the tree, as the population
            // graph's does (M2 design §14.2).
            Action::Point(cell) => {
                if let Some(seat) = self.tree_card_at(cell, world) {
                    self.tree_choice = Some(seat);
                }
            }
            Action::Click {
                at,
                button: Button::Left,
                ..
            } => {
                if let Some(seat) = self.tree_card_at(at, world) {
                    self.tree_choice = Some(seat);
                    self.walk_tree(world);
                }
            }
            Action::Enter => self.walk_tree(world),
            Action::SelectNext => self.redraw_tree_along(world, Direction::Next),
            Action::SelectPrevious => self.redraw_tree_along(world, Direction::Previous),
            Action::FamilyTree | Action::Back => self.screen = Screen::Normal,
            Action::Help => self.open_help(),
            _ => {}
        }
        Flow::Continue
    }

    /// Selects the next or previous sprite, as `Tab` does, and redraws the
    /// family tree around it, from the middle (M2 design §17.2).
    fn redraw_tree_along(&mut self, world: &World, direction: Direction) {
        self.select_along(world, direction);
        self.tree_choice = None;
        self.tree_scroll = family_tree::Scroll::default();
    }

    /// The family tree of the selected sprite or egg, living or dead, as
    /// the screen lays it out now, if one is selected, the policy shows it,
    /// and there's room (M2 design §17.2).
    pub(crate) fn tree(&self, world: &World) -> Option<family_tree::Drawn> {
        let centre = self.selection?.id();
        if !family_tree::shown(self, centre) {
            return None;
        }
        let inner = self.overlay?.inner(Margin::new(1, 1));
        Some(family_tree::draw(
            &family_tree::FamilyTree::of(self, world, centre),
            usize::from(inner.width),
            self.tree_scroll,
            self.tree_choice,
            self.flash_on(),
            self.theme.text().muted,
        ))
    }

    /// The card on the family tree `at` is over, if any.
    fn tree_card_at(&self, at: Position, world: &World) -> Option<family_tree::Seat> {
        let inner = self.overlay?.inner(Margin::new(1, 1));
        if !inner.contains(at) {
            return None;
        }
        let tree = self.tree(world)?;
        tree.at(i32::from(at.x - inner.x), usize::from(at.y - inner.y))
    }

    /// Walks the family tree to the sprite chosen on it (M2 design §17.3):
    /// the tree centres on it and it's selected, and the map centres on it
    /// if it's on the map. A dead one is selected as one that died while
    /// selected is. One the world no longer remembers can't be, nor one
    /// the policy hides.
    fn walk_tree(&mut self, world: &World) {
        let Some(tree) = self.tree(world) else {
            return;
        };
        let to = tree.chosen.id;
        // A card the policy hides is blank, and leads nowhere (M2 design
        // §17.6).
        if self.selection.map(Selection::id) == Some(to) || !family_tree::shown(self, to) {
            return;
        }
        match family_tree::Known::of(world, to) {
            family_tree::Known::Living(sprite) => {
                self.select(to);
                self.centre_on(sprite.pos());
            }
            family_tree::Known::Egg(egg) => {
                self.select(to);
                if let Some(pos) = egg.pos() {
                    self.centre_on(pos);
                }
            }
            family_tree::Known::Dead(dead) => self.select_as(Selection::Dead {
                id: to,
                cause: dead.cause(),
                age: dead.age(),
            }),
            family_tree::Known::Forgotten => {
                return self.refuse(format!(
                    "{} lived long ago: the world no longer remembers it",
                    self.names.label(to)
                ));
            }
        }
        self.tree_choice = None;
        self.tree_scroll = family_tree::Scroll::default();
        self.settle_cursor(world);
    }
}
