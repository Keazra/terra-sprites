//! Snapshots of whole screens (design §6.1): the help screen, the sprite
//! list and each inspector tab, in both themes, drawn on a hand-made field
//! and compared with the text files in `snapshots/`. A screen that changes
//! on purpose is written afresh with `UPDATE_SNAPSHOTS=1 cargo test -p
//! terra-tui --test snapshots`, and the new files reviewed like code.

use std::path::{Path, PathBuf};

use ratatui::layout::{Position, Size};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, Map, Pos, Scenario, ScriptedAction, World};
use terra_tui::app::{App, Tab};
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

/// A field with grass, dirt, sand, water and rock, a berry bush, a berry,
/// a ball and a thornbush, and three starter sprites, one wandering to the
/// water's edge, three ticks on.
fn field() -> World {
    let pack = DataPack::builtin().expect("built-in data pack is valid");
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
    let map = Map::from_ascii(&drawing, &pack).expect("valid drawing");
    let objects = [
        (Pos { x: 1, y: 1 }, "berry_bush"),
        (Pos { x: 2, y: 2 }, "berry"),
        (Pos { x: 9, y: 6 }, "ball"),
        (Pos { x: 12, y: 1 }, "thornbush"),
    ];
    let sprites = [
        (Pos { x: 3, y: 5 }, None),
        (Pos { x: 8, y: 2 }, None),
        (Pos { x: 15, y: 6 }, None),
    ];
    let scripted = [(
        Pos { x: 3, y: 5 },
        ScriptedAction::Wander {
            destination: Pos { x: 17, y: 4 },
        },
    )];
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &sprites,
        scripted: &scripted,
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    for _ in 0..3 {
        world.step();
    }
    world
}

/// An app in `theme` on `world` with the wanderer selected, with a fixed
/// folder for its files.
fn app_for(world: &World, theme: Theme) -> App {
    let areas = ui::areas(Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), theme, 7, areas);
    app.set_data_folder(PathBuf::from("/home/kel/.local/share/terra-sprites"));
    app.apply(Action::SelectNext, world);
    app
}

/// The screen's text, a line per row, without the spaces at each line's
/// end.
fn screen(app: &App, world: &World) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        let row: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        text.push_str(row.trim_end());
        text.push('\n');
    }
    text
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
        // The Cursor in the field, for the status line.
        app.apply(Action::Point(Position::new(10, 8)), &world);
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
