//! The sprite list (`l`, design §6.1): every sprite the player may see, one
//! to a row, with its age, life stage, strongest drive, generation and what
//! it's doing, sorted as the player chooses and filtered by what they type
//! (#313).

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use terra_sim::{EntityId, LifeStage, SpriteView, World};

use crate::app::{App, strongest_drive};
use crate::inspector;
use crate::listed::listed_enum;
use crate::policy::{Panel, Subject};
use crate::text::{display_name, thousands};

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
    id: EntityId,
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
            id: sprite.id(),
            label: app.names().label(sprite.id()),
            age: thousands(sprite.age()),
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

    /// Whether the words `typed` into the filter are found in any of its
    /// columns as shown (#313), its generation read as `generation 3`, the
    /// column's heading and its number.
    fn matches(&self, typed: &[String]) -> bool {
        let drive = self.drive_shown();
        let generation = format!("generation {}", self.generation);
        [
            self.label.as_str(),
            &self.age,
            self.stage,
            &drive,
            &generation,
            &self.doing,
        ]
        .into_iter()
        .any(|column| found_in(column, typed))
    }
}

/// The words of `text`, in lower case, split at whatever isn't a letter or a
/// digit: `Tobek #12` is `tobek` and `12`.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether the words `typed` into the filter start words of `column`, in
/// order and side by side (#313), so `old` finds the old and not what's
/// holding something, and `gen 3` finds `generation 3`. A word ending in a
/// digit is a whole number: `gen 3` isn't generation 31, nor `#1` Sprite
/// #12. With nothing typed, every column has it.
fn found_in(column: &str, typed: &[String]) -> bool {
    let words = words(column);
    typed.is_empty()
        || words.windows(typed.len()).any(|run| {
            run.iter().zip(typed).all(|(word, typed)| {
                if typed.ends_with(|c: char| c.is_ascii_digit()) {
                    word == typed
                } else {
                    word.starts_with(typed.as_str())
                }
            })
        })
}

/// The sprites the policy lets the list show (design §6.4), by ID, before
/// any filter.
fn unfiltered<'a>(app: &App, world: &'a World) -> Vec<SpriteView<'a>> {
    world
        .sprites()
        .filter(|sprite| app.can_view(Panel::SpriteList, Subject::Sprite(sprite.id())))
        .collect()
}

/// The sprites the list shows, in the order it shows them: those the policy
/// lets it show (design §6.4) that match the filter, if there is one,
/// sorted as the player chose.
pub(crate) fn order(app: &App, world: &World) -> Vec<EntityId> {
    let mut sprites = unfiltered(app, world);
    if let Some(filter) = app.list_filter() {
        let typed = words(filter);
        sprites.retain(|sprite| Row::of(sprite, app, world).matches(&typed));
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
            unfiltered(app, world).len()
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
    // The columns are as wide as the whole list needs, so they keep still
    // while the filter narrows it (#313).
    let every_row: Vec<Row> = unfiltered(app, world)
        .iter()
        .map(|sprite| Row::of(sprite, app, world))
        .collect();
    let shown = ids
        .iter()
        .filter_map(|&id| every_row.iter().find(|row| row.id == id));
    let label_width = widest("Sprite", every_row.iter().map(|row| &row.label));
    let age_width = widest("Age", every_row.iter().map(|row| &row.age));
    let drive_width = widest("Drive", world.data().drives().map(display_name));
    // Each sprite's life stage after its age (M2 design §9.6).
    let stage_width = widest("Life stage", LifeStage::ALL.map(LifeStage::name));
    // Each sprite's generation after its drive (M2 design §17.1, #313).
    let generation_width = widest(
        "Generation",
        every_row.iter().map(|row| row.generation.to_string()),
    );
    let heading = format!(
        " {:<label_width$}  {:>age_width$}  {:<stage_width$}  {:<drive_width$}  \
         {:>generation_width$}  Doing",
        "Sprite", "Age", "Life stage", "Drive", "Generation"
    );
    let mut lines = vec![Line::styled(
        clipped(&heading, width),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    let chosen = app.list_row(world);
    for (index, row) in shown.enumerate().skip(first).take(rows) {
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

/// How wide a column headed `heading` is: as wide as the widest of it and
/// `texts`, in characters.
fn widest<T: AsRef<str>>(heading: &str, texts: impl IntoIterator<Item = T>) -> usize {
    let widths = texts.into_iter().map(|text| text.as_ref().chars().count());
    widths.fold(heading.chars().count(), usize::max)
}

/// The first `width` characters of `text`.
fn clipped(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `typed` is found in `column`.
    fn finds(column: &str, typed: &str) -> bool {
        found_in(column, &words(typed))
    }

    #[test]
    fn a_word_typed_finds_the_start_of_a_word_case_aside() {
        assert!(finds("Tobek #12", "TOB"));
        assert!(!finds("Tobek #12", "bek"), "only from a word's start");
        assert!(finds("old", "old"));
        assert!(!finds("Holding a berry", "old"));
        assert!(finds("Going to drink · 2 tiles to go", "drink"));
    }

    #[test]
    fn words_typed_are_found_in_order_side_by_side() {
        assert!(finds("generation 3", "gen 3"));
        assert!(finds("Going to drink · 2 tiles to go", "to drink"));
        assert!(!finds("Going to drink · 2 tiles to go", "drink going"));
        assert!(!finds("Going to drink · 2 tiles to go", "going drink"));
    }

    #[test]
    fn a_number_typed_is_a_whole_number() {
        assert!(!finds("generation 31", "gen 3"));
        assert!(finds("generation 31", "gen 31"));
        assert!(!finds("Sprite #12", "#1"));
        assert!(finds("Sprite #12", "#12"));
        assert!(finds("Sprite #12", "12"), "with or without the #");
        assert!(finds("3,410", "3,410"));
    }

    #[test]
    fn with_nothing_typed_every_column_has_it() {
        assert!(finds("-", ""));
        assert!(finds("young", "  "));
        assert!(finds("-", "-"), "a dash is no word");
    }
}
