use terra_sim::{DataError, DataPack, Terrain};

const BUILTIN_TERRAIN: &str = include_str!("../../../data/terrain.ron");

/// Loads the built-in pack with `file` replaced by `text`.
fn builtin_with(file: &str, text: &str) -> Result<DataPack, DataError> {
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| (path, if path == file { text } else { builtin }))
        .collect();
    DataPack::from_sources(&sources)
}

/// Asserts that the built-in pack with `file` replaced by `text` is invalid,
/// with a message mentioning `word`.
fn assert_invalid(file: &str, text: &str, word: &str) {
    match builtin_with(file, text) {
        Err(DataError::Invalid {
            file: bad_file,
            message,
        }) => {
            assert_eq!(bad_file, file);
            assert!(message.contains(word), "{message:?} should mention {word}");
        }
        other => panic!("expected {file} to be invalid, got {other:?}"),
    }
}

#[test]
fn a_pack_without_its_physiology_or_its_starter_genome_is_rejected() {
    for file in ["physiology.ron", "genomes/starter.ron"] {
        assert_eq!(
            builtin_without(file).unwrap_err(),
            DataError::MissingFile(file.into())
        );
    }
}

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

    // Design §3.1: (terrain, step cost, fertility, drinkable, allows fixtures).
    let walkable = [
        (Terrain::Grass, 10, 1.0, false, true),
        (Terrain::Dirt, 10, 0.5, false, true),
        (Terrain::Sand, 15, 0.0, false, true),
        (Terrain::ShallowWater, 25, 0.0, true, false),
    ];
    for (terrain, step_cost, fertility, drinkable, allows_fixtures) in walkable {
        let props = pack.terrain(terrain);
        assert_eq!(props.step_cost(), Some(step_cost), "{terrain:?}");
        assert_eq!(props.fertility(), fertility, "{terrain:?}");
        assert_eq!(props.is_drinkable(), drinkable, "{terrain:?}");
        assert_eq!(props.allows_fixtures(), allows_fixtures, "{terrain:?}");
    }
    for terrain in [Terrain::DeepWater, Terrain::Rock] {
        assert_eq!(pack.terrain(terrain).step_cost(), None, "{terrain:?}");
        assert!(!pack.terrain(terrain).allows_fixtures(), "{terrain:?}");
    }
}

/// A valid `terrain.ron` with one entry's properties replaced, or removed when `props` is empty.
fn terrain_with(name: &str, props: &str) -> String {
    let entries = [
        (
            "grass",
            "(walkable: true, step_cost: 10, fertility: 1.0, drinkable: false, allows_fixtures: true)",
        ),
        (
            "dirt",
            "(walkable: true, step_cost: 10, fertility: 0.5, drinkable: false, allows_fixtures: true)",
        ),
        (
            "sand",
            "(walkable: true, step_cost: 15, fertility: 0.0, drinkable: false, allows_fixtures: true)",
        ),
        (
            "shallow_water",
            "(walkable: true, step_cost: 25, fertility: 0.0, drinkable: true, allows_fixtures: false)",
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
fn a_walkable_terrain_must_say_whether_it_allows_fixtures() {
    let props = "(walkable: true, step_cost: 15, fertility: 0.0, drinkable: false)";
    assert_invalid_terrain(&terrain_with("sand", props), "sand");
}

#[test]
fn an_unwalkable_terrain_gives_nothing_but_walkable_false() {
    for extra in [
        "step_cost: 10",
        "fertility: 0.0",
        "drinkable: true",
        "allows_fixtures: false",
    ] {
        let props = format!("(walkable: false, {extra})");
        assert_invalid_terrain(&terrain_with("rock", &props), "rock");
    }
}

#[test]
fn a_pack_without_a_chemicals_file_is_rejected() {
    let result = DataPack::from_sources(&[
        ("pack.ron", r#"(name: "test", version: "1")"#),
        ("terrain.ron", BUILTIN_TERRAIN),
    ]);
    assert_eq!(
        result.unwrap_err(),
        DataError::MissingFile("chemicals.ron".into())
    );
}

#[test]
fn chemical_ids_and_names_are_unique() {
    let same_id =
        r#"[(id: 1, name: "energy", class: Physical), (id: 1, name: "food", class: Physical)]"#;
    assert_invalid("chemicals.ron", same_id, "1");
    let same_name =
        r#"[(id: 1, name: "food", class: Physical), (id: 4, name: "food", class: Physical)]"#;
    assert_invalid("chemicals.ron", same_name, "food");
}

/// Loads the built-in pack without `file`.
fn builtin_without(file: &str) -> Result<DataPack, DataError> {
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .copied()
        .filter(|&(path, _)| path != file)
        .collect();
    DataPack::from_sources(&sources)
}

#[test]
fn a_pack_without_a_loci_file_is_rejected() {
    assert_eq!(
        builtin_without("loci.ron").unwrap_err(),
        DataError::MissingFile("loci.ron".into())
    );
}

#[test]
fn a_pack_without_an_objects_file_is_rejected() {
    assert_eq!(
        builtin_without("objects.ron").unwrap_err(),
        DataError::MissingFile("objects.ron".into())
    );
}

#[test]
fn locus_ids_and_names_are_unique() {
    let same_id = r#"[(id: 32, name: "ate", kind: Pulse, brain_visible: true), (id: 32, name: "drank", kind: Pulse, brain_visible: true)]"#;
    assert_invalid("loci.ron", same_id, "32");
    let same_name = r#"[(id: 32, name: "ate", kind: Pulse, brain_visible: true), (id: 33, name: "ate", kind: Pulse, brain_visible: true)]"#;
    assert_invalid("loci.ron", same_name, "ate");
}

#[test]
fn dirt_and_shallow_water_must_stay_walkable_because_carving_makes_them() {
    for terrain in ["dirt", "shallow_water"] {
        assert_invalid_terrain(&terrain_with(terrain, "(walkable: false)"), terrain);
    }
}

/// Asserts that a pack whose `objects.ron` holds only `types` is invalid, with
/// a message mentioning every one of `words`.
fn assert_invalid_objects(types: &[&str], words: &[&str]) {
    let text = format!("[{}]", types.join(",\n"));
    match builtin_with("objects.ron", &text) {
        Err(DataError::Invalid { file, message }) => {
            assert_eq!(file, "objects.ron");
            for word in words {
                assert!(message.contains(word), "{message:?} should mention {word}");
            }
        }
        other => panic!("expected objects.ron to be invalid, got {other:?}\n{text}"),
    }
}

/// An object type called `name` with ID `id`, and `fields` added to the given
/// basics: two stages, `young` then `old`, and a `fruit` counter.
fn bush(id: u16, name: &str, fields: &str) -> String {
    format!(
        r#"(id: {id}, name: "{name}", category: BerryBush, tags: [Solid, Fixture],
            counters: {{"fruit": 6}},
            stages: [(name: "young", ticks: (10, 20), next: Stage("old")),
                     (name: "old", ticks: (10, 20), next: Expire)],
            {fields})"#
    )
}

#[test]
fn object_type_ids_and_names_are_unique() {
    assert_invalid_objects(&[&bush(1, "bush", ""), &bush(1, "tree", "")], &["1"]);
    assert_invalid_objects(&[&bush(1, "bush", ""), &bush(2, "bush", "")], &["bush"]);
}

#[test]
fn every_name_objects_ron_uses_must_exist() {
    let cases = [
        (
            r#"rules: [(trigger: Every(5), do: [SpawnNearby("shrub", 1)])]"#,
            "shrub",
        ),
        (
            r#"rules: [(trigger: Every(5), if: [DensityBelow("shrub", 2, 1)], do: [DestroySelf])]"#,
            "shrub",
        ),
        (
            r#"rules: [(trigger: OnStageEnter("adult"), do: [DestroySelf])]"#,
            "adult",
        ),
        (
            r#"rules: [(trigger: Every(5), if: [InStage("adult")], do: [DestroySelf])]"#,
            "adult",
        ),
        (
            r#"rules: [(trigger: Every(5), do: [AddCounter("seeds", 1)])]"#,
            "seeds",
        ),
        (r#"verbs: {Eat: [Inject(Actor, "sugar", 0.1)]}"#, "sugar"),
        (r#"verbs: {Eat: [Signal(Actor, "burped")]}"#, "burped"),
    ];
    for (fields, unknown) in cases {
        assert_invalid_objects(&[&bush(1, "bush", fields)], &["bush", unknown]);
    }
    let dead_end = r#"(id: 1, name: "bush", category: BerryBush,
        stages: [(name: "young", ticks: (10, 20), next: Stage("adult"))])"#;
    assert_invalid_objects(&[dead_end], &["adult"]);
}

#[test]
fn a_lifecycle_rule_may_not_use_a_verb_only_effect() {
    for effect in [
        r#"RequireCounter("fruit", 1)"#,
        r#"Inject(Actor, "food", 0.1)"#,
        r#"Signal(Actor, "ate")"#,
        "Push(1)",
    ] {
        let name = &effect[..effect.find('(').unwrap()];
        let rule = format!("rules: [(trigger: Every(5), do: [{effect}])]");
        assert_invalid_objects(&[&bush(1, "bush", &rule)], &["bush", "rule 1", name]);
    }
}

#[test]
fn a_verb_may_inject_only_physical_chemicals_and_signal_only_pulses() {
    let hunger = r#"verbs: {Eat: [Inject(Actor, "hunger", 0.1)]}"#;
    assert_invalid_objects(&[&bush(1, "bush", hunger)], &["bush", "Eat", "hunger"]);
    let age = r#"verbs: {Eat: [Signal(Actor, "age")]}"#;
    assert_invalid_objects(&[&bush(1, "bush", age)], &["bush", "Eat", "age"]);
}

#[test]
fn chance_must_be_a_probability_and_is_not_allowed_in_visual_rules() {
    for p in ["-0.1", "1.5", "NaN"] {
        let rule = format!("rules: [(trigger: Every(5), if: [Chance({p})], do: [DestroySelf])]");
        assert_invalid_objects(&[&bush(1, "bush", &rule)], &["bush", "Chance"]);
    }
    let visual = r#"visual: [(if: [Chance(0.5)], state: "shiny")]"#;
    assert_invalid_objects(
        &[&bush(1, "bush", visual)],
        &["bush", "visual rule 1", "Chance"],
    );
}

#[test]
fn stages_counters_and_triggers_need_sensible_numbers() {
    let stage = |ticks: &str| {
        format!(
            r#"(id: 1, name: "bush", category: BerryBush,
                stages: [(name: "young", ticks: {ticks}, next: Expire)])"#
        )
    };
    assert_invalid_objects(&[&stage("(0, 10)")], &["bush", "young"]);
    assert_invalid_objects(&[&stage("(20, 10)")], &["bush", "young"]);

    let no_fruit = r#"(id: 1, name: "bush", category: BerryBush, counters: {"fruit": 0})"#;
    assert_invalid_objects(&[no_fruit], &["bush", "fruit"]);

    let never = "rules: [(trigger: Every(0), do: [DestroySelf])]";
    assert_invalid_objects(&[&bush(1, "bush", never)], &["bush", "Every(0)"]);

    let twice = r#"(id: 1, name: "bush", category: BerryBush,
        stages: [(name: "young", ticks: (1, 2), next: Stage("young")),
                 (name: "young", ticks: (1, 2), next: Expire)])"#;
    assert_invalid_objects(&[twice], &["bush", "young"]);
}

#[test]
fn an_object_type_is_solid_and_a_fixture_or_neither() {
    let with_tags = |tags: &str| format!(r#"(id: 1, name: "crate", category: Ball, tags: {tags})"#);
    assert_invalid_objects(&[&with_tags("[Solid]")], &["crate", "solid", "fixture"]);
    assert_invalid_objects(&[&with_tags("[Fixture]")], &["crate", "solid", "fixture"]);
    for valid in ["[Solid, Fixture]", "[]"] {
        let text = format!("[{}]", with_tags(valid));
        assert!(builtin_with("objects.ron", &text).is_ok(), "{valid}");
    }
}

#[test]
fn a_pseudo_type_is_a_verb_table_and_nothing_else() {
    for (fields, what) in [
        ("tags: [Solid, Fixture]", "tags"),
        (r#"counters: {"sips": 3}"#, "counters"),
        (
            r#"stages: [(name: "wet", ticks: (1, 2), next: Expire)]"#,
            "stages",
        ),
        ("rules: [(trigger: Every(5), do: [DestroySelf])]", "rules"),
        (r#"visual: [(state: "wet")]"#, "visual"),
    ] {
        let water = format!(r#"(id: 100, name: "water", category: Water, pseudo: true, {fields})"#);
        assert_invalid_objects(&[&water], &["water", what]);
    }
}

#[test]
fn only_real_object_types_can_be_spawned_spread_replaced_or_counted() {
    let water = r#"(id: 100, name: "water", category: Water, pseudo: true)"#;
    for fields in [
        r#"rules: [(trigger: Every(5), do: [SpawnNearby("water", 1)])]"#,
        r#"rules: [(trigger: Every(5), do: [SpreadTo("water", 1, [])])]"#,
        r#"rules: [(trigger: Every(5), do: [ReplaceWith("water")])]"#,
        r#"rules: [(trigger: Every(5), if: [DensityBelow("water", 2, 1)], do: [DestroySelf])]"#,
    ] {
        assert_invalid_objects(
            &[&bush(1, "bush", fields), water],
            &["bush", "water", "pseudo"],
        );
    }
}

#[test]
fn only_eat_drink_hit_and_play_have_verb_tables() {
    for verb in ["Approach", "Retreat", "Rest", "Wander", "Mate", "Speak"] {
        let fields = format!(r#"verbs: {{{verb}: [AddCounter("fruit", 1)]}}"#);
        assert_invalid_objects(&[&bush(1, "bush", &fields)], &["bush", verb]);
    }
}

#[test]
fn the_built_in_pack_describes_its_object_types_for_display() {
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let names: Vec<&str> = pack.object_type_names().collect();
    assert_eq!(
        names,
        ["berry_bush", "berry", "thornbush", "ball"],
        "ID order, no pseudo types"
    );
    assert_eq!(pack.stage_names("berry_bush"), ["seedling", "mature"]);
    assert_eq!(pack.counter_names("berry_bush"), ["fruit"]);
    assert_eq!(
        pack.visual_states("berry_bush"),
        ["seedling", "fruiting", "default"]
    );
    assert_eq!(pack.visual_states("ball"), ["default"]);
    assert!(pack.stage_names("ball").is_empty());
    assert!(
        pack.stage_names("shrub").is_empty(),
        "an unknown type has nothing"
    );
}

const BUILTIN_PHYSIOLOGY: &str = include_str!("../../../data/physiology.ron");

/// Asserts that the built-in physiology with `from` replaced by `to` is
/// invalid, with a message mentioning `word`.
fn assert_invalid_physiology(from: &str, to: &str, word: &str) {
    assert!(
        BUILTIN_PHYSIOLOGY.contains(from),
        "{from:?} is in physiology.ron"
    );
    assert_invalid(
        "physiology.ron",
        &BUILTIN_PHYSIOLOGY.replace(from, to),
        word,
    );
}

#[test]
fn physiology_levels_are_fractions_from_0_to_1() {
    assert_invalid_physiology("energy: 1.0", "energy: 1.5", "energy");
    assert_invalid_physiology("stamina: 1.0)", "stamina: -0.1)", "stamina");
    assert_invalid_physiology(
        "first_population: (0.6, 1.0)",
        "first_population: (0.6, 1.2)",
        "first_population",
    );
}

#[test]
fn physiology_ranges_go_from_low_to_high() {
    assert_invalid_physiology(
        "first_population: (0.6, 1.0)",
        "first_population: (0.9, 0.6)",
        "first_population",
    );
    assert_invalid_physiology("speed: (4.0, 12.0)", "speed: (12.0, 4.0)", "speed");
    assert_invalid_physiology("(0.25, 4.0)", "(4.0, 0.25)", "exploration_mod");
}

#[test]
fn physiology_rates_are_not_negative() {
    assert_invalid_physiology("basal: 0.0001", "basal: -0.0001", "basal");
    assert_invalid_physiology("healing: 0.0001", "healing: -0.0001", "healing");
    assert_invalid_physiology("old_age: 0.0006", "old_age: -0.0006", "old_age");
}

#[test]
fn causes_of_death_fade_over_at_least_one_tick() {
    assert_invalid_physiology("cause_fade: 350", "cause_fade: 0", "cause_fade");
}

#[test]
fn spawn_variation_is_a_fraction_below_1() {
    assert_invalid_physiology(
        "spawn_variation: 0.1",
        "spawn_variation: 1.0",
        "spawn_variation",
    );
    assert_invalid_physiology(
        "spawn_variation: 0.1",
        "spawn_variation: -0.1",
        "spawn_variation",
    );
}

#[test]
fn every_receptor_target_has_a_range_and_nothing_else_does() {
    assert_invalid_physiology("\"exploration_mod\": (0.25, 4.0),", "", "exploration_mod");
    assert_invalid_physiology(
        "\"exploration_mod\": (0.25, 4.0),",
        "\"exploration_mod\": (0.25, 4.0), \"ate\": (0.0, 1.0),",
        "ate",
    );
}
