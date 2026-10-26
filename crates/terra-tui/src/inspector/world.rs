//! The World tab (design §6.1): the population, the grown sprites fertile,
//! the deaths by cause and each object type's count.

use ratatui::style::Style;
use ratatui::text::Line;
use terra_sim::{DeathCause, Fertility, ObjectView, World};

use super::{WIDTH, listed, wrapped};
use crate::app::App;
use crate::policy::{Panel, Subject};
use crate::text;
use crate::text::{cause_name, display_name, thousands};

/// The generations of the sprites alive (M2 design §17.5), from the oldest
/// to the newest, `generations 1 to 6`, or none with no sprites: the same
/// as the population graph reads now (§14.3).
fn generations_line(world: &World) -> Option<Line<'static>> {
    let (label, value) = text::generations_words(world.population_now().generations?);
    Some(Line::from(format!(" {label:<12}{value:>7}")))
}

/// How many grown sprites are fertile, in a pack that breeds and a world
/// with sprites, and beneath, how many room nearby holds back, or that the
/// world is full (M2 design §10.7, #331): `fertile 6`, `too crowded 9`.
fn fertile_lines(world: &World) -> Vec<Line<'static>> {
    if !world.data().breeds() || world.sprites().next().is_none() {
        return Vec::new();
    }
    let count = |fertility| {
        let n = world.sprites().filter(|s| s.fertility() == fertility);
        thousands(n.count() as u64)
    };
    let beneath = if world.is_full() {
        "world full".to_string()
    } else {
        format!("too crowded {}", count(Fertility::TooCrowded))
    };
    let mut lines = vec![Line::from(format!(
        " {:<12}{:>7}",
        "fertile",
        count(Fertility::Fertile)
    ))];
    lines.extend(wrapped(&beneath, 3, Style::default()));
    lines
}

/// The World tab (design §6.1): a small population graph (M2 design §14),
/// if the policy shows it; the data pack; the population, the grown sprites
/// fertile (M2 design §10.7) and the deaths by cause; then each object type
/// with its count, the count in each stage (for a type with more than one)
/// and the total of each counter.
pub(super) fn world_tab(app: &App, world: &World) -> Vec<Line<'static>> {
    let data = world.data();
    // Physiology's causes always; an object only once it has killed.
    let hurt = world
        .deaths_by_cause()
        .map(|(cause, _)| cause)
        .filter(|cause| !DeathCause::PHYSIOLOGY.contains(cause));
    let causes: Vec<DeathCause> = DeathCause::PHYSIOLOGY.into_iter().chain(hurt).collect();
    let deaths: Vec<String> = causes
        .iter()
        .map(|&cause| {
            let n = world.deaths(cause);
            format!("{} {}", cause_name(cause, data), thousands(n))
        })
        .collect();
    let total: u64 = world.deaths_by_cause().map(|(_, n)| n).sum();
    let plain = Style::default();
    let mut lines = Vec::new();
    if app.can_view(Panel::Graph, Subject::World) {
        lines.extend(crate::graph::small(app, world, WIDTH));
        lines.push(Line::default());
    }
    lines.extend([
        Line::from(format!(
            " {:<12}{} v{}",
            "data pack",
            data.name(),
            data.version()
        )),
        Line::from(format!(
            " {:<12}{:>7}",
            "sprites",
            thousands(world.sprites().count() as u64)
        )),
    ]);
    lines.extend(generations_line(world));
    lines.extend(fertile_lines(world));
    lines.push(Line::from(format!(
        " {:<12}{:>7}",
        "deaths",
        thousands(total)
    )));
    lines.extend(wrapped(&listed(&deaths), 3, plain));
    for object_type in data.object_type_names() {
        let objects: Vec<ObjectView> = world
            .objects()
            .filter(|o| o.type_name() == object_type)
            .collect();
        lines.push(Line::from(format!(
            " {:<12}{:>7}",
            display_name(object_type),
            thousands(objects.len() as u64)
        )));
        let stages = data.stage_names(object_type);
        if stages.len() > 1 {
            let counts: Vec<String> = stages
                .iter()
                .map(|&stage| {
                    let n = objects.iter().filter(|o| o.stage() == Some(stage)).count();
                    format!("{stage} {}", thousands(n as u64))
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
                format!("{counter} {}", thousands(total))
            })
            .collect();
        if !totals.is_empty() {
            lines.extend(wrapped(&listed(&totals), 3, plain));
        }
    }
    lines
}
