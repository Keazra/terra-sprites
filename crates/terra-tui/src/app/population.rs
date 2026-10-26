//! The population graph's screen (M2 design §14): picking the moment it
//! reads.

use ratatui::layout::{Margin, Position, Rect};
use terra_sim::World;

use super::{App, Flow, Screen};
use crate::graph;
use crate::input::{Action, Button};
use crate::policy::{Panel, Subject};

impl App {
    /// What an action does while the population graph is open (M2 design
    /// §14): `←` and `→`, or the pointer over the graph, pick the moment it
    /// reads; time runs as usual, `?` opens the help at the graph's page
    /// (#303), and `p` or `Esc` closes it.
    pub(super) fn apply_in_graph(&mut self, action: Action, world: &World) -> Flow {
        if self.apply_time(&action, world) {
            return Flow::Continue;
        }
        match action {
            Action::Scroll { dx, .. } if dx != 0 => {
                if let Some(plot) = self.graph_plot(world) {
                    let from = plot.column_of(self.graph_pick);
                    let last = plot.reached().saturating_sub(1);
                    let to = from.saturating_add_signed(dx.signum() as isize).min(last);
                    self.graph_pick = plot.moment(to);
                }
            }
            // The pointer picks a moment; it leaves the Cursor where it is
            // on the map under the graph.
            Action::Point(cell)
            | Action::Click {
                at: cell,
                button: Button::Left,
                ..
            } => {
                if let Some((plot, column)) = self.graph_column_at(cell, world) {
                    self.graph_pick = plot.moment(column);
                }
            }
            Action::Population | Action::Back => {
                self.screen = Screen::Normal;
                self.graph_pick = None;
            }
            Action::Help => self.open_help(),
            _ => {}
        }
        Flow::Continue
    }

    /// The population graph as the screen lays it out now, if there's room
    /// for it and the policy shows it (M2 design §14.6).
    fn graph_plot(&self, world: &World) -> Option<graph::Plot> {
        if !self.can_view(Panel::Graph, Subject::World) {
            return None;
        }
        let inner = self.overlay?.inner(Margin::new(1, 1));
        Some(graph::Plot::full(world, usize::from(inner.width)))
    }

    /// The population graph and which of its columns `at` is over, if it's
    /// over one the run has reached, on the graph's rows or its axis.
    fn graph_column_at(&self, at: Position, world: &World) -> Option<(graph::Plot, usize)> {
        let inner = self.overlay?.inner(Margin::new(1, 1));
        let plot = self.graph_plot(world)?;
        let rows = graph::rows_above_axis(usize::from(inner.height)) as u16 + 1;
        let graph = Rect::new(
            inner.x + plot.gutter() as u16,
            inner.y,
            inner.width.saturating_sub(plot.gutter() as u16),
            rows,
        );
        let column = usize::from(at.x.checked_sub(graph.x)?);
        (graph.contains(at) && column < plot.reached()).then_some((plot, column))
    }

    /// The moment the population graph reads, the end of the stretch of
    /// time picked, if one is; otherwise it reads now (M2 design §14.3).
    pub fn graph_pick(&self) -> Option<u64> {
        self.graph_pick
    }
}
