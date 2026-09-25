//! Which genes are expressed (design §4.3): a gene that would change the body
//! directly is flagged, one that sets a value an earlier gene already set is
//! unexpressed, and an unknown gene has no effect.

use std::collections::BTreeSet;

use crate::data::DataPack;
use crate::genome::{Gene, Genome, Term};
use crate::registry::{ChemicalClass, LocusKind, Trait};

/// How a gene is expressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expression {
    /// It has its effect.
    Expressed,
    /// It breaks the restrictions on what genes may change, so it has no
    /// effect. The reason says which.
    Flagged(String),
    /// An earlier gene sets the same value, so it has no effect.
    Unexpressed,
    /// This build can't read it.
    Unknown,
}

/// The one value a gene sets, for the genes that set one. The others add up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Setting {
    HalfLife(u16),
    InitialConcentration(u16),
    Trait(Trait),
}

impl Setting {
    fn of(gene: &Gene) -> Option<Setting> {
        match *gene {
            Gene::HalfLife { chem, .. } => Some(Setting::HalfLife(chem)),
            Gene::InitialConcentration { chem, .. } => Some(Setting::InitialConcentration(chem)),
            Gene::Trait { which, .. } => Some(Setting::Trait(which)),
            Gene::Reaction { .. }
            | Gene::Emitter { .. }
            | Gene::Receptor { .. }
            | Gene::Unknown { .. } => None,
        }
    }
}

/// How each of `genome`'s genes is expressed, in genome order.
pub(crate) fn expressions(genome: &Genome, data: &DataPack) -> Vec<Expression> {
    let mut settings = BTreeSet::new();
    genome
        .genes
        .iter()
        .map(|gene| {
            if let Gene::Unknown { .. } = gene {
                return Expression::Unknown;
            }
            if let Some(reason) = breaks_restrictions(gene, data) {
                return Expression::Flagged(reason);
            }
            match Setting::of(gene) {
                Some(setting) if !settings.insert(setting) => Expression::Unexpressed,
                _ => Expression::Expressed,
            }
        })
        .collect()
}

/// Why `gene` breaks the restrictions (design §4.3), if it does. Genes may
/// read physical chemicals, but never change them.
fn breaks_restrictions(gene: &Gene, data: &DataPack) -> Option<String> {
    let chemical = |id: u16| data.chemical(id).expect("a checked gene");
    let physical = |id: u16| chemical(id).class == ChemicalClass::Physical;
    match *gene {
        Gene::HalfLife { chem, .. } if physical(chem) => Some(format!(
            "sets how fast {} decays, but a physical chemical's decay is fixed",
            chemical(chem).name
        )),
        Gene::Emitter { chem, .. } if physical(chem) => Some(format!(
            "writes {}, but only physiology and verbs change a physical chemical",
            chemical(chem).name
        )),
        Gene::InitialConcentration { chem, .. } if physical(chem) => Some(format!(
            "sets {} at birth, but a newborn's physical levels are fixed",
            chemical(chem).name
        )),
        Gene::Reaction {
            ref reactants,
            ref products,
            ..
        } => {
            let amount = |terms: &[Term], chem: u16| -> u32 {
                terms
                    .iter()
                    .filter(|t| t.chem == chem)
                    .map(|t| u32::from(t.coefficient))
                    .sum()
            };
            reactants
                .iter()
                .chain(products)
                .find(|t| physical(t.chem) && amount(reactants, t.chem) != amount(products, t.chem))
                .map(|t| {
                    format!(
                        "changes {}, but a reaction may use a physical chemical only as a catalyst, the same on both sides",
                        chemical(t.chem).name
                    )
                })
        }
        Gene::Receptor { target, .. } => {
            let locus = data.locus(target).expect("a checked gene");
            (locus.kind != LocusKind::ReceptorTarget).then(|| {
                format!(
                    "writes {}, but a receptor may write only a receptor target",
                    locus.name
                )
            })
        }
        Gene::HalfLife { .. }
        | Gene::Emitter { .. }
        | Gene::InitialConcentration { .. }
        | Gene::Trait { .. }
        | Gene::Unknown { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    /// How each gene of a genome holding `genes` is expressed.
    fn expressions_of(genes: &[&str]) -> Vec<Expression> {
        let data = builtin();
        let text = format!("(format: 1, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        expressions(&genome, &data)
    }

    /// Asserts that `gene` is flagged, with a reason mentioning `word`.
    fn assert_flagged(gene: &str, word: &str) {
        match &expressions_of(&[gene])[0] {
            Expression::Flagged(reason) => {
                assert!(reason.contains(word), "{reason:?} should mention {word}");
            }
            other => panic!("expected {gene} to be flagged, got {other:?}"),
        }
    }

    fn assert_expressed(gene: &str) {
        assert_eq!(expressions_of(&[gene]), [Expression::Expressed], "{gene}");
    }

    #[test]
    fn decay_rates_of_physical_chemicals_are_fixed() {
        assert_flagged(r#"HalfLife(chem: "energy", ticks: 10)"#, "energy");
        assert_expressed(r#"HalfLife(chem: "pain", ticks: 10)"#);
        assert_expressed(r#"HalfLife(chem: "h3", ticks: 10)"#);
    }

    #[test]
    fn an_emitter_may_read_a_physical_chemical_but_not_write_one() {
        assert_flagged(
            r#"Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food")"#,
            "food",
        );
        assert_expressed(
            r#"Emitter(locus: Chem("energy"), mode: Level, invert: true, gain: 0.02, chem: "hunger")"#,
        );
    }

    #[test]
    fn newborn_physical_levels_are_fixed() {
        assert_flagged(
            r#"InitialConcentration(chem: "injury", value: 0.5)"#,
            "injury",
        );
        assert_expressed(r#"InitialConcentration(chem: "boredom", value: 0.2)"#);
    }

    #[test]
    fn a_reaction_may_use_a_physical_chemical_only_as_a_catalyst() {
        assert_expressed(
            r#"Reaction(reactants: [("hunger", 1), ("food", 1)], products: [("food", 1)], rate: 0.1)"#,
        );
        assert_flagged(
            r#"Reaction(reactants: [("food", 1)], products: [("hunger", 1)], rate: 0.1)"#,
            "food",
        );
        assert_flagged(
            r#"Reaction(reactants: [("hunger", 1)], products: [("energy", 1)], rate: 0.1)"#,
            "energy",
        );
        // The same chemical on both sides, but not in the same amount.
        assert_flagged(
            r#"Reaction(reactants: [("food", 1)], products: [("food", 2)], rate: 0.1)"#,
            "food",
        );
        assert_expressed(
            r#"Reaction(reactants: [("h0", 2)], products: [("h1", 1), ("h2", 1)], rate: 0.1)"#,
        );
    }

    #[test]
    fn a_receptor_reads_any_chemical_but_writes_only_a_receptor_target() {
        assert_expressed(
            r#"Receptor(chem: "energy", threshold: 0.2, gain: 0.5, target: "learning_rate_mod")"#,
        );
        assert_flagged(
            r#"Receptor(chem: "reward", gain: 0.5, target: "ate")"#,
            "ate",
        );
    }

    #[test]
    fn only_the_first_gene_to_set_a_value_is_expressed() {
        use Expression::{Expressed, Unexpressed};
        assert_eq!(
            expressions_of(&[
                r#"HalfLife(chem: "pain", ticks: 10)"#,
                r#"HalfLife(chem: "pain", ticks: 20)"#,
                r#"HalfLife(chem: "thirst", ticks: 20)"#,
                r#"InitialConcentration(chem: "boredom", value: 0.2)"#,
                r#"InitialConcentration(chem: "boredom", value: 0.3)"#,
                r#"Trait(trait: "speed", value: 7.0)"#,
                r#"Trait(trait: "lifespan", value: 60000.0)"#,
                r#"Trait(trait: "speed", value: 9.0)"#,
            ]),
            [
                Expressed,
                Unexpressed,
                Expressed,
                Expressed,
                Unexpressed,
                Expressed,
                Expressed,
                Unexpressed,
            ]
        );
    }

    #[test]
    fn genes_that_add_up_all_apply() {
        let emitter =
            r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom")"#;
        let reaction = r#"Reaction(reactants: [("h0", 1)], products: [("h1", 1)], rate: 0.1)"#;
        let receptor = r#"Receptor(chem: "reward", gain: 0.5, target: "learning_rate_mod")"#;
        assert_eq!(
            expressions_of(&[emitter, emitter, reaction, reaction, receptor, receptor]),
            vec![Expression::Expressed; 6]
        );
    }

    #[test]
    fn traits_are_expressed_and_unknown_genes_are_not() {
        assert_eq!(
            expressions_of(&[
                r#"Trait(trait: "speed", value: 7.0)"#,
                r#"Gene(type: 900, version: 1, payload: "")"#,
            ]),
            [Expression::Expressed, Expression::Unknown]
        );
    }
}
