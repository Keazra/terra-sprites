//! The title screen (M2 design §8): the screen the game opens on, before the
//! player's world is running. The opening scene plays, then the menu shows
//! over the world it woke, which runs behind it.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier, Style};
use terra_sim::{EntityId, Pos, World};

use crate::app::{App, Draft};
use crate::input::Action;
use crate::saves::SaveFile;
use crate::start::Preset;
use crate::theme::{Emote, SemanticTile, Theme};
use crate::ui;

/// When the Cursor's light has arrived and starts to spread (M2 design
/// §8.1).
const LIGHT_ARRIVES: Duration = Duration::from_millis(1000);
/// When the sprite under the light wakes.
const WAKES: Duration = Duration::from_millis(2500);
/// When the light has filled the screen, the title starts to fade in and
/// the world starts to move.
const LIGHT_FILLS: Duration = Duration::from_millis(4000);
/// How long the woken sprite shows it's pleased.
const PLEASED_UNTIL: Duration = Duration::from_millis(4500);
/// When the scene ends and the menu shows.
const SCENE_ENDS: Duration = Duration::from_millis(5500);
/// How often the world behind the menu ticks: 4×, 5 ticks a second.
const TICK_EVERY: Duration = Duration::from_millis(200);
/// The most ticks the world catches up on at once, after a long frame.
const MOST_TICKS_AT_ONCE: u32 = 5;
/// The menu's box: as wide as the title lettering and a margin.
const BOX_WIDTH: u16 = 56;
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

/// A choice on the title screen's menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Load the newest save; offered only when there is one.
    Continue,
    NewWorld,
    Load,
    Help,
    Quit,
}

impl Choice {
    pub fn label(self) -> &'static str {
        match self {
            Choice::Continue => "Continue",
            Choice::NewWorld => "New world",
            Choice::Load => "Load",
            Choice::Help => "Help",
            Choice::Quit => "Quit",
        }
    }
}

/// What the game does after an action on the title screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleFlow {
    /// The title screen stays.
    Stay,
    Quit,
    /// Start a new world from `seed`, with the preset in the file, or the
    /// default.
    New {
        seed: u64,
        preset: Option<PathBuf>,
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
    NewWorld {
        seed: Draft,
        preset: usize,
    },
    /// The list of saves, with the one highlighted.
    Load {
        choice: usize,
    },
    Help,
}

/// The title screen's state.
pub struct Title {
    world: World,
    theme: Theme,
    /// How long the title screen has been open, for the scene.
    shown_for: Duration,
    /// Time owed to the world behind, in ticks of `TICK_EVERY`.
    owed: Duration,
    /// The sprite the Cursor's light arrives on.
    woken: Option<EntityId>,
    /// Where the light arrives: the woken sprite's tile.
    light: Pos,
    screen: Screen,
    choice: usize,
    /// The saves, newest first, with when they were saved, said.
    saves: Vec<(SaveFile, String)>,
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
    /// The title screen on `world`, which the scene wakes, offering `seed`
    /// first for a new world. It offers no saves and only the default
    /// preset until told of more.
    pub fn new(world: World, theme: Theme, seed: u64) -> Title {
        let map = world.map();
        let centre = Pos {
            x: map.width() / 2,
            y: map.height() / 2,
        };
        let woken = world
            .sprites()
            .min_by_key(|sprite| {
                let pos = sprite.pos();
                // A cell is about twice as tall as it is wide, so rows count double.
                pos.x.abs_diff(centre.x) + 2 * pos.y.abs_diff(centre.y)
            })
            .map(|sprite| (sprite.id(), sprite.pos()));
        let areas = ui::areas(ui::MIN_SIZE, map);
        let help = App::new(map, theme.clone(), world.seed(), areas);
        Title {
            theme,
            shown_for: Duration::ZERO,
            owed: Duration::ZERO,
            woken: woken.map(|(id, _)| id),
            light: woken.map_or(centre, |(_, pos)| pos),
            screen: Screen::Menu,
            choice: 0,
            saves: Vec::new(),
            presets: vec![Preset::default()],
            seed,
            refusal: None,
            help,
            size: Size::new(map.width(), map.height()),
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

    /// The menu's choices, in order.
    pub fn choices(&self) -> Vec<Choice> {
        let continues = (!self.saves.is_empty()).then_some(Choice::Continue);
        continues
            .into_iter()
            .chain([Choice::NewWorld, Choice::Load, Choice::Help, Choice::Quit])
            .collect()
    }

    /// Whether keys type, as they do in the New world box.
    pub fn typing(&self) -> bool {
        matches!(self.screen, Screen::NewWorld { .. })
    }

    /// Says on the status line why something the player asked for didn't
    /// happen, such as a save that wouldn't load.
    pub fn refuse(&mut self, why: String) {
        self.refusal = Some(why);
    }

    /// Lets `elapsed` pass: the scene plays on, and once the light has
    /// filled the screen, the world runs at 4×.
    pub fn animate(&mut self, elapsed: Duration) {
        let before = self.shown_for;
        self.shown_for = self.shown_for.saturating_add(elapsed);
        if self.shown_for <= LIGHT_FILLS {
            return;
        }
        // Only the time since the light filled the screen counts.
        self.owed += self.shown_for - before.max(LIGHT_FILLS);
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
    /// drawn.
    pub fn resize(&mut self, size: Size) {
        self.size = size;
    }

    /// What `action` does on the title screen.
    pub fn apply(&mut self, action: Action) -> TitleFlow {
        if action == Action::Quit {
            return TitleFlow::Quit;
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
                // Any other key cancels, `Esc` included (design v33 §6.6).
                self.screen = Screen::Menu;
                TitleFlow::Stay
            }
            Screen::NewWorld { seed, preset } => self.apply_in_new_world(action, seed, preset),
            Screen::Load { choice } => self.apply_in_load(action, choice),
            Screen::Help => {
                if matches!(action, Action::Back | Action::Help) {
                    self.screen = Screen::Menu;
                }
                TitleFlow::Stay
            }
        }
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
                self.screen = Screen::Help;
                TitleFlow::Stay
            }
            _ => TitleFlow::Stay,
        }
    }

    fn choose(&mut self, choice: Choice) -> TitleFlow {
        match choice {
            Choice::Continue => return TitleFlow::Load(self.saves[0].0.clone()),
            Choice::NewWorld => {
                self.screen = Screen::NewWorld {
                    seed: Draft::offered(self.seed.to_string()),
                    preset: 0,
                }
            }
            Choice::Load => self.screen = Screen::Load { choice: 0 },
            Choice::Help => self.screen = Screen::Help,
            Choice::Quit => return TitleFlow::Quit,
        }
        TitleFlow::Stay
    }

    fn apply_in_new_world(&mut self, action: Action, mut seed: Draft, preset: usize) -> TitleFlow {
        let mut preset = preset;
        match action {
            Action::Type(c) if c.is_ascii_digit() => seed.type_char(c, MAX_SEED_DIGITS),
            Action::Erase => seed.erase(),
            Action::AnotherName => {
                self.seed = next_seed(self.seed);
                seed = Draft::offered(self.seed.to_string());
            }
            Action::Scroll { dy, .. } => preset = step(preset, dy, self.presets.len()),
            Action::Enter => {
                if seed.text().is_empty() {
                    self.refuse("Type a seed, or Tab for a random one".into());
                } else {
                    match seed.text().parse() {
                        Ok(seed) => {
                            return TitleFlow::New {
                                seed,
                                preset: self.presets[preset].path.clone(),
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
        self.screen = Screen::NewWorld { seed, preset };
        TitleFlow::Stay
    }

    fn apply_in_load(&mut self, action: Action, choice: usize) -> TitleFlow {
        let count = self.saves.len();
        match action {
            Action::Pick(n) if (1..=count).contains(&usize::from(n)) => {
                return TitleFlow::Load(self.saves[usize::from(n) - 1].0.clone());
            }
            Action::Enter if count > 0 => return TitleFlow::Load(self.saves[choice].0.clone()),
            Action::Scroll { dy, .. } if count > 0 => {
                self.screen = Screen::Load {
                    choice: step(choice, dy, count),
                };
            }
            Action::Back => self.screen = Screen::Menu,
            _ => {}
        }
        TitleFlow::Stay
    }
}

/// The menu's box, with `choices` choices, in the middle of a screen of
/// `screen`.
fn menu_box(screen: Size, choices: usize) -> Rect {
    centred_box(screen, choices as u16 + MENU_FRAME_ROWS)
}

/// A box `BOX_WIDTH` wide and `height` tall in the middle of a screen of
/// `screen`, above its status line. The screen is taken to be at least
/// `ui::MIN_SIZE`, as nothing smaller draws the boxes.
fn centred_box(screen: Size, height: u16) -> Rect {
    let width = BOX_WIDTH.min(screen.width.max(ui::MIN_SIZE.width));
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
    let moved = index as i64 + i64::from(by.signum());
    moved.clamp(0, count.saturating_sub(1) as i64) as usize
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
    if area.width < ui::MIN_SIZE.width || area.height < ui::MIN_SIZE.height {
        let quitting = title.screen == Screen::QuitPrompt;
        let question = quitting.then(|| QUIT_PROMPT.to_string());
        return ui::render_too_small_with(buf, area, question.into_iter().collect());
    }
    draw_world(buf, area, title);
    if !title.scene_over() {
        draw_scene(buf, area, title);
    }
    // New world and Load take the menu's place.
    let in_a_box = matches!(title.screen, Screen::NewWorld { .. } | Screen::Load { .. });
    if title.shown_for >= LIGHT_FILLS && !in_a_box {
        draw_menu(buf, title);
    }
    if title.scene_over() {
        match &title.screen {
            Screen::NewWorld { seed, preset } => draw_new_world(buf, title, seed, *preset),
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
            let mut style = Style::default().fg(glyph.fg);
            if glyph.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if glyph.reversed {
                style = style.add_modifier(Modifier::REVERSED);
            }
            cell.set_char(glyph.symbol).set_style(style);
        }
    }
}

/// The scene over the world (M2 design §8.1): dark beyond the light's
/// reach, the sprite under the light asleep, then pleased, and the Cursor's
/// frame of light brightening as it arrives.
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
    let woken = title.woken.and_then(|id| title.world.sprite(id));
    if let Some(sprite) = woken.filter(|_| t < PLEASED_UNTIL) {
        let emote = if t < WAKES {
            Emote::Resting
        } else {
            Emote::Pleased
        };
        let glyph = title.theme.glyph(SemanticTile::Emote(emote));
        let pos = sprite.pos();
        if area.contains(Position::new(pos.x, pos.y)) {
            buf[(pos.x, pos.y)]
                .set_char(glyph.symbol)
                .set_style(Style::default().fg(glyph.fg));
        }
    }
    let frame = title.theme.visible_frame();
    let style = Style::default().fg(fade(t.as_secs_f64() / LIGHT_ARRIVES.as_secs_f64()));
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
            let said = format!("{} · {ago}", save.name);
            buf.set_stringn(said_x, y, said, room, Style::default().fg(Color::Gray));
        }
    }
}

/// "TERRA SPRITES" in block letters, two rows high.
const LETTERING: [&str; 2] = [
    "▀█▀ █▀▀ █▀█ █▀█ ▄▀█   █▀ █▀█ █▀█ █ ▀█▀ █▀▀ █▀",
    " █  ██▄ █▀▄ █▀▄ █▀█   ▄█ █▀▀ █▀▄ █  █  ██▄ ▄█",
];

/// The New world box (M2 design §8.3), over the menu.
fn draw_new_world(buf: &mut Buffer, title: &Title, seed: &Draft, preset: usize) {
    let rows = title.presets.len().min(8) as u16;
    // The border, the seed, a blank row, the presets, a blank row, the
    // hints and the border.
    let area = centred_box(buf.area.as_size(), rows + 6).intersection(buf.area);
    let inner = double_box(buf, area, " New world ");
    // The seed offered shows highlighted, as the first key replaces it;
    // one being typed shows where the next digit goes.
    let (shown, style) = if seed.typed() {
        (format!("{}_", seed.text()), Style::default())
    } else {
        (seed.text().to_string(), chosen_style(true))
    };
    buf.set_string(inner.x, inner.y, " Seed    ", Style::default());
    buf.set_string(inner.x + LABEL_WIDTH, inner.y, shown, style);
    let first = preset.saturating_sub(usize::from(rows) - 1);
    for (row, (index, choice)) in title
        .presets
        .iter()
        .enumerate()
        .skip(first)
        .take(usize::from(rows))
        .enumerate()
    {
        let label = if row == 0 { " Preset  " } else { "         " };
        let chosen = index == preset;
        let mark = if chosen { '►' } else { ' ' };
        let y = inner.y + 2 + row as u16;
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
    let hints = " tab another seed  ↑↓ preset  enter start  esc back";
    buf.set_stringn(
        inner.x,
        inner.bottom().saturating_sub(1),
        hints,
        usize::from(inner.width),
        Style::default().fg(Color::Gray),
    );
}

/// The list of saves to load, newest first, over the menu.
fn draw_load(buf: &mut Buffer, title: &Title, choice: usize) {
    let rows = title.saves.len().clamp(1, 12) as u16;
    let area = centred_box(buf.area.as_size(), rows + 2).intersection(buf.area);
    let inner = double_box(buf, area, " Load ");
    if title.saves.is_empty() {
        buf.set_string(inner.x, inner.y, " No saves yet", Style::default());
        return;
    }
    let first = choice.saturating_sub(usize::from(rows) - 1);
    for (row, (index, (save, ago))) in title
        .saves
        .iter()
        .enumerate()
        .skip(first)
        .take(usize::from(rows))
        .enumerate()
    {
        let number = if index < 9 {
            (index + 1).to_string()
        } else {
            " ".into()
        };
        let line = format!(" {number} {}  ", save.name);
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
    let inner = ui::clear_box(buf, overlay, " Help ", " esc close ");
    let lines = crate::help::lines(&title.help, &title.world, usize::from(inner.width));
    for (row, line) in (inner.y..inner.bottom()).zip(&lines) {
        buf.set_line(inner.x, row, line, inner.width);
    }
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
        match title.screen {
            Screen::Menu => " ↑↓ choose  enter go  esc quit".into(),
            _ => String::new(),
        }
    };
    buf.set_stringn(area.x, y, left, usize::from(area.width), Style::default());
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
