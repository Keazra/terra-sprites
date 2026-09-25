//! Genomes and the genome file format (design §2.8, §4.3).

use terra_sim::{DataPack, Genome, GenomeError};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A genome file holding `genes`, one per line, laid out as exporting writes it.
fn genome_file(genes: &[&str]) -> String {
    let mut text = String::from("(\n    format: 1,\n    genes: [\n");
    for gene in genes {
        text += &format!("        {gene},\n");
    }
    text + "    ],\n)\n"
}

/// Asserts that a genome of `genes` is invalid, with a message mentioning `word`.
fn assert_invalid(genes: &[&str], word: &str) {
    match Genome::from_ron(&genome_file(genes), &builtin()) {
        Err(GenomeError::Invalid(message)) => {
            assert!(message.contains(word), "{message:?} should mention {word}");
        }
        other => panic!("expected {genes:?} to be invalid, got {other:?}"),
    }
}

#[test]
fn a_genome_file_reads_each_kind_of_gene_by_name_and_writes_it_back() {
    let text = genome_file(&[
        r#"HalfLife(chem: "pain", ticks: 30)"#,
        r#"Reaction(reactants: [("hunger", 1), ("food", 1)], products: [("food", 1)], rate: 0.1)"#,
        r#"Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.02, chem: "hunger")"#,
        r#"Emitter(locus: Locus("ate"), mode: Fall, gain: -0.6, chem: "hunger")"#,
        r#"Receptor(chem: "reward", threshold: 0.2, gain: 0.5, target: "learning_rate_mod")"#,
        r#"InitialConcentration(chem: "boredom", value: 0.2)"#,
        r#"Trait(trait: "speed", value: 7.0)"#,
    ]);
    let data = builtin();
    let genome = Genome::from_ron(&text, &data).expect("a valid genome");
    assert_eq!(genome.to_ron(&data), text);
}

#[test]
fn fields_at_their_defaults_may_be_left_out_and_are_written_without_them() {
    let data = builtin();
    let written_out = genome_file(&[
        r#"Emitter(locus: Locus("always"), mode: Level, invert: false, threshold: 0.0, gain: 0.001, chem: "boredom")"#,
        r#"Receptor(chem: "reward", threshold: 0.0, gain: 0.5, target: "learning_rate_mod")"#,
    ]);
    let left_out = genome_file(&[
        r#"Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom")"#,
        r#"Receptor(chem: "reward", gain: 0.5, target: "learning_rate_mod")"#,
    ]);
    let genome = Genome::from_ron(&written_out, &data).expect("a valid genome");
    assert_eq!(genome.to_ron(&data), left_out);
}

#[test]
fn a_gene_naming_something_the_pack_lacks_is_an_error_naming_it() {
    assert_invalid(&[r#"HalfLife(chem: "glee", ticks: 30)"#], "glee");
    assert_invalid(
        &[r#"Emitter(locus: Locus("sneezed"), mode: Level, gain: 1.0, chem: "hunger")"#],
        "sneezed",
    );
    // `ate` is a pulse, not a chemical.
    assert_invalid(
        &[r#"Emitter(locus: Chem("ate"), mode: Level, gain: 1.0, chem: "hunger")"#],
        "ate",
    );
    assert_invalid(&[r#"Trait(trait: "wings", value: 1.0)"#], "wings");
    assert_invalid(
        &[r#"Receptor(chem: "reward", gain: 0.5, target: "mood")"#],
        "mood",
    );
    assert_invalid(
        &[r#"Reaction(reactants: [("hunger", 1)], products: [("glee", 1)], rate: 0.1)"#],
        "glee",
    );
}

#[test]
fn gene_values_out_of_their_range_are_errors() {
    assert_invalid(&[r#"HalfLife(chem: "pain", ticks: 0)"#], "ticks");
    assert_invalid(
        &[r#"Reaction(reactants: [("hunger", 1)], products: [], rate: 1.5)"#],
        "rate",
    );
    assert_invalid(
        &[
            r#"Emitter(locus: Locus("ate"), mode: Level, threshold: -0.1, gain: 1.0, chem: "hunger")"#,
        ],
        "threshold",
    );
    assert_invalid(
        &[r#"InitialConcentration(chem: "boredom", value: 2.0)"#],
        "value",
    );
    assert_invalid(
        &[r#"Emitter(locus: Locus("ate"), mode: Level, gain: inf, chem: "hunger")"#],
        "gain",
    );
}

#[test]
fn a_reaction_has_one_or_two_reactants_and_at_most_two_products() {
    assert_invalid(
        &[r#"Reaction(reactants: [], products: [("hunger", 1)], rate: 0.1)"#],
        "reactant",
    );
    assert_invalid(
        &[r#"Reaction(reactants: [("h0", 1), ("h1", 1), ("h2", 1)], products: [], rate: 0.1)"#],
        "reactant",
    );
    assert_invalid(
        &[
            r#"Reaction(reactants: [("h0", 1)], products: [("h1", 1), ("h2", 1), ("h3", 1)], rate: 0.1)"#,
        ],
        "product",
    );
    assert_invalid(
        &[r#"Reaction(reactants: [("h0", 0)], products: [], rate: 0.1)"#],
        "coefficient",
    );
}

#[test]
fn a_gene_written_by_number_reads_the_same_as_by_name() {
    // HalfLife is gene type 1. Its version 1 payload is MessagePack
    // [chemical ID, ticks]: 0x92 (an array of two), 0x12 (pain is chemical 18), 0x1e (30).
    let data = builtin();
    let by_number = genome_file(&[r#"Gene(type: 1, version: 1, payload: "92121e")"#]);
    let by_name = genome_file(&[r#"HalfLife(chem: "pain", ticks: 30)"#]);
    let genome = Genome::from_ron(&by_number, &data).expect("a valid genome");
    assert_eq!(
        genome,
        Genome::from_ron(&by_name, &data).expect("a valid genome")
    );
    assert_eq!(
        genome.to_ron(&data),
        by_name,
        "a known gene is written by name"
    );
}

#[test]
fn an_unknown_gene_is_kept_and_written_back_by_number() {
    let data = builtin();
    let text = genome_file(&[
        r#"Gene(type: 42, version: 1, payload: "c0ffee")"#,
        // A HalfLife from a build whose payload version this one doesn't know yet.
        r#"Gene(type: 1, version: 9, payload: "0badf00d")"#,
        r#"HalfLife(chem: "pain", ticks: 30)"#,
    ]);
    let genome = Genome::from_ron(&text, &data).expect("a valid genome");
    assert_eq!(genome.to_ron(&data), text);
}

#[test]
fn a_gene_written_by_number_must_be_readable_if_its_type_is_known() {
    // Chemical 99 isn't in the pack.
    assert_invalid(&[r#"Gene(type: 1, version: 1, payload: "92631e")"#], "99");
    // One element short.
    assert_invalid(
        &[r#"Gene(type: 1, version: 1, payload: "9112")"#],
        "payload",
    );
    assert_invalid(
        &[r#"Gene(type: 42, version: 1, payload: "c0ffe")"#],
        "payload",
    );
    assert_invalid(&[r#"Gene(type: 42, version: 1, payload: "zz")"#], "payload");
    assert_invalid(&[r#"Gene(type: 42, version: 0, payload: "")"#], "version");
}

#[test]
fn golden_a_genome_with_unknown_genes_round_trips_byte_for_byte() {
    // Genes of types no build defines, and a HalfLife payload version from the
    // far future. Every later build must write this file back unchanged.
    let golden = include_str!("golden/unknown-genes.ron");
    let data = builtin();
    let genome = Genome::from_ron(golden, &data).expect("the golden genome loads");
    assert_eq!(genome.to_ron(&data), golden);
}

#[test]
fn a_genome_file_newer_than_the_build_is_refused() {
    let newer = genome_file(&[]).replace("format: 1", "format: 2");
    match Genome::from_ron(&newer, &builtin()) {
        Err(GenomeError::Invalid(message)) => assert!(message.contains("format"), "{message}"),
        other => panic!("expected format 2 to be refused, got {other:?}"),
    }
    let zero = genome_file(&[]).replace("format: 1", "format: 0");
    assert!(
        Genome::from_ron(&zero, &builtin()).is_err(),
        "formats start at 1"
    );
}

#[test]
fn a_gene_type_the_build_has_never_heard_of_by_name_is_refused_naming_it() {
    let text = genome_file(&[r#"Photosynthesis(rate: 0.1)"#]);
    match Genome::from_ron(&text, &builtin()) {
        Err(GenomeError::Parse(message)) => {
            assert!(message.contains("Photosynthesis"), "{message}");
        }
        other => panic!("expected an unknown gene type to be refused, got {other:?}"),
    }
}

#[test]
fn golden_the_first_starter_genome_still_loads_with_every_gene_readable() {
    // The starter genome as slice 4 shipped it. Every later build must load it.
    let golden = include_str!("golden/starter-v1.ron");
    let data = builtin();
    let genome = Genome::from_ron(golden, &data).expect("the golden starter genome loads");
    assert!(
        !genome.to_ron(&data).contains("Gene("),
        "every gene is known, so none is written by number"
    );
}
