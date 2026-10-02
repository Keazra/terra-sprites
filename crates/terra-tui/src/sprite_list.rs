//! The sprite list (`l`, design §6.1): every sprite the player may see, one
//! to a row, with its age, its strongest drive and what it's doing, sorted
//! as the player chooses.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use terra_sim::{ChemicalKind, ChemicalLevel, EntityId, SpriteView, World};

use crate::app::App;
use crate::inspector;
use crate::policy::{Panel, Subject};
use crate::text::{display_name, group_thousands};

/// How strong a drive must be to colour its sprite, and to show in the
/// list's Drive column (design §6.3).
pub(crate) const DRIVE_SHOWS: f32 = 0.5;

/// What the sprite list is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// By ID, lowest first.
    Number,
    /// Named sprites first, by name, then the rest by ID.
    Name,
    /// Oldest first.
    Age,
    /// By strongest drive, in the data pack's order, strongest first
    /// within each; sprites with none above half last.
    Drive,
}

impl SortBy {
    /// Every order, in the order `Tab` goes through them.
    pub const ALL: [SortBy; 4] = [SortBy::Number, SortBy::Name, SortBy::Age, SortBy::Drive];

    /// What the list's title calls it.
    pub fn label(self) -> &'static str {
        match self {
            SortBy::Number => "number",
            SortBy::Name => "name",
            SortBy::Age => "age",
            SortBy::Drive => "drive",
        }
    }
}

/// `sprite`'s strongest drive, if one is above half (design §6.3). Of two
/// equally strong, the first in the data pack's order.
pub(crate) fn strongest_drive<'a>(sprite: &SpriteView<'a>) -> Option<ChemicalLevel<'a>> {
    sprite
        .chemicals()
        .filter(|chemical| chemical.kind == ChemicalKind::Drive && chemical.level > DRIVE_SHOWS)
        .fold(None::<ChemicalLevel>, |best, chemical| match best {
            Some(best) if best.level >= chemical.level => Some(best),
            _ => Some(chemical),
        })
}

/// The sprites the list shows, in the order it shows them: those the policy
/// lets it show (design §6.4), sorted by `sort`.
pub(crate) fn order(app: &App, world: &World, sort: SortBy) -> Vec<EntityId> {
    let mut sprites: Vec<SpriteView> = world
        .sprites()
        .filter(|sprite| app.can_view(Panel::SpriteList, Subject::Sprite(sprite.id())))
        .collect();
    let drives: Vec<&str> = world.data().drives().collect();
    match sort {
        // The world lists its sprites by ID already.
        SortBy::Number => {}
        SortBy::Name => sprites.sort_by_key(|sprite| match app.names().get(sprite.id()) {
            Some(name) => (0, name.to_lowercase(), sprite.id()),
            None => (1, String::new(), sprite.id()),
        }),
        SortBy::Age => sprites.sort_by_key(|sprite| (std::cmp::Reverse(sprite.age()), sprite.id())),
        SortBy::Drive => sprites.sort_by(|a, b| {
            let rank = |sprite: &SpriteView| match strongest_drive(sprite) {
                Some(drive) => (
                    drives
                        .iter()
                        .position(|&name| name == drive.name)
                        .unwrap_or(drives.len()),
                    -drive.level,
                ),
                None => (drives.len() + 1, 0.0),
            };
            let (a_drive, a_level) = rank(a);
            let (b_drive, b_level) = rank(b);
            a_drive
                .cmp(&b_drive)
                .then(a_level.total_cmp(&b_level))
                .then(a.id().cmp(&b.id()))
        }),
    }
    sprites.iter().map(SpriteView::id).collect()
}

/// The list's heading and its rows, from row `first`, as many as `rows`,
/// `width` columns wide, with the highlighted sprite in reverse video.
pub(crate) fn lines(
    app: &App,
    world: &World,
    width: usize,
    first: usize,
    rows: usize,
) -> Vec<Line<'static>> {
    let ids = order(app, world, app.list_sort());
    if ids.is_empty() {
        return vec![Line::from(" No sprites")];
    }
    let sprites: Vec<SpriteView> = ids.iter().filter_map(|&id| world.sprite(id)).collect();
    let labels: Vec<String> = ids.iter().map(|&id| app.names().label(id)).collect();
    let ages: Vec<String> = sprites
        .iter()
        .map(|sprite| group_thousands(sprite.age()))
        .collect();
    let label_width = labels
        .iter()
        .map(|label| label.chars().count())
        .chain(["Sprite".len()])
        .max()
        .unwrap_or(0);
    let age_width = ages
        .iter()
        .map(|age| age.chars().count())
        .chain(["Age".len()])
        .max()
        .unwrap_or(0);
    let drive_width = world
        .data()
        .drives()
        .map(|drive| display_name(drive).chars().count())
        .chain(["Drive".len()])
        .max()
        .unwrap_or(0);
    let heading = format!(
        " {:<label_width$}  {:>age_width$}  {:<drive_width$}  Doing",
        "Sprite", "Age", "Drive"
    );
    let mut lines = vec![Line::styled(
        clipped(&heading, width),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    let chosen = app.list_choice(world);
    let shown = sprites.iter().zip(&labels).zip(&ages).enumerate();
    for (index, ((sprite, label), age)) in shown.skip(first).take(rows) {
        let doing = inspector::doing_line(sprite, app, world).unwrap_or_default();
        let (drive, colour) = match strongest_drive(sprite) {
            Some(drive) => (display_name(drive.name), app.theme.drive_colour(drive.name)),
            None => ("-".to_string(), None),
        };
        let mut drive_style = Style::default();
        if let Some(colour) = colour {
            drive_style = drive_style.fg(colour);
        }
        let head = format!(" {label:<label_width$}  {age:>age_width$}  ");
        let drive = format!("{drive:<drive_width$}");
        let tail = format!("  {doing}");
        let mut room = width;
        let mut spans = Vec::new();
        for (text, style) in [
            (head, Style::default()),
            (drive, drive_style),
            (tail, Style::default()),
        ] {
            let text = clipped(&text, room);
            room -= text.chars().count();
            spans.push(Span::styled(text, style));
        }
        // The highlight runs the width of the list.
        spans.push(Span::raw(" ".repeat(room)));
        let mut line = Line::from(spans);
        if index == chosen {
            line = line.style(Style::default().add_modifier(Modifier::REVERSED));
        }
        lines.push(line);
    }
    lines
}

/// The first `width` characters of `text`.
fn clipped(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}
