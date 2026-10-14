//! Gene values named for what their genes do and the fields they fill (M2
//! design §12.7, §16.4), and the middle of each among the living, which is
//! how a species is seen to drift from the starter genome.

mod common;

use common::{WorldBuilder, at, builtin, traits_only};
use terra_sim::{Genome, gene_values, middle_values};

#[test]
fn each_number_is_named_for_what_its_gene_does_and_its_field() {
    let data = builtin();
    let text = r#"(format: 2, genes: [
        Trait(trait: "speed", value: 12.0),
        Instinct(inputs: [("hunger", false), ("ate", true)], verb: Eat, weight: 0.5),
        Receptor(chem: "boredom", threshold: 0.25, gain: 2.0, target: "exploration_mod"),
        BrainParam(param: "habit_fade", value: 0.01),
    ])"#;
    let genome = Genome::from_ron(text, &data).expect("a valid genome");
    let values = gene_values(&genome.genes(&data));
    let named: Vec<(&str, f32)> = values.iter().map(|(name, n)| (name.as_str(), *n)).collect();
    assert_eq!(
        named,
        [
            ("Speed: trait", 12.0),
            ("hunger & not ate → Eat: weight", 0.5),
            ("boredom → exploration_mod: threshold", 0.25),
            ("boredom → exploration_mod: gain", 2.0),
            ("habit_fade: brain", 0.01),
        ]
    );
}

#[test]
fn the_starter_genome_has_values_to_follow() {
    let data = builtin();
    let starter = gene_values(&data.starter().genes(&data));
    assert!(starter.len() > 50, "{} values", starter.len());
}

#[test]
fn each_number_s_middle_among_the_living() {
    let data = builtin();
    let sprites = [10.0, 30.0, 20.0]
        .into_iter()
        .enumerate()
        .map(|(x, speed)| {
            (
                at(x as u16 * 2 + 1, 1),
                Some(traits_only(speed, 14.0, &data)),
            )
        })
        .collect();
    let world = WorldBuilder::new(&["........"; 3])
        .sprites_with(sprites)
        .build();
    let values = [
        ("Speed: trait".to_string(), 10.0),
        ("SenseRadius: trait".to_string(), 10.0),
        ("GrowsOld: trait".to_string(), 1.0),
    ];
    assert_eq!(
        middle_values(&world, &values),
        Some(vec![Some(20.0), Some(14.0), None]),
        "a number no sprite has has no middle"
    );
}

#[test]
fn with_none_alive_there_are_no_middles() {
    let world = WorldBuilder::new(&["........"; 3]).build();
    assert_eq!(middle_values(&world, &[("Speed: trait".into(), 1.0)]), None);
}
