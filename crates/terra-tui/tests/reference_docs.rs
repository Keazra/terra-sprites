//! The data-format reference's working examples for the front end's formats
//! (design §7.4, A9): the theme example in `docs/reference/` loads as
//! `--theme` would load it. `terra-sim`'s own test covers the data pack's.

// The example reader `terra-sim`'s test uses too, from its `tests/common`.
#[path = "../../terra-sim/tests/common/reference_example.rs"]
mod reference_example;
use reference_example::example;

use ratatui::style::Color;
use terra_sim::DataPack;
use terra_tui::space::{Depth, Pane};
use terra_tui::theme::{SemanticTile, Theme};

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
}
