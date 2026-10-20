//! Spawn variation (design §4.9): small random changes to a genome's gene
//! values, never to what a gene refers to or how it works.

use rand_chacha::ChaCha8Rng;

use crate::data::DataPack;
use crate::genome::{Gene, Genome};
use crate::random::unit;
use crate::registry::Trait;

/// `genome` with each gene value multiplied by a factor drawn uniformly
/// within physiology's `spawn_variation` of 1, or a trait's own share from
/// `trait_variation` (M2 design §9.3), then clamped to its range and, for a
/// whole number, rounded. Values are drawn in genome order, one draw
/// each. Structural fields, such as what a gene refers to, never change, so a
/// valid gene stays valid and an expressed one stays expressed.
pub(crate) fn varied(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Genome {
    let physiology = data.physiology();
    let spread = physiology.spawn_variation;
    let genes = genome
        .genes
        .iter()
        .map(|gene| {
            each_value(gene, data, |value, which| {
                let spread = which
                    .and_then(|which| physiology.trait_variation.get(&which))
                    .copied()
                    .unwrap_or(spread);
                value * (1.0 - spread + 2.0 * spread * unit(rng))
            })
        })
        .collect();
    // Only values vary: when each gene works is structural (M2 design §9.2).
    Genome {
        genes,
        switches: genome.switches.clone(),
    }
}

/// `gene` with each of its values (design §4.3) put through `change`, in
/// the order of its fields, then held within its range and, for a whole
/// number, rounded. `change` is told which trait a value is, if it's a
/// trait's. Structural fields, such as what a gene refers to, never change,
/// so a valid gene stays valid and an expressed one stays expressed; nor
/// does an unknown or unmatched gene, which has no values this build reads.
pub(crate) fn each_value(
    gene: &Gene,
    data: &DataPack,
    mut change: impl FnMut(f32, Option<Trait>) -> f32,
) -> Gene {
    let physiology = data.physiology();
    let mut vary = |value: f32| change(value, None);
    let level = |value: f32| value.clamp(0.0, 1.0);
    let mut gene = gene.clone();
    match gene {
        Gene::HalfLife { ref mut ticks, .. } => {
            *ticks = (vary(*ticks as f32).round() as u32).max(1);
        }
        Gene::Reaction { ref mut rate, .. } => *rate = level(vary(*rate)),
        Gene::Emitter {
            ref mut threshold,
            ref mut gain,
            ..
        }
        | Gene::Receptor {
            ref mut threshold,
            ref mut gain,
            ..
        } => {
            *threshold = vary(*threshold).max(0.0);
            *gain = vary(*gain);
        }
        Gene::InitialConcentration { ref mut value, .. } => *value = level(vary(*value)),
        Gene::Trait {
            which,
            ref mut value,
        } => {
            let (low, high) = physiology.traits.range(which);
            *value = change(*value, Some(which)).clamp(low, high);
        }
        Gene::BrainParam {
            param,
            ref mut value,
        } => {
            let (low, high) = physiology.brain.of(param).range;
            let varied = vary(*value).clamp(low, high);
            *value = if param.is_whole() {
                varied.round()
            } else {
                varied
            };
        }
        Gene::Instinct { ref mut weight, .. } | Gene::AttentionInstinct { ref mut weight, .. } => {
            *weight = vary(*weight)
        }
        Gene::Unknown { .. } | Gene::Unmatched(_) => {}
    }
    gene
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::ageing::LifeStage;
    use crate::expression::expressions;
    use crate::genome::Gene;
    use crate::random::unit;
    use crate::registry::{BrainParam, Trait};
    use crate::test_util::{builtin, every_kind};

    /// Whether `varied` is `original` times a factor within ±10%.
    fn within_ten_percent(original: f32, varied: f32) -> bool {
        let (low, high) = (original * 0.9, original * 1.1);
        (low.min(high)..=low.max(high)).contains(&varied)
    }

    /// Every value of `genome`'s genes (design §4.3), in genome order.
    fn values(genome: &Genome, data: &DataPack) -> Vec<f32> {
        let mut values = Vec::new();
        for gene in &genome.genes {
            each_value(gene, data, |value, _| {
                values.push(value);
                value
            });
        }
        values
    }

    /// `genome`'s genes with every value replaced by the same stand-in, so
    /// two genomes compare equal when only their values differ.
    fn shape(genome: &Genome, data: &DataPack) -> Vec<Gene> {
        let stand_in = |gene: &Gene| each_value(gene, data, |_, _| 0.0);
        genome.genes.iter().map(stand_in).collect()
    }

    #[test]
    fn spawn_variation_changes_only_gene_values_each_within_ten_percent() {
        let data = builtin();
        let original = every_kind(&data);
        for seed in 0..50 {
            let varied = varied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            assert_eq!(shape(&varied, &data), shape(&original, &data));
            let (before, after) = (values(&original, &data), values(&varied, &data));
            for (before, after) in before.into_iter().zip(after) {
                assert!(within_ten_percent(before, after), "{before} became {after}");
            }
        }
    }

    #[test]
    fn a_trait_with_its_own_variation_varies_by_that_instead() {
        let data = builtin();
        let text = r#"(format: 1, genes: [
            Trait(trait: "speed", value: 7.0),
            Trait(trait: "grows_old", value: 20000.0),
        ])"#;
        let original = Genome::from_ron(text, &data).expect("a valid genome");
        let (mut least, mut most) = (f32::MAX, f32::MIN);
        for seed in 0..200 {
            let varied = varied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            for gene in &varied.genes {
                match *gene {
                    Gene::Trait {
                        which: Trait::Speed,
                        value,
                    } => assert!(within_ten_percent(7.0, value), "{value}"),
                    Gene::Trait {
                        which: Trait::GrowsOld,
                        value,
                    } => {
                        // The built-in pack's `trait_variation` gives
                        // `grows_old` 40%.
                        assert!((12000.0..=28000.0).contains(&value), "{value}");
                        least = least.min(value);
                        most = most.max(value);
                    }
                    _ => unreachable!("only traits"),
                }
            }
        }
        assert!(least < 15000.0 && most > 25000.0, "{least}–{most}");
    }

    #[test]
    fn varied_values_are_clamped_to_their_ranges() {
        let data = builtin();
        let text = r#"(format: 1, genes: [
            HalfLife(chem: "pain", ticks: 1),
            Emitter(locus: Locus("ate"), mode: Level, threshold: 1.0, gain: 1.0, chem: "hunger"),
            InitialConcentration(chem: "boredom", value: 1.0),
            Trait(trait: "speed", value: 12.0),
        ])"#;
        let original = Genome::from_ron(text, &data).expect("a valid genome");
        for seed in 0..50 {
            let varied = varied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            for gene in &varied.genes {
                match *gene {
                    Gene::HalfLife { ticks, .. } => assert_eq!(ticks, 1, "at least 1 tick"),
                    Gene::Emitter { threshold, .. } => assert!(threshold >= 0.0),
                    Gene::InitialConcentration { value, .. } => assert!(value <= 1.0),
                    Gene::Trait { value, .. } => assert!(value <= 12.0, "speed's range is 4–12"),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn varied_brain_parameters_stay_in_range_and_whole_numbers_stay_whole() {
        let data = builtin();
        let text = r#"(format: 1, genes: [
            BrainParam(param: "tau_base", value: 2.0),
            BrainParam(param: "pool_size", value: 30.0),
            BrainParam(param: "max_arity", value: 3.0),
            BrainParam(param: "forget_ticks", value: 5000.0),
        ])"#;
        let original = Genome::from_ron(text, &data).expect("a valid genome");
        for seed in 0..50 {
            let varied = varied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            for gene in &varied.genes {
                let Gene::BrainParam { param, value } = *gene else {
                    unreachable!("only brain parameters");
                };
                let (low, high) = data.physiology().brain.of(param).range;
                assert!((low..=high).contains(&value), "{param:?} {value}");
                if param != BrainParam::TauBase {
                    assert_eq!(value, value.round(), "{param:?} is a whole number");
                }
            }
        }
    }

    #[test]
    fn spawn_variation_takes_one_draw_per_gene_value_in_genome_order() {
        let data = builtin();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let mut expected = rng.clone();
        varied(&every_kind(&data), &data, &mut rng);
        // Values: ticks, rate, threshold and gain, threshold and gain, value,
        // value, and the brain genes' value, weight and weight.
        for _ in 0..11 {
            unit(&mut expected);
        }
        assert_eq!(rng, expected);
    }

    proptest! {
        #[test]
        fn a_varied_genome_has_no_flagged_genes(seed: u64) {
            let data = builtin();
            for genome in [data.starter().clone(), every_kind(&data)] {
                let before = expressions(&genome, &data, LifeStage::Adult);
                let varied = varied(&genome, &data, &mut ChaCha8Rng::seed_from_u64(seed));
                prop_assert_eq!(expressions(&varied, &data, LifeStage::Adult), before);
            }
        }
    }
}
