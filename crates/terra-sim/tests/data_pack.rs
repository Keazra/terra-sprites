use terra_sim::{DataError, DataPack};

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
