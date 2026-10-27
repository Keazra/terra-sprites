//! `settings.ron` (design §6.7, #307): what the player has changed on the
//! settings screen, saved in the game's folder each time a row changes or
//! `o` is pressed, and read at start. Never part of a world's save.

mod common;
use common::{apply, every_row, map_app, on_row, scratch_folder, settings_world};

use std::path::{Path, PathBuf};

use ratatui::layout::Rect;
use terra_sim::World;
use terra_tui::app::App;
use terra_tui::args::Args;
use terra_tui::input::Action;
use terra_tui::settings::{self, SavedSettings, Setting, Settings};
use terra_tui::start;

const ASCII_THEME: &str = include_str!("../../../themes/ascii.ron");

#[test]
fn the_file_holds_only_what_the_player_has_changed() {
    let colours = SavedSettings {
        colours: Some("mono".into()),
        ..SavedSettings::default()
    };
    let text = colours.to_ron();
    assert!(text.contains("colours: \"mono\","), "{text}");
    for unchanged in ["glyphs", "motion", "square_tiles"] {
        assert!(!text.contains(unchanged), "{text}");
    }
    assert_eq!(SavedSettings::from_ron(&text), Ok(colours));
    let nothing = SavedSettings::default().to_ron();
    assert_eq!(
        SavedSettings::from_ron(&nothing),
        Ok(SavedSettings::default())
    );
}

#[test]
fn each_row_is_written_by_name_a_theme_file_by_its_file_s() {
    let text = every_row().to_ron();
    for line in [
        "glyphs: \"ASCII\",",
        "colours: \"night.ron\",",
        "motion: false,",
        "square_tiles: true,",
    ] {
        assert!(text.contains(line), "{line} in\n{text}");
    }
    assert_eq!(SavedSettings::from_ron(&text), Ok(every_row()));
    // Whatever a file's name holds.
    let odd = SavedSettings {
        glyphs: Some("my \"best\" \\ one.ron".into()),
        ..SavedSettings::default()
    };
    assert_eq!(SavedSettings::from_ron(&odd.to_ron()), Ok(odd));
}

#[test]
fn a_field_the_game_doesn_t_know_is_passed_over() {
    // A later version's rows (design §6.7), such as Look 8's.
    let text = "(bell: true, colours: \"mono\", focus: (after: [1, 2], on: Some(false)))";
    assert_eq!(
        SavedSettings::from_ron(text),
        Ok(SavedSettings {
            colours: Some("mono".into()),
            ..SavedSettings::default()
        })
    );
}

#[test]
fn a_file_that_isn_t_settings_is_refused() {
    assert!(SavedSettings::from_ron("(colours: 7)").is_err());
    assert!(SavedSettings::from_ron("colours = mono").is_err());
}

#[test]
fn with_no_file_nothing_has_been_changed() {
    let folder = scratch_folder("none");
    assert_eq!(
        settings::read(&folder.join(settings::FILE)),
        Ok(SavedSettings::default())
    );
}

#[test]
fn it_is_written_beside_its_place_and_then_moved_into_it() {
    let folder = scratch_folder("written").join("terra-sprites");
    let path = folder.join(settings::FILE);
    settings::write(&path, &every_row()).expect("written, its folder made");
    assert_eq!(settings::read(&path), Ok(every_row()));
    let names: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["settings.ron"]);
    // One that can't be written whole never breaks the one in place.
    std::fs::create_dir(folder.join("settings.ron.partial")).unwrap();
    assert!(settings::write(&path, &SavedSettings::default()).is_err());
    assert_eq!(settings::read(&path), Ok(every_row()));
}

#[test]
fn a_file_that_can_t_be_read_says_why() {
    let folder = scratch_folder("unread");
    let path = folder.join(settings::FILE);
    std::fs::write(&path, "(colours: 7)").unwrap();
    assert!(settings::read(&path).is_err());
    std::fs::create_dir_all(folder.join("a folder.ron")).unwrap();
    assert!(settings::read(&folder.join("a folder.ron")).is_err());
}

// The settings screen saves what changes.

/// An app saving its settings in `folder`, reading themes from it too.
fn saving_in(world: &World, folder: &Path) -> (App, PathBuf) {
    let mut app = map_app(world, Rect::new(1, 2, 40, 12));
    app.set_theme_folder(folder.to_path_buf());
    let path = folder.join(settings::FILE);
    app.set_settings_file(path.clone());
    (app, path)
}

/// Changes row `row` of the settings screen to the next value.
fn change(app: &mut App, world: &World, row: Setting) {
    on_row(app, world, row);
    apply(app, world, Action::Enter);
    apply(app, world, Action::Back);
}

/// What `path` holds.
fn saved(path: &Path) -> SavedSettings {
    settings::read(path).expect("saved")
}

#[test]
fn each_row_changed_is_saved_and_only_what_has_changed() {
    let folder = scratch_folder("rows");
    let world = settings_world();
    let (mut app, path) = saving_in(&world, &folder);
    assert!(!path.exists(), "nothing changed yet");
    change(&mut app, &world, Setting::Colours);
    assert_eq!(
        saved(&path),
        SavedSettings {
            colours: Some("high contrast".into()),
            ..SavedSettings::default()
        }
    );
    change(&mut app, &world, Setting::Glyphs);
    change(&mut app, &world, Setting::Motion);
    change(&mut app, &world, Setting::SquareTiles);
    assert_eq!(
        saved(&path),
        SavedSettings {
            glyphs: Some("ASCII".into()),
            colours: Some("high contrast".into()),
            motion: Some(true),
            square_tiles: Some(true),
        }
    );
}

#[test]
fn a_theme_file_is_saved_by_its_file_name() {
    let folder = scratch_folder("file");
    std::fs::write(folder.join("night.ron"), ASCII_THEME).unwrap();
    let world = settings_world();
    let (mut app, path) = saving_in(&world, &folder);
    change(&mut app, &world, Setting::Glyphs);
    change(&mut app, &world, Setting::Glyphs);
    assert_eq!(saved(&path).glyphs.as_deref(), Some("night.ron"));
}

#[test]
fn pressing_o_saves_square_tiles() {
    let folder = scratch_folder("o");
    let world = settings_world();
    let (mut app, path) = saving_in(&world, &folder);
    apply(&mut app, &world, Action::Zoom);
    assert_eq!(saved(&path).square_tiles, Some(true));
    apply(&mut app, &world, Action::Zoom);
    assert_eq!(saved(&path).square_tiles, Some(false));
}

#[test]
fn a_theme_refused_saves_nothing() {
    let folder = scratch_folder("refused");
    std::fs::write(folder.join("broken.ron"), "not a theme").unwrap();
    let world = settings_world();
    let (mut app, path) = saving_in(&world, &folder);
    change(&mut app, &world, Setting::Glyphs);
    change(&mut app, &world, Setting::Glyphs);
    assert!(app.refusal().is_some());
    assert_eq!(saved(&path).glyphs.as_deref(), Some("ASCII"));
}

#[test]
fn settings_that_can_t_be_saved_say_so_and_still_change() {
    let folder = scratch_folder("unsaved");
    let world = settings_world();
    let (mut app, path) = saving_in(&world, &folder);
    std::fs::create_dir_all(&path).unwrap();
    change(&mut app, &world, Setting::Colours);
    let refusal = app.refusal().expect("said");
    assert!(
        refusal.starts_with("Couldn't save the settings: "),
        "{refusal}"
    );
    assert_eq!(
        app.look().palette(),
        &terra_tui::theme::Pick::BuiltIn(terra_tui::theme::ColourPalette::HighContrast)
    );
}

/// The settings the game starts with from the `settings.ron` at `path`, as
/// `main.rs` reads it, with no flags.
fn started_from(path: &Path, world: &World) -> Settings {
    let saved = settings::read(path);
    let folder = path.parent();
    let (settings, _) =
        start::settings(&Args::default(), saved, folder, None, world.data()).expect("started");
    settings
}

#[test]
fn settings_that_couldn_t_be_read_are_kept_aside_before_the_first_save() {
    // Design §6.7: what the player wrote in it isn't lost to the next row
    // changed.
    let folder = scratch_folder("kept");
    let path = folder.join(settings::FILE);
    let broken = folder.join("settings.ron.broken");
    std::fs::write(&path, "(colours: 7)").unwrap();
    std::fs::write(&broken, "an older one").unwrap();
    let world = settings_world();
    let (mut app, _) = saving_in(&world, &folder);
    app.use_settings(started_from(&path, &world));
    // Into another world before anything changes, as the game carries them.
    let (mut next, _) = saving_in(&world, &folder);
    next.use_settings(app.settings());
    change(&mut next, &world, Setting::Colours);
    assert_eq!(std::fs::read_to_string(&broken).unwrap(), "(colours: 7)");
    assert_eq!(
        saved(&path),
        SavedSettings {
            colours: Some("high contrast".into()),
            ..SavedSettings::default()
        }
    );
    assert_eq!(
        next.notice(),
        Some("The settings couldn't be read, so they were kept as settings.ron.broken")
    );
    // Once: the next change writes over the file just saved.
    change(&mut next, &world, Setting::Motion);
    assert_eq!(std::fs::read_to_string(&broken).unwrap(), "(colours: 7)");
    assert_eq!(saved(&path).motion, Some(false));
    assert_eq!(next.notice(), Some("Motion: off"));
    // A file that was read is written over as ever.
    let (mut again, _) = saving_in(&world, &folder);
    again.use_settings(started_from(&path, &world));
    change(&mut again, &world, Setting::SquareTiles);
    assert_eq!(std::fs::read_to_string(&broken).unwrap(), "(colours: 7)");
    assert_eq!(again.notice(), Some("Square tiles: on"));
}

#[test]
fn why_the_saved_settings_weren_t_taken_shows_as_the_world_opens() {
    let world = settings_world();
    let mut app = map_app(&world, Rect::new(1, 2, 40, 12));
    let why = "Couldn't read the settings: 1:10: Expected a boolean";
    app.note_unread_settings(why.into());
    assert_eq!(app.refusal(), Some(why));
}
