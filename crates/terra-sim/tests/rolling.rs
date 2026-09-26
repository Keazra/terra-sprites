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
        &[(at(1, 1), &[ScriptedAction::Play { at: at(2, 1) }])],
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
        &[(at(3, 5), &[ScriptedAction::Play { at: at(3, 5) }])],
    );
    rolls(&mut world, 10);
    assert_eq!(ball(&world), at(3, 1));
}
