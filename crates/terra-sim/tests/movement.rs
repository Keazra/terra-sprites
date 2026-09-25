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
    seeded(rows, objects, speed, sprites, scripted, 1)
}

/// `world_with`, from the world seed `seed`.
fn seeded(
    rows: &[&str],
    objects: &[(Pos, &str)],
    speed: f32,
    sprites: &[Pos],
    scripted: &[(Pos, ScriptedAction)],
    seed: u64,
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
    World::from_scenario(scenario, data, seed).expect("a valid scenario")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// The ID of the sprite on `pos`.
fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
}

/// Steps `world` until sprite `id`'s first action ends, at most `ticks`
/// times, returning the tile it stands on after each tick and how the action
/// ended.
fn first_action(world: &mut World, id: EntityId, ticks: usize) -> (Vec<Pos>, Outcome) {
    let mut tiles = Vec::new();
    for _ in 0..ticks {
        let events = world.step();
        tiles.push(world.sprite(id).expect("still alive").pos());
        for event in events {
            if let EventKind::ActionEnded {
                id: ended, outcome, ..
            } = event.kind
                && ended == id
            {
                return (tiles, outcome);
            }
        }
    }
    panic!("the first action didn't end in {ticks} ticks: {tiles:?}");
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
    let (tiles, outcome) = first_action(&mut world, id, 30);
    assert_eq!(outcome, Outcome::Applied, "arrived");
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

/// The change over the last tick in the chemical called `name`, for sprite `id`.
fn change(world: &World, id: EntityId, name: &str) -> f32 {
    let sprite = world.sprite(id).expect("alive");
    let level = sprite.chemicals().find(|c| c.name == name);
    level.expect("a chemical").change
}

#[test]
fn a_rest_lasts_10_ticks_and_ends_applied() {
    let start = at(0, 0);
    let mut world = world(&["..."], 10.0, &[start], &[(start, ScriptedAction::Rest)]);
    let id = sprite_on(&world, start);
    let (_, tiles) = run(&mut world, id, 4);
    let progress = world
        .sprite(id)
        .expect("alive")
        .action()
        .expect("resting")
        .progress;
    assert_eq!(progress, Progress::Resting { ticks: 4, of: 10 });
    let (events, more) = run(&mut world, id, 6);
    let tiles = [tiles, more].concat();
    assert!(tiles.iter().all(|&pos| pos == start), "a rest stays put");
    let ended: Vec<(u64, Outcome)> = events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionEnded {
                verb: Verb::Rest,
                outcome,
                ..
            } => Some((e.tick, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(ended.first(), Some(&(9, Outcome::Applied)));
}

#[test]
fn walking_tires_a_sprite_and_resting_restores_it() {
    // 14 steps east, one a tick, then a rest.
    let start = at(0, 0);
    let wander = ScriptedAction::Wander {
        destination: at(14, 0),
    };
    let scripted = [(start, wander), (start, ScriptedAction::Rest)];
    let mut world = world(&["..............."], 10.0, &[start], &scripted);
    let id = sprite_on(&world, start);
    let (mut stamina, mut energy) = (Vec::new(), Vec::new());
    for _ in 0..25 {
        world.step();
        stamina.push(change(&world, id, "stamina"));
        energy.push(change(&world, id, "energy"));
    }
    // The body feels each tick's steps, and each tick of rest, on the next tick.
    assert!(stamina[1..=14].iter().all(|&c| c < 0.0), "{stamina:?}");
    assert!(stamina[15..=24].iter().all(|&c| c > 0.0), "{stamina:?}");
    assert!(
        energy[5] < energy[20],
        "walking costs more energy than resting: {energy:?}"
    );
}

#[test]
fn a_sprite_walks_around_another_when_that_costs_less_than_3_grass_steps_more() {
    // Straight through the resting sprite would cost 60 + 30; around it, 68.
    let (walker, rester) = (at(0, 1), at(3, 1));
    let wander = ScriptedAction::Wander {
        destination: at(6, 1),
    };
    let scripted = [(walker, wander), (rester, ScriptedAction::Rest)];
    let mut world = world(&["......."; 3], 10.0, &[walker, rester], &scripted);
    let id = sprite_on(&world, walker);
    let (tiles, outcome) = first_action(&mut world, id, 8);
    assert_eq!(outcome, Outcome::Applied, "arrived: {tiles:?}");
    assert!(!tiles.contains(&rester), "{tiles:?}");
}

/// Every `ActionStarted` and `ActionEnded` in `events`, as `(tick, verb, how
/// it ended)`, with `None` for a start.
fn action_events(events: &[Event]) -> Vec<(u64, Verb, Option<Outcome>)> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionStarted { verb, .. } => Some((e.tick, verb, None)),
            EventKind::ActionEnded { verb, outcome, .. } => Some((e.tick, verb, Some(outcome))),
            _ => None,
        })
        .collect()
}

#[test]
fn a_sprite_with_nothing_to_do_wanders_or_rests_of_its_own_accord() {
    let start = at(4, 4);
    let mut world = world(&["........."; 9], 7.0, &[start], &[]);
    let id = sprite_on(&world, start);
    let (events, _) = run(&mut world, id, 300);
    let started: Vec<Verb> = action_events(&events)
        .into_iter()
        .filter(|(_, _, ended)| ended.is_none())
        .map(|(_, verb, _)| verb)
        .collect();
    assert!(started.contains(&Verb::Wander), "{started:?}");
    assert!(started.contains(&Verb::Rest), "{started:?}");
    assert!(
        started
            .iter()
            .all(|v| matches!(v, Verb::Wander | Verb::Rest))
    );
}

#[test]
fn a_sprite_with_nowhere_to_go_gives_up_each_wander_at_once() {
    let start = at(0, 0);
    let mut world = world(&["."], 7.0, &[start], &[]);
    let id = sprite_on(&world, start);
    let (events, _) = run(&mut world, id, 100);
    let wanders: Vec<(u64, Verb, Option<Outcome>)> = action_events(&events)
        .into_iter()
        .filter(|(_, verb, _)| *verb == Verb::Wander)
        .collect();
    assert!(
        !wanders.is_empty(),
        "the stand-in chose to wander at some point"
    );
    for pair in wanders.chunks(2) {
        let [(started, _, None), (ended, _, Some(outcome))] = pair else {
            panic!("a start then an end: {pair:?}");
        };
        assert_eq!((ended, outcome), (started, &Outcome::Failed));
    }
}

fn wander_to(destination: Pos) -> ScriptedAction {
    ScriptedAction::Wander { destination }
}

#[test]
fn of_two_sprites_stepping_onto_one_tile_the_first_in_the_tick_s_shuffled_order_gets_it() {
    let (left, right, middle) = (at(0, 0), at(2, 0), at(1, 0));
    let scripted = [(left, wander_to(middle)), (right, wander_to(middle))];
    let mut winners = Vec::new();
    for seed in 1..=20 {
        let mut world = seeded(&["..."], &[], 10.0, &[left, right], &scripted, seed);
        let (a, b) = (sprite_on(&world, left), sprite_on(&world, right));
        world.step();
        assert_eq!(world.check_invariants(), Ok(()));
        let winner = sprite_on(&world, middle);
        let loser = if winner == a { b } else { a };
        let stayed = world.sprite(loser).expect("alive").pos();
        assert!(stayed == left || stayed == right, "the other waits");
        winners.push(winner == a);
    }
    assert!(
        winners.contains(&true) && winners.contains(&false),
        "{winners:?}"
    );
}

#[test]
fn two_sprites_meeting_head_on_in_a_one_tile_corridor_swap_and_pass() {
    let (west, east) = (at(0, 1), at(6, 1));
    let rows = ["#######", ".......", "#######"];
    let scripted = [(west, wander_to(east)), (east, wander_to(west))];
    let mut world = world(&rows, 10.0, &[west, east], &scripted);
    let (a, b) = (sprite_on(&world, west), sprite_on(&world, east));
    let mut arrived = Vec::new();
    for _ in 0..12 {
        for event in world.step() {
            if let EventKind::ActionEnded { id, outcome, .. } = event.kind {
                arrived.push((id, outcome));
            }
        }
        assert_eq!(world.check_invariants(), Ok(()));
        if arrived.len() == 2 {
            break;
        }
    }
    arrived.sort_by_key(|&(id, _)| id);
    assert_eq!(arrived, [(a, Outcome::Applied), (b, Outcome::Applied)]);
}

#[test]
fn two_sprites_swap_diagonally_when_neither_cuts_a_corner() {
    let (a_start, b_start) = (at(0, 0), at(1, 1));
    let scripted = [(a_start, wander_to(b_start)), (b_start, wander_to(a_start))];
    let mut world = world(&["..", ".."], 10.0, &[a_start, b_start], &scripted);
    let (a, b) = (sprite_on(&world, a_start), sprite_on(&world, b_start));
    world.step();
    world.step();
    assert_eq!(world.sprite(a).expect("alive").pos(), b_start);
    assert_eq!(world.sprite(b).expect("alive").pos(), a_start);
}

/// `n` rests in a row for the sprite on `pos`.
fn rests(pos: Pos, n: usize) -> Vec<(Pos, ScriptedAction)> {
    vec![(pos, ScriptedAction::Rest); n]
}

#[test]
fn a_sprite_that_can_find_no_way_past_gives_up_after_3_blocked_ticks() {
    let (walker, rester) = (at(0, 0), at(2, 0));
    let mut scripted = vec![(walker, wander_to(at(4, 0)))];
    scripted.extend(rests(rester, 5));
    let mut world = world(&["....."], 10.0, &[walker, rester], &scripted);
    let id = sprite_on(&world, walker);
    run(&mut world, id, 2);
    let progress = world
        .sprite(id)
        .expect("alive")
        .action()
        .expect("wandering")
        .progress;
    assert_eq!(progress, Progress::Waiting { blocked_ticks: 1 });
    let (events, _) = run(&mut world, id, 2);
    let ended: Vec<_> = action_events(&events)
        .into_iter()
        .filter(|(_, _, outcome)| outcome.is_some())
        .collect();
    assert_eq!(ended, [(3, Verb::Wander, Some(Outcome::Blocked))]);
}

/// Two long corridors joined at both ends, with an alcove above the top one
/// at (3, 0): the way along the bottom is short, the way round the top long.
const LOOP: [&str; 4] = ["###.####", "........", ".######.", "........"];

#[test]
fn a_stuck_sprite_keeps_to_the_way_round_it_found() {
    // The flood keeps pricing the way through the rester cheaper than the
    // way round, so without keeping to the way it found, it would turn back.
    let (walker, rester) = (at(0, 3), at(4, 3));
    let mut scripted = vec![(walker, wander_to(at(7, 3)))];
    scripted.extend(rests(rester, 10));
    let mut world = world(&LOOP, 10.0, &[walker, rester], &scripted);
    let id = sprite_on(&world, walker);
    let (tiles, outcome) = first_action(&mut world, id, 60);
    assert_eq!(outcome, Outcome::Applied, "{tiles:?}");
    assert!(tiles.contains(&at(4, 1)), "round the top: {tiles:?}");
    assert!(!tiles.contains(&rester), "{tiles:?}");
}

#[test]
fn a_sprite_blocked_again_on_its_way_round_searches_again() {
    // A second sprite rests in the alcove, then steps into the top corridor
    // and rests there, after the walker has set off round the top.
    let (walker, rester, lurker) = (at(0, 3), at(4, 3), at(3, 0));
    let mut scripted = vec![(walker, wander_to(at(7, 3)))];
    scripted.extend(rests(rester, 10));
    scripted.push((lurker, ScriptedAction::Rest));
    scripted.push((lurker, wander_to(at(3, 1))));
    scripted.extend(rests(lurker, 10));
    let mut world = world(&LOOP, 10.0, &[walker, rester, lurker], &scripted);
    let id = sprite_on(&world, walker);
    let (tiles, outcome) = first_action(&mut world, id, 60);
    assert_eq!(outcome, Outcome::Blocked, "{tiles:?}");
    assert!(
        tiles.contains(&at(1, 1)),
        "it set off round the top: {tiles:?}"
    );
    assert!(
        !tiles.contains(&at(3, 1)) && !tiles.contains(&rester),
        "{tiles:?}"
    );
}

#[test]
fn an_action_still_going_after_60_ticks_times_out() {
    // Speed 4 through shallow water: a step every 6 or 7 ticks.
    let start = at(0, 0);
    let row = ["~~~~~~~~~~~~~~~"];
    let mut world = world(&row, 4.0, &[start], &[(start, wander_to(at(14, 0)))]);
    let id = sprite_on(&world, start);
    let (events, _) = run(&mut world, id, 61);
    let ended: Vec<_> = action_events(&events)
        .into_iter()
        .filter(|(_, _, outcome)| outcome.is_some())
        .collect();
    assert_eq!(
        ended.first(),
        Some(&(60, Verb::Wander, Some(Outcome::TimedOut)))
    );
}
