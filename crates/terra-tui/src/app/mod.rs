//! The UI state (design §6.8): everything the screen shows that isn't the world.
//!
//! `App` and its state are here, with `apply`, which carries out each
//! action. Each area's methods are in a module of its own: the Cursor,
//! aiming, the map view, the selection, what each tick brings, what shows
//! for a while, the menus, the sprite list, the population graph, the
//! family tree, naming, saves and replays.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::time::Duration;

use ratatui::layout::{Position, Rect, Size};
use serde::Deserialize;
use terra_sim::{
    ChemicalKind, ChemicalLevel, Command, DeathCause, Divergence, EntityId, Event, Genome, Map,
    Mode, Playback, Pos, SpriteView, Thing, World,
};

use crate::clock::{Clock, Speed};
use crate::cp437;
use crate::debug_overlay::{DebugOverlay, FrameTimes};
use crate::family_tree;
use crate::input::{Action, Button, KeyReports};
use crate::inspector;
use crate::listed::listed_enum;
use crate::policy::{InfoPolicy, Omniscient, Panel, Subject};
use crate::saves::{QUICKSAVE, SaveFile};
use crate::space::Place;
use crate::spots::{Spot, Spots};
use crate::sprite_list::SortBy;
use crate::text::{Names, Words};
use crate::theme::{Emote, Theme};

mod aim;
mod cursor;
mod events;
mod help;
mod list;
mod menus;
mod naming;
mod population;
mod replay;
mod saving;
mod selection;
mod spots;
mod status;
mod tree;
mod view;

pub(crate) use cursor::{ItemLook, ShownItems, item_look};
pub(crate) use help::finds;
pub(crate) use spots::in_the_world;

/// Whether the game carries on after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
    /// Leave the world for the title screen (M2 design §8.4).
    Title,
}

/// What fills the screen besides the map (design §6.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Normal,
    /// "Quit? y quit the game  t title screen …" is waiting for an answer
    /// (M2 design §8.4).
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
    /// The list of themes is open (`Ctrl+T`, design v34 §6.7).
    ThemeMenu,
    /// The help screen is open (design §6.1).
    Help,
    /// The sprite list is open (design §6.1).
    SpriteList,
    /// The population graph is open (M2 design §14).
    Population,
    /// The family tree is open (M2 design §17.2).
    FamilyTree,
    /// The command palette is open (#303).
    Palette,
}

listed_enum! {
    /// What colours sprites on the map (design §6.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ColourMode {
        /// The colour of its strongest drive above .5, or its own colour if
        /// none is.
        Drive => "strongest drive",
        /// Its own colour.
        Plain => "plain",
        /// The selected sprite's parents and grandparents, brothers and
        /// sisters, and young and theirs each in their colour, and the rest
        /// dark gray (M2 design §17.4).
        Lineage => "lineage",
    }

    /// Every colour mode, in the order `b` goes through them.
    pub const ALL;

    /// What the status line calls it when `b` switches to it.
    pub fn label(self) -> &'static str;
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

listed_enum! {
    /// Which events the event log shows (design §6.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EventFilter {
        /// Every event it logs.
        All => "all",
        /// What the selected sprite did, or had done to it.
        Selected => "selected",
        /// Deaths, lessons learned and refusals.
        Major => "major",
    }

    /// Every filter, in the order `m` goes through them.
    pub const ALL;

    /// Its name on the event log's title row.
    pub fn label(self) -> &'static str;
}

impl EventFilter {
    /// The filters on the event log's title row: every filter, the one in
    /// use in brackets.
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

/// A file's name as the screen can show it: a file put in a folder by hand
/// may have a name with characters outside CP437 (design §7.2). The title
/// screen shows saves' names so too (M2 design §8.2).
pub(crate) fn shown(name: &str) -> String {
    name.chars()
        .map(|c| if cp437::contains(c) { c } else { '?' })
        .collect()
}

listed_enum! {
    /// What a click on the map does (design v21 §6.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum CursorMode {
        Select => "SELECT",
        /// Teaching: a left click rewards, a right click corrects.
        Train => "TRAIN",
        /// Moving things: a left click grabs, or lets go (design v23 §6.5).
        Grab => "GRAB",
    }

    /// Every cursor mode. Each theme must give all of them a mark.
    pub const ALL;

    /// The mode's name on the status line.
    pub fn label(self) -> &'static str;
}

impl CursorMode {
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
    /// A sprite in the world, or an egg, which the sprite it hatches into
    /// goes on being (M2 design §10.7).
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

listed_enum! {
    /// An inspector tab (design §6.1). The Brain tab joins with the brain.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Tab {
        Body => "Body",
        Brain => "Brain",
        Chem => "Chem",
        Genome => "Genome",
        World => "World",
    }

    /// Every tab, in the order `[` and `]` go through them.
    pub const ALL;

    /// The tab's name in the inspector's title.
    pub fn label(self) -> &'static str;
}

impl Tab {
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

/// Where a highlight on row `index` of a list of `count` rows goes, moved
/// `by` rows, down being positive: it stops at either end. The arrow keys
/// move it a row, and the wheel a row a notch (#128), in every list and
/// menu, the title screen's too (M2 design §8.2).
pub(crate) fn highlight_moved(index: usize, by: i32, count: usize) -> usize {
    let last = count.saturating_sub(1) as i64;
    (index as i64 + i64::from(by)).clamp(0, last) as usize
}

/// Where on screen the panels the app works with are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Areas {
    /// The map view's screen cells, where it draws the map's tiles and
    /// what's past its wall.
    pub tiles: Rect,
    /// The inspector, its title row included, if the screen has room for
    /// it.
    pub inspector: Option<Rect>,
    /// The event log, its title row included, if the screen has room for
    /// it.
    pub event_log: Option<Rect>,
    /// Where the help screen and the sprite list are drawn: between the top
    /// bar and the status line, if the screen has room for the game.
    pub overlay: Option<Rect>,
}

/// Where the viewport starts: the tile at its top-left, counted from the
/// map's top-left corner. It can be past the wall, so negative on the left
/// or the top (M2 design §11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub x: i32,
    pub y: i32,
}

/// How many lines a notch of the mouse wheel scrolls an inspector tab.
const WHEEL_LINES: i32 = 3;

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
        self.note(events, world);
    }

    /// Notes what the screen shows of a tick just run: its `events`, and
    /// how `world` stands after it.
    pub fn note(&mut self, events: Vec<Event>, world: &World) {
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
    /// Where the viewport starts, which can be past the wall.
    viewport: Origin,
    /// The map's size, in tiles.
    map_size: Size,
    /// The map view's screen cells, where it draws the map and what's past
    /// its wall.
    tile_area: Rect,
    /// The inspector, its title row included, if the screen has room for
    /// it.
    inspector: Option<Rect>,
    /// The screen cell under the mouse pointer, while it's over the map
    /// view.
    pointer: Option<Position>,
    /// Whether the pointer points past the wall, at the glass or space,
    /// rather than at a tile or off the map view (M2 design §11.4).
    pointer_past_wall: bool,
    /// How far the stars have drifted, as time at 1× (M2 design §11.3).
    drift: Duration,
    /// The latest events the event log shows, newest first, each with how
    /// many times in a row it came (design v21 §6.1).
    event_log: VecDeque<(Event, u32)>,
    /// The selected sprite's observed list, newest first.
    observed: VecDeque<Observed>,
    selection: Option<Selection>,
    tab: Tab,
    /// How many lines the open tab is scrolled down.
    tab_scroll: usize,
    /// Memory blocks the player has opened on the Brain tab (design v40
    /// §6.1). Shut when another sprite is selected. Not part of the world.
    open_lessons: BTreeSet<inspector::MemoryOpen>,
    /// The last screen cell the pointer was on, including the inspector,
    /// so Enter can open the lesson under it.
    mouse: Option<Position>,
    /// Whether a key has been pressed since the pointer last moved, which
    /// puts its tooltip away until it moves again (design §6.1, #304).
    keyed: bool,
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
    /// says in the key hints' place, the sprites it names, and since when
    /// (design v22 §6.1).
    refusal: Option<(String, Vec<EntityId>, Duration)>,
    /// The names the player has given sprites, as last seen, so the log
    /// keeps naming a sprite after it dies (design §6.5).
    names: Names,
    /// The genes copying changed in what they do in each hatchling the log
    /// tells of, as its hatching line names them (M2 design §13.2), kept
    /// while the line is in the log.
    news: BTreeMap<EntityId, String>,
    /// The Place menu item waiting on the Cursor (design v28 §6.5).
    placing: Option<Placing>,
    /// The highlighted item of the open menu, from 0.
    menu_choice: usize,
    /// Where genome files are saved and read from (design §6.7), if
    /// anywhere.
    genome_folder: Option<PathBuf>,
    /// The genome files the genome menu lists, by name, with their paths.
    genome_files: Vec<(String, PathBuf)>,
    /// The folder the player's own themes are read from, if there is one
    /// (design v34 §6.7).
    theme_folder: Option<PathBuf>,
    /// The theme files the themes menu lists after the built-in themes, by
    /// name, with their paths.
    theme_files: Vec<(String, PathBuf)>,
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
    /// The event log, its title row included, if the screen has room for
    /// it.
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
    /// While the sprite list is filtered, what's typed into its filter
    /// (#313).
    list_filter: Option<Draft>,
    /// The moment the population graph reads, the end of the stretch of
    /// time picked, if one is; otherwise it reads now (M2 design §14.3).
    graph_pick: Option<u64>,
    /// The card the family tree's arrows are on, if not the one in the
    /// middle (M2 design §17.3).
    tree_choice: Option<family_tree::Seat>,
    /// How far along the family tree's rows that scroll are.
    tree_scroll: family_tree::Scroll,
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
    /// While a replay plays back (design §2.7).
    replay: Option<Replay>,
    /// Whether the player has just taken over the replay, for the frame
    /// loop to take (`take_taken_over`).
    taken_over: bool,
    /// The debug overlay (`F3`, design §6.1, #311).
    debug: DebugOverlay,
    /// The help screen's page and search (#303), while it's open or as it
    /// was last.
    help: help::HelpView,
    /// The command palette, while it's open (#303).
    palette: Option<help::Palette>,
    /// Whether the player last pointed at the inspector or used its keys,
    /// rather than the map's, so help opens at the inspector's page (#303).
    on_inspector: bool,
    /// The spots of the frame last drawn (design §6.1), which drawing replaces each
    /// frame: what the pointer is over, and what a click lands on.
    spots: RefCell<Spots>,
}

/// A replay as the screen follows it (design §2.7).
#[derive(Debug, Clone, Copy)]
struct Replay {
    /// The tick the recording reached.
    end: u64,
    /// Whether playback has reached `end`, and paused there.
    ended: bool,
    /// Where the world first parted from the recording, once it has.
    divergence: Option<Divergence>,
}

/// When the frame loop stops running a replay's ticks (`App::replay_stops`):
/// at the recording's end, the first time it's reached, and where playback
/// first finds the world different from the recording.
#[derive(Debug, Clone, Copy)]
pub struct ReplayStops {
    end: Option<u64>,
    divergence: bool,
}

impl ReplayStops {
    /// Whether `playback` has just reached a stop.
    pub fn at(self, playback: &Playback) -> bool {
        self.end.is_some_and(|end| playback.world().tick() >= end)
            || (self.divergence && playback.divergence().is_some())
    }
}

/// What the status line says when the player tries to change a replay.
pub const REPLAY_REFUSAL: &str =
    "It's a replay: only time, the view and the inspector work, until Ctrl+R takes it over";

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
/// the one offered, which the first letter typed replaces. The title
/// screen's seed is typed so too (M2 design §8.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Draft {
    text: String,
    typed: bool,
}

impl Draft {
    /// `text`, offered.
    pub(crate) fn offered(text: String) -> Draft {
        Draft { text, typed: false }
    }

    /// What's there so far.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// Whether the player has typed it, rather than it being the one
    /// offered.
    pub(crate) fn typed(&self) -> bool {
        self.typed
    }

    /// Types `c`, replacing the name offered, up to `most` characters.
    pub(crate) fn type_char(&mut self, c: char, most: usize) {
        if !self.typed {
            self.text.clear();
            self.typed = true;
        }
        if self.text.chars().count() < most {
            self.text.push(c);
        }
    }

    /// Rubs out the last character.
    pub(crate) fn erase(&mut self) {
        self.text.pop();
        self.typed = true;
    }
}

/// Whether `c` can be typed into a save's name or the sprite list's filter
/// (#313): a character the screen can show (design §7.2), not a control one.
fn typeable(c: char) -> bool {
    !c.is_control() && cp437::contains(c)
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
    /// Where the pointer pulls to, in the map's tile coordinates, which
    /// run on past the wall: the pull goes on over the glass and space
    /// (M2 design §11.4).
    to: (i32, i32),
}

/// How long an emote lasts, in real time, whatever the speed (design §6.3).
const EMOTE_FOR: Duration = Duration::from_secs(1);

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
            viewport: Origin { x: 0, y: 0 },
            map_size: Size::new(map.width(), map.height()),
            tile_area: areas.tiles,
            inspector: areas.inspector,
            pointer: None,
            pointer_past_wall: false,
            drift: Duration::ZERO,
            event_log: VecDeque::new(),
            observed: VecDeque::new(),
            selection: None,
            tab: Tab::World,
            tab_scroll: 0,
            open_lessons: BTreeSet::new(),
            mouse: None,
            keyed: false,
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
            news: BTreeMap::new(),
            placing: None,
            menu_choice: 0,
            genome_folder: None,
            genome_files: Vec::new(),
            theme_folder: None,
            theme_files: Vec::new(),
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
            list_filter: None,
            graph_pick: None,
            tree_choice: None,
            tree_scroll: family_tree::Scroll::default(),
            data_folder: None,
            save_folder: None,
            save_files: Vec::new(),
            save_naming: None,
            to_load: None,
            loaded: None,
            saved: None,
            unsaved_run: Duration::ZERO,
            since_autosave: Duration::ZERO,
            replay: None,
            taken_over: false,
            debug: DebugOverlay::default(),
            help: help::HelpView::default(),
            palette: None,
            on_inspector: false,
            spots: RefCell::default(),
        };
        app.centre_on(cursor);
        app
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
            news: &self.news,
        }
    }

    /// Takes the commands the player's clicks have made since last taken,
    /// in the order made, for the world to apply at its next tick.
    pub fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }

    /// Moves the app's real-time clock on by `elapsed`, for what flashes,
    /// and the stars with the game's speed (M2 design §11.3).
    pub fn animate(&mut self, elapsed: Duration) {
        self.running_for += elapsed;
        self.drift += self.clock.drift(elapsed);
        if !self.clock.is_paused() {
            self.unsaved_run += elapsed;
            self.since_autosave += elapsed;
        }
        let now = self.running_for;
        self.emotes.retain(|_, &mut (_, at)| now - at < EMOTE_FOR);
        self.resting
            .retain(|_, rest| rest.until().is_none_or(|until| now < until));
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

    /// Whether the debug overlay is shown (`F3`, design §6.1, #311).
    pub fn debug_overlay(&self) -> bool {
        self.debug.shown()
    }

    /// Takes in what the frame loop measured of a frame, and what the
    /// terminal reports of keys, for the debug overlay (design §6.1, §6.6).
    /// Noted whether it's shown or not, so it has numbers as soon as it is.
    pub fn note_frame(&mut self, times: FrameTimes, keys: KeyReports) {
        self.debug.note(times, keys);
    }

    /// The debug overlay's lines, inside its border.
    pub(crate) fn debug_lines(&self) -> Vec<String> {
        self.debug.lines(&self.clock)
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn mode(&self) -> CursorMode {
        self.mode
    }

    /// Carries out an action on `world`, and says whether the game carries on.
    pub fn apply(&mut self, action: Action, world: &World) -> Flow {
        // Where the pointer is, whatever's open, and whether a key has put
        // its tooltip away since (design §6.1, #304).
        match action.pointer_cell() {
            Some(cell) => {
                self.mouse = Some(cell);
                self.keyed = false;
            }
            None if !action.is_pointer() => self.keyed = true,
            None => {}
        }
        // `F3` shows or hides the debug overlay over whatever's open, and
        // does nothing else: it answers no prompt and never touches the
        // world (design §6.1, #311).
        if action == Action::DebugOverlay {
            self.debug.toggle();
            return Flow::Continue;
        }
        // `Ctrl+C` quits at once, whatever is open, as on the title screen
        // (design §6.6).
        if action == Action::Quit {
            return Flow::Quit;
        }
        match self.screen {
            Screen::PlaceMenu | Screen::GenomeMenu | Screen::LoadMenu | Screen::ThemeMenu => {
                return self.apply_in_menu(action, world);
            }
            Screen::Naming => return self.apply_naming(action, world),
            Screen::SaveNaming => return self.apply_save_naming(action, world),
            Screen::LoadPrompt => return self.apply_load_prompt(action),
            Screen::Help => return self.apply_in_help(action, world),
            Screen::Palette => return self.apply_in_palette(action, world),
            Screen::SpriteList => return self.apply_in_list(action, world),
            Screen::Population => return self.apply_in_graph(action, world),
            Screen::FamilyTree => return self.apply_in_tree(action, world),
            Screen::Normal | Screen::QuitPrompt => {}
        }
        if self.screen == Screen::QuitPrompt {
            match action {
                Action::Confirm => return Flow::Quit,
                // `t`, which tracks a sprite elsewhere, goes back to the
                // title screen (M2 design §8.4).
                Action::Track => return Flow::Title,
                // Only `y` and `t` answer it: `Esc` cancels, as any other key does,
                // and never quits on its own (design v33 §6.6).
                Action::Back => {
                    self.screen = Screen::Normal;
                    return Flow::Continue;
                }
                // The mouse carries on as usual and doesn't answer the prompt;
                // nor does letting go of a button, or of `E`, which isn't a
                // key pressed.
                _ if action.is_pointer() => {}
                // Any other key cancels the prompt, and does nothing else;
                // `F3` never reaches it (above).
                _ => {
                    self.screen = Screen::Normal;
                    return Flow::Continue;
                }
            }
        }
        self.note_focus(&action);
        if self.replay.is_some() && changes_the_world(&action, self.pointed_at_map(&action)) {
            self.refuse(REPLAY_REFUSAL.into());
            return Flow::Continue;
        }
        match action {
            Action::TogglePause
            | Action::StepOnce
            | Action::Faster { .. }
            | Action::Slower { .. } => {
                self.apply_time(&action, world);
            }
            // Scrolling by hand takes the view back from Track.
            Action::Scroll { dx, dy } => {
                self.tracking = false;
                self.scroll(dx, dy);
            }
            // On a pop-up's box, the pointer points at nothing on the map
            // beneath it, so no click or turn of the wheel lands there
            // (`pointed_xy`, design §6.1).
            Action::Point(cell) => self.point(cell),
            // A left click on a spot that takes one, where the last frame
            // drew it, such as a tab or a line of the log (design §6.1).
            Action::Click {
                at,
                button: Button::Left,
                ..
            } if self.clicks_spot(at) => self.click_spot(at, world),
            Action::Click {
                at,
                button,
                amplified,
            } => {
                self.point(at);
                match self.pointed_place(at) {
                    // A click lands where the Cursor is: not past the leash
                    // (design v23 §6.5).
                    Some(Place::Tile(tile)) => {
                        self.act(self.within_leash(tile), button, amplified, world);
                    }
                    // Past the wall, it reaches the sprite the Cursor
                    // follows, or what the player aims, as `Q` and `E` do,
                    // and nothing otherwise (M2 design §11.4).
                    Some(Place::Glass(_) | Place::Space) if !self.out_of_reach() => {
                        self.act(self.cursor, button, amplified, world);
                    }
                    _ => {}
                }
            }
            Action::Press { button, amplified } if !self.out_of_reach() => {
                self.act(self.cursor, button, amplified, world);
            }
            Action::Press { .. } => {}
            // A middle click points, then acts where the Cursor is, as `F`
            // does (design v26 §6.5); past the wall, on no tile (M2 design
            // §11.4).
            Action::Follow { at } => {
                if let Some(at) = at {
                    self.point(at);
                }
                if at.is_none_or(|at| self.pointed_place(at).is_some()) {
                    let tile = self.shown_cursor();
                    self.toggle_follow(tile, world);
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
            Action::OpenThemes => self.list_themes(),
            Action::TakeOver => self.take_over(world),
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
                } else if self.pointed_place(at).is_some() {
                    self.mode = self.mode.along(notches);
                }
            }
            Action::ToggleDetail => self.detail = !self.detail,
            Action::Track => self.toggle_tracking(world),
            Action::CycleEventFilter => self.next_filter(),
            Action::Help => self.open_help(),
            Action::Palette => self.open_palette(),
            Action::SpriteList => {
                self.end_aim();
                self.open_list(world);
            }
            Action::Population => {
                self.end_aim();
                self.screen = Screen::Population;
                self.graph_pick = None;
            }
            Action::FamilyTree => {
                self.end_aim();
                self.screen = Screen::FamilyTree;
                self.tree_choice = None;
                self.tree_scroll = family_tree::Scroll::default();
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
            | Action::Type(_)
            | Action::Erase
            | Action::AnotherName
            | Action::HelpReleased
            | Action::FilterList
            | Action::DebugOverlay
            | Action::Quit => {}
            // `Enter` opens or shuts the memory block under the pointer
            // (design v40 §6.1).
            Action::Enter => {
                if let Some(Spot::Memory(which)) = self.pointed_spot() {
                    self.toggle_lesson(which);
                }
            }
        }
        // Only Grab mode aims.
        if self.mode != CursorMode::Grab {
            self.end_aim();
        }
        self.settle_cursor(world);
        Flow::Continue
    }

    /// `+`: one step faster, as fast as the world's mode allows (M2 design
    /// §16). Pressed at 4× in a Story world, it says that's the fastest.
    fn faster(&mut self, held: bool, world: &World) {
        self.clock.set_fastest(Speed::fastest_in(world.mode()));
        let at_most = if held {
            self.clock.faster_held()
        } else {
            self.clock.faster()
        };
        if at_most && world.mode() == Mode::Story {
            self.tell_player(format!(
                "{} is Story mode's fastest",
                self.clock.fastest().label()
            ));
        }
    }

    /// Notes whether `action` was the player's at the inspector or at the
    /// map, so help opens at the page for the one they were at (#303): the
    /// pointer over either, or the inspector's keys or the map's.
    fn note_focus(&mut self, action: &Action) {
        match *action {
            Action::Point(at) | Action::Click { at, .. } | Action::Wheel { at, .. } => {
                if self.inspector.is_some_and(|area| area.contains(at)) {
                    self.on_inspector = true;
                } else if self.tile_area.contains(at) {
                    self.on_inspector = false;
                }
            }
            Action::NextTab | Action::PreviousTab | Action::ScrollTab { .. } => {
                self.on_inspector = true;
            }
            Action::Scroll { .. }
            | Action::Mode(_)
            | Action::Press { .. }
            | Action::Follow { .. }
            | Action::Track => self.on_inspector = false,
            _ => {}
        }
    }

    /// What the keys for time do, whatever's open: pause, step, faster and
    /// slower (design §6.6). Returns whether `action` was one of them.
    fn apply_time(&mut self, action: &Action, world: &World) -> bool {
        match action {
            Action::TogglePause => self.clock.toggle_pause(),
            Action::StepOnce => self.clock.step_once(),
            Action::Faster { held } => self.faster(*held, world),
            Action::Slower { held: false } => self.clock.slower(),
            Action::Slower { held: true } => self.clock.slower_held(),
            _ => return false,
        }
        true
    }
}

/// Which way `Tab` and `Shift+Tab` go through the sprites.
#[derive(Debug, Clone, Copy)]
enum Direction {
    Next,
    Previous,
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

/// Whether `action` would change the world, or lead to changing it, which a
/// replay refuses (design §2.7): the cursor modes that touch it (Select
/// only points and selects), the wheel over the map view, which changes mode,
/// naming a sprite, showing the Cursor to sprites, and loading a save.
/// Saving, and exporting a genome, only read it.
pub(crate) fn changes_the_world(action: &Action, wheel_on_map: bool) -> bool {
    match action {
        Action::Mode(mode) => *mode != CursorMode::Select,
        Action::Wheel { .. } => wheel_on_map,
        Action::Rename | Action::ToggleVisible | Action::Quickload | Action::OpenSaves => true,
        _ => false,
    }
}
