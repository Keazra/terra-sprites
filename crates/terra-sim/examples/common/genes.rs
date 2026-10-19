//! Genes counted by what each does and when it works (M2 design §13), for
//! the examples that set the living's genes against the starter's
//! (`generations`, `childhood`).
//!
//! Examples include this module via `#[path]`, so each example is its own
//! crate and may not call every helper.
#![allow(dead_code)]

use std::collections::BTreeMap;

use terra_sim::{GeneView, LifeStage, Switch};

/// How many of `genes` there are of each, by what each does and when it
/// works, `switches` saying when: `thirst → Drink`, `always → Wander from
/// Adult`. Genes this build can't read aren't counted.
pub fn counted(genes: &[GeneView], switches: &[Switch]) -> BTreeMap<String, usize> {
    counted_where(genes, switches, |_| true)
}

/// How many of `genes` there are of each, as `counted` says, of those at
/// the indices `keep` keeps.
pub fn counted_where(
    genes: &[GeneView],
    switches: &[Switch],
    keep: impl Fn(usize) -> bool,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for (index, (gene, switch)) in genes.iter().zip(switches).enumerate() {
        if !keep(index) {
            continue;
        }
        if let Some((name, _)) = gene.named() {
            *counts
                .entry(format!("{name}{}", when(*switch)))
                .or_insert(0) += 1;
        }
    }
    counts
}

/// When a gene works, after what it does: nothing for all its life.
pub fn when(switch: Switch) -> String {
    let mut text = String::new();
    if switch.from != LifeStage::Young {
        text += &format!(" from {:?}", switch.from);
    }
    if let Some(until) = switch.until {
        text += &format!(" until {until:?}");
    }
    text
}
