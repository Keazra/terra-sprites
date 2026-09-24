use ratatui::style::Color;
use terra_sim::{DataPack, Terrain};
use terra_tui::app::CursorMode;
use terra_tui::cp437;
use terra_tui::theme::{SemanticTile, Theme};

#[test]
fn the_themes_draw_terrain_as_the_design_table_says() {
    // Design §6.2. Terminals have no brown, so dirt is dark yellow.
    let expected = [
        (Terrain::Grass, '.', '.', Color::Green),
        (Terrain::Dirt, ',', ',', Color::Yellow),
        (Terrain::Sand, ':', ':', Color::LightYellow),
        (Terrain::ShallowWater, '~', '~', Color::Cyan),
        (Terrain::DeepWater, '≈', '=', Color::Blue),
        (Terrain::Rock, '#', '#', Color::Gray),
    ];
    let (cp437_theme, ascii_theme) = (Theme::cp437(), Theme::ascii());
    for (terrain, cp437_glyph, ascii_glyph, colour) in expected {
        let tile = SemanticTile::Terrain(terrain);
        let (a, b) = (cp437_theme.glyph(tile), ascii_theme.glyph(tile));
        assert_eq!((a.symbol, a.fg), (cp437_glyph, colour), "cp437 {terrain:?}");
        assert_eq!((b.symbol, b.fg), (ascii_glyph, colour), "ascii {terrain:?}");
    }
}

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// Every glyph a theme draws the map with, with the kind of thing it stands
/// for: a terrain, or an object type in any of its visual states.
fn map_glyphs(theme: &Theme) -> Vec<(char, String)> {
    let pack = pack();
    let terrain = SemanticTile::ALL
        .iter()
        .map(|&tile| (theme.glyph(tile).symbol, format!("{tile:?}")));
    let objects = pack.object_type_names().flat_map(|name| {
        pack.visual_states(name)
            .into_iter()
            .map(move |state| (theme.object_glyph(name, state).symbol, name.to_string()))
    });
    terrain.chain(objects).collect()
}

#[test]
fn every_kind_of_thing_on_the_map_has_its_own_glyphs_in_each_theme() {
    // Design §6.2: colour marks state; the glyph says what kind of thing it is.
    for (name, theme) in [("cp437", Theme::cp437()), ("ascii", Theme::ascii())] {
        let glyphs = map_glyphs(&theme);
        for (symbol, kind) in &glyphs {
            for (other_symbol, other_kind) in &glyphs {
                assert!(
                    symbol != other_symbol || kind == other_kind,
                    "{name} draws both {kind} and {other_kind} as {symbol:?}"
                );
            }
        }
    }
}

#[test]
fn theme_glyphs_are_cp437_and_the_ascii_themes_are_plain_ascii() {
    for (symbol, kind) in map_glyphs(&Theme::cp437()) {
        assert!(cp437::contains(symbol), "{symbol:?} for {kind}");
    }
    for (symbol, kind) in map_glyphs(&Theme::ascii()) {
        assert!(symbol.is_ascii_graphic(), "{symbol:?} for {kind}");
    }
}

#[test]
fn the_themes_draw_objects_as_the_design_table_says() {
    // Design §6.2: (object, visual state, cp437, ascii, colour, bold).
    let expected = [
        ("berry_bush", "seedling", '\'', '\'', Color::Green, false),
        ("berry_bush", "default", '♣', '&', Color::Green, false),
        ("berry_bush", "fruiting", '♣', '&', Color::Red, true),
        ("thornbush", "default", '♠', '*', Color::Magenta, false),
        ("berry", "default", '•', '%', Color::Red, false),
        ("ball", "default", '○', 'o', Color::White, false),
    ];
    let (cp437_theme, ascii_theme) = (Theme::cp437(), Theme::ascii());
    for (object, state, cp437_glyph, ascii_glyph, colour, bold) in expected {
        let (a, b) = (
            cp437_theme.object_glyph(object, state),
            ascii_theme.object_glyph(object, state),
        );
        assert_eq!(
            (a.symbol, a.fg, a.bold),
            (cp437_glyph, colour, bold),
            "cp437 {object} {state}"
        );
        assert_eq!(
            (b.symbol, b.fg, b.bold),
            (ascii_glyph, colour, bold),
            "ascii {object} {state}"
        );
    }
}

#[test]
fn each_theme_covers_every_visual_state_of_the_built_in_objects() {
    let pack = pack();
    for (name, theme) in [("cp437", Theme::cp437()), ("ascii", Theme::ascii())] {
        for object in pack.object_type_names() {
            for state in pack.visual_states(object) {
                assert!(
                    theme.object_entry(object, state).is_some(),
                    "{name} has no glyph for {object} ({state})"
                );
            }
        }
    }
}

#[test]
fn a_theme_falls_back_to_the_objects_default_look_then_to_a_question_mark() {
    let theme = Theme::cp437();
    assert_eq!(
        theme.object_glyph("berry", "squashed"),
        theme.object_glyph("berry", "default")
    );
    assert_eq!(theme.object_glyph("shrub", "default").symbol, '?');
}

#[test]
fn cp437_covers_the_glyphs_the_design_uses_and_nothing_outside_the_code_page() {
    for c in "☺♣♠•○►♥≈═║╔╗╚╝╒╕╘╛╓╖╙╜│─┌┐└┘ az~#".chars()
    {
        assert!(cp437::contains(c), "{c:?} is in CP437");
    }
    for c in "⅛é€✓\u{7}".chars() {
        let expected = c == 'é'; // é is CP437 0x82; the rest are not in the code page
        assert_eq!(cp437::contains(c), expected, "{c:?}");
    }
}

/// A theme's cursor glyphs for Select mode: the four arrows (up, down, left,
/// right), the mode mark and the idle status mark.
fn select_cursor(theme: &Theme) -> [char; 6] {
    let (arrows, status) = (theme.arrows(), theme.status_marks());
    let mark = theme.mode_mark(CursorMode::Select).symbol;
    [
        arrows.up,
        arrows.down,
        arrows.left,
        arrows.right,
        mark,
        status.idle,
    ]
}

#[test]
fn the_themes_draw_the_select_cursor_as_the_design_table_says() {
    // Design §6.2: arrows, the Select mode mark, and the idle status mark.
    assert_eq!(
        select_cursor(&Theme::cp437()),
        ['↑', '↓', '←', '→', '♦', '·']
    );
    assert_eq!(
        select_cursor(&Theme::ascii()),
        ['^', 'v', '<', '>', 'S', '-']
    );
    for glyph in select_cursor(&Theme::cp437()) {
        assert!(cp437::contains(glyph), "{glyph:?}");
    }
}

#[test]
fn the_select_mode_is_white_in_both_themes() {
    // Design §6.5: the mode mark's colour is the whole cursor's colour.
    for theme in [Theme::cp437(), Theme::ascii()] {
        assert_eq!(theme.mode_mark(CursorMode::Select).fg, Color::White);
    }
}
