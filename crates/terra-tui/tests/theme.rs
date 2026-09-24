use std::collections::BTreeSet;

use ratatui::style::Color;
use terra_sim::Terrain;
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

#[test]
fn every_kind_of_tile_has_its_own_glyph_in_each_theme() {
    for (name, theme) in [("cp437", Theme::cp437()), ("ascii", Theme::ascii())] {
        let glyphs: BTreeSet<char> = SemanticTile::ALL
            .iter()
            .map(|&tile| theme.glyph(tile).symbol)
            .collect();
        assert_eq!(
            glyphs.len(),
            SemanticTile::ALL.len(),
            "{name} reuses a glyph"
        );
    }
}

#[test]
fn theme_glyphs_are_cp437_and_the_ascii_themes_are_plain_ascii() {
    for tile in SemanticTile::ALL {
        let symbol = Theme::cp437().glyph(tile).symbol;
        assert!(cp437::contains(symbol), "{symbol:?} for {tile:?}");
        let symbol = Theme::ascii().glyph(tile).symbol;
        assert!(symbol.is_ascii_graphic(), "{symbol:?} for {tile:?}");
    }
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
