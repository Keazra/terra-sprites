//! The data-format reference's working examples for the front end's formats
//! (design §7.4, A9): the theme example in `docs/reference/` loads as
//! `--theme` would load it. `terra-sim`'s own test covers the data pack's.

use ratatui::style::Color;
use terra_sim::DataPack;
use terra_tui::space::{Depth, Pane};
use terra_tui::theme::{SemanticTile, Theme};

/// The example `name` from a reference page: the fenced block after its
/// `<!-- example: name -->` marker, without the fences. The same as
/// `terra-sim`'s `tests/reference_docs.rs`, which a test crate can't share.
fn example(page: &str, name: &str) -> String {
    let marker = format!("<!-- example: {name} -->");
    let after = page
        .split_once(&marker)
        .unwrap_or_else(|| panic!("no example `{name}` in the page"))
        .1;
    let block = after
        .trim_start()
        .strip_prefix("```ron\n")
        .unwrap_or_else(|| panic!("example `{name}` isn't a ```ron block"));
    let end = block
        .find("\n```")
        .unwrap_or_else(|| panic!("example `{name}` isn't closed"));
    block[..end].to_string()
}

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
