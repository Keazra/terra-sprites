//! Verbs aimed at a target (design §3.5, §5.5): the walk there, the one
//! attempt, and what the target's verb table does, driven through hand-made
//! worlds with scripted actions.

use terra_sim::{
    DataPack, DeathCause, EntityId, Event, EventKind, Genome, Map, ObjectView, Outcome, Pos,
    Progress, Removal, Scenario, ScriptedAction, Target, Verb, World,
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

#[test]
fn a_sprite_walks_to_its_target_and_bites_on_the_tick_it_arrives() {
    let bush = at(6, 1);
    let start = at(1, 1);
    let mut world = world(
        &["........", "........", "........"],
        &[(bush, "berry_bush")],
        start,
        &[ScriptedAction::Eat { at: bush }],
    );
    world
        .start_object(bush, "mature", &[("fruit", 3)])
        .expect("a bush to ripen");
    let id = the_sprite(&world);
    for _ in 0..20 {
        let events = world.step();
        let sprite = world.sprite(id).expect("the sprite");
        let action = sprite.action().expect("an action");
        assert_eq!(action.verb, Verb::Eat);
        assert_eq!(
            action.target,
            Some(Target::Object(object(&world, bush).id()))
        );
        if sprite.pos() == at(5, 1) {
            // The one tile beside the bush on the way: it bites as it gets there.
            assert_eq!(endings(&events), [(Verb::Eat, Outcome::Applied)]);
            assert_eq!(object(&world, bush).counter("fruit"), Some(2));
            return;
        }
        assert!(endings(&events).is_empty(), "{events:?}");
        assert!(
            matches!(action.progress, Progress::Walking { .. }),
            "{:?}",
            action.progress
        );
    }
    panic!("the sprite should reach the bush");
}

/// A world drawn from `rows`, with `objects`, and a walker on each tile of
/// `sprites` doing its script.
fn world_of(rows: &[&str], objects: &[(Pos, &str)], sprites: &[(Pos, &[ScriptedAction])]) -> World {
    let data = builtin();
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let walkers: Vec<(Pos, Option<Genome>)> = sprites
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

#[test]
fn a_target_that_is_gone_ends_the_action_as_failed() {
    // The near sprite eats the berry first; the far one loses its target.
    let berry = at(1, 1);
    let eat = [ScriptedAction::Eat { at: berry }];
    let mut world = world_of(
        &["........", "........", "........"],
        &[(berry, "berry")],
        &[(at(1, 1), &eat), (at(7, 1), &eat)],
    );
    let far = world.sprite_at(at(7, 1)).expect("the far sprite").id();
    world.step();
    assert!(world.object_at(berry).is_none(), "eaten");
    let events = world.step();
    let far_ended = events.iter().find_map(|e| match e.kind {
        EventKind::ActionEnded { id, verb, outcome } if id == far => Some((verb, outcome)),
        _ => None,
    });
    assert_eq!(far_ended, Some((Verb::Eat, Outcome::Failed)));
}

/// Whether `a` and `b` are different tiles that touch, at a side or a corner.
fn beside(a: Pos, b: Pos) -> bool {
    a != b && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}

#[test]
fn approaching_a_bush_ends_on_arrival_beside_it() {
    let bush = at(6, 1);
    let mut world = world(
        &["........", "........", "........"],
        &[(bush, "berry_bush")],
        at(1, 1),
        &[ScriptedAction::Approach { at: bush }],
    );
    let id = the_sprite(&world);
    for _ in 0..20 {
        let events = world.step();
        if !endings(&events).is_empty() {
            assert_eq!(endings(&events), [(Verb::Approach, Outcome::Applied)]);
            assert!(beside(world.sprite(id).expect("it").pos(), bush));
            return;
        }
    }
    panic!("the sprite should reach the bush");
}

#[test]
fn approaching_a_sprite_follows_it_as_it_moves() {
    // The leader wanders off along the top row; the follower sets off after it.
    let (leader_start, leader_end) = (at(3, 1), at(14, 1));
    let wander = [ScriptedAction::Wander {
        destination: leader_end,
    }];
    let approach = [ScriptedAction::Approach { at: leader_start }];
    let mut world = world_of(
        &[
            "................",
            "................",
            "................",
            "................",
        ],
        &[],
        &[(leader_start, &wander), (at(0, 3), &approach)],
    );
    let leader = world.sprite_at(leader_start).expect("the leader").id();
    let follower = world.sprite_at(at(0, 3)).expect("the follower").id();
    for _ in 0..60 {
        let events = world.step();
        let ended = events.iter().find_map(|e| match e.kind {
            EventKind::ActionEnded { id, verb, outcome } if id == follower => Some((verb, outcome)),
            _ => None,
        });
        if let Some(ended) = ended {
            assert_eq!(ended, (Verb::Approach, Outcome::Applied));
            let (follower, leader) = (world.sprite(follower), world.sprite(leader));
            let leader = leader.expect("the leader").pos();
            assert!(leader.x > 8, "the leader got well away: {leader:?}");
            // It reached the leader on its own turn; the leader may have
            // stepped on after that, in the same tick.
            let follower = follower.expect("the follower").pos();
            assert!(follower.x > 6, "it followed the leader: {follower:?}");
            assert!(
                follower
                    .x
                    .abs_diff(leader.x)
                    .max(follower.y.abs_diff(leader.y))
                    <= 2
            );
            return;
        }
    }
    panic!("the follower should catch the leader");
}
