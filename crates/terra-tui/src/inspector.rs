//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.

use ratatui::style::{Color, Style};
use ratatui::text::Line;
use terra_sim::{
    ChemicalKind, ChemicalLevel, EmitterMode, Expression, GeneView, ObjectView, SpriteView, World,
};

use crate::app::{App, Selection, Tab};
use crate::ui::{INSPECTOR_WIDTH, cause_name, display_name, group_thousands, sprite_label};

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
            let name = tab_name(tab);
            if tab == app.tab() {
                format!("[{name}]")
            } else {
                name.to_string()
            }
        })
        .collect();
    let tabs = tabs.join(" ");
    match app.selection() {
        Some(selection) => format!(" {} ── {tabs} ", sprite_label(selection.id())),
        None => format!(" {tabs} "),
    }
}

fn tab_name(tab: Tab) -> &'static str {
    match tab {
        Tab::Body => "Body",
        Tab::Chem => "Chem",
        Tab::Genome => "Genome",
        Tab::World => "World",
    }
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
        (_, Some(Selection::Dead { id, cause, age })) => vec![Line::from(format!(
            " {} died of {} at age {}",
            sprite_label(id),
            cause_name(cause),
            group_thousands(age)
        ))],
    }
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
            " age {} · lifespan {}",
            group_thousands(sprite.age()),
            group_thousands(traits.lifespan.round() as u64)
        ),
        format!(
            " speed {} · sense {}",
            significant(traits.speed),
            significant(traits.sense_radius)
        ),
        String::new(),
    ];
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    for drive in chemicals.iter().filter(|c| c.kind == ChemicalKind::Drive) {
        let filled = (drive.level * 10.0).round() as usize;
        let line = format!(
            " {:<12}{}{} {} {}",
            drive.name,
            "█".repeat(filled),
            "░".repeat(10 - filled),
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

/// The Chem tab (design §6.1): each chemical on its own line with its level
/// and its change per tick, the physical chemicals, then the signal
/// chemicals; then the hormones' levels, four to a line.
fn chem_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    let line = |c: &ChemicalLevel| {
        let text = format!(" {:<13}{:>4}  {}", c.name, level(c.level), change(c.change));
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
            .map(|c| format!("{:<4}{:>4}", c.name, level(c.level)))
            .collect();
        lines.push(format!(" {}", cells.join("   ")));
    }
    lines.into_iter().map(Line::from).collect()
}

/// The Genome tab's groups, in order, with their headings. Traits come
/// first, since they share one line.
const GENE_GROUPS: [&str; 7] = [
    "TRAITS",
    "HALF-LIVES",
    "REACTIONS",
    "EMITTERS",
    "RECEPTORS",
    "STARTING LEVELS",
    "UNKNOWN GENES",
];

/// Which of `GENE_GROUPS` a gene goes in.
fn gene_group(gene: &GeneView) -> usize {
    match gene {
        GeneView::Trait { .. } => 0,
        GeneView::HalfLife { .. } => 1,
        GeneView::Reaction { .. } => 2,
        GeneView::Emitter { .. } => 3,
        GeneView::Receptor { .. } => 4,
        GeneView::InitialConcentration { .. } => 5,
        GeneView::Unknown { .. } => 6,
    }
}

/// The Genome tab (design §6.1): the genes grouped under headings, each
/// group in genome order, as plain lines. The expressed traits share one
/// line. A gene with no effect is dimmed, with the reason below it.
fn genome_tab(sprite: &SpriteView) -> Vec<Line<'static>> {
    let genes = sprite.genes();
    let mut lines = Vec::new();
    for (group, heading) in GENE_GROUPS.iter().enumerate() {
        let members: Vec<&(GeneView, Expression)> = genes
            .iter()
            .filter(|(gene, _)| gene_group(gene) == group)
            .collect();
        if members.is_empty() {
            continue;
        }
        lines.push(Line::from(format!(" {heading}")));
        let (together, apart): (Vec<_>, Vec<_>) = members.into_iter().partition(|(gene, how)| {
            matches!(gene, GeneView::Trait { .. }) && *how == Expression::Expressed
        });
        if !together.is_empty() {
            let traits: Vec<String> = together.iter().map(|(gene, _)| gene_text(gene)).collect();
            lines.extend(wrapped(&traits.join(" · "), 1, Style::default()));
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
        GeneView::Trait { name, value } => match name {
            "lifespan" => format!("lifespan {}", group_thousands(value.round() as u64)),
            "sense_radius" => format!("sense {}", significant(value)),
            name => format!("{} {}", display_name(name), significant(value)),
        },
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

/// `text` wrapped at word boundaries to fit the inspector, its first line
/// indented `indent` columns and the rest 3, all in `style`.
fn wrapped(text: &str, indent: usize, style: Style) -> Vec<Line<'static>> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = " ".repeat(indent);
    let mut empty = true;
    for word in text.split(' ') {
        let fits = line.chars().count() + 1 + word.chars().count() <= WIDTH;
        if !empty && !fits {
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

/// A gene value with its sign: `+.004`, `-.5`.
fn signed(value: f32) -> String {
    if value < 0.0 {
        significant(value)
    } else {
        format!("+{}", significant(value))
    }
}

/// A change per tick as the inspector shows it: to 4 decimals, with its sign
/// and no leading zero, or nothing when it rounds to 0.
fn change(change: f32) -> String {
    if change.abs() < SMALLEST_CHANGE {
        return String::new();
    }
    let sign = if change > 0.0 { "+" } else { "-" };
    format!(
        "{sign}{}",
        without_leading_zero(&format!("{:.4}", change.abs()))
    )
}

/// A level as the inspector shows it: two decimals, with no leading zero.
fn level(level: f32) -> String {
    without_leading_zero(&format!("{level:.2}"))
}

/// The smallest change the inspector shows: .0001, to 4 decimals.
const SMALLEST_CHANGE: f32 = 0.00005;

/// An arrow for which way a level went over the last tick, or nothing when
/// the change is too small to show.
fn trend(change: f32) -> &'static str {
    if change >= SMALLEST_CHANGE {
        "▲"
    } else if change <= -SMALLEST_CHANGE {
        "▼"
    } else {
        ""
    }
}

/// `value` to 3 significant figures, with no trailing zeros and no leading
/// zero: `7.25`, `9.5`, `.00428`, `61200`.
fn significant(value: f32) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (2 - magnitude).max(0) as usize;
    let text = format!("{value:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    without_leading_zero(text)
}

/// `0.42` → `.42`, `-0.5` → `-.5`.
fn without_leading_zero(number: &str) -> String {
    if let Some(rest) = number.strip_prefix("0.") {
        format!(".{rest}")
    } else if let Some(rest) = number.strip_prefix("-0.") {
        format!("-.{rest}")
    } else {
        number.to_string()
    }
}

/// The World tab (design §6.1): the data pack, then each object type with its
/// count, the count in each stage (for a type with more than one) and the
/// total of each counter.
fn world_tab(world: &World) -> Vec<Line<'static>> {
    let data = world.data();
    let mut lines = vec![format!(
        " {:<12}{} v{}",
        "data pack",
        data.name(),
        data.version()
    )];
    for object_type in data.object_type_names() {
        let objects: Vec<ObjectView> = world
            .objects()
            .filter(|o| o.type_name() == object_type)
            .collect();
        lines.push(format!(
            " {:<12}{:>7}",
            display_name(object_type),
            group_thousands(objects.len() as u64)
        ));
        let stages = data.stage_names(object_type);
        if stages.len() > 1 {
            let counts: Vec<String> = stages
                .iter()
                .map(|&stage| {
                    let n = objects.iter().filter(|o| o.stage() == Some(stage)).count();
                    format!("{stage} {}", group_thousands(n as u64))
                })
                .collect();
            lines.push(format!("   {}", counts.join(" · ")));
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
            lines.push(format!("   {}", totals.join(" · ")));
        }
    }
    lines.into_iter().map(Line::from).collect()
}
