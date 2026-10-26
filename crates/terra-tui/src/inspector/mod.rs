//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.
//!
//! Each tab's lines come from a module of its own, as do actions in words
//! and the event log's lines. The helpers they share are here, but for
//! the names of what one module tells of, which stay in it for the rest
//! to use: verbs in `actions`, traits and parents in `body`, and concepts
//! and what was learned in `brain`.

use ratatui::style::Style;
use ratatui::text::Line;
use terra_sim::{DeathCause, EntityId, Pos, SpriteView, World};

use crate::app::{App, Selection, Tab};
use crate::policy::{Panel, Subject};
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
pub(crate) use genome::something_new;
use world::world_tab;

/// The inspector's width, in columns, border included (design §6.1).
pub(crate) const INSPECTOR_WIDTH: u16 = 46;

/// The columns inside the inspector's border.
const WIDTH: usize = INSPECTOR_WIDTH as usize - 2;

/// What a sprite tab says with no sprite selected.
pub(crate) const NOTHING_SELECTED: &str = " No sprite selected: click one, or press Tab";

/// The inspector's title: the selected sprite, if any, then the tabs, with
/// the open one in brackets.
pub fn title(app: &App) -> String {
    let tabs: Vec<String> = Tab::ALL
        .iter()
        .map(|&tab| {
            if tab == app.tab() {
                format!("[{}]", tab.label())
            } else {
                tab.label().to_string()
            }
        })
        .collect();
    let tabs = tabs.join(" ");
    // The open tab's policy decides whether the title names the sprite
    // (design §6.4).
    let labels = app
        .selection()
        .map(|selection| selection.id())
        .filter(|&id| app.can_view(Panel::Tab(app.tab()), Subject::Sprite(id)))
        .map(|id| (app.names().label(id), format!("#{}", id.0)));
    fitted_title(
        labels
            .as_ref()
            .map(|(label, short)| (label.as_str(), short.as_str())),
        &tabs,
    )
}

/// The columns the border leaves the title: all but its corners and the
/// line before the title.
const TITLE_ROOM: usize = INSPECTOR_WIDTH as usize - 3;

/// The title with the sprite's `label` if it fits, or else its `short`
/// label, or else no label: the tabs are never cut (design §6.1).
fn fitted_title(labels: Option<(&str, &str)>, tabs: &str) -> String {
    let (label, short) = labels.unzip();
    [label, short]
        .into_iter()
        .flatten()
        .map(|label| format!(" {label} ── {tabs} "))
        .find(|title| title.chars().count() <= TITLE_ROOM)
        .unwrap_or_else(|| format!(" {tabs} "))
}

/// The open tab's lines, from the top.
pub fn lines(app: &App, world: &World) -> Vec<Line<'static>> {
    shown(app, world)
        .into_iter()
        .map(|(line, _)| line)
        .collect()
}

/// The open tab's lines, and which of them open or shut a memory block.
/// The policy is applied here, so drawing and hit-testing stay one list
/// (design §6.4, v40 §6.1).
fn shown(app: &App, world: &World) -> Vec<(Line<'static>, Option<MemoryOpen>)> {
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
            (None, Some(egg)) => bare(egg_card(&egg, world, &app.words(world))),
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
            bare(wrapped(&text, 1, Style::default()))
        }
    }
}

/// Lines that open nothing.
fn bare(lines: Vec<Line<'static>>) -> Vec<(Line<'static>, Option<MemoryOpen>)> {
    lines.into_iter().map(|line| (line, None)).collect()
}

/// The first line shown of a tab `length` lines long in `rows` rows, when
/// it's scrolled `scroll` lines: no further than where its last line comes
/// into view, since the tab may have got shorter, or the rows more.
pub(crate) fn first_shown(scroll: usize, length: usize, rows: usize) -> usize {
    scroll.min(length.saturating_sub(rows))
}

/// A sprite tab's lines for `sprite`, the selection. Only the Brain tab's
/// memory headings open anything.
fn sprite_tab(
    tab: Tab,
    sprite: &SpriteView,
    app: &App,
    world: &World,
) -> Vec<(Line<'static>, Option<MemoryOpen>)> {
    match tab {
        Tab::Body => bare(body_tab(sprite, app, world)),
        Tab::Brain => brain_rows(sprite, app, world)
            .into_iter()
            .map(|(text, open)| (Line::from(text), open))
            .collect(),
        Tab::Chem => bare(chem_tab(sprite)),
        Tab::Genome => bare(genome_tab(sprite, app)),
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
/// indented `indent` columns and the rest 3, all in `style`.
fn wrapped(text: &str, indent: usize, style: Style) -> Vec<Line<'static>> {
    wrap(&" ".repeat(indent), "   ", text, WIDTH)
        .into_iter()
        .map(|line| Line::styled(line.replace(BOUND, " "), style))
        .collect()
}
