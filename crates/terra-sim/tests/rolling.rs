//! Rolling items (design §3.5.4): a push sets an item rolling one tile a
//! tick, and it bounces off what it meets, driven through hand-made worlds
//! with scripted kicks.

use terra_sim::{
    DataPack, EventKind, Genome, Map, Outcome, Pos, Removal, Scenario, ScriptedAction, Verb, World,
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
    world_with(builtin(), rows, objects, sprites)
}

/// The built-in pack, with the object types `extra` added.
fn builtin_and(extra: &str) -> DataPack {
    let objects = DataPack::builtin_sources()
        .iter()
        .find(|&&(path, _)| path == "objects.ron")
        .expect("the built-in objects")
        .1;
    let body = objects.trim_end().strip_suffix(']').expect("a list");
    let body = body.trim_end().trim_end_matches(',');
    let objects = format!("{body}, {extra}]");
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            (
                path,
                if path == "objects.ron" {
                    &objects
                } else {
                    text
                },
            )
        })
        .collect();
    DataPack::from_sources(&sources).expect("a valid pack")
}

/// A world drawn from `rows` with the pack `data`, `objects`, and walkers on
/// the given tiles doing their scripts.
fn world_with(
    data: DataPack,
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[(Pos, &[ScriptedAction])],
) -> World {
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

/// The tiles of every object of type `kind`, in ID order.
fn tiles_of(world: &World, kind: &str) -> Vec<Pos> {
    world
        .objects()
        .filter(|o| o.type_name() == kind)
        .map(|o| o.pos())
        .collect()
}

/// Where each ball is after each of the next `ticks` ticks.
fn all_rolls(world: &mut World, ticks: usize) -> Vec<Vec<Pos>> {
    (0..ticks)
        .map(|_| {
            world.step();
            tiles_of(world, "ball")
        })
        .collect()
}

/// A kick at the item on `pos`, then a walk to `away`, out of the balls' way.
fn kick_and_leave(pos: Pos, away: Pos) -> [ScriptedAction; 3] {
    [
        ScriptedAction::Play { at: pos },
        ScriptedAction::Wander { destination: away },
        ScriptedAction::Rest,
    ]
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
                ScriptedAction::Rest,
                ScriptedAction::Rest,
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
    let cases: [(&str, [&str; 3], Option<&str>); 4] = [
        ("rock", ["...........", "......#....", "..........."], None),
        (
            "deep water",
            ["...........", "......=....", "..........."],
            None,
        ),
        ("the map's edge", ["......", "......", "......"], None),
        ("a bush", LANE, Some("thornbush")),
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

#[test]
fn a_sprite_on_a_way_round_follows_a_ball_that_rolls_off() {
    // The chaser heads north for the ball through the gap at (5, 2), where a
    // sprite rests. Held up there, it commits to the long way round, through
    // (10, 2). Meanwhile the ball is kicked west along row 0, away from where
    // the way round leads; the chaser drops it and follows the ball instead
    // (design §3.6).
    let rows = [
        "...........",
        "...........",
        "#####.####.",
        "...........",
        "...........",
    ];
    let wait = [ScriptedAction::Rest; 6];
    let mut late_kick = vec![ScriptedAction::Rest];
    late_kick.extend(kick(at(5, 0)));
    let mut world = world(
        &rows,
        &[(at(5, 0), "ball")],
        &[
            (at(5, 4), &kick(at(5, 0))),
            (at(5, 2), &wait),
            (at(6, 0), &late_kick),
        ],
    );
    let chaser = world.sprite_at(at(5, 4)).expect("the chaser").id();
    let mut ending = None;
    for _ in 0..70 {
        for event in world.step() {
            if let EventKind::ActionEnded {
                id,
                verb: Verb::Play,
                outcome,
                ..
            } = event.kind
                && id == chaser
                && ending.is_none()
            {
                ending = Some(outcome);
            }
        }
    }
    assert_eq!(
        ending,
        Some(Outcome::Applied),
        "the chaser caught up and kicked"
    );
}

/// Expected paths, one list of tiles per tick.
fn paths<const N: usize>(ticks: &[[(u16, u16); N]]) -> Vec<Vec<Pos>> {
    ticks
        .iter()
        .map(|tiles| tiles.iter().map(|&(x, y)| at(x, y)).collect())
        .collect()
}

#[test]
fn a_ball_rolling_into_a_ball_at_rest_stops_and_sends_it_on() {
    // A knock-on swaps rolls (design §3.5.4). The kick of 4 meets the other
    // ball on tick 3, after a tile, so it stops, and the other rolls on from
    // tick 4 with the 2 tiles left after the knock.
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball"), (at(4, 1), "ball")],
        &[(at(1, 1), &kick(at(2, 1)))],
    );
    let expected = paths(&[
        [(2, 1), (4, 1)],
        [(3, 1), (4, 1)],
        [(3, 1), (4, 1)],
        [(3, 1), (5, 1)],
        [(3, 1), (6, 1)],
        [(3, 1), (6, 1)],
    ]);
    assert_eq!(all_rolls(&mut world, 6), expected);
}

#[test]
fn a_ball_on_its_last_tile_only_nudges_a_ball_and_neither_moves() {
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball"), (at(6, 1), "ball")],
        &[(at(1, 1), &kick(at(2, 1)))],
    );
    let path = all_rolls(&mut world, 7);
    assert_eq!(path[3], [at(5, 1), at(6, 1)], "on its last tile");
    assert_eq!(path[6], [at(5, 1), at(6, 1)], "and there they stay");
}

#[test]
fn two_balls_meeting_head_on_both_bounce_back_with_each_other_s_distance() {
    // Both kicked on tick 1, towards each other. On tick 3 the first reaches
    // (4, 1) with 2 tiles to go, and the second, with 3, meets it: they swap,
    // so each goes back 2.
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball"), (at(6, 1), "ball")],
        &[
            (at(1, 1), &kick_and_leave(at(2, 1), at(0, 2))),
            (at(7, 1), &kick_and_leave(at(6, 1), at(8, 2))),
        ],
    );
    let expected = paths(&[
        [(2, 1), (6, 1)],
        [(3, 1), (5, 1)],
        [(4, 1), (5, 1)],
        [(3, 1), (6, 1)],
        [(2, 1), (7, 1)],
        [(2, 1), (7, 1)],
    ]);
    assert_eq!(all_rolls(&mut world, 6), expected);
}

#[test]
fn a_ball_crushes_a_berry_in_its_way_and_rolls_on() {
    let mut world = world(
        &LANE,
        &[(at(2, 1), "ball"), (at(4, 1), "berry")],
        &[(at(1, 1), &kick(at(2, 1)))],
    );
    let mut crushed = Vec::new();
    let mut path = Vec::new();
    for step in 1..=6 {
        for event in world.step() {
            if let EventKind::ObjectRemoved {
                object_type,
                reason,
                ..
            } = event.kind
            {
                crushed.push((step, object_type, reason));
            }
        }
        path.push(ball(&world));
    }
    let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (6, 1), (6, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
    assert_eq!(crushed, [(3, "berry".to_string(), Removal::Destroyed)]);
    assert!(tiles_of(&world, "berry").is_empty());
}

/// An item smaller than a ball, and harder.
const PEBBLE: &str = r#"(id: 5, name: "pebble", category: Berry, size: Small, hardness: 0.9)"#;

/// An item as big as a sprite, which nothing fixes to the ground.
const CRATE: &str = r#"(id: 6, name: "crate", category: Ball, size: Large, hardness: 0.5)"#;

#[test]
fn a_ball_knocks_on_a_smaller_harder_item_rather_than_crushing_it() {
    let mut world = world_with(
        builtin_and(PEBBLE),
        &LANE,
        &[(at(2, 1), "ball"), (at(4, 1), "pebble")],
        &[(at(1, 1), &kick(at(2, 1)))],
    );
    for _ in 0..6 {
        world.step();
    }
    assert_eq!(ball(&world), at(3, 1));
    assert_eq!(tiles_of(&world, "pebble"), [at(6, 1)]);
}

#[test]
fn a_ball_bounces_off_a_bigger_item() {
    let mut world = world_with(
        builtin_and(CRATE),
        &LANE,
        &[(at(2, 1), "ball"), (at(6, 1), "crate")],
        &[(at(1, 1), &kick(at(2, 1)))],
    );
    let path = rolls(&mut world, 6);
    let expected = [(2, 1), (3, 1), (4, 1), (5, 1), (4, 1), (4, 1)];
    assert_eq!(path, expected.map(|(x, y)| at(x, y)));
    assert_eq!(tiles_of(&world, "crate"), [at(6, 1)]);
}

#[test]
fn a_ball_knocked_on_never_moves_twice_in_a_tick() {
    // The first ball, kicked east, meets the second on tick 2 as the second
    // starts rolling south. They swap. The second, later in ID order, then
    // waits for tick 3 before it heads east (design §3.5.4).
    let mut world = world(
        &FIELD,
        &[(at(3, 1), "ball"), (at(4, 1), "ball")],
        &[(at(2, 1), &kick(at(3, 1))), (at(4, 0), &kick(at(4, 1)))],
    );
    let path = all_rolls(&mut world, 3);
    assert_eq!(path[1], [at(3, 1), at(4, 1)], "tick 2: the knock");
    assert_eq!(
        path[2],
        [at(3, 2), at(5, 1)],
        "tick 3: each rolls the other's way"
    );
}

#[test]
fn after_a_bounce_a_ball_crushes_or_knocks_on_what_it_meets() {
    // The slantwise case: the ball glances off the wall at (4, 1) towards
    // (3, 0), where something waits.
    let rows = [
        ".....#.....",
        ".....#.....",
        ".....#.....",
        ".....#.....",
        "...........",
        "...........",
        "...........",
    ];
    let mut crush = world(
        &rows,
        &[(at(2, 3), "ball"), (at(3, 0), "berry")],
        &[(at(1, 4), &kick(at(2, 3)))],
    );
    let expected = [(2, 3), (3, 2), (4, 1), (3, 0), (2, 1), (2, 1)];
    assert_eq!(rolls(&mut crush, 6), expected.map(|(x, y)| at(x, y)));
    assert!(tiles_of(&crush, "berry").is_empty());

    let mut knock = world(
        &rows,
        &[(at(2, 3), "ball"), (at(3, 0), "ball")],
        &[(at(1, 4), &kick(at(2, 3)))],
    );
    let path = all_rolls(&mut knock, 6);
    assert_eq!(path[3], [at(4, 1), at(3, 0)], "tick 4: the knock");
    // The other ball goes north-west with 1 tile, and glances off the map's edge.
    assert_eq!(path[5], [at(4, 1), at(2, 1)]);
}

#[test]
fn a_ball_beside_a_diagonal_is_not_a_wall_for_a_ball_glancing_off() {
    // Kicked north-east, the ball meets rock at (4, 1) on tick 3. The side to
    // its east holds a ball, which it would knock on, not bounce off, so only
    // the corner stops it: head on, it comes straight back.
    let rows = [
        "...........",
        "....#......",
        "...........",
        "...........",
        "...........",
        "...........",
        "...........",
    ];
    let mut world = world(
        &rows,
        &[(at(2, 3), "ball"), (at(4, 2), "ball")],
        &[(at(1, 4), &kick_and_leave(at(2, 3), at(0, 6)))],
    );
    let path = all_rolls(&mut world, 3);
    assert_eq!(path[2], [at(2, 3), at(4, 2)]);
}

/// A small, soft item that rolls when played with.
const PIP: &str = r#"(id: 5, name: "pip", category: Berry, size: Small, hardness: 0.1,
    verbs: { Play: [Push(4)] })"#;

#[test]
fn a_rolling_item_can_be_crushed_before_its_turn() {
    // The pip starts rolling south as the ball, earlier in ID order, arrives
    // from the west and crushes it.
    let mut world = world_with(
        builtin_and(PIP),
        &FIELD,
        &[(at(3, 1), "ball"), (at(4, 1), "pip")],
        &[(at(2, 1), &kick(at(3, 1))), (at(4, 0), &kick(at(4, 1)))],
    );
    world.step();
    world.step();
    assert!(tiles_of(&world, "pip").is_empty());
    assert_eq!(ball(&world), at(4, 1));
}
