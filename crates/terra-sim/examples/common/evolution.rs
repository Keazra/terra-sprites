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
/// of `lasted` a seed and its numbers. A seed that didn't last, or had no
/// sprite alive at the end to measure, each of `dropped`, drops out of G4's
/// count (#222, #226). Returns whether G4's bar of at least 3 numbers is
/// met. Chance moves a number up in some worlds and down in others, and the
/// same way in all ten about 1 time in 500. A number at the edge of its
/// range can only move one way, so check the list for one.
pub fn evolution(starter: &[(String, f32)], lasted: &[(u64, &[f32])], dropped: &[u64]) -> bool {
    let left_out = if dropped.is_empty() {
        String::new()
    } else {
        let seeds: Vec<String> = dropped.iter().map(u64::to_string).collect();
        format!(
            "; seeds {} drop out, as they didn't last or had no sprite alive at the end",
            seeds.join(", ")
        )
    };
    println!(
        "\nEvolution (G4): each gene number's middle value among the living at the end, against \
         the starter's, in the {} seeds that lasted{left_out}.",
        lasted.len()
    );
    if lasted.is_empty() {
        println!("\nG4 isn't met: no seed lasted with sprites alive at the end.");
        return false;
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
        // A seed whose living all lack the number has no middle value for
        // it, so it's left out of the least, middle and most.
        let mut sorted: Vec<f32> = ratios.iter().copied().filter(|r| !r.is_nan()).collect();
        sorted.sort_by(f32::total_cmp);
        let at = |i: usize| sorted.get(i).copied().unwrap_or(f32::NAN);
        moved.push(Moved {
            name,
            starter: *value,
            least: at(0),
            middle: at(sorted.len() / 2),
            most: at(sorted.len().saturating_sub(1)),
            up,
            down,
            gone: ratios.len() - sorted.len(),
        });
    }
    let seeds = lasted.len();
    let all_way = moved.iter().filter(|m| m.every_seed(seeds)).count();
    let met = all_way >= 3;
    println!(
        "\n{all_way} of {} numbers moved by at least a tenth the same way in all {seeds} seeds \
         (bar: at least 3): {}.",
        moved.len(),
        if met { "met" } else { "not met" }
    );
    let mut by_size = moved.clone();
    by_size.sort_by(|a, b| a.distance().total_cmp(&b.distance()).reverse());
    by_size.truncate(25);
    let width = by_size.iter().map(|m| m.name.chars().count()).max();
    let width = width.unwrap_or_default().max(50);
    println!(
        "\n {:<width$} {:>10} {:>6} {:>6} {:>6} {:>4} {:>4} {:>4}",
        "Number", "Starter", "Least", "Middle", "Most", "Up", "Down", "Gone"
    );
    for m in &by_size {
        let mark = if m.every_seed(seeds) { "*" } else { " " };
        println!(
            "{mark}{:<width$} {:>10.4} {:>6.2} {:>6.2} {:>6.2} {:>4} {:>4} {:>4}",
            m.name, m.starter, m.least, m.middle, m.most, m.up, m.down, m.gone
        );
    }
    println!(
        "\n* moved the same way in every seed. Least, middle and most are seeds' ratios to the \
         starter, over the seeds where some of the living carry it; Gone counts those where none do."
    );
    met
}

/// How far one gene number moved from the starter's, across seeds: its
/// least, middle and most ratio to the starter's value, in how many seeds
/// it moved a tenth up and a tenth down, and in how many none of the living
/// carry it.
#[derive(Debug, Clone)]
struct Moved<'a> {
    name: &'a str,
    starter: f32,
    least: f32,
    middle: f32,
    most: f32,
    up: usize,
    down: usize,
    gone: usize,
}

impl Moved<'_> {
    /// Whether it moved a tenth the same way in every one of `seeds`.
    fn every_seed(&self, seeds: usize) -> bool {
        self.up == seeds || self.down == seeds
    }

    /// How far its middle ratio is from 1, for listing the furthest first;
    /// one no seed's living carry lists last.
    fn distance(&self) -> f32 {
        if self.middle.is_nan() {
            -1.0
        } else {
            distance(self.middle)
        }
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
