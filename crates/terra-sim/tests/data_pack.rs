use terra_sim::{DataError, DataPack, Terrain};

#[test]
fn a_pack_without_a_manifest_is_rejected() {
    let result = DataPack::from_sources(&[]);
    assert_eq!(
        result.unwrap_err(),
        DataError::MissingFile("pack.ron".into())
    );
}

#[test]
fn a_malformed_manifest_is_reported_as_a_parse_error_naming_the_file() {
    let result = DataPack::from_sources(&[("pack.ron", "(name: \"unterminated")]);
    match result.unwrap_err() {
        DataError::Parse { file, .. } => assert_eq!(file, "pack.ron"),
        other => panic!("expected a parse error, got {other:?}"),
    }
}

#[test]
fn a_manifest_with_an_empty_name_or_version_is_invalid() {
    for text in [
        r#"(name: "", version: "1")"#,
        r#"(name: "core", version: " ")"#,
    ] {
        match DataPack::from_sources(&[("pack.ron", text)]).unwrap_err() {
            DataError::Invalid { file, .. } => assert_eq!(file, "pack.ron"),
            other => panic!("expected an invalid-pack error for {text}, got {other:?}"),
        }
    }
}

#[test]
fn the_built_in_pack_loads_and_identifies_itself() {
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    assert_eq!(pack.name(), "core");
    assert!(!pack.version().is_empty());
}

#[test]
fn the_built_in_pack_has_the_design_terrain_table() {
    let pack = DataPack::builtin().expect("built-in data pack is valid");

    // Design §3.1: (terrain, step cost, fertility, drinkable).
    let walkable = [
        (Terrain::Grass, 10, 1.0, false),
        (Terrain::Dirt, 10, 0.5, false),
        (Terrain::Sand, 15, 0.0, false),
        (Terrain::ShallowWater, 25, 0.0, true),
    ];
    for (terrain, step_cost, fertility, drinkable) in walkable {
        let props = pack.terrain(terrain);
        assert_eq!(props.step_cost(), Some(step_cost), "{terrain:?}");
        assert_eq!(props.fertility(), fertility, "{terrain:?}");
        assert_eq!(props.is_drinkable(), drinkable, "{terrain:?}");
    }
    for terrain in [Terrain::DeepWater, Terrain::Rock] {
        assert_eq!(pack.terrain(terrain).step_cost(), None, "{terrain:?}");
    }
}

/// A valid `terrain.ron` with one entry's properties replaced, or removed when `props` is empty.
fn terrain_with(name: &str, props: &str) -> String {
    let entries = [
        (
            "grass",
            "(walkable: true, step_cost: 10, fertility: 1.0, drinkable: false)",
        ),
        (
            "dirt",
            "(walkable: true, step_cost: 10, fertility: 0.5, drinkable: false)",
        ),
        (
            "sand",
            "(walkable: true, step_cost: 15, fertility: 0.0, drinkable: false)",
        ),
        (
            "shallow_water",
            "(walkable: true, step_cost: 25, fertility: 0.0, drinkable: true)",
        ),
        ("deep_water", "(walkable: false)"),
        ("rock", "(walkable: false)"),
    ];
    let body: Vec<String> = entries
        .iter()
        .filter_map(|&(entry, default)| match (entry == name, props) {
            (true, "") => None,
            (true, props) => Some(format!("{entry}: {props}")),
            (false, _) => Some(format!("{entry}: {default}")),
        })
        .collect();
    format!("{{ {} }}", body.join(", "))
}

/// Loads a pack with a valid manifest and the given `terrain.ron`.
fn load_with_terrain(terrain: &str) -> Result<DataPack, DataError> {
    DataPack::from_sources(&[
        ("pack.ron", r#"(name: "test", version: "1")"#),
        ("terrain.ron", terrain),
    ])
}

/// Asserts the terrain file is rejected as invalid, with a message naming `word`.
fn assert_invalid_terrain(terrain: &str, word: &str) {
    match load_with_terrain(terrain) {
        Err(DataError::Invalid { file, message }) => {
            assert_eq!(file, "terrain.ron");
            assert!(message.contains(word), "{message:?} should mention {word}");
        }
        other => panic!("expected terrain.ron to be invalid, got {other:?}"),
    }
}

#[test]
fn a_pack_without_a_terrain_file_is_rejected() {
    let result = DataPack::from_sources(&[("pack.ron", r#"(name: "test", version: "1")"#)]);
    assert_eq!(
        result.unwrap_err(),
        DataError::MissingFile("terrain.ron".into())
    );
}

#[test]
fn every_terrain_must_be_listed() {
    assert_invalid_terrain(&terrain_with("sand", ""), "sand");
}

#[test]
fn a_walkable_terrain_needs_a_step_cost_above_zero() {
    let no_cost = "(walkable: true, fertility: 1.0, drinkable: false)";
    assert_invalid_terrain(&terrain_with("grass", no_cost), "grass");
    let zero_cost = "(walkable: true, step_cost: 0, fertility: 1.0, drinkable: false)";
    assert_invalid_terrain(&terrain_with("grass", zero_cost), "grass");
}

#[test]
fn a_walkable_terrain_needs_a_fertility_from_zero_to_one() {
    for fertility in ["", "fertility: -0.1,", "fertility: 1.5,", "fertility: NaN,"] {
        let props = format!("(walkable: true, step_cost: 10, {fertility} drinkable: false)");
        assert_invalid_terrain(&terrain_with("dirt", &props), "dirt");
    }
}

#[test]
fn a_walkable_terrain_must_say_whether_it_is_drinkable() {
    let props = "(walkable: true, step_cost: 25, fertility: 0.0)";
    assert_invalid_terrain(&terrain_with("shallow_water", props), "shallow_water");
}

#[test]
fn an_unwalkable_terrain_gives_nothing_but_walkable_false() {
    for extra in ["step_cost: 10", "fertility: 0.0", "drinkable: true"] {
        let props = format!("(walkable: false, {extra})");
        assert_invalid_terrain(&terrain_with("rock", &props), "rock");
    }
}

#[test]
fn dirt_and_shallow_water_must_stay_walkable_because_carving_makes_them() {
    for terrain in ["dirt", "shallow_water"] {
        assert_invalid_terrain(&terrain_with(terrain, "(walkable: false)"), terrain);
    }
}
