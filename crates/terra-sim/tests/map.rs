use terra_sim::{DataPack, Dir, Map, MapError, Pos};

/// Draws a map with the ascii legend: `.` grass, `,` dirt, `:` sand,
/// `~` shallow water, `=` deep water, `#` rock.
fn draw(rows: &[&str]) -> Map {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    Map::from_ascii(rows, &data).expect("valid drawing")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

#[test]
fn an_orthogonal_step_costs_the_destination_tiles_step_cost() {
    let map = draw(&[
        ".:.", //
        "~.,", //
        ".:.", //
    ]);
    let centre = at(1, 1);
    assert_eq!(map.step_cost(centre, Dir::N), Some(15), "onto sand");
    assert_eq!(map.step_cost(centre, Dir::E), Some(10), "onto dirt");
    assert_eq!(map.step_cost(centre, Dir::S), Some(15), "onto sand");
    assert_eq!(
        map.step_cost(centre, Dir::W),
        Some(25),
        "onto shallow water"
    );
    assert_eq!(map.step_cost(at(0, 1), Dir::E), Some(10), "onto grass");
}

#[test]
fn a_diagonal_step_costs_fourteen_tenths_in_integer_maths() {
    let map = draw(&[
        ".:.", //
        "...", //
        "~.,", //
    ]);
    let centre = at(1, 1);
    assert_eq!(
        map.step_cost(centre, Dir::NW),
        Some(14),
        "grass: 10 × 14 / 10"
    );
    assert_eq!(
        map.step_cost(centre, Dir::SE),
        Some(14),
        "dirt: 10 × 14 / 10"
    );
    assert_eq!(
        map.step_cost(centre, Dir::SW),
        Some(35),
        "shallow water: 25 × 14 / 10"
    );
    assert_eq!(
        map.step_cost(at(0, 1), Dir::NE),
        Some(21),
        "sand: 15 × 14 / 10"
    );
}

#[test]
fn no_step_goes_onto_deep_water_or_rock_or_into_the_wall() {
    let map = draw(&[
        "=.", //
        ".#", //
    ]);
    assert_eq!(map.step_cost(at(1, 0), Dir::W), None, "onto deep water");
    assert_eq!(map.step_cost(at(0, 1), Dir::E), None, "onto rock");
    for dir in [Dir::N, Dir::NE, Dir::E] {
        assert_eq!(map.step_cost(at(1, 0), dir), None, "into the wall, {dir:?}");
    }
    for dir in [Dir::W, Dir::SW, Dir::S] {
        assert_eq!(map.step_cost(at(0, 1), dir), None, "into the wall, {dir:?}");
    }
}

#[test]
fn a_diagonal_step_needs_both_tiles_beside_it_walkable() {
    // A step to (2,1) from (1,0) passes the rock; from (1,2), the deep water.
    let map = draw(&[
        "..#", //
        "...", //
        ".:=", //
    ]);
    let centre = at(1, 1);
    let south_edge = draw(&[
        ".~.", //
        "~..", //
    ]);
    assert_eq!(map.step_cost(at(1, 0), Dir::SE), None, "cuts past the rock");
    assert_eq!(
        map.step_cost(at(1, 2), Dir::NE),
        None,
        "cuts past the deep water"
    );
    assert_eq!(map.step_cost(centre, Dir::NW), Some(14), "both sides grass");
    assert_eq!(
        map.step_cost(centre, Dir::SW),
        Some(14),
        "sides grass and sand"
    );
    assert_eq!(
        south_edge.step_cost(at(0, 0), Dir::SE),
        Some(14),
        "shallow water on both sides is walkable"
    );
}

fn try_draw(rows: &[&str]) -> Result<Map, MapError> {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    Map::from_ascii(rows, &data)
}

#[test]
fn a_drawing_with_rows_of_different_lengths_is_rejected() {
    assert_eq!(
        try_draw(&["...", "..", "..."]).unwrap_err(),
        MapError::RaggedRow { row: 1 }
    );
}

#[test]
fn a_drawing_with_a_character_outside_the_legend_is_rejected() {
    assert_eq!(
        try_draw(&["...", ".@."]).unwrap_err(),
        MapError::UnknownGlyph {
            glyph: '@',
            pos: at(1, 1)
        }
    );
}

#[test]
fn a_drawing_must_be_from_1_to_1024_tiles_on_each_side() {
    let wide = ".".repeat(1025);
    let tall = vec!["."; 1025];
    for rows in [vec![], vec![""], vec![wide.as_str()], tall] {
        let result = try_draw(&rows);
        assert!(
            matches!(result, Err(MapError::BadSize { .. })),
            "{} rows: {result:?}",
            rows.len()
        );
    }
    assert!(try_draw(&["."]).is_ok());
    let widest = ".".repeat(1024);
    assert!(try_draw(&[widest.as_str()]).is_ok());
}

#[test]
fn diagonal_costs_round_down_in_integer_maths() {
    // Sand at 19 makes a diagonal 19 × 14 / 10 = 26.6, which must round down to 26.
    let terrain =
        include_str!("../../../data/terrain.ron").replace("step_cost: 15", "step_cost: 19");
    let data = DataPack::from_sources(&[
        ("pack.ron", r#"(name: "test", version: "1")"#),
        ("terrain.ron", &terrain),
        ("chemicals.ron", include_str!("../../../data/chemicals.ron")),
        ("loci.ron", include_str!("../../../data/loci.ron")),
        ("objects.ron", include_str!("../../../data/objects.ron")),
    ])
    .expect("valid pack");
    let map = Map::from_ascii(&["..", ".:"], &data).expect("valid drawing");
    assert_eq!(map.step_cost(at(0, 0), Dir::SE), Some(26));
}
