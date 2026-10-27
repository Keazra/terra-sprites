//! The settings screen's rows (design §6.7, #307): how the game looks and
//! feels, as the player chooses it; and `settings.ron`, which holds what of
//! them the player has changed, in the game's folder, read at start.
//! They're the player's, never the world's.

use std::io;
use std::path::Path;

use serde::Deserialize;
use terra_sim::DataPack;

use crate::app::shown;
use crate::files;
use crate::listed::listed_enum;
use crate::theme::{ColourPalette, GlyphSet, Part, Pick, Theme};

/// The settings' file's name, in the game's folder (design §6.7).
pub const FILE: &str = "settings.ron";

listed_enum! {
    /// A row of the settings screen (design §6.7, #307), in its order.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Setting {
        /// The glyph set (§6.2).
        Glyphs => "Glyphs",
        /// The palette (§6.2).
        Colours => "Colours",
        /// Motion's switch (§6.9).
        Motion => "Motion",
        /// Square tiles, the switch `o` is (M2 design §11.7).
        SquareTiles => "Square tiles",
    }

    /// Every row, in the order the screen shows them.
    pub const ALL;

    /// The row's name on the screen.
    pub fn label(self) -> &'static str;
}

/// How the game is drawn (design §6.2, §6.7, #307): the glyph set and the
/// palette in use, each as the player picked it, built in or a theme file,
/// with the theme it's taken from.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    glyphs: Pick<GlyphSet>,
    glyph_theme: Theme,
    palette: Pick<ColourPalette>,
    palette_theme: Theme,
}

impl Look {
    /// The glyph set `glyphs`, its characters taken from `glyph_theme`, in
    /// the palette `palette`, the rest taken from `palette_theme`.
    pub fn new(
        glyphs: Pick<GlyphSet>,
        glyph_theme: Theme,
        palette: Pick<ColourPalette>,
        palette_theme: Theme,
    ) -> Look {
        Look {
            glyphs,
            glyph_theme,
            palette,
            palette_theme,
        }
    }

    /// `theme`, as a new app draws until it's told what the player has
    /// picked, taken as the CP437 glyph set in the default palette.
    pub fn of(theme: &Theme) -> Look {
        Look::new(
            Pick::BuiltIn(GlyphSet::Cp437),
            theme.clone(),
            Pick::BuiltIn(ColourPalette::Default),
            theme.clone(),
        )
    }

    /// The glyph set, as picked.
    pub fn glyphs(&self) -> &Pick<GlyphSet> {
        &self.glyphs
    }

    /// The palette, as picked.
    pub fn palette(&self) -> &Pick<ColourPalette> {
        &self.palette
    }

    /// The theme they make: the glyph set's characters in the palette.
    pub fn theme(&self) -> Theme {
        Theme::compose(&self.glyph_theme, &self.palette_theme)
    }

    /// The same, with the glyph set `glyphs`, taken from `theme`.
    pub(crate) fn with_glyphs(self, glyphs: Pick<GlyphSet>, theme: Theme) -> Look {
        Look {
            glyphs,
            glyph_theme: theme,
            ..self
        }
    }

    /// The same, in the palette `palette`, taken from `theme`.
    pub(crate) fn with_palette(self, palette: Pick<ColourPalette>, theme: Theme) -> Look {
        Look {
            palette,
            palette_theme: theme,
            ..self
        }
    }
}

impl Default for Look {
    /// The CP437 glyph set in the default palette.
    fn default() -> Look {
        Look::of(&Theme::cp437())
    }
}

/// The settings screen's rows as the game carries them, from world to
/// world and back to the title screen (design §6.7, #307): the look, motion
/// and square tiles; and what of them the player has changed, as
/// `settings.ron` holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub look: Look,
    /// Whether motion is on (§6.9).
    pub motion: bool,
    /// Whether tiles are drawn two cells wide (M2 design §11.7).
    pub square_tiles: bool,
    pub saved: SavedSettings,
}

impl Default for Settings {
    /// The game's own: the CP437 glyph set in the default palette, motion
    /// on and square tiles off, with nothing changed.
    fn default() -> Settings {
        Settings {
            look: Look::default(),
            motion: true,
            square_tiles: false,
            saved: SavedSettings::default(),
        }
    }
}

/// What `settings.ron` holds (design §6.7, #307): each row of the settings
/// screen the player has changed, and none they haven't, so a later
/// version's new default still reaches a row they never touched. A glyph
/// set or a palette built in is named as the game shows it, `"ASCII"` or
/// `"high contrast"`, and a theme file by its file's name, `"night.ron"`.
/// A field the game doesn't know, from a later version, is passed over.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SavedSettings {
    #[serde(deserialize_with = "written")]
    pub glyphs: Option<String>,
    #[serde(deserialize_with = "written")]
    pub colours: Option<String>,
    #[serde(deserialize_with = "written")]
    pub motion: Option<bool>,
    #[serde(deserialize_with = "written")]
    pub square_tiles: Option<bool>,
}

/// A field the file has, written without `Some(…)`.
fn written<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    field: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(field).map(Some)
}

/// The line `settings.ron` starts with, for a player who opens it.
const HEADER: &str =
    "// Terra Sprites' settings: what you've changed on the settings screen (Ctrl+T).";

impl SavedSettings {
    /// The settings in the text of a `settings.ron`.
    pub fn from_ron(text: &str) -> Result<SavedSettings, String> {
        ron::from_str(text).map_err(|err| err.to_string())
    }

    /// The text of a `settings.ron` holding these: a field for each row
    /// changed, and none for the rest.
    pub fn to_ron(&self) -> String {
        let fields: Vec<(&str, String)> = [
            ("glyphs", self.glyphs.as_ref().map(ron_value)),
            ("colours", self.colours.as_ref().map(ron_value)),
            ("motion", self.motion.as_ref().map(ron_value)),
            ("square_tiles", self.square_tiles.as_ref().map(ron_value)),
        ]
        .into_iter()
        .filter_map(|(name, value)| Some((name, value?)))
        .collect();
        if fields.is_empty() {
            return format!("{HEADER}\n()\n");
        }
        let lines: String = fields
            .iter()
            .map(|(name, value)| format!("    {name}: {value},\n"))
            .collect();
        format!("{HEADER}\n(\n{lines})\n")
    }
}

/// `value` as RON writes it: a string quoted, with what it holds escaped.
fn ron_value(value: &(impl serde::Serialize + ?Sized)) -> String {
    ron::to_string(value).expect("a string or a bool is always written")
}

/// What the `settings.ron` at `path` holds: nothing changed if there's no
/// file there; why not, if it can't be read or isn't settings.
pub fn read(path: &Path) -> Result<SavedSettings, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => SavedSettings::from_ron(&text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(SavedSettings::default()),
        Err(err) => Err(err.to_string()),
    }
}

/// Saves `saved` as the `settings.ron` at `path`, making its folder if need
/// be: written beside it and then moved into its place, as a save is, so
/// one cut short never breaks the one there (design §6.7).
pub fn write(path: &Path, saved: &SavedSettings) -> io::Result<()> {
    files::write_in_place(path, saved.to_ron().as_bytes())
}

/// The name the game shows `pick` by (design §6.7): a built-in one's, or a
/// theme file's, without its `.ron`, as a file's name is shown.
pub(crate) fn name_of<T: Part>(pick: &Pick<T>) -> String {
    match pick {
        Pick::BuiltIn(part) => part.name().to_string(),
        Pick::File(path) => shown(
            &path
                .file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()),
        ),
    }
}

/// The name `settings.ron` gives `pick`: a built-in one's, as the game
/// shows it, or a theme file's file name, with its `.ron`.
pub(crate) fn saved_name<T: Part>(pick: &Pick<T>) -> String {
    match pick {
        Pick::BuiltIn(part) => part.name().to_string(),
        Pick::File(path) => path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
    }
}

/// The glyph set or palette `settings.ron` names `name`: a theme file in
/// `theme_folder` by its file name, with its `.ron`, or one built in by its
/// name, in any case. None if it names neither.
pub fn named<T: Part>(name: &str, theme_folder: Option<&Path>) -> Option<Pick<T>> {
    if name.ends_with(".ron") {
        let path = theme_folder.map_or_else(|| name.into(), |folder| folder.join(name));
        return Some(Pick::File(path));
    }
    T::all()
        .iter()
        .find(|part| part.name().eq_ignore_ascii_case(name))
        .map(|&part| Pick::BuiltIn(part))
}

/// The theme `pick` is taken from: a built-in one's, or a theme file's,
/// read and checked against `data`, the data pack it will draw (design v34
/// §6.7).
pub fn theme_of<T: Part>(pick: &Pick<T>, data: &DataPack) -> Result<Theme, String> {
    match pick {
        Pick::BuiltIn(part) => Ok(part.source()),
        Pick::File(path) => std::fs::read_to_string(path)
            .map_err(|err| err.to_string())
            .and_then(|text| Theme::from_ron(&text, data)),
    }
}
