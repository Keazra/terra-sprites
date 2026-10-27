//! The inspector's tabs as drawn (design §6.1): the Body, Brain, Chem,
//! Genome and World tabs, their title, and how a long tab scrolls.

mod common;
use common::screen::{
    COPY_AND_CHANGE, HUNGRY_GENOME, SPEED_10, a_lone_egg, app_for, garden, garden_with_sprites,
    generated_world, inside, inspector, inspector_on, log_rows, one_sprite, one_sprite_among,
    one_sprite_doing, one_sprite_in, pointer_on, render,
};
use common::{
    LETHAL_THORNS, WorldBuilder, at, died, finished, group, inspector_line, inspector_rows, lines,
    open, open_tab, pack, pack_changing, run, select,
};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use terra_sim::{
    Command, CopyChangeView, DataPack, DeathCause, EntityId, Event, EventKind, GeneView, Outcome,
    Pos, ScriptedAction, Verb, World,
};
use terra_tui::app::{App, CursorMode, Selection, Tab};
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

/// The lines the World tab starts with: the small population graph's title,
/// its six rows and axis, and a blank line (M2 design §14).
const GRAPH_LINES: usize = 9;

/// The right-hand `width` columns of each screen row, trimmed.
fn right_part(buffer: &Buffer, width: u16) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let row: String = (buffer.area.width - width..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            row.trim_end().to_string()
        })
        .collect()
}

#[test]
fn with_room_the_world_tab_shows_the_pack_and_every_object_types_numbers() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let inspector = right_part(&render(&app, &world, 100, 30), 45);
    assert_eq!(
        inspector[1], "               Body Brain Chem Genome  World",
        "with nothing selected, the title is just the tabs, at its right"
    );
    assert_eq!(open_tab(&app, &world), "World");
    let body: Vec<&str> = inspector[2 + GRAPH_LINES..12 + GRAPH_LINES]
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(
        body,
        [
            " data pack   core v1",
            " sprites           0",
            " deaths            0",
            "   starvation 0 · dehydration 0 · old age 0",
            " berry bush        1",
            "   seedling 1 · mature 0",
            "   fruit 0",
            " berry             1",
            " thornbush         1",
            " ball              1",
        ]
    );
}

/// Where in `text` the first row that reads `heading` is.
fn row_of(text: &[String], heading: &str) -> usize {
    text.iter()
        .position(|row| row == heading)
        .unwrap_or_else(|| panic!("no {heading:?} in {text:?}"))
}

#[test]
fn the_sprite_tabs_say_how_to_select_a_sprite_when_none_is() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    for (tab, title) in [
        ("Body", " Body  Brain Chem Genome World"),
        ("Brain", "Body  Brain  Chem Genome World"),
        ("Chem", "Body Brain  Chem  Genome World"),
        ("Genome", "Body Brain Chem  Genome  World"),
    ] {
        app.apply(Action::NextTab, &world);
        let (top, text) = inspector(&app, &world);
        // The open tab is in reverse video, as wide as brackets would be
        // (design §6.1).
        assert_eq!(top, format!("{title:>45}"), "{tab}");
        assert_eq!(open_tab(&app, &world), tab);
        assert_eq!(
            text[0], "No sprite selected: click one, or press Tab",
            "{tab}"
        );
        assert!(text[1..].iter().all(String::is_empty), "{tab}: {text:?}");
    }
}

#[test]
fn the_title_names_the_selected_sprite_left_of_the_tabs() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let label = format!(
        "Sprite #{}",
        world.sprites().next().expect("a sprite").id().0
    );
    let (top, _) = inspector(&app, &world);
    assert_eq!(top, format!(" {label:<14} Body  Brain Chem Genome World"));
    app.apply(Action::PreviousTab, &world);
    let (top, _) = inspector(&app, &world);
    assert_eq!(top, format!(" {label:<14}Body Brain Chem Genome  World"));
}

#[test]
fn selecting_a_sprite_leaves_the_tabs_where_they_were() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    open(&mut app, &world, Tab::Body);
    let (before, _) = inspector(&app, &world);
    app.apply(Action::SelectNext, &world);
    let (after, _) = inspector(&app, &world);
    assert_eq!(before.chars().count(), 45, "{before:?}");
    assert_eq!(
        after.chars().skip(15).collect::<String>(),
        before.chars().skip(15).collect::<String>(),
        "{before:?} then {after:?}"
    );
}

#[test]
fn a_name_too_long_for_its_number_shows_alone_or_shortened() {
    for (name, shown) in [
        ("Mira", "Mira #"),
        ("WWWWWWWWA", "WWWWWWWWA "),
        ("Abcdefghijklmnop", "Abcdefghij... "),
    ] {
        let mut world = garden_with_sprites();
        let id = world.sprites().next().expect("a sprite").id();
        world.submit(Command::Rename {
            sprite: id,
            name: name.into(),
        });
        run(&mut world, 1);
        let mut app = app_for(&world, Theme::cp437(), 100, 30);
        app.record(&[], &world);
        app.apply(Action::SelectNext, &world);
        let (top, _) = inspector(&app, &world);
        assert!(top.starts_with(&format!(" {shown}")), "{name}: {top:?}");
        assert!(
            top.ends_with(" Body  Brain Chem Genome World"),
            "{name}: {top:?}"
        );
    }
}

#[test]
fn a_click_on_a_tab_opens_it() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    // The title row is the screen's second, right of the inspector's left
    // column at 54.
    let click = |app: &mut App, column: usize| {
        app.apply(
            Action::left_click(Position::new(55 + column as u16, 1)),
            &world,
        );
    };
    for tab in ["Genome", "Chem", "World", "Brain", "Body"] {
        let (top, _) = inspector(&app, &world);
        let column = top.find(&format!(" {tab}")).expect(tab) + 1;
        click(&mut app, column + tab.len() - 1);
        assert_eq!(open_tab(&app, &world), tab, "{top:?}");
    }
    let (top, _) = inspector(&app, &world);
    click(&mut app, top.find("Chem").expect("Chem") - 1);
    assert_eq!(open_tab(&app, &world), "Body", "between two tabs");
    click(&mut app, 2);
    assert_eq!(open_tab(&app, &world), "Body", "on the sprite's label");
    assert!(app.selection().is_some(), "{:?}", app.selection());
}

#[test]
fn when_the_selected_sprite_dies_its_tabs_say_how_and_at_what_age() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    app.record(&[died(4_012, id, DeathCause::Dehydration, 4_012)], &world);
    for tab in ["Body", "Brain", "Chem", "Genome"] {
        let (top, text) = inspector(&app, &world);
        assert!(
            top.starts_with(&format!(" Sprite #{}  ", id.0)),
            "{tab}: {top:?}"
        );
        assert_eq!(
            text[0],
            format!("Sprite #{} died of dehydration at age 4,012", id.0),
            "{tab}"
        );
        app.apply(Action::NextTab, &world);
    }
}

#[test]
fn a_sprite_hurt_to_death_by_an_object_is_named_after_its_type() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    // The thornbush is object type 3.
    app.record(&[died(4_012, id, DeathCause::HurtBy(3), 4_012)], &world);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..2].join(" "),
        format!("Sprite #{} died hurt by thornbush at age 4,012", id.0)
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert_eq!(
        log_rows(&screen)[0],
        format!(
            "4,012  Sprite #{} died (hurt by thornbush, age 4,012)",
            id.0
        )
    );
}

/// Traits, and drives set so one tick changes them in ways worked out by
/// hand: hunger halves every tick, and boredom gains .1 a tick.
const WORKED_GENOME: &str = r#"(format: 1, genes: [
    Trait(trait: "speed", value: 7.25),
    Trait(trait: "sense_radius", value: 9.5),
    Trait(trait: "grows_old", value: 13500.0),
    InitialConcentration(chem: "hunger", value: 0.8),
    HalfLife(chem: "hunger", ticks: 1),
    InitialConcentration(chem: "thirst", value: 0.18),
    InitialConcentration(chem: "tiredness", value: 0.39),
    InitialConcentration(chem: "boredom", value: 0.2),
    Emitter(locus: Locus("always"), mode: Level, gain: 0.1, chem: "boredom"),
])"#;

#[test]
fn the_body_tab_shows_age_life_stage_traits_drives_physical_levels_and_wear() {
    let (world, app) = one_sprite_doing(WORKED_GENOME, &[ScriptedAction::Rest], 1);
    let (_, text) = inspector_on(&app, &world, 32);
    assert_eq!(
        text[..19],
        [
            "Resting · 9 ticks left",
            "age 7,501 · adult · grows old 13,500",
            "speed 7.25 · sense 9.5",
            // Its sex, and fed and watered, fertile (M2 design §10).
            "male · fertile",
            // Where it came from, and its generation (M2 design §17.5).
            "one of the first sprites · generation 1",
            "",
            "hunger      ████░░░░░░ .40 ▼",
            "thirst      ██░░░░░░░░ .18",
            "pain        ░░░░░░░░░░ .00",
            "tiredness   ████░░░░░░ .39",
            "boredom     ███░░░░░░░ .30 ▲",
            "loneliness  ░░░░░░░░░░ .00",
            "crowdedness ░░░░░░░░░░ .00",
            "arousal     ░░░░░░░░░░ .00",
            "anxiety     ░░░░░░░░░░ .00",
            "",
            // A newborn's are full, and one tick at rest takes them only to .9998 or so.
            "energy 1.00 · hydration 1.00 · stamina 1.00",
            "food .00 · water .00 · injury .00",
            "wear .00",
        ]
    );
    assert_eq!(text[19..22], ["", "Observed", "nothing yet"]);
    assert!(text[22..].iter().all(String::is_empty), "{text:?}");
}

#[test]
fn the_body_tab_says_when_the_young_grow_up_and_the_old_are_old() {
    // M2 design §9.6: `age … · young · grows up 4,500`, then `· adult ·
    // grows old …`, then `· old`. In this pack a sprite may grow old as
    // soon as it likes.
    let pack = pack_changing(&[(
        "physiology.ron",
        &[
            ("grows_old: (6000.0, 60000.0)", "grows_old: (10.0, 60000.0)"),
            ("grows_up: (1500.0, 13500.0)", "grows_up: (10.0, 13500.0)"),
        ],
    )]);
    let genes = |grows_up: f32, grows_old: f32| {
        format!(
            r#"(format: 1, genes: [
                Trait(trait: "grows_up", value: {grows_up:?}),
                Trait(trait: "grows_old", value: {grows_old:?}),
            ])"#
        )
    };
    let (young, grown) = (at(2, 3), at(6, 1));
    let mut world = WorldBuilder::grass(10, 5)
        .genes(&[
            (young, &genes(4500.0, 13500.0)),
            (grown, &genes(10.0, 20.0)),
        ])
        .newborns(&[young])
        .data(pack)
        .seed(7)
        .build();
    world.step();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        inspector_line(&app, &world, "age "),
        "age 1 · young · grows up 4,500"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        inspector_line(&app, &world, "age "),
        "age 11 · adult · grows old 20"
    );
    run(&mut world, 10);
    assert_eq!(inspector_line(&app, &world, "age "), "age 21 · old");
}

/// Changes after which water runs out at once and thirst injures badly,
/// and a bad injury scars a gene every tick, by .38 of its health (M2
/// design §9.4).
const SCARRING: common::Changes = (
    "physiology.ron",
    &[
        ("hydration_loss: 0.00033", "hydration_loss: 1.0"),
        ("dehydration: 0.0011", "dehydration: 0.3"),
        ("chance: 0.002, hit: 0.05", "chance: 1.0, hit: 0.38"),
    ],
);

#[test]
fn a_worn_sprite_grows_old_sooner_and_the_body_tab_says_its_genes_health() {
    // M2 design §9.4, §9.6. In this pack water runs out at once and thirst
    // injures badly, which wears the sprite, and a bad injury scars a gene
    // every tick: once one has weakened, the wear line says the genes'
    // average health too. Its one gene that weakens, the emitter, is whole
    // to begin with, as the wear line says: a scenario's sprite carries no
    // flaws (M2 design §12.3). The first scar takes .38 of its health.
    let genome = r#"(format: 1, genes: [
        Trait(trait: "grows_old", value: 13500.0),
        Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom"),
    ])"#;
    let (mut world, app) = one_sprite_in(pack_changing(&[SCARRING]), genome, &[], &[], 0);
    assert_eq!(
        inspector_line(&app, &world, "age"),
        "age 7,500 · adult · grows old 13,500"
    );
    assert_eq!(
        inspector_line(&app, &world, "wear"),
        "wear .00",
        "no gene weakened yet"
    );
    let vigour = |world: &World| world.sprites().next().expect("a sprite").vigour();
    while vigour(&world) == 1.0 {
        world.step();
    }
    let sprite = world.sprites().next().expect("a sprite");
    let grows_old = sprite.grows_old_at();
    assert!(grows_old < 13_499.5, "worn sooner");
    let said = format!(
        "age {} · adult · grows old {}",
        group(sprite.age()),
        group(grows_old.round() as u64)
    );
    assert_eq!(inspector_line(&app, &world, "age"), said);
    assert_eq!(inspector_line(&app, &world, "wear"), "wear .00 · genes .62");
}

#[test]
fn the_body_tab_starts_with_what_the_sprite_is_doing_in_plain_words() {
    let wander = ScriptedAction::Wander {
        destination: at(7, 3),
    };
    let (world, app) = one_sprite_doing(SPEED_10, &[wander], 1);
    assert_eq!(
        inspector(&app, &world).1[0],
        "Wandering off · 4 tiles to go"
    );
    let (world, app) = one_sprite_doing(SPEED_10, &[wander], 5);
    assert_eq!(inspector(&app, &world).1[0], "Arrived");
}

#[test]
fn a_sprite_with_nothing_to_do_shows_how_its_action_ended_then_what_it_watches() {
    // M2 design §19.2: the ending shows on the tick the action ended. From
    // the next, with nothing pulling as hard as the act threshold, it
    // stands, its eye on the bush beside its way.
    let genome = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        BrainParam(param: "act_threshold", value: 0.25),
    ])"#;
    let wander = ScriptedAction::Wander {
        destination: at(7, 3),
    };
    let bush = at(5, 1);
    let (mut world, mut app) = one_sprite_among(genome, &[(bush, "berry_bush")], &[wander], 5);
    assert_eq!(inspector(&app, &world).1[0], "Arrived");
    world.step();
    assert_eq!(inspector(&app, &world).1[0], "Watching the berry bush");
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(text.iter().any(|row| row == "DECISION: none"), "{text:?}");
    open(&mut app, &world, Tab::Body);
    app.apply(Action::ToggleDetail, &world);
    let id = world.object_at(bush).expect("the bush").id();
    assert_eq!(
        inspector(&app, &world).1[0],
        format!("STAND → berry_bush #{}", id.0)
    );
}

#[test]
fn the_body_tab_says_a_led_sprite_is_being_led_and_how_far_behind() {
    // Design v23 §6.1. It walks a grass step a tick, from (2, 3).
    let (mut world, mut app) = one_sprite_doing(SPEED_10, &[], 0);
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.submit(Command::MoveCursor { tile: at(7, 3) });
    world.step();
    assert_eq!(inspector(&app, &world).1[0], "Being led · 4 tiles behind");
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(
        inspector(&app, &world).1[0],
        "LED → (7,3) · walking (4 tiles)"
    );
    run(&mut world, 4);
    assert_eq!(inspector(&app, &world).1[0], "LED → (7,3)");
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(inspector(&app, &world).1[0], "Being led", "caught up");
}

#[test]
fn a_led_sprite_as_close_as_it_can_get_to_the_cursor_has_caught_up() {
    // Design v23 §6.1: the Cursor is on a bush, which it can't stand on.
    let bush = at(7, 3);
    let (mut world, app) = one_sprite_among(SPEED_10, &[(bush, "berry_bush")], &[], 0);
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.submit(Command::MoveCursor { tile: bush });
    run(&mut world, 6);
    assert_eq!(world.sprite(sprite).expect("it").pos(), at(6, 3));
    assert_eq!(inspector(&app, &world).1[0], "Being led");
}

#[test]
fn the_brain_tab_shows_attention_scores_and_what_adds_most_to_the_decision() {
    let objects = [(at(8, 3), "berry_bush"), (at(6, 1), "berry")];
    let (world, mut app) = one_sprite_among(HUNGRY_GENOME, &objects, &[], 1);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(open_tab(&app, &world), "Brain");
    // Attention: hunger's 1 × .8 on the berry, plus salience .5 × (1 − its
    // distance, .34); the bush has salience alone, .5 × (1 − .5).
    // The decision: hunger .8 × 1, .8 × (1 − 0) × .5 and always 1 × −.1;
    // not target adjacent's .004 rounds to nothing, so it's left out.
    assert_eq!(
        text[..8],
        [
            "ATTENTION",
            "► berry                                1.13",
            "berry bush                            .25",
            "",
            "DECISION: EAT                          1.10",
            "hunger                               +.80",
            "hunger & not target adjacent         +.40",
            "always                               -.10",
        ]
    );
    assert!(text[8..].iter().all(String::is_empty), "{text:?}");
}

/// A hungry sprite drawn to a berry and to eating it, whose eating halves
/// its hunger, as the starter genome's does.
const BERRY_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 1.0),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "fruit", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
])"#;

/// Eating a berry eases all six needs, so berries have six lessons.
const EVERY_NEED_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 1.0),
    InitialConcentration(chem: "thirst", value: 1.0),
    InitialConcentration(chem: "tiredness", value: 1.0),
    InitialConcentration(chem: "boredom", value: 1.0),
    InitialConcentration(chem: "loneliness", value: 1.0),
    InitialConcentration(chem: "crowdedness", value: 1.0),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "thirst"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "tiredness"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "boredom"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "loneliness"),
    Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "crowdedness"),
    BrainParam(param: "habit_rate", value: 0.0),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "fruit", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
])"#;

#[test]
fn the_brain_tab_shows_what_the_sprite_has_learned_as_memory() {
    // It eats the berry beside it at tick 0; at tick 1 its hunger falls by
    // .5, and it learns berries are good for hunger: .5 × .5.
    let objects = [(at(3, 3), "berry")];
    let (world, mut app) = one_sprite_among(BERRY_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let at = row_of(&text, "BERRIES");
    assert_eq!(text[at - 1], "", "a blank row before it");
    assert!(
        text[at + 1].starts_with("good for hunger") && text[at + 1].ends_with("+.25"),
        "{:?}",
        text[at + 1]
    );
}

/// The inspector's text, keeping the indent the Brain tab uses.
fn brain_lines(app: &App, world: &World) -> Vec<String> {
    inspector_rows(app, world, 30).1
}

/// The Brain tab's lines, keeping their indent, for `world`'s first sprite
/// once `world` has run `ticks` ticks.
fn brain_after(mut world: World, ticks: u64) -> Vec<String> {
    run(&mut world, ticks);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    brain_lines(&app, &world)
}

/// A left click on the inspector row that contains `needle`.
fn click_brain_row(app: &mut App, world: &World, needle: &str) {
    let at = brain_cell(app, world, needle);
    app.apply(Action::left_click(at), world);
}

/// The screen cell of the inspector row that contains `needle`.
fn brain_cell(app: &App, world: &World, needle: &str) -> Position {
    let rows = brain_lines(app, world);
    let row = rows
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle} not in {rows:?}"));
    // The inspector's text starts one cell in from its left border, on the
    // row under the title. On a 100-wide screen the border is at column 54.
    Position::new(55, 2 + row as u16)
}

#[test]
fn the_brain_tab_opens_the_rest_of_a_things_lessons() {
    // Six lessons about berries, all the same size, so the old list of five
    // would have dropped the last. Three show until the player opens them.
    let objects = [(at(3, 3), "berry")];
    let (world, mut app) = one_sprite_among(EVERY_NEED_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    assert!(text.iter().any(|row| row == " ► BERRIES"), "{text:?}");
    assert!(
        text.iter().any(|row| row.contains("and 3 more")),
        "{text:?}"
    );
    assert!(
        !text.iter().any(|row| row.contains("crowdedness")),
        "{text:?}"
    );
    click_brain_row(&mut app, &world, "good for hunger");
    assert!(
        !brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness")),
        "a lesson line does not open the rest"
    );
    click_brain_row(&mut app, &world, "and 3 more");
    let text = brain_lines(&app, &world);
    assert!(text.iter().any(|row| row == " ▼ BERRIES"), "{text:?}");
    assert!(
        text.iter().any(|row| row.contains("crowdedness")),
        "{text:?}"
    );
    assert!(!text.iter().any(|row| row.contains("and ")), "{text:?}");
    click_brain_row(&mut app, &world, "BERRIES");
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("and 3 more")),
        "clicking the name shuts it"
    );
    let at = brain_cell(&app, &world, "and 3 more");
    app.apply(Action::Point(at), &world);
    app.apply(Action::Enter, &world);
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness")),
        "Enter opens the thing under the pointer"
    );
}

#[test]
fn enter_opens_the_lessons_where_the_pointer_went_while_the_graph_was_open() {
    // The graph keeps the Cursor where it was, but where the pointer is,
    // for Enter and the tooltips, follows it (#304).
    let objects = [(at(3, 3), "berry")];
    let (world, mut app) = one_sprite_among(EVERY_NEED_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let more = brain_cell(&app, &world, "and 3 more");
    // Each action followed by a frame, as the game draws them, the view
    // settling first.
    let frame = |app: &mut App| {
        app.fit(ui::areas(Size::new(100, 30)));
        render(app, &world, 100, 30);
    };
    for action in [
        Action::Point(Position::new(5, 5)),
        Action::Population,
        Action::Point(more),
        Action::Back,
    ] {
        app.apply(action, &world);
        frame(&mut app);
    }
    app.apply(Action::Enter, &world);
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness")),
        "Enter opens the thing under the pointer"
    );
}

#[test]
fn choosing_another_sprite_shuts_lessons_that_were_open() {
    let neighbor = "(format: 1, genes: [])";
    let (eater, bystander) = (at(2, 3), at(8, 1));
    let objects = [(at(3, 3), "berry")];
    let mut world = WorldBuilder::grass(10, 5)
        .objects(&objects)
        .genes(&[(eater, EVERY_NEED_GENOME), (bystander, neighbor)])
        .seed(7)
        .build();
    run(&mut world, 2);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let eater_id = app.selection().expect("the eater").id();
    open(&mut app, &world, Tab::Brain);
    click_brain_row(&mut app, &world, "and 3 more");
    assert!(
        brain_lines(&app, &world)
            .iter()
            .any(|row| row.contains("crowdedness"))
    );
    app.apply(Action::SelectNext, &world);
    assert_ne!(app.selection().map(|s| s.id()), Some(eater_id));
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.selection().map(|s| s.id()), Some(eater_id));
    let text = brain_lines(&app, &world);
    assert!(
        text.iter().any(|row| row.contains("and 3 more")),
        "{text:?}"
    );
    assert!(
        !text.iter().any(|row| row.contains("crowdedness")),
        "{text:?}"
    );
}

#[test]
fn sprites_in_general_sit_above_the_sprites_it_remembers() {
    // Two bullies, each a punishment of 1, so each is frightening at -1.
    // Sprites in general are half that average until it knows three: -.50,
    // and that line comes before either of them.
    let (me, first, second) = (at(2, 3), at(3, 3), at(1, 3));
    let sprites = [(me, SKITTISH_GENOME), (first, SPEED_10), (second, SPEED_10)];
    let scripted = [
        (me, ScriptedAction::Rest),
        (first, ScriptedAction::Hit { at: me }),
        (
            second,
            ScriptedAction::Wander {
                destination: at(0, 3),
            },
        ),
        (second, ScriptedAction::Hit { at: me }),
    ];
    let world = WorldBuilder::grass(10, 5)
        .genes(&sprites)
        .scripts(&scripted)
        .seed(7)
        .build();
    let text = brain_after(world, 8);
    let sprites_at = row_of(&text, " SPRITES");
    let general = text[sprites_at + 1].as_str();
    assert!(
        general.starts_with("   frightening") && general.ends_with("-.50"),
        "{text:?}"
    );
    let people: Vec<_> = text[sprites_at + 1..]
        .iter()
        .filter(|row| row.starts_with("   #"))
        .collect();
    assert_eq!(people.len(), 2, "{text:?}");
    assert!(
        text.iter()
            .any(|row| row.starts_with("     frightening") && row.ends_with("-1.00")),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_groups_what_it_thinks_of_new_things() {
    // A first prick is from something new, so new things are bad on their
    // own, not under the thornbush (design v16 §5.6, v40 §6.1).
    let genome = r#"(format: 1, genes: [
        Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
    ])"#;
    let thornbush = at(3, 3);
    let objects = [(thornbush, "thornbush")];
    let scripted = [ScriptedAction::Eat { at: thornbush }, ScriptedAction::Rest];
    let (world, mut app) = one_sprite_among(genome, &objects, &scripted, 2);
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    let at = row_of(&text, " NEW THINGS");
    assert!(
        text[at + 1].starts_with("   bad") && text[at + 1].contains("-"),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_lists_the_cursor_on_its_own() {
    // Eating the Cursor teaches a habit about the Cursor, under its own
    // heading. It is not a sprite (design v29 §5.6, v40 §6.1).
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "hunger", value: 1.0),
        AttentionInstinct(input: "always", category: "cursor", weight: 5.0),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 3.0),
        BrainParam(param: "tau_base", value: 0.05),
    ])"#;
    let (mut world, mut app) = one_sprite(genome, 0);
    world.submit(Command::MoveCursor { tile: at(3, 3) });
    world.submit(Command::ShowCursor { visible: true });
    world.step();
    world.step();
    open(&mut app, &world, Tab::Brain);
    let text = brain_lines(&app, &world);
    let at = row_of(&text, " THE CURSOR");
    assert!(
        text[at + 1].starts_with("   eating it") && text[at + 1].contains("-"),
        "{text:?}"
    );
    assert!(
        !text.iter().any(|row| row == " SPRITES"),
        "the Cursor is not under Sprites: {text:?}"
    );
}

#[test]
fn a_habit_about_sprites_sits_with_what_it_thinks_of_them_in_general() {
    // It hits two sprites, and they hit it back. The habit of hitting them,
    // and what it thinks of sprites in general, are the first step under
    // Sprites. That step is not folded at three (design v40 §6.1).
    let hitter = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        InitialConcentration(chem: "loneliness", value: 1.0),
        Emitter(locus: Locus("did_hit"), mode: Level, gain: 0.5, chem: "reward"),
        Emitter(locus: Locus("did_hit"), mode: Level, gain: -0.5, chem: "loneliness"),
        Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
        BrainParam(param: "tau_base", value: 0.05),
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Hit, weight: 1.0),
    ])"#;
    let bully = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 10.0),
        AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Hit, weight: 1.0),
    ])"#;
    let (me, first, second) = (at(2, 3), at(3, 3), at(1, 3));
    let world = WorldBuilder::grass(10, 5)
        .genes(&[(me, hitter), (first, bully), (second, bully)])
        .seed(7)
        .build();
    let text = brain_after(world, 16);
    let sprites_at = row_of(&text, " SPRITES");
    let individual = text[sprites_at + 1..]
        .iter()
        .position(|row| row.starts_with("   #"))
        .unwrap_or_else(|| panic!("{text:?}"));
    let general = &text[sprites_at + 1..sprites_at + 1 + individual];
    assert!(
        general.iter().any(|row| row.starts_with("   hitting them")),
        "{text:?}"
    );
    assert!(
        general.len() > 3 && general.iter().all(|row| !row.contains("and ")),
        "sprites in general are not folded at three: {text:?}"
    );
}

#[test]
fn the_brain_tab_leaves_out_what_rounds_to_nothing() {
    // Learning at .004, it learns berries are worth .004 × .5 = .002 for
    // hunger: something, but .00 to two places (design §5.9).
    let genome = BERRY_GENOME.replace(
        "])",
        "    BrainParam(param: \"worth_rate_good\", value: 0.004),\n])",
    );
    let objects = [(at(3, 3), "berry")];
    let (world, mut app) = one_sprite_among(&genome, &objects, &[], 2);
    let memory = world.sprites().next().expect("the sprite").memory();
    assert!(
        memory.iter().any(|m| m.amount > 0.0 && m.amount < 0.005),
        "{memory:?}"
    );
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        !text.iter().any(|row| row.starts_with("MEMORY")),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_leaves_memory_out_until_it_has_learned_something() {
    let (world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        !text.iter().any(|row| row.starts_with("MEMORY")),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_shows_memory_before_the_first_decision() {
    // Design v17 §6.1. A scripted bite at tick 0 hurts; at tick 1, still on
    // its scripted rest, it learns thornbushes are bad: .8 × a punishment of 1.
    let objects = [(at(3, 3), "thornbush")];
    let scripted = [ScriptedAction::Eat { at: at(3, 3) }, ScriptedAction::Rest];
    let (world, mut app) = one_sprite_among(THORN_GENOME, &objects, &scripted, 2);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..4],
        [
            "Nothing decided yet",
            "",
            "THORNBUSHES",
            "bad                                  -.80",
        ]
    );
}

#[test]
fn the_brain_tab_lists_the_places_the_sprite_remembers() {
    // M2 design §7: a scripted drink at the water on its west eases its
    // thirst, so it remembers the water there.
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "thirst", value: 1.0),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
    ])"#;
    let start = at(1, 3);
    let water = at(0, 3);
    let scripted = [
        (start, ScriptedAction::Drink { at: water }),
        (start, ScriptedAction::Rest),
    ];
    let mut world = WorldBuilder::new(&["~........."; 5])
        .genes(&[(start, genome)])
        .scripts(&scripted)
        .seed(7)
        .build();
    run(&mut world, 2);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let at = row_of(&text, "PLACES");
    assert_eq!(
        text[at..at + 2],
        ["PLACES", "water · 1 tile W                     1.00"]
    );
}

#[test]
fn the_brain_tab_lists_the_places_best_remembered_first_below_what_it_has_learned() {
    // M2 design §7.5: it drinks at the water on its west, then eats from
    // the bush on its east, and its places fade fast, so the water is
    // remembered less well by the time it looks.
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "thirst", value: 1.0),
        Emitter(locus: Locus("drank"), mode: Level, gain: -0.5, chem: "thirst"),
        InitialConcentration(chem: "hunger", value: 1.0),
        Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
        BrainParam(param: "place_fade", value: 0.01),
    ])"#;
    let (start, water, bush) = (at(1, 3), at(0, 3), at(4, 3));
    let scripted = [
        (start, ScriptedAction::Drink { at: water }),
        (start, ScriptedAction::Eat { at: bush }),
        (start, ScriptedAction::Rest),
    ];
    let mut world = WorldBuilder::new(&["~........."; 5])
        .objects(&[(bush, "berry_bush")])
        .genes(&[(start, genome)])
        .scripts(&scripted)
        .seed(7)
        .build();
    world
        .start_object(bush, "mature", &[("fruit", 6)])
        .expect("the bush");
    run(&mut world, 20);
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let row = |starts: &str| {
        let at = text.iter().position(|row| row.starts_with(starts));
        at.unwrap_or_else(|| panic!("no {starts:?} in {text:?}"))
    };
    let places = row("PLACES");
    assert!(row("WATER") < places, "{text:?}");
    assert!(row("BERRY BUSHES") < places, "{text:?}");
    assert_eq!(
        (row("berry bush ·"), row("water ·")),
        (places + 1, places + 2),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_shows_what_the_sprite_thinks_of_a_category_once_it_counts() {
    // Design v19 §5.6, §6.1: a bite of a thornbush makes it −.8 bad, and a
    // bite of a berry bush with no fruit teaches nothing, but the sprite
    // knows both types of bush now: bushes are the mean, −.4, at half
    // strength, −.2. A category reads as its own plural.
    let (thornbush, bush) = (at(3, 3), at(1, 3));
    let objects = [(thornbush, "thornbush"), (bush, "berry_bush")];
    let scripted = [
        ScriptedAction::Eat { at: thornbush },
        ScriptedAction::Eat { at: bush },
        ScriptedAction::Rest,
    ];
    let genome = r#"(format: 1, genes: [
        Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
    ])"#;
    let (world, mut app) = one_sprite_among(genome, &objects, &scripted, 3);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(text.iter().any(|row| row == "BUSHES"), "{text:?}");
    assert!(
        text.iter()
            .any(|row| row.starts_with("bad") && row.ends_with("-.20")),
        "{text:?}"
    );
}

#[test]
fn the_brain_tab_says_a_led_sprite_decides_nothing() {
    // Design v23 §2.4: led, it makes no decisions, so the one before it was
    // taken hold of would mislead.
    let (mut world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    assert_ne!(
        inspector(&app, &world).1[0],
        "Being led: it decides nothing"
    );
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    assert_eq!(
        inspector(&app, &world).1[0],
        "Being led: it decides nothing"
    );
}

#[test]
fn the_brain_tab_says_when_nothing_has_been_decided_or_is_in_sight() {
    let (world, mut app) = one_sprite(HUNGRY_GENOME, 0);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(text[0], "Nothing decided yet");

    let (world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..4],
        [
            "ATTENTION",
            "nothing in sight",
            "",
            "DECISION: WANDER                        .30",
        ]
    );
}

#[test]
fn the_chem_tab_lists_every_chemical_with_its_level_and_change_per_tick() {
    let (world, mut app) = one_sprite(WORKED_GENOME, 1);
    open(&mut app, &world, Tab::Chem);
    let (_, text) = inspector_on(&app, &world, 32);
    assert_eq!(open_tab(&app, &world), "Chem");
    assert_eq!(
        text,
        [
            // One tick at rest: basal metabolism with sense radius 9.5 costs
            // .00016 of energy, and hydration loses .00033 (Appendix B).
            "energy       1.00  -.0002",
            "hydration    1.00  -.0003",
            "stamina      1.00",
            "food          .00",
            "water         .00",
            "injury        .00",
            "",
            "hunger        .40  -.4000",
            "thirst        .18",
            "pain          .00",
            "tiredness     .39",
            "boredom       .30  +.1000",
            "loneliness    .00",
            "crowdedness   .00",
            "arousal       .00",
            "anxiety       .00",
            // Nothing felt: learning used up no reward or punishment.
            "reward        .00  felt +.00",
            "punishment    .00",
            "HORMONES",
            "h0   .00   h1   .00   h2   .00   h3   .00",
            "h4   .00   h5   .00   h6   .00   h7   .00",
            "h8   .00   h9   .00   h10  .00   h11  .00",
            "h12  .00   h13  .00   h14  .00   h15  .00",
            "feminiser     .00",
            "longing       .00",
        ],
        "the whole tab at 100×32"
    );
}

/// A hungry sprite drawn to bushes and to eating them, even more so
/// beside one, whose every prick punishes it by 1.
const THORN_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 1.0),
    Emitter(locus: Locus("pricked"), mode: Level, gain: 1.0, chem: "punishment"),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "bush", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
    Instinct(inputs: [("hunger", false), ("target_adjacent", false)], verb: Eat, weight: 1.0),
])"#;

#[test]
fn the_brain_tab_names_a_thing_s_worth_neutrally_whatever_its_sign() {
    // Its bite at tick 0 pricks, and at tick 1 it learns thornbushes are bad;
    // so hungry, it bites again, and that worth takes from biting. The row
    // reads "worth: thornbush" whatever its sign, beside "habit: …".
    let objects = [(at(3, 3), "thornbush")];
    let (world, mut app) = one_sprite_among(THORN_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    assert!(
        text.iter().any(|row| row.starts_with("worth: thornbush")),
        "{text:?}"
    );
    assert!(!text.iter().any(|row| row.contains("worth it")), "{text:?}");
}

#[test]
fn the_chem_tab_shows_what_was_felt_on_the_reward_line() {
    // The bite at tick 0 pricks; at tick 1 learning uses up a punishment of 1.
    let objects = [(at(3, 3), "thornbush")];
    let (world, mut app) = one_sprite_among(THORN_GENOME, &objects, &[], 2);
    open(&mut app, &world, Tab::Chem);
    let (_, text) = inspector(&app, &world);
    let row = |name: &str| text.iter().find(|r| r.starts_with(name)).cloned();
    assert_eq!(
        row("reward").as_deref(),
        Some("reward        .00  felt -1.00")
    );
    assert_eq!(row("punishment").as_deref(), Some("punishment    .00"));
}

#[test]
fn the_genome_tab_marks_genes_naming_a_category_the_world_lacks() {
    // Unmatched genes (design v19 §5.7), dimmed with their reason as the
    // other marks are: an attention instinct and an instinct by name, and a
    // gene by number, which shows as its number.
    let genome = r#"(format: 1, genes: [
        AttentionInstinct(input: "hunger", category: "tree", weight: 0.8),
        Instinct(inputs: [("hunger", false), ("attended_tree", false)], verb: Eat, weight: 0.5),
        Gene(type: 9, version: 1, payload: "93011aca3f4ccccd"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 40), 45);
    let text = rows[2..30]
        .iter()
        .map(|row| inside(row))
        .collect::<Vec<_>>()
        .join(" ");
    for gene in [
        "hunger & attended tree → eat +.5",
        "hunger → attends to tree +.8",
        "type 9, version 1, 8 bytes",
    ] {
        assert!(text.contains(gene), "{gene:?} in {text:?}");
    }
    let reason = "unmatched: names a category this world doesn't have";
    assert_eq!(text.matches(reason).count(), 3, "{text:?}");
}

#[test]
fn the_genome_tab_shows_brain_settings_instincts_and_attention_instincts() {
    let genome = r#"(format: 1, genes: [
        AttentionInstinct(input: "hunger", category: "bush", weight: 0.8),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.2),
        Instinct(inputs: [("thirst", false), ("target_adjacent", true)], verb: Drink, weight: -0.5),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 30), 45);
    assert_eq!(open_tab(&app, &world), "Genome");
    let text: Vec<&str> = rows[2..11].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "BRAIN SETTINGS",
            "tau base .2",
            "INSTINCTS",
            "hunger → eat +1",
            "thirst & not target adjacent → drink -.5",
            "ATTENTION INSTINCTS",
            "hunger → attends to bush +.8",
            "",
            "",
        ]
    );
}

#[test]
fn the_genome_tab_shows_whom_an_attention_instinct_is_about() {
    // M2 design §15.6: one about anything draws the eye to the category,
    // and one about whom it is, only to them. Grown-ups aren't its own
    // (§18.4); a parent showing it something is, and wraps.
    let genome = r#"(format: 2, genes: [
        AttentionInstinct(input: "arousal", category: "sprite", weight: 1.0),
        AttentionInstinct(input: "arousal", category: "sprite", whom: Kin, weight: -1.0),
        AttentionInstinct(input: "anxiety", category: "egg", whom: Family, weight: 1.0),
        AttentionInstinct(input: "anxiety", category: "sprite", whom: Young, weight: 1.0),
        AttentionInstinct(input: "always", category: "sprite", whom: Grown, weight: 0.3),
        AttentionInstinct(input: "always", category: "sprite", whom: Showing, weight: 1.0),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 30), 45);
    let text: Vec<&str> = rows[2..10].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "ATTENTION INSTINCTS",
            "arousal → attends to sprite +1",
            "arousal → attends to its kin (sprite) -1",
            "anxiety → attends to its family (egg) +1",
            "anxiety → attends to its young (sprite) +1",
            "always → attends to grown-ups (sprite) +.3",
            "always → attends to its parent showing",
            "something (sprite) +1",
        ]
    );
}

#[test]
fn the_genome_tab_shows_switches_dormant_genes_and_weakened_genes() {
    // M2 design §9.6. In this pack water runs out at once and thirst
    // injures badly, and a bad injury scars a gene every tick: the tick its
    // injury first passes .5, its one gene that weakens, the emitter, is
    // scarred to .62. Its brain setting switches off as it grows up, so in
    // an adult it's dormant.
    let genome = r#"(format: 2, genes: [
        BrainParam(param: "curiosity", value: 0.6, until: Adult),
        Emitter(locus: Locus("always"), mode: Level, gain: 0.001, chem: "boredom"),
    ])"#;
    let (mut world, mut app) = one_sprite_in(pack_changing(&[SCARRING]), genome, &[], &[], 0);
    let vigour = |world: &World| world.sprites().next().expect("a sprite").vigour();
    for _ in 0..10 {
        if vigour(&world) < 1.0 {
            break;
        }
        world.step();
    }
    assert!(vigour(&world) < 1.0, "a gene was scarred");
    open(&mut app, &world, Tab::Genome);
    let rows = right_part(&render(&app, &world, 100, 30), 45);
    let text: Vec<&str> = rows[2..7].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "EMITTERS",
            "always → boredom +.001 · health .62",
            "BRAIN SETTINGS",
            "curiosity .6 · until adult",
            "dormant: switched off at this life stage",
        ]
    );
}

#[test]
fn the_genome_tab_groups_genes_as_plain_lines_and_marks_those_with_no_effect() {
    let genome = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 7.25),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.00428, chem: "hunger"),
        HalfLife(chem: "hunger", ticks: 2041),
        Trait(trait: "sense_radius", value: 9.5),
        Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
        Emitter(locus: Chem("hunger"), mode: Fall, threshold: 0.0198, gain: 1.02, chem: "reward"),
        Emitter(locus: Chem("injury"), mode: Rise, gain: 10.0, chem: "pain"),
        Emitter(locus: Locus("nearby_sprites"), mode: Level, invert: true, threshold: 0.5, gain: 0.0005, chem: "loneliness"),
        Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food"),
        HalfLife(chem: "hunger", ticks: 20),
        Trait(trait: "grows_old", value: 13500.0),
        Reaction(reactants: [("h0", 2)], products: [("h1", 1), ("h2", 1)], rate: 0.1),
        Receptor(chem: "reward", threshold: 0.2, gain: 0.5, target: "learning_rate_mod"),
        Trait(trait: "speed", value: 9.0),
        InitialConcentration(chem: "boredom", value: 0.2),
        Gene(type: 900, version: 1, payload: "c0ffee"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let screen = render(&app, &world, 100, 40);
    let rows = right_part(&screen, 45);
    assert_eq!(open_tab(&app, &world), "Genome");
    let text: Vec<&str> = rows[2..30].iter().map(|row| inside(row)).collect();
    assert_eq!(
        text,
        [
            "TRAITS",
            "speed 7.25 · sense 9.5 · grows old 13,500",
            "speed 9",
            "unexpressed: an earlier gene sets this",
            "HALF-LIVES",
            "hunger halves every 2,041 ticks",
            "hunger halves every 20 ticks",
            "unexpressed: an earlier gene sets this",
            "REACTIONS",
            "2 h0 → h1 + h2, rate .1",
            "EMITTERS",
            "low energy → hunger +.00428 past .5",
            "ate → hunger -.5",
            "hunger falls → reward +1.02 past .0198",
            "injury rises → pain +10",
            // Too long for one line, it wraps, keeping "past" with its number.
            "low nearby sprites → loneliness +.0005",
            "past .5",
            "ate → food +.3",
            "flagged: writes food, but only physiology",
            "and verbs change a physical chemical",
            "RECEPTORS",
            "reward past .2 → learning rate mod +.5",
            "STARTING LEVELS",
            "boredom starts at .2",
            "UNKNOWN GENES",
            "type 900, version 1, 3 bytes",
            "unknown: this version can't read it",
            "",
        ]
    );
    // A gene with no effect is dimmed, and so is its reason, indented under it.
    let row_of = |text: &str| {
        2 + rows[2..]
            .iter()
            .position(|r| inside(r) == text)
            .expect(text)
    };
    let (x, flagged) = (100 - 46 + 2, row_of("ate → food +.3") as u16);
    assert_eq!(screen[(x, flagged)].fg, Color::DarkGray);
    assert_eq!(
        screen[(x, flagged + 1)].symbol(),
        " ",
        "the reason is indented"
    );
    assert_eq!(screen[(x + 2, flagged + 1)].symbol(), "f");
    assert_eq!(screen[(x + 2, flagged + 1)].fg, Color::DarkGray);
    // An expressed gene's name is in the spot colour, since a tooltip says
    // what it does, and its numbers are plain (#304).
    let expressed = row_of("ate → hunger -.5") as u16;
    let spot = Theme::cp437().text().spot.fg.expect("a colour");
    assert_eq!(screen[(x, expressed)].fg, spot);
    let number = x + "ate → hunger ".chars().count() as u16;
    assert_eq!(screen[(number, expressed)].fg, Color::Reset);
}

/// A genome whose Genome tab is 61 lines: the heading, then 60 emitters
/// whose gains, .001 to .060, tell them apart.
fn long_genome() -> String {
    let emitters: Vec<String> = (1..=60)
        .map(|n| {
            let gain = n as f32 / 1000.0;
            format!(
                r#"Emitter(locus: Locus("always"), mode: Level, gain: {gain}, chem: "boredom")"#
            )
        })
        .collect();
    format!("(format: 1, genes: [{}])", emitters.join(", "))
}

/// The first and last lines the inspector shows.
fn first_and_last(app: &App, world: &World) -> (String, String) {
    let (_, text) = inspector(app, world);
    (text[0].clone(), text[text.len() - 1].clone())
}

#[test]
fn page_down_and_up_scroll_a_long_tab_a_page_and_stop_at_either_end() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    // 23 rows fit at 100×30, so the tab scrolls at most 38 lines.
    let mut page = |pages: i32| {
        app.apply(Action::ScrollTab { pages }, &world);
        first_and_last(&app, &world)
    };
    let shown = |first: &str, last: &str| (first.to_string(), last.to_string());
    let gain = |gain: &str| format!("always → boredom +{gain}");
    assert_eq!(page(1), shown(&gain(".023"), &gain(".045")), "a page down");
    assert_eq!(
        page(1),
        shown(&gain(".038"), &gain(".06")),
        "no further than the end"
    );
    assert_eq!(page(-1), shown(&gain(".015"), &gain(".037")), "a page up");
    assert_eq!(
        page(-1),
        shown("EMITTERS", &gain(".022")),
        "no further than the top"
    );
}

#[test]
fn the_wheel_over_the_inspector_scrolls_3_lines_a_notch_and_elsewhere_does_not() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    let over_inspector = Position::new(80, 10);
    app.apply(
        Action::Wheel {
            at: over_inspector,
            notches: 2,
        },
        &world,
    );
    assert_eq!(first_and_last(&app, &world).0, "always → boredom +.006");
    app.apply(
        Action::Wheel {
            at: over_inspector,
            notches: -1,
        },
        &world,
    );
    assert_eq!(first_and_last(&app, &world).0, "always → boredom +.003");
    assert_eq!(app.mode(), CursorMode::Select, "the mode stays");
    app.apply(
        Action::Wheel {
            at: pointer_on(&world, 2, 1),
            notches: 2,
        },
        &world,
    );
    assert_eq!(
        first_and_last(&app, &world).0,
        "always → boredom +.003",
        "over the map"
    );
    assert_eq!(
        app.cursor(),
        at(2, 1),
        "a wheel event points, like every mouse event"
    );
}

#[test]
fn a_tab_goes_back_to_the_top_when_the_tab_or_the_selection_changes() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    app.apply(Action::NextTab, &world);
    app.apply(Action::PreviousTab, &world);
    assert_eq!(
        first_and_last(&app, &world).0,
        "EMITTERS",
        "after switching tabs"
    );
}

#[test]
fn selecting_another_sprite_starts_its_tab_from_the_top_and_the_same_one_again_does_not() {
    let genome = long_genome();
    let sprites = [(at(2, 3), genome.as_str()), (at(6, 1), genome.as_str())];
    let world = WorldBuilder::grass(10, 5).genes(&sprites).seed(7).build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3).
    app.apply(Action::left_click(pointer_on(&world, 2, 3)), &world);
    assert_eq!(
        first_and_last(&app, &world).0,
        "always → boredom +.023",
        "the same sprite again"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(first_and_last(&app, &world).0, "EMITTERS", "another sprite");
}

#[test]
fn after_the_inspector_grows_page_up_scrolls_from_what_is_shown() {
    let (world, mut app) = one_sprite(&long_genome(), 0);
    open(&mut app, &world, Tab::Genome);
    app.apply(Action::ScrollTab { pages: 2 }, &world); // the end: line 38 at the top
    // At 100×40 the tab has 33 rows, so the end is line 28 at the top.
    app.fit(ui::areas(Size::new(100, 40)));
    app.apply(Action::ScrollTab { pages: -1 }, &world);
    let rows = right_part(&render(&app, &world, 100, 40), 45);
    assert_eq!(inside(&rows[2]), "EMITTERS", "a page up from line 28");
}

#[test]
fn a_change_too_small_to_show_leaves_no_number_and_no_arrow() {
    // Boredom gains .00005 a tick, which rounds to .0000 at 4 decimals.
    let genome = r#"(format: 1, genes: [
        Emitter(locus: Locus("always"), mode: Level, gain: 0.00005, chem: "boredom"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 1);
    let (_, body) = inspector(&app, &world);
    // After the action line, age, traits, sex, where it came from, a
    // blank line and four drives.
    assert_eq!(body[10], "boredom     ░░░░░░░░░░ .00", "the Body tab");
    open(&mut app, &world, Tab::Chem);
    let (_, chem) = inspector(&app, &world);
    assert_eq!(chem[11], "boredom       .00", "the Chem tab");
}

#[test]
fn big_and_negative_gene_values_keep_3_significant_figures_and_their_sign() {
    // `lifespan` is what genome files from before M2 call `grows_old`.
    let genome = r#"(format: 1, genes: [
        Trait(trait: "lifespan", value: -1234.5),
        Emitter(locus: Locus("always"), mode: Level, gain: 1234.5, chem: "boredom"),
        Emitter(locus: Locus("always"), mode: Level, gain: -98765.0, chem: "boredom"),
    ])"#;
    let (world, mut app) = one_sprite(genome, 0);
    open(&mut app, &world, Tab::Genome);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..5],
        [
            "TRAITS",
            "grows old -1,235",
            "EMITTERS",
            "always → boredom +1,230",
            "always → boredom -98,800",
        ]
    );
}

#[test]
fn every_tab_s_text_is_within_cp437() {
    // Design §6.2: any CP437 font or tileset can draw every panel.
    let mut world = generated_world();
    run(&mut world, 500);
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    app.apply(Action::SelectNext, &world);
    for _ in terra_tui::app::Tab::ALL {
        let screen = render(&app, &world, 100, 40);
        for line in lines(&screen) {
            for c in line.chars() {
                assert!(terra_tui::cp437::contains(c), "{c:?} in {line:?}");
            }
        }
        app.apply(Action::NextTab, &world);
    }
}

#[test]
fn the_world_tab_shows_the_population_and_the_deaths_by_cause() {
    // Water runs out in two ticks, and then dehydration kills in two more.
    let pack = pack_changing(&[(
        "physiology.ron",
        &[
            ("hydration_loss: 0.00033", "hydration_loss: 0.5"),
            ("dehydration: 0.0011", "dehydration: 0.5"),
        ],
    )]);
    let mut world = WorldBuilder::grass(10, 5)
        .starters(&[at(1, 1), at(4, 2), at(8, 3)])
        .data(pack)
        .seed(7)
        .build();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &world);
    let text = &text[GRAPH_LINES..];
    assert_eq!(
        text[1..7],
        [
            "sprites           3",
            // The generations of the living (M2 design §17.5).
            "generation        1",
            // The grown who can breed, and those room nearby holds back
            // (M2 design §10.7).
            "fertile           3",
            "too crowded 0",
            "deaths            0",
            "starvation 0 · dehydration 0 · old age 0",
        ]
    );
    run(&mut world, 10);
    let (_, text) = inspector(&app, &world);
    let text = &text[GRAPH_LINES..];
    assert_eq!(
        text[1..4],
        [
            "sprites           0",
            "deaths            3",
            "starvation 0 · dehydration 3 · old age 0",
        ]
    );
    assert_eq!(text[4], "berry bush        0", "then the objects");
}

#[test]
fn the_world_tab_counts_the_grown_sprites_fertile_and_too_crowded_to_breed() {
    // M2 design §10.7, #331: each of the 5 on the left has 4 others within
    // 10 tiles, and the 2 on the right, 15 tiles from them, only each other.
    // The young one in between counts for neither, and can't breed itself.
    let crowd = [at(0, 0), at(2, 0), at(4, 0), at(1, 2), at(3, 2)];
    let young = at(10, 4);
    let world = WorldBuilder::grass(20, 5)
        .starters(&crowd)
        .starters(&[at(19, 0), at(19, 2), young])
        .newborns(&[young])
        .seed(7)
        .build();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[GRAPH_LINES + 1..GRAPH_LINES + 6],
        [
            "sprites           8",
            "generation        1",
            "fertile           2",
            "too crowded 5",
            "deaths            0",
        ]
    );
}

#[test]
fn in_a_full_world_the_world_tab_says_so_beneath_the_fertile() {
    // M2 design §10.6, §10.7, #331: no sprite can breed until one dies.
    let world = full_world();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[GRAPH_LINES + 1..GRAPH_LINES + 6],
        [
            "sprites         100",
            "generation        1",
            "fertile           0",
            "world full",
            "deaths            0",
        ]
    );
}

#[test]
fn a_world_tab_without_breeding_or_sprites_counts_no_fertile() {
    // M2 design §10.7: a pack with no egg can't breed, and a world with no
    // sprites has nobody to count, as with its generations.
    let eggless = pack_changing(&[("objects.ron", &[("next: Hatch", "next: Expire")])]);
    let world = WorldBuilder::grass(10, 5)
        .starters(&[at(1, 1)])
        .data(eggless)
        .seed(7)
        .build();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &world);
    assert_eq!(text[GRAPH_LINES + 3], "deaths            0", "{text:?}");
    let empty = garden(pack());
    let app = app_for(&empty, Theme::cp437(), 100, 30);
    let (_, text) = inspector(&app, &empty);
    assert_eq!(text[GRAPH_LINES + 2], "deaths            0", "{text:?}");
}

#[test]
fn a_death_message_too_long_for_one_line_wraps() {
    let world = garden_with_sprites();
    let id = world.sprites().next().expect("a sprite").id();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    app.record(
        &[died(4_012, id, DeathCause::Dehydration, 12_345_678)],
        &world,
    );
    let (_, text) = inspector(&app, &world);
    assert_eq!(
        text[..3],
        [
            format!("Sprite #{} died of dehydration at", id.0),
            "age 12,345,678".to_string(),
            String::new(),
        ],
        "a line break keeps \"age\" with its number"
    );
}

#[test]
fn a_world_tab_list_too_long_for_one_line_wraps_after_a_dot() {
    let world = garden(pack_changing(&[(
        "objects.ron",
        &[("\"mature\"", "\"fully_grown_and_bearing_fruit\"")],
    )]));
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let (_, rows) = inspector_rows(&app, &world, 30);
    let text = &rows[GRAPH_LINES..8 + GRAPH_LINES];
    assert_eq!(
        text[4..7],
        [
            " berry bush        1",
            "   seedling 1 ·",
            "   fully_grown_and_bearing_fruit 0",
        ]
    );
}

#[test]
fn selecting_the_selected_sprite_from_a_scrolled_world_tab_opens_body_at_the_top() {
    // At 100×12 the inspector has 8 rows: too few for the World tab's 19
    // lines, or the Body tab's 13.
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 12);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::PreviousTab, &world);
    app.apply(Action::ScrollTab { pages: 1 }, &world);
    // The first sprite is at (2, 3).
    app.apply(Action::left_click(pointer_on(&world, 2, 3)), &world);
    let rows = right_part(&render(&app, &world, 100, 12), 45);
    assert_eq!(open_tab(&app, &world), "Body");
    // Its first line is what it's doing: nothing has moved it to act yet
    // (M2 design §19.2).
    assert!(inside(&rows[2]).starts_with("Standing"), "{:?}", rows[2]);
    assert!(inside(&rows[3]).starts_with("age "), "{:?}", rows[3]);
}

#[test]
fn chemical_names_show_with_spaces_for_underscores() {
    // "boredom" renamed "bored_ness" in the pack, its brain inputs and the starter genome.
    let renamed: &[(&str, &str)] = &[("\"boredom\"", "\"bored_ness\"")];
    let pack = pack_changing(&[
        ("chemicals.ron", renamed),
        ("brain_io.ron", renamed),
        ("genomes/starter.ron", renamed),
    ]);
    let world = WorldBuilder::grass(10, 5)
        .starters(&[at(2, 3)])
        .data(pack)
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let (_, body) = inspector(&app, &world);
    assert!(body[10].starts_with("bored ness  "), "{:?}", body[10]);
    open(&mut app, &world, Tab::Chem);
    let (_, chem) = inspector(&app, &world);
    assert!(chem[11].starts_with("bored ness  "), "{:?}", chem[11]);
}

/// The Body tab's lines from "Observed" to the end, on a screen tall
/// enough to show it all, keeping their leading spaces.
fn observed_section(app: &App, world: &World) -> Vec<String> {
    let text = right_part(&render(app, world, 100, 60), 45);
    let start = row_of(&text, " Observed");
    let end = text[start..]
        .iter()
        .position(String::is_empty)
        .map_or(text.len(), |n| start + n);
    text[start..end].to_vec()
}

#[test]
fn the_body_tab_ends_with_what_was_observed_and_how_long_ago() {
    let (mut world, _) = one_sprite(WORKED_GENOME, 0);
    let mut app = app_for(&world, Theme::cp437(), 100, 60);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        observed_section(&app, &world),
        [" Observed", "   nothing yet"]
    );
    let id = world.sprites().next().expect("the sprite").id();
    run(&mut world, 1_300);
    // The world has run ticks 0 to 1,299.
    app.record(
        &[
            finished(3, id, Verb::Rest, Outcome::Applied),
            finished(1_280, id, Verb::Wander, Outcome::Applied),
            finished(1_290, id, Verb::Wander, Outcome::Applied),
            finished(1_291, id, Verb::Wander, Outcome::Blocked),
            finished(1_299, id, Verb::Rest, Outcome::Applied),
        ],
        &world,
    );
    // Newest first; the times right-aligned, and a long line wrapped under
    // its text.
    assert_eq!(
        observed_section(&app, &world),
        [
            " Observed",
            "        just now · Rested",
            "     8 ticks ago · Wandered off, but gave up:",
            "                   the way was blocked",
            "     9 ticks ago · Wandered off x2",
            " 1,296 ticks ago · Rested",
        ]
    );
}

/// A sprite that attends to sprites, would rather rest a little, and feels
/// every hit as a punishment of 1.
const SKITTISH_GENOME: &str = r#"(format: 1, genes: [
    Trait(trait: "speed", value: 10.0),
    Trait(trait: "sense_radius", value: 10.0),
    BrainParam(param: "tau_base", value: 0.05),
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false)], verb: Rest, weight: 0.3),
    Emitter(locus: Locus("was_hit"), mode: Level, gain: 1.0, chem: "punishment"),
])"#;

/// A skittish sprite, #1, hit by a bully, #2, that then rests beside it,
/// `ticks` ticks on, with an app that logged every tick and selects #1.
fn hit_by_a_bully(ticks: u32) -> (World, App) {
    let (me, bully) = (at(2, 3), at(3, 3));
    let scripted = [
        (bully, ScriptedAction::Hit { at: me }),
        (bully, ScriptedAction::Rest),
    ];
    let mut world = WorldBuilder::grass(10, 5)
        .genes(&[(me, SKITTISH_GENOME), (bully, SPEED_10)])
        .scripts(&scripted)
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    for _ in 0..ticks {
        let events = world.step();
        app.record(&events, &world);
    }
    app.apply(Action::SelectNext, &world);
    (world, app)
}

#[test]
fn the_brain_tab_names_the_sprite_it_attends_to_and_how_frightening_it_is() {
    // Design v18 §6.1. Hit at tick 0, it learns at tick 1 to fear the
    // bully, −1 × fear_fade's little; from tick 2 it backs away from it:
    // flight (.8) × its fear, with the bully beside it.
    let (world, mut app) = hit_by_a_bully(3);
    open(&mut app, &world, Tab::Brain);
    let (_, text) = inspector(&app, &world);
    let row = |start: &str| text.iter().find(|row| row.starts_with(start)).cloned();
    assert!(text[1].starts_with("► Sprite #2"), "{text:?}");
    assert!(
        row("DECISION: RETREAT").is_some_and(|r| r.ends_with(".80")),
        "{text:?}"
    );
    assert!(
        row("fear: Sprite #2").is_some_and(|r| r.ends_with("+.80")),
        "{text:?}"
    );
    let kept = brain_lines(&app, &world);
    assert!(kept.iter().any(|row| row == " SPRITES"), "{kept:?}");
    assert!(kept.iter().any(|row| row == "   #2"), "{kept:?}");
    assert!(
        kept.iter()
            .any(|row| row.starts_with("     frightening") && row.ends_with("-1.00")),
        "{kept:?}"
    );
    let screen = lines(&render(&app, &world, 100, 30));
    assert!(
        screen
            .iter()
            .any(|row| row.contains("Sprite #1 learned: Sprite #2 is frightening")),
        "{screen:?}"
    );
    let bully = world.sprites().nth(1).expect("the bully").id();
    app.record(
        &[Event {
            tick: 3,
            kind: EventKind::Renamed {
                id: bully,
                name: "Mira".into(),
            },
        }],
        &world,
    );
    assert!(
        brain_lines(&app, &world).iter().any(|row| row == "   Mira"),
        "{:?}",
        brain_lines(&app, &world)
    );
}

/// The player takes hold of `world`'s first sprite on one tick, and shoves it
/// 3 tiles east on the next (design v25 §6.1).
fn shove_east(world: &mut World) {
    let sprite = world.sprites().next().expect("the sprite").id();
    world.submit(Command::TakeHold { sprite });
    world.step();
    world.submit(Command::Shove {
        toward: terra_sim::Dir::E,
        tiles: 3,
    });
    world.step();
}

#[test]
fn the_body_tab_says_a_shoved_sprite_slides_and_how_far_it_has_to_go() {
    // Design v25 §6.1. It's shoved 3 tiles east from (2, 3), and slides a
    // tile a tick.
    let (mut world, mut app) = one_sprite_doing(SPEED_10, &[], 0);
    shove_east(&mut world);
    assert_eq!(inspector(&app, &world).1[0], "Shoved · 2 tiles to go");
    app.apply(Action::ToggleDetail, &world);
    assert_eq!(inspector(&app, &world).1[0], "SHOVED → E · 2 tiles left");
}

#[test]
fn the_brain_tab_says_a_sliding_sprite_decides_nothing() {
    let (mut world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Brain);
    shove_east(&mut world);
    assert_eq!(inspector(&app, &world).1[0], "Shoved: it decides nothing");
}

/// Runs `world` until the egg `egg` has hatched (M2 design §10.4).
fn hatch(world: &mut World, egg: EntityId) {
    while world.sprite(egg).is_none() {
        assert!(world.tick() < 2_000, "the egg hatches");
        world.step();
    }
}

#[test]
fn the_body_tab_says_how_near_a_male_is_to_turning_female_once_grown() {
    // M2 design §10.7: `turning 50%` with half the feminiser it takes;
    // never in a young one (§10.1).
    let genome = r#"(format: 1, genes: [
        InitialConcentration(chem: "feminiser", value: 0.4),
    ])"#;
    let (grown, young) = (at(2, 3), at(6, 1));
    let world = WorldBuilder::grass(10, 5)
        .genes(&[(grown, genome), (young, genome)])
        .newborns(&[young])
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        inspector_line(&app, &world, "male"),
        "male · fertile · turning 50%"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(inspector_line(&app, &world, "male"), "male", "young");
}

/// The Body tab's line that starts with the selected sprite's sex.
fn sex_line(app: &App, world: &World) -> String {
    let Some(Selection::Living(id)) = app.selection() else {
        panic!("a sprite selected");
    };
    let sprite = world.sprite(id).expect("the sprite");
    inspector_line(app, world, sprite.sex().name())
}

#[test]
fn the_body_tab_says_when_room_nearby_or_a_full_world_holds_a_sprite_back() {
    // M2 design §10.7, #331: grown, fed and rested, but with 4 other grown
    // sprites within 10 tiles, it's `too crowded to breed`.
    let crowd = [at(2, 3), at(0, 0), at(9, 0), at(0, 4), at(9, 4)];
    let world = WorldBuilder::grass(10, 5).starters(&crowd).seed(7).build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(sex_line(&app, &world), "male · too crowded to breed");
    // With one of them out of reach, it has room nearby.
    let world = WorldBuilder::grass(14, 5)
        .starters(&crowd[..4])
        .starters(&[at(13, 4)])
        .seed(7)
        .build();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(sex_line(&app, &world), "male · fertile");
    // A world that holds its 100 is full first, whatever is near (§10.6).
    let world = full_world();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    assert_eq!(sex_line(&app, &world), "male · world full");
}

/// 100 starters, as many as a world holds (M2 design §10.6).
fn full_world() -> World {
    let sprites: Vec<Pos> = (0..100).map(|i| at(i % 12, i / 12)).collect();
    WorldBuilder::grass(12, 9)
        .starters(&sprites)
        .seed(7)
        .build()
}

/// A keen female and a keen male, his genome with `his` genes too, side by
/// side by a thornbush, by `pack`'s rules, once they have had their first
/// egg and become partners: the world, her and him.
fn a_pair(pack: DataPack, his: &str) -> (World, EntityId, EntityId) {
    let keen = |extra: &str| {
        format!(
            r#"(format: 1, genes: [
                Trait(trait: "speed", value: 10.0),
                Trait(trait: "sense_radius", value: 10.0),
                AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
                Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
                InitialConcentration(chem: "receptivity", value: 1.0),
                {extra}
            ])"#
        )
    };
    let hers = r#"InitialConcentration(chem: "feminiser", value: 1.0),"#;
    let mut world = WorldBuilder::grass(6, 2)
        .objects(&[(at(4, 1), "thornbush")])
        .genes(&[(at(1, 0), &keen(hers)), (at(2, 0), &keen(his))])
        .data(pack)
        .seed(7)
        .build();
    let paired = (0..100)
        .flat_map(|_| world.step())
        .find_map(|event| match event.kind {
            EventKind::Paired { a, b } => Some((a, b)),
            _ => None,
        });
    let (she, he) = paired.expect("they had an egg");
    (world, she, he)
}

/// The screen, as text, with `id` clicked on and the Body tab open.
fn body_of(world: &World, id: EntityId) -> String {
    let mut app = app_for(world, Theme::cp437(), 100, 30);
    let tile = world.sprite(id).expect("alive").pos();
    app.apply(Action::left_click(pointer_on(world, tile.x, tile.y)), world);
    open(&mut app, world, Tab::Body);
    lines(&render(&app, world, 100, 30)).join("\n")
}

#[test]
fn the_body_tab_names_a_sprite_s_partner_after_their_first_egg() {
    // M2 design §15.8: `partner Sprite #2`, after the parents' line.
    let (world, she, he) = a_pair(pack(), "");
    let text = body_of(&world, she);
    assert!(
        text.contains(&format!("partner Sprite #{}", he.0)),
        "{text}"
    );
    assert!(!text.contains("· died"), "{text}");
}

#[test]
fn the_body_tab_says_when_a_sprite_s_partner_has_died() {
    // M2 design §15.8: `partner Kit #20 · died`, until she pairs again.
    // Once paired, he hits the thornbush, whose thorns kill at once.
    let his = r#"AttentionInstinct(input: "paired", category: "bush", weight: 2.0),
        Instinct(inputs: [("paired", false), ("attended_bush", false)], verb: Hit, weight: 2.0),"#;
    let (mut world, she, he) = a_pair(pack_changing(&[LETHAL_THORNS]), his);
    while world.sprite(he).is_some() {
        assert!(world.tick() < 200, "he died");
        world.step();
    }
    let text = body_of(&world, she);
    assert!(
        text.contains(&format!("partner Sprite #{} · died", he.0)),
        "{text}"
    );
}

/// How many genes `listed` names after `label`: one, and one more for each
/// `·` before the next label's colon.
fn named_after(listed: &str, label: &str) -> usize {
    let rest = listed.split(label).nth(1).unwrap_or_default();
    rest.split(':').next().unwrap_or(rest).matches('·').count() + 1
}

#[test]
fn the_genome_tab_of_a_sprite_with_parents_says_first_what_s_new_in_it() {
    // M2 design §12: a lone egg, every gene value copied wrongly and each
    // copy of a gene flawed half the time. Its mother, whom the world
    // began with, has no such list.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.5,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.0, lost: 0.0, changed: 0.0,",
        ),
    ]);
    hatch(&mut world, egg);
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    app.apply(Action::SelectNext, &world);
    open(&mut app, &world, Tab::Genome);
    let text = |app: &App| inspector_on(app, &world, 40).1;
    assert_eq!(text(&app)[0], "TRAITS", "the mother has no list");
    select(&mut app, &world, egg);
    let rows = text(&app);
    assert_eq!(rows[0], "FROM SPRITE #1 ALONE");
    let view = world.sprite(egg).expect("the hatchling");
    let changed = view.copying_errors().len();
    assert!(changed >= 3, "{changed} genes changed");
    let list = rows
        .iter()
        .position(|row| row.is_empty())
        .expect("a blank line");
    assert_eq!(rows[list + 1], "TRAITS");
    let listed = rows[1..list].join(" ");
    assert_eq!(listed.matches(", was ").count(), changed, "{listed}");
    let flaws = view.flaws();
    // A lone egg's copies are its mother's doubled, but copying flaws each
    // on its own, so it has broken genes and hidden flaws.
    let broken = flaws.iter().filter(|&&flawed| flawed == 2).count();
    let carried = flaws.iter().filter(|&&flawed| flawed == 1).count();
    assert!(broken > 0 && carried > 0, "{flaws:?}");
    assert_eq!(named_after(&listed, "broken: "), broken, "{listed}");
    assert_eq!(named_after(&listed, "carries flaws: "), carried, "{listed}");
    for name in ["speed", "low energy → hunger", "always → breed_alone"] {
        assert!(listed.contains(name), "{name:?} in {listed:?}");
    }
    // Each broken gene shows its health where it sits, as one old age
    // weakened does.
    let genes = rows[list + 1..].join(" ");
    assert_eq!(genes.matches("health .5").count(), broken, "{genes}");
}

#[test]
fn the_genome_tab_lists_the_genes_copying_changed_doubled_and_lost() {
    // M2 design §13: each changed gene with what it was, or what it's a
    // copy of; then the genes doubled, and those lost.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.3, lost: 0.3, changed: 0.3,",
        ),
    ]);
    hatch(&mut world, egg);
    let mut app = app_for(&world, Theme::cp437(), 100, 40);
    select(&mut app, &world, egg);
    open(&mut app, &world, Tab::Genome);
    let (_, rows) = inspector_on(&app, &world, 40);
    let list = rows
        .iter()
        .position(|row| row.is_empty())
        .expect("a blank line");
    let listed = rows[1..list].join(" ");
    let changes = world.sprite(egg).expect("the hatchling").copy_changes();
    let count = |kind: fn(&CopyChangeView) -> bool| changes.iter().filter(|c| kind(c)).count();
    let changed = count(|c| matches!(c, CopyChangeView::Changed { .. }));
    let copies = count(|c| matches!(c, CopyChangeView::Changed { on_copy: true, .. }));
    let doubled = count(|c| matches!(c, CopyChangeView::Doubled { .. }));
    let lost = count(|c| matches!(c, CopyChangeView::Lost { .. }));
    assert!(changed > 0 && doubled > 0 && lost > 0, "{changes:?}");
    assert_eq!(listed.matches("new: ").count(), changed, "{listed}");
    assert_eq!(listed.matches(", from ").count(), copies, "{listed}");
    assert_eq!(
        listed.matches(", was ").count(),
        changed - copies,
        "{listed}"
    );
    assert_eq!(named_after(&listed, "doubled: "), doubled, "{listed}");
    assert_eq!(named_after(&listed, "lost: "), lost, "{listed}");
}

/// What's new in `id`, a hatchling selected with `Tab`, as the top of its
/// Genome tab lists it (M2 design §12.6), up to the blank row: each line
/// whole, though the tab wraps a long one onto rows indented further.
fn whats_new(world: &World, id: EntityId) -> Vec<String> {
    let mut app = app_for(world, Theme::cp437(), 100, 40);
    select(&mut app, world, id);
    open(&mut app, world, Tab::Genome);
    let mut said: Vec<String> = Vec::new();
    for row in inspector_rows(&app, world, 40).1 {
        if row.is_empty() {
            return said;
        }
        match row.strip_prefix("   ") {
            Some(more) => {
                let line = said.last_mut().expect("a line it goes on");
                line.push(' ');
                line.push_str(more);
            }
            None => said.push(row.trim_start().to_string()),
        }
    }
    panic!("no blank row after {said:#?}");
}

#[test]
fn the_genome_tab_of_a_pair_s_young_says_it_is_from_them_both() {
    // M2 design §12.6: `FROM PIP #12 AND BO #7`, the one that laid it
    // first.
    let (mut world, she, he) = a_pair(pack(), "");
    let egg = world.eggs().next().expect("their egg").id();
    assert_eq!(world.egg(egg).expect("the egg").parents(), [she, he]);
    hatch(&mut world, egg);
    let from = format!("FROM SPRITE #{} AND SPRITE #{}", she.0, he.0);
    assert_eq!(whats_new(&world, egg)[0], from);
}

#[test]
fn the_genome_tab_says_of_a_gene_copying_changed_only_the_numbers_that_differ() {
    // M2 design §12.6: `+.00512 past .515, was +.00428`. Copying changes
    // each gene value seven times in ten, so a gene may change in each of
    // its numbers or in only one of two. Each changed gene's line says the
    // gene as it is, then what it was: the numbers that read differently,
    // or all of them if none does.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 0.7,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.0, lost: 0.0, changed: 0.0,",
        ),
    ]);
    hatch(&mut world, egg);
    let listed = whats_new(&world, egg);
    let view = world.sprite(egg).expect("the hatchling");
    let (genes, errors) = (view.genes(), view.copying_errors());
    // Where it's from, then a line for each changed gene, in genome order.
    assert_eq!(listed.len(), errors.len() + 1, "{listed:#?}");
    let mut some_alike = false;
    for ((index, was), line) in errors.iter().zip(&listed[1..]) {
        let (now_said, was_said) = line
            .split_once(", was ")
            .unwrap_or_else(|| panic!("{line:?}"));
        let (now, was) = (numbers_of(&genes[*index].0), numbers_of(was));
        let all = |values: &[f32]| values.iter().map(|&v| reads(v)).collect::<Vec<_>>();
        assert_eq!(numbers_in(now_said), all(&now), "{line:?}");
        let differ: Vec<String> = was
            .iter()
            .zip(&now)
            .filter(|&(&was, &now)| reads(was) != reads(now))
            .map(|(&was, _)| reads(was))
            .collect();
        some_alike |= !differ.is_empty() && differ.len() < was.len();
        let shown = if differ.is_empty() { all(&was) } else { differ };
        assert_eq!(numbers_in(was_said), shown, "{line:?}");
    }
    assert!(
        some_alike,
        "a gene with a number copying left alone: {listed:#?}"
    );
}

/// A gene's numbers in the order the Genome tab writes them, for the kinds
/// of gene `a_lone_egg`'s mother has: an emitter's gain, then its
/// threshold unless it's 0.
fn numbers_of(gene: &GeneView) -> Vec<f32> {
    match *gene {
        GeneView::Trait { value, .. } | GeneView::InitialConcentration { value, .. } => {
            vec![value]
        }
        GeneView::Emitter {
            gain,
            threshold: 0.0,
            ..
        } => vec![gain],
        GeneView::Emitter {
            gain, threshold, ..
        } => vec![gain, threshold],
        GeneView::Instinct { weight, .. } => vec![weight],
        ref other => panic!("no such gene in the mother: {other:?}"),
    }
}

/// How a gene's value reads to 3 significant figures, as the Genome tab
/// writes it: `.00428` and `4.28e-3` read alike.
fn reads(value: f32) -> String {
    format!("{value:.2e}")
}

/// The numbers written in `said`, as they read.
fn numbers_in(said: &str) -> Vec<String> {
    said.split(char::is_whitespace)
        .filter_map(|word| word.replace(',', "").parse::<f32>().ok())
        .map(reads)
        .collect()
}

#[test]
fn the_genome_tab_says_what_s_new_in_its_order() {
    // M2 design §12.6, §13.2: where its genes came from; each gene
    // changed; the genes doubled; those lost; each gene copying changed a
    // number of; the genes broken; those that carry a flaw. Everything
    // copying can do, it does often here.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 0.5,"),
        (
            COPY_AND_CHANGE,
            "copy_and_change: (doubled: 0.3, lost: 0.3, changed: 0.3,",
        ),
    ]);
    hatch(&mut world, egg);
    let listed = whats_new(&world, egg);
    // Each kind of line, in the order the tab lists them; a gene copying
    // changed a number of is named first, as the tab writes the gene.
    let starts = [
        "FROM ",
        "new: ",
        "doubled: ",
        "lost: ",
        "",
        "broken: ",
        "carries flaws: ",
    ];
    let kinds: Vec<usize> = listed
        .iter()
        .map(|line| {
            let named = starts
                .iter()
                .position(|&start| !start.is_empty() && line.starts_with(start));
            named.unwrap_or(4)
        })
        .collect();
    assert!(
        (0..starts.len()).all(|kind| kinds.contains(&kind)),
        "{kinds:?}: {listed:#?}"
    );
    assert!(kinds.is_sorted(), "{kinds:?}: {listed:#?}");
}

#[test]
fn a_broken_gene_counts_in_the_body_tab_s_genes_but_not_its_wear() {
    // M2 design §12.6: the Body tab's `genes .97` counts a broken gene,
    // whose health is .5; §12.4: it isn't wear. Every copy of every gene
    // that can weaken is flawed, so each is broken.
    let (mut world, egg) = a_lone_egg(&[
        ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
        ("flaws: (arise: 0.004,", "flaws: (arise: 1.0,"),
    ]);
    hatch(&mut world, egg);
    let view = world.sprite(egg).expect("the hatchling");
    assert!(view.flaws().contains(&2), "{:?}", view.flaws());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    select(&mut app, &world, egg);
    open(&mut app, &world, Tab::Body);
    assert_eq!(inspector_line(&app, &world, "wear"), "wear .00 · genes .50");
}
