//! Learning by watching (M2 design §18): a sprite takes on a share of what
//! the sprite its eye is on knows about what it uses.

mod common;
use common::{WorldBuilder, at, builtin, genome, id_at, value_of};

use terra_sim::{
    Command, EntityId, Event, EventKind, Learned, Pos, ScriptedAction, Thing, Verb, World,
};

/// A room with water along its west wall.
const ROOM: [&str; 3] = ["~.........", "~.........", "~........."];

/// A thirsty sprite whose drink halves its thirst, as the starter's does.
const THIRSTY: &str = r#"InitialConcentration(chem: "thirst", value: 1.0),
    Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),"#;

/// The same, that drinks for its thirst: thirst is a drink's motive (design
/// §5.5).
const DRINKER: &str = r#"InitialConcentration(chem: "thirst", value: 1.0),
    Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
    Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),"#;

/// A sprite whose eye is always on the sprite it sees, never on anything
/// new, and that rests, taking on `watch_rate` of what it watches.
fn looking_on(watch_rate: f32) -> String {
    looking_at("sprite", watch_rate)
}

/// The same, its eye always on what it sees of `category`.
fn looking_at(category: &str, watch_rate: f32) -> String {
    format!(
        r#"AttentionInstinct(input: "always", category: "{category}", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
        BrainParam(param: "curiosity", value: 0.0),
        BrainParam(param: "tau_att_base", value: 0.05),
        BrainParam(param: "tau_base", value: 0.02),
        BrainParam(param: "watch_rate", value: {watch_rate:?}),"#
    )
}

/// A world of `ROOM` with a sprite of its own genes on each tile, all grown
/// up, those on `scripted` doing what they're given.
fn room(sprites: &[(Pos, &str)], scripted: &[(Pos, ScriptedAction)]) -> World {
    room_with(&[], sprites, scripted)
}

/// The same, with `objects`.
fn room_with(
    objects: &[(Pos, &str)],
    sprites: &[(Pos, &str)],
    scripted: &[(Pos, ScriptedAction)],
) -> World {
    let data = builtin();
    let sprites = sprites
        .iter()
        .map(|&(pos, genes)| (pos, Some(genome(genes, &data))))
        .collect();
    WorldBuilder::new(&ROOM)
        .data(data)
        .objects(objects)
        .sprites_with(sprites)
        .scripts(scripted)
        .build()
}

/// Water's worth for thirst.
fn water_for_thirst() -> Learned {
    Learned::Worth {
        thing: "water".into(),
        need: Some("thirst".into()),
    }
}

/// Water's worth for hunger.
fn water_for_hunger() -> Learned {
    Learned::Worth {
        thing: "water".into(),
        need: Some("hunger".into()),
    }
}

/// Steps `world` to the step 4 after the sprite `drinker`'s first drink,
/// where it learns what water is worth for thirst, and returns the events.
fn first_drink(world: &mut World, drinker: EntityId) -> Vec<Event> {
    let mut events = Vec::new();
    while value_of(world, drinker, &water_for_thirst()) == 0.0 {
        assert!(world.tick() < 20, "it never drank");
        events.extend(world.step());
    }
    events
}

#[test]
fn a_sprite_takes_on_a_share_of_what_the_sprite_it_watches_knows_about_what_it_uses() {
    // M2 design §18.2: the drinker's thirst falls by .5 at its drink, so it
    // learns water is worth .25 for thirst (.5 × worth_rate_good .5); the
    // sprite watching it takes on half of that.
    let looking_on = looking_on(0.5);
    let mut world = room(&[(at(1, 1), DRINKER), (at(5, 1), &looking_on)], &[]);
    let drinker = id_at(&world, at(1, 1));
    let watching = id_at(&world, at(5, 1));
    first_drink(&mut world, drinker);
    assert_eq!(value_of(&world, drinker, &water_for_thirst()), 0.25);
    assert_eq!(value_of(&world, watching, &water_for_thirst()), 0.125);
}

#[test]
fn a_sprite_takes_on_a_thing_s_worth_only_for_the_need_the_one_it_watches_used_it_for() {
    // M2 design §18.2: a drink that eases hunger as well teaches the
    // drinker water is worth .25 for each, but it drank for its thirst, so
    // the sprite watching takes on only water's worth for thirst. Nothing
    // chose a scripted drink, which shows no worth at all.
    let quenches = format!(
        r#"{DRINKER}
        InitialConcentration(chem: "hunger", value: 1.0),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "hunger"),"#
    );
    let looking_on = looking_on(0.5);
    let mut world = room(&[(at(1, 1), &quenches), (at(5, 1), &looking_on)], &[]);
    let drinker = id_at(&world, at(1, 1));
    let watching = id_at(&world, at(5, 1));
    first_drink(&mut world, drinker);
    assert_eq!(value_of(&world, drinker, &water_for_hunger()), 0.25);
    assert_eq!(value_of(&world, watching, &water_for_thirst()), 0.125);
    assert_eq!(value_of(&world, watching, &water_for_hunger()), 0.0);

    let mut world = room(
        &[(at(1, 1), THIRSTY), (at(5, 1), &looking_on)],
        &[(at(1, 1), ScriptedAction::Drink { at: at(0, 1) })],
    );
    let drinker = id_at(&world, at(1, 1));
    let watching = id_at(&world, at(5, 1));
    first_drink(&mut world, drinker);
    assert_eq!(value_of(&world, watching, &water_for_thirst()), 0.0);
}

/// Every lesson in `events`, as `(sprite, learned, good, whom it watched)`.
fn lessons(events: &[Event]) -> Vec<(EntityId, Learned, bool, Option<EntityId>)> {
    events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::LearnedMilestone {
                id,
                learned,
                good,
                watching,
            } => Some((*id, learned.clone(), *good, *watching)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_lesson_watching_tips_says_whom_it_watched() {
    // M2 design §18.4: a drinker that learns fast learns water is worth .5
    // for thirst at one drink, a lesson; a sprite that takes on all it
    // watches learns the same, from watching it.
    let drinker = format!(
        r#"{DRINKER}
        BrainParam(param: "worth_rate_good", value: 1.0),"#
    );
    let looking_on = looking_on(1.0);
    let mut world = room(&[(at(1, 1), &drinker), (at(5, 1), &looking_on)], &[]);
    let drinker = id_at(&world, at(1, 1));
    let watching = id_at(&world, at(5, 1));
    let mut learned = lessons(&first_drink(&mut world, drinker));
    // The drinker learns new things are good too, at the same drink.
    learned.retain(|(_, l, ..)| *l == water_for_thirst());
    assert_eq!(
        learned,
        [
            (drinker, water_for_thirst(), true, None),
            (watching, water_for_thirst(), true, Some(drinker)),
        ]
    );
}

/// Water's worth for thirst to the sprite `watching` at the start, once a
/// thirsty sprite on (1, 1) has drunk at the water beside it, in a room
/// with `objects`.
fn watched_drinking(objects: &[(Pos, &str)], watching: (Pos, &str)) -> f32 {
    let mut world = room_with(objects, &[(at(1, 1), DRINKER), watching], &[]);
    let drinker = id_at(&world, at(1, 1));
    let id = id_at(&world, watching.0);
    first_drink(&mut world, drinker);
    value_of(&world, id, &water_for_thirst())
}

#[test]
fn a_sprite_learns_from_one_using_the_kind_of_thing_its_eye_is_on() {
    // M2 design §18.1: its eye on the water, not the drinker, it still
    // takes on half of what the drinker knows of water.
    let on_water = looking_at("water", 0.5);
    assert_eq!(watched_drinking(&[], (at(5, 1), &on_water)), 0.125);
}

#[test]
fn a_sprite_whose_eye_is_elsewhere_or_that_doesn_t_watch_learns_nothing_by_watching() {
    // M2 design §18.1: its eye on a ball, it doesn't watch the drinker; and
    // with a watch_rate of 0, the default, it takes nothing on.
    let on_ball = looking_at("toy", 0.5);
    let ball = [(at(6, 2), "ball")];
    assert_eq!(watched_drinking(&ball, (at(5, 1), &on_ball)), 0.0);
    let not_watching = looking_on(0.0);
    assert_eq!(watched_drinking(&[], (at(5, 1), &not_watching)), 0.0);
}

#[test]
fn a_sprite_that_watches_another_eased_at_water_remembers_the_place() {
    // M2 design §18.2: as if it had drunk there itself (§7).
    let looking_on = looking_on(0.5);
    let mut world = room(
        &[(at(1, 1), THIRSTY), (at(5, 1), &looking_on)],
        &[
            (at(1, 1), ScriptedAction::Drink { at: at(0, 1) }),
            (at(1, 1), ScriptedAction::Rest),
        ],
    );
    let id = id_at(&world, at(5, 1));
    for _ in 0..5 {
        world.step();
    }
    let places: Vec<(Thing, Pos)> = world
        .sprite(id)
        .expect("the sprite watching")
        .remembered_places()
        .into_iter()
        .map(|place| (place.thing, place.at))
        .collect();
    assert_eq!(places, [("water".into(), at(0, 1))]);
}

/// Feels every hit as a punishment of 1, and rests.
const HIT_HURTS: &str = r#"
    Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
    Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
"#;

#[test]
fn a_sprite_that_watches_another_hit_learns_to_fear_the_hitter() {
    // M2 design §18.2 (Q4, #106): the one hit learns to fear the bully at
    // once, -1 (fear_rate 1 × punishment 1); the sprite watching it takes
    // on half, and more while the hit is felt, a lesson from watching it.
    let (me, victim, bully) = (at(4, 1), at(6, 1), at(8, 1));
    let looking_on = looking_on(0.5);
    let mut world = room(
        &[(me, &looking_on), (victim, HIT_HURTS), (bully, HIT_HURTS)],
        &[
            (bully, ScriptedAction::Hit { at: victim }),
            (bully, ScriptedAction::Rest),
        ],
    );
    let (me, victim, bully) = (
        id_at(&world, me),
        id_at(&world, victim),
        id_at(&world, bully),
    );
    let mut learned = Vec::new();
    for _ in 0..8 {
        learned.extend(lessons(&world.step()));
    }
    let bully_frightening = Learned::Fear {
        thing: Thing::Sprite(bully),
    };
    assert_eq!(value_of(&world, victim, &bully_frightening), -1.0);
    let mine = value_of(&world, me, &bully_frightening);
    assert!((-1.0..=-0.5).contains(&mine), "{mine}");
    assert!(
        learned.contains(&(me, bully_frightening, false, Some(victim))),
        "{learned:?}"
    );
}

#[test]
fn a_pet_for_what_the_watched_sprite_tried_passes_on_as_a_taught_habit() {
    // M2 design §18.2 (Q3, Q4): the kicker kicks the ball at tick 0 and is
    // petted at tick 6, so balls are good to it, .25 (.5 × worth_rate_good
    // .5), and so is kicking them, .45 (.5 × taught_rate .9). The sprite
    // watching it takes on half of each.
    let (kicker, ball, me) = (at(2, 1), at(3, 1), at(6, 1));
    let looking_on = looking_on(0.5);
    let mut script = vec![(kicker, ScriptedAction::Play { at: ball })];
    script.extend([(kicker, ScriptedAction::Rest); 5]);
    let mut world = room_with(
        &[(ball, "ball")],
        &[(kicker, ""), (me, &looking_on)],
        &script,
    );
    let (kicker, me) = (id_at(&world, kicker), id_at(&world, me));
    while world.tick() < 6 {
        world.step();
    }
    world.submit(Command::Reward {
        sprite: kicker,
        amplified: false,
        reach_back: 10,
    });
    world.step();
    let balls_good = Learned::Worth {
        thing: "ball".into(),
        need: None,
    };
    let kicking_balls = Learned::Habit {
        thing: "ball".into(),
        verb: Verb::Play,
    };
    assert_eq!(value_of(&world, kicker, &balls_good), 0.25);
    assert_eq!(value_of(&world, kicker, &kicking_balls), 0.45);
    assert_eq!(value_of(&world, me, &balls_good), 0.125);
    assert_eq!(value_of(&world, me, &kicking_balls), 0.225);
}

/// Where a young sprite looks with an attention instinct about grown-ups of
/// `weight`, a young one beside it and a grown one 5 tiles off: the tile
/// it attends to, the young one's and the grown one's.
fn young_looks(weight: f32) -> (Option<Pos>, Pos, Pos) {
    let (me, young, grown) = (at(1, 1), at(2, 1), at(6, 1));
    let genes = format!(
        r#"AttentionInstinct(input: "always", category: "sprite", whom: Grown, weight: {weight:?}),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
        BrainParam(param: "curiosity", value: 0.0),
        BrainParam(param: "tau_att_base", value: 0.05),"#
    );
    let rester = r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#;
    let data = builtin();
    let sprites = vec![
        (me, Some(genome(&genes, &data))),
        (young, Some(genome(rester, &data))),
        (grown, Some(genome(rester, &data))),
    ];
    let mut world = WorldBuilder::new(&ROOM)
        .data(data)
        .sprites_with(sprites)
        .newborns(&[me, young])
        .build();
    world.step();
    let sprite = world.sprite_at(me).expect("the young sprite");
    (sprite.attending_to(), young, grown)
}

#[test]
fn an_attention_instinct_about_grown_ups_draws_a_young_sprite_s_eye_to_them() {
    // M2 design §18.1: to the grown one before the nearer young one, or,
    // negative, away from it; it does nothing for the young one.
    let (attends, _, grown) = young_looks(1.0);
    assert_eq!(attends, Some(grown));
    let (attends, young, _) = young_looks(-1.0);
    assert_eq!(attends, Some(young));
}

#[test]
fn learning_rate_mod_scales_what_a_sprite_takes_in_by_watching_but_plasticity_doesn_t() {
    // M2 design §18.1: watch_rate is a share rather than a rate. At a
    // plasticity of 1.5 a sprite still takes on half of the drinker's .25;
    // at a learning_rate_mod of .5 it takes on a quarter.
    let watching = looking_on(0.5);
    let plastic = format!(r#"{watching}BrainParam(param: "plasticity", value: 1.5),"#);
    assert_eq!(watched_drinking(&[], (at(5, 1), &plastic)), 0.125);
    let slow = format!(
        r#"{watching}InitialConcentration(chem: "h3", value: 1.0),
        Receptor(chem: "h3", gain: -0.5, target: "learning_rate_mod"),"#
    );
    assert_eq!(watched_drinking(&[], (at(5, 1), &slow)), 0.0625);
}

#[test]
fn a_sprite_heard_calling_out_of_sight_is_not_watched() {
    // M2 design §18.1: the drinker calls, then drinks, 24 tiles from a
    // sprite that heard it, out of its sight, whose eye is on it.
    let field = format!("~{}", ".".repeat(29));
    let rows = [field.as_str(); 3];
    let calls_then_drinks = r#"
        Emitter(locus: Locus("always"), mode: Level, gain: 0.1, chem: "thirst"),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
        Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Call, weight: 0.5),
        BrainParam(param: "tau_base", value: 0.02),"#;
    let looking_on = looking_on(1.0);
    let data = builtin();
    let sprites = vec![
        (at(1, 1), Some(genome(calls_then_drinks, &data))),
        (at(25, 1), Some(genome(&looking_on, &data))),
    ];
    let mut world = WorldBuilder::new(&rows)
        .data(data)
        .sprites_with(sprites)
        .build();
    let drinker = id_at(&world, at(1, 1));
    let watching = id_at(&world, at(25, 1));
    first_drink(&mut world, drinker);
    let sprite = world.sprite(watching).expect("the sprite watching");
    assert_eq!(sprite.attending_to(), Some(at(1, 1)), "its eye is on it");
    assert!(value_of(&world, drinker, &water_for_thirst()) > 0.0);
    assert_eq!(value_of(&world, watching, &water_for_thirst()), 0.0);
}

/// `DRINKER`, its eye always on the water, with `genes` as well.
fn drinker_looking_at_water(genes: &str) -> String {
    format!(
        r#"{DRINKER}
        AttentionInstinct(input: "always", category: "water", weight: 1.0),
        BrainParam(param: "curiosity", value: 0.0),
        BrainParam(param: "tau_att_base", value: 0.05),
        {genes}"#
    )
}

#[test]
fn a_sprite_watches_several_in_one_tick() {
    // M2 design §18.1: its eye on the water, it watches two sprites drink at
    // once, and moves half way towards each one's view in turn: to .125,
    // then to .1875.
    let on_water = looking_at("water", 0.5);
    let drinker = drinker_looking_at_water("");
    let mut world = room(
        &[
            (at(1, 0), &drinker),
            (at(1, 2), &drinker),
            (at(5, 1), &on_water),
        ],
        &[],
    );
    let (one, other) = (id_at(&world, at(1, 0)), id_at(&world, at(1, 2)));
    let watching = id_at(&world, at(5, 1));
    first_drink(&mut world, one);
    assert_eq!(
        value_of(&world, other, &water_for_thirst()),
        0.25,
        "both drank"
    );
    assert_eq!(value_of(&world, watching, &water_for_thirst()), 0.1875);
}

#[test]
fn nothing_passes_through_two_sprites_in_one_tick() {
    // M2 design §18.2: the first drinker learns fast, .5 from its drink. The
    // second, its eye on the water, learns .25 from its own and takes on
    // all of the first's, .5. The third, its eye on the second, takes on
    // all the second knew before anyone took anything on: .25.
    let fast = drinker_looking_at_water(r#"BrainParam(param: "worth_rate_good", value: 1.0),"#);
    let second = drinker_looking_at_water(r#"BrainParam(param: "watch_rate", value: 1.0),"#);
    let third = looking_on(1.0);
    let mut world = room(
        &[(at(1, 0), &fast), (at(1, 2), &second), (at(2, 2), &third)],
        &[],
    );
    let ids = [at(1, 0), at(1, 2), at(2, 2)].map(|pos| id_at(&world, pos));
    first_drink(&mut world, ids[0]);
    let views = ids.map(|id| value_of(&world, id, &water_for_thirst()));
    assert_eq!(views, [0.5, 0.5, 0.25]);
}

#[test]
fn a_sprite_that_watches_the_cursor_correct_another_learns_to_fear_the_cursor() {
    // M2 design §18.2: if it sees the Cursor shock her, to be wary of the
    // Cursor. The one shocked sees the Cursor on it, and learns to fear it;
    // the sprite watching it takes on half.
    let (me, other) = (at(4, 1), at(7, 1));
    let looking_on = looking_on(0.5);
    let rests = r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#;
    let mut world = room(&[(me, &looking_on), (other, rests)], &[]);
    let (me, shocked) = (id_at(&world, me), id_at(&world, other));
    world.submit(Command::MoveCursor { tile: other });
    world.submit(Command::ShowCursor { visible: true });
    world.step();
    world.submit(Command::Correct {
        sprite: shocked,
        amplified: true,
    });
    world.step();
    let fears_the_cursor = Learned::Fear {
        thing: Thing::Cursor,
    };
    let theirs = value_of(&world, shocked, &fears_the_cursor);
    assert!(theirs < 0.0, "{theirs}");
    let mine = value_of(&world, me, &fears_the_cursor);
    assert!(
        (mine - theirs / 2.0).abs() < 1e-6,
        "{mine} against {theirs}"
    );
}
