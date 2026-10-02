//! The UI state (design §6.8): everything the screen shows that isn't the world.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::layout::{Margin, Position, Rect, Size};
use serde::Deserialize;
use terra_sim::{
    ActionView, Command, CursorTouch, DeathCause, Dir, EntityId, Event, EventKind, Genome, Grip,
    MAX_NAME_CHARS, Map, Pos, Target, Thing, World,
};

use crate::clock::Clock;
use crate::cp437;
use crate::input::{Action, Button};
use crate::inspector;
use crate::text::{Names, ROOTED, Words, display_name};
use crate::theme::{Emote, Theme};

/// Whether the game carries on after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

/// What fills the screen besides the map (design §6.8). Help comes later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Normal,
    /// "Quit? (y/n)" is waiting for an answer.
    QuitPrompt,
    /// The Place menu is open (design v27 §6.5).
    PlaceMenu,
    /// The Place menu's list of genome files is open (design v27 §6.5).
    GenomeMenu,
    /// The player is typing a sprite's name (design §6.5).
    Naming,
}

/// What a Place menu item makes (design v27 §6.5).
#[derive(Debug, Clone, PartialEq)]
pub enum PlaceItem {
    /// An object of the type with this stable ID, called `name` in the data.
    Object { object_type: u16, name: String },
    /// A sprite from the starter genome with spawn variation.
    NewSprite,
    /// A sprite from a genome read from a file.
    Genome(Genome),
}

/// The Place menu item waiting on the Cursor until a click places it, and
/// what the menu called it.
#[derive(Debug, Clone)]
struct Placing {
    item: PlaceItem,
    label: String,
}

/// The Place menu's own items, after the object types the data offers.
const NEW_SPRITE: &str = "new sprite";
const FROM_A_FILE: &str = "sprite from a genome file";

/// What a click on the map does (design v21 §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorMode {
    Select,
    /// Teaching: a left click rewards, a right click corrects.
    Train,
    /// Moving things: a left click grabs, or lets go (design v23 §6.5).
    Grab,
}

impl CursorMode {
    /// Every cursor mode. Each theme must give all of them a mark.
    pub const ALL: [CursorMode; 3] = [CursorMode::Select, CursorMode::Train, CursorMode::Grab];

    /// The mode's name on the status line.
    pub fn label(self) -> &'static str {
        match self {
            CursorMode::Select => "SELECT",
            CursorMode::Train => "TRAIN",
            CursorMode::Grab => "GRAB",
        }
    }

    /// The mode `steps` along from this one, wrapping round.
    fn along(self, steps: i32) -> CursorMode {
        along(&CursorMode::ALL, self, steps)
    }
}

/// What the Cursor's status marks show (design v22 §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMark {
    /// Nothing to report.
    Idle,
    /// A click has just sent a command.
    Sent,
    /// The world has just applied one.
    Applied,
    /// The world has just refused one.
    Rejected,
    /// In Grab mode, empty, `Y`: a click would grab something (design v23
    /// §6.5).
    Grab,
    /// In Grab mode, empty, `N`.
    Empty,
    /// In Grab mode, holding or leading, `Y`: a click would put it down or
    /// let go.
    Release,
    /// In Grab mode, holding or leading, `N`: the thing's own glyph.
    Holding,
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
    fn along(self, steps: i32) -> Tab {
        along(&Tab::ALL, self, steps)
    }
}

/// The one of `all` that's `steps` along from `here`, wrapping round.
fn along<T: Copy + PartialEq>(all: &[T], here: T, steps: i32) -> T {
    let here = all
        .iter()
        .position(|&one| one == here)
        .expect("one of them") as i32;
    all[(here + steps).rem_euclid(all.len() as i32) as usize]
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
    /// Where the Cursor is: on the sprite it follows, or else on
    /// `pointed`.
    cursor: Pos,
    /// The tile under the pointer, or its last one (design §6.5).
    pointed: Pos,
    /// The sprite the Cursor follows, selected or not (design v26 §6.5).
    follow: Option<EntityId>,
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
    /// The latest events the event log shows, newest first, each with how
    /// many times in a row it came (design v21 §6.1).
    event_log: VecDeque<(Event, u32)>,
    /// The selected sprite's observed list, newest first.
    observed: VecDeque<Observed>,
    selection: Option<Selection>,
    tab: Tab,
    /// How many lines the open tab is scrolled down.
    tab_scroll: usize,
    /// Whether the detail view is on (design §6.1).
    detail: bool,
    /// Real time the app has been running, for the Decision marker's flashing.
    running_for: Duration,
    /// Each sprite's latest emote lately, and when, in `running_for`, it
    /// began (design §6.3).
    emotes: BTreeMap<EntityId, (Emote, Duration)>,
    /// The commands the player's clicks made, for the world (design §6.8).
    commands: Vec<Command>,
    /// Grab mode's commands the world hasn't applied yet, each with the
    /// tick the world applies it at, so the marks and the next click follow
    /// the queue (design v23 §6.5).
    queued: Vec<(u64, Command)>,
    /// While the Cursor leads a sprite, the tile the world was last told
    /// it's on (design v23 §6.5).
    told_tile: Option<Pos>,
    /// The sprite the Cursor leads, as the player sees it (`App::grip`),
    /// and where it stands, as of the latest action or tick: the leash's
    /// centre (design v23 §6.5).
    led: Option<(EntityId, Pos)>,
    /// While the player aims a throw or a shove (design v25 §6.5).
    aim: Option<Aim>,
    /// While the selected sprite slides from a shove: its observed line
    /// waits for the slide to end (design v25 §6.1).
    shoved: Option<Shoved>,
    /// When, in `running_for`, a click last sent a command (design v22 §6.5).
    sent_at: Option<Duration>,
    /// The world's latest report on a command, and when it began to show.
    report: Option<(StatusMark, Duration)>,
    /// Why the player's latest click was refused, which the status line
    /// says in the key hints' place, and since when (design v22 §6.1).
    refusal: Option<(String, Duration)>,
    /// The names the player has given sprites, as last seen, so the log
    /// keeps naming a sprite after it dies (design §6.5).
    names: Names,
    /// The Place menu item waiting on the Cursor (design v27 §6.5).
    placing: Option<Placing>,
    /// The highlighted item of the open menu, from 0.
    menu_choice: usize,
    /// Where genome files are saved and read from (design §6.7), if
    /// anywhere.
    genome_folder: Option<PathBuf>,
    /// The genome files the genome menu lists, by name, with their paths.
    genome_files: Vec<(String, PathBuf)>,
    /// While naming: the sprite, the name so far, and whether the player
    /// has typed it, rather than it being an offered random one.
    naming: Option<Naming>,
    /// How many random names have been offered, so each is another.
    names_offered: u64,
    /// What the player's latest action did that the status line says, in
    /// the key hints' place, and since when: where a genome was saved.
    notice: Option<(String, Duration)>,
}

/// A sprite's name being typed (design §6.5).
#[derive(Debug, Clone)]
struct Naming {
    sprite: EntityId,
    draft: String,
    typed: bool,
}

/// A shove of the selected sprite, which its observed list tells of once
/// the slide ends (design v25 §6.1).
#[derive(Debug, Clone)]
struct Shoved {
    sprite: EntityId,
    /// What it crashed into, once it has.
    crash: Option<Crash>,
}

/// What a sliding sprite crashed into, and whether that hurt it (design v25
/// §3.5.4).
#[derive(Debug, Clone)]
struct Crash {
    into: Thing,
    hurt: bool,
}

/// A throw or a shove being aimed (design v25 §6.5).
#[derive(Debug, Clone, Copy)]
struct Aim {
    /// Where the Cursor was when the right button was pressed, and stays,
    /// unless a led sprite slides on: a throw starts here.
    from: Pos,
}

/// How long the Decision marker shows, and then doesn't: once a second in
/// all, like a text cursor (design §6.1).
const FLASH_HALF: Duration = Duration::from_millis(500);

/// How long an emote shows, and then the sprite does: twice as fast as the
/// Decision marker, so the two can't be confused (design §6.3).
const EMOTE_HALF: Duration = Duration::from_millis(250);

/// How long an emote lasts, in real time, whatever the speed (design §6.3).
const EMOTE_FOR: Duration = Duration::from_secs(1);

/// How long the status marks flash, in real time (design v21 §6.5).
const MARK_FLASH_FOR: Duration = Duration::from_millis(300);

/// How far the Cursor goes from a sprite it leads, in tiles, in a square: a
/// UI setting (design v23 §6.5).
const LEASH: u16 = 5;

/// How long the status line says why a click was refused, in real time
/// (design v22 §6.1).
const REFUSAL_FOR: Duration = Duration::from_secs(3);

/// How a grab with nothing to grab is refused: "Nothing here to grab", or
/// on a fixture, "Can't grab the berry bush" (design v22 §6.1).
struct Refused {
    nothing_to: &'static str,
    cant: &'static str,
}

/// A left click's grab.
const TO_GRAB: Refused = Refused {
    nothing_to: "grab",
    cant: "grab",
};

/// A right press's grab, to aim what it grabs (design v25 §6.1).
const TO_SEND: Refused = Refused {
    nothing_to: "throw or shove",
    cant: "throw",
};

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
            pointed: cursor,
            follow: None,
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
            running_for: Duration::ZERO,
            emotes: BTreeMap::new(),
            commands: Vec::new(),
            queued: Vec::new(),
            told_tile: None,
            led: None,
            aim: None,
            shoved: None,
            sent_at: None,
            report: None,
            refusal: None,
            names: Names::default(),
            placing: None,
            menu_choice: 0,
            genome_folder: None,
            genome_files: Vec::new(),
            naming: None,
            names_offered: 0,
            notice: None,
        };
        app.centre_on(cursor);
        app
    }

    /// Takes in what happened during a tick, for the event log (design §6.1),
    /// and the death of the selected sprite. Object events and most action
    /// events are left out of the log: they happen dozens of times a minute
    /// and would bury everything else. The World tab counts objects instead,
    /// and the Body tab shows the selected sprite's action. The log keeps
    /// every Play and Hit, and any action that hurt a sprite.
    ///
    /// An action the selected sprite finishes, or another's done to it, goes
    /// on the front of its observed list, or counts up the line there if it
    /// reads the same.
    pub fn record(&mut self, events: &[Event], world: &World) {
        self.note_names(world);
        for event in events {
            // A sprite named and killed in one tick is gone before
            // `note_names` sees it, but its death carries its name.
            if let EventKind::Died {
                id,
                name: Some(name),
                ..
            } = &event.kind
            {
                self.names.note(*id, name);
            }
            if let EventKind::ActionEnded { id, ref action, .. } = event.kind {
                self.note_hurt(id, action);
                self.note_done_to_selected(event.tick, id, action, world);
            }
            if let (EventKind::Rewarded { id, .. } | EventKind::Corrected { id, .. }, Some(touch)) =
                (&event.kind, CursorTouch::reported(&event.kind))
            {
                self.note_touch(event.tick, *id, touch);
                self.flash_report(StatusMark::Applied);
            }
            // Let go, the selected sprite felt it as a pull from nowhere, since
            // it can't see the Cursor (design v23 §6.1).
            if let EventKind::LetGo { sprite } | EventKind::Shoved { sprite } = event.kind
                && self.selection == Some(Selection::Living(sprite))
            {
                self.observe(event.tick, "Was pulled along out of nowhere".into());
            }
            // A shove, felt as one from nowhere, is observed once the slide
            // ends, with what it crashed into (design v25 §6.1).
            match &event.kind {
                EventKind::Shoved { sprite }
                    if self.selection == Some(Selection::Living(*sprite)) =>
                {
                    self.shoved = Some(Shoved {
                        sprite: *sprite,
                        crash: None,
                    });
                }
                EventKind::Crashed { sprite, into, hurt } => {
                    if let Some(shoved) = &mut self.shoved
                        && shoved.sprite == *sprite
                    {
                        shoved.crash = Some(Crash {
                            into: into.clone(),
                            hurt: *hurt,
                        });
                    }
                }
                _ => {}
            }
            if let EventKind::CommandRejected { command, .. } = &event.kind {
                // The marks report the Cursor's clicks; naming isn't one
                // (design v27 §6.5).
                if !matches!(command, Command::Rename { .. }) {
                    self.flash_report(StatusMark::Rejected);
                }
                // And why, on the status line, as the log words it (design
                // v22 §6.1).
                if let Some(why) = inspector::event_line(event, &self.words(world)) {
                    self.refusal = Some((why, self.running_for));
                }
            }
            if let EventKind::Died { id, cause, age, .. } = event.kind {
                if self.selection == Some(Selection::Living(id)) {
                    self.selection = Some(Selection::Dead { id, cause, age });
                }
                // The followed sprite's death ends Follow (design v26 §6.5).
                if self.follow == Some(id) {
                    self.follow = None;
                }
            }
            // What the log says of it, if it's logged at all; a line that
            // reads as the one before merges into it with a count (design v21
            // §6.1).
            let words = Words {
                data: world.data(),
                names: &self.names,
            };
            let Some(line) = inspector::event_line(event, &words) else {
                continue;
            };
            match self.event_log.front_mut() {
                Some((front, count)) if inspector::event_line(front, &words) == Some(line) => {
                    *front = event.clone();
                    *count += 1;
                }
                _ => self.event_log.push_front((event.clone(), 1)),
            }
        }
        self.event_log.truncate(EVENT_LOG_LENGTH);
        self.note_slide_ended(world);
        // The world has applied what was queued for the ticks it has run.
        self.queued.retain(|&(tick, _)| tick >= world.tick());
        // An aim ends when what it aimed is gone: a held berry expired, or
        // a led sprite died (design v25 §6.5).
        if self.grip(world).is_none() {
            self.end_aim();
        }
        self.settle_cursor(world);
    }

    /// Notes the names the world's sprites have now, so a sprite named and
    /// then dead keeps its name in the log.
    fn note_names(&mut self, world: &World) {
        for sprite in world.sprites() {
            if let Some(name) = sprite.name() {
                self.names.note(sprite.id(), name);
            }
        }
    }

    /// The names the player has given sprites, as the screen last saw them.
    pub fn names(&self) -> &Names {
        &self.names
    }

    /// What the screen's sentences are made from, for `world`.
    pub(crate) fn words<'a>(&'a self, world: &'a World) -> Words<'a> {
        Words {
            data: world.data(),
            names: &self.names,
        }
    }

    /// Settles the Cursor after an action or a tick: what it leads, then
    /// where it is, then telling the world if it leads a sprite.
    fn settle_cursor(&mut self, world: &World) {
        self.led = match self.grip(world) {
            Some(Grip::Leads(id)) => world.sprite(id).map(|sprite| (id, sprite.pos())),
            _ => None,
        };
        self.track(world);
        self.tell(world);
    }

    /// While the Cursor leads a sprite, the tile it stands on: where the
    /// leash runs to from the Cursor (design v23 §6.5).
    pub fn leash(&self) -> Option<Pos> {
        self.led.map(|(_, at)| at)
    }

    /// The tile within the leash nearest `tile`: while the Cursor leads a
    /// sprite, it goes no further from it than `LEASH` tiles, in a square
    /// (design v23 §6.5).
    fn within_leash(&self, tile: Pos) -> Pos {
        let Some((_, at)) = self.led else {
            return tile;
        };
        let near =
            |to: u16, from: u16| to.clamp(from.saturating_sub(LEASH), from.saturating_add(LEASH));
        Pos {
            x: near(tile.x, at.x),
            y: near(tile.y, at.y),
        }
    }

    /// While the Cursor leads a sprite, tells the world each new tile it
    /// moves onto, in every mode (design v23 §6.5): the led sprite heads
    /// there.
    fn tell(&mut self, world: &World) {
        if !matches!(self.grip(world), Some(Grip::Leads(_))) {
            self.told_tile = None;
        } else if self.told_tile != Some(self.cursor) {
            self.commands
                .push(Command::MoveCursor { tile: self.cursor });
            self.told_tile = Some(self.cursor);
        }
    }

    /// What the Cursor has hold of as the player sees it: what the world
    /// says, as the commands queued since will leave it (design v23 §6.5).
    /// If the world refuses one, this goes back to the world's word.
    pub fn grip(&self, world: &World) -> Option<Grip> {
        let now = world.cursor();
        let held = now.holds().map(|item| Grip::Holds(item.id()));
        let start = now.leads().map(Grip::Leads).or(held);
        self.queued
            .iter()
            .fold(start, |grip, (_, command)| match *command {
                Command::TakeHold { sprite } => grip.or(Some(Grip::Leads(sprite))),
                Command::PickUp { item } => grip.or(Some(Grip::Holds(item))),
                Command::LetGo
                | Command::PutDown { .. }
                | Command::Throw { .. }
                | Command::Shove { .. } => None,
                Command::Reward { .. }
                | Command::Correct { .. }
                | Command::MoveCursor { .. }
                | Command::Place { .. }
                | Command::SpawnSprite { .. }
                | Command::Rename { .. } => grip,
            })
    }

    /// The sprite the Cursor follows, if any (design v26 §6.5). While the
    /// Cursor leads that sprite, Follow steps aside, in every mode, and the
    /// Cursor follows the pointer, within the leash (design v23 §6.5).
    pub fn followed(&self) -> Option<EntityId> {
        if self.follow_waits() {
            return None;
        }
        self.follow
    }

    /// Whether Follow steps aside: while the Cursor leads the followed
    /// sprite (design v23 §6.5).
    fn follow_waits(&self) -> bool {
        matches!((self.follow, self.led), (Some(followed), Some((led, _))) if followed == led)
    }

    /// Keeps the Cursor on the sprite it follows, wherever it has walked; and
    /// a leading one on the pointer, so a keyboard click lands where the
    /// player points. Either way, within the leash (design v23 §6.5). While
    /// the player aims, it stays on what it aims: a led sprite, which a
    /// slide may carry on, or where a held item will be thrown from (design
    /// v25 §6.5).
    fn track(&mut self, world: &World) {
        let wanted = if self.aim.is_some() {
            self.leash()
        } else {
            match self.followed().and_then(|id| world.sprite(id)) {
                Some(sprite) => Some(sprite.pos()),
                None if self.led.is_some() => Some(self.pointed),
                None => None,
            }
        };
        if let Some(tile) = wanted {
            self.cursor = self.within_leash(tile);
        }
    }

    /// Once the selected sprite's slide has ended, its observed list tells of
    /// the shove: "Was shoved out of nowhere", and what it crashed into, and
    /// whether that hurt (design v25 §6.1).
    fn note_slide_ended(&mut self, world: &World) {
        let Some(shoved) = self.shoved.take() else {
            return;
        };
        let id = shoved.sprite;
        if world
            .sprite(id)
            .is_some_and(|sprite| sprite.slide().is_some())
        {
            self.shoved = Some(shoved);
            return;
        }
        if self.selection != Some(Selection::Living(id)) {
            return;
        }
        let line = match shoved.crash {
            None => "Was shoved out of nowhere".to_string(),
            Some(Crash { into, hurt }) => {
                let hurt = if hurt { ", and got hurt" } else { "" };
                let into = inspector::crashed_into(&into, &self.words(world));
                format!("Was shoved out of nowhere, into {into}{hurt}")
            }
        };
        self.observe(world.tick().saturating_sub(1), line);
    }

    /// Starts the Hurt emote on each sprite that sprite `actor`'s `action` hurt.
    fn note_hurt(&mut self, actor: EntityId, action: &ActionView) {
        let hurt_target = match action.target {
            Some(Target::Sprite(id)) if action.hurt.target => Some(id),
            _ => None,
        };
        let hurt_actor = action.hurt.actor.then_some(actor);
        for id in hurt_actor.into_iter().chain(hurt_target) {
            self.emotes.insert(id, (Emote::Hurt, self.running_for));
        }
    }

    /// The Cursor's touch on sprite `id`, on `tick`: its emote, and a line
    /// on the observed list if it's the selected sprite's, told as the
    /// sprite felt it, from nowhere, since it can't see the Cursor (design
    /// v21 §6.1, §6.3).
    fn note_touch(&mut self, tick: u64, id: EntityId, touch: CursorTouch) {
        let (emote, line) = match touch {
            CursorTouch::Pet => (Emote::Pleased, "a gentle touch"),
            CursorTouch::Hug => (Emote::Pleased, "a warm embrace"),
            CursorTouch::Zap => (Emote::Shocked, "a zap"),
            CursorTouch::Shock => (Emote::Shocked, "a jolt"),
        };
        self.emotes.insert(id, (emote, self.running_for));
        if self.selection == Some(Selection::Living(id)) {
            self.observe(tick, format!("Felt {line} out of nowhere"));
        }
    }

    /// Puts sprite `actor`'s `action`, finished on `tick`, on the selected
    /// sprite's observed list, if it was the selected sprite's own or done
    /// to it.
    fn note_done_to_selected(
        &mut self,
        tick: u64,
        actor: EntityId,
        action: &ActionView,
        world: &World,
    ) {
        let Some(Selection::Living(selected)) = self.selection else {
            return;
        };
        if actor == selected {
            let line = inspector::observed_line(action, &self.words(world));
            self.observe(tick, line);
        } else if action.target == Some(Target::Sprite(selected))
            && let Some(line) = inspector::done_to_line(actor, action, &self.words(world))
        {
            self.observe(tick, line);
        }
    }

    /// The emote sprite `id` shows now, if any: the Hurt emote, taking
    /// turns with the sprite for a second after it's hurt (design §6.3).
    pub fn emote(&self, id: EntityId) -> Option<Emote> {
        let &(emote, at) = self.emotes.get(&id)?;
        let since = self.running_for.checked_sub(at)?;
        let showing = (since.as_millis() / EMOTE_HALF.as_millis()).is_multiple_of(2);
        (since < EMOTE_FOR && showing).then_some(emote)
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

    /// What the Cursor's status marks show now (design v22 §6.5): of the
    /// flashes under way, the one that began last.
    pub fn status_mark(&self) -> StatusMark {
        let sent = self.sent_at.map(|at| (StatusMark::Sent, at));
        // Last, so a report wins a tie: within a frame, it's the later news.
        [sent, self.report]
            .into_iter()
            .flatten()
            .filter(|&(_, from)| self.shows(from, MARK_FLASH_FOR))
            .max_by_key(|&(_, at)| at)
            .map_or(StatusMark::Idle, |(mark, _)| mark)
    }

    /// What the Cursor's two status marks show now, top right (`Y`) then
    /// bottom left (`N`) (design v23 §6.5). In Grab mode they show what the
    /// Cursor has hold of, as the queue will leave it, unless a refusal is
    /// flashing; in the other modes, both show `status_mark`.
    pub fn status_marks(&self, world: &World) -> [StatusMark; 2] {
        let flash = self.status_mark();
        if self.mode != CursorMode::Grab || flash == StatusMark::Rejected {
            return [flash; 2];
        }
        // A Place menu item waiting on the Cursor shows `↓` and its glyph, as
        // something carried would (design v27 §6.5).
        if self.placing.is_some() {
            return [StatusMark::Release, StatusMark::Holding];
        }
        match self.grip(world) {
            Some(_) => [StatusMark::Release, StatusMark::Holding],
            None => [StatusMark::Grab, StatusMark::Empty],
        }
    }

    /// Why the player's latest click was refused, while the status line
    /// says so (design v22 §6.1).
    pub fn refusal(&self) -> Option<&str> {
        let (why, from) = self.refusal.as_ref()?;
        self.shows(*from, REFUSAL_FOR).then_some(why.as_str())
    }

    /// Whether something that shows from `from`, for `lasting`, shows now.
    fn shows(&self, from: Duration, lasting: Duration) -> bool {
        self.running_for
            .checked_sub(from)
            .is_some_and(|since| since < lasting)
    }

    /// Flashes the world's report on a command in the status marks, once
    /// the latest `+` has shown for its time, so both flashes show however
    /// soon the report comes. A refusal still to show, or showing, isn't
    /// replaced by an applied command: it's what needs noticing (design v22
    /// §6.5). A click that sends clears it, so it's never an earlier
    /// click's.
    fn flash_report(&mut self, mark: StatusMark) {
        let now = self.running_for;
        if let Some((StatusMark::Rejected, at)) = self.report
            && mark == StatusMark::Applied
            && now < at + MARK_FLASH_FOR
        {
            return;
        }
        let from = self.sent_at.map_or(now, |at| now.max(at + MARK_FLASH_FOR));
        self.report = Some((mark, from));
    }

    /// Takes the commands the player's clicks have made since last taken,
    /// in the order made, for the world to apply at its next tick.
    pub fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }

    /// The events the event log shows, newest first, each the latest of
    /// the same event in a row, with how many there were.
    pub fn event_log(&self) -> impl Iterator<Item = (&Event, u32)> {
        self.event_log.iter().map(|(event, count)| (event, *count))
    }

    /// Moves the app's real-time clock on by `elapsed`, for what flashes.
    pub fn animate(&mut self, elapsed: Duration) {
        self.running_for += elapsed;
        let now = self.running_for;
        self.emotes.retain(|_, &mut (_, at)| now - at < EMOTE_FOR);
    }

    /// Whether the Decision marker is in its "on" half just now.
    pub fn flash_on(&self) -> bool {
        (self.running_for.as_millis() / FLASH_HALF.as_millis()).is_multiple_of(2)
    }

    /// Whether the detail view is on: the exact workings behind what the
    /// screen describes in words (design §6.1).
    pub fn detail(&self) -> bool {
        self.detail
    }

    /// The tile the Cursor is on: its followed sprite's, or else the
    /// pointer's.
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
        match self.screen {
            Screen::PlaceMenu | Screen::GenomeMenu => return self.apply_in_menu(action, world),
            Screen::Naming => return self.apply_naming(action, world),
            Screen::Normal | Screen::QuitPrompt => {}
        }
        if self.screen == Screen::QuitPrompt {
            match action {
                Action::Confirm | Action::Back | Action::Quit => return Flow::Quit,
                // The mouse carries on as usual and doesn't answer the prompt;
                // nor does letting go of a button, or of `E`, which isn't a
                // key pressed.
                Action::Point(_)
                | Action::Click { .. }
                | Action::Follow { at: Some(_) }
                | Action::Wheel { .. }
                | Action::Release { .. } => {}
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
            Action::Click {
                at,
                button,
                amplified,
            } => {
                self.point(at);
                // A click lands where the Cursor is: not past the leash
                // (design v23 §6.5).
                if let Some(tile) = self.tile_at(at) {
                    self.act(self.within_leash(tile), button, amplified, world);
                }
            }
            Action::Press { button, amplified } => {
                self.act(self.cursor, button, amplified, world);
            }
            // A middle click points, then acts where the Cursor is, as `F`
            // does (design v26 §6.5).
            Action::Follow { at } => {
                if let Some(at) = at {
                    self.point(at);
                }
                if at.is_none_or(|at| self.tile_at(at).is_some()) {
                    self.toggle_follow(self.cursor, world);
                }
            }
            Action::Release { button, at } => {
                if let Some(at) = at {
                    self.point(at);
                }
                if button == Button::Right && self.aim.is_some() {
                    self.send_aimed(world);
                }
            }
            // `C` again in Grab mode opens the Place menu (design §6.5).
            Action::Mode(CursorMode::Grab) if self.mode == CursorMode::Grab => {
                self.end_aim();
                self.open_menu(Screen::PlaceMenu);
            }
            Action::Mode(mode) => self.mode = mode,
            Action::Rename => self.start_naming(world),
            Action::ExportGenome => self.export_genome(world),
            Action::SelectNext => self.select_along(world, Direction::Next),
            Action::SelectPrevious => self.select_along(world, Direction::Previous),
            Action::NextTab => self.open(self.tab.along(1)),
            Action::PreviousTab => self.open(self.tab.along(-1)),
            Action::ScrollTab { pages } => {
                let page = self.inspector_rows() as i32;
                self.scroll_tab(pages * page, world);
            }
            // The wheel scrolls the inspector's tab, and over the map cycles
            // the cursor modes (design v22 §6.5).
            Action::Wheel { at, notches } => {
                self.point(at);
                if self.inspector.is_some_and(|area| area.contains(at)) {
                    self.scroll_tab(notches * WHEEL_LINES, world);
                } else if self.tile_at(at).is_some() {
                    self.mode = self.mode.along(notches);
                }
            }
            Action::ToggleDetail => self.detail = !self.detail,
            // Esc cancels an aim first (design v25 §6.5); then returns to
            // Select; from Select it asks to quit (design v21 §6.5).
            Action::Back if self.aim.is_some() => self.end_aim(),
            Action::Back if self.mode != CursorMode::Select => self.mode = CursorMode::Select,
            Action::Back => self.screen = Screen::QuitPrompt,
            Action::Confirm
            | Action::Dismiss
            | Action::Pick(_)
            | Action::Enter
            | Action::Type(_)
            | Action::Erase
            | Action::AnotherName => {}
            Action::Quit => return Flow::Quit,
        }
        // Only Grab mode aims.
        if self.mode != CursorMode::Grab {
            self.end_aim();
        }
        self.settle_cursor(world);
        Flow::Continue
    }

    /// What a click with `button` on `tile` does, in the cursor mode (design
    /// v21 §6.5).
    ///
    /// In Select, a left click selects the sprite there; on empty ground it
    /// clears the selection, which leaves Follow as it is. A right
    /// click activates what's there, and nothing can be activated yet
    /// (design v26 §6.5).
    ///
    /// In Train, a left click rewards the target and a right click corrects
    /// it: the followed sprite, or else the one there. With none, nothing is
    /// sent.
    fn act(&mut self, tile: Pos, button: Button, amplified: bool, world: &World) {
        let sprite = world.sprite_at(tile).map(|sprite| sprite.id());
        match (self.mode, button) {
            (CursorMode::Select, Button::Left) => match sprite {
                Some(id) => self.select(id),
                None => self.selection = None,
            },
            // Until there are devices, nothing can be activated (design v26
            // §6.5).
            (CursorMode::Select, Button::Right) => {
                self.refuse("Nothing here to activate".into());
            }
            // A Place menu item waiting on the Cursor takes the next click,
            // where the Cursor is: on the followed sprite's tile, or else
            // the one clicked. A right click puts it away (design v27 §6.5).
            (CursorMode::Grab, Button::Left) if self.placing.is_some() => {
                let followed = self.followed().and_then(|id| world.sprite(id));
                self.place(followed.map_or(tile, |sprite| sprite.pos()));
            }
            (CursorMode::Grab, Button::Right) if self.placing.is_some() => self.placing = None,
            (CursorMode::Grab, Button::Left) => self.grab_click(tile, sprite, world),
            (CursorMode::Grab, Button::Right) => self.aim_click(tile, sprite, world),
            (CursorMode::Train, button) => {
                let touch = match (button, amplified) {
                    (Button::Left, false) => CursorTouch::Pet,
                    (Button::Left, true) => CursorTouch::Hug,
                    (Button::Right, false) => CursorTouch::Zap,
                    (Button::Right, true) => CursorTouch::Shock,
                };
                // With nothing to act on, nothing is sent, so `?` flashes at
                // once, and the status line says why (design v22 §6.5).
                let Some(sprite) = self.followed().or(sprite) else {
                    self.refuse(format!("No sprite here to {}", touch.name()));
                    return;
                };
                self.commands
                    .push(touch.command(sprite, self.clock.reach_back()));
                // The marks follow the latest click: an earlier click's
                // report, not shown yet, is behind it (design v22 §6.5).
                self.sent_at = Some(self.running_for);
                self.report = None;
            }
        }
    }

    /// A Grab-mode click on `tile`, with `sprite` on it (design v23 §6.5).
    /// Leading, it lets go; holding, it puts the item down there, or at the
    /// followed sprite's feet, wherever the click lands (design v26 §6.5).
    /// Empty, it takes hold of the followed sprite, or else the sprite there,
    /// or else picks up the item there; a fixture is rooted to the ground.
    fn grab_click(&mut self, tile: Pos, sprite: Option<EntityId>, world: &World) {
        match self.grip(world) {
            Some(Grip::Leads(_)) => self.send(Command::LetGo, world),
            Some(Grip::Holds(_)) => {
                let tile = if self.followed().is_some() {
                    self.cursor
                } else {
                    tile
                };
                self.send(Command::PutDown { tile }, world);
            }
            None => {
                self.grab(tile, sprite, world, TO_GRAB);
            }
        }
    }

    /// With the Cursor empty, grabs what's on `tile`, with `sprite` on it
    /// (design v23 §6.5): takes hold of the followed sprite, or else the
    /// sprite there, or else picks up the item there; a fixture is rooted to
    /// the ground, and with nothing there, it's `refused`. Says whether it
    /// grabbed anything.
    fn grab(
        &mut self,
        tile: Pos,
        sprite: Option<EntityId>,
        world: &World,
        refused: Refused,
    ) -> bool {
        let command = match (self.followed().or(sprite), world.object_at(tile)) {
            (Some(sprite), _) => {
                // Taking hold puts the Cursor, as the world knows it, on the
                // sprite.
                self.told_tile = world.sprite(sprite).map(|s| s.pos());
                Command::TakeHold { sprite }
            }
            // In M1 every solid object is a fixture (design §3.3).
            (None, Some(object)) if object.is_solid() => {
                let (cant, name) = (refused.cant, display_name(object.type_name()));
                self.refuse(format!("Can't {cant} the {name}: {ROOTED}"));
                return false;
            }
            (None, Some(object)) => Command::PickUp { item: object.id() },
            (None, None) => {
                self.refuse(format!("Nothing here to {}", refused.nothing_to));
                return false;
            }
        };
        self.send(command, world);
        true
    }

    /// Sends a Grab-mode command, queued, so the marks and the next click
    /// follow it (design v23 §6.5).
    fn send(&mut self, command: Command, world: &World) {
        self.commands.push(command.clone());
        self.queued.push((world.tick(), command));
        // The marks follow the latest click (design v22 §6.5).
        self.report = None;
    }

    /// A Grab-mode right click on `tile`, with `sprite` on it (design v25
    /// §6.5): it starts aiming what the Cursor has hold of, the pull measured
    /// from the Cursor to the pointer. With the Cursor empty, it grabs what's
    /// there first, as a left click would. Aiming already, it sends the aim,
    /// as `E` pressed again does in a terminal that doesn't report a key let
    /// go.
    fn aim_click(&mut self, tile: Pos, sprite: Option<EntityId>, world: &World) {
        if self.aim.is_some() {
            self.send_aimed(world);
            return;
        }
        if self.grip(world).is_none() && !self.grab(tile, sprite, world, TO_SEND) {
            return;
        }
        // Leading, the Cursor goes onto the sprite, which then stands still,
        // since a led sprite walks towards the Cursor (design v25 §6.5).
        if let Some(Grip::Leads(id)) = self.grip(world)
            && let Some(led) = world.sprite(id)
        {
            self.cursor = led.pos();
        }
        self.aim = Some(Aim { from: self.cursor });
    }

    /// Sends what the player aimed (design v25 §6.5): the thing goes the
    /// opposite way to the pull, from the Cursor to the pointer, snapped to
    /// the nearest of the 8 directions, as far as the pull, in the game's own
    /// measure. With the pointer on the Cursor, it sends nothing.
    fn send_aimed(&mut self, world: &World) {
        let (aim, aimed) = (self.aim, self.aimed());
        self.end_aim();
        let (Some(Aim { from, .. }), Some((toward, tiles))) = (aim, aimed) else {
            return;
        };
        let command = match self.grip(world) {
            Some(Grip::Holds(_)) => Command::Throw {
                from,
                toward,
                tiles,
            },
            Some(Grip::Leads(_)) => Command::Shove { toward, tiles },
            None => return,
        };
        self.send(command, world);
    }

    /// While the player aims, the way the thing will go and how far, as
    /// the pull says (design v25 §6.5): the opposite way to the pull, from
    /// the Cursor to the pointer, snapped to the nearest of the 8
    /// directions, as far as the pull, in the game's own measure. `None` for
    /// a pull of nothing.
    fn aimed(&self) -> Option<(Dir, u16)> {
        self.aim?;
        let (dx, dy) = (
            i32::from(self.cursor.x) - i32::from(self.pointed.x),
            i32::from(self.cursor.y) - i32::from(self.pointed.y),
        );
        let toward = Dir::nearest(dx, dy)?;
        let tiles = dx.unsigned_abs().max(dy.unsigned_abs());
        Some((toward, u16::try_from(tiles).unwrap_or(u16::MAX)))
    }

    /// Ends the aim, sent or not: the Cursor follows the pointer again, as
    /// far as Follow and the leash let it (design v25 §6.5).
    fn end_aim(&mut self) {
        if self.aim.take().is_some() && self.followed().is_none() {
            self.cursor = self.within_leash(self.pointed);
        }
    }

    /// Whether the player is aiming a throw or a shove (design v25 §6.5).
    pub fn aiming(&self) -> bool {
        self.aim.is_some()
    }

    /// While the player aims a held item, it and where it will be thrown
    /// from, under the Cursor, where it's drawn until it's thrown (design
    /// v25 §6.5).
    pub fn thrown_from(&self, world: &World) -> Option<(EntityId, Pos)> {
        match (self.aim, self.grip(world)) {
            (Some(Aim { from }), Some(Grip::Holds(item))) => Some((item, from)),
            _ => None,
        }
    }

    /// While the player aims, the tiles the thing will cross, from the one
    /// after where it is to where it would stop if nothing's in the way:
    /// as far as the pull sends it, no further than the Cursor can send it,
    /// and not past the wall (design v25 §6.5). Empty otherwise. A held item
    /// starts where aiming began, and a led sprite where it stands.
    pub fn aim_line(&self, world: &World) -> Vec<Pos> {
        let (Some(Aim { from, .. }), Some((toward, tiles)), Some(grip)) =
            (self.aim, self.aimed(), self.grip(world))
        else {
            return Vec::new();
        };
        let start = match grip {
            Grip::Holds(_) => Some(from),
            Grip::Leads(id) => world.sprite(id).map(|sprite| sprite.pos()),
        };
        let Some(mut at) = start else {
            return Vec::new();
        };
        let mut line = Vec::new();
        for _ in 0..tiles.min(world.furthest(grip)) {
            match world.map().neighbour(at, toward) {
                Some(next) => at = next,
                None => break,
            }
            line.push(at);
        }
        line
    }

    /// A click with nothing to act on: it sends nothing, so `?` flashes at
    /// once, and the status line says why (design v22 §6.5).
    fn refuse(&mut self, why: String) {
        self.report = Some((StatusMark::Rejected, self.running_for));
        self.refusal = Some((why, self.running_for));
        self.notice = None;
    }

    /// Says what the player's action did on the status line, for a while.
    fn tell_player(&mut self, what: String) {
        self.notice = Some((what, self.running_for));
        self.refusal = None;
    }

    /// What the player's latest action did, while the status line says so:
    /// where a genome was saved.
    pub fn notice(&self) -> Option<&str> {
        let (what, from) = self.notice.as_ref()?;
        self.shows(*from, REFUSAL_FOR).then_some(what.as_str())
    }

    /// Sets where genome files are saved and read from (design §6.7).
    pub fn set_genome_folder(&mut self, folder: PathBuf) {
        self.genome_folder = Some(folder);
    }

    /// What the Place menu item waiting on the Cursor is called, if one is
    /// (design v27 §6.5).
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
        let empty = match self.screen {
            Screen::GenomeMenu => self.no_genome_files(),
            _ => String::new(),
        };
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

    /// What the genome menu says with no files to list.
    pub fn no_genome_files(&self) -> String {
        match &self.genome_folder {
            Some(folder) => format!("No genome files in {}", folder.display()),
            None => "No folder for genome files".into(),
        }
    }

    /// Opens `menu`, highlighting its first item.
    fn open_menu(&mut self, menu: Screen) {
        self.screen = menu;
        self.menu_choice = 0;
    }

    /// What an action does while a menu is open: a number or `Enter` picks
    /// an item, the arrow keys move the highlight, a click picks the item
    /// under it or, off the menu, closes it, and `Esc` closes it.
    fn apply_in_menu(&mut self, action: Action, world: &World) -> Flow {
        let count = self.menu_items(world).len();
        match action {
            Action::Pick(n) if usize::from(n) <= count => {
                self.choose(usize::from(n) - 1, world);
            }
            Action::Enter if count > 0 => self.choose(self.menu_choice, world),
            Action::Scroll { dy, .. } if count > 0 => {
                let moved = self.menu_choice as i32 + dy.signum();
                self.menu_choice = moved.clamp(0, count as i32 - 1) as usize;
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
            Action::Quit => return Flow::Quit,
            _ => {}
        }
        self.settle_cursor(world);
        Flow::Continue
    }

    /// Picks item `index` of the open menu (design v27 §6.5): an object type the
    /// data offers, or a new sprite, waits on the Cursor; the Place menu's
    /// last item lists the genome files; a genome file is read, and its
    /// sprite waits.
    fn choose(&mut self, index: usize, world: &World) {
        let menu = self.screen;
        let label = self.menu_items(world)[index].clone();
        self.screen = Screen::Normal;
        let placeable: Vec<&str> = world.data().placeable().map(|(name, _)| name).collect();
        let item = match menu {
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
        let mut files: Vec<(String, PathBuf)> = self
            .genome_folder
            .as_ref()
            .and_then(|folder| std::fs::read_dir(folder).ok())
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let is_ron = path.extension().is_some_and(|ext| ext == "ron");
                let name = path.file_stem()?.to_string_lossy().into_owned();
                is_ron.then_some((name, path))
            })
            .collect();
        files.sort();
        self.genome_files = files;
        self.open_menu(Screen::GenomeMenu);
    }

    /// Places the item waiting on the Cursor on `tile` (design v27 §6.5).
    fn place(&mut self, tile: Pos) {
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

    /// Whether keys type letters, rather than act: while naming.
    pub fn typing(&self) -> bool {
        self.screen == Screen::Naming
    }

    /// The name being typed, while naming (design §6.5).
    pub fn name_draft(&self) -> Option<&str> {
        self.naming.as_ref().map(|naming| naming.draft.as_str())
    }

    /// The sprite being named, while naming.
    pub fn naming_sprite(&self) -> Option<EntityId> {
        self.naming.as_ref().map(|naming| naming.sprite)
    }

    /// Starts naming the selected sprite, offering a random name to start
    /// from (design §6.5).
    fn start_naming(&mut self, world: &World) {
        let Some(Selection::Living(sprite)) = self.selection else {
            return self.refuse("Select a sprite to name it".into());
        };
        let draft = self.random_name(sprite, world);
        self.naming = Some(Naming {
            sprite,
            draft,
            typed: false,
        });
        self.screen = Screen::Naming;
    }

    /// Another random name for `sprite`, made up by the screen, so naming
    /// never draws from the world's randomness (design §6.5).
    fn random_name(&mut self, sprite: EntityId, world: &World) -> String {
        self.names_offered += 1;
        // Mixes the session's seed, the sprite and how many names have been
        // offered, so each offer differs; the odd constants are the usual
        // 64-bit multiplicative hash ones, spreading nearby numbers apart.
        let seed = self
            .seed
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(sprite.0.wrapping_mul(0xbf58_476d_1ce4_e5b9))
            .wrapping_add(self.names_offered);
        world.data().random_name(seed)
    }

    /// What an action does while the player types a name: letters CP437
    /// can show, and spaces, are typed, up to the most a name may have, the first
    /// replacing the offered name; `Backspace` rubs one out; `Tab` offers
    /// another; `Enter` sends the name and `Esc` gives up.
    fn apply_naming(&mut self, action: Action, world: &World) -> Flow {
        let Some(naming) = self.naming.as_mut() else {
            self.screen = Screen::Normal;
            return Flow::Continue;
        };
        match action {
            // Only letters CP437 can show, and spaces between them (design v27 §6.5).
            Action::Type(c) if (c.is_alphabetic() || c == ' ') && cp437::contains(c) => {
                if !naming.typed {
                    naming.draft.clear();
                    naming.typed = true;
                }
                if naming.draft.chars().count() < MAX_NAME_CHARS {
                    naming.draft.push(c);
                }
            }
            Action::Erase => {
                naming.draft.pop();
                naming.typed = true;
            }
            Action::AnotherName => {
                let sprite = naming.sprite;
                let draft = self.random_name(sprite, world);
                let naming = self.naming.as_mut().expect("naming");
                naming.draft = draft;
                naming.typed = false;
            }
            Action::Enter => {
                let naming = self.naming.take().expect("naming");
                self.screen = Screen::Normal;
                if naming.draft.trim_matches(' ').is_empty() {
                    self.refuse("A name needs a letter in it".into());
                } else {
                    self.commands.push(Command::Rename {
                        sprite: naming.sprite,
                        name: naming.draft,
                    });
                }
            }
            Action::Back => {
                self.naming = None;
                self.screen = Screen::Normal;
            }
            Action::Quit => return Flow::Quit,
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }

    /// Saves the selected sprite's genome to a file in the genome folder
    /// (design §6.1, §6.7), named for the sprite and the tick, and says
    /// where.
    fn export_genome(&mut self, world: &World) {
        let Some(sprite) = self.selection.and_then(|s| match s {
            Selection::Living(id) => world.sprite(id),
            Selection::Dead { .. } => None,
        }) else {
            return self.refuse("Select a sprite to save its genome".into());
        };
        let Some(folder) = self.genome_folder.clone() else {
            return self.refuse("There's no folder to save genomes in".into());
        };
        let label = self.names.label(sprite.id());
        let slug: String = label
            .chars()
            .filter_map(|c| match c {
                c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
                ' ' | '-' => Some('-'),
                _ => None,
            })
            .collect();
        let file = format!("{slug}-tick-{}.ron", world.tick());
        let text = sprite.genome().to_ron(world.data());
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

    /// `F`, or a middle click, on `tile`, in every mode (design v26 §6.5).
    /// Following, it stops, wherever `tile` is. Otherwise the Cursor follows
    /// the sprite on `tile`, or else the selected sprite, leaving the
    /// selection as it is; with neither, it's refused.
    fn toggle_follow(&mut self, tile: Pos, world: &World) {
        if self.follow.is_some() {
            self.follow = None;
            // It follows the pointer again, unless the player is aiming:
            // then it stays on what it aims until the aim ends (design v25
            // §6.5).
            if self.aim.is_none() {
                self.cursor = self.pointed;
            }
            return;
        }
        let selected = match self.selection {
            Some(Selection::Living(id)) => Some(id),
            _ => None,
        };
        match world.sprite_at(tile).map(|sprite| sprite.id()).or(selected) {
            Some(id) => self.follow = Some(id),
            None => self.refuse("No sprite here to follow".into()),
        }
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

    /// Notes the tile at screen cell `cell` as the pointer's, and puts the
    /// Cursor there, within the leash, unless it follows a sprite. Off the map
    /// view's tiles, both stay on their last tile.
    fn point(&mut self, cell: Position) {
        match self.tile_at(cell) {
            Some(tile) => {
                self.pointer = Some(cell);
                self.pointed = tile;
                // While the player aims, the Cursor stays on what it aims,
                // and the pointer pulls (design v25 §6.5).
                if self.followed().is_none() && self.aim.is_none() {
                    self.cursor = self.within_leash(tile);
                }
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
