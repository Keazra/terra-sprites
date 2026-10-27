//! The settings screen (`Ctrl+T`, design §6.7, #307): a box over the map,
//! like a menu's, a row each for how the game looks and feels, each
//! changed at once.

use std::path::PathBuf;

use terra_sim::World;

use super::{App, Screen};
use crate::settings::{self, Look, Setting, Settings, name_of, saved_name};
use crate::theme::{ColourPalette, GlyphSet, Part, Pick, Theme};

/// What the settings screen's on-off rows show.
const ON: &str = "on";
const OFF: &str = "off";

/// What an on-off row shows.
fn on_off(on: bool) -> &'static str {
    if on { ON } else { OFF }
}

impl App {
    /// Sets where the settings are saved, `settings.ron` in the game's
    /// folder (design §6.7, #307). Without one, they last only until the
    /// game is closed.
    pub fn set_settings_file(&mut self, path: PathBuf) {
        self.settings_file = Some(path);
    }

    /// Says on the status line, as the world opens, why what `settings.ron`
    /// holds wasn't all taken: it couldn't be read, or names a theme that
    /// no longer loads, so the game started on its own instead (design
    /// §6.7, #307).
    pub fn note_unread_settings(&mut self, why: String) {
        self.refuse(why);
    }

    /// Saves what the player has changed of the settings, if there's a file
    /// to save them in, saying so if they can't be (design §6.7, #307). A
    /// file that couldn't be read as the game started is kept aside first,
    /// as `settings.ron.broken`, saying so once, rather than written over.
    pub(super) fn save_settings(&mut self) {
        let Some(path) = self.settings_file.clone() else {
            return;
        };
        let mut kept = false;
        if self.settings_unreadable {
            match settings::keep_unreadable(&path) {
                Ok(was_there) => kept = was_there,
                Err(err) => {
                    let broken = settings::BROKEN;
                    self.refuse(format!(
                        "Couldn't save the settings: couldn't keep the old ones as {broken}: {err}"
                    ));
                    return;
                }
            }
            self.settings_unreadable = false;
        }
        if let Err(err) = settings::write(&path, &self.saved_settings) {
            self.refuse(format!("Couldn't save the settings: {err}"));
        } else if kept {
            let broken = settings::BROKEN;
            self.tell_player(format!(
                "The settings couldn't be read, so they were kept as {broken}"
            ));
        }
    }

    /// The settings as the player has left them, to carry to the next
    /// world (design §6.7, #307).
    pub fn settings(&self) -> Settings {
        Settings {
            look: self.look.clone(),
            motion: self.motion(),
            square_tiles: self.zoom,
            saved: self.saved_settings.clone(),
            unreadable: self.settings_unreadable,
        }
    }

    /// Takes `settings` from now on, as a world starts (design §6.7, #307):
    /// draws in its look, turns motion on or off, and draws square tiles
    /// or not, setting the view up as `o` does but saying nothing; and saves
    /// what's changed next with what it says was saved.
    pub fn use_settings(&mut self, settings: Settings) {
        self.use_look(settings.look);
        self.set_motion(settings.motion);
        self.set_square_tiles(settings.square_tiles);
        self.saved_settings = settings.saved;
        self.settings_unreadable = settings.unreadable;
    }

    /// The glyph set and the palette the player has picked (design §6.2,
    /// §6.7, #307).
    pub fn look(&self) -> &Look {
        &self.look
    }

    /// The settings screen's rows, in order, each with what it shows
    /// (design §6.7, #307).
    pub fn settings_rows(&self) -> Vec<(Setting, String)> {
        Setting::ALL
            .iter()
            .map(|&row| (row, self.shown_setting(row)))
            .collect()
    }

    /// What row `row` of the settings screen shows.
    fn shown_setting(&self, row: Setting) -> String {
        match row {
            Setting::Glyphs => name_of(self.look.glyphs()),
            Setting::Colours => name_of(self.look.palette()),
            Setting::Motion => on_off(self.motion()).into(),
            Setting::SquareTiles => on_off(self.zoom).into(),
        }
    }

    /// The settings screen's rows as its box writes them, after their
    /// numbers: each row's name, then what it shows between arrows, each in
    /// a column as wide as the longest it can be, so the box keeps its size
    /// as rows change.
    pub(super) fn settings_lines(&self) -> Vec<String> {
        let label = Setting::ALL
            .iter()
            .map(|row| row.label().chars().count())
            .max()
            .unwrap_or(0);
        let glyphs = self.options::<GlyphSet>();
        let palettes = self.options::<ColourPalette>();
        let value = glyphs
            .iter()
            .map(name_of)
            .chain(palettes.iter().map(name_of))
            .chain([ON.to_string(), OFF.to_string()])
            .map(|name| name.chars().count())
            .max()
            .unwrap_or(0);
        self.settings_rows()
            .into_iter()
            .map(|(row, shows)| format!("{:<label$}  ◄ {shows:<value$} ►", row.label()))
            .collect()
    }

    /// `Ctrl+T`: opens the settings screen on its first row, offering the
    /// `.ron` files in the themes folder, by name, in order, after the
    /// built-in glyph sets and palettes (design §6.7, #307).
    pub(super) fn open_settings(&mut self) {
        self.end_aim();
        self.theme_files = super::menus::ron_files(self.theme_folder.as_deref());
        self.open_menu(Screen::Settings);
    }

    /// Every glyph set or palette the settings screen offers: the built-in
    /// ones, then the theme files.
    fn options<T: Part>(&self) -> Vec<Pick<T>> {
        T::all()
            .iter()
            .map(|&part| Pick::BuiltIn(part))
            .chain(
                self.theme_files
                    .iter()
                    .map(|(_, path)| Pick::File(path.clone())),
            )
            .collect()
    }

    /// The one of the settings screen's `options` `by` along from
    /// `current`, round from either end: from the first, or the last going
    /// back, where `current` isn't one of them, as a theme file `--theme`
    /// gives from elsewhere isn't.
    fn moved<T: PartialEq + Clone>(options: &[T], current: &T, by: i32) -> T {
        let count = options.len() as i32;
        let next = match options.iter().position(|option| option == current) {
            Some(at) => (at as i32 + by).rem_euclid(count),
            None if by > 0 => 0,
            None => count - 1,
        };
        options[next as usize].clone()
    }

    /// Changes row `row` of the settings screen `by` a value along, at
    /// once, says so on the status line, as "Colours: mono", and saves it
    /// (design §6.7, #307): that row alone, so what the flags pick for the
    /// run is never saved. A theme file that doesn't load is refused, and
    /// the row keeps what it had.
    pub(super) fn change_setting(&mut self, row: Setting, by: i32, world: &World) {
        match row {
            Setting::Glyphs => {
                let pick = App::moved(&self.options::<GlyphSet>(), self.look.glyphs(), by);
                let Some(theme) = self.theme_for(&pick, world) else {
                    return;
                };
                self.saved_settings.glyphs = Some(saved_name(&pick));
                self.use_look(self.look.clone().with_glyphs(pick, theme));
            }
            Setting::Colours => {
                let pick = App::moved(&self.options::<ColourPalette>(), self.look.palette(), by);
                let Some(theme) = self.theme_for(&pick, world) else {
                    return;
                };
                self.saved_settings.colours = Some(saved_name(&pick));
                self.use_look(self.look.clone().with_palette(pick, theme));
            }
            Setting::Motion => {
                self.set_motion(!self.motion());
                self.saved_settings.motion = Some(self.motion());
            }
            Setting::SquareTiles => {
                self.set_square_tiles(!self.zoom);
                self.saved_settings.square_tiles = Some(self.zoom);
            }
        }
        self.tell_player(format!("{}: {}", row.label(), self.shown_setting(row)));
        self.save_settings();
    }

    /// Draws with `look` from now on.
    fn use_look(&mut self, look: Look) {
        self.theme = look.theme();
        self.look = look;
    }

    /// The theme `pick` is taken from, or none, saying why, if it's a file
    /// that doesn't load.
    fn theme_for<T: Part>(&mut self, pick: &Pick<T>, world: &World) -> Option<Theme> {
        match settings::theme_of(pick, world.data()) {
            Ok(theme) => Some(theme),
            Err(why) => {
                self.refuse(format!("Couldn't use theme {}: {why}", name_of(pick)));
                None
            }
        }
    }
}
