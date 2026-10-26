//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.
//!
//! Each tab's lines come from a module of its own, as do actions in words
//! and the event log's lines. The helpers they share are here, but for
//! the names of what one module tells of, which stay in it for the rest
//! to use: verbs in `actions`, traits and parents in `body`, and concepts
//! and what was learned in `brain`.

use std::ops::Range;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::{DeathCause, EntityId, Pos, SpriteView, World};

use crate::app::{App, Selection, Tab};
use crate::policy::{Panel, Subject};
use crate::spots::Spot;
use crate::text::{BOUND, cause_name, thousands, unbroken, wrap};

mod actions;
mod body;
mod brain;
mod chem;
mod event_log;
mod genome;
#[cfg(test)]
mod tests;
mod world;

pub(crate) use actions::{done_to_line, observed_line};
pub(crate) use body::{Hatching, doing_line, hatching};
use body::{body_tab, egg_card};
use brain::brain_rows;
pub(crate) use brain::{MemoryOpen, lesson_at};
use chem::chem_tab;
pub(crate) use event_log::{crashed_into, event_line, event_sprites, refusal_sprites};
use genome::genome_tab;
pub(crate) use genome::{gene_about, something_new};
use world::world_tab;

/// The inspector's width, in columns, the column down its left included
/// (design §6.1).
pub(crate) const INSPECTOR_WIDTH: u16 = 46;

/// The columns a tab's lines have: all but the column down its left.
const WIDTH: usize = INSPECTOR_WIDTH as usize - 1;

/// Where the open tab's lines are drawn in the inspector, `area`: below its
/// title row, and right of the column down its left, which is a line in a
/// theme without shades (design §6.1).
pub(crate) fn tab_area(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(1),
        area.height.saturating_sub(1),
    )
}

/// What a sprite tab says with no sprite selected.
pub(crate) const NOTHING_SELECTED: &str = " No sprite selected: click one, or press Tab";

/// The inspector's title row: the selected sprite, if any, at its left, and
/// the tabs at its right, the open one in the theme's style for what's
/// chosen, reverse video in the built-in themes. The tabs keep their places
/// whatever is selected, so a click finds them where they were (design §6.1).
pub fn title(app: &App) -> Line<'static> {
    // The open tab's policy decides whether the title names the sprite
    // (design §6.4).
    let sprite = app
        .selection()
        .map(|selection| selection.id())
        .filter(|&id| app.can_view(Panel::Tab(app.tab()), Subject::Sprite(id)))
        .map(|id| (app.names().label(id), app.names().get(id)));
    let label = fitted_label(
        sprite
            .as_ref()
            .map(|(label, name)| (label.as_str(), name.unwrap_or(label))),
        TITLE_ROOM - tabs_width(),
    );
    let mut spans = vec![Span::raw(label)];
    for (i, (tab, text)) in tab_texts(app.tab()).enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        if tab == app.tab() {
            spans.push(Span::styled(text, app.theme.text().selected));
        } else {
            spans.push(Span::raw(text));
        }
    }
    Line::from(spans)
}

/// The tabs as the title row writes them with `open` open, which is as
/// wide as brackets made it, so the row doesn't move.
fn tab_texts(open: Tab) -> impl Iterator<Item = (Tab, String)> {
    Tab::ALL.iter().map(move |&tab| {
        let text = if tab == open {
            format!(" {} ", tab.label())
        } else {
            tab.label().to_string()
        };
        (tab, text)
    })
}

/// How many columns the tabs take in the title row, the open one's
/// spaces included.
fn tabs_width() -> usize {
    let labels: usize = Tab::ALL.iter().map(|tab| tab.label().chars().count()).sum();
    labels + Tab::ALL.len() - 1 + 2
}

/// Where each tab is written in the title row with `open` open: the
/// columns it takes, counting from the row's first column, right of the
/// line down the inspector's left (design §6.1). A click there opens it.
pub(crate) fn tab_places(open: Tab) -> Vec<(Tab, Range<usize>)> {
    let mut start = TITLE_ROOM - tabs_width();
    tab_texts(open)
        .map(|(tab, text)| {
            let end = start + text.chars().count();
            let place = (tab, start..end);
            start = end + 1;
            place
        })
        .collect()
}

/// The columns the title row has: all but the column down the inspector's
/// left.
const TITLE_ROOM: usize = INSPECTOR_WIDTH as usize - 1;

/// What the title row says before the tabs, `room` columns of it: a
/// sprite's `label`, "Mira #12", if it fits, or else its `name` alone, or
/// else as much of its name as fits, ending "..." since CP437 has no "…"
/// (design §6.1, §6.2). A space comes before it, at least one after it, and
/// the tabs are never cut. An unnamed sprite's name is its label.
fn fitted_label(sprite: Option<(&str, &str)>, room: usize) -> String {
    let fits = room.saturating_sub(2);
    let text = match sprite {
        Some((label, name)) => [label, name]
            .into_iter()
            .find(|text| text.chars().count() <= fits)
            .map_or_else(|| shortened(name, fits), str::to_string),
        None => String::new(),
    };
    format!("{:<room$}", format!(" {text}"))
}

/// The first of `text` that fits in `width` columns with "..." after it.
fn shortened(text: &str, width: usize) -> String {
    let kept: String = text.chars().take(width.saturating_sub(3)).collect();
    format!("{}...", kept.trim_end())
}

/// A line of a tab as it's drawn: the memory block it opens or shuts, if it
/// does (design v40 §6.1), and its spots, each with the columns it takes
/// from the line's start (#304).
pub(crate) struct TabLine {
    pub(crate) line: Line<'static>,
    pub(crate) opens: Option<MemoryOpen>,
    pub(crate) spots: Vec<(Range<u16>, Spot)>,
}

impl From<Line<'static>> for TabLine {
    /// A line that opens nothing and has no spot.
    fn from(line: Line<'static>) -> TabLine {
        TabLine {
            line,
            opens: None,
            spots: Vec::new(),
        }
    }
}

impl TabLine {
    /// `line`, all of it `spot`.
    fn spotted(line: Line<'static>, spot: Spot) -> TabLine {
        TabLine {
            line,
            opens: None,
            spots: vec![(0..WIDTH as u16, spot)],
        }
    }
}

/// The open tab's lines, from the top.
pub fn lines(app: &App, world: &World) -> Vec<Line<'static>> {
    shown(app, world)
        .into_iter()
        .map(|shown| shown.line)
        .collect()
}

/// The open tab's lines, which of them open or shut a memory block, and
/// their spots. The policy is applied here, so drawing, the spots and what
/// a click opens stay one list (design §6.4, v40 §6.1, #304).
pub(crate) fn shown(app: &App, world: &World) -> Vec<TabLine> {
    // The policy may hide a tab (design §6.4).
    let subject = match (app.tab(), app.selection()) {
        (Tab::World, _) | (_, None) => Subject::World,
        (_, Some(selection)) => Subject::Sprite(selection.id()),
    };
    // Saying how to select a sprite tells nothing about the world.
    let hint = app.tab() != Tab::World && app.selection().is_none();
    if !hint && !app.can_view(Panel::Tab(app.tab()), subject) {
        return Vec::new();
    }
    match (app.tab(), app.selection()) {
        (Tab::World, _) => bare(world_tab(app, world)),
        (_, None) => bare(vec![Line::from(NOTHING_SELECTED)]),
        (tab, Some(Selection::Living(id))) => match (world.sprite(id), world.egg(id)) {
            (Some(sprite), _) => sprite_tab(tab, &sprite, app, world),
            // An egg has no body, brain or chemistry yet (M2 design §10.7).
            (None, Some(egg)) => bare(egg_card(
                &egg,
                world,
                &app.words(world),
                app.theme.text().muted,
            )),
            (None, None) => Vec::new(),
        },
        (_, Some(Selection::Dead { id, cause, age })) => {
            let how = match cause {
                DeathCause::HurtBy(_) => cause_name(cause, world.data()),
                _ => format!("of {}", cause_name(cause, world.data())),
            };
            let text = format!(
                "{} died {} at age{BOUND}{}",
                unbroken(&app.names().label(id)),
                unbroken(&how),
                thousands(age)
            );
            bare(wrapped(&text, 1, Style::default(), app.theme.text().muted))
        }
    }
}

/// Lines that open nothing and have no spots.
fn bare(lines: Vec<Line<'static>>) -> Vec<TabLine> {
    lines.into_iter().map(TabLine::from).collect()
}

/// The first line shown of a tab `length` lines long in `rows` rows, when
/// it's scrolled `scroll` lines: no further than where its last line comes
/// into view, since the tab may have got shorter, or the rows more.
pub(crate) fn first_shown(scroll: usize, length: usize, rows: usize) -> usize {
    scroll.min(length.saturating_sub(rows))
}

/// A sprite tab's lines for `sprite`, the selection. Only the Brain tab's
/// memory headings open anything, and only the Body, Chem and Genome tabs
/// have spots (#304).
fn sprite_tab(tab: Tab, sprite: &SpriteView, app: &App, world: &World) -> Vec<TabLine> {
    match tab {
        Tab::Body => body_tab(sprite, app, world),
        Tab::Brain => brain_rows(sprite, app, world)
            .into_iter()
            .map(|(line, opens)| TabLine {
                opens,
                ..TabLine::from(line)
            })
            .collect(),
        Tab::Chem => chem_tab(sprite, app.theme.text()),
        Tab::Genome => genome_tab(sprite, app),
        Tab::World => Vec::new(),
    }
}

/// The type of object `id` on tile `at`, where the eye found it: a held
/// object is never attended, and a tile holds one object.
fn object_type(id: EntityId, at: Pos, world: &World) -> &str {
    world
        .object_at(at)
        .filter(|object| object.id() == id)
        .map_or("?", |object| object.type_name())
}

/// `name` after "a", or "an" before a vowel, whatever its case: "a ball",
/// "an Apple".
pub(crate) fn with_article(name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "aeiou".contains(c.to_ascii_lowercase()));
    format!("{} {name}", if vowel { "an" } else { "a" })
}

/// `n` of `thing`, pluralised: `1 tile`, `5 tiles`.
fn counted(n: u32, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

/// How the screen names the Cursor, which sprites learn about while they
/// can see it (design v29 §6.1): "the Cursor is frightening".
const CURSOR: &str = "the Cursor";

/// `items` as one line, `a · b · c`, which wraps only after a dot, never
/// inside an item.
fn listed(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|item| unbroken(item)).collect();
    items.join(&format!("{BOUND}· "))
}

/// `text` wrapped at word boundaries to fit the inspector, its first line
/// indented `indent` columns and the rest 3, all in `style` but for each
/// `·` between its parts, which is `muted` (design §6.2).
fn wrapped(text: &str, indent: usize, style: Style, muted: Style) -> Vec<Line<'static>> {
    wrap(&" ".repeat(indent), "   ", text, WIDTH)
        .into_iter()
        .map(|line| dotted(&line.replace(BOUND, " "), style, muted))
        .collect()
}

/// `text` as a line in `style`, but for each `·` between its parts, which
/// is `muted`: minor beside the parts (design §6.2).
pub(crate) fn dotted(text: &str, style: Style, muted: Style) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, part) in text.split('·').enumerate() {
        if i > 0 {
            spans.push(Span::styled("·", muted));
        }
        if !part.is_empty() {
            spans.push(Span::styled(part.to_string(), style));
        }
    }
    Line::from(spans)
}
