//! The sprite list (`l`, design §6.1): every sprite the player may see, one
//! to a row, with its age, its strongest drive, its generation and what it's
//! doing, sorted as the player chooses and filtered by what they type (#313).

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use terra_sim::{EntityId, LifeStage, SpriteView, World};

use crate::app::{App, strongest_drive};
use crate::inspector;
use crate::listed::listed_enum;
use crate::policy::{Panel, Subject};
use crate::text::{display_name, group_thousands};

listed_enum! {
    /// What the sprite list is sorted by.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SortBy {
        /// By ID, lowest first.
        Number => "number",
        /// Named sprites first, by name, then the rest by ID.
        Name => "name",
        /// Oldest first.
        Age => "age",
        /// By strongest drive, in the data pack's order, strongest first
        /// within each; sprites with none above half last.
        Drive => "drive",
    }

    /// Every order, in the order `Tab` goes through them.
    pub const ALL;

    /// What the list's title calls it.
    pub fn label(self) -> &'static str;
}

/// The most characters the list's filter takes (#313): more than a name has.
pub(crate) const MAX_FILTER_CHARS: usize = 24;

/// A sprite's row as the list shows it, a column at a time.
struct Row<'a> {
    label: String,
    age: String,
    stage: &'static str,
    /// Its strongest drive above half, by the data pack's name, if any.
    drive: Option<&'a str>,
    generation: u32,
    doing: String,
}

impl<'a> Row<'a> {
    fn of(sprite: &SpriteView<'a>, app: &App, world: &World) -> Row<'a> {
        Row {
            label: app.names().label(sprite.id()),
            age: group_thousands(sprite.age()),
            stage: sprite.life_stage().name(),
            drive: strongest_drive(sprite).map(|drive| drive.name),
            generation: sprite.generation(),
            doing: inspector::doing_line(sprite, app, world).unwrap_or_default(),
        }
    }

    /// What its Drive column says: the drive, or `-` with none above half.
    fn drive_shown(&self) -> String {
        self.drive.map_or_else(|| "-".to_string(), display_name)
    }

    /// Whether `filter`, its ends trimmed, is in any of its columns as
    /// shown, case aside (#313): its generation as `gen 3`, as the column's
    /// heading and its number.
    fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim().to_lowercase();
        [
            self.label.clone(),
            self.age.clone(),
            self.stage.to_string(),
            self.drive_shown(),
            format!("gen {}", self.generation),
            self.doing.clone(),
        ]
        .iter()
        .any(|cell| cell.to_lowercase().contains(&filter))
    }
}

/// The sprites the policy lets the list show (design §6.4), by ID, whatever
/// the filter.
fn listed<'a>(app: &App, world: &'a World) -> Vec<SpriteView<'a>> {
    world
        .sprites()
        .filter(|sprite| app.can_view(Panel::SpriteList, Subject::Sprite(sprite.id())))
        .collect()
}

/// The sprites the list shows, in the order it shows them: those the policy
/// lets it show (design §6.4) that match the filter, if there is one,
/// sorted as the player chose.
pub(crate) fn order(app: &App, world: &World) -> Vec<EntityId> {
    let mut sprites = listed(app, world);
    if let Some(filter) = app.list_filter() {
        sprites.retain(|sprite| Row::of(sprite, app, world).matches(filter));
    }
    let drives: Vec<&str> = world.data().drives().collect();
    match app.list_sort() {
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

/// The list's title: its order, and while it's filtered, the filter being
/// typed and how many of the sprites it would list match (#313), as
/// ` Sprites ── sorted by name ── filter: old_ ── 3 of 87 `.
pub(crate) fn title(app: &App, world: &World) -> String {
    let sorted = format!(" Sprites ── sorted by {} ", app.list_sort().label());
    match app.list_filter() {
        Some(filter) => format!(
            "{sorted}── filter: {filter}_ ── {} of {} ",
            order(app, world).len(),
            listed(app, world).len()
        ),
        None => sorted,
    }
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
    let ids = order(app, world);
    if ids.is_empty() {
        return vec![Line::from(match app.list_filter() {
            Some(_) => " No sprite matches",
            None => " No sprites",
        })];
    }
    let shown: Vec<Row> = ids
        .iter()
        .filter_map(|&id| world.sprite(id))
        .map(|sprite| Row::of(&sprite, app, world))
        .collect();
    // The columns are as wide as the whole list needs, so they keep still
    // while the filter narrows it (#313).
    let every: Vec<Row> = listed(app, world)
        .iter()
        .map(|sprite| Row::of(sprite, app, world))
        .collect();
    let label_width = every
        .iter()
        .map(|row| row.label.chars().count())
        .chain(["Sprite".len()])
        .max()
        .unwrap_or(0);
    let age_width = every
        .iter()
        .map(|row| row.age.chars().count())
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
    // Each sprite's life stage after its age (M2 design §9.6).
    let stage_width = [LifeStage::Young, LifeStage::Adult, LifeStage::Old]
        .iter()
        .map(|stage| stage.name().len())
        .chain(["Life stage".len()])
        .max()
        .unwrap_or(0);
    // Each sprite's generation after its drive (M2 design §17.5, #313).
    let generation_width = every
        .iter()
        .map(|row| row.generation.to_string().len())
        .chain(["Gen".len()])
        .max()
        .unwrap_or(0);
    let heading = format!(
        " {:<label_width$}  {:>age_width$}  {:<stage_width$}  {:<drive_width$}  \
         {:>generation_width$}  Doing",
        "Sprite", "Age", "Life stage", "Drive", "Gen"
    );
    let mut lines = vec![Line::styled(
        clipped(&heading, width),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    let chosen = app.list_row(world);
    for (index, row) in shown.iter().enumerate().skip(first).take(rows) {
        let colour = row.drive.and_then(|drive| app.theme.drive_colour(drive));
        let mut drive_style = Style::default();
        if let Some(colour) = colour {
            drive_style = drive_style.fg(colour);
        }
        let head = format!(
            " {:<label_width$}  {:>age_width$}  {:<stage_width$}  ",
            row.label, row.age, row.stage
        );
        let drive = format!("{:<drive_width$}", row.drive_shown());
        let tail = format!("  {:>generation_width$}  {}", row.generation, row.doing);
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
