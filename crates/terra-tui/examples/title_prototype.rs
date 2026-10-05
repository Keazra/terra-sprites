//! PROTOTYPE, throwaway (Slice 21, #139): four title screens to try in the
//! terminal and pick from. Not the real title screen, and not for merging:
//! nothing here is tested, and the menu only says what it would do.
//!
//! Run it: `cargo run --release -p terra-tui --example title_prototype`
//!
//! - `Tab` / `Shift+Tab`: the next or previous title screen (A to D)
//! - `↑↓`, `1`-`5`, `Enter`: the menu (in D, the arrows move the Cursor)
//! - `c`: pretend there is a save, or that there isn't (Continue comes and goes)
//! - `d`: dim the world behind the menu
//! - `+` / `-`: how fast the world behind runs
//! - `r`: play the opening scene again (C)
//! - `Esc` or `q`: quit

use std::io;
use std::time::{Duration, Instant, SystemTime};

use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use terra_sim::{DataPack, Pos, World, WorldConfig};
use terra_tui::theme::{Emote, Glyph, SemanticTile, Theme};
use terra_tui::{files, saves};

const FRAME: Duration = Duration::from_millis(33);
/// Speeds for the world behind, in ticks a second: 1×, 2×, 4×, 8×, 16×.
const SPEEDS: [(f64, &str); 5] = [
    (1.25, "1×"),
    (2.5, "2×"),
    (5.0, "4×"),
    (10.0, "8×"),
    (20.0, "16×"),
];
const VARIANTS: [&str; 4] = [
    "A: still picture",
    "B: live terrarium",
    "C: opening scene",
    "D: terrarium menu",
];
/// How long the opening scene runs, in seconds.
const OPENING: f64 = 6.0;

#[derive(Clone, Copy, PartialEq)]
enum Item {
    Continue,
    NewWorld,
    Load,
    Help,
    Quit,
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::Continue => "Continue",
            Item::NewWorld => "New world",
            Item::Load => "Load",
            Item::Help => "Help",
            Item::Quit => "Quit",
        }
    }
    /// What it is inside the terrarium, in D.
    fn thing(self) -> (&'static str, Color) {
        match self {
            Item::Continue => ("z☺z", Color::White),
            Item::NewWorld => ("'", Color::LightGreen),
            Item::Load => ("≡", Color::Yellow),
            Item::Help => ("?", Color::LightCyan),
            Item::Quit => ("■", Color::Gray),
        }
    }
    /// Where it sits in D, as a fraction of the screen.
    fn spot(self) -> (f32, f32) {
        match self {
            Item::Continue => (0.28, 0.32),
            Item::NewWorld => (0.66, 0.36),
            Item::Load => (0.40, 0.62),
            Item::Help => (0.74, 0.66),
            Item::Quit => (0.16, 0.84),
        }
    }
}

struct Proto {
    variant: usize,
    choice: usize,
    note: Option<(String, Instant)>,
    world: World,
    still: World,
    theme: Theme,
    speed: usize,
    owed: f64,
    dim: bool,
    has_save: bool,
    save_label: String,
    opening: Instant,
    /// The tile the opening scene's light arrives on.
    woken: Option<Pos>,
    screen: (u16, u16),
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let size = terminal.size()?;
    let mut proto = Proto::new(size.width, size.height);
    let mut last = Instant::now();
    loop {
        let size = terminal.size()?;
        proto.screen = (size.width, size.height);
        terminal.draw(|frame| {
            let area = frame.area();
            proto.render(frame.buffer_mut(), area);
        })?;
        let deadline = last + FRAME;
        while event::poll(deadline.saturating_duration_since(Instant::now()))? {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
                && !proto.key(key.code, key.modifiers)
            {
                return Ok(());
            }
        }
        let now = Instant::now();
        proto.advance((now - last).as_secs_f64());
        last = now;
    }
}

impl Proto {
    fn new(cols: u16, rows: u16) -> Proto {
        let data = DataPack::builtin().expect("the built-in data pack loads");
        // The map fills the terminal, so the whole terrarium is in view.
        let config = |w: u16, h: u16| {
            let text = format!(
                "(width: {w}, height: {h}, sprites: 20, \
                 objects: {{\"berry_bush\": 150, \"thornbush\": 40, \"ball\": 6}}, per_tiles: 15360)"
            );
            WorldConfig::from_ron(&text, &data).expect("the prototype's preset is valid")
        };
        let (w, h) = (cols.clamp(32, 1024), rows.clamp(32, 1024));
        let world = World::new(config(w, h), data.clone(), 7);
        let still = World::new(config(64, 32), data.clone(), 21);
        let newest = files::save_folder()
            .map(|folder| saves::list(&folder))
            .and_then(|list| list.into_iter().next());
        let save_label = match &newest {
            Some(save) => format!("{} · {}", save.name, ago(save.modified)),
            None => "autosave-1 · 2 hours ago".into(),
        };
        let mut proto = Proto {
            variant: 1,
            choice: 0,
            note: None,
            world,
            still,
            theme: Theme::cp437(),
            speed: 2,
            owed: 0.0,
            dim: false,
            has_save: newest.is_some(),
            save_label,
            opening: Instant::now(),
            woken: None,
            screen: (cols, rows),
        };
        proto.woken = proto.centre_sprite();
        proto
    }

    fn items(&self) -> Vec<Item> {
        let mut items = vec![Item::NewWorld, Item::Load, Item::Help, Item::Quit];
        if self.has_save {
            items.insert(0, Item::Continue);
        }
        items
    }

    /// Handles a key; false to quit.
    fn key(&mut self, code: KeyCode, mods: KeyModifiers) -> bool {
        // Any key skips the opening scene, and does nothing else.
        if self.variant == 2 && self.opening_t() < OPENING {
            self.opening = Instant::now() - Duration::from_secs_f64(OPENING);
            if !matches!(code, KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc) {
                return true;
            }
        }
        let items = self.items();
        match code {
            KeyCode::Esc | KeyCode::Char('q') => return false,
            KeyCode::Tab if !mods.contains(KeyModifiers::SHIFT) => self.switch(1),
            KeyCode::Tab | KeyCode::BackTab => self.switch(3),
            KeyCode::Char('c') => {
                self.has_save = !self.has_save;
                self.choice = 0;
            }
            KeyCode::Char('d') => self.dim = !self.dim,
            KeyCode::Char('+') | KeyCode::Char('=') => self.speed = (self.speed + 1).min(4),
            KeyCode::Char('-') => self.speed = self.speed.saturating_sub(1),
            KeyCode::Char('r') => self.switch(0),
            KeyCode::Char(n @ '1'..='9') => {
                let index = n as usize - '1' as usize;
                if index < items.len() {
                    self.choice = index;
                    return self.pick(items[index]);
                }
            }
            KeyCode::Enter => return self.pick(items[self.choice.min(items.len() - 1)]),
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right if self.variant == 3 => {
                self.walk(code, &items)
            }
            KeyCode::Up => self.choice = (self.choice + items.len() - 1) % items.len(),
            KeyCode::Down => self.choice = (self.choice + 1) % items.len(),
            _ => {}
        }
        true
    }

    fn switch(&mut self, by: usize) {
        self.variant = (self.variant + by) % VARIANTS.len();
        if self.variant == 2 {
            self.opening = Instant::now();
            self.woken = self.centre_sprite();
        }
    }

    /// In D, moves the Cursor to the nearest choice that way.
    fn walk(&mut self, code: KeyCode, items: &[Item]) {
        let (fx, fy) = items[self.choice.min(items.len() - 1)].spot();
        let (dx, dy) = match code {
            KeyCode::Up => (0.0, -1.0),
            KeyCode::Down => (0.0, 1.0),
            KeyCode::Left => (-1.0, 0.0),
            _ => (1.0, 0.0),
        };
        let best = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let (x, y) = item.spot();
                let (ox, oy) = (x - fx, y - fy);
                let along = ox * dx + oy * dy;
                (along > 0.01).then(|| (i, along + 2.0 * (ox * dy - oy * dx).abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = best {
            self.choice = i;
        }
    }

    fn pick(&mut self, item: Item) -> bool {
        let says = match item {
            Item::Quit => return false,
            Item::Continue => format!("would open {}", self.save_label),
            Item::NewWorld => "would ask for a seed and a preset, then start".into(),
            Item::Load => "would list your saves".into(),
            Item::Help => "would show the help screen".into(),
        };
        self.note = Some((format!("Prototype: {says}"), Instant::now()));
        true
    }

    fn opening_t(&self) -> f64 {
        self.opening.elapsed().as_secs_f64()
    }

    fn advance(&mut self, seconds: f64) {
        // The opening scene holds the world still until the light has spread.
        let moving = match self.variant {
            0 => false,
            2 => self.opening_t() > 4.0,
            _ => true,
        };
        if !moving {
            return;
        }
        self.owed = (self.owed + seconds * SPEEDS[self.speed].0).min(20.0);
        while self.owed >= 1.0 {
            self.world.step();
            self.owed -= 1.0;
        }
    }

    /// The sprite nearest the middle of the screen, which the opening wakes.
    fn centre_sprite(&self) -> Option<Pos> {
        let (cols, rows) = self.screen;
        let (cx, cy) = (i32::from(cols / 2), i32::from(rows.saturating_sub(2) / 2));
        self.world
            .sprites()
            .map(|s| s.pos())
            .min_by_key(|p| (i32::from(p.x) - cx).abs() + 2 * (i32::from(p.y) - cy).abs())
    }

    fn render(&self, buf: &mut Buffer, area: Rect) {
        let body = Rect {
            height: area.height.saturating_sub(2),
            ..area
        };
        match self.variant {
            0 => self.render_still(buf, body),
            1 => {
                self.draw_world(buf, body, self.dim);
                self.menu_box(buf, body, 1.0);
            }
            2 => self.render_opening(buf, body),
            _ => self.render_touch(buf, body),
        }
        let hints = if self.variant == 3 {
            "  ←↑↓→ move the Cursor   Enter touch   Esc quit"
        } else {
            "  ↑↓ or 1-5 choose   Enter go   Esc quit"
        };
        let y = area.bottom().saturating_sub(2);
        clear_row(buf, area, y);
        buf.set_string(area.x, y, hints, Style::default().fg(Color::Gray));
        let version = format!("v{}  ", env!("CARGO_PKG_VERSION"));
        let vx = area.right().saturating_sub(version.chars().count() as u16);
        buf.set_string(vx, y, version, Style::default().fg(Color::DarkGray));
        if let Some((note, at)) = &self.note
            && at.elapsed() < Duration::from_secs(3)
        {
            let x = area.x + area.width.saturating_sub(note.chars().count() as u16) / 2;
            let row = y.saturating_sub(1);
            clear_row(buf, area, row);
            buf.set_string(x, row, note, Style::default().fg(Color::LightYellow));
        }
        let bar = format!(
            " PROTOTYPE  {}  ({} of 4)  Tab next · c save: {} · d dim: {} · +/- speed {} · r replay opening ",
            VARIANTS[self.variant],
            self.variant + 1,
            if self.has_save { "yes" } else { "no" },
            if self.dim { "on" } else { "off" },
            SPEEDS[self.speed].1,
        );
        let y = area.bottom().saturating_sub(1);
        clear_row(buf, area, y);
        buf.set_stringn(
            area.x,
            y,
            bar,
            usize::from(area.width),
            Style::default().fg(Color::Black).bg(Color::LightMagenta),
        );
    }

    /// A: the title, a still scene in a box, and the menu under it.
    fn render_still(&self, buf: &mut Buffer, area: Rect) {
        let title = big("TERRA SPRITES");
        let mut y = area.y + area.height.saturating_sub(24) / 2 + 1;
        for row in &title {
            centred(buf, area, y, row, Style::default().fg(Color::White).bold());
            y += 1;
        }
        let tagline = "a terrarium of small lives";
        centred(buf, area, y, tagline, Style::default().fg(Color::DarkGray));
        y += 2;
        let scene_w = area.width.saturating_sub(8).min(62);
        let scene = Rect::new(area.x + (area.width - scene_w) / 2, y, scene_w, 9);
        draw_box(buf, scene, Color::DarkGray);
        let inner = Rect::new(scene.x + 1, scene.y + 1, scene_w.saturating_sub(2), 7);
        draw_map(
            buf,
            inner,
            &self.still,
            &self.theme,
            Pos { x: 0, y: 12 },
            false,
        );
        y = scene.bottom() + 1;
        let x = area.x + area.width.saturating_sub(40) / 2;
        self.menu_rows(buf, x, y, true);
    }

    /// B's menu, in a box over the world. `fade` from 0 (hidden) to 1.
    fn menu_box(&self, buf: &mut Buffer, area: Rect, fade: f64) {
        let items = self.items();
        let w = 56.min(area.width);
        let save_row = if self.has_save { 2 } else { 0 };
        let h = (items.len() as u16 + 7 + save_row).min(area.height);
        let r = Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        );
        if fade <= 0.0 {
            return;
        }
        clear(buf, r);
        draw_box(buf, r, Color::Gray);
        let colour = fade_colour(fade);
        for (i, row) in big("TERRA SPRITES").iter().enumerate() {
            centred(
                buf,
                r,
                r.y + 2 + i as u16,
                row,
                Style::default().fg(colour).bold(),
            );
        }
        if fade >= 1.0 {
            self.menu_rows(buf, r.x + 6, r.y + 5, false);
        }
    }

    /// The menu's rows from (x, y). With `beside`, Continue's save is
    /// named beside it; otherwise under the menu.
    fn menu_rows(&self, buf: &mut Buffer, x: u16, mut y: u16, beside: bool) {
        let items = self.items();
        for (i, item) in items.iter().enumerate() {
            let chosen = i == self.choice;
            let mark = if chosen { "►" } else { " " };
            let line = format!("{mark} {} {:<10}", i + 1, item.label());
            let style = if chosen {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            buf.set_string(x, y, line, style);
            if beside && *item == Item::Continue {
                let label = format!("  {}", self.save_label);
                buf.set_string(x + 15, y, label, Style::default().fg(Color::DarkGray));
            }
            y += 1;
        }
        if !beside && self.has_save {
            let label = format!("Continue: {}", self.save_label);
            buf.set_string(x, y + 1, label, Style::default().fg(Color::DarkGray));
        }
    }

    /// C: the dark terrarium, the light arriving and spreading, then B.
    fn render_opening(&self, buf: &mut Buffer, area: Rect) {
        let t = self.opening_t();
        if t >= OPENING {
            self.draw_world(buf, area, self.dim);
            return self.menu_box(buf, area, 1.0);
        }
        self.draw_world(buf, area, false);
        let Some(centre) = self.woken else {
            return self.menu_box(buf, area, 1.0);
        };
        // How far the light has spread, in tiles; a tile is half as tall as
        // it is wide on screen, so distances across count half.
        let reach = if t < 1.0 {
            1.2
        } else {
            1.2 + (t - 1.0) / 3.0 * f64::from(area.width.max(area.height))
        };
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let dx = (f64::from(x) - f64::from(centre.x)) / 2.0;
                let dy = f64::from(y) - f64::from(centre.y);
                let d = (dx * dx + dy * dy).sqrt();
                if d > reach + 1.0 {
                    buf[(x, y)].set_char(' ').set_style(Style::reset());
                } else if d > reach {
                    buf[(x, y)]
                        .set_char('░')
                        .set_style(Style::default().fg(Color::DarkGray));
                }
            }
        }
        // The sprite under the light: asleep, then woken and pleased.
        let (cx, cy) = (centre.x, centre.y);
        if t < 4.5 && area.contains((cx, cy).into()) {
            let emote = if t < 2.5 {
                Emote::Resting
            } else {
                Emote::Pleased
            };
            let glyph = self.theme.glyph(SemanticTile::Emote(emote));
            buf[(cx, cy)]
                .set_char(glyph.symbol)
                .set_style(Style::default().fg(glyph.fg));
        }
        // The Cursor's frame of light, brightening as it arrives.
        let light = fade_colour((t / 1.0).min(1.0));
        let frame = self.theme.visible_frame();
        let style = Style::default().fg(light);
        for (x, y, c) in [
            (cx, cy.wrapping_sub(1), frame.up),
            (cx, cy + 1, frame.down),
            (cx.wrapping_sub(1), cy, frame.left),
            (cx + 1, cy, frame.right),
        ] {
            if area.contains((x, y).into()) {
                buf[(x, y)].set_char(c).set_style(style);
            }
        }
        if t > 4.0 {
            self.menu_box(buf, area, ((t - 4.0) / 1.5).min(1.0));
        }
    }

    /// D: the choices are things in the terrarium, touched with the Cursor.
    fn render_touch(&self, buf: &mut Buffer, area: Rect) {
        self.draw_world(buf, area, self.dim);
        for (i, row) in big("TERRA SPRITES").iter().enumerate() {
            let w = row.chars().count() as u16 + 4;
            let x = area.x + area.width.saturating_sub(w) / 2;
            let y = area.y + 1 + i as u16;
            clear(buf, Rect::new(x, y, w.min(area.width), 1));
            buf.set_string(x + 2, y, row, Style::default().fg(Color::White).bold());
        }
        let items = self.items();
        for (i, item) in items.iter().enumerate() {
            let (fx, fy) = item.spot();
            let x = area.x + (f32::from(area.width) * fx) as u16;
            let y = area.y + (f32::from(area.height) * fy) as u16;
            let (glyph, colour) = item.thing();
            let gw = glyph.chars().count() as u16;
            let mid = x + gw / 2;
            clear(buf, Rect::new(x.saturating_sub(1), y, gw + 2, 1));
            buf.set_string(x, y, glyph, Style::default().fg(colour).bold());
            let label = format!(" {} ", item.label());
            let lx = mid.saturating_sub(label.chars().count() as u16 / 2);
            let chosen = i == self.choice;
            let style = if chosen {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::Gray).bg(Color::Black)
            };
            buf.set_string(lx, y + 2, label, style);
            if chosen {
                // The Cursor, as a frame of light round the thing's middle.
                let frame = self.theme.visible_frame();
                let s = Style::default().fg(Color::White).bold();
                buf.set_string(mid, y.saturating_sub(1), frame.up.to_string(), s);
                buf.set_string(mid, y + 1, frame.down.to_string(), s);
                buf.set_string(x.saturating_sub(1), y, frame.left.to_string(), s);
                buf.set_string(x + gw, y, frame.right.to_string(), s);
            }
        }
    }

    fn draw_world(&self, buf: &mut Buffer, area: Rect, dim: bool) {
        draw_map(buf, area, &self.world, &self.theme, Pos { x: 0, y: 0 }, dim);
    }
}

/// Draws the world from `origin` into `area`, as the map view does. Dimmed,
/// everything but the sprites is dark grey.
fn draw_map(buf: &mut Buffer, area: Rect, world: &World, theme: &Theme, origin: Pos, dim: bool) {
    let map = world.map();
    for row in 0..area.height {
        for col in 0..area.width {
            let pos = Pos {
                x: origin.x + col,
                y: origin.y + row,
            };
            let (x, y) = (area.x + col, area.y + row);
            if pos.x >= map.width() || pos.y >= map.height() {
                buf[(x, y)].set_char(' ').set_style(Style::reset());
                continue;
            }
            let sprite = world.sprite_at(pos).is_some();
            let glyph: Glyph = if sprite {
                theme.glyph(SemanticTile::Sprite)
            } else if let Some(object) = world.object_at(pos) {
                theme.object_glyph(object.type_name(), object.visual_state())
            } else {
                theme.glyph(SemanticTile::Terrain(map.terrain(pos)))
            };
            let fg = if dim && !sprite {
                Color::DarkGray
            } else {
                glyph.fg
            };
            let mut style = Style::default().fg(fg);
            if glyph.bold && !dim {
                style = style.add_modifier(Modifier::BOLD);
            }
            buf[(x, y)].set_char(glyph.symbol).set_style(style);
        }
    }
}

/// The title's lettering, two rows high.
fn big(text: &str) -> [String; 2] {
    let mut rows = [String::new(), String::new()];
    for (i, c) in text.chars().enumerate() {
        let [top, bottom] = match c {
            'T' => ["▀█▀", " █ "],
            'E' => ["█▀▀", "██▄"],
            'R' => ["█▀█", "█▀▄"],
            'A' => ["▄▀█", "█▀█"],
            'S' => ["█▀", "▄█"],
            'P' => ["█▀█", "█▀▀"],
            'I' => ["█", "█"],
            _ => ["  ", "  "],
        };
        if i > 0 {
            rows[0].push(' ');
            rows[1].push(' ');
        }
        rows[0].push_str(top);
        rows[1].push_str(bottom);
    }
    rows
}

fn fade_colour(fade: f64) -> Color {
    if fade < 0.34 {
        Color::DarkGray
    } else if fade < 0.67 {
        Color::Gray
    } else {
        Color::White
    }
}

fn centred(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    let w = text.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    if y < area.bottom() {
        buf.set_string(x, y, text, style);
    }
}

fn clear(buf: &mut Buffer, r: Rect) {
    let r = r.intersection(buf.area);
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            buf[(x, y)].set_char(' ').set_style(Style::reset());
        }
    }
}

fn clear_row(buf: &mut Buffer, area: Rect, y: u16) {
    clear(buf, Rect::new(area.x, y, area.width, 1));
}

fn draw_box(buf: &mut Buffer, r: Rect, colour: Color) {
    let r = r.intersection(buf.area);
    if r.width < 2 || r.height < 2 {
        return;
    }
    let style = Style::default().fg(colour);
    for x in r.left()..r.right() {
        buf[(x, r.top())].set_char('═').set_style(style);
        buf[(x, r.bottom() - 1)].set_char('═').set_style(style);
    }
    for y in r.top()..r.bottom() {
        buf[(r.left(), y)].set_char('║').set_style(style);
        buf[(r.right() - 1, y)].set_char('║').set_style(style);
    }
    buf[(r.left(), r.top())].set_char('╔');
    buf[(r.right() - 1, r.top())].set_char('╗');
    buf[(r.left(), r.bottom() - 1)].set_char('╚');
    buf[(r.right() - 1, r.bottom() - 1)].set_char('╝');
}

/// "2 hours ago", for a save's time.
fn ago(when: Option<SystemTime>) -> String {
    let Some(secs) = when.and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()) else {
        return "saved".into();
    };
    let (n, unit) = match secs {
        0..60 => return "just now".into(),
        60..3600 => (secs / 60, "minute"),
        3600..86400 => (secs / 3600, "hour"),
        _ => (secs / 86400, "day"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}
