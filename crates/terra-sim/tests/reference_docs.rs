//! The data-format reference's working examples (design §7.4, A9): every
//! example in `docs/reference/` loads in the format its page documents.
//!
//! A page marks an example with an `<!-- example: name -->` line just
//! before its fenced block, so the reference can't drift from the formats.

mod common;
use common::builtin;

use terra_sim::{DataError, DataPack, Genome, LabScenario, WorldConfig};

/// The example `name` from a reference page: the fenced block after its
/// marker, without the fences.
fn example(page: &str, name: &str) -> String {
    let marker = format!("<!-- example: {name} -->");
    let after = page
        .split_once(&marker)
        .unwrap_or_else(|| panic!("no example `{name}` in the page"))
        .1;
    let block = after
        .trim_start()
        .strip_prefix("```ron\n")
        .unwrap_or_else(|| panic!("example `{name}` isn't a ```ron block"));
    let end = block
        .find("\n```")
        .unwrap_or_else(|| panic!("example `{name}` isn't closed"));
    block[..end].to_string()
}

fn builtin_source(file: &str) -> &'static str {
    DataPack::builtin_sources()
        .iter()
        .find(|&&(path, _)| path == file)
        .unwrap_or_else(|| panic!("the built-in pack has no {file}"))
        .1
}

/// Loads the built-in pack with each of `files` replaced by its text.
fn builtin_with(files: &[(&str, &str)]) -> Result<DataPack, DataError> {
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| {
            let text = files
                .iter()
                .find(|&&(file, _)| file == path)
                .map_or(builtin, |&(_, text)| text);
            (path, text)
        })
        .collect();
    DataPack::from_sources(&sources)
}

/// The built-in `file`, a list, with `entry` added at its end.
fn appended(file: &str, entry: &str) -> String {
    let text = builtin_source(file);
    let end = text.rfind(']').expect("the file is a list");
    format!("{}{entry}\n{}", &text[..end], &text[end..])
}

#[test]
fn the_pack_example_loads() {
    let page = include_str!("../../../docs/reference/pack.md");
    let pack = builtin_with(&[("pack.ron", &example(page, "pack"))]).unwrap();
    assert_eq!((pack.name(), pack.version()), ("core", "1"));
}

#[test]
fn the_terrain_example_loads() {
    let page = include_str!("../../../docs/reference/terrain.md");
    builtin_with(&[("terrain.ron", &example(page, "terrain"))]).unwrap();
}

#[test]
fn the_categories_example_loads() {
    let page = include_str!("../../../docs/reference/categories.md");
    builtin_with(&[("categories.ron", &example(page, "categories"))]).unwrap();
}

#[test]
fn the_tags_example_loads() {
    let page = include_str!("../../../docs/reference/tags.md");
    builtin_with(&[("tags.ron", &example(page, "tags"))]).unwrap();
}

#[test]
fn the_object_type_example_loads_and_can_be_placed() {
    let page = include_str!("../../../docs/reference/objects.md");
    let objects = appended("objects.ron", &example(page, "objects"));
    let pack = builtin_with(&[("objects.ron", &objects)]).unwrap();
    assert_eq!(pack.category_of("mushroom"), Some("fruit"));
    assert!(pack.placeable().any(|(name, _)| name == "mushroom"));
}

#[test]
fn the_chemical_and_brain_input_examples_load_together() {
    let chemicals = include_str!("../../../docs/reference/chemicals.md");
    let brain_io = include_str!("../../../docs/reference/brain-io.md");
    let chemicals = appended("chemicals.ron", &example(chemicals, "chemicals"));
    // The input goes at the end of `inputs`, and the needs replace the built-in ones.
    let builtin_io = builtin_source("brain_io.ron");
    let inputs_end = builtin_io
        .find("\n],")
        .expect("brain_io.ron has an inputs list");
    let needs_start = builtin_io
        .find("needs: [")
        .expect("brain_io.ron lists the needs");
    let needs_end = needs_start + builtin_io[needs_start..].find("],").unwrap() + 2;
    let io = format!(
        "{}\n{}{}{}{}",
        &builtin_io[..inputs_end],
        example(brain_io, "brain_io"),
        &builtin_io[inputs_end..needs_start],
        example(brain_io, "brain_io_needs"),
        &builtin_io[needs_end..],
    );
    let pack = builtin_with(&[("chemicals.ron", &chemicals), ("brain_io.ron", &io)]).unwrap();
    assert!(pack.drives().any(|drive| drive == "chill"));
    assert!(pack.needs().any(|need| need == "chill"));
}

#[test]
fn the_locus_example_loads() {
    let page = include_str!("../../../docs/reference/loci.md");
    builtin_with(&[("loci.ron", &appended("loci.ron", &example(page, "loci")))]).unwrap();
}

#[test]
fn the_names_example_loads() {
    let page = include_str!("../../../docs/reference/names.md");
    builtin_with(&[("names.ron", &example(page, "names"))]).unwrap();
}

#[test]
fn the_genome_example_loads_and_keeps_its_unknown_gene() {
    let page = include_str!("../../../docs/reference/genomes.md");
    let data = builtin();
    let genome = Genome::from_ron(&example(page, "genome"), &data).unwrap();
    assert!(genome.to_ron(&data).contains("Gene(type: 900"));
}

#[test]
fn the_preset_example_loads() {
    let page = include_str!("../../../docs/reference/presets.md");
    let data = builtin();
    let config = WorldConfig::from_ron(&example(page, "preset"), &data).unwrap();
    assert_eq!((config.width(), config.height()), (96, 64));
}

#[test]
fn the_lab_scenario_example_loads() {
    let page = include_str!("../../../docs/reference/lab-scenarios.md");
    LabScenario::from_ron(&example(page, "scenario"), &builtin()).unwrap();
}
