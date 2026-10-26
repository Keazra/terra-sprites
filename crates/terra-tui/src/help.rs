//! The help screen (`?`, design §6.1): every key, the colour legend and
//! where the game keeps its files.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::World;

use crate::app::App;
use crate::text::display_name;
use crate::theme::{Emote, SemanticTile, TextLook};

/// A group of keys: its heading, and each key with what it does.
type Group = (&'static str, &'static [(&'static str, &'static str)]);

/// The keys, grouped, a column of groups at a time.
const COLUMNS: [&[Group]; 3] = [
    &[
        (
            "TIME",
            &[
                ("space", "pause / resume"),
                (".", "step"),
                ("+ -", "faster / slower"),
            ],
        ),
        (
            "VIEW",
            &[
                ("WASD", "scroll (Shift: 5)"),
                ("T", "track the selected"),
                ("b", "sprite colours"),
                ("m", "event filter"),
                ("v", "exact detail"),
                ("?", "this help"),
                ("Esc", "let go, back, quit?"),
                ("Ctrl+C", "quit at once"),
            ],
        ),
        (
            "FILES",
            &[
                ("F5 F9", "quicksave / load"),
                ("Ctrl+S", "save as..."),
                ("Ctrl+O", "load a save"),
                ("Ctrl+T", "pick a theme"),
                ("Ctrl+R", "take over a replay"),
            ],
        ),
    ],
    &[(
        "CURSOR",
        &[
            ("Z X C", "select/train/grab"),
            ("Q E", "left / right click"),
            ("Shift+Q E", "hug / shock"),
            ("Ctrl+click", "hug / shock"),
            ("H", "show to sprites"),
            ("F", "follow a sprite"),
            ("middle", "follow a sprite"),
            ("wheel", "change mode"),
            ("C again", "the Place menu"),
            ("hold E", "aim, let go to send"),
        ],
    )],
    &[
        (
            "SPRITES",
            &[
                ("Tab", "next sprite"),
                ("Shift+Tab", "previous sprite"),
                ("l", "sprite list"),
                ("l then /", "filter the list"),
                ("p", "population graph"),
                ("k", "family tree"),
                ("r", "name it"),
                ("g", "save its genome"),
            ],
        ),
        (
            "INSPECTOR",
            &[("[ ]", "switch tabs"), ("PgUp PgDn", "scroll a tab")],
        ),
        // The debug overlay (design §6.1, #311).
        ("DEBUG", &[("F3", "how it's running")]),
    ],
];

/// How wide a key is set, before what it does.
const KEY_WIDTH: usize = 11;

/// The help screen's lines, for a screen `width` columns wide inside its
/// border: the headings in the theme's heading colour and each key in its
/// key style (design §6.2).
pub(crate) fn lines(app: &App, world: &World, width: usize) -> Vec<Line<'static>> {
    let look = app.theme.text();
    let column_width = width / COLUMNS.len();
    let columns: Vec<Vec<Vec<Span<'static>>>> = COLUMNS
        .iter()
        .map(|groups| {
            groups
                .iter()
                .flat_map(|(heading, keys)| {
                    let keys = keys.iter().map(move |(key, what)| {
                        let pad = KEY_WIDTH.saturating_sub(key.chars().count());
                        vec![
                            Span::raw(" "),
                            Span::styled(*key, look.key),
                            Span::raw(format!("{}{what}", " ".repeat(pad))),
                        ]
                    });
                    std::iter::once(headed(heading, "", look)).chain(keys)
                })
                .collect()
        })
        .collect();
    let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
    let mut lines: Vec<Line<'static>> = (0..rows)
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
        .collect();
    lines.push(Line::default());
    lines.push(Line::from(headed(
        "COLOURS",
        ": a sprite's strongest drive, once it's above half",
        look,
    )));
    lines.extend(colour_legend(app, world, width));
    lines.extend(emote_legend(app, width));
    lines.push(lineage_legend(app));
    let folder = app
        .data_folder()
        .map_or_else(|| "none".to_string(), |folder| folder.display().to_string());
    lines.push(Line::from(headed("FILES", &format!("    {folder}"), look)));
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
