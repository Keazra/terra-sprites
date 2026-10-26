//! The menus and the folders they read: the Place menu and what waits on
//! the Cursor to be placed (design v28 §6.5), the genome files, and the
//! themes (v34 §6.7); and saving a genome to its folder (§6.7).

use std::path::{Path, PathBuf};

use ratatui::layout::Rect;
use terra_sim::{Command, Genome, Pos, World};

use super::{App, Flow, PlaceItem, Placing, Screen, Selection, highlight_moved, shown};
use crate::input::Action;
use crate::theme::Theme;

/// The Place menu's own items, after the object types the data offers.
const NEW_SPRITE: &str = "new sprite";
const FROM_A_FILE: &str = "sprite from a genome file";

/// A built-in theme: its name, and how to make it.
type BuiltInTheme = (&'static str, fn() -> Theme);

/// The themes built into the game, first on the themes menu (design v34
/// §6.7).
const BUILT_IN_THEMES: [BuiltInTheme; 2] = [("cp437", Theme::cp437), ("ascii", Theme::ascii)];

/// The `.ron` files in `folder`, by name, with their paths, in order. None
/// if there's no folder, or it can't be read.
fn ron_files(folder: Option<&Path>) -> Vec<(String, PathBuf)> {
    let mut files: Vec<(String, PathBuf)> = folder
        .and_then(|folder| std::fs::read_dir(folder).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
            let is_ron = path.extension().is_some_and(|ext| ext == "ron");
            let name = path.file_stem()?.to_string_lossy().into_owned();
            (is_file && is_ron).then_some((name, path))
        })
        .collect();
    files.sort();
    files
}

impl App {
    /// Sets the game's folder for its files (design §6.7), which the help
    /// screen shows.
    pub fn set_data_folder(&mut self, folder: PathBuf) {
        self.data_folder = Some(folder);
    }

    /// The game's folder for its files, if there is one.
    pub fn data_folder(&self) -> Option<&Path> {
        self.data_folder.as_deref()
    }

    /// Sets where genome files are saved and read from (design §6.7).
    pub fn set_genome_folder(&mut self, folder: PathBuf) {
        self.genome_folder = Some(folder);
    }

    /// Sets the folder the player's own themes are read from (design v34
    /// §6.7).
    pub fn set_theme_folder(&mut self, folder: PathBuf) {
        self.theme_folder = Some(folder);
    }

    /// What the Place menu item waiting on the Cursor is called, if one is
    /// (design v28 §6.5).
    pub fn placing(&self) -> Option<&str> {
        self.placing.as_ref().map(|placing| placing.label.as_str())
    }

    /// What the Place menu item waiting on the Cursor makes, if one is.
    pub fn placing_item(&self) -> Option<&PlaceItem> {
        self.placing.as_ref().map(|placing| &placing.item)
    }

    /// The open menu's items, in order: the Place menu's, or the genome
    /// files'. Empty with no menu open.
    pub fn menu_items(&self, world: &World) -> Vec<String> {
        match self.screen {
            Screen::PlaceMenu => world
                .data()
                .placeable()
                .map(|(_, label)| label.to_string())
                .chain([NEW_SPRITE.to_string(), FROM_A_FILE.to_string()])
                .collect(),
            Screen::GenomeMenu => self
                .genome_files
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
            Screen::ThemeMenu => BUILT_IN_THEMES
                .iter()
                .map(|(name, _)| format!("{name} (built in)"))
                .chain(self.theme_files.iter().map(|(name, _)| shown(name)))
                .collect(),
            Screen::LoadMenu => self
                .save_files
                .iter()
                .map(|file| shown(&file.name))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The open menu's highlighted item, from 0.
    pub fn menu_choice(&self) -> usize {
        self.menu_choice
    }

    /// The folder genome files are read from, if there is one.
    pub fn genome_folder(&self) -> Option<&Path> {
        self.genome_folder.as_deref()
    }

    /// The open menu's title, or `None` with no menu open.
    pub fn menu_title(&self) -> Option<&'static str> {
        match self.screen {
            Screen::PlaceMenu => Some(" Place "),
            Screen::GenomeMenu => Some(" Genome files "),
            Screen::LoadMenu => Some(" Load "),
            Screen::ThemeMenu => Some(" Themes "),
            _ => None,
        }
    }

    /// Where the open menu is drawn: from the map view's top-left tile, as
    /// wide as its longest item, numbered, inside a border, and one row per
    /// item, or one saying there are none. It stays within the map view, so
    /// a long list shows as many items as fit (see `menu_first`).
    pub fn menu_area(&self, world: &World) -> Option<Rect> {
        let title = self.menu_title()?;
        let items = self.menu_items(world);
        let empty = self.menu_empty();
        let widest = items
            .iter()
            .map(|item| item.chars().count() + 3)
            .chain([title.chars().count() + 2, empty.chars().count() + 1])
            .max()
            .unwrap_or(0);
        let rows = items.len().max(1);
        let wanted = Rect::new(
            self.tile_area.x,
            self.tile_area.y,
            (widest + 3).min(usize::from(u16::MAX)) as u16,
            (rows + 2).min(usize::from(u16::MAX)) as u16,
        );
        Some(wanted.intersection(self.tile_area))
    }

    /// How many of the open menu's items fit in it at once.
    fn menu_rows(&self, world: &World) -> usize {
        self.menu_area(world)
            .map_or(0, |area| usize::from(area.height.saturating_sub(2)))
    }

    /// The first item the open menu shows: the list scrolls so the
    /// highlighted item stays in view.
    pub fn menu_first(&self, world: &World) -> usize {
        let rows = self.menu_rows(world).max(1);
        self.menu_choice.saturating_sub(rows - 1)
    }

    /// What the open menu says with nothing to list.
    pub fn menu_empty(&self) -> String {
        match self.screen {
            Screen::GenomeMenu => self.no_genome_files(),
            Screen::LoadMenu => self.no_saves(),
            _ => String::new(),
        }
    }

    /// What the genome menu says with no files to list.
    pub fn no_genome_files(&self) -> String {
        match &self.genome_folder {
            Some(folder) => format!("No genome files in {}", folder.display()),
            None => "No folder for genome files".into(),
        }
    }

    /// Opens `menu`, highlighting its first item.
    pub(super) fn open_menu(&mut self, menu: Screen) {
        self.screen = menu;
        self.menu_choice = 0;
    }

    /// What an action does while a menu is open: a number or `Enter` picks
    /// an item, the arrow keys or the wheel move the highlight, a click
    /// picks the item under it or, off the menu, closes it, and `Esc`
    /// closes it. `?` opens help over it (#303).
    pub(super) fn apply_in_menu(&mut self, action: Action, world: &World) -> Flow {
        let count = self.menu_items(world).len();
        match action {
            // Numbers count from 1.
            Action::Pick(n) if (1..=count).contains(&usize::from(n)) => {
                self.choose(usize::from(n) - 1, world);
            }
            Action::Enter if count > 0 => self.choose(self.menu_choice, world),
            Action::Scroll { dy, .. } if count > 0 => {
                self.menu_choice = highlight_moved(self.menu_choice, dy.signum(), count);
            }
            // The wheel moves the highlight a row a notch, as the sprite
            // list's does (#128).
            Action::Wheel { at, notches } if count > 0 => {
                self.point(at);
                self.menu_choice = highlight_moved(self.menu_choice, notches, count);
            }
            Action::Click { at, .. } => {
                let area = self.menu_area(world).expect("a menu is open");
                let row = usize::from(at.y.wrapping_sub(area.y + 1));
                let item = self.menu_first(world) + row;
                if !area.contains(at) {
                    self.screen = Screen::Normal;
                } else if row < self.menu_rows(world) && item < count {
                    self.choose(item, world);
                }
            }
            Action::Point(cell) => self.point(cell),
            Action::Back => self.screen = Screen::Normal,
            // Closing help goes back to the menu (#303).
            Action::Help => self.open_help(),
            _ => {}
        }
        self.settle_cursor(world);
        Flow::Continue
    }

    /// Picks item `index` of the open menu (design v28 §6.5): an object type the
    /// data offers, or a new sprite, waits on the Cursor; the Place menu's
    /// last item lists the genome files; a genome file is read, and its
    /// sprite waits.
    fn choose(&mut self, index: usize, world: &World) {
        let menu = self.screen;
        let label = self.menu_items(world)[index].clone();
        self.screen = Screen::Normal;
        let placeable: Vec<&str> = world.data().placeable().map(|(name, _)| name).collect();
        let item = match menu {
            Screen::LoadMenu => {
                let file = self.save_files[index].clone();
                return self.ask_to_load(file, world);
            }
            Screen::ThemeMenu => return self.use_theme(index, world),
            Screen::GenomeMenu => {
                let (name, path) = &self.genome_files[index];
                let read = std::fs::read_to_string(path)
                    .map_err(|err| err.to_string())
                    .and_then(|text| {
                        Genome::from_ron(&text, world.data()).map_err(|err| err.to_string())
                    });
                match read {
                    Ok(genome) => PlaceItem::Genome(genome),
                    Err(why) => return self.refuse(format!("Couldn't read {name}: {why}")),
                }
            }
            _ if index < placeable.len() => PlaceItem::Object {
                object_type: world
                    .data()
                    .object_type_id(placeable[index])
                    .expect("a placeable type"),
                name: placeable[index].to_string(),
            },
            _ if index == placeable.len() => PlaceItem::NewSprite,
            _ => return self.list_genome_files(),
        };
        self.placing = Some(Placing { item, label });
    }

    /// Opens the genome menu, listing the `.ron` files in the genome folder
    /// by name, in order.
    fn list_genome_files(&mut self) {
        self.genome_files = ron_files(self.genome_folder.as_deref());
        self.open_menu(Screen::GenomeMenu);
    }

    /// `Ctrl+T`: opens the themes menu, listing the built-in themes, then
    /// the `.ron` files in the themes folder by name, in order (design v34
    /// §6.7).
    pub(super) fn list_themes(&mut self) {
        self.end_aim();
        self.theme_files = ron_files(self.theme_folder.as_deref());
        self.open_menu(Screen::ThemeMenu);
    }

    /// Draws the map with the themes menu's item `index` from now on: a
    /// built-in theme, or a file read and checked against the world's pack.
    /// A file that doesn't load is refused, and the theme in use stays.
    fn use_theme(&mut self, index: usize, world: &World) {
        let (name, theme) = match BUILT_IN_THEMES.get(index) {
            Some(&(name, theme)) => (name.to_string(), theme()),
            None => {
                let (name, path) = &self.theme_files[index - BUILT_IN_THEMES.len()];
                let read = std::fs::read_to_string(path)
                    .map_err(|err| err.to_string())
                    .and_then(|text| Theme::from_ron(&text, world.data()));
                match read {
                    Ok(theme) => (shown(name), theme),
                    Err(why) => {
                        return self.refuse(format!("Couldn't use theme {}: {why}", shown(name)));
                    }
                }
            }
        };
        self.theme = theme;
        self.tell_player(format!("Theme: {name}"));
    }

    /// Places the item waiting on the Cursor on `tile` (design v28 §6.5).
    pub(super) fn place(&mut self, tile: Pos) {
        let Some(placing) = self.placing.take() else {
            return;
        };
        let command = match placing.item {
            PlaceItem::Object { object_type, .. } => Command::Place { tile, object_type },
            PlaceItem::NewSprite => Command::SpawnSprite { tile, genome: None },
            PlaceItem::Genome(genome) => Command::SpawnSprite {
                tile,
                genome: Some(genome),
            },
        };
        self.commands.push(command);
        // The marks follow the latest click (design v22 §6.5).
        self.report = None;
    }

    /// Saves the selected sprite's genome, or the selected egg's, to a file
    /// in the genome folder (design §6.1, §6.7), named for the sprite and
    /// the tick, and says where.
    pub(super) fn export_genome(&mut self, world: &World) {
        let genome = |id| match world.sprite(id) {
            Some(sprite) => Some(sprite.genome().to_ron(world.data())),
            None => Some(world.egg(id)?.genome().to_ron(world.data())),
        };
        let Some((id, text)) = self.selection.and_then(|s| match s {
            Selection::Living(id) => Some((id, genome(id)?)),
            Selection::Dead { .. } => None,
        }) else {
            return self.refuse("Select a sprite to save its genome".into());
        };
        let Some(folder) = self.genome_folder.clone() else {
            return self.refuse("There's no folder to save genomes in".into());
        };
        let label = self.names.label(id);
        let slug: String = label
            .chars()
            .filter_map(|c| match c {
                c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
                ' ' | '-' => Some('-'),
                _ => None,
            })
            .collect();
        let file = format!("{slug}-tick-{}.ron", world.tick());
        let saved = std::fs::create_dir_all(&folder)
            .and_then(|()| std::fs::write(folder.join(&file), text));
        match saved {
            Ok(()) => {
                let path = folder.join(&file);
                self.tell_player(format!("Saved {label}'s genome to {}", path.display()));
            }
            Err(err) => self.refuse(format!("Couldn't save the genome: {err}")),
        }
    }
}
