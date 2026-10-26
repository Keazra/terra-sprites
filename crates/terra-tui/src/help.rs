//! The help screen (`?`, design §6.1): a page for each part of the game,
//! with its keys, from the control list, and its legends; and its search,
//! across every page (#303).

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::World;

use crate::app::{App, Tab, finds};
use crate::controls::{self, CONTROLS, Control, Heading};
use crate::listed::listed_enum;
use crate::sprite_list;
use crate::text::display_name;
use crate::theme::{Emote, SemanticTile, TextLook};

listed_enum! {
    /// A page of the help (#303): the keys and legends for one part of the
    /// game. `?` opens the one for where the player is.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HelpPage {
        World => "World",
        Inspector => "Inspector",
        SpriteList => "Sprite list",
        Graph => "Population graph",
        Tree => "Family tree",
    }

    /// Every page, in the order the help's title row gives them.
    pub const ALL;

    /// The page's name in the help's title row.
    pub fn label(self) -> &'static str;
}

/// What the help's top edge says before its pages' names.
pub(crate) const TITLE: &str = " Help ─";

/// Each page's name as the help's top edge writes it after `TITLE`, side
/// by side: ` World `.
pub(crate) fn page_names() -> impl Iterator<Item = (HelpPage, String)> {
    HelpPage::ALL
        .into_iter()
        .map(|page| (page, format!(" {} ", page.label())))
}

/// The page whose name the help's top edge writes at `column`, counting
/// from where the title starts (#303).
pub(crate) fn page_at(column: usize) -> Option<HelpPage> {
    let mut start = TITLE.chars().count();
    for (page, name) in page_names() {
        let end = start + name.chars().count();
        if (start..end).contains(&column) {
            return Some(page);
        }
        start = end;
    }
    None
}

impl HelpPage {
    /// The headings of the page's keys, a column of them at a time.
    fn columns(self) -> &'static [&'static [Heading]] {
        match self {
            HelpPage::World => &[
                &[Heading::Time, Heading::View],
                &[Heading::Cursor, Heading::Files],
                &[Heading::Sprites, Heading::Inspector, Heading::Debug],
            ],
            HelpPage::Inspector => &[&[Heading::Inspector], &[Heading::Pointer]],
            HelpPage::SpriteList => &[&[Heading::SpriteList]],
            HelpPage::Graph => &[&[Heading::Graph], &[Heading::Time]],
            HelpPage::Tree => &[&[Heading::Tree], &[Heading::Time]],
        }
    }
}

/// How wide a key is set, before what it does.
const KEY_WIDTH: usize = 11;

/// The help's lines, for a screen `width` columns wide inside its border:
/// the open page's, or what its search found (#303). The headings are in
/// the theme's heading colour and each key in its key style (design §6.2).
pub(crate) fn lines(app: &App, world: &World, width: usize) -> Vec<Line<'static>> {
    match app.help_search() {
        Some(typed) => found(app, world, width, typed),
        None => page(app, world, width, app.help_page()),
    }
}

/// `page`'s keys, in its columns, then its legends.
fn page(app: &App, world: &World, width: usize, page: HelpPage) -> Vec<Line<'static>> {
    let look = app.theme.text();
    let mut lines = in_columns(page.columns(), width, look);
    lines.push(Line::default());
    for legend in legends(page, app, world, width, Some(app.tab())) {
        lines.extend(legend.heading);
        lines.extend(legend.lines);
    }
    lines
}

/// The controls under `heading`, in the control list's order.
fn under(heading: Heading) -> impl Iterator<Item = &'static Control> {
    CONTROLS
        .iter()
        .filter(move |control| control.headings.contains(&heading))
}

/// A row of the help: `control`'s keys in the theme's key style, set to
/// `KEY_WIDTH`, then what it does.
fn row(control: &Control, look: TextLook) -> Vec<Span<'static>> {
    let pad = KEY_WIDTH.saturating_sub(control.written.chars().count());
    vec![
        Span::raw(" "),
        Span::styled(control.written, look.key),
        Span::raw(format!("{}{}", " ".repeat(pad), control.does)),
    ]
}

/// The keys under each heading of `columns`, a column of headings at a
/// time, side by side in a screen `width` columns wide.
fn in_columns(columns: &[&[Heading]], width: usize, look: TextLook) -> Vec<Line<'static>> {
    let column_width = width / columns.len().max(1);
    let columns: Vec<Vec<Vec<Span<'static>>>> = columns
        .iter()
        .map(|headings| {
            headings
                .iter()
                .flat_map(|&heading| {
                    let keys = under(heading).map(move |control| row(control, look));
                    std::iter::once(headed(heading.label(), "", look)).chain(keys)
                })
                .collect()
        })
        .collect();
    let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
    (0..rows)
        .map(|row| {
            // Each column starts where the one before it ends, and no sooner
            // than a column's width after it.
            let mut spans = Vec::new();
            let (mut used, mut start) = (0, 0);
            for column in &columns {
                let cell = column.get(row).cloned().unwrap_or_default();
                let cell_width: usize = cell.iter().map(|span| span.content.chars().count()).sum();
                if !cell.is_empty() {
                    spans.push(Span::raw(" ".repeat(start - used)));
                    spans.extend(cell);
                    used = start + cell_width;
                }
                start += cell_width.max(column_width);
            }
            Line::from(spans)
        })
        .collect()
}

/// A legend on a page of the help: its heading line, if it has one of its
/// own, and its lines.
struct Legend {
    heading: Option<Line<'static>>,
    lines: Vec<Line<'static>>,
}

impl Legend {
    /// A legend of plain `lines` under `heading`.
    fn of(heading: &str, lines: &[&str], look: TextLook) -> Legend {
        Legend {
            heading: Some(Line::from(headed(heading, "", look))),
            lines: lines
                .iter()
                .map(|line| Line::from(format!(" {line}")))
                .collect(),
        }
    }
}

/// `page`'s legends, for a screen `width` columns wide: on the inspector's,
/// the legend of `tab`, or of every tab with none.
fn legends(
    page: HelpPage,
    app: &App,
    world: &World,
    width: usize,
    tab: Option<Tab>,
) -> Vec<Legend> {
    let look = app.theme.text();
    match page {
        HelpPage::World => {
            let folder = app
                .data_folder()
                .map_or_else(|| "none".to_string(), |folder| folder.display().to_string());
            let alone = |line: Line<'static>| Legend {
                heading: None,
                lines: vec![line],
            };
            vec![
                Legend {
                    heading: Some(Line::from(headed(
                        "COLOURS",
                        ": a sprite's strongest drive, once it's above half",
                        look,
                    ))),
                    lines: colour_legend(app, world, width),
                },
                Legend {
                    heading: None,
                    lines: emote_legend(app, width),
                },
                alone(lineage_legend(app)),
                alone(Line::from(headed("FILES", &format!("    {folder}"), look))),
            ]
        }
        HelpPage::Inspector => {
            let tabs = tab.map_or(Tab::ALL.to_vec(), |tab| vec![tab]);
            tabs.into_iter()
                .map(|tab| {
                    let heading = format!("{} TAB", tab.label().to_uppercase());
                    Legend::of(&heading, tab_legend(tab), look)
                })
                .collect()
        }
        HelpPage::SpriteList => vec![Legend::of(
            "COLUMNS",
            &[
                "Drive: its strongest drive, once it's above half, in its colour",
                "Doing: what it's doing, as the Body tab says it",
                "A filter finds the starts of words in any column: old, gen 3, #12",
            ],
            look,
        )],
        HelpPage::Graph => vec![Legend::of(
            "THE GRAPH",
            &[
                "The population over the world's whole run, from the youngest up, eggs at the bottom",
                "Each column is a stretch of time, as shares of the count at its end",
                "Pick a moment with ←→ or the pointer to read its numbers",
            ],
            look,
        )],
        HelpPage::Tree => vec![Legend::of(
            "THE TREE",
            &[
                "The selected sprite or egg in the middle, its parents and grandparents above",
                "Its young below, grouped by their other parent, and its brothers and sisters at the foot",
                "The dead are grey; b's lineage colours mark the same sprites on the map",
            ],
            look,
        )],
    }
}

/// What each inspector tab shows, as the help tells it (#303).
fn tab_legend(tab: Tab) -> &'static [&'static str] {
    match tab {
        Tab::Body => &[
            "What it's doing, its age and life stage, and when it reaches the next",
            "A bar for each drive, in the drive's colour: ▲ rising, ▼ falling",
            "Its body's levels and its wear, then what you've seen it do",
            "An egg shows who laid it and when it hatches",
        ],
        Tab::Brain => &[
            "ATTENTION: what it can sense, scored; ► marks what it looks at",
            "DECISION: what it chose to do, and what added most to the choice",
            "Then what it learned, under the thing it's about: + good, - bad",
            "► on a heading means more is hidden: click it, or press enter on it",
            "PLACES: where it remembers things are, how far and which way",
        ],
        Tab::Chem => &[
            "Each chemical's level, from 0 to 1, and its change each tick",
            "felt: the good it took in last tick, its relief and reward, less the punishment",
            "HORMONES: its genes' own chemicals, and the ones with a name",
        ],
        Tab::Genome => &[
            "Its genes by kind: traits, half-lives, reactions, emitters, receptors",
            "A dimmed gene does nothing, with why on the line below it",
            "g saves the genome to a file, which the Place menu brings back as a sprite",
        ],
        Tab::World => &[
            "A small population graph: p opens the full one",
            "The data pack, the population, and how many died of what",
            "Each kind of object, how many there are, and how many in each stage",
        ],
    }
}

/// What the help's search found for `typed` (#303): on each page, the keys
/// and the legends' lines with each word typed starting a word of theirs,
/// under the page's name. A key on more than one page shows on the first.
fn found(app: &App, world: &World, width: usize, typed: &str) -> Vec<Line<'static>> {
    let look = app.theme.text();
    let typed_words = sprite_list::words(typed);
    let mut shown: Vec<&Control> = Vec::new();
    let mut lines = Vec::new();
    for page in HelpPage::ALL {
        let mut found = Vec::new();
        for &heading in page.columns().iter().flat_map(|column| column.iter()) {
            for control in under(heading) {
                let seen = shown.iter().any(|seen| std::ptr::eq(*seen, control));
                let text = controls::found_by(control, control.written, control.keys);
                if seen || !finds(&text, &typed_words) {
                    continue;
                }
                shown.push(control);
                found.push(Line::from(row(control, look)));
            }
        }
        for legend in legends(page, app, world, width, None) {
            let heading_found = legend
                .heading
                .as_ref()
                .is_some_and(|heading| finds(&heading.to_string(), &typed_words));
            let lines: Vec<Line<'static>> = legend
                .lines
                .into_iter()
                .filter(|line| heading_found || finds(&line.to_string(), &typed_words))
                .collect();
            if heading_found || !lines.is_empty() {
                found.extend(legend.heading);
                found.extend(lines);
            }
        }
        if !found.is_empty() {
            lines.push(Line::from(headed(&page.label().to_uppercase(), "", look)));
            lines.extend(found);
        }
    }
    if lines.is_empty() {
        lines.push(Line::styled(
            format!(" Nothing in the help has \"{typed}\""),
            look.muted,
        ));
    }
    lines
}

/// A line of the help that starts with a `heading`, in the theme's heading
/// colour, then `rest`.
fn headed(heading: &str, rest: &str, look: TextLook) -> Vec<Span<'static>> {
    let mut spans = vec![
        Span::raw(" "),
        Span::styled(heading.to_string(), look.heading),
    ];
    if !rest.is_empty() {
        spans.push(Span::raw(rest.to_string()));
    }
    spans
}

/// Each drive's colour on a sprite, then a sprite with none above half,
/// on as many lines `width` columns wide as they need.
fn colour_legend(app: &App, world: &World, width: usize) -> Vec<Line<'static>> {
    let sprite = app.theme.glyph(SemanticTile::Sprite);
    let drives = world
        .data()
        .drives()
        .map(|drive| (display_name(drive), app.theme.drive_colour(drive)))
        .chain([("none".to_string(), None)]);
    let mut lines = Vec::new();
    let mut spans = vec![Span::raw(" ")];
    let mut used = 1;
    for (name, colour) in drives {
        let entry = format!(" {name}  ");
        let needs = 1 + entry.trim_end().chars().count();
        if used > 1 && used + needs > width {
            lines.push(Line::from(std::mem::replace(
                &mut spans,
                vec![Span::raw(" ")],
            )));
            used = 1;
        }
        let colour = colour.unwrap_or(sprite.fg);
        spans.push(Span::styled(
            sprite.symbol.to_string(),
            Style::default().fg(colour),
        ));
        used += 1 + entry.chars().count();
        spans.push(Span::raw(entry));
    }
    lines.push(Line::from(spans));
    lines
}

/// The lineage colours (M2 design §17.4): who's drawn in each, of the
/// selected sprite's lineage.
fn lineage_legend(app: &App) -> Line<'static> {
    let sprite = app.theme.glyph(SemanticTile::Sprite).symbol.to_string();
    let lineage = app.theme.lineage();
    let marked = [
        (lineage.elders, "parents, grandparents"),
        (lineage.siblings, "brothers, sisters"),
        (lineage.young, "young, theirs"),
        (lineage.others, "the rest"),
    ];
    let mut spans = headed("LINEAGE", "  ", app.theme.text());
    for (colour, who) in marked {
        spans.push(Span::styled(sprite.clone(), Style::default().fg(colour)));
        spans.push(Span::raw(format!(" {who}  ")));
    }
    Line::from(spans)
}

/// Each emote, in its colour, and what it means.
fn emote_legend(app: &App, width: usize) -> Vec<Line<'static>> {
    let emotes = [
        (Emote::Hurt, "hurt"),
        (Emote::Shocked, "zapped"),
        (Emote::Failed, "gave up"),
        (Emote::Resting, "resting"),
        (Emote::Pleased, "pleased"),
        // Courtship (M2 design §15.8).
        (Emote::Dancing, "danced"),
        (Emote::Calling, "called"),
    ];
    const HEADING: &str = "EMOTES";
    // The heading, with a space before it and three after.
    const HEAD: usize = HEADING.len() + 4;
    let indent = || vec![Span::raw(" ".repeat(HEAD))];
    let mut lines = Vec::new();
    let mut spans = headed(HEADING, "   ", app.theme.text());
    let mut used = HEAD;
    for (emote, meaning) in emotes {
        let entry = format!(" {meaning}  ");
        let needs = 1 + entry.trim_end().chars().count();
        if used > HEAD && used + needs > width {
            lines.push(Line::from(std::mem::replace(&mut spans, indent())));
            used = HEAD;
        }
        let glyph = app.theme.glyph(SemanticTile::Emote(emote));
        spans.push(Span::styled(
            glyph.symbol.to_string(),
            Style::default().fg(glyph.fg),
        ));
        used += 1 + entry.chars().count();
        spans.push(Span::raw(entry));
    }
    lines.push(Line::from(spans));
    lines
}
