//! Gene values (M2 design §12.7, §16.4), each named for what its gene does,
//! when it works and the field it fills, as `hunger → Eat: weight` or
//! `always → Wander until Adult: weight`. Following each one's
//! middle value among the living is how a species is seen to drift from
//! the starter genome, in the long run and the generations example (G4,
//! which calls them gene numbers) and in fast-forward's genes table.

use std::collections::BTreeMap;

use crate::ageing::Switch;
use crate::genome::GeneView;
use crate::world::World;

impl GeneView<'_> {
    /// What the gene does, named, and its values, each with its field's
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
                whom,
                weight,
            } => {
                let attended = match whom.name() {
                    Some(whom) => format!("its {whom} ({category})"),
                    None => category.to_string(),
                };
                (
                    format!("{input} → attends to {attended}"),
                    vec![("weight", *weight)],
                )
            }
            GeneView::Unknown { .. } => return None,
        };
        Some(named)
    }
}

/// Which way a gene value moved from where it started, when it moved by a
/// tenth or more (M2 design G4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    Up,
    Down,
}

/// Which way a gene value moved, from `ratio`, its ratio to where it
/// started: up at 1.1 or more, down at 0.9 or less, and neither between
/// (M2 design G4). The generations run and fast-forward count it alike.
pub fn drift(ratio: f32) -> Option<Drift> {
    if ratio >= 1.1 {
        Some(Drift::Up)
    } else if ratio <= 0.9 {
        Some(Drift::Down)
    } else {
        None
    }
}

/// Each value in `genes`, named `what its gene does when: its field`, in
/// order (design §4.3's value fields), `switches` saying when each works
/// (M2 design §9.2), so a gene split by life stage counts as two. A gene
/// this build can't read has none.
pub fn gene_values(genes: &[GeneView], switches: &[Switch]) -> Vec<(String, f32)> {
    let mut values = Vec::new();
    for (gene, switch) in genes.iter().zip(switches) {
        let Some((name, fields)) = gene.named() else {
            continue;
        };
        let when = switch.when();
        for (field, value) in fields {
            values.push((format!("{name}{when}: {field}"), value));
        }
    }
    values
}

/// Each of `values`' middle among the sprites alive in `world`, eggs
/// aside, or `None` if none is alive. A sprite's values line up with
/// `values` by name, what the gene does and when it works, the nth of a
/// name with the nth of it there, wherever it sits, since genes may be
/// doubled, lost or changed (M2 design §13); a
/// value no sprite has has no middle. With an even count, the middle is
/// the higher of the two.
pub fn middle_values(world: &World, values: &[(String, f32)]) -> Option<Vec<Option<f32>>> {
    let mut columns: Vec<Vec<f32>> = vec![Vec::new(); values.len()];
    let mut sprites = 0;
    for sprite in world.sprites() {
        sprites += 1;
        let genes: Vec<GeneView> = sprite.genes().into_iter().map(|(gene, _)| gene).collect();
        let its = gene_values(&genes, &sprite.switches());
        let mut nth: BTreeMap<&str, usize> = BTreeMap::new();
        for (column, (name, _)) in columns.iter_mut().zip(values) {
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
