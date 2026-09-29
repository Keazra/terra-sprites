use terra_sim::{DataError, DataPack, Terrain, Verb};

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
    let same_id = r#"[(id: 32, name: "ate", kind: Pulse), (id: 32, name: "drank", kind: Pulse)]"#;
    assert_invalid("loci.ron", same_id, "32");
    let same_name = r#"[(id: 32, name: "ate", kind: Pulse), (id: 33, name: "ate", kind: Pulse)]"#;
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
/// basics: large and hard, two stages, `young` then `old`, and a `fruit` counter.
fn bush(id: u16, name: &str, fields: &str) -> String {
    format!(
        r#"(id: {id}, name: "{name}", category: "bush", tags: [Solid, Fixture],
            size: Large, hardness: 1.0,
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
    let dead_end = r#"(id: 1, name: "bush", category: "bush", size: Large, hardness: 1.0,
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
            r#"(id: 1, name: "bush", category: "bush", size: Large, hardness: 1.0,
                stages: [(name: "young", ticks: {ticks}, next: Expire)])"#
        )
    };
    assert_invalid_objects(&[&stage("(0, 10)")], &["bush", "young"]);
    assert_invalid_objects(&[&stage("(20, 10)")], &["bush", "young"]);

    let no_fruit = r#"(id: 1, name: "bush", category: "bush", size: Large, hardness: 1.0, counters: {"fruit": 0})"#;
    assert_invalid_objects(&[no_fruit], &["bush", "fruit"]);

    let never = "rules: [(trigger: Every(0), do: [DestroySelf])]";
    assert_invalid_objects(&[&bush(1, "bush", never)], &["bush", "Every(0)"]);

    let twice = r#"(id: 1, name: "bush", category: "bush", size: Large, hardness: 1.0,
        stages: [(name: "young", ticks: (1, 2), next: Stage("young")),
                 (name: "young", ticks: (1, 2), next: Expire)])"#;
    assert_invalid_objects(&[twice], &["bush", "young"]);
}

#[test]
fn an_object_type_is_solid_and_a_fixture_or_neither() {
    let with_tags = |tags: &str| {
        format!(
            r#"(id: 1, name: "crate", category: "toy", size: Medium, hardness: 0.5, tags: {tags})"#
        )
    };
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
        let water =
            format!(r#"(id: 100, name: "water", category: "water", pseudo: true, {fields})"#);
        assert_invalid_objects(&[&water], &["water", what]);
    }
}

#[test]
fn only_real_object_types_can_be_spawned_spread_replaced_or_counted() {
    let water = r#"(id: 100, name: "water", category: "water", pseudo: true)"#;
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

#[test]
fn each_object_type_is_in_the_category_it_names() {
    // Design v20 §3.5.3, §3.5.5: thornbushes are bushes, as berry bushes are.
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let types = [
        "berry_bush",
        "berry",
        "thornbush",
        "ball",
        "water",
        "sprite",
    ];
    assert_eq!(
        types.map(|object_type| pack.category_of(object_type)),
        [
            Some("bush"),
            Some("fruit"),
            Some("bush"),
            Some("toy"),
            Some("water"),
            Some("sprite"),
        ]
    );
    assert_eq!(pack.category_of("dragon"), None, "no such object type");
}

/// The built-in categories plus `extra`, as `categories.ron`.
fn categories_with(extra: &str) -> String {
    format!(
        r#"[(id: 1, name: "bush"), (id: 2, name: "fruit"), (id: 4, name: "water"),
            (id: 5, name: "toy"), (id: 6, name: "sprite"), {extra}]"#
    )
}

#[test]
fn each_category_is_worded_in_general_by_its_plural() {
    // Design v19 §3.5.5: as an object type's is, less any spaces at either
    // end, and never empty.
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let names = ["bush", "fruit", "water", "toy", "sprite"];
    assert_eq!(
        names.map(|name| pack.category_plural(name)),
        [Some("bushes"), None, None, Some("toys"), Some("sprites")],
        "fruit and water aren't counted"
    );
    assert_eq!(pack.category_plural("dragon"), None, "no such category");

    let text = categories_with(r#"(id: 7, name: "tree", plural: "  trees ")"#);
    let pack = builtin_with("categories.ron", &text).expect("a valid pack");
    assert_eq!(pack.category_plural("tree"), Some("trees"));
    for plural in [r#""""#, r#""  ""#] {
        let text = categories_with(&format!(r#"(id: 7, name: "tree", plural: {plural})"#));
        assert_invalid("categories.ron", &text, "`tree` has an empty plural");
    }
}

#[test]
fn a_pack_without_a_categories_file_is_rejected() {
    assert_eq!(
        builtin_without("categories.ron").unwrap_err(),
        DataError::MissingFile("categories.ron".into())
    );
}

#[test]
fn category_ids_and_names_are_unique_and_from_1_to_26() {
    // Each category's brain input takes an ID from 36 to 63 (design v19 §3.5.5).
    let file = "categories.ron";
    assert_invalid(
        file,
        &categories_with(r#"(id: 1, name: "tree")"#),
        "the id 1",
    );
    assert_invalid(file, &categories_with(r#"(id: 7, name: "bush")"#), "`bush`");
    for id in [0, 27] {
        let text = categories_with(&format!(r#"(id: {id}, name: "tree")"#));
        assert_invalid(file, &text, &format!("the id {id}"));
    }
    let text = categories_with(r#"(id: 26, name: "tree")"#);
    assert!(builtin_with(file, &text).is_ok(), "26 is the last ID");
}

#[test]
fn the_categories_include_water_and_sprites() {
    // Water tiles and sprites are physics, not objects: the engine perceives
    // them as these two categories, whatever else the list holds.
    for missing in ["water", "sprite"] {
        let text =
            categories_with("").replace(&format!(r#", name: "{missing}")"#), r#", name: "moss")"#);
        assert_invalid("categories.ron", &text, &format!("`{missing}`"));
    }
}

#[test]
fn an_object_type_names_a_category_in_the_list() {
    let shrub = r#"(id: 1, name: "shrub", category: "tree", tags: [Solid, Fixture],
        size: Large, hardness: 1.0)"#;
    assert_invalid_objects(&[shrub], &["shrub", "unknown category `tree`"]);
}

#[test]
fn each_object_type_is_worded_in_general_by_its_own_plural() {
    // Design v19 §6.1, replacing v17's wording of a kind by the first object
    // type perceived as it.
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let types = [
        "berry_bush",
        "berry",
        "thornbush",
        "water",
        "ball",
        "sprite",
    ];
    assert_eq!(
        types.map(|object_type| pack.plural_of(object_type)),
        [
            Some("berry bushes"),
            Some("berries"),
            Some("thornbushes"),
            None,
            Some("balls"),
            Some("sprites"),
        ],
        "water isn't counted"
    );
    assert_eq!(pack.plural_of("dragon"), None, "no such object type");

    // Two bushes in one category each keep their own, and one that gives
    // none isn't counted.
    let text = format!(
        "[{}, {}]",
        bush(1, "shrub", ""),
        bush(2, "bramble", r#"plural: "brambles""#)
    );
    let pack = builtin_with("objects.ron", &text).expect("a valid pack");
    assert_eq!(pack.plural_of("bramble"), Some("brambles"));
    assert_eq!(pack.plural_of("shrub"), None);
}

#[test]
fn an_object_type_s_plural_loses_any_spaces_at_either_end() {
    let text = format!("[{}]", bush(1, "shrub", r#"plural: "  shrubs ""#));
    let pack = builtin_with("objects.ron", &text).expect("a valid pack");
    assert_eq!(pack.plural_of("shrub"), Some("shrubs"));
}

#[test]
fn an_object_type_s_plural_may_not_be_empty() {
    for plural in [r#""""#, r#""  ""#] {
        let fields = format!("plural: {plural}");
        assert_invalid_objects(&[&bush(1, "bush", &fields)], &["bush", "plural"]);
    }
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

#[test]
fn a_starter_genome_with_a_flagged_unknown_or_unmatched_gene_does_not_load() {
    let starter = |gene: &str| format!("(format: 1, genes: [{gene}])");
    // Each would silently do nothing (design §4.3, v19 §5.7). An unmatched
    // gene names the category it's missing, however it names it.
    assert_invalid(
        "genomes/starter.ron",
        &starter(r#"AttentionInstinct(input: "hunger", category: "tree", weight: 1.0)"#),
        "(`tree`)",
    );
    assert_invalid(
        "genomes/starter.ron",
        &starter(
            r#"Instinct(inputs: [("hunger", false), ("attended_tree", false)], verb: Eat, weight: 1.0)"#,
        ),
        "(`tree`)",
    );
    assert_invalid(
        "genomes/starter.ron",
        &starter(r#"HalfLife(chem: "energy", ticks: 10)"#),
        "energy",
    );
    assert_invalid(
        "genomes/starter.ron",
        &starter(r#"Gene(type: 900, version: 1, payload: "")"#),
        "900",
    );
    assert_invalid(
        "genomes/starter.ron",
        &starter(r#"HalfLife(chem: "glee", ticks: 10)"#),
        "glee",
    );
}

#[test]
fn physiology_needs_its_physical_chemicals_and_body_sensors() {
    let chemicals = include_str!("../../../data/chemicals.ron");
    let loci = include_str!("../../../data/loci.ron");
    let changed = |text: &str, from: &str, to: &str| {
        assert!(text.contains(from), "{from:?} is in the file");
        text.replace(from, to)
    };
    let stamina = r#"(id: 3,  name: "stamina",     class: Physical)"#;
    assert_invalid(
        "chemicals.ron",
        &changed(
            chemicals,
            stamina,
            r#"(id: 3,  name: "vigour",      class: Physical)"#,
        ),
        "stamina",
    );
    assert_invalid(
        "chemicals.ron",
        &changed(
            chemicals,
            stamina,
            r#"(id: 3,  name: "stamina",     class: Signal)"#,
        ),
        "stamina",
    );
    assert_invalid(
        "loci.ron",
        &changed(loci, r#"name: "resting","#, r#"name: "lazing","#),
        "resting",
    );
    assert_invalid(
        "loci.ron",
        &changed(
            loci,
            r#"name: "always",            kind: BodySensor"#,
            r#"name: "always",            kind: Pulse"#,
        ),
        "always",
    );
}

#[test]
fn trait_ranges_are_finite_and_above_0() {
    assert_invalid_physiology("speed: (4.0, 12.0)", "speed: (0.0, 12.0)", "speed");
    assert_invalid_physiology(
        "lifespan: (20000.0, 200000.0)",
        "lifespan: (0.0, 200000.0)",
        "lifespan",
    );
    assert_invalid_physiology(
        "lifespan: (20000.0, 200000.0)",
        "lifespan: (20000.0, inf)",
        "lifespan",
    );
}

#[test]
fn physiology_rates_are_finite() {
    assert_invalid_physiology("healing: 0.0001", "healing: inf", "healing");
}

#[test]
fn the_brain_feels_the_drives_hormones_body_sensors_and_pulses_brain_io_lists() {
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let inputs: Vec<(u16, &str)> = pack.brain_inputs().collect();
    let hormones: Vec<String> = (0..16).map(|n| format!("h{n}")).collect();
    let mut expected: Vec<(u16, &str)> = [
        "hunger",
        "thirst",
        "pain",
        "tiredness",
        "boredom",
        "loneliness",
        "crowdedness",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, name)| (i as u16 + 1, name))
    .collect();
    expected.extend(
        hormones
            .iter()
            .enumerate()
            .map(|(i, h)| (i as u16 + 8, h.as_str())),
    );
    expected.extend([(24, "nearby_sprites"), (25, "age"), (26, "always")]);
    let pulses = [
        "ate",
        "drank",
        "played",
        "played_social",
        "pricked",
        "was_hit",
        "did_hit",
        "petted",
        "shocked",
    ];
    expected.extend(pulses.iter().enumerate().map(|(i, &p)| (i as u16 + 27, p)));
    // The Target inputs, numbered from 36: one for each category, then the
    // two fixed in code (design v19 §3.5.5, Appendix A). 38 was
    // attended_thornbush, until thornbushes joined bush (v20).
    expected.extend([
        (36, "attended_bush"),
        (37, "attended_fruit"),
        (39, "attended_water"),
        (40, "attended_toy"),
        (41, "attended_sprite"),
        (42, "target_distance"),
        (43, "target_adjacent"),
    ]);
    // State inputs carry on from 64, past the IDs kept for Target inputs.
    expected.extend([(64, "cornered"), (65, "fruitless")]);
    assert_eq!(inputs, expected);
}

#[test]
fn a_category_s_brain_input_takes_its_id_from_the_category_s() {
    // 35 + id for categories 1 to 6, and 37 + id from 7, past target_distance
    // and target_adjacent (design v19 §3.5.5).
    let text = categories_with(r#"(id: 7, name: "tree"), (id: 26, name: "shell")"#);
    let pack = builtin_with("categories.ron", &text).expect("a valid pack");
    let targets: Vec<(u16, &str)> = pack
        .brain_inputs()
        .filter(|&(id, _)| (36..=63).contains(&id))
        .collect();
    assert_eq!(
        targets,
        [
            (36, "attended_bush"),
            (37, "attended_fruit"),
            (39, "attended_water"),
            (40, "attended_toy"),
            (41, "attended_sprite"),
            (42, "target_distance"),
            (43, "target_adjacent"),
            (44, "attended_tree"),
            (63, "attended_shell"),
        ]
    );

    // A retired category leaves its input's ID unused.
    let text = r#"[(id: 1, name: "bush"), (id: 2, name: "fruit"), (id: 4, name: "water"),
        (id: 5, name: "toy"), (id: 6, name: "sprite"), (id: 8, name: "thornbush")]"#;
    let pack = builtin_with("categories.ron", text).expect("a valid pack");
    let ids: Vec<u16> = pack
        .brain_inputs()
        .map(|(id, _)| id)
        .filter(|id| (36..=63).contains(id))
        .collect();
    assert_eq!(ids, [36, 37, 39, 40, 41, 42, 43, 45]);
}

#[test]
fn a_pack_without_a_brain_io_file_is_rejected() {
    assert_eq!(
        builtin_without("brain_io.ron").unwrap_err(),
        DataError::MissingFile("brain_io.ron".into())
    );
}

#[test]
fn a_brain_input_reads_a_drive_hormone_body_sensor_or_pulse_that_exists() {
    for (reads, word) in [
        (r#"Chem("nectar")"#, "nectar"),
        (r#"Locus("glow")"#, "glow"),
        // Physical chemicals: the brain feels the body only through drives.
        (r#"Chem("energy")"#, "energy"),
        // Learning signals.
        (r#"Chem("reward")"#, "reward"),
        (r#"Locus("exploration_mod")"#, "exploration_mod"),
    ] {
        let text = format!(r#"(inputs: [(id: 1, name: "odd", reads: {reads})], needs: [])"#);
        assert_invalid("brain_io.ron", &text, word);
    }
}

#[test]
fn brain_input_ids_and_names_are_unique_and_clear_of_the_target_inputs() {
    let same_id = r#"(inputs: [(id: 1, name: "a", reads: Chem("hunger")), (id: 1, name: "b", reads: Chem("thirst"))], needs: [])"#;
    assert_invalid("brain_io.ron", same_id, "1");
    let same_name = r#"(inputs: [(id: 1, name: "a", reads: Chem("hunger")), (id: 2, name: "a", reads: Chem("thirst"))], needs: [])"#;
    assert_invalid("brain_io.ron", same_name, "`a`");
    let target_name =
        r#"(inputs: [(id: 1, name: "target_adjacent", reads: Chem("hunger"))], needs: [])"#;
    assert_invalid("brain_io.ron", target_name, "target_adjacent");
    // 36 to 63 are kept for Target inputs.
    for id in [36, 63] {
        let target_id =
            format!(r#"(inputs: [(id: {id}, name: "hungry", reads: Chem("hunger"))], needs: [])"#);
        assert_invalid("brain_io.ron", &target_id, &id.to_string());
    }
}

#[test]
fn a_need_is_a_state_input_that_reads_a_drive_named_once() {
    // Design v16 §5.2.
    let inputs = r#"inputs: [
        (id: 1, name: "hungry", reads: Chem("hunger")),
        (id: 2, name: "h", reads: Chem("h0")),
        (id: 3, name: "ate", reads: Locus("ate")),
    ]"#;
    for (needs, word) in [
        (r#"["peckish"]"#, "peckish"),
        (r#"["h"]"#, "`h`"),
        (r#"["ate"]"#, "`ate`"),
        (r#"["target_adjacent"]"#, "target_adjacent"),
        (r#"["hungry", "hungry"]"#, "twice"),
    ] {
        let text = format!("({inputs}, needs: {needs})");
        assert_invalid("brain_io.ron", &text, word);
    }
}

#[test]
fn the_needs_are_the_drives_whose_relief_teaches() {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    let needs: Vec<&str> = data.needs().collect();
    assert_eq!(
        needs,
        [
            "hunger",
            "thirst",
            "tiredness",
            "boredom",
            "loneliness",
            "crowdedness"
        ],
        "pain is a drive but not a need"
    );
}

#[test]
fn every_brain_parameter_has_a_range_and_a_default_within_it() {
    let tau = r#""tau_base":          (range: (0.05, 2.0),    default: 0.2),"#;
    assert_invalid_physiology(tau, "", "tau_base");
    assert_invalid_physiology(
        tau,
        &format!(r#"{tau} "wonder": (range: (0.0, 1.0), default: 0.5),"#),
        "wonder",
    );
    assert_invalid_physiology(
        tau,
        r#""tau_base": (range: (0.05, 2.0), default: 3.0),"#,
        "tau_base",
    );
    assert_invalid_physiology(
        tau,
        r#""tau_base": (range: (2.0, 0.05), default: 0.2),"#,
        "tau_base",
    );
}

#[test]
fn the_brain_needs_the_exploration_mod_receptor_target() {
    let loci = include_str!("../../../data/loci.ron");
    let renamed = loci.replace(r#"name: "exploration_mod""#, r#"name: "whim_mod""#);
    assert_ne!(renamed, loci);
    assert_invalid("loci.ron", &renamed, "exploration_mod");
}

#[test]
fn a_pack_says_which_verbs_push_which_object_types() {
    // Object types: thornbush 3, ball 4, sprite 101.
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    assert!(pack.pushes(4, Verb::Play), "a ball rolls when played with");
    assert!(pack.pushes(4, Verb::Hit), "and when hit");
    assert!(!pack.pushes(3, Verb::Play), "a thornbush doesn't");
    assert!(!pack.pushes(101, Verb::Play), "nor does a sprite");
    assert!(!pack.pushes(999, Verb::Play), "nor a type there isn't");
}

#[test]
fn only_an_item_can_be_pushed() {
    // A fixture can't be pushed (design §3.5.1); nor can water or a sprite,
    // which have no objects to push.
    let bush = bush(1, "bush", "verbs: { Play: [Push(2)] }");
    assert_invalid_objects(&[&bush], &["bush", "Push", "item"]);
    let water = r#"(id: 100, name: "water", category: "water", pseudo: true,
        verbs: { Play: [Push(2)] })"#;
    assert_invalid_objects(&[water], &["water", "Push", "item"]);
}

#[test]
fn a_pseudo_type_s_verbs_only_inject_and_signal() {
    // Water and sprites aren't objects, so nothing else a verb does applies.
    for effect in ["DestroySelf", r#"SpawnNearby("bush", 1)"#] {
        let water = format!(
            r#"(id: 100, name: "water", category: "water", pseudo: true,
                verbs: {{ Drink: [{effect}] }})"#
        );
        let words = [
            "water",
            effect.split('(').next().expect("a name"),
            "a pseudo type's verbs may only Inject and Signal",
        ];
        assert_invalid_objects(&[&bush(1, "bush", ""), &water], &words);
    }
}

/// An item type called `pebble`, with `fields`.
fn pebble(fields: &str) -> String {
    format!(r#"(id: 5, name: "pebble", category: "toy", {fields})"#)
}

#[test]
fn an_object_type_with_objects_needs_a_size_and_a_hardness() {
    // What a rolling item does to what it meets depends on both (design §3.5.4).
    assert_invalid_objects(&[&pebble("hardness: 0.5")], &["pebble", "size"]);
    assert_invalid_objects(&[&pebble("size: Small")], &["pebble", "hardness"]);
    assert_invalid_objects(&[&pebble("")], &["pebble", "size", "hardness"]);
    let text = format!("[{}]", pebble("size: Small, hardness: 0.5"));
    assert!(builtin_with("objects.ron", &text).is_ok());
}

#[test]
fn hardness_is_from_0_to_1() {
    for hardness in ["-0.1", "1.1"] {
        let fields = format!("size: Small, hardness: {hardness}");
        assert_invalid_objects(&[&pebble(&fields)], &["pebble", "hardness"]);
    }
    for hardness in ["0.0", "1.0"] {
        let text = format!(
            "[{}]",
            pebble(&format!("size: Small, hardness: {hardness}"))
        );
        assert!(builtin_with("objects.ron", &text).is_ok(), "{hardness}");
    }
}

#[test]
fn a_pseudo_type_gives_both_a_size_and_a_hardness_or_neither() {
    // Sprites have a size and water doesn't, which the data says, not the code.
    let water = |fields: &str| {
        format!(r#"(id: 100, name: "water", category: "water", pseudo: true, {fields})"#)
    };
    assert_invalid_objects(
        &[&bush(1, "bush", ""), &water("size: Large")],
        &["water", "hardness"],
    );
    assert_invalid_objects(
        &[&bush(1, "bush", ""), &water("hardness: 0.5")],
        &["water", "size"],
    );
    for fields in ["", "size: Large, hardness: 0.5"] {
        let text = format!("[{}, {}]", bush(1, "bush", ""), water(fields));
        assert!(builtin_with("objects.ron", &text).is_ok(), "{fields:?}");
    }
}
