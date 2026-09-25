//! Verbs aimed at a target (design §3.5, §5.5): the walk there, the one
//! attempt, and what the target's verb table does, driven through hand-made
//! worlds with scripted actions.

use terra_sim::{
    DataPack, DeathCause, EntityId, Event, EventKind, Genome, Map, ObjectView, Outcome, Pos,
    Removal, Scenario, ScriptedAction, Verb, World,
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

/// A world drawn from `rows`, with `objects`, and one walker on `sprite`
/// doing `script`.
fn world(rows: &[&str], objects: &[(Pos, &str)], sprite: Pos, script: &[ScriptedAction]) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites = [(sprite, Some(walker(&data)))];
    let scripted: Vec<(Pos, ScriptedAction)> = script.iter().map(|&s| (sprite, s)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// The object on `pos`.
fn object(world: &World, pos: Pos) -> ObjectView<'_> {
    world.object_at(pos).expect("an object there")
}

/// The one sprite's ID.
fn the_sprite(world: &World) -> EntityId {
    world.sprites().next().expect("a sprite").id()
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

#[test]
fn eating_from_a_bush_with_fruit_takes_one_and_fills_the_stomach() {
    // The sprite starts beside the bush, on a goal tile, so it bites at once.
    let bush = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(bush, "berry_bush")],
        at(1, 1),
        &[ScriptedAction::Eat { at: bush }],
    );
    world
        .start_object(bush, "mature", &[("fruit", 3)])
        .expect("a bush to ripen");
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Eat, Outcome::Applied)]);
    assert_eq!(object(&world, bush).counter("fruit"), Some(2));
    let sprite = world.sprite(the_sprite(&world)).expect("the sprite");
    assert_eq!(sprite.chemical("food"), Some(0.3));
}

#[test]
fn eating_from_a_bush_with_no_fruit_fails_and_changes_nothing() {
    let bush = at(2, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(bush, "berry_bush")],
        at(1, 1),
        &[ScriptedAction::Eat { at: bush }],
    );
    world
        .start_object(bush, "mature", &[])
        .expect("a bare bush");
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Eat, Outcome::Failed)]);
    assert_eq!(object(&world, bush).counter("fruit"), Some(0));
    let sprite = world.sprite(the_sprite(&world)).expect("the sprite");
    assert_eq!(sprite.chemical("food"), Some(0.0));
}

#[test]
fn eating_a_berry_removes_it() {
    // An item can be eaten from its own tile.
    let berry = at(1, 1);
    let mut world = world(
        &[".....", ".....", "....."],
        &[(berry, "berry")],
        berry,
        &[ScriptedAction::Eat { at: berry }],
    );
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Eat, Outcome::Applied)]);
    assert!(world.object_at(berry).is_none());
    let removed = events.iter().any(|e| {
        matches!(
            &e.kind,
            EventKind::ObjectRemoved { object_type, reason: Removal::Destroyed, .. }
                if object_type == "berry"
        )
    });
    assert!(removed, "{events:?}");
    let sprite = world.sprite(the_sprite(&world)).expect("the sprite");
    assert_eq!(sprite.chemical("food"), Some(0.3));
}

#[test]
fn drinking_from_shallow_water_fills_the_stomach_with_water() {
    let water = at(2, 1);
    let mut world = world(
        &[".....", "..~..", "....."],
        &[],
        at(1, 1),
        &[ScriptedAction::Drink { at: water }],
    );
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Drink, Outcome::Applied)]);
    let sprite = world.sprite(the_sprite(&world)).expect("the sprite");
    assert_eq!(sprite.chemical("water"), Some(0.2));
}

#[test]
fn an_aimed_action_with_nothing_to_aim_at_fails_at_once() {
    let mut world = world(
        &[".....", ".....", "....."],
        &[],
        at(1, 1),
        &[
            ScriptedAction::Eat { at: at(2, 1) },
            ScriptedAction::Drink { at: at(2, 1) },
        ],
    );
    let events = world.step();
    assert_eq!(endings(&events)[0], (Verb::Eat, Outcome::Failed));
}

#[test]
fn eating_a_thornbush_pricks_and_enough_bites_kill_hurt_by_thornbush() {
    let thornbush = at(2, 1);
    let bites = [ScriptedAction::Eat { at: thornbush }; 30];
    let mut world = world(
        &[".....", ".....", "....."],
        &[(thornbush, "thornbush")],
        at(1, 1),
        &bites,
    );
    let id = the_sprite(&world);
    let events = world.step();
    assert_eq!(endings(&events), [(Verb::Eat, Outcome::Applied)]);
    let injury = world.sprite(id).expect("the sprite").chemical("injury");
    assert_eq!(injury, Some(0.05));
    for _ in 1..30 {
        let events = world.step();
        let died = events.iter().find_map(|e| match e.kind {
            EventKind::Died { cause, .. } => Some(cause),
            _ => None,
        });
        if let Some(cause) = died {
            // Death check #2 (design §2.4): the bite that kills is this tick's.
            assert_eq!(endings(&events), [(Verb::Eat, Outcome::Applied)]);
            let thornbush_type = DeathCause::HurtBy(3);
            assert_eq!(cause, thornbush_type);
            assert_eq!(world.deaths(thornbush_type), 1);
            assert_eq!(world.data().object_type_name(3), Some("thornbush"));
            return;
        }
    }
    panic!("30 bites of .05 should kill");
}
