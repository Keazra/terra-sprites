//! Shared helpers for terra-sim unit tests. The integration tests' helpers
//! (`tests/common`) are here too, so the unit tests build their worlds and
//! packs as those tests do (#136).

#[path = "../tests/common/mod.rs"]
mod common;

pub use common::*;

use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::genome::Genome;
use crate::save::Preset;

/// One gene of every type the build knows, and an unknown one, each value
/// well inside its range.
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

/// A genome of one gene of every type the build knows, and an unknown one,
/// each value well inside its range, read with `data`.
pub fn every_kind(data: &DataPack) -> Genome {
    Genome::from_ron(EVERY_KIND, data).expect("a valid genome")
}

/// `config` as a save or replay keeps it, broken in each way a damaged file
/// might hold it: every rule a preset file must keep, broken once.
pub fn broken_presets(config: &WorldConfig) -> Vec<Preset> {
    let breaks: [fn(&mut Preset); 13] = [
        |preset| preset.width = 0,
        |preset| preset.height = 31,
        |preset| preset.width = 1025,
        |preset| {
            preset.objects.insert("not_an_object_type".into(), 1);
        },
        |preset| preset.per_tiles = 0,
        |preset| preset.sprites = 19,
        |preset| preset.sprites = 101,
        |preset| preset.most_sprites = 0,
        |preset| {
            preset.sprites = 50;
            preset.most_sprites = 49;
        },
        |preset| preset.water = 0,
        |preset| preset.water = 61,
        |preset| preset.lake_size = 0,
        |preset| preset.lake_size = 6,
    ];
    breaks
        .into_iter()
        .map(|damage| {
            let mut preset = Preset::from(config);
            damage(&mut preset);
            preset
        })
        .collect()
}
