//! Rolling items (design §3.5.4): a push sets an item rolling one tile a
//! tick, and it bounces off what it meets, driven through hand-made worlds
//! with scripted kicks.

use terra_sim::{DataPack, Genome, Map, Pos, Scenario, ScriptedAction, World};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A genome with only traits, so nothing but the script decides what it does.
fn walker(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `objects`, and walkers on the given tiles
/// doing their scripts.
fn world(rows: &[&str], objects: &[(Pos, &str)], sprites: &[(Pos, &[ScriptedAction])]) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let walkers: Vec<_> = sprites
        .iter()
        .map(|&(pos, _)| (pos, Some(walker(&data))))
        .collect();
    let scripted: Vec<(Pos, ScriptedAction)> = sprites
        .iter()
        .flat_map(|&(pos, script)| script.iter().map(move |&s| (pos, s)))
        .collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &walkers,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// A kick at the item on `pos`, then rest, so the kicker stays out of the way.
fn kick(pos: Pos) -> [ScriptedAction; 4] {
    [
        ScriptedAction::Play { at: pos },
        ScriptedAction::Rest,
        ScriptedAction::Rest,
        ScriptedAction::Rest,
    ]
}

/// Where the ball is.
fn ball(world: &World) -> Pos {
    world
        .objects()
        .find(|o| o.type_name() == "ball")
        .expect("a ball")
        .pos()
}

/// Where the ball is after each of the next `ticks` ticks.
fn rolls(world: &mut World, ticks: usize) -> Vec<Pos> {
    (0..ticks)
        .map(|_| {
            world.step();
            ball(world)
        })
        .collect()
}

const LANE: [&str; 3] = ["...........", "...........", "..........."];

#[test]
fn a_kicked_ball_rolls_four_tiles_away_from_the_kicker_one_a_tick() {
    // The kick lands on tick 1; the ball moves from tick 2.
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball")],
        &[(at(1, 1), &kick(at(2, 1)))],
    );
    let path = rolls(&mut world, 7);
    let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (6, 1), (6, 1), (6, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
}

const FIELD: [&str; 7] = [
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
    "...........",
];

#[test]
fn a_ball_kicked_from_its_own_tile_rolls_the_way_the_kicker_last_stepped() {
    let mut world = world(
        &FIELD,
        &[(at(3, 3), "ball")],
        &[(
            at(1, 3),
            &[
                ScriptedAction::Wander { destination: at(3, 3) },
                ScriptedAction::Play { at: at(3, 3) },
            ],
        )],
    );
    rolls(&mut world, 20);
    assert_eq!(ball(&world), at(7, 3));
}

#[test]
fn a_ball_kicked_from_its_own_tile_by_a_sprite_that_never_stepped_rolls_north() {
    let mut world = world(
        &FIELD,
        &[(at(3, 5), "ball")],
        &[(at(3, 5), &kick(at(3, 5)))],
    );
    rolls(&mut world, 10);
    assert_eq!(ball(&world), at(3, 1));
}

#[test]
fn a_ball_meeting_anything_head_on_bounces_straight_back() {
    // Kicked east from (2, 1), the ball reaches (5, 1) on tick 4. On tick 5
    // it meets what's on (6, 1) and comes back to (4, 1), its last tile.
    let ahead = at(6, 1);
    let cases: [(&str, [&str; 3], Option<&str>); 5] = [
        ("rock", ["...........", "......#....", "..........."], None),
        ("deep water", ["...........", "......=....", "..........."], None),
        ("the map's edge", ["......", "......", "......"], None),
        ("a bush", LANE, Some("thornbush")),
        ("another ball", LANE, Some("ball")),
    ];
    for (what, rows, object) in cases {
        let mut objects = vec![(at(2, 1), "ball")];
        objects.extend(object.map(|kind| (ahead, kind)));
        let mut world = world(
            &rows,
            &objects,
            &[(at(1, 1), &kick(at(2, 1)))],
        );
        let path = rolls(&mut world, 6);
        let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (4, 1), (4, 1)];
        assert_eq!(path, expected.map(|(x, y)| at(x, y)), "{what}");
    }
}

#[test]
fn a_ball_meeting_a_sprite_head_on_bounces_straight_back() {
    let rest = [ScriptedAction::Rest; 3];
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball")],
        &[
            (at(1, 1), &kick(at(2, 1))),
            (at(6, 1), &rest),
        ],
    );
    let path = rolls(&mut world, 6);
    let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (4, 1), (4, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
}
