//! The settings screen (design §6.7, #307): `Ctrl+T` opens a box over the
//! map, `Settings`, a row each for the glyph set, the palette, motion and
//! square tiles, each changed at once.

mod common;
use common::{
    SCREEN, WorldBuilder, apply, at, lines, map_app, pointer_on, render, scratch_folder, sized_app,
};

use std::path::Path;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position, Rect};
use terra_sim::World;
use terra_tui::app::{App, Screen};
use terra_tui::input::{Action, Button, Keys};
use terra_tui::settings::{self, Look, SavedSettings, Setting, Settings};
use terra_tui::theme::{ColourPalette, GlyphSet, Pick, SemanticTile, Theme};

const ASCII_THEME: &str = include_str!("../../../themes/ascii.ron");

/// An 11×8 grass world with a starter sprite on (2, 2).
fn world() -> World {
    WorldBuilder::grass(11, 8)
        .starters(&[at(2, 2)])
        .seed(7)
        .build()
}

/// An app for `world` in the default theme, reading themes from `folder`.
fn app_for(world: &World, folder: &Path) -> App {
    let mut app = map_app(world, Rect::new(1, 2, 40, 12));
    app.set_theme_folder(folder.to_path_buf());
    app
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

/// What the settings screen's row `row` shows just now.
fn shows(app: &App, row: Setting) -> String {
    let rows = app.settings_rows();
    let (_, value) = rows
        .iter()
        .find(|(setting, _)| *setting == row)
        .expect("a row for it");
    value.clone()
}

/// Each value row `row` goes through as `→` is pressed on it, from the one
/// after what it shows, until it's back.
fn values_of(app: &mut App, world: &World, row: Setting) -> Vec<String> {
    let first = shows(app, row);
    let mut values = Vec::new();
    loop {
        apply(app, world, Action::Scroll { dx: 1, dy: 0 });
        let value = shows(app, row);
        values.push(value.clone());
        if value == first || values.len() > 20 {
            return values;
        }
    }
}

/// Opens the settings screen on row `row`, by its number.
fn on_row(app: &mut App, world: &World, row: Setting) {
    apply(app, world, Action::OpenSettings);
    let number = Setting::ALL.iter().position(|&s| s == row).unwrap() + 1;
    apply(app, world, Action::Pick(number as u8));
}

#[test]
fn ctrl_t_opens_the_settings_screen() {
    let mut keys = Keys::with_release_reporting(false);
    let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
    assert_eq!(keys.action_for(key), Some(Action::OpenSettings));
}

#[test]
fn the_settings_screen_is_a_box_over_the_map_a_numbered_row_each() {
    let world = world();
    let mut app = sized_app(&world, Theme::cp437(), SCREEN);
    app.set_motion(true);
    apply(&mut app, &world, Action::OpenSettings);
    assert_eq!(app.screen(), Screen::Settings);
    assert_eq!(
        app.settings_rows(),
        [
            (Setting::Glyphs, "CP437".to_string()),
            (Setting::Colours, "default".to_string()),
            (Setting::Motion, "on".to_string()),
            (Setting::SquareTiles, "off".to_string()),
        ]
    );
    let screen = lines(&render(&app, &world));
    let top = screen
        .iter()
        .position(|row| row.contains("┌─ Settings ─"))
        .expect("the box");
    assert!(screen[top].contains(" esc close ─┐"), "{}", screen[top]);
    let rows = [
        "│ 1 Glyphs        ◄ CP437         ► │",
        "│ 2 Colours       ◄ default       ► │",
        "│ 3 Motion        ◄ on            ► │",
        "│ 4 Square tiles  ◄ off           ► │",
    ];
    for (n, row) in rows.iter().enumerate() {
        assert!(screen[top + 1 + n].contains(row), "{}", screen[top + 1 + n]);
    }
    // Over the map, from its top-left corner, as the menus are.
    let area = app.menu_area(&world).expect("drawn");
    let tiles = terra_tui::ui::areas(SCREEN).tiles;
    assert_eq!(area.as_position(), tiles.as_position());
    assert!(screen[top + 5].contains("└───"), "{}", screen[top + 5]);
}

#[test]
fn up_and_down_or_a_row_s_number_choose_a_row_and_change_nothing() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("choose"));
    apply(&mut app, &world, Action::OpenSettings);
    assert_eq!(app.menu_choice(), 0);
    apply(&mut app, &world, Action::Scroll { dx: 0, dy: 1 });
    assert_eq!(app.menu_choice(), 1);
    apply(&mut app, &world, Action::Pick(4));
    assert_eq!(app.menu_choice(), 3);
    apply(&mut app, &world, Action::Scroll { dx: 0, dy: -1 });
    assert_eq!(app.menu_choice(), 2);
    assert_eq!(app.screen(), Screen::Settings);
    assert_eq!(app.theme, Theme::cp437(), "nothing changed");
    assert_eq!(app.tile_width(), 1);
    assert_eq!(app.notice(), None);
}

#[test]
fn right_or_enter_changes_the_glyphs_at_once_and_left_goes_back() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("glyphs"));
    on_row(&mut app, &world, Setting::Glyphs);
    apply(&mut app, &world, Action::Scroll { dx: 1, dy: 0 });
    assert_eq!(shows(&app, Setting::Glyphs), "ASCII");
    assert_eq!(app.theme, Theme::ascii());
    assert_eq!(app.notice(), Some("Glyphs: ASCII"));
    assert_eq!(app.screen(), Screen::Settings, "it stays open");
    apply(&mut app, &world, Action::Scroll { dx: -1, dy: 0 });
    assert_eq!(app.theme, Theme::cp437());
    assert_eq!(app.notice(), Some("Glyphs: CP437"));
    apply(&mut app, &world, Action::Enter);
    assert_eq!(shows(&app, Setting::Glyphs), "ASCII");
    // Round from either end.
    apply(&mut app, &world, Action::Enter);
    assert_eq!(shows(&app, Setting::Glyphs), "CP437");
    apply(&mut app, &world, Action::Scroll { dx: -1, dy: 0 });
    assert_eq!(shows(&app, Setting::Glyphs), "ASCII");
}

#[test]
fn the_colours_row_goes_through_the_built_in_palettes_then_the_theme_files() {
    let folder = scratch_folder("colours");
    std::fs::write(folder.join("night.ron"), theme_with_sprite('N')).unwrap();
    std::fs::write(folder.join("bright.ron"), theme_with_sprite('B')).unwrap();
    std::fs::write(folder.join("notes.txt"), "not a theme").unwrap();
    std::fs::create_dir_all(folder.join("old.ron")).unwrap();
    let world = world();
    let mut app = app_for(&world, &folder);
    on_row(&mut app, &world, Setting::Colours);
    assert_eq!(
        values_of(&mut app, &world, Setting::Colours),
        [
            "high contrast",
            "colour-blind",
            "mono",
            "bright",
            "night",
            "default"
        ]
    );
    for _ in 0..3 {
        apply(&mut app, &world, Action::Enter);
    }
    assert_eq!(app.notice(), Some("Colours: mono"));
    assert_eq!(
        app.theme,
        Theme::built_in(GlyphSet::Cp437, ColourPalette::Mono)
    );
    // The glyphs keep to theirs.
    on_row(&mut app, &world, Setting::Glyphs);
    assert_eq!(
        values_of(&mut app, &world, Setting::Glyphs),
        ["ASCII", "bright", "night", "CP437"]
    );
}

#[test]
fn a_theme_file_gives_its_glyphs_on_one_row_and_its_colours_on_the_other() {
    let folder = scratch_folder("file");
    let night = theme_with_sprite('N');
    std::fs::write(folder.join("night.ron"), &night).unwrap();
    let world = world();
    let night = Theme::from_ron(&night, world.data()).unwrap();
    let mut app = app_for(&world, &folder);
    // Glyphs: CP437, ASCII, night.
    on_row(&mut app, &world, Setting::Glyphs);
    apply(&mut app, &world, Action::Scroll { dx: -1, dy: 0 });
    assert_eq!(app.notice(), Some("Glyphs: night"));
    assert_eq!(shows(&app, Setting::Glyphs), "night");
    assert_eq!(app.theme, Theme::compose(&night, &Theme::cp437()));
    assert_eq!(app.theme.glyph(SemanticTile::Sprite).symbol, 'N');
    // Colours: default, high contrast, colour-blind, mono, night.
    apply(&mut app, &world, Action::Pick(2));
    apply(&mut app, &world, Action::Scroll { dx: -1, dy: 0 });
    assert_eq!(app.notice(), Some("Colours: night"));
    assert_eq!(app.theme, night, "both from the file, as written");
    apply(&mut app, &world, Action::Pick(1));
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.theme, Theme::compose(&Theme::cp437(), &night));
}

#[test]
fn a_theme_that_doesnt_load_is_refused_and_the_row_keeps_its_value() {
    let folder = scratch_folder("broken");
    std::fs::write(folder.join("broken.ron"), theme_with_sprite('λ')).unwrap();
    let world = world();
    let mut app = app_for(&world, &folder);
    on_row(&mut app, &world, Setting::Glyphs);
    apply(&mut app, &world, Action::Scroll { dx: -1, dy: 0 });
    let refusal = app.refusal().expect("refused");
    assert!(
        refusal.starts_with("Couldn't use theme broken: ") && refusal.contains("CP437"),
        "{refusal}"
    );
    assert_eq!(shows(&app, Setting::Glyphs), "CP437");
    assert_eq!(app.theme, Theme::cp437());
    assert_eq!(app.screen(), Screen::Settings);
    apply(&mut app, &world, Action::Pick(2));
    apply(&mut app, &world, Action::Scroll { dx: -1, dy: 0 });
    assert!(app.refusal().is_some());
    assert_eq!(shows(&app, Setting::Colours), "default");
}

#[test]
fn the_motion_row_turns_motion_on_and_off() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("motion"));
    app.set_motion(true);
    on_row(&mut app, &world, Setting::Motion);
    apply(&mut app, &world, Action::Enter);
    assert!(!app.motion());
    assert_eq!(app.notice(), Some("Motion: off"));
    assert_eq!(shows(&app, Setting::Motion), "off");
    apply(&mut app, &world, Action::Scroll { dx: 1, dy: 0 });
    assert!(app.motion());
    assert_eq!(app.notice(), Some("Motion: on"));
}

#[test]
fn motion_turned_off_while_the_view_pans_puts_the_cursor_under_a_still_pointer() {
    // Design §6.9: the pan ends at once, so the Cursor goes to the tile
    // under the pointer then, as it would have as the pan ended.
    let world = WorldBuilder::grass(150, 12)
        .starters(&[at(10, 5), at(140, 5)])
        .build();
    let mut app = sized_app(&world, Theme::cp437(), SCREEN);
    app.set_motion(true);
    let pointer = pointer_on(&app, at(90, 6));
    apply(&mut app, &world, Action::Point(pointer));
    // `Tab` to the sprite in the west: the view pans from column 48 to -13.
    apply(&mut app, &world, Action::SelectNext);
    assert_eq!(app.cursor(), at(90, 6), "waiting for the pan");
    on_row(&mut app, &world, Setting::Motion);
    apply(&mut app, &world, Action::Enter);
    assert!(!app.motion());
    // Where column 90 was, 42 columns into the view from column -13.
    assert_eq!(app.cursor(), at(29, 6));
}

#[test]
fn the_square_tiles_row_is_the_switch_o_is() {
    let world = WorldBuilder::grass(60, 20).seed(7).build();
    let mut app = map_app(&world, Rect::new(0, 0, 40, 12));
    let mut by_o = map_app(&world, Rect::new(0, 0, 40, 12));
    on_row(&mut app, &world, Setting::SquareTiles);
    apply(&mut app, &world, Action::Enter);
    apply(&mut by_o, &world, Action::Zoom);
    assert_eq!(app.tile_width(), 2);
    assert_eq!(app.viewport(), by_o.viewport(), "the middle kept");
    assert_eq!(app.notice(), Some("Square tiles: on"));
    assert_eq!(shows(&app, Setting::SquareTiles), "on");
    apply(&mut app, &world, Action::Back);
    apply(&mut app, &world, Action::Zoom);
    apply(&mut app, &world, Action::OpenSettings);
    assert_eq!(shows(&app, Setting::SquareTiles), "off");
    assert_eq!(app.tile_width(), 1);
}

#[test]
fn a_click_on_a_row_changes_it_and_one_off_the_box_closes_it() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("click"));
    apply(&mut app, &world, Action::OpenSettings);
    let area = app.menu_area(&world).expect("open");
    let click = |at: Position| Action::Click {
        at,
        button: Button::Left,
        amplified: false,
    };
    // The Colours row, though Glyphs is chosen.
    apply(
        &mut app,
        &world,
        click(Position::new(area.x + 4, area.y + 2)),
    );
    assert_eq!(app.menu_choice(), 1);
    assert_eq!(shows(&app, Setting::Colours), "high contrast");
    assert_eq!(app.notice(), Some("Colours: high contrast"));
    // On the box's edge, it does nothing; off it, it closes.
    apply(&mut app, &world, click(area.as_position()));
    assert_eq!(app.screen(), Screen::Settings);
    assert_eq!(shows(&app, Setting::Colours), "high contrast");
    let off = Position::new(area.right() + 1, area.bottom() - 1);
    apply(&mut app, &world, click(off));
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.selection(), None, "the click went no further");
}

#[test]
fn esc_closes_it_and_question_mark_opens_help_over_it() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("esc"));
    apply(&mut app, &world, Action::OpenSettings);
    apply(&mut app, &world, Action::Help);
    assert_eq!(app.screen(), Screen::Help);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Settings, "back to it");
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
}

#[test]
fn time_runs_on_behind_it() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("time"));
    assert!(!app.clock.is_paused());
    apply(&mut app, &world, Action::OpenSettings);
    assert!(!app.clock.is_paused());
    // As behind a menu, its keys are its own.
    apply(&mut app, &world, Action::TogglePause);
    assert!(!app.clock.is_paused());
}

#[test]
fn its_status_line_says_how_to_choose_and_change_a_row() {
    let world = world();
    let mut app = sized_app(&world, Theme::cp437(), SCREEN);
    apply(&mut app, &world, Action::OpenSettings);
    let line = common::status_line(&app, &world);
    assert!(
        line.ends_with("↑↓ choose  ←→ change  esc close"),
        "{line:?}"
    );
}

#[test]
fn with_no_themes_folder_it_offers_the_built_in_ones() {
    let world = world();
    let mut app = map_app(&world, Rect::new(1, 2, 40, 12));
    on_row(&mut app, &world, Setting::Glyphs);
    assert_eq!(
        values_of(&mut app, &world, Setting::Glyphs),
        ["ASCII", "CP437"]
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

    on_row(&mut app, &loaded, Setting::Glyphs);
    assert_eq!(
        values_of(&mut app, &loaded, Setting::Glyphs),
        ["ASCII", "night", "CP437"]
    );
}

// The settings carry everywhere (design §6.7, #307): over a load, and back
// to the title screen and into the next world.

/// Every row changed on `app`: ASCII glyphs, mono, motion on and square
/// tiles.
fn change_every_row(app: &mut App, world: &World) {
    on_row(app, world, Setting::Glyphs);
    apply(app, world, Action::Enter);
    apply(app, world, Action::Pick(2));
    apply(app, world, Action::Scroll { dx: -1, dy: 0 });
    apply(app, world, Action::Pick(3));
    apply(app, world, Action::Enter);
    apply(app, world, Action::Pick(4));
    apply(app, world, Action::Enter);
    apply(app, world, Action::Back);
}

/// The settings `change_every_row` leaves.
fn every_row_changed() -> Settings {
    Settings {
        look: Look::new(
            Pick::BuiltIn(GlyphSet::Ascii),
            GlyphSet::Ascii.source(),
            Pick::BuiltIn(ColourPalette::Mono),
            ColourPalette::Mono.source(),
        ),
        motion: true,
        square_tiles: true,
        saved: SavedSettings {
            glyphs: Some("ASCII".into()),
            colours: Some("mono".into()),
            motion: Some(true),
            square_tiles: Some(true),
        },
    }
}

#[test]
fn the_app_hands_on_the_settings_as_the_player_left_them() {
    let world = world();
    let mut app = app_for(&world, &scratch_folder("handed-on"));
    assert_eq!(
        app.settings(),
        Settings {
            motion: false,
            ..Settings::default()
        },
        "as a test's app starts"
    );
    change_every_row(&mut app, &world);
    assert_eq!(app.settings(), every_row_changed());
    assert_eq!(
        app.theme,
        Theme::built_in(GlyphSet::Ascii, ColourPalette::Mono)
    );
}

#[test]
fn the_next_world_starts_with_the_settings_the_last_one_left() {
    // Back to the title screen and into another world (M2 design §8.4).
    let folder = scratch_folder("next-world");
    let world = world();
    let mut app = app_for(&world, &folder);
    change_every_row(&mut app, &world);
    let next_world = WorldBuilder::grass(60, 20).seed(8).build();
    let mut next = map_app(&next_world, Rect::new(0, 0, 40, 12));
    let mut pressed_o = map_app(&next_world, Rect::new(0, 0, 40, 12));
    apply(&mut pressed_o, &next_world, Action::Zoom);
    next.use_settings(app.settings());
    assert_eq!(next.settings(), every_row_changed());
    assert_eq!(next.theme, app.theme);
    assert!(next.motion());
    // Square tiles set up as `o` sets them up, but said nothing of.
    assert_eq!(next.tile_width(), 2);
    assert_eq!(next.viewport(), pressed_o.viewport());
    assert_eq!(next.notice(), None);
    // And still saved as they were, with what's changed next.
    let path = folder.join(settings::FILE);
    next.set_settings_file(path.clone());
    on_row(&mut next, &next_world, Setting::Motion);
    apply(&mut next, &next_world, Action::Enter);
    assert_eq!(
        settings::read(&path),
        Ok(SavedSettings {
            motion: Some(false),
            ..every_row_changed().saved
        })
    );
}

#[test]
fn what_the_flags_pick_isn_t_saved_as_another_row_changes() {
    // `--ascii` for its run, say, over settings that saved nothing.
    let folder = scratch_folder("flags");
    let world = world();
    let mut app = app_for(&world, &folder);
    let path = folder.join(settings::FILE);
    app.set_settings_file(path.clone());
    app.use_settings(Settings {
        look: Look::new(
            Pick::BuiltIn(GlyphSet::Ascii),
            GlyphSet::Ascii.source(),
            Pick::BuiltIn(ColourPalette::Default),
            ColourPalette::Default.source(),
        ),
        ..Settings::default()
    });
    assert_eq!(app.theme, Theme::ascii());
    on_row(&mut app, &world, Setting::Colours);
    apply(&mut app, &world, Action::Enter);
    assert_eq!(
        settings::read(&path),
        Ok(SavedSettings {
            colours: Some("high contrast".into()),
            ..SavedSettings::default()
        })
    );
}

#[test]
fn the_settings_carry_over_a_load() {
    let folder = scratch_folder("load");
    let world = world();
    let mut app = app_for(&world, &folder);
    let path = folder.join(settings::FILE);
    app.set_settings_file(path.clone());
    app.set_save_folder(folder.join("saves"));
    apply(&mut app, &world, Action::Quicksave);
    change_every_row(&mut app, &world);
    apply(&mut app, &world, Action::Quickload);
    apply(&mut app, &world, Action::Confirm);
    let loaded = app.take_loaded().expect("the quicksave loaded");
    assert_eq!(app.settings(), every_row_changed());
    assert_eq!(
        app.theme,
        Theme::built_in(GlyphSet::Ascii, ColourPalette::Mono)
    );
    // Saved in the same place.
    on_row(&mut app, &loaded, Setting::Motion);
    apply(&mut app, &loaded, Action::Enter);
    assert_eq!(
        settings::read(&path).map(|saved| saved.motion),
        Ok(Some(false))
    );
}
