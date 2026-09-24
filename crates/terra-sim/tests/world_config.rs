use terra_sim::{ConfigError, WorldConfig};

#[test]
fn the_built_in_preset_is_160_by_96() {
    let config = WorldConfig::builtin();
    assert_eq!((config.width(), config.height()), (160, 96));
}

#[test]
fn a_preset_sets_the_map_size() {
    let config = WorldConfig::from_ron("(width: 64, height: 40)").expect("valid preset");
    assert_eq!((config.width(), config.height()), (64, 40));
}

#[test]
fn a_preset_with_an_unknown_field_is_a_parse_error() {
    let result = WorldConfig::from_ron("(width: 64, height: 40, sprites: 30)");
    assert!(matches!(result, Err(ConfigError::Parse(_))), "{result:?}");
}

#[test]
fn map_sides_must_be_from_32_to_1024_tiles() {
    for (width, height) in [(31, 64), (64, 31), (1025, 64), (64, 1025)] {
        let result = WorldConfig::from_ron(&format!("(width: {width}, height: {height})"));
        assert!(
            matches!(result, Err(ConfigError::Invalid(_))),
            "{width}x{height}: {result:?}"
        );
    }
    for (width, height) in [(32, 32), (1024, 1024)] {
        let result = WorldConfig::from_ron(&format!("(width: {width}, height: {height})"));
        assert!(result.is_ok(), "{width}x{height}: {result:?}");
    }
}
