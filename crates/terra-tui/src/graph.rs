//! The population graph (M2 design §14): the world's population over its
//! whole run, stacked from the youngest up, eggs at the bottom, in the
//! theme's colours, full screen on `p` and small at the top of the World tab. Each
//! column is a stretch of time, showing the count at its end: in the full
//! graph, as shares, the column full height; in the small one, how many.
//! Until the run fills the graph, a column is 100 ticks and the graph grows
//! from the left.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::{DataPack, PopulationChanges, PopulationCount, World};

use crate::app::App;
use crate::listed::listed_enum;
use crate::text::{cause_name, generations_words, thousands};
use crate::theme::PopulationLook;

/// The ticks a column stands for while the run is too short to fill the
/// graph: as often as the world counts at first (M2 design §14).
const TICKS_PER_COLUMN: u64 = terra_sim::FIRST_COUNT_EVERY;

listed_enum! {
    /// What the graph stacks (M2 design §14.2): the eggs, which are items
    /// until they hatch (§10.4), then the sprites at each life stage.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Band {
        Eggs => "eggs",
        Young => "young",
        Adults => "adults",
        Old => "old",
    }

    /// The bands, stacked from the bottom, youngest first (M2 design
    /// §14.2).
    const ALL;

    /// The band's name, as the legend says it.
    fn plural(self) -> &'static str;
}

impl Band {
    /// How many of `count` are in the band.
    fn of(self, count: &PopulationCount) -> u32 {
        match self {
            Band::Eggs => count.eggs,
            Band::Young => count.young,
            Band::Adults => count.adults,
            Band::Old => count.old,
        }
    }

    /// The band's name for `n` of them.
    fn name(self, n: u32) -> &'static str {
        match (self, n) {
            (Band::Eggs, 1) => "egg",
            (Band::Adults, 1) => "adult",
            _ => self.plural(),
        }
    }
}

/// Rows the full graph keeps for what's under it: the axis, its ticks, a
/// blank line, the legend and the three lines that read a moment.
const UNDER: usize = 7;

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
    /// Whether each column shows its count as shares, full height, as the
    /// full graph does, rather than how many (M2 design §14.2).
    shares: bool,
}

impl Plot {
    /// The full graph: the population of `world` over its whole run as
    /// shares, `width` columns wide with the labels left of it.
    pub(crate) fn full(world: &World, width: usize) -> Plot {
        Plot::new(world, width, true)
    }

    /// The small graph: the population of `world` over its whole run, how
    /// many, `width` columns wide with the labels left of it.
    fn small(world: &World, width: usize) -> Plot {
        Plot::new(world, width, false)
    }

    /// The population of `world` over its whole run, `width` columns wide
    /// with the labels left of it, as shares if `shares`.
    fn new(world: &World, width: usize, shares: bool) -> Plot {
        let mut points = world.population().to_vec();
        let now = world.population_now();
        if points.last().is_none_or(|last| last.tick < now.tick) {
            points.push(now);
        }
        let limit = u32::from(world.most_sprites());
        let most = points
            .iter()
            .map(PopulationCount::with_eggs)
            .max()
            .unwrap_or(0);
        let top = limit.max(most).max(1);
        let start = points[0].tick;
        let end = points.last().map_or(start, |last| last.tick);
        let width = width.saturating_sub(label_width(top, shares) + 3).max(1);
        let mut plot = Plot {
            columns: vec![None; width],
            start,
            span: (end - start).max(width as u64 * TICKS_PER_COLUMN),
            top,
            limit,
            shares,
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

    /// The column that reads `moment`, a picked tick: the first the run
    /// has reached whose stretch takes it in, or with none picked, the
    /// last, which reads now.
    pub(crate) fn column_of(&self, moment: Option<u64>) -> usize {
        let last = self.reached().saturating_sub(1);
        moment
            .and_then(|tick| (0..last).find(|&j| self.boundary(j + 1) >= tick))
            .unwrap_or(last)
    }

    /// The moment to pick for column `j`: the end of its stretch of time,
    /// or none for the last, so a pick there goes on reading now.
    pub(crate) fn moment(&self, j: usize) -> Option<u64> {
        let last = self.reached().saturating_sub(1);
        (j < last).then(|| self.boundary(j + 1))
    }

    /// The width of the labels left of the graph, with the axis: where its
    /// first column is.
    pub(crate) fn gutter(&self) -> usize {
        label_width(self.top, self.shares) + 3
    }

    /// The graph's `rows` rows, each with its label, in `look`.
    fn rows(&self, rows: usize, look: &PopulationLook) -> Vec<Line<'static>> {
        let rows = rows.max(1);
        let halves = 2 * rows;
        let width = label_width(self.top, self.shares);
        let middle = rows / 2;
        // Each row of characters is two halves, each `step` sprites high.
        let step = f64::from(self.top) / halves as f64;
        // The row the limit falls in, when the graph shows how many.
        let limit_row = (!self.shares)
            .then(|| rows - ((f64::from(self.limit) / step / 2.0).ceil() as usize).clamp(1, rows));
        // Each column's band in each half, from the bottom.
        let stacks: Vec<Vec<Option<Band>>> = self
            .columns
            .iter()
            .map(|column| match column.map(|i| &self.points[i]) {
                None => vec![None; halves],
                Some(count) if self.shares => share_stack(count, halves),
                Some(count) => (0..halves).map(|half| band_at(count, half, step)).collect(),
            })
            .collect();
        // The label of the height `part` rows of `rows` up. With shares,
        // half way falls in the middle row, at its top or, with an odd
        // number of rows, its middle, so it says 50% either way.
        let height = |part: usize| {
            if self.shares && part < rows && part == rows.div_ceil(2) {
                "50%".to_string()
            } else if self.shares {
                format!("{}%", part * 100 / rows)
            } else {
                (part as u32 * self.top / rows as u32).to_string()
            }
        };
        (0..rows)
            .map(|row| {
                let label = if row == 0 {
                    height(rows)
                } else if row == middle && rows > 2 {
                    height(rows - row)
                } else {
                    String::new()
                };
                let axis = if label.is_empty() { '│' } else { '┤' };
                let mut spans = vec![Span::raw(format!(" {label:>width$} {axis}"))];
                let lower = 2 * (rows - 1 - row);
                for stack in &stacks {
                    let on_limit = limit_row == Some(row);
                    spans.push(block(stack[lower], stack[lower + 1], on_limit, look));
                }
                Line::from(spans)
            })
            .collect()
    }

    /// The horizontal axis, with a mark at each quarter and `▲` under the
    /// picked column, if any.
    fn axis(&self, picked: Option<usize>) -> Line<'static> {
        let width = label_width(self.top, self.shares);
        let zero = if self.shares { "0%" } else { "0" };
        let marks = self.marks();
        let line: String = (0..self.columns.len())
            .map(|j| {
                if Some(j) == picked {
                    '▲'
                } else if marks.contains(&j) {
                    '┬'
                } else {
                    '─'
                }
            })
            .collect();
        Line::from(format!(" {zero:>width$} └{line}"))
    }

    /// The ticks under the axis's marks: the first count's at the corner,
    /// then each mark's, to the nearest hundred, leaving out any that
    /// would run into the one before.
    fn ticks(&self) -> Line<'static> {
        let gutter = self.gutter();
        let mut text = String::new();
        let mut put = |at: usize, label: String| {
            let from = (at + 1).saturating_sub(label.chars().count());
            let used = text.chars().count();
            if used == 0 || from > used {
                text.push_str(&" ".repeat(from - used));
                text.push_str(&label);
            }
        };
        put(gutter - 1, thousands(self.start));
        for j in self.marks() {
            let tick = self.boundary(j + 1);
            put(gutter + j, thousands((tick + 50) / 100 * 100));
        }
        Line::from(text)
    }

    /// The columns marked on the axis: the last of each quarter, if a
    /// quarter is at least 2 columns.
    fn marks(&self) -> Vec<usize> {
        let width = self.columns.len();
        let quarter = width / 4;
        if quarter < 2 {
            return Vec::new();
        }
        (quarter - 1..width).step_by(quarter).collect()
    }

    /// The legend: each band's swatch and name, then the limit's line, or
    /// with shares, that they're shares, in `muted` (design §6.2).
    fn legend(&self, look: &PopulationLook, muted: Style) -> Line<'static> {
        let mut spans = vec![Span::raw(" ")];
        for band in Band::ALL {
            spans.push(Span::styled(
                look.full.to_string(),
                Style::default().fg(look.colour(band)),
            ));
            spans.push(Span::raw(format!(" {}  ", band.plural())));
        }
        if self.shares {
            spans.push(Span::styled("as shares of each column", muted));
            return Line::from(spans);
        }
        spans.push(Span::styled(
            look.limit.symbol.to_string().repeat(3),
            Style::default().fg(look.limit.fg),
        ));
        spans.push(Span::raw(format!(" most sprites, {}", self.limit)));
        Line::from(spans)
    }

    /// The three lines that read column `j`, or the last the run has
    /// reached (M2 design §14.3): when the count it shows was taken, with
    /// how many sprites and of which generations; the eggs and the sprites
    /// at each stage; and what happened since the column before's.
    pub(crate) fn reading(&self, j: usize, data: &DataPack) -> [String; 3] {
        let Some(shown) = self.columns.get(j).copied().flatten() else {
            return [String::new(), String::new(), String::new()];
        };
        let count = &self.points[shown];
        let when = if shown + 1 == self.points.len() {
            format!("now, tick {}", thousands(count.tick))
        } else {
            format!("tick {}", thousands(count.tick))
        };
        let sprites = count.sprites();
        let mut first = format!(
            " {when}: {sprites} sprite{}",
            if sprites == 1 { "" } else { "s" }
        );
        // A count from a save made before the record kept them has none
        // (#208).
        if let Some(alive) = count.generations {
            let (label, value) = generations_words(alive);
            first.push_str(&format!(" · {label} {value}"));
        }
        // With shares, each band's share of the column, eggs included.
        let column = count.with_eggs();
        let stages = Band::ALL.map(|band| {
            let n = band.of(count);
            if self.shares && column > 0 {
                let percent = (200 * n + column) / (2 * column);
                format!("{n} {} ({percent}%)", band.name(n))
            } else {
                format!("{n} {}", band.name(n))
            }
        });
        let second = format!(" {}", stages.join(" · "));
        // Since the count the nearest column before shows, if it's an
        // earlier one: columns narrower than the time between counts show
        // the same count again. The first column reads on from the first
        // count.
        let before = self.columns[..j]
            .iter()
            .rev()
            .flatten()
            .copied()
            .find(|&before| before < shown)
            .unwrap_or(0);
        let third = if before < shown {
            let mut changes = PopulationChanges::default();
            for point in &self.points[before + 1..=shown] {
                changes.add(&point.since);
            }
            let since = thousands(self.points[before].tick);
            format!(" since tick {since}: {}", changed(&changes, data))
        } else {
            " the first count".to_string()
        };
        [first, second, third]
    }
}

/// The width of the labels on the vertical axis of a graph reaching `top`,
/// or showing shares: `100%`.
fn label_width(top: u32, shares: bool) -> usize {
    if shares { 4 } else { top.to_string().len() }
}

/// The band in each of `halves` halves of a column showing `count` as
/// shares, from the bottom: each band's share of the halves, by largest
/// remainder, and at least one for each band with any in it, room allowing,
/// so a small one is never lost (M2 design §14.2).
fn share_stack(count: &PopulationCount, halves: usize) -> Vec<Option<Band>> {
    let n = Band::ALL.map(|band| i64::from(band.of(count)));
    let total: i64 = n.iter().sum();
    if total == 0 {
        return vec![None; halves];
    }
    let room = halves as i64;
    let present = n.iter().filter(|&&n| n > 0).count();
    let mut given = n.map(|n| n * room / total);
    if present <= halves {
        for (given, &n) in given.iter_mut().zip(&n) {
            if n > 0 {
                *given = (*given).max(1);
            }
        }
    }
    // How far each band is short of its share, in 1/total of a half.
    let short = |i: usize, given: &[i64; 4]| n[i] * room - given[i] * total;
    loop {
        let sum: i64 = given.iter().sum();
        if sum < room {
            // The band furthest short, the lowest if several are.
            let i = (0..4)
                .filter(|&i| n[i] > 0)
                .max_by_key(|&i| (short(i, &given), std::cmp::Reverse(i)))
                .expect("a band with sprites in it");
            given[i] += 1;
        } else if sum > room {
            // The band furthest over that can spare one, the lowest if
            // several are.
            let i = (0..4)
                .filter(|&i| given[i] > 1)
                .min_by_key(|&i| (short(i, &given), i))
                .expect("a band given more than one");
            given[i] -= 1;
        } else {
            break;
        }
    }
    Band::ALL
        .iter()
        .zip(given)
        .flat_map(|(&band, given)| std::iter::repeat_n(Some(band), given as usize))
        .collect()
}

/// The band drawn in half `half` of a column's characters, from 0 at the
/// bottom, showing `count`, each half `step` sprites high: the band at its
/// middle, or at the top of the stack if that's lower. A half the stack
/// doesn't reach is empty.
fn band_at(count: &PopulationCount, half: usize, step: f64) -> Option<Band> {
    let total = f64::from(count.with_eggs());
    let bottom = half as f64 * step;
    if total <= bottom {
        return None;
    }
    let at = (bottom + step / 2.0).min(total - 0.5);
    let mut below = 0.0;
    Band::ALL.into_iter().find(|&band| {
        below += f64::from(band.of(count));
        at < below
    })
}

/// One character of the graph: its lower half's band and its upper half's,
/// in `look`; an empty one on the limit's row is a dot of its line.
fn block(
    lower: Option<Band>,
    upper: Option<Band>,
    on_limit: bool,
    look: &PopulationLook,
) -> Span<'static> {
    let colour = |band| look.colour(band);
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

/// What `changes` did, in words: `2 laid · 4 hatched · 1 spawned · 3 died
/// (old age 2 · dehydration 1)`, leaving out what didn't happen.
fn changed(changes: &PopulationChanges, data: &DataPack) -> String {
    let mut parts = Vec::new();
    if changes.laid > 0 {
        parts.push(format!("{} laid", changes.laid));
    }
    if changes.hatched > 0 {
        parts.push(format!("{} hatched", changes.hatched));
    }
    if changes.spawned > 0 {
        parts.push(format!("{} spawned", changes.spawned));
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
        "none laid, hatched or died".to_string()
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
    let plot = Plot::full(world, width);
    let mut lines = plot.rows(rows_above_axis(rows), &look);
    let column = plot.column_of(app.graph_pick());
    lines.push(plot.axis(Some(column)));
    lines.push(plot.ticks());
    lines.push(Line::default());
    lines.push(plot.legend(&look, app.theme.text().muted));
    lines.extend(plot.reading(column, world.data()).map(Line::from));
    lines
}

/// The small graph at the top of the World tab, `width` columns wide: a
/// heading, in the theme's heading colour (design §6.2), the graph and its
/// axis.
pub(crate) fn small(app: &App, world: &World, width: usize) -> Vec<Line<'static>> {
    let look = app.theme.population();
    let plot = Plot::small(world, width);
    let heading = format!(" population, {}", covers(world));
    let mut lines = vec![Line::styled(heading, app.theme.text().heading)];
    lines.extend(plot.rows(SMALL_ROWS, &look));
    lines.push(plot.axis(None));
    lines
}

/// The ticks the graph covers, as its headings say: `ticks 0 to 48,000`.
pub(crate) fn covers(world: &World) -> String {
    let first = world
        .population()
        .first()
        .map_or(world.tick(), |count| count.tick);
    format!("ticks {} to {}", thousands(first), thousands(world.tick()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(eggs: u32, young: u32, adults: u32, old: u32) -> PopulationCount {
        PopulationCount {
            eggs,
            young,
            adults,
            old,
            ..PopulationCount::default()
        }
    }

    /// How many halves each band takes in `stack`.
    fn halves_of(stack: &[Option<Band>]) -> [usize; 4] {
        Band::ALL.map(|band| stack.iter().filter(|&&b| b == Some(band)).count())
    }

    #[test]
    fn shares_fill_the_column_in_proportion() {
        let stack = share_stack(&count(0, 11, 75, 14), 40);
        assert_eq!(stack.len(), 40);
        assert_eq!(halves_of(&stack), [0, 4, 30, 6]);
        assert_eq!(stack[0], Some(Band::Young), "from the bottom");
        assert_eq!(stack[39], Some(Band::Old));
    }

    #[test]
    fn a_band_too_small_for_a_half_still_gets_one() {
        // 1 in 100 is 0.4 of a half: rounded down, it would be lost.
        let stack = share_stack(&count(1, 1, 97, 1), 40);
        assert_eq!(halves_of(&stack), [1, 1, 37, 1]);
        assert_eq!(stack[0], Some(Band::Eggs), "the eggs at the bottom");
    }

    #[test]
    fn a_column_with_no_sprites_is_empty() {
        assert_eq!(share_stack(&count(0, 0, 0, 0), 4), vec![None; 4]);
    }
}
