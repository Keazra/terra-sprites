//! The sprite list (`l`, design §6.1): every sprite, its age, its strongest
//! drive and what it's doing, sorted as the player chooses.

mod common;
use common::{WorldBuilder, app_on, at, pack, render, row};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::{Color, Modifier};
use terra_sim::{Command, DataPack, EntityId, Genome, Pos, ScriptedAction, World};
use terra_tui::app::{App, Flow, Screen};
use terra_tui::input::Action;
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::sprite_list::SortBy;
use terra_tui::ui;

/// A genome starting `chem` at `value`.
fn starting(chem: &str, value: f32, pack: &DataPack) -> Genome {
    let ron =
        format!(r#"(format: 1, genes: [InitialConcentration(chem: "{chem}", value: {value})])"#);
    Genome::from_ron(&ron, pack).expect("a valid genome")
}

/// A 120×60 field with four newborn sprites, by ID: #1 at (2, 3) with no strong
/// drive, wandering to (8, 3); #2 at (110, 55), very hungry; #3 at (6, 1),
/// thirsty; #4 at (4, 4), a little hungry. #1 is named Zed, and #3 Abel.
fn field() -> World {
    let pack = pack();
    let sprites = vec![
        (at(2, 3), None),
        (at(110, 55), Some(starting("hunger", 0.9, &pack))),
        (at(6, 1), Some(starting("thirst", 0.7, &pack))),
        (at(4, 4), Some(starting("hunger", 0.6, &pack))),
    ];
    let wander = ScriptedAction::Wander {
        destination: at(8, 3),
    };
    WorldBuilder::grass(120, 60)
        .sprites_with(sprites)
        .scripts(&[(at(2, 3), wander)])
        .newborns(&[at(2, 3), at(110, 55), at(6, 1), at(4, 4)])
        .data(pack)
        .seed(7)
        .build()
}

/// The field's sprites' IDs, in ID order.
fn ids(world: &World) -> Vec<EntityId> {
    world.sprites().map(|sprite| sprite.id()).collect()
}

/// An app on `world` a tick on, which has seen #1 named Zed and #3 Abel.
fn named(world: &mut World) -> App {
    let mut app = app_on(world);
    let ids = ids(world);
    for (sprite, name) in [(ids[0], "Zed"), (ids[2], "Abel")] {
        world.submit(Command::Rename {
            sprite,
            name: name.into(),
        });
    }
    let events = world.step();
    app.record(&events, world);
    app
}

/// The labels of the list's rows, top to bottom, as drawn.
fn listed(buffer: &Buffer) -> Vec<String> {
    (3..28)
        .map(|y| row(buffer, y))
        .take_while(|line| line.starts_with("│ "))
        .map(|line| {
            let label: String = line.chars().skip(2).take(9).collect();
            label.trim().to_string()
        })
        .take_while(|label| !label.is_empty())
        .collect()
}

/// The row of the list that names `label`.
fn row_of(buffer: &Buffer, label: &str) -> u16 {
    (3..28)
        .find(|&y| row(buffer, y).starts_with(&format!("│ {label} ")))
        .unwrap_or_else(|| panic!("no row for {label}"))
}

#[test]
fn l_lists_every_sprite_over_the_panels_with_its_age_stage_drive_and_doing() {
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SpriteList, &world);
    assert_eq!(app.screen(), Screen::SpriteList);
    let screen = render(&app, &world);
    assert!(
        row(&screen, 0).starts_with(" Terra Sprites"),
        "the top bar stays"
    );
    let title = row(&screen, 1);
    assert!(
        title.starts_with("┌─ Sprites ── sorted by number ─"),
        "{title}"
    );
    assert!(title.ends_with(" esc close ─┐"), "{title}");
    let heading = row(&screen, 2);
    assert!(
        heading.starts_with("│ Sprite     Age  Life stage  Drive"),
        "{heading}"
    );
    assert!(heading.contains("Doing"), "{heading}");
    assert_eq!(
        listed(&screen),
        ["Zed #1", "Sprite #2", "Abel #3", "Sprite #4"]
    );
    let first = row(&screen, 3);
    assert!(
        first.starts_with("│ Zed #1       1  young       -"),
        "{first}"
    );
    assert!(first.contains("Wandering"), "the Body tab's line: {first}");
    let hungry = row_of(&screen, "Sprite #2");
    assert!(row(&screen, hungry).contains("hunger"));
    let x = row(&screen, hungry).find("hunger").unwrap() as u16;
    let x = row(&screen, hungry)[..x as usize].chars().count() as u16;
    assert_eq!(screen[(x, hungry)].fg, Color::Yellow, "in the map's colour");
    let thirsty = row_of(&screen, "Abel #3");
    assert!(row(&screen, thirsty).contains("thirst"));
    let status = row(&screen, 29);
    assert!(
        status.ends_with("↑↓ choose  enter go to it  tab sort  esc close"),
        "{status}"
    );
    app.apply(Action::SpriteList, &world);
    assert_eq!(app.screen(), Screen::Normal, "l closes it");
    app.apply(Action::SpriteList, &world);
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal, "and so does Esc");
}

#[test]
fn tab_sorts_by_number_then_name_then_age_then_drive() {
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SpriteList, &world);
    assert_eq!(app.list_sort(), SortBy::Number);
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.list_sort(), SortBy::Name);
    let screen = render(&app, &world);
    assert!(row(&screen, 1).starts_with("┌─ Sprites ── sorted by name ─"));
    assert_eq!(
        listed(&screen),
        ["Abel #3", "Zed #1", "Sprite #2", "Sprite #4"],
        "named first, by name, then by number"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.list_sort(), SortBy::Age);
    assert_eq!(
        listed(&render(&app, &world)),
        ["Zed #1", "Sprite #2", "Abel #3", "Sprite #4"],
        "oldest first, and of the same age, by number"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.list_sort(), SortBy::Drive);
    assert_eq!(
        listed(&render(&app, &world)),
        ["Sprite #2", "Sprite #4", "Abel #3", "Zed #1"],
        "hunger, strongest first, then thirst, then none"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(app.list_sort(), SortBy::Number, "and round again");
    app.apply(Action::SelectPrevious, &world);
    assert_eq!(app.list_sort(), SortBy::Drive, "Shift+Tab goes back");
}

#[test]
fn the_highlight_moves_with_the_arrows_and_stays_on_its_sprite_when_resorted() {
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SpriteList, &world);
    assert_eq!(app.list_row(&world), 0, "the first, with nothing selected");
    let highlighted = |app: &App| {
        let screen = render(app, &world);
        let rows: Vec<u16> = (3..28)
            .filter(|&y| screen[(2, y)].modifier.contains(Modifier::REVERSED))
            .collect();
        assert_eq!(rows.len(), 1, "one row highlighted");
        assert!(
            screen[(97, rows[0])].modifier.contains(Modifier::REVERSED),
            "across the list"
        );
        let line = row(&screen, rows[0]);
        line.chars()
            .skip(2)
            .take(9)
            .collect::<String>()
            .trim()
            .to_string()
    };
    assert_eq!(highlighted(&app), "Zed #1");
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    app.apply(Action::Scroll { dx: 0, dy: 5 }, &world);
    assert_eq!(highlighted(&app), "Abel #3", "one row at a time");
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    assert_eq!(
        highlighted(&app),
        "Sprite #4",
        "and no further than the last"
    );
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        highlighted(&app),
        "Sprite #4",
        "resorted, it's the same sprite"
    );
    assert_eq!(app.list_row(&world), 3);
    app.apply(Action::Scroll { dx: 0, dy: -9 }, &world);
    assert_eq!(highlighted(&app), "Sprite #2", "up a row");
}

#[test]
fn opening_the_list_highlights_the_selected_sprite() {
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::SpriteList, &world);
    assert_eq!(app.list_row(&world), 1);
}

/// A policy that keeps one sprite out of the sprite list.
struct Unlisted(EntityId);

impl InfoPolicy for Unlisted {
    fn can_view(&self, panel: Panel, subject: Subject) -> bool {
        !(panel == Panel::SpriteList && subject == Subject::Sprite(self.0))
    }
}

#[test]
fn a_selected_sprite_the_list_leaves_out_isnt_highlighted() {
    // Design §6.4: the first listed sprite is, so Enter goes to it.
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SelectNext, &world);
    let ids = ids(&world);
    app.set_policy(Unlisted(ids[0]));
    app.apply(Action::SpriteList, &world);
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    app.apply(Action::Enter, &world);
    assert_eq!(app.selection().map(|s| s.id()), Some(ids[2]));
}

#[test]
fn enter_selects_the_highlighted_sprite_centres_the_view_on_it_and_closes_the_list() {
    let mut world = field();
    let mut app = named(&mut world);
    let tiles = ui::areas(Size::new(100, 30)).tiles;
    let before = app.viewport();
    app.apply(Action::SpriteList, &world);
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    assert_eq!(app.apply(Action::Enter, &world), Flow::Continue);
    assert_eq!(app.screen(), Screen::Normal);
    let hungry = ids(&world)[1];
    assert_eq!(app.selection().map(|s| s.id()), Some(hungry));
    let pos = world.sprite(hungry).unwrap().pos();
    let view = app.viewport();
    let (width, height) = (i32::from(tiles.width), i32::from(tiles.height));
    let centre = (view.x + width / 2, view.y + height / 2);
    // As near the middle as the margin past the wall allows (M2 design
    // §11.1).
    let margin_x = view.x == -(width / 4) || view.x == 120 - width + width / 4;
    let margin_y = view.y == -(height / 4) || view.y == 60 - height + height / 4;
    assert!(
        (margin_x || centre.0 == i32::from(pos.x)) && (margin_y || centre.1 == i32::from(pos.y)),
        "{view:?} for {pos:?} in {tiles:?}"
    );
    assert!(
        view.x > before.x && view.y > before.y,
        "the view moved to it: {view:?}"
    );
}

#[test]
fn a_click_on_a_row_goes_to_its_sprite_and_a_click_elsewhere_does_nothing() {
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SpriteList, &world);
    let screen = render(&app, &world);
    app.apply(Action::left_click(Position::new(40, 2)), &world);
    assert_eq!(
        app.screen(),
        Screen::SpriteList,
        "the heading isn't a sprite"
    );
    app.apply(Action::left_click(Position::new(40, 20)), &world);
    assert_eq!(app.screen(), Screen::SpriteList, "nor is a row with none");
    let abel = row_of(&screen, "Abel #3");
    app.apply(Action::left_click(Position::new(40, abel)), &world);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.selection().map(|s| s.id()), Some(ids(&world)[2]));
}

#[test]
fn with_no_sprites_the_list_says_so() {
    let world = WorldBuilder::grass(10, 5).seed(7).build();
    let mut app = app_on(&world);
    app.apply(Action::SpriteList, &world);
    let screen = render(&app, &world);
    assert_eq!(row(&screen, 2), format!("│ No sprites{}│", " ".repeat(87)));
    app.apply(Action::Enter, &world);
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    assert_eq!(app.screen(), Screen::SpriteList, "nothing to go to");
}

#[test]
fn a_long_list_scrolls_to_keep_the_highlight_in_view() {
    let sprites: Vec<Pos> = (0..30).map(|x| at(x * 2, 10)).collect();
    let world = WorldBuilder::grass(60, 40)
        .starters(&sprites)
        .seed(7)
        .build();
    let mut app = app_on(&world);
    app.apply(Action::SpriteList, &world);
    assert_eq!(app.list_rows(), 25);
    for _ in 0..29 {
        app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    }
    assert_eq!(app.list_first(&world), 5);
    let screen = render(&app, &world);
    assert!(
        row(&screen, 3).starts_with("│ Sprite #6 "),
        "{}",
        row(&screen, 3)
    );
    assert!(
        row(&screen, 27).starts_with("│ Sprite #30 "),
        "{}",
        row(&screen, 27)
    );
    let last = ids(&world)[29];
    app.apply(Action::left_click(Position::new(10, 27)), &world);
    assert_eq!(app.selection().map(|s| s.id()), Some(last));
}

#[test]
fn the_list_is_ignored_by_other_keys_and_quit_still_quits() {
    let mut world = field();
    let mut app = named(&mut world);
    app.apply(Action::SpriteList, &world);
    for action in [
        Action::TogglePause,
        Action::Help,
        Action::NextTab,
        Action::Track,
    ] {
        app.apply(action, &world);
        assert_eq!(app.screen(), Screen::SpriteList, "{action:?}");
    }
    assert!(!app.clock.is_paused());
    assert_eq!(app.apply(Action::Quit, &world), Flow::Quit);
}
