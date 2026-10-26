//! The help screen's pages and its search, and the command palette (design
//! §6.1, #303).

use ratatui::layout::Position;
use terra_sim::World;

use super::{App, Draft, Flow, Screen, WHEEL_LINES, along, highlight_moved, typeable};
use crate::controls::{self, PaletteRow};
use crate::help::HelpPage;
use crate::input::{Action, Button};
use crate::sprite_list;

/// The most characters help's search or the palette takes.
pub const MAX_SEARCH_CHARS: usize = 30;

/// The help screen as it's open (#303).
#[derive(Debug, Clone)]
pub(super) struct HelpView {
    /// The page it shows.
    page: HelpPage,
    /// What's typed into its search, while it searches.
    search: Option<Draft>,
    /// How many lines it's scrolled down.
    scroll: usize,
    /// What was open when it opened, to go back to.
    from: Screen,
}

impl Default for HelpView {
    fn default() -> HelpView {
        HelpView {
            page: HelpPage::World,
            search: None,
            scroll: 0,
            from: Screen::Normal,
        }
    }
}

/// The command palette as it's open (#303): what's typed, and the
/// highlighted row of what it found.
#[derive(Debug, Clone)]
pub(super) struct Palette {
    typed: Draft,
    choice: usize,
}

impl App {
    /// Opens the help at the page for where the player is (#303): the
    /// sprite list's, the population graph's or the family tree's over
    /// those, the inspector's once the player last pointed at it or used
    /// its keys, and otherwise the world's.
    pub(super) fn open_help(&mut self) {
        self.end_aim();
        self.help = HelpView {
            page: self.help_page_here(),
            from: self.screen,
            ..HelpView::default()
        };
        self.screen = Screen::Help;
    }

    /// The help's page for where the player is, which `?` opens and the top
    /// bar names beside it (#303).
    pub fn help_page_here(&self) -> HelpPage {
        match self.screen {
            Screen::SpriteList => HelpPage::SpriteList,
            Screen::Population => HelpPage::Graph,
            Screen::FamilyTree => HelpPage::Tree,
            _ if self.on_inspector => HelpPage::Inspector,
            _ => HelpPage::World,
        }
    }

    /// The help's page.
    pub fn help_page(&self) -> HelpPage {
        self.help.page
    }

    /// What's typed into the help's search, while it searches.
    pub fn help_search(&self) -> Option<&str> {
        self.help.search.as_ref().map(Draft::text)
    }

    /// How many lines the help is scrolled down.
    pub fn help_scroll(&self) -> usize {
        self.help.scroll
    }

    /// What an action does while the help screen is open (#303): `←` and
    /// `→`, or `Tab` and `Shift+Tab`, turn its pages, `↑` and `↓` or the
    /// wheel scroll one too long to fit, and typing searches it,
    /// `Backspace` rubbing a letter out. A click on a page's name opens it,
    /// ending a search. `Esc` clears the search, then closes the help, as
    /// `?` does at once, going back to what was open. The mouse still
    /// points.
    pub(super) fn apply_in_help(&mut self, action: Action, world: &World) -> Flow {
        let last = self.help_scroll_room(world);
        let help = &mut self.help;
        match action {
            Action::Help | Action::Type('?') => self.close_help(),
            Action::Type(c) if typeable(c) => {
                let search = help.search.get_or_insert_with(Draft::default);
                search.type_char(c, MAX_SEARCH_CHARS);
                help.scroll = 0;
            }
            Action::Erase => {
                if let Some(search) = help.search.as_mut() {
                    search.erase();
                    if search.text().is_empty() {
                        help.search = None;
                    }
                }
                help.scroll = 0;
            }
            Action::Back if help.search.is_some() => {
                help.search = None;
                help.scroll = 0;
            }
            Action::Back => self.close_help(),
            Action::Scroll { dx, .. } if dx != 0 => self.turn_page(dx.signum()),
            Action::AnotherName => self.turn_page(1),
            Action::SelectPrevious => self.turn_page(-1),
            Action::Scroll { dy, .. } => {
                let scroll = help.scroll.saturating_add_signed(dy.signum() as isize);
                help.scroll = scroll.min(last);
            }
            Action::Wheel { at, notches } => {
                let lines = (notches * WHEEL_LINES) as isize;
                help.scroll = help.scroll.saturating_add_signed(lines).min(last);
                self.point(at);
            }
            Action::Click {
                at,
                button: Button::Left,
                ..
            } => {
                if let Some(page) = self.help_page_at(at) {
                    self.help = HelpView {
                        page,
                        from: self.help.from,
                        ..HelpView::default()
                    };
                }
                self.point(at);
            }
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }

    /// The page whose name the help's top edge writes at `at`, if any
    /// (#303).
    fn help_page_at(&self, at: Position) -> Option<HelpPage> {
        let area = self.overlay()?;
        let column = at.x.checked_sub(area.x + crate::ui::HELP_TITLE_AT)?;
        if at.y != area.y {
            return None;
        }
        crate::help::page_at(usize::from(column))
    }

    /// How far the help can scroll down: as far as brings its last line to
    /// the bottom of its box, or nowhere, when it fits.
    fn help_scroll_room(&self, world: &World) -> usize {
        let Some(area) = self.overlay() else {
            return 0;
        };
        // Inside the box's border.
        let width = usize::from(area.width.saturating_sub(2));
        let height = usize::from(area.height.saturating_sub(2));
        crate::help::lines(self, world, width)
            .len()
            .saturating_sub(height)
    }

    /// Turns the help `steps` pages along, wrapping round, to its top.
    fn turn_page(&mut self, steps: i32) {
        self.help.page = along(&HelpPage::ALL, self.help.page, steps);
        self.help.scroll = 0;
    }

    /// Closes the help, going back to what was open when it opened.
    fn close_help(&mut self) {
        self.screen = self.help.from;
    }

    /// Opens the command palette (#303), over the map with nothing typed.
    pub(super) fn open_palette(&mut self) {
        self.end_aim();
        self.palette = Some(Palette {
            typed: Draft::default(),
            choice: 0,
        });
        self.screen = Screen::Palette;
    }

    /// What's typed into the command palette, while it's open.
    pub fn palette_text(&self) -> Option<&str> {
        self.palette.as_ref().map(|palette| palette.typed.text())
    }

    /// The palette's highlighted row, from 0.
    pub fn palette_choice(&self) -> usize {
        self.palette.as_ref().map_or(0, |palette| palette.choice)
    }

    /// The rows the palette finds for what's typed, each key on a row of
    /// its own (#303): every key that works in the world with each word
    /// typed starting a word of it, of what it does, of its heading or of
    /// its other words; every one, with nothing typed.
    pub fn palette_rows(&self) -> Vec<PaletteRow> {
        let typed = sprite_list::words(self.palette_text().unwrap_or_default());
        controls::palette_rows()
            .filter(|row| {
                let text = controls::found_by(row.control, row.key.written, &[row.key]);
                finds(&text, &typed)
            })
            .collect()
    }

    /// What an action does while the command palette is open (#303):
    /// typing finds controls, `↑` and `↓` or the wheel choose one, `Enter`
    /// closes the palette and does what was chosen, if the palette can
    /// press its key, and `Esc` closes it.
    pub(super) fn apply_in_palette(&mut self, action: Action, world: &World) -> Flow {
        let rows = self.palette_rows();
        let Some(palette) = self.palette.as_mut() else {
            self.screen = Screen::Normal;
            return Flow::Continue;
        };
        match action {
            Action::Type(c) if typeable(c) => {
                palette.typed.type_char(c, MAX_SEARCH_CHARS);
                palette.choice = 0;
            }
            Action::Erase => {
                palette.typed.erase();
                palette.choice = 0;
            }
            Action::Scroll { dy, .. } => {
                palette.choice = highlight_moved(palette.choice, dy.signum(), rows.len());
            }
            Action::Wheel { at, notches } => {
                palette.choice = highlight_moved(palette.choice, notches, rows.len());
                self.point(at);
            }
            Action::Enter => {
                let chosen = rows.get(palette.choice).and_then(|row| row.key.action);
                self.close_palette();
                if let Some(action) = chosen {
                    return self.apply(action, world);
                }
            }
            Action::Back => self.close_palette(),
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }

    /// Closes the command palette.
    fn close_palette(&mut self) {
        self.palette = None;
        self.screen = Screen::Normal;
    }
}

/// Whether each word `typed` starts a word of `text`, in any order: `tree`
/// finds "family tree", and `gen sav` "save its genome". With nothing
/// typed, everything is found. Unlike the sprite list's filter, which reads
/// a row's words in order (#313), a control's words have no order to keep.
pub(crate) fn finds(text: &str, typed: &[String]) -> bool {
    let words = sprite_list::words(text);
    typed
        .iter()
        .all(|typed| words.iter().any(|word| word.starts_with(typed.as_str())))
}
