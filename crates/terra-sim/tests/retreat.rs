//! Retreat (design §3.7, §5.5): backing away from a target, straight away
//! from it, a step at a time, until the bout is done or the sprite is
//! cornered. Driven through hand-made worlds with scripted actions.

use terra_sim::{
    DataPack, Event, EventKind, Genome, Map, Outcome, Pos, Scenario, ScriptedAction, Verb, World,
};

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

/// A walker that feels being cornered: the `cornered` pulse raises `h0`.
fn feeling_cornered(data: &DataPack) -> Genome {
    let text = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        Trait(trait: "sense_radius", value: 10.0),
        Emitter(locus: Locus("cornered"), mode: Level, gain: 1.0, chem: "h0"),
    ])"#;
    Genome::from_ron(text, data).expect("a valid genome")
}

/// A world drawn from `rows`, with `objects`, and one walker on `sprite`
/// retreating from what's on `from`.
fn retreating(rows: &[&str], objects: &[(Pos, &str)], sprite: Pos, from: Pos) -> World {
    retreating_with(rows, objects, sprite, from, walker)
}

/// `retreating`, with the sprite's genome made by `genome`.
fn retreating_with(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprite: Pos,
    from: Pos,
    genome: fn(&DataPack) -> Genome,
) -> World {
    let script = [(sprite, ScriptedAction::Retreat { at: from })];
    scene(rows, objects, &[(sprite, genome)], &script)
}

/// A world drawn from `rows`, with `objects`, and `sprites` with genomes
/// made as given, each starting on the scripted actions given for its tile.
fn scene(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[(Pos, fn(&DataPack) -> Genome)],
    scripted: &[(Pos, ScriptedAction)],
) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&(pos, genome)| (pos, Some(genome(&data))))
        .collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// The `ActionEnded` events in `events`, as `(verb, outcome)`.
fn endings(events: &[Event]) -> Vec<(Verb, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionEnded { verb, outcome, .. } => Some((verb, outcome)),
            _ => None,
        })
        .collect()
}

/// Where the one sprite stands, or the first placed, if there are more.
fn where_is(world: &World) -> Pos {
    world.sprites().next().expect("a sprite").pos()
}

/// The first sprite placed's level of `chemical`.
fn level(world: &World, chemical: &str) -> f32 {
    let sprite = world.sprites().next().expect("a sprite");
    sprite.chemical(chemical).expect("a known chemical")
}

/// The `ActionEnded` events of the first sprite placed.
fn first_endings(world: &World, events: &[Event]) -> Vec<(Verb, Outcome)> {
    let first = world.sprites().next().expect("a sprite").id();
    let own: Vec<Event> = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::ActionEnded { id, .. } if id == first))
        .cloned()
        .collect();
    endings(&own)
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

#[test]
fn a_retreat_is_done_after_six_steps_at_the_sprite_s_own_speed() {
    let rows = ["............"];
    let mut world = retreating(&rows, &[(at(0, 0), "thornbush")], at(1, 0), at(0, 0));
    // Speed 10 takes a grass step a tick.
    for tick in 1..6 {
        let events = world.step();
        assert_eq!(endings(&events), [], "still backing away at tick {tick}");
        assert_eq!(where_is(&world), at(1 + tick, 0));
    }
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Retreat, Outcome::Applied)]);
    assert_eq!(where_is(&world), at(7, 0));
}

#[test]
fn cornered_by_the_map_alone_a_retreat_ends_at_once_and_the_sprite_feels_it() {
    // The berry is beside it, and the map's edge is behind it.
    let rows = ["..."];
    let mut world = retreating_with(&rows, &[(at(1, 0), "berry")], at(0, 0), at(1, 0), feeling_cornered);
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Retreat, Outcome::Blocked)]);
    assert_eq!(where_is(&world), at(0, 0));
    assert_eq!(level(&world, "h0"), 0.0, "a pulse goes live at the next tick");
    world.step();
    assert_eq!(level(&world, "h0"), 1.0, "cornered");
}

#[test]
fn a_retreat_that_steps_away_is_not_cornered() {
    let rows = ["....."];
    let mut world = retreating_with(&rows, &[(at(0, 0), "berry")], at(1, 0), at(0, 0), feeling_cornered);
    world.step();
    world.step();
    assert_eq!(level(&world, "h0"), 0.0);
}

#[test]
fn held_up_by_a_sprite_a_retreat_waits_then_is_cornered_after_three_ticks() {
    // The only step away is onto a sprite resting there.
    let (me, other) = (at(1, 0), at(2, 0));
    let script = [
        (me, ScriptedAction::Retreat { at: at(0, 0) }),
        (other, ScriptedAction::Rest),
    ];
    let mut world = scene(
        &["...."],
        &[(at(0, 0), "berry")],
        &[(me, feeling_cornered), (other, walker)],
        &script,
    );
    for tick in 1..3 {
        let events = world.step();
        assert_eq!(first_endings(&world, &events), [], "waiting at tick {tick}");
    }
    let events = world.step();
    assert_eq!(first_endings(&world, &events), [(Verb::Retreat, Outcome::Blocked)]);
    assert_eq!(where_is(&world), me);
    world.step();
    assert_eq!(level(&world, "h0"), 1.0, "cornered");
}

#[test]
fn a_retreat_held_up_by_a_sprite_goes_on_once_it_moves_off() {
    let (me, other) = (at(1, 0), at(2, 0));
    let script = [
        (me, ScriptedAction::Retreat { at: at(0, 0) }),
        (other, ScriptedAction::Wander { destination: at(5, 0) }),
    ];
    let mut world = scene(
        &["......"],
        &[(at(0, 0), "berry")],
        &[(me, feeling_cornered), (other, walker)],
        &script,
    );
    world.step();
    world.step();
    assert!(where_is(&world).x >= 2, "it followed the other sprite out");
    world.step();
    assert_eq!(level(&world, "h0"), 0.0, "never cornered");
}
