//! The UI state (design §6.8): everything the screen shows that isn't the world.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::layout::{Margin, Position, Rect, Size};
use serde::Deserialize;
use terra_sim::{
    ActionView, ChemicalKind, ChemicalLevel, Command, CursorTouch, DeathCause, Dir, EntityId,
    Event, EventKind, Genome, Grip, MAX_NAME_CHARS, Map, Outcome, Pos, Progress, SpriteView,
    Target, Thing, Verb, World,
};

use crate::clock::Clock;
use crate::cp437;
use crate::input::{Action, Button};
use crate::inspector;
use crate::policy::{InfoPolicy, Omniscient, Panel, Subject};
use crate::saves::{self, MAX_SAVE_NAME_CHARS, QUICKSAVE, SaveFile};
use crate::sprite_list::{self, SortBy};
use crate::text::{Names, ROOTED, Words, display_name, group_thousands};
use crate::theme::{Emote, Theme};

/// Whether the game carries on after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

/// What fills the screen besides the map (design §6.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Normal,
    /// "Quit? (y/n)" is waiting for an answer.
    QuitPrompt,
    /// The Place menu is open (design v28 §6.5).
    PlaceMenu,
    /// The Place menu's list of genome files is open (design v28 §6.5).
    GenomeMenu,
    /// The player is typing a sprite's name (design §6.5).
    Naming,
    /// The player is typing a name to save the world as (`Ctrl+S`, design
    /// §6.7).
    SaveNaming,
    /// The list of saves to load is open (`Ctrl+O`, design §6.7).
    LoadMenu,
    /// "Load …? (y/n)" is waiting for an answer: the world has run since it
    /// was last saved (design §6.7).
    LoadPrompt,
    /// The help screen is open (design §6.1).
    Help,
    /// The sprite list is open (design §6.1).
    SpriteList,
}

/// What colours sprites on the map (design §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourMode {
    /// The colour of its strongest drive above .5, or its own colour if
    /// none is.
    Drive,
    /// Its own colour.
    Plain,
}

impl ColourMode {
    /// Every colour mode, in the order `b` goes through them.
    pub const ALL: [ColourMode; 2] = [ColourMode::Drive, ColourMode::Plain];

    /// What the status line calls it when `b` switches to it.
    pub fn label(self) -> &'static str {
        match self {
            ColourMode::Drive => "strongest drive",
            ColourMode::Plain => "plain",
        }
    }
}

/// How strong a drive must be to colour its sprite, and to show in the
/// sprite list's Drive column (design §6.3).
pub(crate) const DRIVE_SHOWS: f32 = 0.5;

/// `sprite`'s strongest drive, if one is above half (design §6.3). Of two
/// equally strong, the first in the data pack's order.
pub(crate) fn strongest_drive<'a>(sprite: &SpriteView<'a>) -> Option<ChemicalLevel<'a>> {
    sprite
        .chemicals()
        .filter(|chemical| chemical.kind == ChemicalKind::Drive && chemical.level > DRIVE_SHOWS)
        .fold(None::<ChemicalLevel>, |best, chemical| match best {
            Some(best) if best.level >= chemical.level => Some(best),
            _ => Some(chemical),
        })
}

/// Which events the event log shows (design §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFilter {
    /// Every event it logs.
    All,
    /// What the selected sprite did, or had done to it.
    Selected,
    /// Deaths, lessons learned and refusals.
    Major,
}

impl EventFilter {
    /// Every filter, in the order `m` goes through them.
    pub const ALL: [EventFilter; 3] = [EventFilter::All, EventFilter::Selected, EventFilter::Major];

    /// Its name on the event log's border.
    pub fn label(self) -> &'static str {
        match self {
            EventFilter::All => "all",
            EventFilter::Selected => "selected",
            EventFilter::Major => "major",
        }
    }

    /// The event log's border's label: every filter, the one in use in
    /// brackets, as the inspector's title shows its tabs.
    pub fn labels(self) -> String {
        let labels: Vec<String> = EventFilter::ALL
            .iter()
            .map(|&filter| {
                if filter == self {
                    format!("[{}]", filter.label())
                } else {
                    filter.label().to_string()
                }
            })
            .collect();
        format!(" {} ", labels.join(" "))
    }
}

/// What a Place menu item makes (design v28 §6.5).
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
    /// The event log, border included, if the screen has room for it.
    pub event_log: Option<Rect>,
    /// Where the help screen and the sprite list are drawn: between the top
    /// bar and the status line, if the screen has room for the game.
    pub overlay: Option<Rect>,
}

/// How many lines a notch of the mouse wheel scrolls an inspector tab.
const WHEEL_LINES: i32 = 3;

/// How many events the event log keeps.
const EVENT_LOG_LENGTH: usize = 100;

/// How good a tick must feel to a sprite, its `last_r`, for it to show the
/// Pleased emote: a UI setting (design §6.3). Measured on the default world,
/// seeds 1 to 3, about one tick in seven that felt good at all felt this good:
/// a meal or a drink when it mattered, not small comforts.
pub const PLEASED_AT: f32 = 0.3;

/// What a frame's ticks did that the screen takes in, tick by tick: each
/// one's events, and the sprites that felt a strong reward on it (design
/// §6.3), which a look at the world after the last tick would miss.
#[derive(Debug, Default)]
pub struct Ticks(Vec<(Vec<Event>, Vec<EntityId>)>);

impl Ticks {
    /// Runs one tick of `world`, noting what the screen shows of it.
    pub fn step(&mut self, world: &mut World) {
        let events = world.step();
        let pleased = world
            .sprites()
            .filter(|sprite| sprite.felt() >= PLEASED_AT)
            .map(|sprite| sprite.id())
            .collect();
        self.0.push((events, pleased));
    }
}

/// A sprite's rest, as the Resting emote shows it (design §6.3): since when,
/// in `running_for`, and when it ended, once it has.
#[derive(Debug, Clone, Copy)]
struct Rest {
    since: Duration,
    ended: Option<Duration>,
}

impl Rest {
    /// Until when its emote shows: as long as the rest lasts, and at least
    /// as long as any emote, so a rest over in a blink at speed is seen.
    fn until(self) -> Option<Duration> {
        self.ended.map(|ended| ended.max(self.since + EMOTE_FOR))
    }
}

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
    /// The tile the pointer points at, or its last one (design v27 §6.5).
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
    /// The screen cell under the mouse pointer, while it points at a tile.
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
    /// The rests the Resting emote shows (design §6.3).
    resting: BTreeMap<EntityId, Rest>,
    /// The commands the player's clicks made, for the world (design §6.8).
    commands: Vec<Command>,
    /// Grab mode's commands the world hasn't applied yet, each with the
    /// tick the world applies it at, so the marks and the next click follow
    /// the queue (design v23 §6.5).
    queued: Vec<(u64, Command)>,
    /// While the Cursor leads a sprite, or sprites can see it, the tile the
    /// world was last told it's on (design v23, v29 §6.5).
    told_tile: Option<Pos>,
    /// The cursor modes in which sprites can see the Cursor (design v29
    /// §6.5): none, to begin with.
    visible_in: BTreeSet<CursorMode>,
    /// Whether the world was last told sprites can see the Cursor.
    told_visible: bool,
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
    /// The Place menu item waiting on the Cursor (design v28 §6.5).
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
    /// What the screen may show the player (design §6.4).
    policy: Box<dyn InfoPolicy>,
    /// What colours sprites on the map (design §6.3).
    colour_mode: ColourMode,
    /// Whether the view follows the selected sprite (`T`, design v21
    /// §6.1).
    tracking: bool,
    /// The event log, border included, if the screen has room for it.
    event_log_area: Option<Rect>,
    /// Where the help screen and the sprite list are drawn.
    overlay: Option<Rect>,
    /// Which events the event log shows (design §6.1).
    event_filter: EventFilter,
    /// What the sprite list is sorted by.
    list_sort: SortBy,
    /// The sprite the sprite list highlights, if any; the first row's if
    /// it's gone from the list.
    list_choice: Option<EntityId>,
    /// The game's folder for its files (design §6.7), which the help
    /// screen shows, if there is one.
    data_folder: Option<PathBuf>,
    /// Where saves go (design §6.7), if anywhere.
    save_folder: Option<PathBuf>,
    /// The saves the load menu lists, newest first.
    save_files: Vec<SaveFile>,
    /// While naming a save, the name so far.
    save_naming: Option<Draft>,
    /// The save waiting for the player to say yes to loading it.
    to_load: Option<SaveFile>,
    /// A world just loaded, and the save's name, for the frame loop to take
    /// (`take_loaded`).
    loaded: Option<(World, String)>,
    /// When the world was last saved, or loaded: its tick then, and the
    /// time then, in `running_for`. `None` for a world never saved.
    saved: Option<(u64, Duration)>,
    /// How long time has run, unpaused, since the world was last saved or
    /// loaded, for the question before loading over it.
    unsaved_run: Duration,
    /// How long time has run, unpaused, since the last autosave (design
    /// §6.7).
    since_autosave: Duration,
}

/// How often the world saves itself while time runs (design §6.7).
pub const AUTOSAVE_EVERY: Duration = Duration::from_secs(10 * 60);

/// A sprite's name being typed (design §6.5).
#[derive(Debug, Clone)]
struct Naming {
    sprite: EntityId,
    draft: Draft,
}

/// A name being typed on the status line, for a sprite or a save: what's
/// there so far, and whether the player has typed it, rather than it being
/// the one offered, which the first letter typed replaces.
#[derive(Debug, Clone)]
struct Draft {
    text: String,
    typed: bool,
}

impl Draft {
    /// `text`, offered.
    fn offered(text: String) -> Draft {
        Draft { text, typed: false }
    }

    /// Types `c`, replacing the name offered, up to `most` characters.
    fn type_char(&mut self, c: char, most: usize) {
        if !self.typed {
            self.text.clear();
            self.typed = true;
        }
        if self.text.chars().count() < most {
            self.text.push(c);
        }
    }

    /// Rubs out the last character.
    fn erase(&mut self) {
        self.text.pop();
        self.typed = true;
    }
}

/// A shove of the selected sprite, which its observed list tells of once
/// the slide ends (design v25 §6.1).
#[derive(Debug, Clone)]
struct Shoved {
    sprite: EntityId,
    /// What it crashed into, once it has.
    crash: Option<Crash>,
    /// Where the shove came from, as the sprite felt it (design v29 §6.1).
    how: String,
}

/// Where the Cursor's doing came from, as a sprite felt it in the tick just
/// run (design v29 §6.1): "out of nowhere", or, while sprites could see it,
/// "by the Cursor" (or "from" it, as `preposition` says).
fn whence(world: &World, preposition: &str) -> String {
    if world.cursor().visible() {
        format!("{preposition} the Cursor")
    } else {
        "out of nowhere".into()
    }
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
            resting: BTreeMap::new(),
            commands: Vec::new(),
            queued: Vec::new(),
            told_tile: None,
            visible_in: BTreeSet::new(),
            told_visible: false,
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
            policy: Box::new(Omniscient),
            colour_mode: ColourMode::Drive,
            tracking: false,
            event_log_area: areas.event_log,
            overlay: areas.overlay,
            event_filter: EventFilter::All,
            list_sort: SortBy::Number,
            list_choice: None,
            data_folder: None,
            save_folder: None,
            save_files: Vec::new(),
            save_naming: None,
            to_load: None,
            loaded: None,
            saved: None,
            unsaved_run: Duration::ZERO,
            since_autosave: Duration::ZERO,
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
            // The events carry the names, so the log names a sprite as it
            // was then: one named and killed in one tick is gone before
            // `note_names` sees it.
            if let EventKind::Renamed { id, name }
            | EventKind::Died {
                id,
                name: Some(name),
                ..
            } = &event.kind
            {
                self.names.note(*id, name);
            }
            if let EventKind::ActionEnded { id, ref action, .. } = event.kind {
                self.note_hurt(id, action);
                self.note_gave_up(id, action);
                self.note_done_to_selected(event.tick, id, action, world);
            }
            self.note_rest(&event.kind);
            if let (EventKind::Rewarded { id, .. } | EventKind::Corrected { id, .. }, Some(touch)) =
                (&event.kind, CursorTouch::reported(&event.kind))
            {
                self.note_touch(event.tick, *id, touch, world);
                self.flash_report(StatusMark::Applied);
            }
            // Let go, the selected sprite felt it as a pull from nowhere,
            // unless it could see the Cursor (design v23, v29 §6.1).
            if let EventKind::LetGo { sprite } | EventKind::Shoved { sprite } = event.kind
                && self.selection == Some(Selection::Living(sprite))
            {
                let how = whence(world, "by");
                self.observe(event.tick, format!("Was pulled along {how}"));
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
                        how: whence(world, "by"),
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
                // (design v28 §6.5).
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
                self.resting.remove(&id);
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
        self.track_selected(world);
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
    /// there. So too while sprites can see it, where they see it; and
    /// whether they can, as the cursor mode changes (design v29 §6.5).
    fn tell(&mut self, world: &World) {
        let leading = matches!(self.grip(world), Some(Grip::Leads(_)));
        let visible = self.visible();
        if !leading && !visible {
            self.told_tile = None;
        } else if self.told_tile != Some(self.cursor) {
            self.commands
                .push(Command::MoveCursor { tile: self.cursor });
            self.told_tile = Some(self.cursor);
        }
        if self.told_visible != visible {
            self.commands.push(Command::ShowCursor { visible });
            self.told_visible = visible;
        }
    }

    /// Whether sprites can see the Cursor: the current cursor mode's switch
    /// (design v29 §6.5).
    pub fn visible(&self) -> bool {
        self.visible_in.contains(&self.mode)
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
                | Command::ShowCursor { .. }
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
        let how = shoved.how;
        let line = match shoved.crash {
            None => format!("Was shoved {how}"),
            Some(Crash { into, hurt }) => {
                let hurt = if hurt { ", and got hurt" } else { "" };
                let into = inspector::crashed_into(&into, &self.words(world));
                format!("Was shoved {how}, into {into}{hurt}")
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
            self.start_emote(id, Emote::Hurt);
        }
    }

    /// Starts the Failed emote on sprite `actor` if it gave its `action` up:
    /// failed, blocked or timed out (design §6.3). An action that also hurt
    /// it shows Hurt instead, which matters more.
    fn note_gave_up(&mut self, actor: EntityId, action: &ActionView) {
        let gave_up = matches!(
            action.progress,
            Progress::Ended(Outcome::Failed | Outcome::Blocked | Outcome::TimedOut)
        );
        if gave_up && !action.hurt.actor {
            self.start_emote(actor, Emote::Failed);
        }
    }

    /// Notes a rest starting or ending, for the Resting emote (design §6.3).
    fn note_rest(&mut self, event: &EventKind) {
        let now = self.running_for;
        match *event {
            EventKind::ActionStarted {
                id,
                verb: Verb::Rest,
            } => {
                self.resting.insert(
                    id,
                    Rest {
                        since: now,
                        ended: None,
                    },
                );
            }
            EventKind::ActionEnded {
                id,
                verb: Verb::Rest,
                ..
            } => {
                if let Some(rest) = self.resting.get_mut(&id) {
                    rest.ended.get_or_insert(now);
                }
            }
            _ => {}
        }
    }

    /// Starts `emote` on sprite `id` now: the newest emote wins (design
    /// §6.3).
    fn start_emote(&mut self, id: EntityId, emote: Emote) {
        self.emotes.insert(id, (emote, self.running_for));
    }

    /// Takes in what a frame's ticks did: their events, as `record` does,
    /// and the Pleased emote on each sprite that felt a strong reward
    /// (design §6.3). One already showing Pleased carries on, rather than
    /// starting again each tick a reward lasts.
    pub fn take_in(&mut self, ticks: Ticks, world: &World) {
        // Tick by tick, so of two emotes in one frame the later tick's wins.
        for (events, pleased) in ticks.0 {
            self.record(&events, world);
            for id in pleased {
                let showing = matches!(self.emotes.get(&id), Some(&(Emote::Pleased, at)) if self.shows(at, EMOTE_FOR));
                if !showing {
                    self.start_emote(id, Emote::Pleased);
                }
            }
        }
    }

    /// The Cursor's touch on sprite `id`, on `tick`: its emote, and a line
    /// on the observed list if it's the selected sprite's, told as the
    /// sprite felt it: from nowhere, unless it could see the Cursor (design
    /// v21, v29 §6.1, §6.3).
    fn note_touch(&mut self, tick: u64, id: EntityId, touch: CursorTouch, world: &World) {
        let (emote, line) = match touch {
            CursorTouch::Pet => (Emote::Pleased, "a gentle touch"),
            CursorTouch::Hug => (Emote::Pleased, "a warm embrace"),
            CursorTouch::Zap => (Emote::Shocked, "a zap"),
            CursorTouch::Shock => (Emote::Shocked, "a jolt"),
        };
        self.start_emote(id, emote);
        if self.selection == Some(Selection::Living(id)) {
            let from = whence(world, "from");
            self.observe(tick, format!("Felt {line} {from}"));
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

    /// The emote sprite `id` shows now, if any, taking turns with the
    /// sprite (design §6.3): the newest for a second after what set it off,
    /// or else Resting, for as long as it rests.
    pub fn emote(&self, id: EntityId) -> Option<Emote> {
        let flashing = |at: Duration| {
            let since = self.running_for.saturating_sub(at);
            (since.as_millis() / EMOTE_HALF.as_millis()).is_multiple_of(2)
        };
        if let Some(&(emote, at)) = self.emotes.get(&id)
            && self.shows(at, EMOTE_FOR)
        {
            return flashing(at).then_some(emote);
        }
        let rest = self.resting.get(&id)?;
        let resting = rest.until().is_none_or(|until| self.running_for < until);
        (resting && flashing(rest.since)).then_some(Emote::Resting)
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
        // something carried would (design v28 §6.5).
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
    /// the same event in a row, with how many there were: those the filter
    /// lets through (design §6.1).
    pub fn event_log(&self) -> impl Iterator<Item = (&Event, u32)> {
        self.event_log
            .iter()
            .filter(|(event, _)| self.filter_shows(event))
            .map(|(event, count)| (event, *count))
    }

    /// Whether the event filter lets `event` through (design §6.1).
    fn filter_shows(&self, event: &Event) -> bool {
        match self.event_filter {
            EventFilter::All => true,
            EventFilter::Selected => self
                .selection
                .is_some_and(|selected| inspector::event_sprites(event).contains(&selected.id())),
            EventFilter::Major => matches!(
                event.kind,
                EventKind::Died { .. }
                    | EventKind::LearnedMilestone { .. }
                    | EventKind::CommandRejected { .. }
            ),
        }
    }

    /// Which events the event log shows (design §6.1).
    pub fn event_filter(&self) -> EventFilter {
        self.event_filter
    }

    /// Where the event log's border shows the filters, if it's drawn.
    pub fn filter_label(&self) -> Option<Rect> {
        let area = self.event_log_area?;
        crate::ui::top_right(area, self.event_filter.labels().chars().count() as u16)
    }

    /// Moves the app's real-time clock on by `elapsed`, for what flashes.
    pub fn animate(&mut self, elapsed: Duration) {
        self.running_for += elapsed;
        if !self.clock.is_paused() {
            self.unsaved_run += elapsed;
            self.since_autosave += elapsed;
        }
        let now = self.running_for;
        self.emotes.retain(|_, &mut (_, at)| now - at < EMOTE_FOR);
        self.resting
            .retain(|_, rest| rest.until().is_none_or(|until| now < until));
    }

    /// Whether the Decision marker is in its "on" half just now.
    pub fn flash_on(&self) -> bool {
        (self.running_for.as_millis() / FLASH_HALF.as_millis()).is_multiple_of(2)
    }

    /// Sets what the screen may show the player (design §6.4).
    pub fn set_policy(&mut self, policy: impl InfoPolicy + 'static) {
        self.policy = Box::new(policy);
    }

    /// Whether `panel` may show what it knows about `subject` (design §6.4).
    pub fn can_view(&self, panel: Panel, subject: Subject) -> bool {
        self.policy.can_view(panel, subject)
    }

    /// What colours sprites on the map (design §6.3).
    pub fn colour_mode(&self) -> ColourMode {
        self.colour_mode
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

    /// The tile the pointer on screen cell `cell` points at (design v27 §6.5):
    /// the one up and to the left of the tile under it, so the pointer's
    /// arrow rests on the Cursor's corner rather than hiding it. On the map
    /// view's top row it stays in that row, and on its left column in that
    /// column; and the border just right of or below the tiles points at the
    /// last column or row, so every tile in view can be pointed at.
    pub fn pointed_at(&self, cell: Position) -> Option<Pos> {
        let area = self.tile_area;
        let reach = Rect::new(
            area.x,
            area.y,
            area.width.saturating_add(1),
            area.height.saturating_add(1),
        );
        if area.is_empty() || !reach.contains(cell) {
            return None;
        }
        self.tile_at(Position::new(
            cell.x.saturating_sub(1).max(area.x),
            cell.y.saturating_sub(1).max(area.y),
        ))
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
        self.event_log_area = areas.event_log;
        self.overlay = areas.overlay;
        self.settle();
    }

    /// Carries out an action on `world`, and says whether the game carries on.
    pub fn apply(&mut self, action: Action, world: &World) -> Flow {
        match self.screen {
            Screen::PlaceMenu | Screen::GenomeMenu | Screen::LoadMenu => {
                return self.apply_in_menu(action, world);
            }
            Screen::Naming => return self.apply_naming(action, world),
            Screen::SaveNaming => return self.apply_save_naming(action, world),
            Screen::LoadPrompt => return self.apply_load_prompt(action),
            Screen::Help => return self.apply_in_help(action),
            Screen::SpriteList => return self.apply_in_list(action, world),
            Screen::Normal | Screen::QuitPrompt => {}
        }
        if self.screen == Screen::QuitPrompt {
            match action {
                // Only `y` answers it: `Esc` never quits on its own (design
                // v33 §6.6).
                Action::Confirm | Action::Quit => return Flow::Quit,
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
            // Scrolling by hand takes the view back from Track.
            Action::Scroll { dx, dy } => {
                self.tracking = false;
                self.scroll(dx, dy);
            }
            Action::Point(cell) => self.point(cell),
            // A click on the event log's filters goes to the next (design
            // §6.1).
            Action::Click {
                at,
                button: Button::Left,
                ..
            } if self.filter_label().is_some_and(|label| label.contains(at)) => {
                self.next_filter();
            }
            Action::Click {
                at,
                button,
                amplified,
            } => {
                self.point(at);
                // A click lands where the Cursor is: not past the leash
                // (design v23 §6.5).
                if let Some(tile) = self.pointed_at(at) {
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
                if at.is_none_or(|at| self.pointed_at(at).is_some()) {
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
            Action::Quicksave => self.save_as(QUICKSAVE, world),
            Action::Quickload => self.quickload(world),
            Action::SaveAs => self.start_save_naming(world),
            Action::OpenSaves => self.list_saves(),
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
                } else if self.pointed_at(at).is_some() {
                    self.mode = self.mode.along(notches);
                }
            }
            Action::ToggleDetail => self.detail = !self.detail,
            Action::Track => self.toggle_tracking(world),
            Action::CycleEventFilter => self.next_filter(),
            Action::Help => {
                self.end_aim();
                self.screen = Screen::Help;
            }
            Action::SpriteList => {
                self.end_aim();
                self.open_list(world);
            }
            Action::CycleColours => {
                self.colour_mode = along(&ColourMode::ALL, self.colour_mode, 1);
                self.tell_player(format!("Colours: {}", self.colour_mode.label()));
            }
            Action::ToggleVisible => {
                if !self.visible_in.remove(&self.mode) {
                    self.visible_in.insert(self.mode);
                }
            }
            // Esc cancels an aim first (design v25 §6.5); then lets go of
            // what the Cursor holds or leads, a Place menu item waiting on it
            // first, in any mode (design v33 §6.5); then returns to Select;
            // from Select it asks to quit (design v21 §6.5).
            Action::Back if self.aim.is_some() => self.end_aim(),
            Action::Back if self.placing.is_some() => self.placing = None,
            Action::Back if self.grip(world).is_some() => self.let_go(world),
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
            // where the Cursor is: at the followed sprite's feet, as a held
            // item is put down, or else on the tile clicked. A right click
            // puts it away (design v28 §6.5).
            (CursorMode::Grab, Button::Left) if self.placing.is_some() => {
                let tile = if self.followed().is_some() {
                    self.cursor
                } else {
                    tile
                };
                self.place(tile);
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

    /// Lets go of what the Cursor has hold of, where the Cursor is, as `Esc`
    /// does (design v33 §6.5): a led sprite is let go, and a held item put
    /// down on the Cursor's tile, as a click there would.
    fn let_go(&mut self, world: &World) {
        match self.grip(world) {
            Some(Grip::Leads(_)) => self.send(Command::LetGo, world),
            Some(Grip::Holds(_)) => self.send(Command::PutDown { tile: self.cursor }, world),
            None => {}
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

    /// What an action does while the help screen is open: `?` or `Esc`
    /// closes it, and the mouse still points.
    fn apply_in_help(&mut self, action: Action) -> Flow {
        match action {
            Action::Help | Action::Back => self.screen = Screen::Normal,
            Action::Quit => return Flow::Quit,
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }

    /// Opens the sprite list, highlighting the selected sprite if it's
    /// listed, or else the first.
    fn open_list(&mut self, world: &World) {
        self.screen = Screen::SpriteList;
        let listed = sprite_list::order(self, world);
        self.list_choice = self
            .selection
            .map(Selection::id)
            .filter(|id| listed.contains(id))
            .or_else(|| listed.first().copied());
    }

    /// What an action does while the sprite list is open (design §6.1): the
    /// arrow keys or the wheel move the highlight, `Tab` changes the order,
    /// `Enter` or a click on a row goes to that sprite, and `l` or `Esc`
    /// closes the list.
    fn apply_in_list(&mut self, action: Action, world: &World) -> Flow {
        let ids = sprite_list::order(self, world);
        let choice = self.list_row(world) as i32;
        let moved = |by: i32| {
            let last = ids.len().max(1) as i32 - 1;
            ids.get((choice + by).clamp(0, last) as usize).copied()
        };
        match action {
            Action::Scroll { dy, .. } => self.list_choice = moved(dy.signum()),
            Action::Wheel { at, notches } => {
                self.point(at);
                self.list_choice = moved(notches);
            }
            Action::SelectNext => self.list_sort = along(&SortBy::ALL, self.list_sort, 1),
            Action::SelectPrevious => self.list_sort = along(&SortBy::ALL, self.list_sort, -1),
            Action::Enter => {
                if let Some(&id) = ids.get(self.list_row(world)) {
                    self.go_to(id, world);
                }
            }
            Action::Click {
                at,
                button: Button::Left,
                ..
            } => {
                self.point(at);
                let row = self.list_row_at(at);
                if let Some(&id) = row.and_then(|row| ids.get(self.list_first(world) + row)) {
                    self.go_to(id, world);
                }
            }
            Action::Point(cell) => self.point(cell),
            Action::SpriteList | Action::Back => self.screen = Screen::Normal,
            Action::Quit => return Flow::Quit,
            _ => {}
        }
        Flow::Continue
    }

    /// Selects sprite `id`, centres the view on it and closes the sprite
    /// list.
    fn go_to(&mut self, id: EntityId, world: &World) {
        self.screen = Screen::Normal;
        self.select(id);
        if let Some(sprite) = world.sprite(id) {
            self.centre_on(sprite.pos());
        }
        self.settle_cursor(world);
    }

    /// Which of the sprite list's rows `at` is on, from 0 for the first
    /// shown, if it's on one.
    fn list_row_at(&self, at: Position) -> Option<usize> {
        let inner = self.overlay?.inner(Margin::new(1, 1));
        // The heading takes the first row.
        let rows = Rect::new(
            inner.x,
            inner.y + 1,
            inner.width,
            inner.height.saturating_sub(1),
        );
        rows.contains(at).then(|| usize::from(at.y - rows.y))
    }

    /// What the sprite list is sorted by.
    pub fn list_sort(&self) -> SortBy {
        self.list_sort
    }

    /// Which row of the sprite list is highlighted, from 0, in its current
    /// order.
    pub fn list_row(&self, world: &World) -> usize {
        let ids = sprite_list::order(self, world);
        self.list_choice
            .and_then(|chosen| ids.iter().position(|&id| id == chosen))
            .unwrap_or(0)
    }

    /// How many sprites the sprite list shows at once: the rows inside its
    /// border, less the heading.
    pub fn list_rows(&self) -> usize {
        self.overlay.map_or(0, |area| {
            usize::from(area.inner(Margin::new(1, 1)).height.saturating_sub(1))
        })
    }

    /// The first row the sprite list shows: it scrolls so the highlighted
    /// sprite stays in view.
    pub fn list_first(&self, world: &World) -> usize {
        self.list_row(world)
            .saturating_sub(self.list_rows().max(1) - 1)
    }

    /// Sets the game's folder for its files (design §6.7), which the help
    /// screen shows.
    pub fn set_data_folder(&mut self, folder: PathBuf) {
        self.data_folder = Some(folder);
    }

    /// The game's folder for its files, if there is one.
    pub fn data_folder(&self) -> Option<&Path> {
        self.data_folder.as_deref()
    }

    /// Where the help screen and the sprite list are drawn, if the screen
    /// has room for the game.
    pub fn overlay(&self) -> Option<Rect> {
        self.overlay
    }

    /// Sets where genome files are saved and read from (design §6.7).
    pub fn set_genome_folder(&mut self, folder: PathBuf) {
        self.genome_folder = Some(folder);
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
            // A file put there by hand may have a name the screen can't
            // show (design §7.2).
            Screen::LoadMenu => self
                .save_files
                .iter()
                .map(|file| {
                    file.name
                        .chars()
                        .map(|c| if cp437::contains(c) { c } else { '?' })
                        .collect()
                })
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
            // Numbers count from 1.
            Action::Pick(n) if (1..=count).contains(&usize::from(n)) => {
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
                let entry = entry.ok()?;
                let path = entry.path();
                let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
                let is_ron = path.extension().is_some_and(|ext| ext == "ron");
                let name = path.file_stem()?.to_string_lossy().into_owned();
                (is_file && is_ron).then_some((name, path))
            })
            .collect();
        files.sort();
        self.genome_files = files;
        self.open_menu(Screen::GenomeMenu);
    }

    /// Places the item waiting on the Cursor on `tile` (design v28 §6.5).
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

    /// Whether keys type letters, rather than act: while naming a sprite or
    /// a save.
    pub fn typing(&self) -> bool {
        matches!(self.screen, Screen::Naming | Screen::SaveNaming)
    }

    /// The name being typed, while naming (design §6.5).
    pub fn name_draft(&self) -> Option<&str> {
        self.naming
            .as_ref()
            .map(|naming| naming.draft.text.as_str())
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
            draft: Draft::offered(draft),
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
            // Only letters CP437 can show, and spaces between them (design v28 §6.5).
            Action::Type(c) if (c.is_alphabetic() || c == ' ') && cp437::contains(c) => {
                naming.draft.type_char(c, MAX_NAME_CHARS);
            }
            Action::Erase => naming.draft.erase(),
            Action::AnotherName => {
                let sprite = naming.sprite;
                let draft = self.random_name(sprite, world);
                let naming = self.naming.as_mut().expect("naming");
                naming.draft = Draft::offered(draft);
            }
            Action::Enter => {
                let naming = self.naming.take().expect("naming");
                self.screen = Screen::Normal;
                if naming.draft.text.trim_matches(' ').is_empty() {
                    self.refuse("A name needs a letter in it".into());
                } else {
                    self.commands.push(Command::Rename {
                        sprite: naming.sprite,
                        name: naming.draft.text,
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

    /// Sets where saves go (design §6.7).
    pub fn set_save_folder(&mut self, folder: PathBuf) {
        self.save_folder = Some(folder);
    }

    /// Where saves go, if anywhere.
    pub fn save_folder(&self) -> Option<&Path> {
        self.save_folder.as_deref()
    }

    /// Saves the world as `name` in the saves folder (design §6.7), and says
    /// so, or why it couldn't.
    fn save_as(&mut self, name: &str, world: &World) {
        let Some(folder) = self.save_folder.clone() else {
            return self.refuse("There's no folder to save in".into());
        };
        match saves::write(&folder, name, &self.save_of(world)) {
            Ok(_) => {
                self.saved_now(world);
                self.tell_player(format!("Saved as {}", saves::name_for(name)));
            }
            Err(err) => self.refuse(format!("Couldn't save: {err}")),
        }
    }

    /// `world`'s save, with the tile at the middle of the view, so a load
    /// shows what the player saw (design §6.7).
    fn save_of(&self, world: &World) -> Vec<u8> {
        let area = self.tile_area;
        let middle = Pos {
            x: (self.viewport.x + area.width / 2).min(self.map_size.width - 1),
            y: (self.viewport.y + area.height / 2).min(self.map_size.height - 1),
        };
        world.save_with_view(middle)
    }

    /// Notes that the world, as it stands, has just been saved or loaded.
    fn saved_now(&mut self, world: &World) {
        self.saved = Some((world.tick(), self.running_for));
        self.unsaved_run = Duration::ZERO;
    }

    /// Whether the world has run since it was last saved or loaded, or, if
    /// it never was, since it began: whether a load would lose anything.
    fn has_run_since_save(&self, world: &World) -> bool {
        world.tick() != self.saved.map_or(0, |(tick, _)| tick)
    }

    /// When the world was last saved or loaded, as real time since, or
    /// `None` if it never has been: for the top bar (design §6.1).
    pub fn saved_ago(&self) -> Option<Duration> {
        self.saved.map(|(_, at)| self.running_for - at)
    }

    /// Autosaves if time has run, unpaused, for `AUTOSAVE_EVERY` since the
    /// last autosave (design §6.7). The frame loop calls it every frame.
    pub fn autosave_if_due(&mut self, world: &World) {
        if self.since_autosave >= AUTOSAVE_EVERY {
            self.autosave(world);
        }
    }

    /// Saves the world as the newest autosave (design §6.7), as on quitting,
    /// unless it hasn't run since it was last saved: then the autosaves
    /// already hold it, or a save does, and an older autosave stays.
    pub fn autosave(&mut self, world: &World) {
        self.since_autosave = Duration::ZERO;
        if !self.has_run_since_save(world) {
            return;
        }
        let Some(folder) = self.save_folder.clone() else {
            return;
        };
        match saves::autosave(&folder, &self.save_of(world)) {
            Ok(_) => {
                self.saved_now(world);
                self.tell_player("Autosaved".into());
            }
            Err(err) => self.refuse(format!("Couldn't autosave: {err}")),
        }
    }

    /// `Ctrl+S`: starts naming a save, offering the seed and the tick
    /// (design §6.7).
    fn start_save_naming(&mut self, world: &World) {
        self.end_aim();
        let offered = format!("seed {} tick {}", world.seed(), world.tick());
        self.save_naming = Some(Draft::offered(offered));
        self.screen = Screen::SaveNaming;
    }

    /// The save's name being typed, while naming one.
    pub fn save_name_draft(&self) -> Option<&str> {
        self.save_naming.as_ref().map(|draft| draft.text.as_str())
    }

    /// What an action does while naming a save: letters type, the first
    /// replacing the name offered; `Backspace` rubs one out; `Enter` saves;
    /// `Esc` gives up.
    fn apply_save_naming(&mut self, action: Action, world: &World) -> Flow {
        let Some(draft) = self.save_naming.as_mut() else {
            self.screen = Screen::Normal;
            return Flow::Continue;
        };
        match action {
            // Only what the screen can show (design §7.2).
            Action::Type(c) if !c.is_control() && cp437::contains(c) => {
                draft.type_char(c, MAX_SAVE_NAME_CHARS);
            }
            Action::Erase => draft.erase(),
            Action::Enter => {
                let draft = self.save_naming.take().expect("naming a save").text;
                self.screen = Screen::Normal;
                if draft.trim().is_empty() {
                    self.refuse("A save needs a name".into());
                } else {
                    self.save_as(&draft, world);
                }
            }
            Action::Back => {
                self.save_naming = None;
                self.screen = Screen::Normal;
            }
            Action::Quit => return Flow::Quit,
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }

    /// `F9`: loads the quicksave, if there is one (design §6.7).
    fn quickload(&mut self, world: &World) {
        let Some(folder) = self.save_folder.clone() else {
            return self.refuse("There's no folder to load from".into());
        };
        let path = saves::path_for(&folder, QUICKSAVE);
        if !path.is_file() {
            return self.refuse("There's no quicksave yet: F5 makes one".into());
        }
        let file = SaveFile {
            name: QUICKSAVE.into(),
            path,
            modified: None,
        };
        self.ask_to_load(file, world);
    }

    /// `Ctrl+O`: opens the list of saves to load, newest first (design
    /// §6.7).
    fn list_saves(&mut self) {
        self.end_aim();
        self.save_files = self
            .save_folder
            .as_deref()
            .map(saves::list)
            .unwrap_or_default();
        self.open_menu(Screen::LoadMenu);
    }

    /// What the load menu says with no saves to list.
    pub fn no_saves(&self) -> String {
        match &self.save_folder {
            Some(folder) => format!("No saves in {}", folder.display()),
            None => "No folder for saves".into(),
        }
    }

    /// Loads `file`, first asking if the world has run since it was last
    /// saved, as what has happened since would be lost (design §6.7).
    fn ask_to_load(&mut self, file: SaveFile, world: &World) {
        if !self.has_run_since_save(world) {
            return self.load(file);
        }
        self.to_load = Some(file);
        self.screen = Screen::LoadPrompt;
    }

    /// The question before loading, while it waits for an answer.
    pub fn load_question(&self) -> Option<String> {
        let file = self.to_load.as_ref()?;
        let since = if self.saved.is_some() {
            format!(
                "The world has run {} since it was last saved",
                spoken_duration(self.unsaved_run)
            )
        } else {
            "This world has never been saved".into()
        };
        Some(format!("Load {}? {since} (y/n)", file.name))
    }

    /// What an action does while the question before loading waits: `y`
    /// loads, as does `F9` when the question is the quicksave's; the mouse
    /// carries on as usual; any other key cancels.
    fn apply_load_prompt(&mut self, action: Action) -> Flow {
        let quicksave = self
            .to_load
            .as_ref()
            .is_some_and(|file| file.name == QUICKSAVE);
        let yes = action == Action::Confirm || (action == Action::Quickload && quicksave);
        match action {
            _ if yes => {
                self.screen = Screen::Normal;
                if let Some(file) = self.to_load.take() {
                    self.load(file);
                }
            }
            Action::Quit => return Flow::Quit,
            Action::Point(cell) => self.point(cell),
            Action::Click { .. }
            | Action::Follow { at: Some(_) }
            | Action::Wheel { .. }
            | Action::Release { .. } => {}
            _ => {
                self.to_load = None;
                self.screen = Screen::Normal;
            }
        }
        Flow::Continue
    }

    /// Reads `file` and loads the world in it, for the frame loop to take,
    /// or says on the status line why it couldn't (design §2.9).
    fn load(&mut self, file: SaveFile) {
        let loaded = std::fs::read(&file.path)
            .map_err(|err| err.to_string())
            .and_then(|bytes| World::load(&bytes).map_err(|err| err.to_string()));
        match loaded {
            Ok(world) => self.loaded = Some((world, file.name)),
            Err(why) => self.refuse(format!("Couldn't load {}: {why}", file.name)),
        }
    }

    /// The world just loaded, if one was, for the frame loop to play from
    /// now on. The app starts afresh on it, as for a new world, paused so
    /// the player can see where they are (design §6.7). What's the
    /// player's rather than the world's carries over: the speed, the theme,
    /// the colours, the open tab and the folders.
    pub fn take_loaded(&mut self) -> Option<World> {
        let (world, name) = self.loaded.take()?;
        let areas = Areas {
            tiles: self.tile_area,
            inspector: self.inspector,
            event_log: self.event_log_area,
            overlay: self.overlay,
        };
        let mut fresh = App::new(world.map(), self.theme.clone(), world.seed(), areas);
        fresh.clock = std::mem::take(&mut self.clock);
        fresh.clock.pause();
        fresh.policy = std::mem::replace(&mut self.policy, Box::new(Omniscient));
        fresh.colour_mode = self.colour_mode;
        fresh.event_filter = self.event_filter;
        fresh.list_sort = self.list_sort;
        fresh.tab = self.tab;
        fresh.running_for = self.running_for;
        fresh.data_folder = self.data_folder.take();
        fresh.genome_folder = self.genome_folder.take();
        fresh.save_folder = self.save_folder.take();
        // The Cursor as the world left it: where it was, what it has hold
        // of, in Grab mode, which holds and leads, and whether sprites can
        // see it.
        let cursor = world.cursor();
        if cursor.leads().is_some() || cursor.holds().is_some() {
            fresh.mode = CursorMode::Grab;
        }
        if let Some(tile) = cursor.tile() {
            fresh.cursor = tile;
            fresh.pointed = tile;
            fresh.told_tile = Some(tile);
            fresh.centre_on(tile);
        }
        // The view as the player left it, if the save kept it.
        if let Some(view) = world.view() {
            fresh.centre_on(view);
        }
        if cursor.visible() {
            fresh.visible_in.insert(fresh.mode);
        }
        fresh.told_visible = cursor.visible();
        fresh.saved_now(&world);
        fresh.tell_player(format!(
            "Loaded {name}, at tick {}",
            group_thousands(world.tick())
        ));
        *self = fresh;
        self.settle_cursor(&world);
        Some(world)
    }

    /// Switches the event log to the next filter (design §6.1).
    fn next_filter(&mut self) {
        self.event_filter = along(&EventFilter::ALL, self.event_filter, 1);
    }

    /// `T`: the view follows the selected sprite, or stops (design v21
    /// §6.1). With none selected, it's refused.
    fn toggle_tracking(&mut self, world: &World) {
        if self.tracking {
            self.tracking = false;
            return self.tell_player("Stopped tracking".into());
        }
        let Some(Selection::Living(id)) = self.selection else {
            return self.refuse("Select a sprite to track it".into());
        };
        self.tracking = true;
        self.tell_player(format!("Tracking {}", self.names.label(id)));
        self.track_selected(world);
    }

    /// Whether the view follows the selected sprite (design v21 §6.1).
    pub fn tracking(&self) -> bool {
        self.tracking
    }

    /// While tracking, centres the view on the selected sprite, as far as
    /// the wall allows.
    fn track_selected(&mut self, world: &World) {
        let tracked = match self.selection {
            Some(Selection::Living(id)) if self.tracking => world.sprite(id),
            _ => None,
        };
        if let Some(sprite) = tracked {
            self.centre_on(sprite.pos());
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

    /// Keeps the viewport within the wall, and the cursor on whatever tile a
    /// still pointer points at.
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

    /// Notes the tile the pointer on screen cell `cell` points at as the
    /// pointer's, and puts the Cursor there, within the leash, unless it
    /// follows a sprite. Where it points at no tile, both stay on their last
    /// tile.
    fn point(&mut self, cell: Position) {
        match self.pointed_at(cell) {
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

/// A length of real time as the player reads it: "under a minute", "3m",
/// "1h 5m".
pub(crate) fn spoken_duration(time: Duration) -> String {
    let minutes = time.as_secs() / 60;
    match (minutes / 60, minutes % 60) {
        (0, 0) => "under a minute".into(),
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}
