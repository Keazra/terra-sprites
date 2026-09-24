//! Themes map semantic tiles to glyphs and colours (design §6.2). They are UI
//! assets, embedded in the binary, and never affect the simulation.

use std::collections::BTreeMap;

use ratatui::style::Color;
use serde::Deserialize;
use terra_sim::Terrain;

/// What the map view draws for a tile, named by meaning rather than by character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticTile {
    Terrain(Terrain),
}

impl SemanticTile {
    /// Every semantic tile. Each theme must draw all of them.
    pub const ALL: [SemanticTile; 6] = [
        SemanticTile::Terrain(Terrain::Grass),
        SemanticTile::Terrain(Terrain::Dirt),
        SemanticTile::Terrain(Terrain::Sand),
        SemanticTile::Terrain(Terrain::ShallowWater),
        SemanticTile::Terrain(Terrain::DeepWater),
        SemanticTile::Terrain(Terrain::Rock),
    ];
}

/// How a theme draws one semantic tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub symbol: char,
    pub fg: Color,
}

/// The glyphs a theme draws the cursor with (design §6.5). The arrows are
/// named by the way they point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorGlyphs {
    pub up: char,
    pub down: char,
    pub left: char,
    pub right: char,
    /// The Select mode's mark.
    pub select: char,
    /// A status mark with nothing to report.
    pub idle: char,
}

/// A mapping from semantic tiles to glyphs and colours.
#[derive(Debug, Clone)]
pub struct Theme {
    tiles: BTreeMap<SemanticTile, Glyph>,
    cursor: CursorGlyphs,
}

impl Theme {
    /// The default theme: CP437 glyphs.
    pub fn cp437() -> Theme {
        Theme::builtin("cp437", include_str!("../../../themes/cp437.ron"))
    }

    /// The `--ascii` theme: the same meanings in plain ASCII.
    pub fn ascii() -> Theme {
        Theme::builtin("ascii", include_str!("../../../themes/ascii.ron"))
    }

    /// How this theme draws `tile`.
    pub fn glyph(&self, tile: SemanticTile) -> Glyph {
        self.tiles[&tile]
    }

    /// How this theme draws the cursor.
    pub fn cursor(&self) -> CursorGlyphs {
        self.cursor
    }

    fn builtin(name: &str, text: &str) -> Theme {
        let file: ThemeFile = ron::from_str(text)
            .unwrap_or_else(|e| panic!("the built-in {name} theme doesn't parse: {e}"));
        let tiles: BTreeMap<SemanticTile, Glyph> = file
            .tiles
            .into_iter()
            .map(|(tile, entry)| {
                let glyph = Glyph {
                    symbol: entry.glyph,
                    fg: entry.fg.into(),
                };
                (tile, glyph)
            })
            .collect();
        for tile in SemanticTile::ALL {
            assert!(
                tiles.contains_key(&tile),
                "the {name} theme has no {tile:?}"
            );
        }
        Theme {
            tiles,
            cursor: file.cursor,
        }
    }
}

/// A theme file, as written in `themes/*.ron`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    tiles: BTreeMap<SemanticTile, GlyphEntry>,
    cursor: CursorGlyphs,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlyphEntry {
    glyph: char,
    fg: Colour,
}

/// The 16 terminal colours, as named in theme files.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Colour {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
}

impl From<Colour> for Color {
    fn from(colour: Colour) -> Color {
        match colour {
            Colour::Black => Color::Black,
            Colour::Red => Color::Red,
            Colour::Green => Color::Green,
            Colour::Yellow => Color::Yellow,
            Colour::Blue => Color::Blue,
            Colour::Magenta => Color::Magenta,
            Colour::Cyan => Color::Cyan,
            Colour::Gray => Color::Gray,
            Colour::DarkGray => Color::DarkGray,
            Colour::LightRed => Color::LightRed,
            Colour::LightGreen => Color::LightGreen,
            Colour::LightYellow => Color::LightYellow,
            Colour::LightBlue => Color::LightBlue,
            Colour::LightMagenta => Color::LightMagenta,
            Colour::LightCyan => Color::LightCyan,
            Colour::White => Color::White,
        }
    }
}
