//! The title screen (M2 design §8): the screen the game opens on, before the
//! player's world is running. The opening scene plays, then the menu shows
//! over the world it woke, which runs behind it.

use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use terra_sim::{Mode, Pos, World};

use crate::app::{self, App, Draft, highlight_moved, shown};
use crate::clock::Speed;
use crate::controls::{self, Context};
use crate::input::Action;
use crate::listed::listed_enum;
use crate::saves::{self, SaveFile};
use crate::start::Preset;
use crate::theme::{Glyph, SemanticTile, Theme};
use crate::ui;

/// When the Cursor's light has arrived and starts to spread (M2 design
/// §8.1).
const LIGHT_ARRIVES: Duration = Duration::from_millis(1000);
/// When the light has filled the screen, and the title starts to fade in.
const LIGHT_FILLS: Duration = Duration::from_millis(4000);
/// When the scene ends and the menu shows.
const SCENE_ENDS: Duration = Duration::from_millis(5500);
/// How often the world behind the menu ticks: 4×, 5 ticks a second.
const TICK_EVERY: Duration = Duration::from_millis(200);
/// The most ticks the world catches up on at once, after a long frame.
const MOST_TICKS_AT_ONCE: u32 = 5;
/// The menu's box: as wide as the title lettering and a margin.
const BOX_WIDTH: u16 = 56;
/// The most saves the list of saves shows at once; it scrolls for more.
const LOAD_ROWS: usize = 12;
/// The list of saves' rows besides the saves: its border.
const LOAD_FRAME_ROWS: u16 = 2;
/// The most presets the New world box shows at once; it scrolls for more.
const PRESET_ROWS: usize = 8;
/// The New world box's rows besides its presets: the border, the seed,
/// the mode, a blank row, then under the presets a blank row, two rows of
/// hints and the border.
const NEW_WORLD_FRAME_ROWS: u16 = 8;
/// The longest seed: `u64::MAX` has 20 digits.
const MAX_SEED_DIGITS: usize = 20;
/// What `Esc` asks on the title screen (M2 design §8.2).
const QUIT_PROMPT: &str = "Quit? (y/n)";
/// The rows of the menu's box above its choices: the border, a blank row,
/// the lettering's two rows and another blank row.
const ABOVE_CHOICES: u16 = 5;
/// The menu's box's rows besides its choices: those above them, a blank
/// row under them and the border.
const MENU_FRAME_ROWS: u16 = ABOVE_CHOICES + 2;
/// Where in the menu's box the choices start, from its left edge.
const CHOICES_INDENT: u16 = 6;
/// Where the save Continue loads is named, from the choices' left edge:
/// past the widest choice, its mark and number, and a gap.
const CONTINUE_SAID_AT: u16 = 17;
/// How wide a label to the left of a field in the New world box is.
const LABEL_WIDTH: u16 = 9;
/// The modes New world offers, in order (M2 design §16.1).
const MODES: [(Mode, &str); 2] = [(Mode::Story, "Story"), (Mode::Lab, "Lab")];

/// What `mode` means in a few words, beside it in New world (M2 design
/// §16.1).
fn in_short(mode: Mode) -> String {
    match mode {
        Mode::Story => format!("{} at most", Speed::fastest_in(mode).label()),
        Mode::Lab => "every speed, for experiments".into(),
    }
}

/// What `mode` means, while `?` is held in New world (M2 design §16.1).
fn explained(mode: Mode) -> [String; 3] {
    let fastest = Speed::fastest_in(mode).label();
    let [what, speeds] = match mode {
        Mode::Story => [
            "Story mode: the game as it's meant to be played.".to_string(),
            format!("· Speeds up to {fastest}, so a life never feels rushed."),
        ],
        Mode::Lab => [
            "Lab mode: a world for experiments.".to_string(),
            format!("· Every speed, up to {fastest}; it can be fast-forwarded."),
        ],
    };
    let kept = "· A world keeps its mode for its whole life.".to_string();
    [what, speeds, kept]
}

listed_enum! {
    /// A choice on the title screen's menu.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Choice {
        /// Load the newest save; offered only when there is one, so it stays
        /// first ([`Title::choices`]).
        Continue => "Continue",
        NewWorld => "New world",
        Load => "Load",
        Help => "Help",
        Quit => "Quit",
    }

    /// Every choice, in the menu's order.
    pub const ALL;

    pub fn label(self) -> &'static str;
}

/// What the game does after an action on the title screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleFlow {
    /// The title screen stays.
    Stay,
    Quit,
    /// Start a new world from `seed`, with the preset in the file, or the
    /// default, in `mode`.
    New {
        seed: u64,
        preset: Option<PathBuf>,
        mode: Mode,
    },
    /// Load the world in this save.
    Load(SaveFile),
}

/// What's open on the title screen.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Screen {
    Menu,
    /// "Quit? (y/n)" waits for an answer.
    QuitPrompt,
    /// The New world box (M2 design §8.3).
    NewWorld(NewWorld),
    /// The list of saves, with the one highlighted.
    Load {
        choice: usize,
    },
    Help,
}

/// What's picked in the New world box (M2 design §8.3, §16).
#[derive(Debug, Clone, PartialEq, Eq)]
struct NewWorld {
    seed: Draft,
    preset: usize,
    mode: Mode,
    /// Whether `?` is held, explaining the mode.
    explaining: bool,
}

/// The title screen's state.
pub struct Title {
    world: World,
    theme: Theme,
    /// How long the title screen has been open, for the scene.
    shown_for: Duration,
    /// Time owed to the world behind, in ticks of `TICK_EVERY`.
    owed: Duration,
    /// Where the Cursor's light arrives and stays, over its sprite.
    light: Pos,
    screen: Screen,
    choice: usize,
    /// The saves, newest first, with when they were saved, said.
    saves: Vec<(SaveFile, String)>,
    /// The folder they're in, if there is one.
    save_folder: Option<PathBuf>,
    presets: Vec<Preset>,
    /// The seed New world offers next.
    seed: u64,
    refusal: Option<String>,
    /// Draws the help screen, as a world's does.
    help: App,
    /// The terminal's size, as last told, where a click lands.
    size: Size,
}

impl Title {
    /// The title screen on `world`, which the scene wakes, on a terminal
    /// of `screen`, offering `seed` first for a new world. It offers no
    /// saves and only the default preset until told of more.
    pub fn new(world: World, theme: Theme, seed: u64, screen: Size) -> Title {
        let map = world.map();
        let areas = ui::areas(ui::MIN_SIZE);
        let help = App::new(map, theme.clone(), world.seed(), areas);
        Title {
            theme,
            shown_for: Duration::ZERO,
            owed: Duration::ZERO,
            light: light_for(&world, screen),
            screen: Screen::Menu,
            choice: 0,
            saves: Vec::new(),
            save_folder: None,
            presets: vec![Preset::default()],
            seed,
            refusal: None,
            help,
            size: screen,
            world,
        }
    }

    /// The saves Continue and Load offer, newest first, and the time now,
    /// to say how long ago each was saved.
    pub fn set_saves(&mut self, saves: Vec<SaveFile>, now: SystemTime) {
        self.saves = saves
            .into_iter()
            .map(|save| {
                let ago = save
                    .modified
                    .and_then(|when| now.duration_since(when).ok())
                    .map_or_else(|| "saved".into(), ago);
                (save, ago)
            })
            .collect();
        self.choice = 0;
    }

    /// The folder the saves are in, which Load names when it has none to
    /// list.
    pub fn set_save_folder(&mut self, folder: PathBuf) {
        self.save_folder = Some(folder);
    }

    /// The presets New world offers, the default first. With none, it
    /// offers the default alone.
    pub fn set_presets(&mut self, presets: Vec<Preset>) {
        self.presets = if presets.is_empty() {
            vec![Preset::default()]
        } else {
            presets
        };
    }

    /// The game's folder, which the help screen names.
    pub fn set_data_folder(&mut self, folder: PathBuf) {
        self.help.set_data_folder(folder);
    }

    /// The world behind the menu.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Whether the opening scene has ended, and the menu shows.
    pub fn scene_over(&self) -> bool {
        self.shown_for >= SCENE_ENDS
    }

    /// The menu's choices, in order: Continue, first, only when there's a
    /// save.
    pub fn choices(&self) -> Vec<Choice> {
        Choice::ALL[usize::from(self.saves.is_empty())..].to_vec()
    }

    /// Whether keys type, as they do in the New world box, and in the help,
    /// which searches what's typed (#303).
    pub fn typing(&self) -> bool {
        matches!(self.screen, Screen::NewWorld(_) | Screen::Help)
    }

    /// Says on the status line why something the player asked for didn't
    /// happen, such as a save that wouldn't load.
    pub fn refuse(&mut self, why: String) {
        self.refusal = Some(why);
    }

    /// Lets `elapsed` pass: the scene plays on, and the world runs at 4×.
    pub fn animate(&mut self, elapsed: Duration) {
        self.shown_for = self.shown_for.saturating_add(elapsed);
        self.owed += elapsed;
        let mut ticks = 0;
        while self.owed >= TICK_EVERY && ticks < MOST_TICKS_AT_ONCE {
            self.world.step();
            self.owed -= TICK_EVERY;
            ticks += 1;
        }
        // What a long frame left owing is dropped rather than caught up.
        if ticks == MOST_TICKS_AT_ONCE {
            self.owed = Duration::ZERO;
        }
    }

    /// The terminal is now `size`, so a click finds the menu where it's
    /// drawn. The Cursor stays where it is unless the new size would hide
    /// it, or put a box over it (M2 design §8.1).
    pub fn resize(&mut self, size: Size) {
        self.size = size;
        let (columns, rows) = clear_of_boxes(&self.world, size);
        let hidden = !columns.contains(&self.light.x) || !rows.contains(&self.light.y);
        // Under "Terminal too small" there's no Cursor drawn to hide.
        if hidden && !ui::too_small(size) {
            self.light = light_for(&self.world, size);
        }
    }

    /// What `action` does on the title screen.
    pub fn apply(&mut self, action: Action) -> TitleFlow {
        if action == Action::Quit {
            return TitleFlow::Quit;
        }
        // A terminal too small, as last told, shows only that it is, so,
        // as a world's does, it takes no clicks or turns of the wheel:
        // they'd land on a menu that isn't drawn (M2 design §8.2).
        if ui::too_small(self.size) && action.is_pointer() {
            return TitleFlow::Stay;
        }
        if !self.scene_over() {
            // Any key or click skips the scene, and does nothing else.
            if !matches!(action, Action::Point(_) | Action::Release { .. }) {
                self.shown_for = SCENE_ENDS;
            }
            return TitleFlow::Stay;
        }
        if matches!(action, Action::Point(_) | Action::Release { .. }) {
            return TitleFlow::Stay;
        }
        self.refusal = None;
        match self.screen.clone() {
            Screen::Menu => self.apply_in_menu(action),
            Screen::QuitPrompt => {
                if action == Action::Confirm {
                    return TitleFlow::Quit;
                }
                // Any other key cancels, `Esc` included (design v33 §6.6),
                // and the mouse doesn't answer it, as in a world.
                if !action.is_pointer() {
                    self.screen = Screen::Menu;
                }
                TitleFlow::Stay
            }
            Screen::NewWorld(new) => self.apply_in_new_world(action, new),
            Screen::Load { choice } => self.apply_in_load(action, choice),
            // The help works as a world's does, and closing it goes back
            // to the menu (#303).
            Screen::Help => {
                self.help.apply(action, &self.world);
                if self.help.screen() != app::Screen::Help {
                    self.screen = Screen::Menu;
                }
                TitleFlow::Stay
            }
        }
    }

    /// Opens the help, at the world's page, as a world's opens there.
    fn open_help(&mut self) {
        if self.help.screen() != app::Screen::Help {
            self.help.apply(Action::Help, &self.world);
        }
        self.screen = Screen::Help;
    }

    fn apply_in_menu(&mut self, action: Action) -> TitleFlow {
        let choices = self.choices();
        match action {
            Action::Pick(n) if (1..=choices.len()).contains(&usize::from(n)) => {
                self.choice = usize::from(n) - 1;
                self.choose(choices[self.choice])
            }
            Action::Enter => self.choose(choices[self.choice]),
            Action::Scroll { dy, .. } => {
                self.choice = step(self.choice, dy, choices.len());
                TitleFlow::Stay
            }
            Action::Wheel { notches, .. } => {
                self.choice = step(self.choice, notches, choices.len());
                TitleFlow::Stay
            }
            Action::Click { at, .. } => {
                let row = menu_rows(self.size, choices.len()).find(|&(_, y)| y == at.y);
                match row {
                    Some((index, _)) if menu_box(self.size, choices.len()).contains(at) => {
                        self.choice = index;
                        self.choose(choices[index])
                    }
                    _ => TitleFlow::Stay,
                }
            }
            Action::Back => {
                self.screen = Screen::QuitPrompt;
                TitleFlow::Stay
            }
            Action::Help => {
                self.open_help();
                TitleFlow::Stay
            }
            _ => TitleFlow::Stay,
        }
    }

    fn choose(&mut self, choice: Choice) -> TitleFlow {
        match choice {
            Choice::Continue => return TitleFlow::Load(self.saves[0].0.clone()),
            Choice::NewWorld => {
                self.screen = Screen::NewWorld(NewWorld {
                    seed: Draft::offered(self.seed.to_string()),
                    preset: 0,
                    mode: Mode::Story,
                    explaining: false,
                })
            }
            Choice::Load => self.screen = Screen::Load { choice: 0 },
            Choice::Help => self.open_help(),
            Choice::Quit => return TitleFlow::Quit,
        }
        TitleFlow::Stay
    }

    /// What an action does in the New world box: digits type the seed,
    /// `↑` `↓` pick a preset and `←` `→` a mode, and holding `?` explains
    /// the mode picked (M2 design §16). The explanation goes when `?` is
    /// let go of, or, where the terminal never says so, when a key other
    /// than the arrows that pick a mode is pressed.
    fn apply_in_new_world(&mut self, action: Action, mut new: NewWorld) -> TitleFlow {
        let picks_a_mode = matches!(action, Action::Scroll { dx, .. } if dx != 0);
        new.explaining = match action {
            Action::Type('?') => true,
            _ if picks_a_mode => new.explaining,
            _ => false,
        };
        match action {
            Action::Type(c) if c.is_ascii_digit() => new.seed.type_char(c, MAX_SEED_DIGITS),
            Action::Erase => new.seed.erase(),
            Action::AnotherName => {
                self.seed = next_seed(self.seed);
                new.seed = Draft::offered(self.seed.to_string());
            }
            Action::Scroll { dx, .. } if dx != 0 => {
                let picked = MODES.iter().position(|&(mode, ..)| mode == new.mode);
                new.mode = MODES[step(picked.unwrap_or(0), dx, MODES.len())].0;
            }
            Action::Scroll { dy, .. } => new.preset = step(new.preset, dy, self.presets.len()),
            Action::Enter => {
                if new.seed.text().is_empty() {
                    self.refuse("Type a seed, or Tab for a random one".into());
                } else {
                    match new.seed.text().parse() {
                        Ok(seed) => {
                            return TitleFlow::New {
                                seed,
                                preset: self.presets[new.preset].path.clone(),
                                mode: new.mode,
                            };
                        }
                        Err(_) => {
                            self.refuse(format!("A seed is a whole number from 0 to {}", u64::MAX))
                        }
                    }
                }
            }
            Action::Back => {
                self.screen = Screen::Menu;
                return TitleFlow::Stay;
            }
            _ => {}
        }
        self.screen = Screen::NewWorld(new);
        TitleFlow::Stay
    }

    /// What an action does in the list of saves, as in the one `Ctrl+O`
    /// opens in a world (M2 design §8.2): a number or `Enter` loads a save,
    /// the arrow keys or the wheel move the highlight, a click loads the
    /// save under it or, off the list, closes it, and `Esc` closes it.
    fn apply_in_load(&mut self, action: Action, choice: usize) -> TitleFlow {
        let count = self.saves.len();
        let moved = match action {
            Action::Pick(n) if (1..=count).contains(&usize::from(n)) => {
                return TitleFlow::Load(self.saves[usize::from(n) - 1].0.clone());
            }
            Action::Enter if count > 0 => return TitleFlow::Load(self.saves[choice].0.clone()),
            Action::Scroll { dy, .. } if count > 0 => step(choice, dy, count),
            // A row a notch, as a world's lists move.
            Action::Wheel { notches, .. } if count > 0 => highlight_moved(choice, notches, count),
            Action::Click { at, .. } => {
                let area = self.load_box(self.size);
                if !area.contains(at) {
                    self.screen = Screen::Menu;
                    return TitleFlow::Stay;
                }
                let row = usize::from(at.y.wrapping_sub(area.y + 1));
                let index = self.first_save_shown(choice) + row;
                if row < self.load_rows() && index < count {
                    return TitleFlow::Load(self.saves[index].0.clone());
                }
                choice
            }
            Action::Back => {
                self.screen = Screen::Menu;
                return TitleFlow::Stay;
            }
            _ => choice,
        };
        self.screen = Screen::Load { choice: moved };
        TitleFlow::Stay
    }

    /// How many rows the list of saves shows: a row for each save, up to
    /// `LOAD_ROWS`, or one to say there are none.
    fn load_rows(&self) -> usize {
        self.saves.len().clamp(1, LOAD_ROWS)
    }

    /// The first save the list shows, scrolled to keep the one highlighted,
    /// `choice`, in view.
    fn first_save_shown(&self, choice: usize) -> usize {
        choice.saturating_sub(self.load_rows() - 1)
    }

    /// What the list of saves says with none to list, as a world's does.
    fn no_saves(&self) -> String {
        saves::none_listed(self.save_folder.as_deref())
    }

    /// The list of saves' box on a screen of `screen`: as wide as the
    /// other boxes, or wider to say where there are no saves, as a world's
    /// list is as wide as what it says.
    fn load_box(&self, screen: Size) -> Rect {
        let none = if self.saves.is_empty() {
            // A space before it, and the border either side.
            self.no_saves().chars().count() + 3
        } else {
            0
        };
        let width = u16::try_from(none).unwrap_or(u16::MAX).max(BOX_WIDTH);
        centred_box(screen, width, self.load_rows() as u16 + LOAD_FRAME_ROWS)
    }
}

/// The menu's box, with `choices` choices, in the middle of a screen of
/// `screen`.
fn menu_box(screen: Size, choices: usize) -> Rect {
    centred_box(screen, BOX_WIDTH, choices as u16 + MENU_FRAME_ROWS)
}

/// Where the Cursor's light arrives and stays (M2 design §8.1): over the
/// tile nearest the middle of the rows above the title's boxes that a
/// sprite could stand on and nothing's on, or that middle when there's no
/// such tile.
fn light_for(world: &World, screen: Size) -> Pos {
    let map = world.map();
    let (columns, rows) = clear_of_boxes(world, screen);
    let middle = Pos {
        x: (columns.start + columns.end) / 2,
        y: (rows.start + rows.end.saturating_sub(1)) / 2,
    };
    let free = |pos: Pos| {
        let walkable = world.data().terrain(map.terrain(pos)).step_cost().is_some();
        walkable && world.sprite_at(pos).is_none() && world.object_at(pos).is_none()
    };
    rows.flat_map(|y| columns.clone().map(move |x| Pos { x, y }))
        .filter(|&pos| free(pos))
        // A cell is about twice as tall as it is wide, so rows count double.
        .min_by_key(|pos| pos.x.abs_diff(middle.x) + 2 * pos.y.abs_diff(middle.y))
        .unwrap_or(middle)
}

/// The columns and rows the Cursor's sprite may be in on a terminal of
/// `screen`, with its frame on the map shown and above the tallest box the
/// title draws, however many choices, presets or saves it holds, so none
/// covers it. Only the help screen, over everything, does (M2 design
/// §8.1).
fn clear_of_boxes(world: &World, screen: Size) -> (Range<u16>, Range<u16>) {
    let map = world.map();
    let width = map.width().min(screen.width);
    let height = map.height().min(screen.height);
    let menu = Choice::ALL.len() as u16 + MENU_FRAME_ROWS;
    let new_world = PRESET_ROWS as u16 + NEW_WORLD_FRAME_ROWS;
    let load = LOAD_ROWS as u16 + LOAD_FRAME_ROWS;
    let top = centred_box(screen, BOX_WIDTH, menu.max(new_world).max(load)).y;
    // The frame takes a tile each side of its sprite.
    (
        1..width.saturating_sub(1),
        1..top.min(height).saturating_sub(1),
    )
}

/// A box `width` wide, or as wide as the screen, and `height` tall in the
/// middle of a screen of `screen`, above its status line. The screen is
/// taken to be at least `ui::MIN_SIZE`, as nothing smaller draws the boxes.
fn centred_box(screen: Size, width: u16, height: u16) -> Rect {
    let width = width.min(screen.width.max(ui::MIN_SIZE.width));
    let screen_height = screen.height.max(ui::MIN_SIZE.height);
    Rect::new(
        screen.width.max(ui::MIN_SIZE.width).saturating_sub(width) / 2,
        screen_height.saturating_sub(1).saturating_sub(height) / 2,
        width,
        height,
    )
}

/// Each of `choices` choices' index and row on a screen of `screen`.
fn menu_rows(screen: Size, choices: usize) -> impl Iterator<Item = (usize, u16)> {
    let top = menu_box(screen, choices).y + ABOVE_CHOICES;
    (0..choices).map(move |index| (index, top + index as u16))
}

/// The style of a highlighted item, or of one that isn't.
fn chosen_style(chosen: bool) -> Style {
    if chosen {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}

/// Moves `index` by one, the way `by` points, within `count` items.
fn step(index: usize, by: i32, count: usize) -> usize {
    highlight_moved(index, by.signum(), count)
}

/// Another seed to offer, from the last one: a splitmix64 step, so each
/// `Tab` gives a seed that looks unrelated to the one before.
fn next_seed(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// How long ago `elapsed` was, said: "2 hours ago".
fn ago(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    let (n, unit) = match secs {
        0..60 => return "just now".into(),
        60..3600 => (secs / 60, "minute"),
        3600..86400 => (secs / 3600, "hour"),
        _ => (secs / 86400, "day"),
    };
    let s = if n == 1 { "" } else { "s" };
    format!("{n} {unit}{s} ago")
}

/// Draws the title screen.
pub fn render(frame: &mut Frame, title: &Title) {
    let area = frame.area();
    let buf = frame.buffer_mut();
    if ui::too_small(area.as_size()) {
        let quitting = title.screen == Screen::QuitPrompt;
        let question = quitting.then(|| QUIT_PROMPT.to_string());
        return ui::render_too_small_with(buf, area, question.into_iter().collect());
    }
    draw_world(buf, area, title);
    if !title.scene_over() {
        draw_scene(buf, area, title);
    }
    draw_cursor(buf, area, title);
    // New world and Load take the menu's place.
    let in_a_box = matches!(title.screen, Screen::NewWorld(_) | Screen::Load { .. });
    if title.shown_for >= LIGHT_FILLS && !in_a_box {
        draw_menu(buf, title);
    }
    if title.scene_over() {
        match &title.screen {
            Screen::NewWorld(new) => draw_new_world(buf, title, new),
            Screen::Load { choice } => draw_load(buf, title, *choice),
            Screen::Help => draw_help(buf, area, title),
            Screen::Menu | Screen::QuitPrompt => {}
        }
        draw_status_line(buf, area, title);
    }
}

/// The world, from its top-left corner, as the map view draws it.
fn draw_world(buf: &mut Buffer, area: Rect, title: &Title) {
    let world = &title.world;
    let map = world.map();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let pos = Pos { x, y };
            let cell = &mut buf[(x, y)];
            if x >= map.width() || y >= map.height() {
                cell.set_char(' ').set_style(Style::reset());
                continue;
            }
            let glyph = if world.sprite_at(pos).is_some() {
                title.theme.glyph(SemanticTile::Sprite)
            } else if let Some(object) = world.object_at(pos) {
                title
                    .theme
                    .object_glyph(object.type_name(), object.visual_state())
            } else {
                title.theme.glyph(SemanticTile::Terrain(map.terrain(pos)))
            };
            cell.set_char(glyph.symbol).set_style(style_of(glyph));
        }
    }
}

/// How `glyph` is drawn: in its colour, and bold or in reverse video if
/// it says so.
fn style_of(glyph: Glyph) -> Style {
    let mut style = Style::default().fg(glyph.fg);
    if glyph.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if glyph.reversed {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

/// The scene over the world (M2 design §8.1): dark beyond the light's
/// reach, its edge `░`.
fn draw_scene(buf: &mut Buffer, area: Rect, title: &Title) {
    let t = title.shown_for;
    let light = title.light;
    // How far the light reaches, in cells across; rows count double, as a
    // cell is about twice as tall as it's wide.
    let fills = f64::from(area.width) / 2.0 + f64::from(area.height) + 2.0;
    let spread =
        t.saturating_sub(LIGHT_ARRIVES).as_secs_f64() / (LIGHT_FILLS - LIGHT_ARRIVES).as_secs_f64();
    let reach = 1.2 + spread.min(1.0) * fills;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let dx = (f64::from(x) - f64::from(light.x)) / 2.0;
            let dy = f64::from(y) - f64::from(light.y);
            let distance = (dx * dx + dy * dy).sqrt();
            let cell = &mut buf[(x, y)];
            if distance > reach + 1.0 {
                cell.set_char(' ').set_style(Style::reset());
            } else if distance > reach {
                cell.set_char('░')
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }
}

/// The Cursor's frame of light, brightening as it arrives, with its sprite
/// inside, still and drawn as a sprite is, for as long as the title screen
/// is open (M2 design §8.1). It's the scene's own sprite, drawn over the
/// world as the Cursor is over a map, so it needs nothing, and the Cursor
/// leads none of the world's sprites.
fn draw_cursor(buf: &mut Buffer, area: Rect, title: &Title) {
    let light = title.light;
    if area.contains(Position::new(light.x, light.y)) {
        let sprite = title.theme.glyph(SemanticTile::Sprite);
        buf[(light.x, light.y)]
            .set_char(sprite.symbol)
            .set_style(style_of(sprite));
    }
    let brightness = title.shown_for.as_secs_f64() / LIGHT_ARRIVES.as_secs_f64();
    let frame = title.theme.visible_frame();
    let style = Style::default().fg(fade(brightness));
    let sides = [
        (Some(light.x), light.y.checked_sub(1), frame.up),
        (Some(light.x), light.y.checked_add(1), frame.down),
        (light.x.checked_sub(1), Some(light.y), frame.left),
        (light.x.checked_add(1), Some(light.y), frame.right),
    ];
    for (x, y, symbol) in sides {
        if let (Some(x), Some(y)) = (x, y)
            && area.contains(Position::new(x, y))
        {
            buf[(x, y)].set_char(symbol).set_style(style);
        }
    }
}

/// A colour from dark to white as `brightness` goes from 0 to 1.
fn fade(brightness: f64) -> Color {
    if brightness < 1.0 / 3.0 {
        Color::DarkGray
    } else if brightness < 2.0 / 3.0 {
        Color::Gray
    } else {
        Color::White
    }
}

/// The menu's box over the world: the title lettering, fading in as the
/// scene ends, then the choices once it has.
fn draw_menu(buf: &mut Buffer, title: &Title) {
    let choices = title.choices();
    let screen = buf.area.as_size();
    let area = menu_box(screen, choices.len()).intersection(buf.area);
    double_box(buf, area, "");
    let brightness = title.shown_for.saturating_sub(LIGHT_FILLS).as_secs_f64()
        / (SCENE_ENDS - LIGHT_FILLS).as_secs_f64();
    let style = Style::default()
        .fg(fade(brightness))
        .add_modifier(Modifier::BOLD);
    for (row, line) in LETTERING.iter().enumerate() {
        // Under the border and a blank row.
        centre(buf, area, area.y + 2 + row as u16, line, style);
    }
    if !title.scene_over() {
        return;
    }
    let x = area.x + CHOICES_INDENT;
    for (index, y) in menu_rows(screen, choices.len()) {
        let choice = choices[index];
        let chosen = index == title.choice && title.screen == Screen::Menu;
        let mark = if chosen { '►' } else { ' ' };
        let line = format!("{mark} {} {:<10}", index + 1, choice.label());
        buf.set_string(x, y, &line, chosen_style(chosen));
        if choice == Choice::Continue {
            let (save, ago) = &title.saves[0];
            let said_x = x + CONTINUE_SAID_AT;
            let room = usize::from(area.right().saturating_sub(said_x + 1));
            let said = format!("{} · {ago}", shown(&save.name));
            buf.set_stringn(said_x, y, said, room, Style::default().fg(Color::Gray));
        }
    }
}

/// "TERRA SPRITES" in block letters, two rows high.
const LETTERING: [&str; 2] = [
    "▀█▀ █▀▀ █▀█ █▀█ ▄▀█   █▀ █▀█ █▀█ █ ▀█▀ █▀▀ █▀",
    " █  ██▄ █▀▄ █▀▄ █▀█   ▄█ █▀▀ █▀▄ █  █  ██▄ ▄█",
];

/// The New world box (M2 design §8.3, §16), over the menu.
fn draw_new_world(buf: &mut Buffer, title: &Title, new: &NewWorld) {
    let explained = explained(new.mode);
    // The presets, or the explanation in their place: room for whichever
    // is longer, so the box stays put when `?` is held.
    let rows = title.presets.len().min(PRESET_ROWS).max(explained.len()) as u16;
    let height = rows + NEW_WORLD_FRAME_ROWS;
    let area = centred_box(buf.area.as_size(), BOX_WIDTH, height).intersection(buf.area);
    let inner = double_box(buf, area, " New world ");
    let room = |x: u16| usize::from(inner.right().saturating_sub(x));
    // The seed offered shows highlighted, as the first key replaces it;
    // one being typed shows where the next digit goes.
    let (shown, style) = if new.seed.typed() {
        (format!("{}_", new.seed.text()), Style::default())
    } else {
        (new.seed.text().to_string(), chosen_style(true))
    };
    buf.set_string(inner.x, inner.y, " Seed    ", Style::default());
    buf.set_string(inner.x + LABEL_WIDTH, inner.y, shown, style);

    // The modes, the one picked in brackets, and what it means in short.
    let y = inner.y + 1;
    buf.set_string(inner.x, y, " Mode    ", Style::default());
    let mut x = inner.x + LABEL_WIDTH;
    for (mode, name) in MODES {
        let picked = mode == new.mode;
        let shown = if picked {
            format!("[{name}]")
        } else {
            format!(" {name} ")
        };
        buf.set_string(x, y, &shown, chosen_style(picked));
        x += shown.chars().count() as u16 + 1;
        if picked {
            let said_x = inner.x + LABEL_WIDTH + 15;
            let gray = Style::default().fg(Color::Gray);
            buf.set_stringn(said_x, y, in_short(mode), room(said_x), gray);
        }
    }

    let top = inner.y + 3;
    if new.explaining {
        for (row, line) in explained.iter().enumerate() {
            let y = top + row as u16;
            buf.set_stringn(inner.x + 1, y, line, room(inner.x + 1), Style::default());
        }
    } else {
        draw_presets(buf, title, new.preset, inner, top, rows);
    }

    let hints = [
        " tab another seed  ←→ mode  ↑↓ preset",
        " hold ? what the mode means  enter start  esc back",
    ];
    for (row, hints) in hints.iter().enumerate() {
        let y = inner.bottom().saturating_sub(2) + row as u16;
        let gray = Style::default().fg(Color::Gray);
        buf.set_stringn(inner.x, y, hints, room(inner.x), gray);
    }
}

/// New world's presets, from row `top` of the box's `inner` area, `rows`
/// of them at most, scrolled to keep the one `picked` in view.
fn draw_presets(buf: &mut Buffer, title: &Title, picked: usize, inner: Rect, top: u16, rows: u16) {
    let rows = rows.min(title.presets.len() as u16).max(1);
    let first = picked.saturating_sub(usize::from(rows) - 1);
    for (row, (index, choice)) in title
        .presets
        .iter()
        .enumerate()
        .skip(first)
        .take(usize::from(rows))
        .enumerate()
    {
        let label = if row == 0 { " Preset  " } else { "         " };
        let chosen = index == picked;
        let mark = if chosen { '►' } else { ' ' };
        let y = top + row as u16;
        buf.set_string(inner.x, y, label, Style::default());
        let room = usize::from(inner.width.saturating_sub(LABEL_WIDTH + 1));
        buf.set_stringn(
            inner.x + LABEL_WIDTH,
            y,
            format!("{mark} {}", choice.name),
            room,
            chosen_style(chosen),
        );
    }
}

/// The list of saves to load, newest first, over the menu.
fn draw_load(buf: &mut Buffer, title: &Title, choice: usize) {
    let area = title.load_box(buf.area.as_size()).intersection(buf.area);
    let inner = double_box(buf, area, " Load ");
    if title.saves.is_empty() {
        let none = format!(" {}", title.no_saves());
        let room = usize::from(inner.width);
        buf.set_stringn(inner.x, inner.y, none, room, Style::default());
        return;
    }
    for (row, (index, (save, ago))) in title
        .saves
        .iter()
        .enumerate()
        .skip(title.first_save_shown(choice))
        .take(title.load_rows())
        .enumerate()
    {
        let number = if index < 9 {
            (index + 1).to_string()
        } else {
            " ".into()
        };
        // A name outside CP437 shows as the world's list shows it.
        let line = format!(" {number} {}  ", shown(&save.name));
        let y = inner.y + row as u16;
        let room = usize::from(inner.width);
        buf.set_stringn(inner.x, y, &line, room, chosen_style(index == choice));
        let ago_x = inner.right().saturating_sub(ago.chars().count() as u16 + 1);
        if ago_x > inner.x + line.chars().count() as u16 {
            buf.set_string(ago_x, y, ago, Style::default().fg(Color::Gray));
        }
    }
}

/// The help screen, as a world shows it, over everything but the status
/// line.
fn draw_help(buf: &mut Buffer, area: Rect, title: &Title) {
    let overlay = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    ui::draw_help(buf, overlay, &title.help, &title.world);
}

/// The status line: a prompt or a refusal, or else the keys, with the
/// game's version at the right.
fn draw_status_line(buf: &mut Buffer, area: Rect, title: &Title) {
    let y = area.bottom() - 1;
    let row = Rect::new(area.x, y, area.width, 1);
    buf.set_style(row, Style::reset());
    for x in row.left()..row.right() {
        buf[(x, y)].set_char(' ');
    }
    let left = if title.screen == Screen::QuitPrompt {
        format!(" {QUIT_PROMPT}")
    } else if let Some(why) = &title.refusal {
        format!(" {why}")
    } else {
        String::new()
    };
    buf.set_stringn(area.x, y, left, usize::from(area.width), Style::default());
    // The menu's hints, and the help's, as a world's status line gives
    // them (#303).
    let context = match title.screen {
        Screen::Menu => Some(Context::TitleMenu),
        Screen::Help => Some(controls::context(&title.help, &title.world)),
        _ => None,
    };
    if let Some(context) = context
        && title.refusal.is_none()
    {
        let hints = ui::all_hints(controls::hints(context), &title.help, &title.world);
        let mut spans = vec![Span::raw(" ")];
        spans.extend(ui::hint_spans(&hints, title.theme.text()));
        buf.set_line(area.x, y, &Line::from(spans), area.width);
    }
    let version = concat!("v", env!("CARGO_PKG_VERSION"), " ");
    let x = area.right().saturating_sub(version.len() as u16);
    buf.set_string(x, y, version, Style::default().fg(Color::DarkGray));
}

/// Blanks `area` and draws a double-lined box round it, with `name` in its
/// top edge: the title screen's boxes, set apart from a world's
/// single-lined overlays. Gives the area inside the box.
fn double_box(buf: &mut Buffer, area: Rect, name: &str) -> Rect {
    if area.width < 2 || area.height < 2 {
        return Rect::default();
    }
    buf.set_style(area, Style::reset());
    let border = Style::default().fg(Color::Gray);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let top = y == area.top();
            let bottom = y == area.bottom() - 1;
            let left = x == area.left();
            let right = x == area.right() - 1;
            let symbol = match (top, bottom, left, right) {
                (true, _, true, _) => '╔',
                (true, _, _, true) => '╗',
                (_, true, true, _) => '╚',
                (_, true, _, true) => '╝',
                (true, _, _, _) | (_, true, _, _) => '═',
                (_, _, true, _) | (_, _, _, true) => '║',
                _ => ' ',
            };
            let style = if symbol == ' ' {
                Style::default()
            } else {
                border
            };
            buf[(x, y)].set_char(symbol).set_style(style);
        }
    }
    if !name.is_empty() {
        buf.set_string(area.x + 2, area.y, name, border);
    }
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width - 2,
        area.height.saturating_sub(2),
    )
}

/// `text` in the middle of `area`'s row `y`.
fn centre(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    let width = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    buf.set_string(x, y, text, style);
}
