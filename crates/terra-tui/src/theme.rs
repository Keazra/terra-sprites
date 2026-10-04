//! Themes map semantic tiles to glyphs and colours (design §6.2). They are UI
//! assets and never affect the simulation. Two are embedded in the binary,
//! and `--theme <file>` loads one the player has edited (design v33 §6.7).

use std::collections::BTreeMap;

use ratatui::style::Color;
use serde::Deserialize;
use terra_sim::{DataPack, Terrain};

use crate::app::{CursorMode, StatusMark};

/// What the map view draws for a tile, named by meaning rather than by character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticTile {
    Terrain(Terrain),
    Sprite,
    /// The selected sprite (design §6.1).
    SelectedSprite,
    /// The Decision marker: where the selected sprite is heading (design
    /// §6.1). It flashes.
    DecisionMarker,
    /// An emote, which takes turns with a sprite's glyph (design §6.3).
    Emote(Emote),
}

/// Something that just happened to a sprite, which the map shows by
/// swapping its glyph for a while (design §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Emote {
    /// Something hurt it.
    Hurt,
    /// The Cursor petted or hugged it (design v21 §6.3).
    Pleased,
    /// The Cursor zapped or shocked it (design v21 §6.3).
    Shocked,
    /// It gave up an action: failed, blocked or timed out.
    Failed,
    /// It's resting.
    Resting,
}

impl SemanticTile {
    /// Every semantic tile. Each theme must draw all of them.
    pub const ALL: [SemanticTile; 14] = [
        SemanticTile::Terrain(Terrain::Grass),
        SemanticTile::Terrain(Terrain::Dirt),
        SemanticTile::Terrain(Terrain::Sand),
        SemanticTile::Terrain(Terrain::ShallowWater),
        SemanticTile::Terrain(Terrain::DeepWater),
        SemanticTile::Terrain(Terrain::Rock),
        SemanticTile::Sprite,
        SemanticTile::SelectedSprite,
        SemanticTile::DecisionMarker,
        SemanticTile::Emote(Emote::Hurt),
        SemanticTile::Emote(Emote::Pleased),
        SemanticTile::Emote(Emote::Shocked),
        SemanticTile::Emote(Emote::Failed),
        SemanticTile::Emote(Emote::Resting),
    ];
}

/// How a theme draws one semantic tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub symbol: char,
    pub fg: Color,
    pub bold: bool,
    /// Drawn in reverse video.
    pub reversed: bool,
}

/// What a theme draws for an object it has no glyph for, so a data pack's new
/// object types still show.
const UNKNOWN_OBJECT: Glyph = Glyph {
    symbol: '?',
    fg: Color::White,
    bold: false,
    reversed: false,
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
    /// A click has just sent a command.
    pub sent: char,
    /// The world has just applied one.
    pub applied: char,
    /// The world has just refused one.
    pub rejected: char,
    /// In Grab mode, empty: `Y`, a click would grab something (design v23
    /// §6.5).
    pub grab: char,
    /// In Grab mode, empty: `N`.
    pub empty: char,
    /// In Grab mode, holding or leading: `Y`, a click would let go or put
    /// it down.
    pub release: char,
}

impl StatusMarks {
    /// The glyph the status marks show for `mark`, or `None` for the thing
    /// the Cursor holds or leads, which shows its own glyph.
    pub fn glyph(self, mark: StatusMark) -> Option<char> {
        Some(match mark {
            StatusMark::Idle => self.idle,
            StatusMark::Sent => self.sent,
            StatusMark::Applied => self.applied,
            StatusMark::Rejected => self.rejected,
            StatusMark::Grab => self.grab,
            StatusMark::Empty => self.empty,
            StatusMark::Release => self.release,
            StatusMark::Holding => return None,
        })
    }
}

/// A mapping from semantic tiles, and the cursor, to glyphs and colours.
#[derive(Debug, Clone)]
pub struct Theme {
    tiles: BTreeMap<SemanticTile, Glyph>,
    /// By object type name, then visual state.
    objects: BTreeMap<String, BTreeMap<String, Glyph>>,
    arrows: Arrows,
    followed_arrows: Arrows,
    visible_frame: Arrows,
    status_marks: StatusMarks,
    mode_marks: BTreeMap<CursorMode, Glyph>,
    leash: Glyph,
    aim: Glyph,
    aim_end: Glyph,
    attention_marker: Color,
    /// By drive name, the colour of a sprite whose strongest drive it is.
    drives: BTreeMap<String, Color>,
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

    /// The Attention marker: the background of the tile the selected sprite
    /// attends to.
    pub fn attention_marker(&self) -> Color {
        self.attention_marker
    }

    /// The colour of a sprite whose strongest drive is `drive`, if the
    /// theme gives it one (design §6.3).
    pub fn drive_colour(&self, drive: &str) -> Option<Color> {
        self.drives.get(drive).copied()
    }

    /// The Cursor's arrows.
    pub fn arrows(&self) -> Arrows {
        self.arrows
    }

    /// The Cursor's arrows while it follows a sprite: solid (design
    /// v26 §6.2).
    pub fn followed_arrows(&self) -> Arrows {
        self.followed_arrows
    }

    /// The Cursor's sides while sprites can see it, in place of the arrows,
    /// following or not: a frame of light (design v29 §6.5).
    pub fn visible_frame(&self) -> Arrows {
        self.visible_frame
    }

    /// What the cursor's status marks show.
    pub fn status_marks(&self) -> StatusMarks {
        self.status_marks
    }

    /// The mode mark for `mode`. The cursor's arrows and status marks take its colour.
    pub fn mode_mark(&self, mode: CursorMode) -> Glyph {
        self.mode_marks[&mode]
    }

    /// The leash, a dot of the line from the Cursor to a sprite it leads
    /// (design v23 §6.5).
    pub fn leash(&self) -> Glyph {
        self.leash
    }

    /// A dot of the aim line, along the path a throw or a shove will take
    /// (design v25 §6.5).
    pub fn aim(&self) -> Glyph {
        self.aim
    }

    /// The aim line's end: where a throw or a shove would stop if nothing's
    /// in the way (design v25 §6.5).
    pub fn aim_end(&self) -> Glyph {
        self.aim_end
    }

    /// A theme from the text of a theme file, as `--theme <file>` loads it
    /// (design v33 §6.7), checked against the data pack it will draw.
    ///
    /// It must draw every semantic tile and mode mark, with glyphs CP437
    /// has (design §6.2). It may leave out object types and drives, which
    /// fall back, but every object type, visual state and drive it names
    /// must be in `data`, so a misspelt one is caught rather than drawn as
    /// `?`. The error says what's wrong, in the file's own words.
    pub fn from_ron(text: &str, data: &DataPack) -> Result<Theme, String> {
        let theme = Theme::parse(text)?;
        for (object, states) in &theme.objects {
            if !data.object_type_names().any(|name| name == object) {
                return Err(format!("the data pack has no object type \"{object}\""));
            }
            let known = data.visual_states(object);
            if let Some(state) = states.keys().find(|state| !known.contains(&state.as_str())) {
                return Err(format!(
                    "\"{object}\" has no visual state \"{state}\": it has {}",
                    quoted(&known)
                ));
            }
        }
        if let Some(drive) = theme
            .drives
            .keys()
            .find(|drive| !data.drives().any(|name| name == drive.as_str()))
        {
            let drives: Vec<&str> = data.drives().collect();
            return Err(format!(
                "the data pack has no drive \"{drive}\": its drives are {}",
                quoted(&drives)
            ));
        }
        Ok(theme)
    }

    fn builtin(name: &str, text: &str) -> Theme {
        Theme::parse(text).unwrap_or_else(|e| panic!("the built-in {name} theme is invalid: {e}"))
    }

    /// A theme from a theme file's text, complete and within CP437.
    fn parse(text: &str) -> Result<Theme, String> {
        let file: ThemeFile = ron::from_str(text).map_err(|e| e.to_string())?;
        let tiles = glyphs(file.tiles);
        if let Some(&tile) = SemanticTile::ALL
            .iter()
            .find(|tile| !tiles.contains_key(tile))
        {
            return Err(format!("it has no glyph for {}", tile_name(tile)));
        }
        let mode_marks = glyphs(file.cursor.mode_marks);
        if let Some(&mode) = CursorMode::ALL
            .iter()
            .find(|mode| !mode_marks.contains_key(mode))
        {
            return Err(format!("it has no mode mark for {}", snake_case(&mode)));
        }
        let objects = file
            .objects
            .into_iter()
            .map(|(object, states)| (object, glyphs(states)))
            .collect();
        let theme = Theme {
            tiles,
            objects,
            arrows: file.cursor.arrows,
            followed_arrows: file.cursor.followed_arrows,
            visible_frame: file.cursor.visible_frame,
            status_marks: file.cursor.status_marks,
            mode_marks,
            leash: glyph(file.cursor.leash),
            aim: glyph(file.cursor.aim.path),
            aim_end: glyph(file.cursor.aim.end),
            attention_marker: file.attention_marker.into(),
            drives: file
                .drives
                .into_iter()
                .map(|(drive, colour)| (drive, colour.into()))
                .collect(),
        };
        if let Some((what, symbol)) = theme
            .symbols()
            .into_iter()
            .find(|&(_, symbol)| !terra_sim::is_cp437(symbol))
        {
            return Err(format!(
                "its glyph '{symbol}' for {what} isn't in CP437, which everything on screen stays within"
            ));
        }
        Ok(theme)
    }

    /// Every character this theme draws, with what it's for, in the file's
    /// words.
    fn symbols(&self) -> Vec<(String, char)> {
        let tiles = self
            .tiles
            .iter()
            .map(|(&tile, glyph)| (tile_name(tile), glyph.symbol));
        let objects = self.objects.iter().flat_map(|(object, states)| {
            states
                .iter()
                .map(move |(state, glyph)| (format!("\"{object}\" \"{state}\""), glyph.symbol))
        });
        let arrows = [
            ("arrows", self.arrows),
            ("followed_arrows", self.followed_arrows),
            ("visible_frame", self.visible_frame),
        ]
        .into_iter()
        .flat_map(|(name, a)| {
            [
                ("up", a.up),
                ("down", a.down),
                ("left", a.left),
                ("right", a.right),
            ]
            .map(|(side, symbol)| (format!("the cursor's {name} {side}"), symbol))
        });
        let m = self.status_marks;
        let marks = [
            ("idle", m.idle),
            ("sent", m.sent),
            ("applied", m.applied),
            ("rejected", m.rejected),
            ("grab", m.grab),
            ("empty", m.empty),
            ("release", m.release),
        ]
        .map(|(mark, symbol)| (format!("the cursor's {mark} status mark"), symbol));
        let mode_marks = self
            .mode_marks
            .iter()
            .map(|(mode, glyph)| (format!("the {} mode mark", snake_case(mode)), glyph.symbol));
        let lines = [
            ("the leash", self.leash),
            ("the aim line", self.aim),
            ("the aim line's end", self.aim_end),
        ]
        .map(|(what, glyph)| (what.to_string(), glyph.symbol));
        tiles
            .chain(objects)
            .chain(arrows)
            .chain(marks)
            .chain(mode_marks)
            .chain(lines)
            .collect()
    }
}

/// A semantic tile as a theme file names it, such as `terrain(deep_water)`.
fn tile_name(tile: SemanticTile) -> String {
    match tile {
        SemanticTile::Terrain(terrain) => format!("terrain({})", snake_case(&terrain)),
        SemanticTile::Emote(emote) => format!("emote({})", snake_case(&emote)),
        tile => snake_case(&tile),
    }
}

/// A unit variant's name as theme files spell it: `ShallowWater` is
/// `shallow_water`.
fn snake_case(variant: &impl std::fmt::Debug) -> String {
    let mut name = String::new();
    for (i, c) in format!("{variant:?}").chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                name.push('_');
            }
            name.push(c.to_ascii_lowercase());
        } else {
            name.push(c);
        }
    }
    name
}

/// Names, each in quotes, separated by commas.
fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Turns a theme file's entries into glyphs.
fn glyphs<K: Ord>(entries: BTreeMap<K, GlyphEntry>) -> BTreeMap<K, Glyph> {
    entries
        .into_iter()
        .map(|(key, entry)| (key, glyph(entry)))
        .collect()
}

/// Turns a theme file's entry into a glyph.
fn glyph(entry: GlyphEntry) -> Glyph {
    Glyph {
        symbol: entry.glyph,
        fg: entry.fg.into(),
        bold: entry.bold,
        reversed: entry.reversed,
    }
}

/// A theme file, as written in `themes/*.ron`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    tiles: BTreeMap<SemanticTile, GlyphEntry>,
    /// By drive name, from the data pack's `chemicals.ron`, the colour of a
    /// sprite whose strongest drive it is (design §6.3).
    drives: BTreeMap<String, Colour>,
    /// By object type name (from the data pack's `objects.ron`), then visual state.
    objects: BTreeMap<String, BTreeMap<String, GlyphEntry>>,
    cursor: CursorFile,
    /// The Attention marker: the background of the tile the selected sprite
    /// attends to.
    attention_marker: Colour,
}

/// A theme file's `cursor` section.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorFile {
    arrows: Arrows,
    /// The arrows while the Cursor follows a sprite.
    followed_arrows: Arrows,
    /// The sides while sprites can see the Cursor, named as the arrows they
    /// replace.
    visible_frame: Arrows,
    status_marks: StatusMarks,
    mode_marks: BTreeMap<CursorMode, GlyphEntry>,
    /// A dot of the leash, from the Cursor to a sprite it leads.
    leash: GlyphEntry,
    /// The aim line, while the player aims a throw or a shove.
    aim: AimFile,
}

/// A theme file's aim line (design v25 §6.5).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AimFile {
    /// A dot of the path.
    path: GlyphEntry,
    /// Where it would stop.
    end: GlyphEntry,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlyphEntry {
    glyph: char,
    fg: Colour,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    reversed: bool,
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
