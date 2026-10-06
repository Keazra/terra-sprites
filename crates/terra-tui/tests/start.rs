//! What the game starts with, from its flags (design §6.7): which preset
//! makes the new world, and which theme draws it.

use std::path::PathBuf;

use terra_sim::DataPack;
use terra_tui::args::Args;
use terra_tui::start;
use terra_tui::theme::{SemanticTile, Theme};

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

#[test]
fn the_theme_flag_loads_a_theme_file_at_start() {
    let data = DataPack::builtin().expect("valid pack");
    let sprite = |theme: &Theme| theme.glyph(SemanticTile::Sprite);
    let ascii = Theme::ascii();
    assert_ne!(sprite(&ascii), sprite(&Theme::cp437()), "the two differ");
    let pick = |args: Args| start::theme(&args, &data);
    assert_eq!(
        pick(Args::default()).map(|t| sprite(&t)),
        Ok(sprite(&Theme::cp437()))
    );
    let flagged = Args {
        ascii: true,
        ..Args::default()
    };
    assert_eq!(pick(flagged).map(|t| sprite(&t)), Ok(sprite(&ascii)));
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
    assert_eq!(
        pick(file("mine.ron")).map(|t| sprite(&t)),
        Ok(sprite(&ascii))
    );
    let err = pick(file("broken.ron")).expect_err("refused");
    assert!(
        err.contains("can't use theme") && err.contains("broken.ron"),
        "{err}"
    );
    let err = pick(file("missing.ron")).expect_err("refused");
    assert!(err.contains("missing.ron"), "{err}");
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
