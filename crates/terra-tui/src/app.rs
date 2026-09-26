//! The UI state (design §6.8): everything the screen shows that isn't the world.

use std::collections::VecDeque;

use ratatui::layout::{Margin, Position, Rect, Size};
use serde::Deserialize;
use terra_sim::{DeathCause, EntityId, Event, EventKind, Map, Pos, World};

use crate::clock::Clock;
use crate::input::Action;
use crate::inspector;
use crate::theme::Theme;

/// Whether the game carries on after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

/// What fills the screen besides the map (design §6.8). Menus and help come later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Normal,
    /// "Quit? (y/n)" is waiting for an answer.
    QuitPrompt,
}

/// What a click on the map does (design §6.5). The Hand, Reward and Correct
/// modes arrive with the hand's slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorMode {
    Select,
}

impl CursorMode {
    /// Every cursor mode. Each theme must give all of them a mark.
    pub const ALL: [CursorMode; 1] = [CursorMode::Select];

    /// The mode's name on the status line.
    pub fn label(self) -> &'static str {
        match self {
            CursorMode::Select => "SELECT",
        }
    }
}

/// How many lines the observed list keeps (design §6.1).
pub const OBSERVED_LENGTH: usize = 500;

/// A line of the Body tab's observed list (design §6.1): an action the
/// player watched the selected sprite finish, or several in a row that read
/// the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observed {
    /// What it did, in the past tense.
    pub line: String,
    /// How many times in a row.
    pub count: u32,
    /// The tick the latest of them finished on.
    pub tick: u64,
}

/// The sprite the inspector shows (design §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// A sprite in the world.
    Living(EntityId),
    /// A sprite that died while selected: of what, and at what age.
    Dead {
        id: EntityId,
        cause: DeathCause,
        age: u64,
    },
}

impl Selection {
    /// The selected sprite's ID, living or dead.
    pub fn id(self) -> EntityId {
        match self {
            Selection::Living(id) | Selection::Dead { id, .. } => id,
        }
    }
}

/// An inspector tab (design §6.1). The Brain tab joins with the brain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Body,
    Brain,
    Chem,
    Genome,
    World,
}

impl Tab {
    /// Every tab, in the order `[` and `]` go through them.
    pub const ALL: [Tab; 5] = [Tab::Body, Tab::Brain, Tab::Chem, Tab::Genome, Tab::World];

    /// The tab's name in the inspector's title.
    pub fn label(self) -> &'static str {
        match self {
            Tab::Body => "Body",
            Tab::Brain => "Brain",
            Tab::Chem => "Chem",
            Tab::Genome => "Genome",
            Tab::World => "World",
        }
    }

    /// The tab `steps` along from this one, wrapping around.
    fn along(self, steps: isize) -> Tab {
        let here = Tab::ALL.iter().position(|&tab| tab == self).expect("a tab") as isize;
        Tab::ALL[(here + steps).rem_euclid(Tab::ALL.len() as isize) as usize]
    }
}

/// Where on screen the panels the app works with are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Areas {
    /// The screen cells where the map view draws its tiles.
    pub tiles: Rect,
    /// The inspector, border included, if the screen has room for it.
    pub inspector: Option<Rect>,
}

/// How many lines a notch of the mouse wheel scrolls an inspector tab.
const WHEEL_LINES: i32 = 3;

/// How many events the event log keeps.
const EVENT_LOG_LENGTH: usize = 100;

/// The UI state. Rendering reads it; actions change it.
pub struct App {
    pub clock: Clock,
    pub theme: Theme,
    /// The seed the world was made from, shown so a map can be made again.
    pub seed: u64,
    screen: Screen,
    mode: CursorMode,
    cursor: Pos,
    /// The top-left tile of the viewport.
    viewport: Pos,
    /// The map's size, in tiles.
    map_size: Size,
    /// The screen cells where the map view draws its tiles.
    tile_area: Rect,
    /// The inspector, border included, if the screen has room for it.
    inspector: Option<Rect>,
    /// The screen cell under the mouse pointer, while that cell shows a tile.
    pointer: Option<Position>,
    /// The latest events the event log shows, newest first.
    event_log: VecDeque<Event>,
    /// The selected sprite's observed list, newest first.
    observed: VecDeque<Observed>,
    selection: Option<Selection>,
    tab: Tab,
    /// How many lines the open tab is scrolled down.
    tab_scroll: usize,
    /// Whether the detail view is on (design §6.1).
    detail: bool,
}

impl App {
    /// A new UI for `map`, with the cursor at the map's centre and the viewport
    /// centred on it, and its panels drawn in `areas`.
    pub fn new(map: &Map, theme: Theme, seed: u64, areas: Areas) -> App {
        let cursor = Pos {
            x: map.width() / 2,
            y: map.height() / 2,
        };
        let mut app = App {
            clock: Clock::new(),
            theme,
            seed,
            screen: Screen::Normal,
            mode: CursorMode::Select,
            cursor,
            viewport: Pos { x: 0, y: 0 },
            map_size: Size::new(map.width(), map.height()),
            tile_area: areas.tiles,
            inspector: areas.inspector,
            pointer: None,
            event_log: VecDeque::new(),
            observed: VecDeque::new(),
            selection: None,
            tab: Tab::World,
            tab_scroll: 0,
            detail: false,
        };
        app.centre_on(cursor);
        app
    }

    /// Takes in what happened during a tick, for the event log (design §6.1),
    /// and the death of the selected sprite. Object and action events are
    /// left out of the log: they happen dozens of times a minute and would
    /// bury everything else. The World tab counts objects instead, and the
    /// Body tab shows the selected sprite's action.
    ///
    /// An action the selected sprite finishes goes on the front of its
    /// observed list, or counts up the line there if it reads the same.
    pub fn record(&mut self, events: &[Event], world: &World) {
        for event in events {
            if let EventKind::ActionEnded { id, ref action, .. } = event.kind
                && self.selection == Some(Selection::Living(id))
            {
                self.observe(event.tick, inspector::observed_line(action, world.data()));
            }
            if let EventKind::Died { id, cause, age } = event.kind
                && self.selection == Some(Selection::Living(id))
            {
                self.selection = Some(Selection::Dead { id, cause, age });
            }
            if !matches!(
                event.kind,
                EventKind::ObjectSpawned { .. }
                    | EventKind::ObjectRemoved { .. }
                    | EventKind::ActionStarted { .. }
                    | EventKind::ActionEnded { .. }
            ) {
                self.event_log.push_front(event.clone());
            }
        }
        self.event_log.truncate(EVENT_LOG_LENGTH);
    }

    /// Puts `line`, finished on `tick`, on the front of the observed list.
    fn observe(&mut self, tick: u64, line: String) {
        match self.observed.front_mut() {
            Some(front) if front.line == line => {
                front.count += 1;
                front.tick = tick;
            }
            _ => {
                self.observed.push_front(Observed {
                    line,
                    count: 1,
                    tick,
                });
                self.observed.truncate(OBSERVED_LENGTH);
            }
        }
    }

    /// What the player has watched the selected sprite finish since
    /// selecting it, newest first (design §6.1).
    pub fn observed(&self) -> impl Iterator<Item = &Observed> {
        self.observed.iter()
    }

    /// The events the event log shows, newest first.
    pub fn event_log(&self) -> impl Iterator<Item = &Event> {
        self.event_log.iter()
    }

    /// Whether the detail view is on: the exact workings behind what the
    /// screen describes in words (design §6.1).
    pub fn detail(&self) -> bool {
        self.detail
    }

    /// The tile the cursor is on.
    pub fn cursor(&self) -> Pos {
        self.cursor
    }

    /// The top-left tile of the viewport: the part of the map the map view shows.
    pub fn viewport(&self) -> Pos {
        self.viewport
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn mode(&self) -> CursorMode {
        self.mode
    }

    /// The sprite the inspector shows, if one is selected.
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// The open inspector tab.
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// How many lines the open tab is scrolled down.
    pub fn tab_scroll(&self) -> usize {
        self.tab_scroll
    }

    /// The tile drawn at screen cell `cell`, if the map view draws one there.
    pub fn tile_at(&self, cell: Position) -> Option<Pos> {
        let area = self.tile_area;
        area.contains(cell).then(|| Pos {
            x: (self.viewport.x + (cell.x - area.x)).min(self.map_size.width - 1),
            y: (self.viewport.y + (cell.y - area.y)).min(self.map_size.height - 1),
        })
    }

    /// The screen cell where `tile` is drawn, if it is in view.
    pub fn cell_of(&self, tile: Pos) -> Option<Position> {
        let (area, origin) = (self.tile_area, self.viewport);
        let in_view = (origin.x..origin.x + area.width).contains(&tile.x)
            && (origin.y..origin.y + area.height).contains(&tile.y);
        in_view.then(|| Position::new(area.x + (tile.x - origin.x), area.y + (tile.y - origin.y)))
    }

    /// Refits the app to where its panels are now drawn, as after a resize.
    pub fn fit(&mut self, areas: Areas) {
        self.tile_area = areas.tiles;
        self.inspector = areas.inspector;
        self.settle();
    }

    /// Carries out an action on `world`, and says whether the game carries on.
    pub fn apply(&mut self, action: Action, world: &World) -> Flow {
        if self.screen == Screen::QuitPrompt {
            match action {
                Action::Confirm | Action::Back | Action::Quit => return Flow::Quit,
                // The mouse carries on as usual and doesn't answer the prompt.
                Action::Point(_) | Action::Click(_) | Action::Wheel { .. } => {}
                // Any other key cancels the prompt, and does nothing else.
                _ => {
                    self.screen = Screen::Normal;
                    return Flow::Continue;
                }
            }
        }
        match action {
            Action::TogglePause => self.clock.toggle_pause(),
            Action::StepOnce => self.clock.step_once(),
            Action::Faster { held: false } => self.clock.faster(),
            Action::Faster { held: true } => self.clock.faster_held(),
            Action::Slower { held: false } => self.clock.slower(),
            Action::Slower { held: true } => self.clock.slower_held(),
            Action::Scroll { dx, dy } => self.scroll(dx, dy),
            Action::Point(cell) => self.point(cell),
            // In Select mode (the only mode so far), a click on the map
            // selects the sprite there, or clears the selection.
            Action::Click(cell) => {
                self.point(cell);
                if let Some(tile) = self.tile_at(cell) {
                    match world.sprite_at(tile) {
                        Some(sprite) => self.select(sprite.id()),
                        None => self.selection = None,
                    }
                }
            }
            Action::SelectNext => self.select_along(world, Direction::Next),
            Action::SelectPrevious => self.select_along(world, Direction::Previous),
            Action::NextTab => self.open(self.tab.along(1)),
            Action::PreviousTab => self.open(self.tab.along(-1)),
            Action::ScrollTab { pages } => {
                let page = self.inspector_rows() as i32;
                self.scroll_tab(pages * page, world);
            }
            Action::Wheel { at, notches } => {
                self.point(at);
                if self.inspector.is_some_and(|area| area.contains(at)) {
                    self.scroll_tab(notches * WHEEL_LINES, world);
                }
            }
            Action::ToggleDetail => self.detail = !self.detail,
            Action::Back => self.screen = Screen::QuitPrompt,
            Action::Confirm | Action::Dismiss => {}
            Action::Quit => return Flow::Quit,
        }
        Flow::Continue
    }

    /// Selects the sprite `id`. From the World tab, that opens Body; and
    /// another sprite than before shows its tab from the top.
    fn select(&mut self, id: EntityId) {
        let another = self.selection.map(Selection::id) != Some(id);
        if self.tab == Tab::World {
            self.open(Tab::Body);
        } else if another {
            self.tab_scroll = 0;
        }
        if another {
            self.observed.clear();
        }
        self.selection = Some(Selection::Living(id));
    }

    /// Opens `tab`, from the top.
    fn open(&mut self, tab: Tab) {
        self.tab = tab;
        self.tab_scroll = 0;
    }

    /// The rows inside the inspector's border: a page.
    fn inspector_rows(&self) -> usize {
        self.inspector
            .map_or(0, |area| usize::from(area.inner(Margin::new(1, 1)).height))
    }

    /// Scrolls the open tab by `lines` from where it's shown, down being
    /// positive, stopping at the top and where its last line comes into view.
    fn scroll_tab(&mut self, lines: i32, world: &World) {
        let (length, rows) = (inspector::lines(self, world).len(), self.inspector_rows());
        let from = inspector::first_shown(self.tab_scroll, length, rows);
        let furthest = inspector::first_shown(usize::MAX, length, rows);
        let scrolled = from as i64 + i64::from(lines);
        self.tab_scroll = scrolled.clamp(0, furthest as i64) as usize;
    }

    /// Selects the sprite with the next or previous ID, wrapping around, and
    /// centres the viewport on it if it's out of view. With nothing selected,
    /// it starts from the lowest or highest ID.
    fn select_along(&mut self, world: &World, direction: Direction) {
        let ids: Vec<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
        let current = self.selection.map(Selection::id);
        let chosen = match direction {
            Direction::Next => current
                .and_then(|current| ids.iter().find(|&&id| id > current))
                .or(ids.first()),
            Direction::Previous => current
                .and_then(|current| ids.iter().rev().find(|&&id| id < current))
                .or(ids.last()),
        };
        let Some(&id) = chosen else {
            return;
        };
        self.select(id);
        let pos = world.sprite(id).expect("a sprite just listed").pos();
        if self.cell_of(pos).is_none() {
            self.centre_on(pos);
        }
    }

    /// Scrolls the viewport so that `tile` is at its centre, as far as the wall allows.
    fn centre_on(&mut self, tile: Pos) {
        let area = self.tile_area;
        self.viewport = Pos {
            x: clamp_origin(
                i32::from(tile.x) - i32::from(area.width / 2),
                area.width,
                self.map_size.width,
            ),
            y: clamp_origin(
                i32::from(tile.y) - i32::from(area.height / 2),
                area.height,
                self.map_size.height,
            ),
        };
        self.settle();
    }

    fn scroll(&mut self, dx: i32, dy: i32) {
        let shifted = |origin: u16, delta: i32| {
            (i32::from(origin) + delta).clamp(0, i32::from(u16::MAX)) as u16
        };
        self.viewport = Pos {
            x: shifted(self.viewport.x, dx),
            y: shifted(self.viewport.y, dy),
        };
        self.settle();
    }

    /// Keeps the viewport within the wall, and the cursor on whatever tile is
    /// under a still pointer.
    fn settle(&mut self) {
        let (map, area) = (self.map_size, self.tile_area);
        self.viewport = Pos {
            x: clamp_origin(i32::from(self.viewport.x), area.width, map.width),
            y: clamp_origin(i32::from(self.viewport.y), area.height, map.height),
        };
        if let Some(cell) = self.pointer {
            self.point(cell);
        }
    }

    /// Puts the cursor on the tile at screen cell `cell`. Off the map view's
    /// tiles, the cursor stays on its last tile.
    fn point(&mut self, cell: Position) {
        match self.tile_at(cell) {
            Some(tile) => {
                self.pointer = Some(cell);
                self.cursor = tile;
            }
            None => self.pointer = None,
        }
    }
}

/// Which way `Tab` and `Shift+Tab` go through the sprites.
#[derive(Debug, Clone, Copy)]
enum Direction {
    Next,
    Previous,
}

/// A viewport origin kept within the map, so the view never shows past the wall.
/// A map no bigger than the view is shown from its start.
fn clamp_origin(origin: i32, view: u16, len: u16) -> u16 {
    let furthest = (i32::from(len) - i32::from(view)).max(0);
    origin.clamp(0, furthest) as u16
}
