//! Spawn variation (design §4.9): small random changes to a genome's gene
//! values, never to what a gene refers to or how it works.

use rand_chacha::ChaCha8Rng;

use crate::data::DataPack;
use crate::genome::{Gene, Genome};
use crate::random::unit;

/// `genome` with each gene value multiplied by a factor drawn uniformly
/// within physiology's `spawn_variation` of 1, then clamped to its range and,
/// for a whole number, rounded. Values are drawn in genome order, one draw
/// each. Structural fields, such as what a gene refers to, never change, so a
/// valid gene stays valid and an expressed one stays expressed.
pub(crate) fn varied(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Genome {
    let physiology = data.physiology();
    let spread = physiology.spawn_variation;
    let mut vary = |value: f32| value * (1.0 - spread + 2.0 * spread * unit(rng));
    let level = |value: f32| value.clamp(0.0, 1.0);
    let genes = genome
        .genes
        .iter()
        .map(|gene| {
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
                    *value = vary(*value).clamp(low, high);
                }
                Gene::Unknown { .. } => {}
            }
            gene
        })
        .collect();
    Genome { genes }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::expression::expressions;
    use crate::genome::{Gene, LocusRef, Mode};
    use crate::random::unit;
    use crate::registry::Trait;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    /// One gene of every type the build knows, and an unknown one.
    const EVERY_KIND: &str = r#"(format: 1, genes: [
        HalfLife(chem: "pain", ticks: 30),
        Reaction(reactants: [("hunger", 1), ("food", 1)], products: [("food", 1)], rate: 0.5),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: -0.02, chem: "hunger"),
        Receptor(chem: "reward", threshold: 0.2, gain: 0.5, target: "learning_rate_mod"),
        InitialConcentration(chem: "boredom", value: 0.2),
        Trait(trait: "speed", value: 7.0),
        Gene(type: 900, version: 1, payload: "c0ffee"),
    ])"#;

    fn every_kind(data: &DataPack) -> Genome {
        Genome::from_ron(EVERY_KIND, data).expect("a valid genome")
    }

    /// Whether `varied` is `original` times a factor within ±10%.
    fn within_ten_percent(original: f32, varied: f32) -> bool {
        let (low, high) = (original * 0.9, original * 1.1);
        (low.min(high)..=low.max(high)).contains(&varied)
    }

    #[test]
    fn spawn_variation_changes_only_gene_values_each_within_ten_percent() {
        let data = builtin();
        let original = every_kind(&data);
        for seed in 0..50 {
            let varied = varied(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            let pairs = original.genes.iter().zip(&varied.genes);
            for (before, after) in pairs {
                match (before, after) {
                    (
                        Gene::HalfLife {
                            chem: 18,
                            ticks: 30,
                        },
                        &Gene::HalfLife { chem: 18, ticks },
                    ) => {
                        assert!((27..=33).contains(&ticks), "{ticks}");
                    }
                    (
                        Gene::Reaction {
                            reactants,
                            products,
                            ..
                        },
                        Gene::Reaction {
                            reactants: r,
                            products: p,
                            rate,
                        },
                    ) => {
                        assert_eq!((reactants, products), (r, p));
                        assert!(within_ten_percent(0.5, *rate), "{rate}");
                    }
                    (
                        Gene::Emitter {
                            locus: LocusRef::Chem(1),
                            mode: Mode::Level,
                            invert: true,
                            chem: 16,
                            ..
                        },
                        &Gene::Emitter {
                            locus: LocusRef::Chem(1),
                            mode: Mode::Level,
                            invert: true,
                            threshold,
                            gain,
                            chem: 16,
                        },
                    ) => {
                        assert!(within_ten_percent(0.5, threshold), "{threshold}");
                        assert!(within_ten_percent(-0.02, gain), "{gain}");
                    }
                    (
                        Gene::Receptor {
                            chem: 23,
                            target: 64,
                            ..
                        },
                        &Gene::Receptor {
                            chem: 23,
                            threshold,
                            gain,
                            target: 64,
                        },
                    ) => {
                        assert!(within_ten_percent(0.2, threshold), "{threshold}");
                        assert!(within_ten_percent(0.5, gain), "{gain}");
                    }
                    (
                        Gene::InitialConcentration { chem: 20, .. },
                        &Gene::InitialConcentration { chem: 20, value },
                    ) => {
                        assert!(within_ten_percent(0.2, value), "{value}");
                    }
                    (
                        Gene::Trait {
                            which: Trait::Speed,
                            ..
                        },
                        &Gene::Trait {
                            which: Trait::Speed,
                            value,
                        },
                    ) => {
                        assert!(within_ten_percent(7.0, value), "{value}");
                    }
                    (Gene::Unknown { .. }, _) => assert_eq!(before, after),
                    _ => panic!("{before:?} became {after:?}"),
                }
            }
        }
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
    fn spawn_variation_takes_one_draw_per_gene_value_in_genome_order() {
        let data = builtin();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let mut expected = rng.clone();
        varied(&every_kind(&data), &data, &mut rng);
        // Values: ticks, rate, threshold and gain, threshold and gain, value, value.
        for _ in 0..8 {
            unit(&mut expected);
        }
        assert_eq!(rng, expected);
    }

    proptest! {
        #[test]
        fn a_varied_genome_has_no_flagged_genes(seed: u64) {
            let data = builtin();
            for genome in [data.starter().clone(), every_kind(&data)] {
                let before = expressions(&genome, &data);
                let varied = varied(&genome, &data, &mut ChaCha8Rng::seed_from_u64(seed));
                prop_assert_eq!(expressions(&varied, &data), before);
            }
        }
    }
}
