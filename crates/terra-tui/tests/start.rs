//! What the game starts with, from its flags (design §6.7): which preset
//! makes the new world, and which theme draws it; and from the settings
//! the player saved (#307).

mod common;
use common::every_row;

use std::path::{Path, PathBuf};

use terra_sim::{DataPack, Mode};
use terra_tui::args::Args;
use terra_tui::saves;
use terra_tui::settings::{SavedSettings, Settings};
use terra_tui::start;
use terra_tui::theme::{ColourPalette, GlyphSet, Pick, SemanticTile, Theme};

/// A fresh folder holding `files`, each `(path within it, text)`.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terra-start-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in files {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The map size of the world the flags make.
fn size(args: &Args) -> (u16, u16) {
    let world = start::new_world(args, 1).expect("a world");
    (world.map().width(), world.map().height())
}

#[test]
fn a_preset_flag_wins_over_the_data_folder_s_default_preset() {
    let data = folder(
        "data",
        &[("presets/default.ron", "(width: 64, height: 40)")],
    );
    let own = folder("preset", &[("small.ron", "(width: 48, height: 32)")]);
    let with_data = Args {
        data: Some(data.clone()),
        ..Args::default()
    };
    assert_eq!(size(&Args::default()), (256, 160), "the built-in preset");
    assert_eq!(size(&with_data), (64, 40), "the data folder's own");
    let both = Args {
        preset: Some(own.join("small.ron")),
        ..with_data
    };
    assert_eq!(size(&both), (48, 32), "--preset wins");
}

#[test]
fn a_preset_that_wont_load_is_refused_naming_it() {
    let own = folder("bad-preset", &[("tiny.ron", "(width: 8, height: 8)")]);
    let args = Args {
        preset: Some(own.join("tiny.ron")),
        ..Args::default()
    };
    let Err(err) = start::new_world(&args, 1) else {
        panic!("too small a map is refused");
    };
    assert!(
        err.contains("can't use preset") && err.contains("tiny.ron"),
        "{err}"
    );
}

/// The theme the flags ask for, as the game starts with no saved choice
/// and no `NO_COLOR`.
fn flagged_theme(args: &Args, data: &DataPack) -> Result<Theme, String> {
    let nothing_saved = Ok(SavedSettings::default());
    start::settings(args, nothing_saved, None, None, data)
        .map(|(settings, _)| settings.look.theme())
}

#[test]
fn the_theme_flag_loads_a_theme_file_at_start() {
    let data = DataPack::builtin().expect("valid pack");
    let sprite = |theme: &Theme| theme.glyph(SemanticTile::Sprite);
    let ascii = Theme::ascii();
    assert_ne!(sprite(&ascii), sprite(&Theme::cp437()), "the two differ");
    let pick = |args: Args| flagged_theme(&args, &data);
    assert_eq!(pick(Args::default()), Ok(Theme::cp437()));
    let flagged = Args {
        ascii: true,
        ..Args::default()
    };
    assert_eq!(pick(flagged), Ok(ascii.clone()));
    let themes = folder(
        "themes",
        &[
            ("mine.ron", include_str!("../../../themes/ascii.ron")),
            ("broken.ron", "(not a theme)"),
        ],
    );
    let file = |name: &str| Args {
        theme: Some(themes.join(name)),
        ..Args::default()
    };
    assert_eq!(pick(file("mine.ron")), Ok(ascii));
    let err = pick(file("broken.ron")).expect_err("refused");
    assert!(
        err.contains("can't use theme") && err.contains("broken.ron"),
        "{err}"
    );
    let err = pick(file("missing.ron")).expect_err("refused");
    assert!(err.contains("missing.ron"), "{err}");
}

#[test]
fn ascii_with_a_theme_file_draws_the_file_s_palette_in_ascii() {
    // Design §6.2, #307: `--ascii` picks only the glyph set.
    let data = DataPack::builtin().expect("valid pack");
    let themes = folder(
        "ascii-and-theme",
        &[(
            "contrast.ron",
            include_str!("../../../themes/high_contrast.ron"),
        )],
    );
    let args = Args {
        ascii: true,
        theme: Some(themes.join("contrast.ron")),
        ..Args::default()
    };
    assert_eq!(
        flagged_theme(&args, &data),
        Ok(Theme::built_in(
            GlyphSet::Ascii,
            ColourPalette::HighContrast
        ))
    );
}

#[test]
fn the_flags_give_the_glyph_set_ascii_s_over_a_theme_file_s() {
    // Design §6.2, §6.7 (#307): a flag lasts only for its run, and wins.
    let themes = folder("glyph-flags", &[("day.ron", ASCII)]);
    let day = themes.join("day.ron");
    let glyphs = |args: Args| {
        let (settings, _) = started(&args, Ok(SavedSettings::default()), None);
        settings.look.glyphs().clone()
    };
    assert_eq!(glyphs(Args::default()), Pick::BuiltIn(GlyphSet::Cp437));
    let ascii = Args {
        ascii: true,
        ..Args::default()
    };
    assert_eq!(glyphs(ascii), Pick::BuiltIn(GlyphSet::Ascii));
    let theme = Args {
        theme: Some(day.clone()),
        ..Args::default()
    };
    assert_eq!(glyphs(theme.clone()), Pick::File(day));
    let both = Args {
        ascii: true,
        ..theme
    };
    assert_eq!(glyphs(both), Pick::BuiltIn(GlyphSet::Ascii));
}

#[test]
fn only_flags_that_make_or_load_a_world_skip_the_title_screen() {
    // M2 design §8.5: `--seed`, `--preset` and `--replay` go straight in;
    // `--data`, `--ascii` and `--theme` only change how the title screen
    // and its worlds are made or drawn.
    let skips = |args: Args| start::skips_title(&args);
    assert!(!skips(Args::default()));
    assert!(skips(Args {
        seed: Some(7),
        ..Args::default()
    }));
    assert!(skips(Args {
        preset: Some("small.ron".into()),
        ..Args::default()
    }));
    assert!(skips(Args {
        replay: Some("last_session.replay".into()),
        ..Args::default()
    }));
    assert!(!skips(Args {
        data: Some("mod".into()),
        ascii: true,
        ..Args::default()
    }));
    assert!(!skips(Args {
        theme: Some("night.ron".into()),
        ..Args::default()
    }));
}

#[test]
fn the_title_screen_s_world_fills_the_terminal_at_the_default_preset_s_densities() {
    let world = start::title_world(&Args::default(), 1, (120, 40)).expect("a world");
    assert_eq!((world.map().width(), world.map().height()), (120, 40));
    assert_eq!(world.sprites().count(), 30, "the default preset's sprites");
    // A data folder's own default preset gives its densities, at the
    // terminal's size still.
    let data = folder(
        "title-data",
        &[(
            "presets/default.ron",
            "(width: 64, height: 40, sprites: 20, objects: {\"ball\": 1}, per_tiles: 4800)",
        )],
    );
    let args = Args {
        data: Some(data),
        ..Args::default()
    };
    let world = start::title_world(&args, 1, (120, 40)).expect("a world");
    assert_eq!((world.map().width(), world.map().height()), (120, 40));
    assert_eq!(world.sprites().count(), 20);
}

#[test]
fn the_title_screen_s_world_takes_the_default_preset_whatever_preset_the_flags_name() {
    // Going back to the title screen from a world `--preset` made wakes
    // the default preset's world (M2 design §8.1).
    let own = folder(
        "title-preset",
        &[("crowd.ron", "(width: 48, height: 32, sprites: 90)")],
    );
    let args = Args {
        preset: Some(own.join("crowd.ron")),
        ..Args::default()
    };
    let world = start::title_world(&args, 1, (120, 40)).expect("a world");
    assert_eq!(world.sprites().count(), 30, "the default preset's sprites");
}

#[test]
fn a_terminal_smaller_than_a_map_may_be_still_gets_the_smallest_map() {
    let world = start::title_world(&Args::default(), 1, (20, 10)).expect("a world");
    assert_eq!((world.map().width(), world.map().height()), (32, 32));
}

#[test]
fn new_world_offers_the_default_preset_then_each_in_the_presets_folder() {
    let presets = folder(
        "presets",
        &[
            ("small.ron", "(width: 48, height: 32)"),
            ("big.ron", "(width: 512, height: 512)"),
            ("notes.txt", "not a preset"),
        ],
    );
    let offered = start::presets(Some(&presets));
    let names: Vec<&str> = offered.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["default", "big", "small"]);
    assert_eq!(
        offered[0].path, None,
        "the default is the built-in or --data's"
    );
    assert_eq!(offered[2].path, Some(presets.join("small.ron")));
    let none = start::presets(Some(&presets.join("missing")));
    assert_eq!(none.len(), 1, "no folder, only the default");
}

#[test]
fn a_new_world_is_a_story_world_unless_the_flags_ask_for_the_lab() {
    // M2 design §16: `--lab` makes a Lab world, and a fast-forward makes
    // one too, since only Lab worlds fast-forward.
    let mode = |args: Args| start::new_world(&args, 1).expect("a world").mode();
    assert_eq!(mode(Args::default()), Mode::Story);
    assert_eq!(
        mode(Args {
            mode: Mode::Lab,
            ..Args::default()
        }),
        Mode::Lab
    );
    assert_eq!(
        mode(Args {
            fast_forward: Some(5),
            ..Args::default()
        }),
        Mode::Lab
    );
}

#[test]
fn lab_and_load_skip_the_title_screen_too() {
    assert!(start::skips_title(&Args {
        mode: Mode::Lab,
        ..Args::default()
    }));
    assert!(start::skips_title(&Args {
        load: Some("mira".into()),
        ..Args::default()
    }));
}

/// A saves folder holding a Story world saved as `story` and a Lab world
/// saved as `lab`.
fn saves_folder(name: &str) -> PathBuf {
    let dir = folder(name, &[]);
    for (save, mode) in [("story", Mode::Story), ("lab", Mode::Lab)] {
        let args = Args {
            mode,
            ..Args::default()
        };
        let world = start::new_world(&args, 3).expect("a world");
        saves::write(&dir, save, &world.save()).expect("saved");
    }
    dir
}

#[test]
fn fast_forward_runs_a_lab_save_or_a_new_lab_world_and_refuses_a_story_save() {
    let dir = saves_folder("fast-forward");
    let args = |load: Option<&str>| Args {
        load: load.map(str::to_string),
        fast_forward: Some(5),
        ..Args::default()
    };
    let (world, from) = start::fast_forward_world(&args(None), 7, Some(&dir)).expect("new");
    assert_eq!(
        (world.mode(), world.seed(), from.as_str()),
        (Mode::Lab, 7, "seed-7")
    );
    let (world, from) = start::fast_forward_world(&args(Some("lab")), 7, Some(&dir)).expect("lab");
    assert_eq!(
        (world.mode(), world.seed(), from.as_str()),
        (Mode::Lab, 3, "lab")
    );
    let error = start::fast_forward_world(&args(Some("story")), 7, Some(&dir))
        .err()
        .expect("refused");
    assert_eq!(
        error,
        "story is a Story world: only Lab worlds fast-forward"
    );
}

#[test]
fn a_preset_is_named_by_its_file() {
    assert_eq!(start::preset_name(&Args::default()), "default");
    let args = Args {
        preset: Some(PathBuf::from("worlds/big-lake.ron")),
        ..Args::default()
    };
    assert_eq!(start::preset_name(&args), "big-lake");
}

// The settings saved (design §6.7, #307).

const HIGH_CONTRAST: &str = include_str!("../../../themes/high_contrast.ron");
const ASCII: &str = include_str!("../../../themes/ascii.ron");

/// How the game starts with the settings `saved`, as read, its theme files
/// in `themes`, and `NO_COLOR` unset; and what it says of them.
fn started(
    args: &Args,
    saved: Result<SavedSettings, String>,
    themes: Option<&Path>,
) -> (Settings, Option<String>) {
    let data = DataPack::builtin().expect("valid pack");
    start::settings(args, saved, themes, None, &data).expect("started")
}

#[test]
fn the_game_starts_as_the_settings_say() {
    let themes = folder("saved", &[("night.ron", HIGH_CONTRAST)]);
    let (settings, said) = started(&Args::default(), Ok(every_row()), Some(&themes));
    assert_eq!(said, None);
    let look = &settings.look;
    assert_eq!(look.glyphs(), &Pick::BuiltIn(GlyphSet::Ascii));
    assert_eq!(look.palette(), &Pick::File(themes.join("night.ron")));
    assert_eq!(
        look.theme(),
        Theme::built_in(GlyphSet::Ascii, ColourPalette::HighContrast)
    );
    assert!(!settings.motion);
    assert!(settings.square_tiles);
    assert_eq!(settings.saved, every_row());
    // With nothing saved, the defaults.
    let (settings, said) = started(&Args::default(), Ok(SavedSettings::default()), None);
    assert_eq!((settings, said), (Settings::default(), None));
    assert_eq!(Settings::default().look.theme(), Theme::cp437());
    assert!(Settings::default().motion);
    assert!(!Settings::default().square_tiles);
}

#[test]
fn the_flags_win_over_the_settings_for_their_run_and_aren_t_saved() {
    let themes = folder(
        "flags-win",
        &[("night.ron", HIGH_CONTRAST), ("day.ron", ASCII)],
    );
    let saved = SavedSettings {
        glyphs: Some("night.ron".into()),
        motion: Some(true),
        ..every_row()
    };
    let ascii = Args {
        ascii: true,
        no_motion: true,
        ..Args::default()
    };
    let (settings, said) = started(&ascii, Ok(saved.clone()), Some(&themes));
    assert_eq!(said, None);
    assert_eq!(settings.look.glyphs(), &Pick::BuiltIn(GlyphSet::Ascii));
    assert_eq!(
        settings.look.palette(),
        &Pick::File(themes.join("night.ron"))
    );
    assert!(!settings.motion);
    assert_eq!(settings.saved, saved, "as the file has it");
    let day = Args {
        theme: Some(themes.join("day.ron")),
        ..Args::default()
    };
    let (settings, _) = started(&day, Ok(saved), Some(&themes));
    assert_eq!(settings.look.glyphs(), &Pick::File(themes.join("day.ron")));
    assert_eq!(settings.look.palette(), &Pick::File(themes.join("day.ron")));
    assert_eq!(settings.look.theme(), Theme::ascii());
    assert!(settings.motion, "the file's");
}

#[test]
fn a_theme_the_flags_name_that_won_t_load_still_stops_the_game() {
    let data = DataPack::builtin().expect("valid pack");
    let args = Args {
        theme: Some("missing.ron".into()),
        ..Args::default()
    };
    let err = start::settings(&args, Ok(SavedSettings::default()), None, None, &data)
        .expect_err("refused");
    assert!(err.starts_with("can't use theme missing.ron: "), "{err}");
}

#[test]
fn settings_that_can_t_be_read_start_the_game_on_the_defaults_saying_so() {
    let (settings, said) = started(&Args::default(), Err("expected `(`".into()), None);
    // The file is kept aside before the first save (design §6.7).
    assert_eq!(
        settings,
        Settings {
            unreadable: true,
            ..Settings::default()
        }
    );
    assert_eq!(
        said.as_deref(),
        Some("Couldn't read the settings: expected `(`")
    );
}

#[test]
fn a_theme_file_the_settings_name_that_no_longer_loads_falls_back_to_the_default() {
    let themes = folder("gone", &[("broken.ron", "not a theme")]);
    let saved = SavedSettings {
        glyphs: Some("gone.ron".into()),
        colours: Some("broken.ron".into()),
        ..every_row()
    };
    let (settings, said) = started(&Args::default(), Ok(saved.clone()), Some(&themes));
    assert_eq!(settings.look, Settings::default().look);
    assert!(
        !settings.motion && settings.square_tiles,
        "the rest as saved"
    );
    assert_eq!(settings.saved, saved, "the file left as it is");
    let said = said.expect("said");
    assert!(said.starts_with("Couldn't use theme gone: "), "{said}");
    assert!(said.contains("; Couldn't use theme broken: "), "{said}");
    // A glyph set or palette with a name the game doesn't know.
    let saved = SavedSettings {
        glyphs: Some("runes".into()),
        ..SavedSettings::default()
    };
    let (settings, said) = started(&Args::default(), Ok(saved), Some(&themes));
    assert_eq!(settings.look, Settings::default().look);
    assert_eq!(
        said.as_deref(),
        Some("Couldn't read the settings: there are no glyphs called runes")
    );
}

#[test]
fn a_theme_file_both_rows_name_that_won_t_load_is_said_once() {
    let themes = folder("both-broken", &[("night.ron", "not a theme")]);
    let saved = SavedSettings {
        glyphs: Some("night.ron".into()),
        colours: Some("night.ron".into()),
        ..every_row()
    };
    let (settings, said) = started(&Args::default(), Ok(saved), Some(&themes));
    assert_eq!(settings.look, Settings::default().look);
    let said = said.expect("said");
    assert_eq!(
        said.matches("Couldn't use theme night: ").count(),
        1,
        "{said}"
    );
    assert!(!said.contains("; "), "{said}");
}

#[test]
fn a_built_in_is_named_in_the_settings_in_any_case() {
    let saved = SavedSettings {
        glyphs: Some("ascii".into()),
        colours: Some("High Contrast".into()),
        ..SavedSettings::default()
    };
    let (settings, said) = started(&Args::default(), Ok(saved), None);
    assert_eq!(said, None);
    assert_eq!(
        settings.look.theme(),
        Theme::built_in(GlyphSet::Ascii, ColourPalette::HighContrast)
    );
}

#[test]
fn no_color_starts_the_game_in_mono_unless_a_flag_or_the_settings_pick_colours() {
    // Design §6.2 (#307): `NO_COLOR` set to anything but nothing, as
    // no-color.org asks.
    let data = DataPack::builtin().expect("valid pack");
    let themes = folder("no-color", &[("day.ron", ASCII)]);
    let palette = |args: &Args, saved: SavedSettings, no_color| {
        let (settings, _) = start::settings(args, Ok(saved), None, no_color, &data).unwrap();
        settings.look.palette().clone()
    };
    let (none, nothing) = (Args::default(), SavedSettings::default());
    let mono = Pick::BuiltIn(ColourPalette::Mono);
    let default = Pick::BuiltIn(ColourPalette::Default);
    assert_eq!(palette(&none, nothing.clone(), Some("1")), mono);
    assert_eq!(palette(&none, nothing.clone(), Some("")), default);
    assert_eq!(palette(&none, nothing.clone(), None), default);
    let picked = SavedSettings {
        colours: Some("default".into()),
        ..SavedSettings::default()
    };
    assert_eq!(palette(&none, picked, Some("1")), default);
    let day = Args {
        theme: Some(themes.join("day.ron")),
        ..Args::default()
    };
    assert_eq!(
        palette(&day, nothing.clone(), Some("1")),
        Pick::File(themes.join("day.ron"))
    );
    // `--ascii` picks only the glyphs.
    let ascii = Args {
        ascii: true,
        ..Args::default()
    };
    assert_eq!(palette(&ascii, nothing, Some("1")), mono);
}
