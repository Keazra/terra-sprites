//! Copying errors (M2 design §12): when an egg is laid, its genes are copied
//! with now and then a mistake in a gene value, never in what a gene refers
//! to or how it works.

use rand_chacha::ChaCha8Rng;

use crate::data::DataPack;
use crate::genome::Genome;
use crate::random::{chance, unit};
use crate::variation::each_value;

/// `genome` as copied into an egg (M2 design §12): each gene value has
/// physiology's `mutation.chance` of changing, by a small factor or, now and
/// then, a large one, then is held within its range as spawn variation's
/// are. Values are drawn in genome order. Only values change, so a flagged
/// gene stays flagged and no copying error can make a gene change the body
/// directly. A pack without copying errors copies a genome exactly, drawing
/// nothing.
pub(crate) fn copied(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Genome {
    let mutation = data.physiology().mutation;
    if mutation.chance == 0.0 {
        return genome.clone();
    }
    let (small, large) = (mutation.small, mutation.large);
    let genes = genome
        .genes
        .iter()
        .map(|gene| {
            each_value(gene, data, |value, _| {
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
            })
        })
        .collect();
    // When each gene works is structural (M2 design §9.2).
    Genome {
        genes,
        switches: genome.switches.clone(),
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use proptest::prelude::*;

    use super::*;
    use crate::ageing::LifeStage;
    use crate::expression::expressions;
    use crate::genome::Gene;
    use crate::test_util::builtin_changing;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

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
    fn about_one_gene_value_in_25_changes_by_at_most_half_or_double() {
        // M2 design §12: the built-in pack's chance is 0.04, and the
        // largest change halves or doubles a value.
        let data = builtin();
        let original = every_kind(&data);
        let (mut changed, mut all) = (0, 0);
        for seed in 0..1_000 {
            let copy = copied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
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
        assert!((0.03..0.05).contains(&share), "{changed} of {all} changed");
    }

    #[test]
    fn most_changes_are_small_and_about_one_in_ten_large() {
        // M2 design §12: a small change is within a fifth either way; a
        // tenth of the changes are large, from half to double, as likely
        // one as the other, so most of those fall outside a fifth.
        let data = builtin();
        let original = every_kind(&data);
        let (mut changes, mut outside, mut up) = (0, 0, 0);
        for seed in 0..5_000 {
            let copy = copied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
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
        // 7% of all changes do.
        let share = outside as f32 / changes as f32;
        assert!((0.05..0.09).contains(&share), "{outside} of {changes}");
        let up = up as f32 / outside as f32;
        assert!((0.4..0.6).contains(&up), "{up} of the large ones went up");
    }

    #[test]
    fn a_pack_without_copying_errors_copies_exactly_and_draws_nothing() {
        // A save from before copying errors keeps its pack's rules (M2
        // design §12).
        let data = builtin_changing(
            "physiology.ron",
            &[(
                "    mutation: (chance: 0.04, small: 0.2, large: (share: 0.1, most: 2.0)),\n",
                "",
            )],
        );
        let original = every_kind(&data);
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let before = rng.clone();
        assert_eq!(copied(&original, &data, &mut rng), original);
        assert_eq!(rng, before);
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

    proptest! {
        #[test]
        fn copying_errors_never_change_how_a_gene_is_expressed(seed: u64) {
            // M2 design §12: a flagged gene stays flagged, and no copying
            // error makes a gene change the body directly, at any stage.
            let data = builtin();
            let flagged = Genome::from_ron(FLAGGED, &data).expect("a valid genome");
            for genome in [data.starter().clone(), every_kind(&data), flagged] {
                let copy = copied(&genome, &data, &mut ChaCha8Rng::seed_from_u64(seed));
                for stage in [LifeStage::Young, LifeStage::Adult, LifeStage::Old] {
                    prop_assert_eq!(
                        expressions(&copy, &data, stage),
                        expressions(&genome, &data, stage)
                    );
                }
            }
        }
    }
}
