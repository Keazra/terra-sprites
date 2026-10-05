//! The themes folder (design v34 §6.7): `Ctrl+T` lists the built-in
//! themes and the player's own, and picking one draws the map with it.

use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use terra_sim::{DataPack, Map, Pos, Scenario, World};
use terra_tui::app::{App, Areas, Flow, Screen};
use terra_tui::input::{Action, Keys};
use terra_tui::theme::{SemanticTile, Theme};

const ASCII_THEME: &str = include_str!("../../../themes/ascii.ron");

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn world() -> World {
    let rows = vec!["..........."; 8];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[(Pos { x: 2, y: 2 }, None)],
        scripted: &[],
    };
    World::from_scenario(scenario, pack(), 7).expect("valid scenario")
}

/// A fresh, empty themes folder.
fn scratch_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terra-themes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// An app for `world` in the default theme, reading themes from `folder`.
fn app_for(world: &World, folder: &Path) -> App {
    let areas = Areas {
        tiles: Rect::new(1, 2, 40, 12),
        inspector: None,
        event_log: None,
        overlay: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), world.seed(), areas);
    app.set_theme_folder(folder.to_path_buf());
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

/// The ascii theme with its sprite drawn as `glyph`.
fn theme_with_sprite(glyph: char) -> String {
    let from = "sprite:                 (glyph: '@'";
    assert!(ASCII_THEME.contains(from));
    ASCII_THEME.replacen(
        from,
        &format!("sprite:                 (glyph: '{glyph}'"),
        1,
    )
}

#[test]
fn ctrl_t_opens_the_themes_menu() {
    let mut keys = Keys::with_release_reporting(false);
    let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
    assert_eq!(keys.action_for(key), Some(Action::OpenThemes));
}

#[test]
fn the_menu_lists_the_built_in_themes_then_the_folder_s_files_by_name() {
    let folder = scratch_folder("list");
    std::fs::write(folder.join("night.ron"), theme_with_sprite('N')).unwrap();
    std::fs::write(folder.join("bright.ron"), theme_with_sprite('B')).unwrap();
    std::fs::write(folder.join("notes.txt"), "not a theme").unwrap();
    std::fs::create_dir_all(folder.join("old.ron")).unwrap();
    let world = world();
    let mut app = app_for(&world, &folder);

    apply(&mut app, &world, Action::OpenThemes);
    assert_eq!(app.screen(), Screen::ThemeMenu);
    assert_eq!(app.menu_title(), Some(" Themes "));
    assert_eq!(
        app.menu_items(&world),
        ["cp437 (built in)", "ascii (built in)", "bright", "night"]
    );
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
}

#[test]
fn picking_a_theme_file_draws_the_map_with_it_at_once() {
    let folder = scratch_folder("pick");
    std::fs::write(folder.join("night.ron"), theme_with_sprite('N')).unwrap();
    let world = world();
    let mut app = app_for(&world, &folder);

    apply(&mut app, &world, Action::OpenThemes);
    apply(&mut app, &world, Action::Pick(3));
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.theme.glyph(SemanticTile::Sprite).symbol, 'N');
    assert_eq!(app.notice(), Some("Theme: night"));
}

#[test]
fn the_built_in_themes_can_be_picked_back() {
    let folder = scratch_folder("built-in");
    let world = world();
    let mut app = app_for(&world, &folder);

    apply(&mut app, &world, Action::OpenThemes);
    apply(&mut app, &world, Action::Pick(2));
    assert_eq!(app.theme.glyph(SemanticTile::Sprite).symbol, '@');
    assert_eq!(app.notice(), Some("Theme: ascii"));

    apply(&mut app, &world, Action::OpenThemes);
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.theme.glyph(SemanticTile::Sprite).symbol, '☺');
    assert_eq!(app.notice(), Some("Theme: cp437"));
}

#[test]
fn a_theme_that_doesnt_load_is_refused_and_the_theme_in_use_stays() {
    let folder = scratch_folder("broken");
    std::fs::write(folder.join("broken.ron"), theme_with_sprite('λ')).unwrap();
    let world = world();
    let mut app = app_for(&world, &folder);

    apply(&mut app, &world, Action::OpenThemes);
    apply(&mut app, &world, Action::Pick(3));
    let refusal = app.refusal().expect("refused");
    assert!(
        refusal.starts_with("Couldn't use theme broken: ") && refusal.contains("CP437"),
        "{refusal}"
    );
    assert_eq!(app.theme.glyph(SemanticTile::Sprite).symbol, '☺');
}

#[test]
fn with_no_themes_folder_the_menu_still_offers_the_built_in_themes() {
    let world = world();
    let areas = Areas {
        tiles: Rect::new(1, 2, 40, 12),
        inspector: None,
        event_log: None,
        overlay: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), world.seed(), areas);
    apply(&mut app, &world, Action::OpenThemes);
    assert_eq!(
        app.menu_items(&world),
        ["cp437 (built in)", "ascii (built in)"]
    );
}

#[test]
fn the_themes_folder_is_still_listed_after_a_load() {
    // Design v34 §6.7: the folders are the player's, not the world's, so a
    // loaded world keeps them, the themes folder as much as the saves.
    let folder = scratch_folder("after-load");
    std::fs::write(folder.join("night.ron"), theme_with_sprite('N')).unwrap();
    let world = world();
    let mut app = app_for(&world, &folder);
    app.set_save_folder(folder.join("saves"));
    apply(&mut app, &world, Action::Quicksave);
    apply(&mut app, &world, Action::Quickload);
    let loaded = app.take_loaded().expect("the quicksave loaded");

    apply(&mut app, &loaded, Action::OpenThemes);
    assert_eq!(
        app.menu_items(&loaded),
        ["cp437 (built in)", "ascii (built in)", "night"]
    );
}
