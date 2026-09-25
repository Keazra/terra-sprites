//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.

use ratatui::text::Line;
use terra_sim::{ChemicalKind, ChemicalLevel, ObjectView, SpriteView, World};

use crate::app::{App, Selection, Tab};
use crate::ui::{cause_name, display_name, group_thousands, sprite_label};

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
        Tab::Chem | Tab::Genome | Tab::World => Vec::new(),
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
