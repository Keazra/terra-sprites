//! Gene numbers (M2 design §12, §16): each value a genome's genes carry,
//! named for what its gene does and the field it fills, as `hunger → Eat:
//! weight`. Following each one's middle value among the living is how a
//! species is seen to drift from the starter genome, in the generations
//! example (G4) and in fast-forward's genes table.

use std::collections::BTreeMap;

use crate::genome::GeneView;
use crate::world::World;

impl GeneView<'_> {
    /// What the gene does, named, and its numbers, each with its field's
    /// name; none for a gene this build can't read.
    pub fn named(&self) -> Option<(String, Vec<(&'static str, f32)>)> {
        let named = match self {
            GeneView::HalfLife { chem, ticks } => {
                (chem.to_string(), vec![("half-life", *ticks as f32)])
            }
            GeneView::Reaction {
                reactants,
                products,
                rate,
            } => {
                let side = |terms: &[(&str, u8)]| {
                    let names: Vec<&str> = terms.iter().map(|(chem, _)| *chem).collect();
                    names.join(" + ")
                };
                (
                    format!("{} → {}", side(reactants), side(products)),
                    vec![("rate", *rate)],
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
                let low = if *invert { "low " } else { "" };
                (
                    format!("{low}{locus} {mode:?} → {chem}"),
                    vec![("threshold", *threshold), ("gain", *gain)],
                )
            }
            GeneView::Receptor {
                chem,
                threshold,
                gain,
                target,
            } => (
                format!("{chem} → {target}"),
                vec![("threshold", *threshold), ("gain", *gain)],
            ),
            GeneView::InitialConcentration { chem, value } => {
                (chem.to_string(), vec![("at birth", *value)])
            }
            GeneView::Trait { which, value } => (format!("{which:?}"), vec![("trait", *value)]),
            GeneView::BrainParam { param, value } => (param.to_string(), vec![("brain", *value)]),
            GeneView::Instinct {
                inputs,
                verb,
                weight,
            } => {
                let inputs: Vec<String> = inputs
                    .iter()
                    .map(|(input, negated)| {
                        if *negated {
                            format!("not {input}")
                        } else {
                            input.to_string()
                        }
                    })
                    .collect();
                (
                    format!("{} → {verb:?}", inputs.join(" & ")),
                    vec![("weight", *weight)],
                )
            }
            GeneView::AttentionInstinct {
                input,
                category,
                weight,
            } => (
                format!("{input} → attends to {category}"),
                vec![("weight", *weight)],
            ),
            GeneView::Unknown { .. } => return None,
        };
        Some(named)
    }
}

/// Each number in `genes`, named `what its gene does: its field`, in order
/// (design §4.3's value fields). A gene this build can't read has none.
pub fn gene_numbers(genes: &[GeneView]) -> Vec<(String, f32)> {
    let mut numbers = Vec::new();
    for (name, fields) in genes.iter().filter_map(GeneView::named) {
        for (field, value) in fields {
            numbers.push((format!("{name}: {field}"), value));
        }
    }
    numbers
}

/// Each of `numbers`' middle value among the sprites alive in `world`,
/// eggs aside, or `None` if none is alive. A sprite's numbers line up with
/// `numbers` by name, the nth of a name with the nth of it there, wherever
/// it sits, since genes may be doubled, lost or changed (M2 design §13); a
/// number no sprite has has no middle. With an even count, the middle is
/// the higher of the two.
pub fn middle_numbers(world: &World, numbers: &[(String, f32)]) -> Option<Vec<Option<f32>>> {
    let mut columns: Vec<Vec<f32>> = vec![Vec::new(); numbers.len()];
    let mut sprites = 0;
    for sprite in world.sprites() {
        sprites += 1;
        let genes: Vec<GeneView> = sprite.genes().into_iter().map(|(gene, _)| gene).collect();
        let its = gene_numbers(&genes);
        let mut nth: BTreeMap<&str, usize> = BTreeMap::new();
        for (column, (name, _)) in columns.iter_mut().zip(numbers) {
            let nth = nth.entry(name).or_insert(0);
            let value = its.iter().filter(|(its, _)| its == name).nth(*nth);
            if let Some(&(_, value)) = value {
                column.push(value);
            }
            *nth += 1;
        }
    }
    (sprites > 0).then(|| {
        columns
            .into_iter()
            .map(|mut column| {
                column.sort_by(f32::total_cmp);
                column.get(column.len() / 2).copied()
            })
            .collect()
    })
}
