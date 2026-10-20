//! Gene values named for what their genes do and the fields they fill (M2
//! design §12.7, §16.4), and the middle of each among the living, which is
//! how a species is seen to drift from the starter genome.

mod common;

use common::{WorldBuilder, at, builtin, traits_only};
use terra_sim::{Drift, Genome, drift, gene_values, middle_values};

#[test]
fn each_value_is_named_for_what_its_gene_does_and_its_field() {
    let data = builtin();
    let text = r#"(format: 2, genes: [
        Trait(trait: "speed", value: 12.0),
        Instinct(inputs: [("hunger", false), ("ate", true)], verb: Eat, weight: 0.5),
        Receptor(chem: "boredom", threshold: 0.25, gain: 2.0, target: "exploration_mod"),
        BrainParam(param: "habit_fade", value: 0.01),
        Instinct(inputs: [("always", false)], verb: Wander, weight: 0.15, until: Adult),
    ])"#;
    let genome = Genome::from_ron(text, &data).expect("a valid genome");
    let values = gene_values(&genome.genes(&data), &genome.switches());
    let named: Vec<(&str, f32)> = values.iter().map(|(name, n)| (name.as_str(), *n)).collect();
    assert_eq!(
        named,
        [
            ("Speed: trait", 12.0),
            ("hunger & not ate → Eat: weight", 0.5),
            ("boredom → exploration_mod: threshold", 0.25),
            ("boredom → exploration_mod: gain", 2.0),
            ("habit_fade: brain", 0.01),
            ("always → Wander until Adult: weight", 0.15),
        ]
    );
}

#[test]
fn the_starter_genome_has_values_to_follow() {
    let data = builtin();
    let starter = gene_values(&data.starter().genes(&data), &data.starter().switches());
    assert!(starter.len() > 50, "{} values", starter.len());
}

#[test]
fn a_gene_split_by_life_stage_lines_up_by_when_it_works() {
    // M2 design §13: a sprite that lost the young's copy of a split gene
    // has its grown sprites' value set beside the starter's grown one, not
    // the young's.
    let data = builtin();
    let wander = |genes: &str| {
        let text = format!("(format: 2, genes: [{genes}])");
        Genome::from_ron(&text, &data).expect("a valid genome")
    };
    let young =
        r#"Instinct(inputs: [("always", false)], verb: Wander, weight: 0.15, until: Adult)"#;
    let grown = r#"Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3, from: Adult)"#;
    let both = wander(&format!("{young}, {grown}"));
    let sprites = vec![
        (at(1, 1), Some(both.clone())),
        (at(3, 1), Some(wander(grown))),
    ];
    let world = WorldBuilder::new(&["........"; 3])
        .sprites_with(sprites)
        .build();
    let values = gene_values(&both.genes(&data), &both.switches());
    assert_eq!(
        middle_values(&world, &values),
        Some(vec![Some(0.15), Some(0.3)])
    );
}

#[test]
fn each_value_s_middle_among_the_living() {
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
        "a value no sprite has has no middle"
    );
}

#[test]
fn with_none_alive_there_are_no_middles() {
    let world = WorldBuilder::new(&["........"; 3]).build();
    assert_eq!(middle_values(&world, &[("Speed: trait".into(), 1.0)]), None);
}

#[test]
fn a_value_drifts_once_it_moves_a_tenth_either_way() {
    assert_eq!(drift(1.1), Some(Drift::Up), "a tenth up counts");
    assert_eq!(drift(0.9), Some(Drift::Down), "a tenth down counts");
    assert_eq!(drift(1.09), None);
    assert_eq!(drift(0.91), None);
    assert_eq!(drift(-0.5), Some(Drift::Down), "crossing 0 is down");
    assert_eq!(drift(f32::NAN), None, "nothing to compare");
}
