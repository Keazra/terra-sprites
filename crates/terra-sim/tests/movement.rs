//! Sprites that move (design §3.6–3.7, §5.5): walking, conflicts and
//! re-planning, driven through hand-made worlds.

use terra_sim::{
    DataPack, EntityId, Event, EventKind, Genome, Map, Outcome, Pos, Progress, Scenario,
    ScriptedAction, Verb, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A genome with only traits: `speed`, and a sense radius of 10.
fn walker(speed: f32, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: {speed:?}),
            Trait(trait: "sense_radius", value: 10.0),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with walkers of `speed` on `sprites`, each
/// starting on the scripted action given for its tile, if any.
fn world(rows: &[&str], speed: f32, sprites: &[Pos], scripted: &[(Pos, ScriptedAction)]) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&pos| (pos, Some(walker(speed, &data))))
        .collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// The ID of the sprite on `pos`.
fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
}

/// Steps `world` `ticks` times, returning every event and the tile `id`
/// stands on after each tick.
fn run(world: &mut World, id: EntityId, ticks: usize) -> (Vec<Event>, Vec<Pos>) {
    let mut events = Vec::new();
    let mut tiles = Vec::new();
    for _ in 0..ticks {
        events.extend(world.step());
        tiles.push(world.sprite(id).expect("still alive").pos());
    }
    (events, tiles)
}

#[test]
fn a_sprite_told_to_wander_to_a_spot_walks_there_a_step_at_a_time_and_arrives() {
    // Speed 10 on grass: 100 tenths a tick, and a grass step costs 100.
    let wander = ScriptedAction::Wander {
        destination: at(5, 0),
    };
    let mut world = world(&["........"], 10.0, &[at(0, 0)], &[(at(0, 0), wander)]);
    let id = sprite_on(&world, at(0, 0));

    let (events, tiles) = run(&mut world, id, 5);

    assert_eq!(tiles, [at(1, 0), at(2, 0), at(3, 0), at(4, 0), at(5, 0)]);
    let actions: Vec<(u64, EventKind)> = events
        .into_iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::ActionStarted { .. } | EventKind::ActionEnded { .. }
            )
        })
        .map(|e| (e.tick, e.kind))
        .collect();
    assert_eq!(
        actions,
        [
            (
                0,
                EventKind::ActionStarted {
                    id,
                    verb: Verb::Wander
                }
            ),
            (
                4,
                EventKind::ActionEnded {
                    id,
                    verb: Verb::Wander,
                    outcome: Outcome::Applied
                }
            ),
        ]
    );
    let action = world
        .sprite(id)
        .expect("alive")
        .action()
        .expect("an action");
    assert_eq!(action.verb, Verb::Wander);
    assert_eq!(action.destination, Some(at(5, 0)));
    assert_eq!(action.progress, Progress::Ended(Outcome::Applied));
}
