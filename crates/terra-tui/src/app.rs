//! The UI state (design §6.8): everything the screen shows that isn't the world.

use std::collections::VecDeque;

use ratatui::layout::{Position, Rect, Size};
use serde::Deserialize;
use terra_sim::{DeathCause, EntityId, Event, EventKind, Map, Pos, World};

use crate::clock::Clock;
use crate::input::Action;
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
    Chem,
    Genome,
    World,
}

impl Tab {
    /// Every tab, in the order `[` and `]` go through them.
    pub const ALL: [Tab; 4] = [Tab::Body, Tab::Chem, Tab::Genome, Tab::World];

    /// The tab `steps` along from this one, wrapping around.
    fn along(self, steps: isize) -> Tab {
        let here = Tab::ALL.iter().position(|&tab| tab == self).expect("a tab") as isize;
        Tab::ALL[(here + steps).rem_euclid(Tab::ALL.len() as isize) as usize]
    }
}

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
    /// The screen cell under the mouse pointer, while that cell shows a tile.
    pointer: Option<Position>,
    /// The latest events the event log shows, newest first.
    event_log: VecDeque<Event>,
    selection: Option<Selection>,
    tab: Tab,
}

impl App {
    /// A new UI for `map`, with the cursor at the map's centre and the viewport
    /// centred on it. `tile_area` is where on screen the map view draws its tiles.
    pub fn new(map: &Map, theme: Theme, seed: u64, tile_area: Rect) -> App {
        let cursor = Pos {
            x: map.width() / 2,
            y: map.height() / 2,
        };
        let centred = |cursor: u16, view: u16, len: u16| {
            clamp_origin(i32::from(cursor) - i32::from(view / 2), view, len)
        };
        App {
            clock: Clock::new(),
            theme,
            seed,
            screen: Screen::Normal,
            mode: CursorMode::Select,
            cursor,
            viewport: Pos {
                x: centred(cursor.x, tile_area.width, map.width()),
                y: centred(cursor.y, tile_area.height, map.height()),
            },
            map_size: Size::new(map.width(), map.height()),
            tile_area,
            pointer: None,
            event_log: VecDeque::new(),
            selection: None,
            tab: Tab::World,
        }
    }

    /// Takes in what happened during a tick, for the event log (design §6.1),
    /// and the death of the selected sprite. Object events are left out of the
    /// log: they happen dozens of times a minute and would bury everything
    /// else, and the World tab counts objects instead.
    pub fn record(&mut self, events: &[Event]) {
        for event in events {
            if let EventKind::Died { id, cause, age } = event.kind
                && self.selection == Some(Selection::Living(id))
            {
                self.selection = Some(Selection::Dead { id, cause, age });
            }
            if !matches!(
                event.kind,
                EventKind::ObjectSpawned { .. } | EventKind::ObjectRemoved { .. }
            ) {
                self.event_log.push_front(event.clone());
            }
        }
        self.event_log.truncate(EVENT_LOG_LENGTH);
    }

    /// The events the event log shows, newest first.
    pub fn event_log(&self) -> impl Iterator<Item = &Event> {
        self.event_log.iter()
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

    /// Refits the viewport to where the map view now draws its tiles, as after
    /// a resize.
    pub fn fit_viewport(&mut self, tile_area: Rect) {
        self.tile_area = tile_area;
        self.settle();
    }

    /// Carries out an action on `world`, and says whether the game carries on.
    pub fn apply(&mut self, action: Action, world: &World) -> Flow {
        if self.screen == Screen::QuitPrompt {
            match action {
                Action::Confirm | Action::Back | Action::Quit => return Flow::Quit,
                // The mouse carries on as usual and doesn't answer the prompt.
                Action::Point(_) | Action::Click(_) => {}
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
            Action::NextTab => self.tab = self.tab.along(1),
            Action::PreviousTab => self.tab = self.tab.along(-1),
            Action::Back => self.screen = Screen::QuitPrompt,
            Action::Confirm | Action::Dismiss => {}
            Action::Quit => return Flow::Quit,
        }
        Flow::Continue
    }

    /// Selects the sprite `id`. From the World tab, that opens Body.
    fn select(&mut self, id: EntityId) {
        self.selection = Some(Selection::Living(id));
        if self.tab == Tab::World {
            self.tab = Tab::Body;
        }
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
        let pos = world
            .sprites()
            .find(|sprite| sprite.id() == id)
            .expect("a sprite just listed")
            .pos();
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
