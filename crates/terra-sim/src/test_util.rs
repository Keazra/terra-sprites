//! Shared helpers for terra-sim unit tests; integration tests use `tests/common`.

use crate::config::WorldConfig;
use crate::data::DataPack;
use crate::genome::Genome;
use crate::map::Pos;
use crate::objects::EntityId;
use crate::save::Preset;
use crate::world::World;

/// The built-in data pack.
pub fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// The built-in data pack, with `changes` made to `file`'s text, as
/// `tests/common`'s does.
pub fn builtin_changing(file: &str, changes: &[(&str, &str)]) -> DataPack {
    let sources: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let mut text = text.to_string();
            if path == file {
                for &(from, to) in changes {
                    assert!(text.contains(from), "{from:?} is in {file}");
                    text = text.replace(from, to);
                }
            }
            (path, text)
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    DataPack::from_sources(&borrowed).expect("the changed pack is valid")
}

/// The built-in data pack, with `file`'s text replaced by `text`.
pub fn builtin_with(file: &str, text: &str) -> DataPack {
    let sources = DataPack::builtin_sources();
    assert!(
        sources.iter().any(|&(path, _)| path == file),
        "{file} is in the pack"
    );
    let replaced: Vec<(&str, &str)> = sources
        .iter()
        .map(|&(path, builtin)| (path, if path == file { text } else { builtin }))
        .collect();
    DataPack::from_sources(&replaced).expect("the changed pack is valid")
}

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

/// A position at `(x, y)`.
pub fn at(x: u16, y: u16) -> Pos {
    Pos::new(x, y)
}

/// The ID of the sprite standing on `pos`, as `tests/common`'s does.
pub fn id_at(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
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
