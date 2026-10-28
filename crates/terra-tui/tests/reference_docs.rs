//! The data-format reference's working examples for the front end's formats
//! (design §7.4, A9): the theme example in `docs/reference/` loads as
//! `--theme` would load it, and the settings example is what the game
//! writes. `terra-sim`'s own test covers the data pack's.

// The example reader `terra-sim`'s test uses too, from its `tests/common`.
#[path = "../../terra-sim/tests/common/reference_example.rs"]
mod reference_example;
use reference_example::example;

use ratatui::style::{Color, Style};
use terra_sim::DataPack;
use terra_tui::settings::SavedSettings;
use terra_tui::space::{Depth, Pane};
use terra_tui::theme::{BarWaves, SemanticTile, Theme};

#[test]
fn the_theme_example_loads() {
    let page = include_str!("../../../docs/reference/themes.md");
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let theme = Theme::from_ron(&example(page, "theme"), &pack).unwrap();
    assert_eq!(theme.glyph(SemanticTile::Sprite).symbol, '☺');
    assert_eq!(theme.drive_colour("boredom"), None);
    assert_eq!(theme.star(Depth::Far).fg, Color::Gray);
    let default = Theme::cp437();
    assert_eq!(theme.star(Depth::Near), default.star(Depth::Near));
    assert_eq!(theme.glass(Pane::TopLeft), default.glass(Pane::TopLeft));
    let (graph, default) = (theme.population(), default.population());
    assert_eq!((graph.old, graph.limit.fg), (Color::White, Color::Gray));
    assert_eq!((graph.young, graph.half), (default.young, default.half));
    let text = theme.text();
    assert_eq!(text.heading, Style::new().fg(Color::LightYellow).bold());
    assert_eq!(text.muted, Style::new().fg(Color::Gray));
    assert_eq!(text.good, Theme::cp437().text().good);
    let panels = theme.panels().expect("shaded");
    assert_eq!(panels.shade, Color::Rgb(0x1c, 0x1c, 0x1c));
    let (motion, default) = (theme.motion(), Theme::cp437().motion());
    assert_eq!(motion.flash, Color::White);
    assert_eq!(motion.ash, default.ash);
    assert_eq!(motion.bar, BarWaves::Ripple);
}

#[test]
fn the_settings_example_is_what_the_game_writes() {
    let page = include_str!("../../../docs/reference/settings.md");
    let text = example(page, "settings");
    let saved = SavedSettings {
        glyphs: Some("ASCII".into()),
        colours: Some("night.ron".into()),
        motion: Some(false),
        square_tiles: None,
    };
    assert_eq!(SavedSettings::from_ron(&text), Ok(saved.clone()));
    assert_eq!(saved.to_ron(), format!("{text}\n"));
}
