//! Themes map semantic tiles to glyphs and colours (design §6.2). They are UI
//! assets, embedded in the binary, and never affect the simulation.

use std::collections::BTreeMap;

use ratatui::style::Color;
use serde::Deserialize;
use terra_sim::Terrain;

use crate::app::CursorMode;

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
    pub bold: bool,
}

/// What a theme draws for an object it has no glyph for, so a data pack's new
/// object types still show.
const UNKNOWN_OBJECT: Glyph = Glyph {
    symbol: '?',
    fg: Color::White,
    bold: false,
};

/// The cursor's arrows (design §6.5), named by the way they point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arrows {
    pub up: char,
    pub down: char,
    pub left: char,
    pub right: char,
}

/// What the cursor's status marks, `Y` and `N`, can show (design §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusMarks {
    /// Nothing to report.
    pub idle: char,
}

/// A mapping from semantic tiles, and the cursor, to glyphs and colours.
#[derive(Debug, Clone)]
pub struct Theme {
    tiles: BTreeMap<SemanticTile, Glyph>,
    /// By object type name, then visual state.
    objects: BTreeMap<String, BTreeMap<String, Glyph>>,
    arrows: Arrows,
    status_marks: StatusMarks,
    mode_marks: BTreeMap<CursorMode, Glyph>,
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

    /// How this theme draws an object of type `object` in visual state `state`
    /// (design §6.2). With no glyph for that state, it uses the object's
    /// `"default"` look, and failing that a `?`.
    pub fn object_glyph(&self, object: &str, state: &str) -> Glyph {
        self.object_entry(object, state)
            .or_else(|| self.object_entry(object, "default"))
            .unwrap_or(UNKNOWN_OBJECT)
    }

    /// The glyph this theme gives exactly this object type and visual state, if any.
    pub fn object_entry(&self, object: &str, state: &str) -> Option<Glyph> {
        self.objects.get(object)?.get(state).copied()
    }

    /// The cursor's arrows.
    pub fn arrows(&self) -> Arrows {
        self.arrows
    }

    /// What the cursor's status marks show.
    pub fn status_marks(&self) -> StatusMarks {
        self.status_marks
    }

    /// The mode mark for `mode`. The cursor's arrows and status marks take its colour.
    pub fn mode_mark(&self, mode: CursorMode) -> Glyph {
        self.mode_marks[&mode]
    }

    fn builtin(name: &str, text: &str) -> Theme {
        let file: ThemeFile = ron::from_str(text)
            .unwrap_or_else(|e| panic!("the built-in {name} theme doesn't parse: {e}"));
        let tiles = glyphs(file.tiles);
        for tile in SemanticTile::ALL {
            assert!(
                tiles.contains_key(&tile),
                "the {name} theme has no {tile:?}"
            );
        }
        let mode_marks = glyphs(file.cursor.mode_marks);
        for mode in CursorMode::ALL {
            assert!(
                mode_marks.contains_key(&mode),
                "the {name} theme has no {mode:?} mark"
            );
        }
        let objects = file
            .objects
            .into_iter()
            .map(|(object, states)| (object, glyphs(states)))
            .collect();
        Theme {
            tiles,
            objects,
            arrows: file.cursor.arrows,
            status_marks: file.cursor.status_marks,
            mode_marks,
        }
    }
}

/// Turns a theme file's entries into glyphs.
fn glyphs<K: Ord>(entries: BTreeMap<K, GlyphEntry>) -> BTreeMap<K, Glyph> {
    entries
        .into_iter()
        .map(|(key, entry)| {
            let glyph = Glyph {
                symbol: entry.glyph,
                fg: entry.fg.into(),
                bold: entry.bold,
            };
            (key, glyph)
        })
        .collect()
}

/// A theme file, as written in `themes/*.ron`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    tiles: BTreeMap<SemanticTile, GlyphEntry>,
    /// By object type name (from the data pack's `objects.ron`), then visual state.
    objects: BTreeMap<String, BTreeMap<String, GlyphEntry>>,
    cursor: CursorFile,
}

/// A theme file's `cursor` section.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorFile {
    arrows: Arrows,
    status_marks: StatusMarks,
    mode_marks: BTreeMap<CursorMode, GlyphEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlyphEntry {
    glyph: char,
    fg: Colour,
    #[serde(default)]
    bold: bool,
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
