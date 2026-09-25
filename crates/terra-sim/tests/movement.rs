//! Sprites that move (design §3.6–3.7, §5.5): walking, conflicts and
//! re-planning, driven through hand-made worlds.

use terra_sim::{
    DataPack, EntityId, Event, EventKind, Genome, Map, Outcome, Pos, Progress, Scenario,
    ScriptedAction, Verb, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A genome with only traits: `speed`, and a sense radius of 14, the most.
fn walker(speed: f32, data: &DataPack) -> Genome {
    seer(speed, 14.0, data)
}

/// A genome with only traits: `speed` and `sense_radius`.
fn seer(speed: f32, sense_radius: f32, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: {speed:?}),
            Trait(trait: "sense_radius", value: {sense_radius:?}),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with walkers of `speed` on `sprites`, each
/// starting on the scripted action given for its tile, if any.
fn world(rows: &[&str], speed: f32, sprites: &[Pos], scripted: &[(Pos, ScriptedAction)]) -> World {
    world_with(rows, &[], speed, sprites, scripted)
}

/// `world`, with objects as `(tile, object type)`.
fn world_with(
    rows: &[&str],
    objects: &[(Pos, &str)],
    speed: f32,
    sprites: &[Pos],
    scripted: &[(Pos, ScriptedAction)],
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&pos| (pos, Some(walker(speed, &data))))
        .collect();
    let scenario = Scenario {
        map,
        objects,
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

/// The tiles a lone walker of `speed` stands on after each of `ticks` ticks,
/// wandering east along a row of `rows` from (0, 0) to the row's far end.
fn walk_east(rows: &[&str], speed: f32, ticks: usize) -> Vec<u16> {
    let far = rows[0].chars().count() as u16 - 1;
    let wander = ScriptedAction::Wander {
        destination: at(far, 0),
    };
    let mut world = world(rows, speed, &[at(0, 0)], &[(at(0, 0), wander)]);
    let id = sprite_on(&world, at(0, 0));
    let (_, tiles) = run(&mut world, id, ticks);
    tiles.iter().map(|pos| pos.x).collect()
}

#[test]
fn speed_5_on_grass_is_a_step_every_other_tick() {
    assert_eq!(walk_east(&["........"], 5.0, 6), [0, 1, 1, 2, 2, 3]);
}

#[test]
fn speed_keeps_its_decimals() {
    // 74 tenths a tick against 70: after 15 ticks, 1,110 against 1,050.
    let row = ["..............."];
    assert_eq!(walk_east(&row, 7.4, 15)[14], 11);
    assert_eq!(walk_east(&row, 7.0, 15)[14], 10);
}

#[test]
fn a_step_costs_what_the_terrain_it_lands_on_costs() {
    // At 100 tenths a tick, a sand step (150) takes 1.5 ticks and a shallow
    // water step (250) 2.5.
    assert_eq!(walk_east(&[":::::::"], 10.0, 6), [0, 1, 2, 2, 3, 4]);
    assert_eq!(walk_east(&["~~~~~~~"], 10.0, 6), [0, 0, 1, 1, 2, 2]);
}

/// The tiles a lone walker of speed 10 steps through, from `from` to `to`,
/// on a world drawn from `rows` holding `objects`.
fn route(rows: &[&str], objects: &[(Pos, &str)], from: Pos, to: Pos) -> Vec<Pos> {
    let wander = ScriptedAction::Wander { destination: to };
    let mut world = world_with(rows, objects, 10.0, &[from], &[(from, wander)]);
    let id = sprite_on(&world, from);
    let (_, tiles) = run(&mut world, id, 30);
    let mut route: Vec<Pos> = Vec::new();
    for pos in tiles {
        if route.last() != Some(&pos) && pos != from {
            route.push(pos);
        }
    }
    route
}

#[test]
fn a_diagonal_step_costs_14_tenths_of_an_orthogonal_one() {
    let rows = ["...."; 4];
    let wander = ScriptedAction::Wander {
        destination: at(3, 3),
    };
    let mut world = world(&rows, 10.0, &[at(0, 0)], &[(at(0, 0), wander)]);
    let id = sprite_on(&world, at(0, 0));
    let (_, tiles) = run(&mut world, id, 5);
    // 140 tenths a step, at 100 a tick.
    assert_eq!(tiles, [at(0, 0), at(1, 1), at(2, 2), at(2, 2), at(3, 3)]);
}

#[test]
fn a_sprite_never_cuts_a_corner_past_rock() {
    let rows = [".#", ".."];
    assert_eq!(route(&rows, &[], at(0, 0), at(1, 1)), [at(0, 1), at(1, 1)]);
}

#[test]
fn a_sprite_never_steps_onto_a_bush_or_cuts_a_corner_past_one() {
    let rows = ["...", "...", "..."];
    let bush = [(at(1, 1), "thornbush")];
    let tiles = route(&rows, &bush, at(0, 1), at(2, 1));
    assert_eq!(tiles, [at(0, 0), at(1, 0), at(2, 0), at(2, 1)]);
    let tiles = route(&rows, &bush, at(0, 0), at(2, 2));
    assert!(!tiles.contains(&at(1, 1)), "{tiles:?}");
    assert_eq!(
        tiles.len(),
        4,
        "around the bush, not past its corners: {tiles:?}"
    );
}

/// The outcome of a lone walker's scripted wander to a spot `distance` tiles
/// east, with `sense_radius`, once it ends.
fn wander_east(distance: u16, sense_radius: f32) -> Outcome {
    let data = builtin();
    let row = ".".repeat(usize::from(distance) + 1);
    let map = Map::from_ascii(&[row.as_str()], &data).expect("valid drawing");
    let sprites = [(at(0, 0), Some(seer(10.0, sense_radius, &data)))];
    let wander = ScriptedAction::Wander {
        destination: at(distance, 0),
    };
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[(at(0, 0), wander)],
    };
    let mut world = World::from_scenario(scenario, data, 1).expect("a valid scenario");
    for _ in 0..30 {
        for event in world.step() {
            if let EventKind::ActionEnded { outcome, .. } = event.kind {
                return outcome;
            }
        }
    }
    panic!("the wander never ended");
}

#[test]
fn a_sprite_reaches_as_far_as_its_sense_radius_rounded_to_whole_tiles() {
    assert_eq!(
        wander_east(10, 9.6),
        Outcome::Applied,
        "9.6 reaches 10 tiles"
    );
    assert_eq!(wander_east(10, 9.4), Outcome::Failed, "9.4 reaches 9");
}
