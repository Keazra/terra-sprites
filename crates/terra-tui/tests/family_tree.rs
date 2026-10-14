//! The family tree (M2 design §16): `k` opens it over the panels, on the
//! selected sprite or egg, with its grandparents, parents, young and
//! brothers and sisters, living or dead; the arrows or the pointer choose
//! one, and `Enter` or a click walks the tree to it and selects it. The
//! lineage colours (`b`) mark the same sprites on the map, and the inspector
//! says each sprite's generation.

mod common;
use common::{LONE_MOTHER, QUICK, SCORCHED, at, grow, lone_line, pairs, parched_line};

use std::path::PathBuf;
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Margin, Position, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{Command, DeathCause, EntityId, World};
use terra_tui::app::{App, ColourMode, Screen, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;
use terra_tui::ui;

/// An app at 100×30 on `world`, in the default theme.
fn app_on(world: &World) -> App {
    App::new(
        world.map(),
        Theme::cp437(),
        7,
        ui::areas(Size::new(100, 30)),
    )
}

/// Steps `world` `ticks` ticks, with `app` watching.
fn run(app: &mut App, world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        let events = world.step();
        app.record(&events, world);
    }
}

fn render(app: &App, world: &World) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// The screen as text, a line for each row.
fn screen(app: &App, world: &World) -> String {
    let buffer = render(app, world);
    (0..buffer.area.height)
        .map(|y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            row.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
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

/// Selects `id` and opens its family tree.
fn tree_of(app: &mut App, world: &World, id: EntityId) {
    for _ in 0..world.sprites().count() {
        if app.selection().map(Selection::id) == Some(id) {
            break;
        }
        app.apply(Action::SelectNext, world);
    }
    assert_eq!(app.selection().map(Selection::id), Some(id));
    app.apply(Action::FamilyTree, world);
    assert_eq!(app.screen(), Screen::FamilyTree);
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
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(9));
    let shown = screen(&app, &world);
    assert!(shown.contains("┌─ Family tree ── Sprite #9 ─"), "{shown}");
    assert!(
        shown.contains("←→↑↓ choose  enter centre on it  space pause  esc close"),
        "{shown}"
    );
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[CENTRE]), "Sprite #9");
    assert_eq!(
        said(&rows[CENTRE + 1]),
        "adult · male · age 120 · generation 2"
    );

    app.apply(Action::FamilyTree, &world);
    assert_eq!(app.screen(), Screen::Normal);
    app.apply(Action::FamilyTree, &world);
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(
        app.selection(),
        Some(Selection::Living(EntityId(9))),
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
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(16));
    let rows = tree(&app, &world);
    // #3 is a parent of #16 and of its other parent, #13, so on the tree
    // twice.
    assert_eq!(
        said(&rows[GRANDPARENTS]),
        "one of the first sprites  Sprite #3  Sprite #4"
    );
    assert_eq!(
        said(&rows[GRANDPARENTS + 1]),
        "adult · 7 young  adult · 7 young"
    );
    assert_eq!(rows[GRANDPARENTS_JOIN].trim(), "└───────────┬───────────┘");
    assert_eq!(said(&rows[PARENTS]), "Sprite #3  Sprite #13");
    assert_eq!(said(&rows[PARENTS + 1]), "adult · 7 young  adult · 1 young");
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
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(9));
    let rows = tree(&app, &world);
    assert_eq!(
        rows[BROODS_JOIN].trim(),
        "┌───────────┴───────────────────────────────────┐"
    );
    assert_eq!(said(&rows[BROODS]), "with Sprite #10  with Sprite #3");
    assert_eq!(
        rows[BROOD_JOIN].trim(),
        "┌───────────────────────┼───────────────────────┐                       │"
    );
    assert_eq!(
        said(&rows[YOUNG]),
        "Sprite #12  Sprite #14  Sprite #22  Sprite #18"
    );
    assert_eq!(
        said(&rows[YOUNG + 1]),
        "adult · 2 young  adult · 2 young  egg · hatches in 1  adult"
    );

    let world = lone_line(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(1));
    assert_eq!(said(&tree(&app, &world)[BROODS]), "alone");
}

#[test]
fn brothers_and_sisters_are_at_the_foot_with_the_parent_they_share() {
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(9));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[SIBLINGS_HEADING]), "BROTHERS AND SISTERS");
    assert_eq!(
        said(&rows[SIBLINGS]),
        "Sprite #6  Sprite #8  Sprite #11  Sprite #17"
    );
    assert_eq!(said(&rows[SIBLINGS + 1]), "adult  adult  adult  adult");

    // #16's are #3's young with #4: they share only #3.
    app.apply(Action::Back, &world);
    tree_of(&mut app, &world, EntityId(16));
    let rows = tree(&app, &world);
    assert_eq!(
        said(&rows[SIBLINGS]),
        "Sprite #5  Sprite #7  Sprite #10  Sprite #13  ►"
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
        "Sprite #2  Sprite #4  Sprite #8  Sprite #10  ►"
    );
    // Down to the young nearest the middle, then along.
    press(&mut app, &world, &[DOWN_ARROW, RIGHT, RIGHT, RIGHT]);
    assert_eq!(
        young(&app).0,
        "◄  Sprite #4  Sprite #8  Sprite #10  Sprite #26  ►"
    );
    press(&mut app, &world, &[RIGHT, RIGHT, RIGHT]);
    assert_eq!(
        young(&app),
        (
            "◄  Sprite #10  Sprite #26  Sprite #52  Sprite #72".to_string(),
            "adult · 2 young  adult · 1 young  adult · 1 young  egg · hatches in 1".to_string()
        ),
        "no further than the last"
    );
    press(&mut app, &world, &[LEFT, LEFT, LEFT, LEFT]);
    assert_eq!(
        young(&app).0,
        "◄  Sprite #8  Sprite #10  Sprite #26  Sprite #52  ►"
    );
}

#[test]
fn the_arrows_choose_a_sprite_on_the_tree_and_it_blinks() {
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(9));
    assert!(app.flash_on());
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #9"));
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
        Some("Sprite #9")
    );
    assert_eq!(
        chosen(&mut app, &[DOWN_ARROW]).as_deref(),
        Some("Sprite #14")
    );
    assert_eq!(chosen(&mut app, &[RIGHT]).as_deref(), Some("Sprite #22"));
    assert_eq!(
        chosen(&mut app, &[DOWN_ARROW]).as_deref(),
        Some("Sprite #11")
    );
    assert_eq!(chosen(&mut app, &[LEFT]).as_deref(), Some("Sprite #8"));
    assert_eq!(chosen(&mut app, &[UP]).as_deref(), Some("Sprite #14"));

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
    // #13 is #16's parent, and its brother as well, through #3.
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(16));
    // Down to #21, its young, then to #7, nearest below, and along.
    press(&mut app, &world, &[DOWN_ARROW, DOWN_ARROW, RIGHT, RIGHT]);
    let rows = tree(&app, &world);
    let brother = label_at(&app, &world, SIBLINGS, "Sprite #13");
    let parent = label_at(&app, &world, PARENTS, "Sprite #13");
    let buffer = render(&app, &world);
    let reversed = |cell: Position| {
        buffer[(cell.x, cell.y)]
            .modifier
            .contains(Modifier::REVERSED)
    };
    assert!(reversed(brother), "{rows:#?}");
    assert!(!reversed(parent), "only the card chosen blinks");
    app.apply(LEFT, &world);
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #10"));
}

#[test]
fn enter_walks_the_tree_to_the_sprite_chosen_and_selects_it() {
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(9));
    press(&mut app, &world, &[UP, Action::Enter]);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(1))));
    assert_eq!(app.screen(), Screen::FamilyTree, "still open");
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[CENTRE]), "Sprite #1");
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #1"));
    assert!(said(&rows[YOUNG]).contains("Sprite #9"), "{rows:#?}");

    // `Tab` selects the next sprite, as ever, and the tree follows.
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(2))));
    assert_eq!(said(&tree(&app, &world)[CENTRE]), "Sprite #2");
}

#[test]
fn pointing_at_a_card_chooses_it_and_clicking_walks_to_it() {
    let world = pairs(200);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(9));
    let mother = label_at(&app, &world, PARENTS, "Sprite #1");
    app.apply(Action::Point(mother), &world);
    assert_eq!(blinking(&app, &world).as_deref(), Some("Sprite #1"));
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(9))));

    // Anywhere on a card: here, the last line of a young one's text.
    let young = label_at(&app, &world, YOUNG, "Sprite #22");
    let below = Position::new(young.x + 4, young.y + 2);
    app.apply(
        Action::Click {
            at: below,
            button: Button::Left,
            amplified: false,
        },
        &world,
    );
    assert_eq!(app.selection(), Some(Selection::Living(EntityId(22))));
    assert_eq!(
        said(&tree(&app, &world)[CENTRE + 1]),
        "egg · hatches in 1 · generation 3"
    );
}

#[test]
fn the_dead_are_dark_gray_and_say_how_they_died() {
    let world = parched_line(140);
    let mut app = app_on(&world);
    tree_of(&mut app, &world, EntityId(5));
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[GRANDPARENTS]), "Sprite #1");
    assert_eq!(said(&rows[GRANDPARENTS + 1]), "died of dehydration");
    assert_eq!(said(&rows[PARENTS]), "Sprite #4");
    assert_eq!(said(&rows[PARENTS + 1]), "died of dehydration");
    let buffer = render(&app, &world);
    let colour = |cell: Position| buffer[(cell.x, cell.y)].fg;
    let parent = label_at(&app, &world, PARENTS, "Sprite #4");
    assert_eq!(colour(parent), Color::DarkGray);
    assert_eq!(
        colour(Position::new(parent.x, parent.y + 1)),
        Color::DarkGray
    );
    let centre = label_at(&app, &world, CENTRE, "Sprite #5");
    assert_eq!(colour(centre), Color::Reset, "the living as ever");
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
            id: EntityId(4),
            cause: DeathCause::Dehydration,
            age: 84,
        })
    );
    let rows = tree(&app, &world);
    assert_eq!(said(&rows[CENTRE]), "Sprite #4");
    assert_eq!(
        said(&rows[CENTRE + 1]),
        "died of dehydration at age 84 · generation 2"
    );
    app.apply(Action::FamilyTree, &world);
    while app.tab() != Tab::Body {
        app.apply(Action::NextTab, &world);
    }
    let shown = screen(&app, &world);
    assert!(
        shown.contains("Sprite #4 died of dehydration at age 84"),
        "{shown}"
    );
}

#[test]
fn a_sprite_that_died_before_it_grew_up_died_young() {
    // A mother whose young grow up 100 ticks after they hatch, with no
    // water: her first, #2, hatches on tick 21 and dies young.
    let [quick, eggs] = QUICK;
    let genes = LONE_MOTHER.replace("value: 10.0", "value: 100.0");
    let mut world = grow(&[quick, eggs, SCORCHED], (12, 6), &[(at(1, 4), &genes)], 30);
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

/// A fresh, empty folder for saves.
fn scratch_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terra-family-tree-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
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
    while app.tab() != Tab::Body {
        app.apply(Action::NextTab, &world);
    }
    let shown = screen(&app, &world);
    assert!(
        shown.contains("Bramble #1 died of dehydration at age 95"),
        "{shown}"
    );
}

#[test]
fn the_lineage_colours_mark_the_selected_sprite_s_kin_on_the_map() {
    let world = pairs(200);
    let mut app = app_on(&world);
    let nine = EntityId(9);
    tree_of(&mut app, &world, nine);
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
    let own = colour(&colours(&app), nine).expect("#9 on the map");
    app.apply(Action::CycleColours, &world);
    assert_eq!(app.colour_mode(), ColourMode::Lineage);

    let cells = colours(&app);
    assert_eq!(
        colour(&cells, nine),
        Some(own),
        "the selected sprite keeps its own"
    );
    let parents = [EntityId(1), EntityId(2)];
    let young: Vec<EntityId> = world.young(nine);
    let theirs: Vec<EntityId> = young.iter().flat_map(|&id| world.young(id)).collect();
    let siblings: Vec<EntityId> = parents.iter().flat_map(|&id| world.young(id)).collect();
    let mut seen = Vec::new();
    for &(id, drawn) in &cells {
        let Some(drawn) = drawn else {
            continue;
        };
        if id == nine {
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
fn the_body_tab_says_a_sprite_s_generation_and_the_world_tab_the_living_ones() {
    let world = pairs(200);
    let mut app = app_on(&world);
    let body = |app: &mut App| {
        while app.tab() != Tab::Body {
            app.apply(Action::NextTab, &world);
        }
        screen(app, &world)
    };
    tree_of(&mut app, &world, EntityId(1));
    app.apply(Action::FamilyTree, &world);
    let shown = body(&mut app);
    assert!(
        shown.contains(" one of the first sprites · generation 1"),
        "{shown}"
    );
    tree_of(&mut app, &world, EntityId(16));
    app.apply(Action::FamilyTree, &world);
    let shown = body(&mut app);
    assert!(
        shown.contains(" parents Sprite #3 and Sprite #13"),
        "{shown}"
    );
    assert!(shown.contains(" generation 3"), "{shown}");

    while app.tab() != Tab::World {
        app.apply(Action::NextTab, &world);
    }
    let shown = screen(&app, &world);
    assert!(shown.contains(" generations  1 to 4"), "{shown}");
}
