//! Retreat (design §3.7, §5.5): backing away from a target, straight away
//! from it, a step at a time, until the bout is done or the sprite is
//! cornered. Driven through hand-made worlds with scripted actions.

use terra_sim::{DataPack, Genome, Map, Pos, Scenario, ScriptedAction, World};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A genome with only traits: speed 10, a grass step a tick.
fn walker(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `objects`, and one walker on `sprite`
/// retreating from what's on `from`.
fn retreating(rows: &[&str], objects: &[(Pos, &str)], sprite: Pos, from: Pos) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(sprite, Some(walker(&data)))];
    let scripted = [(sprite, ScriptedAction::Retreat { at: from })];
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// Where the one sprite stands.
fn where_is(world: &World) -> Pos {
    world.sprites().next().expect("a sprite").pos()
}

#[test]
fn a_retreat_steps_straight_away_from_a_threat_due_north() {
    // South, south-east and south-west all gain a tile of distance; south
    // points most directly away.
    let rows = [".....", ".....", ".....", ".....", "....."];
    let mut world = retreating(&rows, &[(at(2, 1), "thornbush")], at(2, 2), at(2, 1));
    world.step();
    assert_eq!(where_is(&world), at(2, 3));
}

#[test]
fn a_retreat_from_a_threat_to_the_north_east_steps_south_west() {
    let rows = [".....", ".....", ".....", ".....", "....."];
    let mut world = retreating(&rows, &[(at(3, 1), "thornbush")], at(2, 2), at(3, 1));
    // A diagonal step costs 14, so it takes a second tick's points.
    world.step();
    world.step();
    assert_eq!(where_is(&world), at(1, 3));
}

#[test]
fn with_the_straight_away_step_walled_off_it_takes_the_next_most_direct() {
    // Rock where it would step south-west. South and west gain as much and
    // point as directly away; the direction order breaks the tie, S before W.
    let rows = [".....", ".....", ".....", ".#...", "....."];
    let mut world = retreating(&rows, &[(at(3, 1), "thornbush")], at(2, 2), at(3, 1));
    world.step();
    assert_eq!(where_is(&world), at(2, 3));
}

#[test]
fn a_retreat_may_step_onto_an_item() {
    let rows = [".....", ".....", ".....", ".....", "....."];
    let objects = [(at(2, 1), "thornbush"), (at(2, 3), "berry")];
    let mut world = retreating(&rows, &objects, at(2, 2), at(2, 1));
    world.step();
    assert_eq!(where_is(&world), at(2, 3));
}
