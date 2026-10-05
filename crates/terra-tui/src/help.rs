//! The help screen (`?`, design §6.1): every key, the colour legend and
//! where the game keeps its files.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::World;

use crate::app::App;
use crate::text::display_name;
use crate::theme::{Emote, SemanticTile};

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
                ("r", "name it"),
                ("g", "save its genome"),
            ],
        ),
        (
            "INSPECTOR",
            &[("[ ]", "switch tabs"), ("PgUp PgDn", "scroll a tab")],
        ),
    ],
];

/// How wide a key is set, before what it does.
const KEY_WIDTH: usize = 11;

/// The help screen's lines, for a screen `width` columns wide inside its
/// border.
pub(crate) fn lines(app: &App, world: &World, width: usize) -> Vec<Line<'static>> {
    let column_width = width / COLUMNS.len();
    let columns: Vec<Vec<String>> = COLUMNS
        .iter()
        .map(|groups| {
            groups
                .iter()
                .flat_map(|(heading, keys)| {
                    let keys = keys
                        .iter()
                        .map(|(key, what)| format!(" {key:<KEY_WIDTH$}{what}"));
                    std::iter::once(format!(" {heading}")).chain(keys)
                })
                .collect()
        })
        .collect();
    let rows = columns.iter().map(Vec::len).max().unwrap_or(0);
    let mut lines: Vec<Line<'static>> = (0..rows)
        .map(|row| {
            let text: String = columns
                .iter()
                .map(|column| {
                    let cell = column.get(row).map_or("", String::as_str);
                    format!("{cell:<column_width$}")
                })
                .collect();
            Line::from(text.trim_end().to_string())
        })
        .collect();
    lines.push(Line::default());
    lines.push(Line::from(
        " COLOURS: a sprite's strongest drive, once it's above half",
    ));
    lines.push(colour_legend(app, world));
    lines.push(emote_legend(app));
    lines.push(Line::default());
    let folder = app
        .data_folder()
        .map_or_else(|| "none".to_string(), |folder| folder.display().to_string());
    lines.push(Line::from(format!(" FILES    {folder}")));
    lines
}

/// Each drive's colour on a sprite, then a sprite with none above half.
fn colour_legend(app: &App, world: &World) -> Line<'static> {
    let sprite = app.theme.glyph(SemanticTile::Sprite);
    let mut spans = vec![Span::raw(" ")];
    let drives = world
        .data()
        .drives()
        .map(|drive| (display_name(drive), app.theme.drive_colour(drive)))
        .chain([("none".to_string(), None)]);
    for (name, colour) in drives {
        let colour = colour.unwrap_or(sprite.fg);
        spans.push(Span::styled(
            sprite.symbol.to_string(),
            Style::default().fg(colour),
        ));
        spans.push(Span::raw(format!(" {name}  ")));
    }
    Line::from(spans)
}

/// Each emote, in its colour, and what it means.
fn emote_legend(app: &App) -> Line<'static> {
    let emotes = [
        (Emote::Hurt, "hurt"),
        (Emote::Shocked, "zapped"),
        (Emote::Failed, "gave up"),
        (Emote::Resting, "resting"),
        (Emote::Pleased, "pleased"),
    ];
    let mut spans = vec![Span::raw(" EMOTES   ")];
    for (emote, meaning) in emotes {
        let glyph = app.theme.glyph(SemanticTile::Emote(emote));
        spans.push(Span::styled(
            glyph.symbol.to_string(),
            Style::default().fg(glyph.fg),
        ));
        spans.push(Span::raw(format!(" {meaning}  ")));
    }
    Line::from(spans)
}
