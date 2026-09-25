//! What the inspector shows (design §6.1): its title, and the lines of the
//! open tab, for drawing and for knowing how far a tab scrolls.

use ratatui::text::Line;
use terra_sim::{ObjectView, World};

use crate::app::{App, Selection, Tab};
use crate::ui::{display_name, group_thousands, sprite_label};

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
        Some(Selection::Living(id)) => format!(" {} ── {tabs} ", sprite_label(id)),
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
        (_, Some(Selection::Living(_))) => Vec::new(),
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
