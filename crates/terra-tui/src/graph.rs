//! The population graph (M2 design §14): the world's population over its
//! whole run, stacked by life stage in the theme's colours, full screen on
//! `p` and small at the top of the World tab. Each column is a stretch of
//! time, showing the count at its end; until the run fills the graph, a
//! column is 100 ticks and the graph grows from the left.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::{Changes, DataPack, LifeStage, PopulationCount, World};

use crate::app::App;
use crate::text::{cause_name, group_thousands};
use crate::theme::PopulationLook;

/// The ticks a column stands for while the run is too short to fill the
/// graph: as often as the world counts at first (M2 design §14).
const TICKS_PER_COLUMN: u64 = 100;

/// The life stages, stacked from the bottom (M2 design §14).
const STACK: [LifeStage; 4] = [
    LifeStage::Young,
    LifeStage::Adult,
    LifeStage::Old,
    LifeStage::Egg,
];

/// Rows the full graph keeps for what's under it: the axis, its ticks, a
/// blank line, the legend and the two lines that read a moment.
const UNDER: usize = 6;

/// Rows of the small graph on the World tab.
const SMALL_ROWS: usize = 6;

/// The world's population laid out over a number of columns.
pub(crate) struct Plot {
    /// The world's counts, then the population now, if it's later.
    points: Vec<PopulationCount>,
    /// For each column, the point it shows: the latest by its end. `None`
    /// for a column the run hasn't reached.
    columns: Vec<Option<usize>>,
    /// The first count's tick, and the ticks the columns stand for
    /// together.
    start: u64,
    span: u64,
    /// The sprites the vertical axis reaches: the world's limit, or more if
    /// it ever held more.
    top: u32,
    limit: u32,
}

impl Plot {
    /// The population of `world` over its whole run, `width` columns wide
    /// with the labels left of it.
    pub(crate) fn new(world: &World, width: usize) -> Plot {
        let mut points = world.population().to_vec();
        let now = world.population_now();
        if points.last().is_none_or(|last| last.tick < now.tick) {
            points.push(now);
        }
        let limit = u32::from(world.most_sprites());
        let most = points
            .iter()
            .map(PopulationCount::sprites)
            .max()
            .unwrap_or(0);
        let top = limit.max(most).max(1);
        let start = points[0].tick;
        let end = points.last().map_or(start, |last| last.tick);
        let width = width.saturating_sub(gutter(top)).max(1);
        let mut plot = Plot {
            columns: vec![None; width],
            start,
            span: (end - start).max(width as u64 * TICKS_PER_COLUMN),
            top,
            limit,
            points: Vec::new(),
        };
        for j in 0..width {
            if j == 0 || plot.boundary(j) < end {
                let last = plot.boundary(j + 1);
                plot.columns[j] = points.iter().rposition(|p| p.tick <= last);
            }
        }
        plot.points = points;
        plot
    }

    /// The tick column `j` begins at; it ends where the next begins.
    fn boundary(&self, j: usize) -> u64 {
        let width = self.columns.len() as u128;
        self.start + (u128::from(self.span) * j as u128 / width) as u64
    }

    /// How many columns the run has reached.
    pub(crate) fn reached(&self) -> usize {
        self.columns
            .iter()
            .filter(|column| column.is_some())
            .count()
    }

    /// The width of the labels left of the graph, with the axis: where its
    /// first column is.
    pub(crate) fn gutter(&self) -> usize {
        gutter(self.top)
    }

    /// The graph's `rows` rows, each with its label, in `look`.
    fn rows(&self, rows: usize, look: &PopulationLook) -> Vec<Line<'static>> {
        let rows = rows.max(1);
        let digits = self.top.to_string().len();
        let middle = rows / 2;
        // Each row is two half tiles, each `step` sprites high.
        let step = f64::from(self.top) / (2 * rows) as f64;
        // The row the limit falls in.
        let limit_row =
            rows - ((f64::from(self.limit) / step / 2.0).ceil() as usize).clamp(1, rows);
        (0..rows)
            .map(|row| {
                let label = if row == 0 {
                    self.top.to_string()
                } else if row == middle && rows > 2 {
                    ((rows - row) as u32 * self.top / rows as u32).to_string()
                } else {
                    String::new()
                };
                let axis = if label.is_empty() { '│' } else { '┤' };
                let mut spans = vec![Span::raw(format!(" {label:>digits$} {axis}"))];
                let lower = 2 * (rows - 1 - row);
                for column in &self.columns {
                    let count = column.map(|i| &self.points[i]);
                    let at = |half: usize| count.and_then(|c| stage_at(c, half, step));
                    spans.push(cell(at(lower), at(lower + 1), row == limit_row, look));
                }
                Line::from(spans)
            })
            .collect()
    }

    /// The horizontal axis, with a mark at each quarter and `▲` under the
    /// picked column, if any.
    fn axis(&self, picked: Option<usize>) -> Line<'static> {
        let digits = self.top.to_string().len();
        let width = self.columns.len();
        let quarter = width / 4;
        let line: String = (0..width)
            .map(|j| {
                if Some(j) == picked {
                    '▲'
                } else if quarter >= 2 && (j + 1) % quarter == 0 {
                    '┬'
                } else {
                    '─'
                }
            })
            .collect();
        Line::from(format!(" {:>digits$} └{line}", 0))
    }

    /// The ticks under the axis's marks: the first count's at the corner,
    /// then each mark's, to the nearest hundred, leaving out any that
    /// would run into the one before.
    fn ticks(&self) -> Line<'static> {
        let gutter = self.gutter();
        let width = self.columns.len();
        let quarter = width / 4;
        let mut text = String::new();
        let mut put = |at: usize, label: String| {
            let from = (at + 1).saturating_sub(label.chars().count());
            let used = text.chars().count();
            if used == 0 || from > used {
                text.push_str(&" ".repeat(from - used));
                text.push_str(&label);
            }
        };
        put(gutter - 1, group_thousands(self.start));
        if quarter >= 2 {
            for j in (quarter - 1..width).step_by(quarter) {
                let tick = self.boundary(j + 1);
                put(gutter + j, group_thousands((tick + 50) / 100 * 100));
            }
        }
        Line::from(text)
    }

    /// The legend: each stage's swatch and name, then the limit's line.
    fn legend(&self, look: &PopulationLook) -> Line<'static> {
        let mut spans = vec![Span::raw(" ")];
        for stage in STACK {
            spans.push(Span::styled(
                look.full.to_string(),
                Style::default().fg(look.colour(stage)),
            ));
            spans.push(Span::raw(format!(" {}  ", stage_name(stage, 0))));
        }
        spans.push(Span::styled(
            look.limit.symbol.to_string().repeat(3),
            Style::default().fg(look.limit.fg),
        ));
        spans.push(Span::raw(format!(" most sprites, {}", self.limit)));
        Line::from(spans)
    }

    /// The two lines that read column `j`, or the last the run has
    /// reached: the count it shows, and what happened since the column
    /// before's.
    pub(crate) fn reading(&self, j: Option<usize>, data: &DataPack) -> [String; 2] {
        let last = self.reached().saturating_sub(1);
        let j = j.unwrap_or(last).min(last);
        let Some(shown) = self.columns[j] else {
            return [String::new(), String::new()];
        };
        let count = &self.points[shown];
        let when = if shown + 1 == self.points.len() {
            format!("now, tick {}", group_thousands(count.tick))
        } else {
            format!("tick {}", group_thousands(count.tick))
        };
        let sprites = count.sprites();
        let mut stages = vec![format!(
            "{sprites} sprite{}",
            if sprites == 1 { "" } else { "s" }
        )];
        stages.extend(STACK.map(|stage| {
            let n = count.at(stage);
            format!("{n} {}", stage_name(stage, n))
        }));
        let first = format!(" {when}: {}", stages.join(" · "));
        // The first column reads on from the first count.
        let before = match j {
            0 => Some(0),
            _ => self.columns[j - 1],
        };
        let second = match before {
            Some(before) if before < shown => {
                let mut changes = Changes::default();
                for point in &self.points[before + 1..=shown] {
                    changes.add(&point.since);
                }
                let since = group_thousands(self.points[before].tick);
                format!(" since tick {since}: {}", changed(&changes, data))
            }
            _ if shown == 0 => " the first count".to_string(),
            _ => String::new(),
        };
        [first, second]
    }
}

/// The width of the labels left of a graph reaching `top`, with the axis.
fn gutter(top: u32) -> usize {
    top.to_string().len() + 3
}

/// The stage drawn in half tile `half`, from 0 at the bottom, of a column
/// showing `count`, each half tile `step` sprites high: the stage at its
/// middle, or at the top of the stack if that's lower. A half tile the
/// stack doesn't reach is empty.
fn stage_at(count: &PopulationCount, half: usize, step: f64) -> Option<LifeStage> {
    let total = f64::from(count.sprites());
    let bottom = half as f64 * step;
    if total <= bottom {
        return None;
    }
    let at = (bottom + step / 2.0).min(total - 0.5);
    let mut below = 0.0;
    STACK.into_iter().find(|&stage| {
        below += f64::from(count.at(stage));
        at < below
    })
}

/// One tile of the graph: its lower half's stage and its upper half's, in
/// `look`; an empty tile on the limit's row is a dot of its line.
fn cell(
    lower: Option<LifeStage>,
    upper: Option<LifeStage>,
    on_limit: bool,
    look: &PopulationLook,
) -> Span<'static> {
    let colour = |stage| look.colour(stage);
    match (lower, upper) {
        (Some(lower), Some(upper)) if lower == upper => {
            Span::styled(look.full.to_string(), Style::default().fg(colour(lower)))
        }
        (Some(lower), Some(upper)) => Span::styled(
            look.half.to_string(),
            Style::default().fg(colour(lower)).bg(colour(upper)),
        ),
        (Some(lower), None) => {
            Span::styled(look.half.to_string(), Style::default().fg(colour(lower)))
        }
        _ if on_limit => Span::styled(
            look.limit.symbol.to_string(),
            Style::default().fg(look.limit.fg),
        ),
        _ => Span::raw(" "),
    }
}

/// A stage's name for `n` sprites at it: `young`, `adults`, `old`, `eggs`,
/// or for 1, `adult` and `egg`.
fn stage_name(stage: LifeStage, n: u32) -> &'static str {
    match (stage, n == 1) {
        (LifeStage::Young, _) => "young",
        (LifeStage::Adult, true) => "adult",
        (LifeStage::Adult, false) => "adults",
        (LifeStage::Old, _) => "old",
        (LifeStage::Egg, true) => "egg",
        (LifeStage::Egg, false) => "eggs",
    }
}

/// What `changes` did, in words: `4 hatched · 1 placed · 3 died (old age
/// 2 · dehydration 1)`, leaving out what didn't happen.
fn changed(changes: &Changes, data: &DataPack) -> String {
    let mut parts = Vec::new();
    if changes.hatched > 0 {
        parts.push(format!("{} hatched", changes.hatched));
    }
    if changes.placed > 0 {
        parts.push(format!("{} placed", changes.placed));
    }
    if changes.deaths() > 0 {
        let causes: Vec<String> = changes
            .died
            .iter()
            .map(|(&cause, n)| format!("{} {n}", cause_name(cause, data)))
            .collect();
        parts.push(format!(
            "{} died ({})",
            changes.deaths(),
            causes.join(" · ")
        ));
    }
    if parts.is_empty() {
        "none hatched or died".to_string()
    } else {
        parts.join(" · ")
    }
}

/// The full graph's rows above its axis, `rows` rows inside its border.
pub(crate) fn rows_above_axis(rows: usize) -> usize {
    rows.saturating_sub(UNDER).max(1)
}

/// The full graph's lines, `width` columns and `rows` rows inside its
/// border, with the picked column marked and read.
pub(crate) fn lines(app: &App, world: &World, width: usize, rows: usize) -> Vec<Line<'static>> {
    let look = app.theme.population();
    let plot = Plot::new(world, width);
    let mut lines = plot.rows(rows_above_axis(rows), &look);
    lines.push(plot.axis(app.graph_pick()));
    lines.push(plot.ticks());
    lines.push(Line::default());
    lines.push(plot.legend(&look));
    lines.extend(plot.reading(app.graph_pick(), world.data()).map(Line::from));
    lines
}

/// The small graph at the top of the World tab, `width` columns wide: a
/// heading, the graph and its axis.
pub(crate) fn small(app: &App, world: &World, width: usize) -> Vec<Line<'static>> {
    let look = app.theme.population();
    let plot = Plot::new(world, width);
    let from = group_thousands(plot.points[0].tick);
    let to = group_thousands(world.tick());
    let mut lines = vec![Line::from(format!(" population, ticks {from} to {to}"))];
    lines.extend(plot.rows(SMALL_ROWS, &look));
    lines.push(plot.axis(None));
    lines
}
