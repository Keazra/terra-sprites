//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.

use ratatui::style::{Color, Style};
use ratatui::text::Line;
use terra_sim::{
    ChemicalKind, ChemicalLevel, DeathCause, EmitterMode, Expression, GeneView, ObjectView,
    SpriteView, Trait, World,
};

use crate::app::{App, Selection, Tab};
use crate::text::{
    cause_name, change, display_name, group_thousands, level, signed, significant, sprite_label,
    whole,
};

/// The inspector's width, in columns, border included (design §6.1).
pub(crate) const INSPECTOR_WIDTH: u16 = 46;

/// The columns inside the inspector's border.
const WIDTH: usize = INSPECTOR_WIDTH as usize - 2;

/// What a sprite tab says with no sprite selected.
const NOTHING_SELECTED: &str = " No sprite selected: click one, or press Tab";

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
    let labels = app.selection().map(|selection| {
        let id = selection.id();
        (sprite_label(id), format!("#{}", id.0))
    });
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
    match (app.tab(), app.selection()) {
        (Tab::World, _) => world_tab(world),
        (_, None) => vec![Line::from(NOTHING_SELECTED)],
        (tab, Some(Selection::Living(id))) => match world.sprite(id) {
            Some(sprite) => sprite_tab(tab, &sprite),
            None => Vec::new(),
        },
        (_, Some(Selection::Dead { id, cause, age })) => {
            let text = format!(
                "{} died of {} at age{BOUND}{}",
                unbroken(&sprite_label(id)),
                unbroken(cause_name(cause)),
                group_thousands(age)
            );
            wrapped(&text, 1, Style::default())
        }
    }
}

/// The first line shown of a tab `length` lines long in `rows` rows, when
/// it's scrolled `scroll` lines: no further than where its last line comes
/// into view, since the tab may have got shorter, or the rows more.
pub(crate) fn first_shown(scroll: usize, length: usize, rows: usize) -> usize {
    scroll.min(length.saturating_sub(rows))
}

/// A sprite tab's lines for `sprite`.
fn sprite_tab(tab: Tab, sprite: &SpriteView) -> Vec<Line<'static>> {
    match tab {
        Tab::Body => body_tab(sprite),
        Tab::Chem => chem_tab(sprite),
        Tab::Genome => genome_tab(sprite),
        Tab::World => Vec::new(),
    }
}

/// The Body tab (design §6.1): age and traits, a bar for each drive, and
/// the physical levels, three to a line.
fn body_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let traits = sprite.traits();
    let mut lines = vec![
        format!(
            " age {} · {}",
            group_thousands(sprite.age()),
            trait_text(Trait::Lifespan, traits.lifespan)
        ),
        format!(
            " {} · {}",
            trait_text(Trait::Speed, traits.speed),
            trait_text(Trait::SenseRadius, traits.sense_radius)
        ),
        String::new(),
    ];
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    for drive in chemicals.iter().filter(|c| c.kind == ChemicalKind::Drive) {
        let line = format!(
            " {:<12}{} {} {}",
            display_name(drive.name),
            bar(drive.level),
            level(drive.level),
            trend(drive.change)
        );
        lines.push(line.trim_end().to_string());
    }
    lines.push(String::new());
    let physical: Vec<String> = chemicals
        .iter()
        .filter(|c| c.kind == ChemicalKind::Physical)
        .map(|c| format!("{} {}", display_name(c.name), level(c.level)))
        .collect();
    for three in physical.chunks(3) {
        lines.push(format!(" {}", three.join(" · ")));
    }
    lines.into_iter().map(Line::from).collect()
}

/// A level as a bar ten cells long, filled to the nearest tenth.
fn bar(level: f32) -> String {
    // `as` makes a NaN 0.
    let filled = ((level * 10.0).round() as usize).min(10);
    format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled))
}

/// An arrow for which way a level went over the last tick, or nothing when
/// the change is too small to show.
fn trend(amount: f32) -> &'static str {
    match change(amount) {
        None => "",
        Some(_) if amount > 0.0 => "▲",
        Some(_) => "▼",
    }
}

/// A trait and its value: `speed 7.25`, `sense 9.5`, `lifespan 61,204`.
fn trait_text(which: Trait, value: f32) -> String {
    match which {
        Trait::Speed => format!("speed {}", significant(value)),
        Trait::SenseRadius => format!("sense {}", significant(value)),
        Trait::Lifespan => format!("lifespan {}", whole(f64::from(value))),
    }
}

/// The Chem tab (design §6.1): each chemical on its own line with its level
/// and its change per tick, the physical chemicals, then the signal
/// chemicals; then the hormones' levels, four to a line.
fn chem_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    let line = |c: &ChemicalLevel| {
        let amount = change(c.change).unwrap_or_default();
        let name = display_name(c.name);
        let text = format!(" {name:<13}{:>4}  {amount}", level(c.level));
        text.trim_end().to_string()
    };
    let of_kinds = |kinds: &[ChemicalKind]| {
        chemicals
            .iter()
            .filter(|c| kinds.contains(&c.kind))
            .collect::<Vec<_>>()
    };
    let mut lines: Vec<String> = of_kinds(&[ChemicalKind::Physical])
        .into_iter()
        .map(line)
        .collect();
    lines.push(String::new());
    lines.extend(
        of_kinds(&[ChemicalKind::Drive, ChemicalKind::LearningSignal])
            .into_iter()
            .map(line),
    );
    lines.push(" HORMONES".into());
    for four in of_kinds(&[ChemicalKind::Hormone]).chunks(4) {
        let cells: Vec<String> = four
            .iter()
            .map(|c| format!("{:<4}{:>4}", display_name(c.name), level(c.level)))
            .collect();
        lines.push(format!(" {}", cells.join("   ")));
    }
    lines.into_iter().map(Line::from).collect()
}

/// A group of genes on the Genome tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneGroup {
    Traits,
    HalfLives,
    Reactions,
    Emitters,
    Receptors,
    StartingLevels,
    Unknown,
}

impl GeneGroup {
    /// Every group, in the tab's order. Traits come first, since they share
    /// one line.
    const ALL: [GeneGroup; 7] = [
        GeneGroup::Traits,
        GeneGroup::HalfLives,
        GeneGroup::Reactions,
        GeneGroup::Emitters,
        GeneGroup::Receptors,
        GeneGroup::StartingLevels,
        GeneGroup::Unknown,
    ];

    /// The group `gene` goes in.
    fn of(gene: &GeneView) -> GeneGroup {
        match gene {
            GeneView::Trait { .. } => GeneGroup::Traits,
            GeneView::HalfLife { .. } => GeneGroup::HalfLives,
            GeneView::Reaction { .. } => GeneGroup::Reactions,
            GeneView::Emitter { .. } => GeneGroup::Emitters,
            GeneView::Receptor { .. } => GeneGroup::Receptors,
            GeneView::InitialConcentration { .. } => GeneGroup::StartingLevels,
            GeneView::Unknown { .. } => GeneGroup::Unknown,
        }
    }

    /// The group's heading.
    fn heading(self) -> &'static str {
        match self {
            GeneGroup::Traits => "TRAITS",
            GeneGroup::HalfLives => "HALF-LIVES",
            GeneGroup::Reactions => "REACTIONS",
            GeneGroup::Emitters => "EMITTERS",
            GeneGroup::Receptors => "RECEPTORS",
            GeneGroup::StartingLevels => "STARTING LEVELS",
            GeneGroup::Unknown => "UNKNOWN GENES",
        }
    }
}

/// The Genome tab (design §6.1): the genes grouped under headings, each
/// group in genome order, as plain lines. The expressed traits share one
/// line. A gene with no effect is dimmed, with the reason below it.
fn genome_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let genes = sprite.genes();
    let mut lines = Vec::new();
    for group in GeneGroup::ALL {
        let members: Vec<&(GeneView, Expression)> = genes
            .iter()
            .filter(|(gene, _)| GeneGroup::of(gene) == group)
            .collect();
        if members.is_empty() {
            continue;
        }
        lines.push(Line::from(format!(" {}", group.heading())));
        let (together, apart): (Vec<_>, Vec<_>) = members.into_iter().partition(|(gene, how)| {
            matches!(gene, GeneView::Trait { .. }) && *how == Expression::Expressed
        });
        if !together.is_empty() {
            let traits: Vec<String> = together.iter().map(|(gene, _)| gene_text(gene)).collect();
            lines.extend(wrapped(&listed(&traits), 1, Style::default()));
        }
        for (gene, how) in apart {
            let reason = match how {
                Expression::Expressed => {
                    lines.extend(wrapped(&gene_text(gene), 1, Style::default()));
                    continue;
                }
                Expression::Flagged(reason) => format!("flagged: {reason}"),
                Expression::Unexpressed => "unexpressed: an earlier gene sets this".into(),
                Expression::Unknown => "unknown: this version can't read it".into(),
            };
            let dim = Style::default().fg(Color::DarkGray);
            lines.extend(wrapped(&gene_text(gene), 1, dim));
            lines.extend(wrapped(&reason, 3, dim));
        }
    }
    lines
}

/// Keeps a word with the number after it when a line wraps.
const BOUND: char = '\u{a0}';

/// A gene as a plain line: `low energy → hunger +.00428 past .5`.
fn gene_text(gene: &GeneView) -> String {
    let past = |threshold: f32| {
        if threshold == 0.0 {
            String::new()
        } else {
            format!(" past{BOUND}{}", significant(threshold))
        }
    };
    match *gene {
        GeneView::Trait { which, value } => trait_text(which, value),
        GeneView::HalfLife { chem, ticks: 1 } => {
            format!("{} halves every tick", display_name(chem))
        }
        GeneView::HalfLife { chem, ticks } => format!(
            "{} halves every {} ticks",
            display_name(chem),
            group_thousands(u64::from(ticks))
        ),
        GeneView::Reaction {
            ref reactants,
            ref products,
            rate,
        } => {
            let side = |terms: &[(&str, u8)]| {
                if terms.is_empty() {
                    return "nothing".to_string();
                }
                let terms: Vec<String> = terms
                    .iter()
                    .map(|&(chem, n)| match n {
                        1 => display_name(chem),
                        n => format!("{n} {}", display_name(chem)),
                    })
                    .collect();
                terms.join(" + ")
            };
            format!(
                "{} → {}, rate{BOUND}{}",
                side(reactants),
                side(products),
                significant(rate)
            )
        }
        GeneView::Emitter {
            locus,
            mode,
            invert,
            threshold,
            gain,
            chem,
        } => {
            let locus = display_name(locus);
            let source = match mode {
                EmitterMode::Level if invert => format!("low {locus}"),
                EmitterMode::Level => locus,
                EmitterMode::Rise => format!("{locus} rises"),
                EmitterMode::Fall => format!("{locus} falls"),
            };
            format!(
                "{source} → {} {}{}",
                display_name(chem),
                signed(gain),
                past(threshold)
            )
        }
        GeneView::Receptor {
            chem,
            threshold,
            gain,
            target,
        } => format!(
            "{}{} → {} {}",
            display_name(chem),
            past(threshold),
            display_name(target),
            signed(gain)
        ),
        GeneView::InitialConcentration { chem, value } => {
            format!(
                "{} starts at{BOUND}{}",
                display_name(chem),
                significant(value)
            )
        }
        GeneView::Unknown {
            type_id,
            version,
            bytes: 1,
        } => format!("type {type_id}, version {version}, 1 byte"),
        GeneView::Unknown {
            type_id,
            version,
            bytes,
        } => format!("type {type_id}, version {version}, {bytes} bytes"),
    }
}

/// `text` with its spaces kept together when a line wraps.
fn unbroken(text: &str) -> String {
    text.replace(' ', &BOUND.to_string())
}

/// `items` as one line, `a · b · c`, which wraps only after a dot, never
/// inside an item.
fn listed(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|item| unbroken(item)).collect();
    items.join(&format!("{BOUND}· "))
}

/// `text` wrapped at word boundaries to fit the inspector, its first line
/// indented `indent` columns and the rest 3, all in `style`.
fn wrapped(text: &str, indent: usize, style: Style) -> Vec<Line<'static>> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = " ".repeat(indent);
    let mut empty = true;
    for word in text.split(' ') {
        // A word after another needs a space before it. The first word on a
        // line goes there whether it fits or not.
        if !empty && line.chars().count() + 1 + word.chars().count() > WIDTH {
            lines.push(std::mem::replace(&mut line, " ".repeat(3)));
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    lines.push(line);
    lines
        .into_iter()
        .map(|line| Line::styled(line.replace(BOUND, " "), style))
        .collect()
}

/// The World tab (design §6.1): the data pack; the population, and the
/// deaths by cause; then each object type with its count, the count in each
/// stage (for a type with more than one) and the total of each counter.
fn world_tab(world: &World) -> Vec<Line<'static>> {
    let data = world.data();
    let deaths: Vec<String> = DeathCause::ALL
        .iter()
        .map(|&cause| {
            let n = world.deaths(cause);
            format!("{} {}", cause_name(cause), group_thousands(n))
        })
        .collect();
    let total: u64 = DeathCause::ALL.iter().map(|&c| world.deaths(c)).sum();
    let plain = Style::default();
    let mut lines: Vec<Line<'static>> = vec![
        Line::from(format!(
            " {:<12}{} v{}",
            "data pack",
            data.name(),
            data.version()
        )),
        Line::from(format!(
            " {:<12}{:>7}",
            "sprites",
            group_thousands(world.sprites().count() as u64)
        )),
        Line::from(format!(" {:<12}{:>7}", "deaths", group_thousands(total))),
    ];
    lines.extend(wrapped(&listed(&deaths), 3, plain));
    for object_type in data.object_type_names() {
        let objects: Vec<ObjectView> = world
            .objects()
            .filter(|o| o.type_name() == object_type)
            .collect();
        lines.push(Line::from(format!(
            " {:<12}{:>7}",
            display_name(object_type),
            group_thousands(objects.len() as u64)
        )));
        let stages = data.stage_names(object_type);
        if stages.len() > 1 {
            let counts: Vec<String> = stages
                .iter()
                .map(|&stage| {
                    let n = objects.iter().filter(|o| o.stage() == Some(stage)).count();
                    format!("{stage} {}", group_thousands(n as u64))
                })
                .collect();
            lines.extend(wrapped(&listed(&counts), 3, plain));
        }
        let totals: Vec<String> = data
            .counter_names(object_type)
            .iter()
            .map(|&counter| {
                let total: u64 = objects
                    .iter()
                    .map(|o| u64::from(o.counter(counter).unwrap_or(0)))
                    .sum();
                format!("{counter} {}", group_thousands(total))
            })
            .collect();
        if !totals.is_empty() {
            lines.extend(wrapped(&listed(&totals), 3, plain));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // No sprite's number or name is long enough yet to crowd the tabs, so
    // this tests the title's fitting directly.
    #[test]
    fn a_label_too_long_for_the_title_is_shortened_before_the_tabs_are() {
        let tabs = "[Body] Chem Genome World";
        let fit = |label: &str, short: &str| fitted_title(Some((label, short)), tabs);
        assert_eq!(
            fit("Sprite #530", "#530"),
            " Sprite #530 ── [Body] Chem Genome World "
        );
        assert_eq!(
            fit("Sprite #1234567", "#1234567"),
            " #1234567 ── [Body] Chem Genome World "
        );
        let crowded = "[Body] Brain Chem Genome World Lineage";
        assert_eq!(
            fitted_title(Some(("Sprite #1234567", "#1234567")), crowded),
            format!(" {crowded} "),
            "with no room for even the number, the tabs alone"
        );
        assert_eq!(fitted_title(None, tabs), format!(" {tabs} "));
    }

    // Levels never leave 0 to 1 (design §4.4), but a bar mustn't crash the
    // screen if one ever did.
    #[test]
    fn a_drive_bar_is_ten_cells_whatever_the_level() {
        assert_eq!(bar(0.42), "████░░░░░░");
        assert_eq!(bar(1.05), "██████████");
        assert_eq!(bar(-0.3), "░░░░░░░░░░");
        assert_eq!(bar(f32::NAN), "░░░░░░░░░░");
    }
}
