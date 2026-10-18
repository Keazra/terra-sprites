//! The family tree (M2 design §17): `k` opens it over the panels, on the
//! selected sprite or egg, with its grandparents, parents, young and
//! brothers and sisters, living or dead; the arrows or the pointer choose
//! one, and `Enter` or a click walks the tree to it and selects it. The
//! lineage colours (`b`) mark the same sprites on the map, and the inspector
//! says each sprite's generation.

mod common;
use common::{
    Changes, LETHAL_THORNS, LONE, LONE_MOTHER, QUICK, SCORCHED, app_on, at, changed, group, grow,
    inspector_rows, lines, lone_line, open, pack_changing, pairs, parched_line, pointer_on, render,
    rows_from, scratch_folder, select, tree_of,
};

use std::time::Duration;

use ratatui::layout::{Margin, Position, Size};
use ratatui::style::{Color, Modifier};
use terra_sim::{Command, DeathCause, EntityId, Genome, Map, Scenario, World};
use terra_tui::app::{App, ColourMode, Screen, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::ui;

/// Steps `world` `ticks` ticks, with `app` watching.
fn run(app: &mut App, world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        let events = world.step();
        app.record(&events, world);
    }
}

/// The screen as text, a line for each row.
fn screen(app: &App, world: &World) -> String {
    lines(&render(app, world)).join("\n")
}

// The tree's rows, from the top of its box's inside.
const GRANDPARENTS: usize = 1;
const GRANDPARENTS_JOIN: usize = 4;
const PARENTS: usize = 5;
const PARENTS_JOIN: usize = 8;
const CENTRE: usize = 9;
const DOWN: usize = 11;
const BROODS_JOIN: usize = 12;
const BROODS: usize = 13;
const BROOD_JOIN: usize = 14;
const YOUNG: usize = 15;
const SIBLINGS_HEADING: usize = 19;
const SIBLINGS: usize = 20;

/// The inside of the tree's box, a row at a time, with what's past each
/// row's last character left off.
fn tree(app: &App, world: &World) -> Vec<String> {
    let buffer = render(app, world);
    let inside = app.overlay().expect("a box").inner(Margin::new(1, 1));
    (inside.top()..inside.bottom())
        .map(|y| {
            let row: String = (inside.left()..inside.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            row.trim_end().to_string()
        })
        .collect()
}

/// What a row of the tree says, card by card: the text of each, two
/// spaces apart.
fn said(row: &str) -> String {
    row.split("  ")
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("  ")
}

/// The cell on the screen of the first character of `text` in the tree's
/// row `row`.
fn label_at(app: &App, world: &World, row: usize, text: &str) -> Position {
    let inside = app.overlay().expect("a box").inner(Margin::new(1, 1));
    let rows = tree(app, world);
    let column = rows[row]
        .chars()
        .collect::<Vec<_>>()
        .windows(text.chars().count())
        .position(|window| window.iter().collect::<String>() == text)
        .unwrap_or_else(|| panic!("{text:?} on row {row}: {:?}", rows[row]));
    Position::new(inside.x + column as u16, inside.y + row as u16)
}

// The arrows, as the actions they make.
const LEFT: Action = Action::Scroll { dx: -1, dy: 0 };
const RIGHT: Action = Action::Scroll { dx: 1, dy: 0 };
const UP: Action = Action::Scroll { dx: 0, dy: -1 };
const DOWN_ARROW: Action = Action::Scroll { dx: 0, dy: 1 };

/// Presses `actions`, one after another.
fn press(app: &mut App, world: &World, actions: &[Action]) {
    for &action in actions {
        app.apply(action, world);
    }
}

/// What's in reverse video on the tree, as the chosen card's label is
/// while it blinks on.
fn blinking(app: &App, world: &World) -> Option<String> {
    let buffer = render(app, world);
    let inside = app.overlay().expect("a box").inner(Margin::new(1, 1));
    (inside.top()..inside.bottom()).find_map(|y| {
        let reversed: String = (inside.left()..inside.right())
            .filter(|&x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        (!reversed.is_empty()).then_some(reversed)
    })
}

#[test]
fn k_opens_the_tree_of_the_selected_sprite_and_k_or_esc_closes_it() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let shown = screen(&app, &world);
    assert!(shown.contains("┌─ Family tree ── Sprite #14 ─"), "{shown}");
    assert!(
        shown.contains("←→↑↓ choose  enter centre on it  space pause  esc close"),
        "{shown}"
    );
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[CENTRE]), "Sprite #14");
    assert_eq!(
        said(&rows[CENTRE + 1]),
        "adult · male · age 90 · generation 2"
    );

    app.apply(Action::FamilyTree, &world);
    assert_eq!(app.screen(), Screen::Normal);
    app.apply(Action::FamilyTree, &world);
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(
        app.selection(),
        Some(Selection::Living(EntityId(14))),
        "closing it keeps the selection"
    );
}

#[test]
fn time_runs_as_usual_while_the_tree_is_open() {
    let world = pairs(10);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    app.apply(Action::TogglePause, &world);
    assert!(screen(&app, &world).contains("|| paused 1x"));
    assert_eq!(app.screen(), Screen::FamilyTree);
}

#[test]
fn with_nothing_selected_the_tree_says_so() {
    let world = pairs(10);
    let mut app = app_on(&world);
    app.apply(Action::FamilyTree, &world);
    assert_eq!(app.screen(), Screen::FamilyTree);
    let shown = screen(&app, &world);
    assert!(shown.contains("┌─ Family tree ─"), "{shown}");
    assert!(
        shown.contains("No sprite selected: click one, or press Tab"),
        "{shown}"
    );
}

#[test]
fn a_sprite_s_parents_are_above_it_and_theirs_above_them() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(16));
    let rows = tree(&app, &world);
    // #3 is a parent of #16 and of its other parent, #12, so on the tree
    // twice.
    assert_eq!(
        said(&rows[GRANDPARENTS]),
        "one of the first sprites  Sprite #3  Sprite #4"
    );
    assert_eq!(
        said(&rows[GRANDPARENTS + 1]),
        "adult · 8 young  adult · 6 young"
    );
    assert_eq!(rows[GRANDPARENTS_JOIN].trim(), "└───────────┬───────────┘");
    assert_eq!(said(&rows[PARENTS]), "Sprite #3  Sprite #12");
    assert_eq!(said(&rows[PARENTS + 1]), "adult · 8 young  adult · 2 young");
    assert_eq!(
        rows[PARENTS_JOIN].trim(),
        "└───────────────────────┬───────────────────────┘"
    );
    assert_eq!(said(&rows[CENTRE]), "Sprite #16");
    assert_eq!(
        said(&rows[CENTRE + 1]),
        "adult · male · age 60 · generation 3"
    );
    let column = |row: &str, c: char| row.chars().position(|at| at == c);
    assert_eq!(
        column(&rows[PARENTS_JOIN], '┬'),
        column(&rows[DOWN], '│'),
        "one line down the middle"
    );
}

#[test]
fn a_sprite_s_young_are_below_it_by_their_other_parent() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let rows = tree(&app, &world);
    assert_eq!(
        rows[BROODS_JOIN].trim(),
        "┌───────────┴───────────────────────┐"
    );
    // #8, the sprite #14 last had an egg with, is its partner (M2 design
    // §15.4), and named so; #1 isn't.
    assert_eq!(
        world.sprite(EntityId(14)).unwrap().partner(),
        Some((EntityId(8), false))
    );
    assert_eq!(said(&rows[BROODS]), "with Sprite #1  partner Sprite #8");
    assert_eq!(
        rows[BROOD_JOIN].trim(),
        "┌───────────┴───────────┐                       │"
    );
    assert_eq!(said(&rows[YOUNG]), "Sprite #15  Sprite #19  Sprite #25");
    assert_eq!(
        said(&rows[YOUNG + 1]),
        "egg · hatches once  adult  egg · hatches in 14"
    );

    let world = lone_line(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    assert_eq!(said(&tree(&app, &world)[BROODS]), "alone");
}

/// A policy that hides from the family tree the sprites it lists.
struct Hiding(&'static [u64]);

impl InfoPolicy for Hiding {
    fn can_view(&self, panel: Panel, subject: Subject) -> bool {
        let hidden = matches!(subject, Subject::Sprite(id) if self.0.contains(&id.0));
        !(panel == Panel::FamilyTree && hidden)
    }
}

#[test]
fn a_sprite_the_policy_hides_is_a_blank_card_that_leads_nowhere() {
    // M2 design §17.6: #14's young #19, and #8, the other parent of #25,
    // are hidden.
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let nineteen = label_at(&app, &world, YOUNG, "Sprite #19");
    let twenty_five = label_at(&app, &world, YOUNG, "Sprite #25");
    app.set_policy(Hiding(&[8, 19]));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[BROODS]), "with Sprite #1", "#8 goes unnamed");
    assert_eq!(said(&rows[YOUNG]), "Sprite #15  Sprite #25");
    assert_eq!(
        label_at(&app, &world, YOUNG, "Sprite #25"),
        twenty_five,
        "#19 keeps its place"
    );
    app.apply(
        Action::Click {
            at: nineteen,
            button: Button::Left,
            amplified: false,
        },
        &world,
    );
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(14))));
}

#[test]
fn the_arrows_and_the_pointer_pass_a_hidden_card_by() {
    // M2 design §17.6: choosing a hidden sprite's card does nothing, so
    // neither the arrows nor the pointer land on it. #14's young are #15,
    // #19 and #25, with #19 right below it; #19 is hidden.
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let nineteen = label_at(&app, &world, YOUNG, "Sprite #19");
    app.set_policy(Hiding(&[19]));
    let chosen = |app: &mut App, actions: &[Action]| {
        press(app, &world, actions);
        blinking(app, &world)
    };
    assert_eq!(
        chosen(&mut app, &[Action::Point(nineteen)]).as_deref(),
        Some("Sprite #14"),
        "pointing at it chooses nothing"
    );
    assert_eq!(
        chosen(&mut app, &[DOWN_ARROW]).as_deref(),
        Some("Sprite #15"),
        "down to the nearest shown, the left one of two as near"
    );
    assert_eq!(
        chosen(&mut app, &[RIGHT]).as_deref(),
        Some("Sprite #25"),
        "along past it"
    );
    assert_eq!(chosen(&mut app, &[LEFT]).as_deref(), Some("Sprite #15"));

    // A row with every card hidden is passed by too: #14's parents, #1
    // and #2, have no parents above them, so up goes nowhere.
    app.set_policy(Hiding(&[1, 2]));
    assert_eq!(chosen(&mut app, &[UP]).as_deref(), Some("Sprite #14"));
    assert_eq!(
        chosen(&mut app, &[UP]).as_deref(),
        Some("Sprite #14"),
        "none shown above"
    );
}

#[test]
fn where_a_hidden_sprite_came_from_goes_unsaid_too() {
    // M2 design §17.6: #14's parents, #1 and #2, are both of the first
    // sprites, which each says above itself; #1 is hidden.
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let first = "one of the first sprites";
    assert_eq!(
        said(&tree(&app, &world)[GRANDPARENTS]),
        [first; 2].join("  ")
    );
    let two = label_at(&app, &world, GRANDPARENTS, first).x;
    app.set_policy(Hiding(&[1]));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[GRANDPARENTS]), first, "#2's alone");
    assert!(
        label_at(&app, &world, GRANDPARENTS, first).x > two,
        "{:?}",
        rows[GRANDPARENTS]
    );
}

#[test]
fn a_brother_or_sister_doesn_t_name_a_hidden_parent_it_shares() {
    // M2 design §17.6: #16's brothers and sisters share only #3 with it,
    // and say so, until #3 is hidden.
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(16));
    assert!(said(&tree(&app, &world)[SIBLINGS + 2]).starts_with("through Sprite #3"));
    app.set_policy(Hiding(&[3]));
    let rows = tree(&app, &world);
    assert_eq!(
        said(&rows[SIBLINGS]),
        "Sprite #5  Sprite #7  Sprite #9  Sprite #12  ►"
    );
    assert_eq!(said(&rows[SIBLINGS + 2]), "");
}

#[test]
fn brothers_and_sisters_are_at_the_foot_with_the_parent_they_share() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[SIBLINGS_HEADING]), "BROTHERS AND SISTERS");
    assert_eq!(
        said(&rows[SIBLINGS]),
        "Sprite #6  Sprite #8  Sprite #10  Sprite #11  ►"
    );
    assert_eq!(
        said(&rows[SIBLINGS + 1]),
        "adult · 2 young  adult · 1 young  adult  adult"
    );
    assert_eq!(said(&rows[SIBLINGS + 2]), "", "they share both parents");

    // #16's are #3's young with #4: they share only #3.
    app.apply(Action::Back, &world);
    tree_of(&mut app, &world, EntityId(16));
    let rows = tree(&app, &world);
    assert_eq!(
        said(&rows[SIBLINGS]),
        "Sprite #5  Sprite #7  Sprite #9  Sprite #12  ►"
    );
    assert_eq!(
        said(&rows[SIBLINGS + 2]),
        "through Sprite #3  through Sprite #3  through Sprite #3  through Sprite #3"
    );
}

#[test]
fn a_sprite_with_no_parents_says_where_it_came_from() {
    let world = lone_line(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[PARENTS]), "one of the first sprites");
    assert_eq!(rows[PARENTS_JOIN], "", "no line up to nothing");

    let mut world = lone_line(0);
    world.submit(Command::SpawnSprite {
        tile: at(8, 1),
        genome: None,
    });
    world.step();
    let placed = world.sprite_at(at(8, 1)).expect("placed").id();
    let mut app = app_on(&world);
    tree_of(&mut app, &world, placed);
    assert_eq!(said(&tree(&app, &world)[PARENTS]), "placed by the Cursor");
}

#[test]
fn more_young_than_fit_scroll_sideways_as_the_arrows_go() {
    let world = lone_line(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    let young = |app: &App| {
        let rows = tree(app, &world);
        (said(&rows[YOUNG]), said(&rows[YOUNG + 1]))
    };
    assert_eq!(
        young(&app).0,
        "Sprite #2  Sprite #3  Sprite #7  Sprite #12  ►"
    );
    // Down to the young nearest the middle, then along.
    press(&mut app, &world, &[DOWN_ARROW, RIGHT, RIGHT, RIGHT]);
    assert_eq!(
        young(&app).0,
        "◄  Sprite #3  Sprite #7  Sprite #12  Sprite #25  ►"
    );
    press(&mut app, &world, &[RIGHT, RIGHT, RIGHT]);
    assert_eq!(
        young(&app),
        (
            "◄  Sprite #7  Sprite #12  Sprite #25  Sprite #45".to_string(),
            "adult · 3 young  adult · 2 young  adult  egg · hatches once".to_string()
        ),
        "no further than the last"
    );
    press(&mut app, &world, &[LEFT, LEFT, LEFT, LEFT]);
    assert_eq!(
        young(&app).0,
        "◄  Sprite #3  Sprite #7  Sprite #12  Sprite #25  ►"
    );
}

#[test]
fn the_arrows_choose_a_sprite_on_the_tree_and_it_blinks() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    assert!(app.flash_on());
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #14"));
    let chosen = |app: &mut App, actions: &[Action]| {
        press(app, &world, actions);
        blinking(app, &world)
    };
    // Up to the nearer parent, the first if they're as near.
    assert_eq!(chosen(&mut app, &[UP]).as_deref(), Some("Sprite #1"));
    assert_eq!(chosen(&mut app, &[RIGHT]).as_deref(), Some("Sprite #2"));
    assert_eq!(
        chosen(&mut app, &[UP]).as_deref(),
        Some("Sprite #2"),
        "none above"
    );
    assert_eq!(
        chosen(&mut app, &[DOWN_ARROW]).as_deref(),
        Some("Sprite #14")
    );
    assert_eq!(
        chosen(&mut app, &[DOWN_ARROW]).as_deref(),
        Some("Sprite #19")
    );
    assert_eq!(chosen(&mut app, &[RIGHT]).as_deref(), Some("Sprite #25"));
    assert_eq!(
        chosen(&mut app, &[DOWN_ARROW]).as_deref(),
        Some("Sprite #10"),
        "the left one of two as near"
    );
    assert_eq!(chosen(&mut app, &[LEFT]).as_deref(), Some("Sprite #8"));
    assert_eq!(chosen(&mut app, &[UP]).as_deref(), Some("Sprite #15"));

    // It blinks about twice a second.
    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.push(blinking(&app, &world).is_some());
        app.animate(Duration::from_millis(250));
    }
    assert_eq!(seen, [true, true, false, false]);
}

#[test]
fn a_sprite_on_the_tree_twice_is_chosen_where_the_arrows_put_it() {
    // #12 is #16's parent, and its brother as well, through #3.
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(16));
    // Down to #20, its young, then to #7, nearest below, and along.
    press(&mut app, &world, &[DOWN_ARROW, DOWN_ARROW, RIGHT, RIGHT]);
    let rows = tree(&app, &world);
    let brother = label_at(&app, &world, SIBLINGS, "Sprite #12");
    let parent = label_at(&app, &world, PARENTS, "Sprite #12");
    let buffer = render(&app, &world);
    let reversed = |cell: Position| {
        buffer[(cell.x, cell.y)]
            .modifier
            .contains(Modifier::REVERSED)
    };
    assert!(reversed(brother), "{rows:#?}");
    assert!(!reversed(parent), "only the card chosen blinks");
    app.apply(LEFT, &world);
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #9"));
}

#[test]
fn enter_walks_the_tree_to_the_sprite_chosen_and_selects_it() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    press(&mut app, &world, &[UP, Action::Enter]);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(1))));
    assert_eq!(app.screen(), Screen::FamilyTree, "still open");
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[CENTRE]), "Sprite #1");
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #1"));
    assert!(said(&rows[YOUNG]).contains("Sprite #6"), "{rows:#?}");

    // `Tab` selects the next sprite, as ever, and the tree follows.
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(2))));
    assert_eq!(said(&tree(&app, &world)[CENTRE]), "Sprite #2");
}

#[test]
fn pointing_at_a_card_chooses_it_and_clicking_walks_to_it() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    let mother = label_at(&app, &world, PARENTS, "Sprite #1");
    app.apply(Action::Point(mother), &world);
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #1"));
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(14))));

    // Anywhere on a card: here, the last line of a young one's text.
    let young = label_at(&app, &world, YOUNG, "Sprite #25");
    let below = Position::new(young.x + 4, young.y + 2);
    app.apply(
        Action::Click {
            at: below,
            button: Button::Left,
            amplified: false,
        },
        &world,
    );
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(25))));
    assert_eq!(
        said(&tree(&app, &world)[CENTRE + 1]),
        "egg · hatches in 14 · generation 3"
    );
}

#[test]
fn the_pointer_on_the_tree_leaves_the_cursor_where_it_is_on_the_map() {
    // As the population graph's does (M2 design §14.2): the pointer
    // chooses and walks the tree, and the Cursor beneath stays put, on its
    // tile and in reach. The pointer points at the tile up and left of it
    // (design v27 §6.5).
    let world = pairs(230);
    let mut app = app_on(&world);
    let tile = at(9, 4);
    app.apply(Action::Point(pointer_on(&app, tile)), &world);
    assert_eq!(app.shown_cursor(), Some(tile));
    tree_of(&mut app, &world, EntityId(14));
    for (row, label) in [(PARENTS, "Sprite #1"), (YOUNG, "Sprite #15")] {
        let card = label_at(&app, &world, row, label);
        app.apply(Action::Point(card), &world);
        assert_eq!(blinking(&app, &world).as_deref(), Some(label));
        assert_eq!(app.shown_cursor(), Some(tile), "pointing at {label}");
    }
    let young = label_at(&app, &world, YOUNG, "Sprite #25");
    app.apply(
        Action::Click {
            at: young,
            button: Button::Left,
            amplified: false,
        },
        &world,
    );
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(25))));
    assert_eq!(app.shown_cursor(), Some(tile), "walking by a click");
    app.apply(Action::FamilyTree, &world);
    assert_eq!(app.shown_cursor(), Some(tile), "and closed");
}

#[test]
fn the_dead_are_dark_gray_and_say_how_they_died() {
    let world = parched_line(140);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(5));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[GRANDPARENTS]), "Sprite #1");
    assert_eq!(said(&rows[GRANDPARENTS + 1]), "died of dehydration");
    assert_eq!(said(&rows[PARENTS]), "Sprite #3");
    assert_eq!(said(&rows[PARENTS + 1]), "died of dehydration");
    let buffer = render(&app, &world);
    let colour = |cell: Position| buffer[(cell.x, cell.y)].fg;
    let parent = label_at(&app, &world, PARENTS, "Sprite #3");
    assert_eq!(colour(parent), Color::DarkGray);
    assert_eq!(
        colour(Position::new(parent.x, parent.y + 1)),
        Color::DarkGray
    );
    let centre = label_at(&app, &world, CENTRE, "Sprite #5");
    assert_eq!(colour(centre), Color::Reset, "the living as ever");
}

#[test]
fn walking_to_a_sprite_or_an_egg_on_the_map_centres_the_map_on_it() {
    // M2 design §17.3, on a field bigger than the map view. By tick 45,
    // #1 has two young: #2, grown up, and #3, an egg lying beside her.
    let [quick, eggs] = QUICK;
    let world = grow(
        &[quick, eggs, LONE],
        (120, 50),
        &[(at(60, 25), LONE_MOTHER)],
        45,
    );
    let mut app = app_on(&world);
    let tiles = ui::areas(Size::new(100, 30)).tiles;
    let middle = Position::new(tiles.x + tiles.width / 2, tiles.y + tiles.height / 2);
    let egg = world.egg(EntityId(3)).and_then(|egg| egg.pos());
    let brother = world.sprite(EntityId(2)).map(|sprite| sprite.pos());
    // `Tab` selects #1, then the map view is scrolled off to its top left
    // corner.
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(1))));
    app.apply(Action::Scroll { dx: -60, dy: -30 }, &world);
    assert_eq!(egg.and_then(|egg| app.cell_of(egg)), None, "out of view");
    app.apply(Action::FamilyTree, &world);
    press(&mut app, &world, &[DOWN_ARROW, RIGHT, Action::Enter]);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(3))));
    assert_eq!(egg.and_then(|egg| app.cell_of(egg)), Some(middle));
    // Down from the egg, which has no young, to its brother.
    press(&mut app, &world, &[DOWN_ARROW, Action::Enter]);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(2))));
    assert_eq!(brother.and_then(|at| app.cell_of(at)), Some(middle));
}

#[test]
fn walking_to_a_dead_sprite_selects_it_as_one_that_died() {
    let world = parched_line(140);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(5));
    press(&mut app, &world, &[UP, Action::Enter]);
    assert_eq!(
        app.selection(),
        Some(Selection::Dead {
            id: EntityId(3),
            cause: DeathCause::Dehydration,
            age: 85,
        })
    );
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[CENTRE]), "Sprite #3");
    assert_eq!(
        said(&rows[CENTRE + 1]),
        "died of dehydration at age 85 · generation 2"
    );
    app.apply(Action::FamilyTree, &world);
    open(&mut app, &world, Tab::Body);
    let shown = screen(&app, &world);
    assert!(
        shown.contains("Sprite #3 died of dehydration at age 85"),
        "{shown}"
    );
}

#[test]
fn a_sprite_that_died_before_it_grew_up_died_young() {
    // A mother whose young grow up 100 ticks after they hatch, with no
    // water: her first, #2, hatches on tick 21 and dies young.
    let [quick, eggs] = QUICK;
    let genes = changed(LONE_MOTHER, "value: 10.0", "value: 100.0");
    let mut world = grow(
        &[quick, eggs, LONE, SCORCHED],
        (12, 6),
        &[(at(1, 4), &genes)],
        30,
    );
    let mut app = app_on(&world);
    for _ in 0..2 {
        app.apply(Action::SelectNext, &world);
    }
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(2))));
    run(&mut app, &mut world, 30);
    assert!(matches!(app.selection(), Some(Selection::Dead { .. })));
    app.apply(Action::FamilyTree, &world);
    let rows = tree(&app, &world);
    assert_eq!(
        said(&rows[CENTRE + 1]),
        "died young of dehydration at age 35 · generation 2"
    );
    assert_eq!(said(&rows[PARENTS + 1]), "died of dehydration");
}

#[test]
fn a_sprite_hurt_to_death_died_hurt_by_what_hurt_it() {
    // M2 design §17.2: `died hurt by thornbush`. A mother who lays alone
    // at once and hits the thornbush beside her, whose thorns kill at
    // once; her young hatches, does as she did, and dies before it grows
    // up.
    let [quick, eggs] = QUICK;
    let pack = pack_changing(&[quick, eggs, LONE, LETHAL_THORNS]);
    let genes = changed(
        LONE_MOTHER,
        "])",
        r#"    AttentionInstinct(input: "always", category: "bush", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_bush", false)], verb: Hit, weight: 0.5),
])"#,
    );
    let map = Map::from_ascii(&["............"; 6], &pack).expect("valid drawing");
    let mother = Genome::from_ron(&genes, &pack).expect("valid genes");
    let scenario = Scenario {
        map,
        objects: &[(at(2, 4), "thornbush")],
        sprites: &[(at(1, 4), Some(mother))],
        scripted: &[],
        newborns: &[],
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let mother = world.sprites().next().expect("the mother").id();
    let young = loop {
        if let Some(egg) = world.eggs().next() {
            break egg.id();
        }
        assert!(world.tick() < 100, "she lays");
        world.step();
    };
    while world.sprite(young).is_none() {
        assert!(world.tick() < 100, "her egg hatches");
        world.step();
    }
    let mut app = app_on(&world);
    select(&mut app, &world, young);
    while world.dead(young).is_none() {
        assert!(world.tick() < 200, "her young dies");
        run(&mut app, &mut world, 1);
    }
    assert!(world.dead(mother).is_some(), "she died");
    app.apply(Action::FamilyTree, &world);
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[PARENTS]), format!("Sprite #{}", mother.0));
    assert_eq!(said(&rows[PARENTS + 1]), "died hurt by thornbush");
    let age = group(world.dead(young).expect("it died").age());
    assert_eq!(
        said(&rows[CENTRE + 1]),
        format!("died young, hurt by thornbush at age {age} · generation 2")
    );
}

/// A save from before the family tree: its world remembers no sprite
/// that died. #1 and #2 had died; #3 is #2's young, and #4 #1's.
const BEFORE_THE_TREE: &[u8] =
    include_bytes!("../../terra-sim/tests/golden/save-v1-before-lineage.tspr");

#[test]
fn a_sprite_that_died_before_the_world_remembered_the_dead_lived_long_ago() {
    let world = World::load(BEFORE_THE_TREE).expect("the old save loads");
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(3));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[GRANDPARENTS]), "", "nothing known above it");
    assert_eq!(said(&rows[PARENTS]), "Sprite #2");
    assert_eq!(said(&rows[PARENTS + 1]), "lived long ago");
    press(&mut app, &world, &[UP, Action::Enter]);
    assert_eq!(
        app.refusal(),
        Some("Sprite #2 lived long ago: the world no longer remembers it")
    );
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(3))));
}

#[test]
fn a_dead_sprite_keeps_its_name_through_a_save_and_a_load() {
    let mut world = parched_line(0);
    world.submit(Command::Rename {
        sprite: EntityId(1),
        name: "Bramble".into(),
    });
    for _ in 0..140 {
        world.step();
    }
    // A screen that never saw it named, as after a load, names it from
    // what the world remembers.
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(5));
    assert_eq!(said(&tree(&app, &world)[GRANDPARENTS]), "Bramble #1");

    let folder = scratch_folder("names");
    app.set_save_folder(folder);
    app.apply(Action::FamilyTree, &world);
    app.apply(Action::Quicksave, &world);
    app.apply(Action::Quickload, &world);
    let world = app.take_loaded().expect("the quicksave loaded");
    tree_of(&mut app, &world, EntityId(5));
    press(
        &mut app,
        &world,
        &[UP, UP, Action::Enter, Action::FamilyTree],
    );
    open(&mut app, &world, Tab::Body);
    let shown = screen(&app, &world);
    assert!(
        shown.contains("Bramble #1 died of dehydration at age 94"),
        "{shown}"
    );
}

#[test]
fn after_a_load_the_log_names_a_dead_sprite_as_she_was_named() {
    // M2 design §17.3: Bramble lays an egg, dies of thirst on tick 35, and
    // her egg hatches on tick 60, after a save and a load on tick 40, on a
    // screen that never saw her named.
    let [quick, _] = QUICK;
    let slow: Changes = (
        "objects.ron",
        &[(
            "ticks: (750, 750), next: Hatch",
            "ticks: (60, 60), next: Hatch",
        )],
    );
    let mut world = grow(
        &[quick, slow, LONE, SCORCHED],
        (12, 6),
        &[(at(1, 4), LONE_MOTHER)],
        0,
    );
    world.submit(Command::Rename {
        sprite: EntityId(1),
        name: "Bramble".into(),
    });
    for _ in 0..40 {
        world.step();
    }
    assert!(world.dead(EntityId(1)).is_some(), "she died");
    let mut app = app_on(&world);
    app.set_save_folder(scratch_folder("log"));
    app.apply(Action::Quicksave, &world);
    app.apply(Action::Quickload, &world);
    let mut world = app.take_loaded().expect("the quicksave loaded");
    run(&mut app, &mut world, 25);
    let shown = screen(&app, &world);
    assert!(
        shown.contains("Sprite #2 hatched (Bramble #1's)"),
        "{shown}"
    );
}

#[test]
fn the_lineage_colours_mark_the_selected_sprite_s_lineage_on_the_map() {
    let world = pairs(230);
    let mut app = app_on(&world);
    let fourteen = EntityId(14);
    tree_of(&mut app, &world, fourteen);
    app.apply(Action::FamilyTree, &world);
    app.apply(Action::CycleColours, &world);
    assert_eq!(app.colour_mode(), ColourMode::Plain);
    // The Decision marker off, so it draws over no one.
    app.animate(Duration::from_millis(500));
    let colours = |app: &App| {
        let buffer = render(app, &world);
        let cells: Vec<(EntityId, Option<Color>)> = world
            .sprites()
            .map(|sprite| {
                let drawn = app.cell_of(sprite.pos()).and_then(|cell| {
                    let cell = &buffer[(cell.x, cell.y)];
                    ["☺", "☻", "ö"].contains(&cell.symbol()).then_some(cell.fg)
                });
                (sprite.id(), drawn)
            })
            .collect();
        cells
    };
    let colour = |cells: &[(EntityId, Option<Color>)], id: EntityId| {
        cells
            .iter()
            .find(|(at, _)| *at == id)
            .and_then(|(_, fg)| *fg)
    };
    let own = colour(&colours(&app), fourteen).expect("#14 on the map");
    app.apply(Action::CycleColours, &world);
    assert_eq!(app.colour_mode(), ColourMode::Lineage);

    let cells = colours(&app);
    assert_eq!(
        colour(&cells, fourteen),
        Some(own),
        "the selected sprite keeps its own"
    );
    let parents = [EntityId(1), EntityId(2)];
    let young: Vec<EntityId> = world.young(fourteen);
    let theirs: Vec<EntityId> = young.iter().flat_map(|&id| world.young(id)).collect();
    let siblings: Vec<EntityId> = parents.iter().flat_map(|&id| world.young(id)).collect();
    let mut seen = Vec::new();
    for &(id, drawn) in &cells {
        let Some(drawn) = drawn else {
            continue;
        };
        if id == fourteen {
            continue;
        }
        let expected = if parents.contains(&id) {
            Color::LightBlue
        } else if young.contains(&id) || theirs.contains(&id) {
            Color::LightMagenta
        } else if siblings.contains(&id) {
            Color::LightRed
        } else {
            Color::DarkGray
        };
        assert_eq!(drawn, expected, "{id:?}");
        seen.push(expected);
    }
    for colour in [
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightRed,
        Color::DarkGray,
    ] {
        assert!(
            seen.contains(&colour),
            "some {colour:?} on the map: {seen:?}"
        );
    }
}

#[test]
fn a_sprite_hidden_from_the_tree_keeps_its_own_colour_in_the_lineage_colours() {
    // M2 design §17.6: of #14's parents, #1 is hidden, and #2 isn't.
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(14));
    app.apply(Action::FamilyTree, &world);
    // The Decision marker off, so it draws over no one.
    app.animate(Duration::from_millis(500));
    let colour = |app: &App, id: u64| {
        let pos = world.sprite(EntityId(id)).expect("alive").pos();
        let cell = app.cell_of(pos).expect("in view");
        let buffer = render(app, &world);
        let cell = &buffer[(cell.x, cell.y)];
        assert_eq!(cell.symbol(), "☺", "#{id} drawn");
        cell.fg
    };
    let to = |app: &mut App, mode: ColourMode| {
        while app.colour_mode() != mode {
            app.apply(Action::CycleColours, &world);
        }
    };
    to(&mut app, ColourMode::Plain);
    let own = colour(&app, 1);
    assert_ne!(own, Color::LightBlue);
    to(&mut app, ColourMode::Lineage);
    assert_eq!(
        (colour(&app, 1), colour(&app, 2)),
        (Color::LightBlue, Color::LightBlue)
    );
    app.set_policy(Hiding(&[1]));
    assert_eq!((colour(&app, 1), colour(&app, 2)), (own, Color::LightBlue));
}

/// The inspector's Body tab, opened, a line for each row inside its box
/// with its margin.
fn body_tab(app: &mut App, world: &World) -> Vec<String> {
    open(app, world, Tab::Body);
    inspector_rows(app, world, 30).1
}

#[test]
fn the_body_tab_says_a_sprite_s_generation_and_the_world_tab_the_living_ones() {
    let world = pairs(230);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    app.apply(Action::FamilyTree, &world);
    let lines = body_tab(&mut app, &world);
    assert_eq!(
        rows_from(&lines, "one of")[0],
        " one of the first sprites · generation 1"
    );
    // Too long for one line, on two (M2 design §17.5).
    tree_of(&mut app, &world, EntityId(16));
    app.apply(Action::FamilyTree, &world);
    let lines = body_tab(&mut app, &world);
    assert_eq!(
        rows_from(&lines, "parents")[..2],
        [" parents Sprite #3 and Sprite #12", " generation 3"]
    );

    open(&mut app, &world, Tab::World);
    let shown = screen(&app, &world);
    assert!(shown.contains(" generations  1 to 4"), "{shown}");
}

#[test]
fn an_egg_s_card_says_its_parents_and_generation_as_the_body_tab_does() {
    // M2 design §17.5. By tick 200, the last of #1's young, #45, is an
    // egg; the tree walks to it.
    let world = lone_line(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    press(&mut app, &world, &[DOWN_ARROW, RIGHT, RIGHT, RIGHT, RIGHT]);
    press(&mut app, &world, &[Action::Enter, Action::FamilyTree]);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(45))));
    assert!(world.egg(EntityId(45)).is_some());
    let lines = body_tab(&mut app, &world);
    assert_eq!(
        rows_from(&lines, "egg ·")[1],
        " parents Sprite #1 · generation 2"
    );
}
