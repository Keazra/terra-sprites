//! Which genes are expressed (design §4.3): a gene that would change the body
//! directly is flagged, one that sets a value an earlier gene already set is
//! unexpressed, and an unknown gene has no effect.

use std::collections::BTreeSet;

use crate::ageing::{LifeStage, Switch};
use crate::brain_io::{InputId, Source};
use crate::data::DataPack;
use crate::genome::{Gene, Genome, Term};
use crate::registry::{BrainParam, CategoryId, ChemId, ChemicalClass, LocusKind, Trait, Verb};

/// How a gene is expressed (design §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    /// It has its effect.
    Expressed,
    /// It breaks the restrictions on what genes may change, so it has no
    /// effect. The reason says which.
    Flagged(String),
    /// An earlier gene sets the same value, so it has no effect.
    Unexpressed,
    /// This build can't read it.
    Unknown,
    /// It names a category this world doesn't have (design v19 §5.7), so it
    /// has no effect.
    Unmatched,
    /// It's switched off at the sprite's life stage (M2 design §9.2), so it
    /// has no effect for now.
    Dormant,
}

/// The one value a gene sets, for the genes that set one. The others add up.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Setting {
    HalfLife(ChemId),
    InitialConcentration(ChemId),
    Trait(Trait),
    BrainParam(BrainParam),
    /// A concept's signature, its inputs in ID order, and a verb.
    Instinct(Vec<(InputId, bool)>, Verb),
    AttentionInstinct(InputId, CategoryId),
}

impl Setting {
    fn of(gene: &Gene) -> Option<Setting> {
        match *gene {
            Gene::HalfLife { chem, .. } => Some(Setting::HalfLife(chem)),
            Gene::InitialConcentration { chem, .. } => Some(Setting::InitialConcentration(chem)),
            Gene::Trait { which, .. } => Some(Setting::Trait(which)),
            Gene::BrainParam { param, .. } => Some(Setting::BrainParam(param)),
            Gene::Instinct {
                ref inputs, verb, ..
            } => {
                let mut signature = inputs.clone();
                signature.sort();
                Some(Setting::Instinct(signature, verb))
            }
            Gene::AttentionInstinct {
                input, category, ..
            } => Some(Setting::AttentionInstinct(input, category)),
            Gene::Reaction { .. }
            | Gene::Emitter { .. }
            | Gene::Receptor { .. }
            | Gene::Unknown { .. }
            | Gene::Unmatched(_) => None,
        }
    }
}

/// How each of `genome`'s genes is expressed in a sprite at `stage`, in
/// genome order. A dormant gene doesn't set its value, so the next gene
/// that sets it does (M2 design §9.2).
pub(crate) fn expressions(genome: &Genome, data: &DataPack, stage: LifeStage) -> Vec<Expression> {
    let mut settings = BTreeSet::new();
    genome
        .genes
        .iter()
        .enumerate()
        .map(|(index, gene)| {
            match gene {
                Gene::Unknown { .. } => return Expression::Unknown,
                Gene::Unmatched(_) => return Expression::Unmatched,
                _ => {}
            }
            let switch = genome.switch(index);
            if let Some(reason) =
                breaks_restrictions(gene, data).or_else(|| breaks_switching(gene, switch))
            {
                return Expression::Flagged(reason);
            }
            if !switch.is_on(stage) {
                return Expression::Dormant;
            }
            match Setting::of(gene) {
                Some(setting) if settings.contains(&setting) => Expression::Unexpressed,
                Some(setting) => {
                    settings.insert(setting);
                    Expression::Expressed
                }
                _ => Expression::Expressed,
            }
        })
        .collect()
}

/// Why `gene` can't have `switch` (M2 design §9.2), if it can't: a trait
/// works all life, since traits set when a sprite grows up and grows old,
/// and a level at birth applies only at birth; and a gene must switch on
/// before it switches off.
fn breaks_switching(gene: &Gene, switch: Switch) -> Option<String> {
    if switch.is_always() {
        return None;
    }
    match gene {
        Gene::Trait { .. } => {
            Some("is a trait, which works all life, but switches with life stage".into())
        }
        Gene::InitialConcentration { .. } => Some(
            "sets a level at birth, which works only at birth, but switches with life stage".into(),
        ),
        _ if !switch.ever_on() => Some(format!(
            "switches off at {} before it switches on at {}, so it would never work",
            switch.until.map_or("", LifeStage::name),
            switch.from.name()
        )),
        _ => None,
    }
}

/// Why `gene` breaks the restrictions (design §4.3), if it does. Genes may
/// read physical chemicals, but never change them.
fn breaks_restrictions(gene: &Gene, data: &DataPack) -> Option<String> {
    let chemical = |id: ChemId| data.chemical(id).expect("a checked gene");
    let physical = |id: ChemId| chemical(id).class == ChemicalClass::Physical;
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
            let amount = |terms: &[Term], chem: ChemId| -> u32 {
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
        Gene::AttentionInstinct { input, .. } => {
            let input = data.brain_input(input).expect("a checked gene");
            (!matches!(input.source, Source::State(_))).then(|| {
                format!(
                    "reads {}, but attention feels only State inputs, not what it attends to",
                    input.name
                )
            })
        }
        Gene::HalfLife { .. }
        | Gene::Emitter { .. }
        | Gene::InitialConcentration { .. }
        | Gene::Trait { .. }
        | Gene::BrainParam { .. }
        | Gene::Instinct { .. }
        | Gene::Unknown { .. }
        | Gene::Unmatched(_) => None,
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
        let text = format!("(format: 2, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        expressions(&genome, &data, LifeStage::Adult)
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

    /// How each gene of a genome holding `genes` is expressed in a sprite at `stage`.
    fn expressions_at(genes: &[&str], stage: LifeStage) -> Vec<Expression> {
        let data = builtin();
        let text = format!("(format: 2, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        expressions(&genome, &data, stage)
    }

    #[test]
    fn a_gene_works_only_in_the_life_stages_it_is_switched_on_for() {
        use Expression::{Dormant, Expressed};
        use LifeStage::{Adult, Old, Young};
        let genes = [
            r#"Instinct(inputs: [("crowdedness", false)], verb: Hit, weight: 0.6, from: Adult)"#,
            r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom", until: Adult)"#,
            r#"Receptor(chem: "pain", gain: 1.0, target: "exploration_mod", from: Adult, until: Old)"#,
        ];
        assert_eq!(expressions_at(&genes, Young), [Dormant, Expressed, Dormant]);
        assert_eq!(
            expressions_at(&genes, Adult),
            [Expressed, Dormant, Expressed]
        );
        assert_eq!(expressions_at(&genes, Old), [Expressed, Dormant, Dormant]);
    }

    #[test]
    fn a_dormant_gene_leaves_the_value_it_sets_to_the_next_gene() {
        use Expression::{Dormant, Expressed, Unexpressed};
        let genes = [
            r#"BrainParam(param: "curiosity", value: 0.6, until: Adult)"#,
            r#"BrainParam(param: "curiosity", value: 0.3)"#,
        ];
        assert_eq!(
            expressions_at(&genes, LifeStage::Young),
            [Expressed, Unexpressed]
        );
        assert_eq!(
            expressions_at(&genes, LifeStage::Adult),
            [Dormant, Expressed]
        );
    }

    #[test]
    fn traits_and_levels_at_birth_work_all_life() {
        // A trait sets when a sprite grows up and grows old, so it can't
        // wait for a stage; a level at birth only ever applies at birth.
        assert_flagged(
            r#"Trait(trait: "speed", value: 7.0, from: Adult)"#,
            "life stage",
        );
        assert_flagged(
            r#"InitialConcentration(chem: "boredom", value: 0.2, until: Adult)"#,
            "life stage",
        );
    }

    #[test]
    fn a_gene_switched_off_before_it_switches_on_is_flagged() {
        assert_flagged(
            r#"Instinct(inputs: [("pain", false)], verb: Retreat, weight: 0.5, from: Old, until: Adult)"#,
            "never",
        );
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
    fn only_the_first_brain_gene_to_set_a_value_is_expressed() {
        use Expression::{Expressed, Unexpressed};
        assert_eq!(
            expressions_of(&[
                r#"BrainParam(param: "tau_base", value: 0.2)"#,
                r#"BrainParam(param: "tau_base", value: 0.3)"#,
                r#"BrainParam(param: "switch_margin", value: 0.3)"#,
                r#"Instinct(inputs: [("hunger", false), ("pain", false)], verb: Eat, weight: 1.0)"#,
                // The same concept, its inputs listed the other way round.
                r#"Instinct(inputs: [("pain", false), ("hunger", false)], verb: Eat, weight: 0.5)"#,
                r#"Instinct(inputs: [("hunger", false), ("pain", false)], verb: Rest, weight: 1.0)"#,
                r#"Instinct(inputs: [("hunger", false), ("pain", true)], verb: Eat, weight: 1.0)"#,
                r#"AttentionInstinct(input: "hunger", category: "fruit", weight: 0.8)"#,
                r#"AttentionInstinct(input: "hunger", category: "fruit", weight: 0.4)"#,
                r#"AttentionInstinct(input: "hunger", category: "bush", weight: 0.8)"#,
            ]),
            [
                Expressed,
                Unexpressed,
                Expressed,
                Expressed,
                Unexpressed,
                Expressed,
                Expressed,
                Expressed,
                Unexpressed,
                Expressed,
            ]
        );
    }

    #[test]
    fn attention_instincts_may_use_only_state_inputs() {
        assert_flagged(
            r#"AttentionInstinct(input: "target_distance", category: "fruit", weight: 0.5)"#,
            "target_distance",
        );
        assert_flagged(
            r#"AttentionInstinct(input: "attended_fruit", category: "fruit", weight: 0.5)"#,
            "attended_fruit",
        );
        assert_expressed(r#"AttentionInstinct(input: "always", category: "fruit", weight: 0.5)"#);
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
