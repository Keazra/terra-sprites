use terra_sim::{ConfigError, DataPack, WorldConfig};

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn preset(text: &str) -> Result<WorldConfig, ConfigError> {
    WorldConfig::from_ron(text, &pack())
}

#[test]
fn the_built_in_preset_is_256_by_160() {
    let config = WorldConfig::builtin(&pack());
    assert_eq!((config.width(), config.height()), (256, 160));
}

#[test]
fn the_built_in_preset_gives_the_design_object_counts_on_the_default_map() {
    // Design §3.9: 150 berry bushes, 40 thornbushes and 6 balls per 15,360 tiles.
    let config = WorldConfig::builtin(&pack());
    assert_eq!(config.object_count("berry_bush"), 400);
    assert_eq!(config.object_count("thornbush"), 107);
    assert_eq!(config.object_count("ball"), 16);
    assert_eq!(config.object_count("berry"), 0, "no berries are placed");
}

#[test]
fn a_preset_sets_the_map_size() {
    let config = preset("(width: 64, height: 40)").expect("valid preset");
    assert_eq!((config.width(), config.height()), (64, 40));
}

#[test]
fn a_preset_with_an_unknown_field_is_a_parse_error() {
    let result = preset("(width: 64, height: 40, critters: 30)");
    assert!(matches!(result, Err(ConfigError::Parse(_))), "{result:?}");
}

#[test]
fn map_sides_must_be_from_32_to_1024_tiles() {
    for (width, height) in [(31, 64), (64, 31), (1025, 64), (64, 1025)] {
        let result = preset(&format!("(width: {width}, height: {height})"));
        assert!(
            matches!(result, Err(ConfigError::Invalid(_))),
            "{width}x{height}: {result:?}"
        );
    }
    for (width, height) in [(32, 32), (1024, 1024)] {
        let result = preset(&format!("(width: {width}, height: {height})"));
        assert!(result.is_ok(), "{width}x{height}: {result:?}");
    }
}

#[test]
fn object_counts_scale_with_the_map_and_round_halves_up() {
    // 32 × 32 = 1,024 tiles: one per 2,048 tiles is exactly half an object.
    let half = preset(r#"(width: 32, height: 32, objects: {"ball": 1}, per_tiles: 2048)"#);
    assert_eq!(half.expect("valid preset").object_count("ball"), 1);
    let under_half = preset(r#"(width: 32, height: 32, objects: {"ball": 1}, per_tiles: 2049)"#);
    assert_eq!(under_half.expect("valid preset").object_count("ball"), 0);
}

/// Asserts `text` is an invalid preset, with a message mentioning `word`.
fn assert_invalid(text: &str, word: &str) {
    match preset(text) {
        Err(ConfigError::Invalid(message)) => {
            assert!(message.contains(word), "{message:?} should mention {word}")
        }
        other => panic!("expected an invalid preset, got {other:?}"),
    }
}

#[test]
fn a_preset_may_only_name_object_types_the_pack_has() {
    assert_invalid(
        r#"(width: 64, height: 64, objects: {"shrub": 1}, per_tiles: 100)"#,
        "shrub",
    );
    assert_invalid(
        r#"(width: 64, height: 64, objects: {"water": 1}, per_tiles: 100)"#,
        "water",
    );
}

#[test]
fn a_preset_that_names_objects_needs_an_area_to_count_them_in() {
    assert_invalid(
        r#"(width: 64, height: 64, objects: {"ball": 1})"#,
        "per_tiles",
    );
    assert_invalid(
        r#"(width: 64, height: 64, objects: {"ball": 1}, per_tiles: 0)"#,
        "per_tiles",
    );
}
