//! Copying errors (M2 design §12): as an egg's genes are copied, now and
//! then a gene value changes, never what a gene refers to or how it works,
//! and now and then a copy of a gene gets a hidden flaw. And the flaws the
//! sprites the world begins with carry.

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::ageing::{LifeStage, frailty};
use crate::data::DataPack;
use crate::expression::{Expression, expressions};
use crate::genome::{Gene, Genome};
use crate::inheritance::{Inherited, flaws_of, tidied};
use crate::random::{chance, unit};
use crate::variation::{each_value, varied};

/// A gene that copying changed in an egg (M2 design §12): where it is in
/// the egg's genome, and what it was as the egg inherited it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CopyingError {
    pub(crate) gene: usize,
    pub(crate) was: Gene,
}

/// An egg's genes once copied (M2 design §12).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Copied {
    pub(crate) genome: Genome,
    /// How many of its two copies of each gene are flawed, in genome order:
    /// empty when none are.
    pub(crate) flaws: Vec<u8>,
    /// The genes copying changed, in genome order.
    pub(crate) errors: Vec<CopyingError>,
}

/// The genes an egg `inherited`, as copied into it (M2 design §12). Each
/// gene value has physiology's `mutation.chance` of changing, by a large
/// factor or, now and then, a small one, then is held within its range as
/// spawn variation's are; values are drawn in genome order. Only values
/// change, so a flagged gene stays flagged and no copying error can make a
/// gene change the body directly. Then each sound copy of each gene that
/// can weaken gets a flaw with `flaws.arise`'s chance. A pack without
/// copying errors or flaws copies genes exactly, drawing nothing.
pub(crate) fn copied(inherited: Inherited, data: &DataPack, rng: &mut ChaCha8Rng) -> Copied {
    let Inherited { genome, flaws } = inherited;
    let (genome, errors) = values_copied(genome, data, rng);
    let flaws = flawed(&genome, flaws, data, rng);
    Copied {
        genome,
        flaws,
        errors,
    }
}

/// `genome` with its values copied, and the genes copying changed.
fn values_copied(
    genome: Genome,
    data: &DataPack,
    rng: &mut ChaCha8Rng,
) -> (Genome, Vec<CopyingError>) {
    let mutation = data.physiology().mutation;
    if mutation.chance == 0.0 {
        return (genome, Vec::new());
    }
    let (small, large) = (mutation.small, mutation.large);
    let mut errors = Vec::new();
    let genes = genome
        .genes
        .into_iter()
        .enumerate()
        .map(|(index, was)| {
            let gene = each_value(&was, data, |value, _| {
                if !chance(rng, mutation.chance) {
                    return value;
                }
                let factor = if chance(rng, large.share) {
                    // As likely to halve as to double.
                    libm::powf(large.most, 2.0 * unit(rng) - 1.0)
                } else {
                    1.0 - small + 2.0 * small * unit(rng)
                };
                value * factor
            });
            if gene != was {
                errors.push(CopyingError { gene: index, was });
            }
            gene
        })
        .collect();
    // When each gene works is structural (M2 design §9.2).
    let genome = Genome {
        genes,
        switches: genome.switches,
    };
    (genome, errors)
}

/// Whether each of `genome`'s genes can carry a flaw (M2 design §12.3):
/// one that can weaken, unless it's flagged, since a flagged gene does
/// nothing whatever its copies are.
fn can_be_flawed(genome: &Genome, data: &DataPack) -> Vec<bool> {
    // Whether a gene is flagged doesn't depend on the life stage.
    expressions(genome, data, LifeStage::Adult)
        .iter()
        .zip(&genome.genes)
        .map(|(expression, gene)| {
            frailty(gene) > 0.0 && !matches!(expression, Expression::Flagged(_))
        })
        .collect()
}

/// `flaws` with the new ones copying left (M2 design §12): each sound copy
/// of each gene of `genome` that can be flawed gets a flaw with
/// `flaws.arise`'s chance.
fn flawed(genome: &Genome, flaws: Vec<u8>, data: &DataPack, rng: &mut ChaCha8Rng) -> Vec<u8> {
    let arise = data.physiology().flaws.arise;
    if arise == 0.0 {
        return flaws;
    }
    let flaws = can_be_flawed(genome, data)
        .into_iter()
        .enumerate()
        .map(|(index, can)| {
            let flawed = flaws_of(&flaws, index);
            if !can {
                return flawed;
            }
            // One draw for each sound copy.
            let new = (flawed..2).filter(|_| chance(rng, arise)).count();
            flawed + new as u8
        })
        .collect();
    tidied(flaws)
}

/// A new sprite's genes from the starter genome, varied (design §4.9),
/// with the hidden flaws it carries, as wild animals do (M2 design §12.3).
pub(crate) fn wild(data: &DataPack, rng: &mut ChaCha8Rng) -> (Genome, Vec<u8>) {
    let genome = varied(data.starter(), data, rng);
    let flaws = founders(&genome, data, rng);
    (genome, flaws)
}

/// The flaws a sprite made from the starter genome carries (M2 design
/// §12), as wild animals do: each gene of `genome` that can be flawed has a
/// flaw in one copy with `flaws.founders`' chance. One sound copy is
/// enough, so they're hidden: only its young may suffer them.
fn founders(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Vec<u8> {
    let founders = data.physiology().flaws.founders;
    if founders == 0.0 {
        return Vec::new();
    }
    let flaws = can_be_flawed(genome, data)
        .into_iter()
        .map(|can| u8::from(can && chance(rng, founders)))
        .collect();
    tidied(flaws)
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use proptest::prelude::*;

    use super::*;
    use crate::ageing::LifeStage;
    use crate::expression::expressions;
    use crate::genome::Gene;
    use crate::test_util::{builtin, builtin_changing};

    /// One gene of every type the build knows, and an unknown one, each
    /// value well inside its range.
    const EVERY_KIND: &str = r#"(format: 1, genes: [
        HalfLife(chem: "pain", ticks: 300),
        Reaction(reactants: [("hunger", 1), ("food", 1)], products: [("food", 1)], rate: 0.5),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: -0.02, chem: "hunger"),
        Receptor(chem: "reward", threshold: 0.2, gain: 0.5, target: "learning_rate_mod"),
        InitialConcentration(chem: "boredom", value: 0.2),
        Trait(trait: "speed", value: 7.0),
        BrainParam(param: "tau_base", value: 0.5),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
        AttentionInstinct(input: "thirst", category: "water", weight: 0.8),
        Gene(type: 900, version: 1, payload: "c0ffee"),
    ])"#;

    fn every_kind(data: &DataPack) -> Genome {
        Genome::from_ron(EVERY_KIND, data).expect("a valid genome")
    }

    /// `genome`, inherited without flaws, as copied into an egg.
    fn copy(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Copied {
        let inherited = Inherited {
            genome: genome.clone(),
            flaws: Vec::new(),
        };
        copied(inherited, data, rng)
    }

    /// Each value of `gene`, in the order of its fields.
    fn values(gene: &Gene) -> Vec<f32> {
        match *gene {
            Gene::HalfLife { ticks, .. } => vec![ticks as f32],
            Gene::Reaction { rate, .. } => vec![rate],
            Gene::Emitter {
                threshold, gain, ..
            }
            | Gene::Receptor {
                threshold, gain, ..
            } => vec![threshold, gain],
            Gene::InitialConcentration { value, .. }
            | Gene::Trait { value, .. }
            | Gene::BrainParam { value, .. } => vec![value],
            Gene::Instinct { weight, .. } | Gene::AttentionInstinct { weight, .. } => vec![weight],
            Gene::Unknown { .. } | Gene::Unmatched(_) => vec![],
        }
    }

    #[test]
    fn about_one_gene_value_in_100_changes_by_at_most_half_or_double() {
        // M2 design §12: the built-in pack's chance is 0.01, and the
        // largest change halves or doubles a value.
        let data = builtin();
        let original = every_kind(&data);
        let (mut changed, mut all) = (0, 0);
        for seed in 0..4_000 {
            let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed)).genome;
            assert_eq!(copy.switches, original.switches);
            for (before, after) in original.genes.iter().zip(&copy.genes) {
                let (old, new) = (values(before), values(after));
                assert_eq!(old.len(), new.len());
                for (&old, &new) in old.iter().zip(&new) {
                    all += 1;
                    if old != new {
                        changed += 1;
                        let factor = new / old;
                        assert!((0.5..=2.0).contains(&factor), "{old} became {new}");
                    }
                }
            }
        }
        let share = changed as f32 / all as f32;
        assert!(
            (0.008..0.012).contains(&share),
            "{changed} of {all} changed"
        );
    }

    #[test]
    fn most_changes_are_large_and_about_one_in_four_small() {
        // M2 design §12: three changes in four are large, from half to
        // double, as likely one as the other, so most of those fall outside
        // a fifth either way; the rest are small, within a fifth.
        let data = builtin();
        let original = every_kind(&data);
        let (mut changes, mut outside, mut up) = (0, 0, 0);
        for seed in 0..20_000 {
            let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed)).genome;
            for (before, after) in original.genes.iter().zip(&copy.genes) {
                for (&old, &new) in values(before).iter().zip(&values(after)) {
                    if old == new {
                        continue;
                    }
                    changes += 1;
                    let factor = new / old;
                    outside += usize::from(!(0.8..=1.2).contains(&factor));
                    up += usize::from(factor > 1.2);
                }
            }
        }
        // A large change lands outside a fifth 71% of the time, so about
        // 53% of all changes do.
        let share = outside as f32 / changes as f32;
        assert!((0.49..0.57).contains(&share), "{outside} of {changes}");
        let up = up as f32 / outside as f32;
        assert!((0.42..0.62).contains(&up), "{up} of the large ones went up");
    }

    #[test]
    fn a_pack_without_copying_errors_or_flaws_copies_exactly_and_draws_nothing() {
        // A save from before copying errors keeps its pack's rules (M2
        // design §12): its eggs' genes are what they inherited.
        let data = builtin_changing(
            "physiology.ron",
            &[
                (
                    "    mutation: (chance: 0.01, small: 0.2, large: (share: 0.75, most: 2.0)),\n",
                    "",
                ),
                (
                    "    flaws: (arise: 0.007, founders: 0.06, health: 0.5, wear: 0.05),\n",
                    "",
                ),
            ],
        );
        let inherited = Inherited {
            genome: every_kind(&data),
            flaws: vec![0, 1, 2, 1],
        };
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let before = rng.clone();
        let copy = copied(inherited.clone(), &data, &mut rng);
        assert_eq!(
            (copy.genome, copy.flaws),
            (inherited.genome, inherited.flaws)
        );
        assert!(copy.errors.is_empty());
        assert_eq!(rng, before);
        assert!(founders(data.starter(), &data, &mut rng).is_empty());
        assert_eq!(rng, before);
    }

    #[test]
    fn an_egg_keeps_each_gene_copying_changed_as_it_inherited_it() {
        // M2 design §12: so the Genome tab can say what each was.
        let data = builtin();
        let original = every_kind(&data);
        let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(9));
        let changed: Vec<usize> = (0..original.genes.len())
            .filter(|&index| copy.genome.genes[index] != original.genes[index])
            .collect();
        let errors: Vec<usize> = copy.errors.iter().map(|error| error.gene).collect();
        assert_eq!(errors, changed);
        for error in &copy.errors {
            assert_eq!(error.was, original.genes[error.gene]);
        }
        let changes: usize = (0..2_000)
            .map(|seed| copy_errors(&original, &data, seed))
            .sum();
        assert!(changes > 0, "copying never changed a gene");
    }

    /// How many genes copying changed in an egg of `genome`, with `seed`.
    fn copy_errors(genome: &Genome, data: &DataPack, seed: u64) -> usize {
        copy(genome, data, &mut ChaCha8Rng::seed_from_u64(seed))
            .errors
            .len()
    }

    #[test]
    fn about_one_egg_in_two_gets_a_new_flaw_only_in_genes_that_can_weaken() {
        // M2 design §12: each sound copy of a gene that can weaken gets a
        // flaw with the chance `flaws.arise` (0.007), so about one egg of
        // the starter's in two has a new one.
        let data = builtin();
        let starter = data.starter();
        let mut flawed_eggs = 0;
        for seed in 0..2_000 {
            let flaws = copy(starter, &data, &mut ChaCha8Rng::seed_from_u64(seed)).flaws;
            flawed_eggs += usize::from(!flaws.is_empty());
            for (index, &flawed) in flaws.iter().enumerate() {
                assert!(flawed <= 2);
                if flawed > 0 {
                    assert!(frailty(&starter.genes[index]) > 0.0, "gene {index}");
                }
            }
        }
        assert!(
            (800..1_200).contains(&flawed_eggs),
            "{flawed_eggs} of 2,000"
        );
    }

    #[test]
    fn a_founder_carries_about_three_flaws_all_hidden() {
        // M2 design §12: each gene of the starter's that can weaken has a
        // flaw in one copy with the chance `flaws.founders` (0.06).
        let data = builtin();
        let starter = data.starter();
        let can_weaken = starter
            .genes
            .iter()
            .filter(|gene| frailty(gene) > 0.0)
            .count();
        let mut carried = 0;
        for seed in 0..1_000 {
            let flaws = founders(starter, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            for (index, &flawed) in flaws.iter().enumerate() {
                assert!(flawed <= 1, "a founder's flaw is hidden");
                if flawed > 0 {
                    assert!(frailty(&starter.genes[index]) > 0.0, "gene {index}");
                }
                carried += usize::from(flawed);
            }
        }
        let each = carried as f32 / 1_000.0;
        let expected = 0.06 * can_weaken as f32;
        assert!(
            (2.0..4.0).contains(&expected),
            "{can_weaken} genes can weaken"
        );
        assert!((each - expected).abs() < 0.3, "{each} each, not {expected}");
    }

    /// A genome with a gene flagged for each way a gene may not change the
    /// body (design §4.3), among genes that work.
    const FLAGGED: &str = r#"(format: 2, genes: [
        HalfLife(chem: "energy", ticks: 300),
        Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food"),
        Reaction(reactants: [("food", 1)], products: [("energy", 1)], rate: 0.5),
        InitialConcentration(chem: "hydration", value: 0.5),
        Trait(trait: "speed", value: 7.0, from: Adult),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "hunger"),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0, from: Adult),
    ])"#;

    /// A random gene of any type, as a genome file writes it, with random
    /// values and a random switch: some change the body directly, or switch
    /// when they can't, so they're flagged.
    fn random_gene() -> impl Strategy<Value = String> {
        let chem = || {
            prop::sample::select(vec![
                "hunger",
                "pain",
                "boredom",
                "reward",
                "energy",
                "food",
                "hydration",
            ])
        };
        let input = || prop::sample::select(vec!["always", "hunger", "thirst", "pain", "age"]);
        let gene = prop_oneof![
            (chem(), 1u32..5_000).prop_map(|(c, t)| format!(r#"HalfLife(chem: "{c}", ticks: {t}"#)),
            (chem(), chem(), 0.0f32..1.0).prop_map(|(a, b, rate)| format!(
                r#"Reaction(reactants: [("{a}", 1)], products: [("{b}", 1)], rate: {rate:?}"#
            )),
            (chem(), 0.0f32..1.0, -1.0f32..1.0, chem()).prop_map(|(from, threshold, gain, to)| {
                format!(
                    r#"Emitter(locus: Chem("{from}"), mode: Level, threshold: {threshold:?}, gain: {gain:?}, chem: "{to}""#
                )
            }),
            (chem(), 0.0f32..1.0, -1.0f32..1.0).prop_map(|(c, threshold, gain)| format!(
                r#"Receptor(chem: "{c}", threshold: {threshold:?}, gain: {gain:?}, target: "learning_rate_mod""#
            )),
            (chem(), 0.0f32..1.0)
                .prop_map(|(c, value)| format!(r#"InitialConcentration(chem: "{c}", value: {value:?}"#)),
            (2.0f32..12.0).prop_map(|value| format!(r#"Trait(trait: "speed", value: {value:?}"#)),
            (6_000.0f32..60_000.0)
                .prop_map(|value| format!(r#"Trait(trait: "grows_old", value: {value:?}"#)),
            (0.0f32..1.0)
                .prop_map(|value| format!(r#"BrainParam(param: "tau_base", value: {value:?}"#)),
            (input(), -1.0f32..1.0).prop_map(|(i, weight)| format!(
                r#"Instinct(inputs: [("{i}", false)], verb: Eat, weight: {weight:?}"#
            )),
            (input(), -1.0f32..1.0).prop_map(|(i, weight)| format!(
                r#"AttentionInstinct(input: "{i}", category: "water", weight: {weight:?}"#
            )),
        ];
        let switch = prop::sample::select(vec![
            "",
            ", from: Adult",
            ", until: Adult",
            ", from: Old, until: Adult",
        ]);
        (gene, switch).prop_map(|(gene, switch)| format!("{gene}{switch})"))
    }

    /// The built-in pack, with every gene value copied wrongly and every
    /// copy that can be flawed flawed.
    fn every_copy_wrong() -> DataPack {
        builtin_changing(
            "physiology.ron",
            &[
                ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,"),
                (
                    "flaws: (arise: 0.007, founders: 0.06,",
                    "flaws: (arise: 1.0, founders: 1.0,",
                ),
            ],
        )
    }

    proptest! {
        #[test]
        fn copying_errors_never_change_how_a_gene_is_expressed(seed: u64) {
            // M2 design §12: a flagged gene stays flagged, and no copying
            // error makes a gene change the body directly, at any stage.
            let data = builtin();
            let flagged = Genome::from_ron(FLAGGED, &data).expect("a valid genome");
            for genome in [data.starter().clone(), every_kind(&data), flagged] {
                let copy = copy(&genome, &data, &mut ChaCha8Rng::seed_from_u64(seed)).genome;
                for stage in [LifeStage::Young, LifeStage::Adult, LifeStage::Old] {
                    prop_assert_eq!(
                        expressions(&copy, &data, stage),
                        expressions(&genome, &data, stage)
                    );
                }
            }
        }

        #[test]
        fn whatever_the_genome_copying_keeps_how_each_gene_works_and_flaws_no_flagged_one(
            genes in prop::collection::vec(random_gene(), 1..12),
            seed: u64,
        ) {
            // M2 design §12.1, §12.3, with every value changed and every
            // copy flawed: a flagged gene stays flagged and does nothing,
            // whatever its copies, and no gene comes to change the body.
            let data = every_copy_wrong();
            let file = format!("(format: 2, genes: [{}])", genes.join(",\n"));
            let genome = Genome::from_ron(&file, &data).expect("a valid genome");
            let rng = &mut ChaCha8Rng::seed_from_u64(seed);
            let copy = copy(&genome, &data, rng);
            for stage in [LifeStage::Young, LifeStage::Adult, LifeStage::Old] {
                prop_assert_eq!(
                    expressions(&copy.genome, &data, stage),
                    expressions(&genome, &data, stage)
                );
            }
            let founders = founders(&genome, &data, rng);
            let expressed = expressions(&genome, &data, LifeStage::Adult);
            for (index, gene) in genome.genes.iter().enumerate() {
                let flagged = matches!(expressed[index], Expression::Flagged(_));
                let can = frailty(gene) > 0.0 && !flagged;
                prop_assert_eq!(flaws_of(&copy.flaws, index), if can { 2 } else { 0 });
                prop_assert_eq!(flaws_of(&founders, index), u8::from(can));
            }
        }
    }
}
