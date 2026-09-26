//! Rolling items (design §3.5.4): a push sets an item rolling one tile a
//! tick, and it bounces off what it meets, driven through hand-made worlds
//! with scripted kicks.

use terra_sim::{
    DataPack, EventKind, Genome, Map, Outcome, Pos, Scenario, ScriptedAction, Verb, World,
};

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
    let mut world = world(&LANE, &[(at(2, 1), "ball")], &[(at(1, 1), &kick(at(2, 1)))]);
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
                ScriptedAction::Wander {
                    destination: at(3, 3),
                },
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
        (
            "deep water",
            ["...........", "......=....", "..........."],
            None,
        ),
        ("the map's edge", ["......", "......", "......"], None),
        ("a bush", LANE, Some("thornbush")),
        ("another ball", LANE, Some("ball")),
    ];
    for (what, rows, object) in cases {
        let mut objects = vec![(at(2, 1), "ball")];
        objects.extend(object.map(|kind| (ahead, kind)));
        let mut world = world(&rows, &objects, &[(at(1, 1), &kick(at(2, 1)))]);
        let path = rolls(&mut world, 6);
        let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (4, 1), (4, 1)];
        assert_eq!(path, expected.map(|(x, y)| at(x, y)), "{what}");
    }
}

#[test]
fn a_ball_meeting_a_sprite_head_on_bounces_straight_back_and_doesnt_hurt_it() {
    let rest = [ScriptedAction::Rest; 3];
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball")],
        &[(at(1, 1), &kick(at(2, 1))), (at(6, 1), &rest)],
    );
    let path = rolls(&mut world, 6);
    let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (4, 1), (4, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
    // No incidental harm (design §3.8): the ball bounced off it, and that's all.
    let stood = world
        .sprite_at(at(6, 1))
        .expect("the sprite it bounced off");
    assert_eq!(stood.chemical("injury"), Some(0.0));
}

/// The ball's path when kicked north-east from (1, 4) at a ball on (2, 3),
/// in a world drawn from `rows`.
fn kicked_north_east(rows: &[&str]) -> Vec<Pos> {
    let mut world = world(rows, &[(at(2, 3), "ball")], &[(at(1, 4), &kick(at(2, 3)))]);
    rolls(&mut world, 6)
}

#[test]
fn a_ball_meeting_a_wall_slantwise_glances_off_at_the_same_angle() {
    // North-east into a wall on its east: it goes north-west, then meets the
    // map's top edge and goes south-west.
    let path = kicked_north_east(&[
        ".....#.....",
        ".....#.....",
        ".....#.....",
        ".....#.....",
        "...........",
        "...........",
        "...........",
    ]);
    let expected = [(2, 3), (3, 2), (4, 1), (3, 0), (2, 1), (2, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
}

#[test]
fn a_ball_cant_cut_a_corner_so_it_glances_off_it() {
    // From (3, 2), (4, 1) is open, but the rock on (4, 2) is a corner the
    // ball may not cut, so it glances off to the north-west.
    let path = kicked_north_east(&[
        "...........",
        "...........",
        "....#......",
        "...........",
        "...........",
        "...........",
        "...........",
    ]);
    let expected = [(2, 3), (3, 2), (2, 1), (1, 0), (0, 1), (0, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
}

#[test]
fn a_ball_with_no_way_back_stops_for_good() {
    // Rock ahead and the kicker behind, so the roll ends on tick 2. The
    // kicker then walks off, and the ball stays where it is.
    let mut world = world(
        &["...........", "...#.......", "..........."],
        &[(at(2, 1), "ball")],
        &[(
            at(1, 1),
            &[
                ScriptedAction::Play { at: at(2, 1) },
                ScriptedAction::Wander {
                    destination: at(8, 2),
                },
                ScriptedAction::Rest,
            ],
        )],
    );
    let path = rolls(&mut world, 8);
    assert_eq!(path, [at(2, 1); 8]);
}

#[test]
fn kicking_a_rolling_ball_starts_a_fresh_roll() {
    // The first kick would stop the ball on (6, 1) after tick 5. On tick 2,
    // with the ball on (3, 1), the kicker steps up beside it and kicks
    // again, so it rolls 4 more from there.
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball")],
        &[(
            at(1, 1),
            &[
                ScriptedAction::Play { at: at(2, 1) },
                ScriptedAction::Play { at: at(3, 1) },
                ScriptedAction::Rest,
                ScriptedAction::Rest,
            ],
        )],
    );
    let path = rolls(&mut world, 8);
    let expected = [
        (2, 1),
        (3, 1),
        (4, 1),
        (5, 1),
        (6, 1),
        (7, 1),
        (7, 1),
        (7, 1),
    ];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
}

#[test]
fn a_sprite_going_to_kick_a_rolling_ball_follows_it_and_kicks_it() {
    // One sprite kicks the ball east along row 1. Another sets off for it
    // from (1, 6) as it goes, follows it to (6, 1), where it stops, and kicks.
    let mut world = world(
        &FIELD,
        &[(at(2, 1), "ball")],
        &[(at(1, 1), &kick(at(2, 1))), (at(1, 6), &kick(at(2, 1)))],
    );
    let chaser = world.sprite_at(at(1, 6)).expect("the chaser").id();
    let mut kicked = false;
    for _ in 0..30 {
        for event in world.step() {
            if let EventKind::ActionEnded {
                id,
                verb: Verb::Play,
                outcome,
                ..
            } = event.kind
                && id == chaser
            {
                assert_eq!(outcome, Outcome::Applied);
                kicked = true;
            }
        }
    }
    assert!(kicked, "the chaser kicked the ball");
    assert!(ball(&world).x > 6, "{:?}", ball(&world));
}
