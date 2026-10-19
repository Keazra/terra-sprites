//! How far the species' genes moved from the starter's (M2 design §12.7,
//! G4), for the examples that judge it (`generations`, `childhood`).
//!
//! Examples include this module via `#[path]`, so each example is its own
//! crate and may not call every helper.
#![allow(dead_code)]

use terra_sim::{DataPack, Drift, World, drift, gene_values, middle_values};

/// The starter genome's numbers, named by what each gene does and when, in
/// genome order.
pub fn starter_numbers(data: &DataPack) -> Vec<(String, f32)> {
    let genome = data.starter();
    gene_values(&genome.genes(data), &genome.switches())
}

/// Each of the starter's numbers' middle value among the sprites alive in
/// `world`, eggs aside; empty if none is alive, and not a number where none
/// has it (M2 design §13). By the end of a long run, all were hatched.
pub fn middles(world: &World, starter: &[(String, f32)]) -> Vec<f32> {
    middle_values(world, starter)
        .map(|middles| {
            middles
                .into_iter()
                .map(|middle| middle.unwrap_or(f32::NAN))
                .collect()
        })
        .unwrap_or_default()
}

/// Prints how far the species' genes moved from the starter's (M2 design
/// §12.7, G4): each number whose middle value among the living at the end
/// moved by at least a tenth the same way in every seed that lasted, each
/// of `lasted` a seed and its numbers. A seed that didn't last, each of
/// `dropped`, drops out of G4's count (#222, #226). Chance moves a number up
/// in some worlds and down in others, and the same way in all ten about 1
/// time in 500. A number at the edge of its range can only move one way, so
/// check the list for one.
pub fn evolution(starter: &[(String, f32)], lasted: &[(u64, &[f32])], dropped: &[u64]) {
    let left_out = if dropped.is_empty() {
        String::new()
    } else {
        let seeds: Vec<String> = dropped.iter().map(u64::to_string).collect();
        format!("; seeds {} didn't last and drop out", seeds.join(", "))
    };
    println!(
        "\nEvolution (G4): each gene number's middle value among the living at the end, against \
         the starter's, in the {} seeds that lasted{left_out}.",
        lasted.len()
    );
    if lasted.is_empty() {
        return;
    }
    // Each number's ratio to the starter's, seed by seed.
    let mut moved = Vec::new();
    for (index, (name, value)) in starter.iter().enumerate() {
        if *value == 0.0 {
            continue;
        }
        let ratios: Vec<f32> = lasted
            .iter()
            .map(|(_, numbers)| numbers[index] / value)
            .collect();
        let drifted = |way| ratios.iter().filter(|&&r| drift(r) == Some(way)).count();
        let (up, down) = (drifted(Drift::Up), drifted(Drift::Down));
        let mut sorted = ratios.clone();
        sorted.sort_by(f32::total_cmp);
        moved.push(Moved {
            name,
            starter: *value,
            least: sorted[0],
            middle: sorted[sorted.len() / 2],
            most: sorted[sorted.len() - 1],
            up,
            down,
        });
    }
    let seeds = lasted.len();
    let all_way = moved.iter().filter(|m| m.every_seed(seeds)).count();
    println!(
        "\n{all_way} of {} numbers moved by at least a tenth the same way in all {seeds} seeds \
         (bar: at least 3).",
        moved.len()
    );
    let mut by_size = moved.clone();
    by_size.sort_by(|a, b| distance(b.middle).total_cmp(&distance(a.middle)));
    by_size.truncate(25);
    let width = by_size.iter().map(|m| m.name.chars().count()).max();
    let width = width.unwrap_or_default().max(50);
    println!(
        "\n {:<width$} {:>10} {:>6} {:>6} {:>6} {:>4} {:>4}",
        "Number", "Starter", "Least", "Middle", "Most", "Up", "Down"
    );
    for m in &by_size {
        let mark = if m.every_seed(seeds) { "*" } else { " " };
        println!(
            "{mark}{:<width$} {:>10.4} {:>6.2} {:>6.2} {:>6.2} {:>4} {:>4}",
            m.name, m.starter, m.least, m.middle, m.most, m.up, m.down
        );
    }
    println!(
        "\n* moved the same way in every seed. Least, middle and most are seeds' ratios to the starter."
    );
}

/// How far one gene number moved from the starter's, across seeds: its
/// least, middle and most ratio to the starter's value, and in how many
/// seeds it moved a tenth up and a tenth down.
#[derive(Debug, Clone)]
struct Moved<'a> {
    name: &'a str,
    starter: f32,
    least: f32,
    middle: f32,
    most: f32,
    up: usize,
    down: usize,
}

impl Moved<'_> {
    /// Whether it moved a tenth the same way in every one of `seeds`.
    fn every_seed(&self, seeds: usize) -> bool {
        self.up == seeds || self.down == seeds
    }
}

/// How far a ratio is from 1, either way: halving and doubling come out
/// alike.
fn distance(ratio: f32) -> f32 {
    if ratio >= 1.0 {
        ratio - 1.0
    } else {
        1.0 / ratio - 1.0
    }
}
