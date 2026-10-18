//! Snapshots of whole screens (design §6.1): the help screen, the sprite
//! list, the population graph (M2 design §14), the family tree (§17) and
//! each inspector tab, in both themes, drawn on a hand-made field
//! and compared with the text files in `snapshots/`. A screen that changes
//! on purpose is written afresh with `UPDATE_SNAPSHOTS=1 cargo test -p
//! terra-tui --test snapshots`, and the new files reviewed like code.

mod common;
use common::{WorldBuilder, app_in, at, lines, pairs, render, tree_of};

use std::path::Path;

use ratatui::layout::Position;
use terra_sim::{EntityId, Pos, ScriptedAction, World};
use terra_tui::app::{App, Tab};
use terra_tui::input::Action;
use terra_tui::theme::Theme;

/// A field with grass, dirt, sand, water and rock, a berry bush, a berry,
/// a ball and a thornbush, and three starter sprites, one wandering to the
/// water's edge, three ticks on.
fn field() -> World {
    let drawing = [
        "..............,,,,::~~==",
        "..............,,,,::~~==",
        "....##........,,,,::~~==",
        "....##..............~~==",
        "....................~~==",
        "..................::~~==",
        "..................::~~==",
        "..................::~~==",
    ];
    let objects = [
        (at(1, 1), "berry_bush"),
        (at(2, 2), "berry"),
        (at(9, 6), "ball"),
        (at(12, 1), "thornbush"),
    ];
    let wander = ScriptedAction::Wander {
        destination: at(17, 4),
    };
    let mut world = WorldBuilder::new(&drawing)
        .objects(&objects)
        .starters(&[at(3, 5), at(8, 2), at(15, 6)])
        .scripts(&[(at(3, 5), wander)])
        .seed(7)
        .build();
    for _ in 0..3 {
        world.step();
    }
    world
}

/// An app in `theme` on `world` with the wanderer selected.
fn app_for(world: &World, theme: Theme) -> App {
    let mut app = app_in(world, theme);
    app.apply(Action::SelectNext, world);
    app
}

/// The screen's text, a line per row, without the spaces at each line's
/// end.
fn screen(app: &App, world: &World) -> String {
    let rows = lines(&render(app, world));
    rows.iter().map(|line| format!("{line}\n")).collect()
}

/// Compares `text` with the snapshot `name`, or with `UPDATE_SNAPSHOTS`
/// set, writes it there.
fn check(name: &str, text: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "no snapshot {}: write it with UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    if want != text {
        let line = want
            .lines()
            .zip(text.lines())
            .position(|(want, got)| want != got)
            .unwrap_or_else(|| want.lines().count().min(text.lines().count()));
        panic!(
            "{name} differs from its snapshot from line {}:\n--- want\n{}\n--- got\n{}\n\
             If the change is meant, write it with UPDATE_SNAPSHOTS=1.",
            line + 1,
            want.lines().nth(line).unwrap_or("(end)"),
            text.lines().nth(line).unwrap_or("(end)"),
        );
    }
}

/// Both themes, by name.
fn themes() -> [(&'static str, Theme); 2] {
    [("cp437", Theme::cp437()), ("ascii", Theme::ascii())]
}

#[test]
fn every_inspector_tab_matches_its_snapshot() {
    let world = field();
    for (theme_name, theme) in themes() {
        let mut app = app_for(&world, theme);
        // The Cursor in the field, on (8, 5), for the status line: the
        // pointer rests a cell down and right of it (design v27 §6.5).
        let at = app.cell_of(Pos { x: 8, y: 5 }).expect("in view");
        app.apply(Action::Point(Position::new(at.x + 1, at.y + 1)), &world);
        for tab in Tab::ALL {
            while app.tab() != tab {
                app.apply(Action::NextTab, &world);
            }
            let name = format!("{theme_name}-{}", tab.label().to_lowercase());
            check(&name, &screen(&app, &world));
        }
    }
}

#[test]
fn the_help_screen_matches_its_snapshot() {
    let world = field();
    for (theme_name, theme) in themes() {
        let mut app = app_for(&world, theme);
        app.apply(Action::Help, &world);
        check(&format!("{theme_name}-help"), &screen(&app, &world));
    }
}

#[test]
fn the_sprite_list_matches_its_snapshot() {
    let world = field();
    for (theme_name, theme) in themes() {
        let mut app = app_for(&world, theme);
        app.apply(Action::SpriteList, &world);
        check(&format!("{theme_name}-sprite-list"), &screen(&app, &world));
    }
}

#[test]
fn the_population_graph_matches_its_snapshot() {
    // Run on to tick 3,000 so the graph has 30 columns, with the moment at
    // tick 1,500 picked.
    let mut world = field();
    while world.tick() < 3_000 {
        world.step();
    }
    for (theme_name, theme) in themes() {
        let mut app = app_for(&world, theme);
        app.apply(Action::Population, &world);
        for _ in 0..15 {
            app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
        }
        check(&format!("{theme_name}-population"), &screen(&app, &world));
    }
}

#[test]
fn the_family_tree_matches_its_snapshot() {
    // Two pairs and their young, 200 ticks on: #16's parents are #3 and
    // #13, who is #3's young too, so #3 is on the tree twice.
    let world = pairs(230);
    for (theme_name, theme) in themes() {
        let mut app = app_in(&world, theme);
        tree_of(&mut app, &world, EntityId(16));
        check(&format!("{theme_name}-family-tree"), &screen(&app, &world));
    }
}
