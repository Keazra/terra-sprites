//! The controls (design §6.1, #303): every key the player has, what it does,
//! and where it works, as one list. The status line's hints, the help
//! screen and the command palette are all built from it, so they can't
//! drift apart. A control isn't a `terra_sim::Command`, which is what a
//! click sends the world.

use crate::app::{App, CursorMode, Screen};
use crate::input::{Action, Button};
use crate::listed::listed_enum;
use terra_sim::{Grip, World};

/// A key: how it's written, what it does in a word or two, as its hint and
/// the command palette say it, and the action it stands for, if the palette
/// can press it.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub written: &'static str,
    pub does: &'static str,
    pub action: Option<Action>,
}

/// One row of the help: a key or keys, and what it does, under one or more
/// of help's headings; with the hint it gives on the status line, and what
/// it needs to do anything there.
#[derive(Debug, Clone, Copy)]
pub struct Control {
    /// The keys as the help and the hint write them: `+/-`, `Z X C`.
    pub written: &'static str,
    /// Each key on its own. The pointer's doing, such as `hold E`, has
    /// none, and nor do keys the help writes together, such as `WASD`.
    pub keys: &'static [Key],
    /// What it does, as the help says it.
    pub does: &'static str,
    /// The help's headings it's under, if it's in the help at all.
    pub headings: &'static [Heading],
    /// What its hint on the status line says.
    pub says: Says,
    /// What it needs to do anything just now, without which its hint is
    /// greyed rather than hidden (#303).
    pub needs: Need,
    /// Other words help's search and the palette find it by.
    pub words: &'static str,
}

/// What a control's hint on the status line says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Says {
    /// Its keys, then this: `+/- speed`.
    Word(&'static str),
    /// Each key with its own name, `Z select  X train  C grab`: hints that
    /// go together when the line is short of room.
    EachKey,
    /// Its keys, then this, and it goes first when the line is short of
    /// room: the sprite list's `/ filter`.
    WordGoingFirst(&'static str),
}

/// What a control needs to do anything just now (#303).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    Nothing,
    /// Time paused: `.` steps only then.
    Paused,
    /// Something to choose: a sprite on the list, an item on a menu, or a
    /// row the palette found.
    Rows,
    /// A palette row chosen whose key the palette can press: the pointer's
    /// rows only teach their keys.
    Pressable,
    /// A sprite selected, for the family tree to be drawn about.
    Selection,
    /// Keys acting, rather than typing into a name or a search, or
    /// answering a question, which any key but its answers cancels.
    KeysAct,
}

listed_enum! {
    /// A heading in the help.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Heading {
        Time => "TIME",
        View => "VIEW",
        Files => "FILES",
        Cursor => "CURSOR",
        Sprites => "SPRITES",
        Inspector => "INSPECTOR",
        Debug => "DEBUG",
        /// The inspector's mouse, on its page alone.
        Pointer => "THE POINTER",
        SpriteList => "SPRITE LIST",
        Graph => "POPULATION GRAPH",
        Tree => "FAMILY TREE",
    }

    /// Every heading, in the order the help gives them.
    pub const ALL;

    /// The heading as the help writes it.
    pub fn label(self) -> &'static str;
}

impl Heading {
    /// Whether its controls work in the world, with nothing open over it,
    /// so the command palette can run them.
    pub fn in_the_world(self) -> bool {
        !matches!(self, Heading::SpriteList | Heading::Graph | Heading::Tree)
    }
}

/// Where the player is, which decides the status line's hints (design
/// §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// The map and the panels, with nothing open over them.
    World,
    /// Aiming a throw (design v25 §6.5).
    Throwing,
    /// Aiming a shove.
    Shoving,
    /// A Place menu item waits on the Cursor, in Grab mode (design v28 §6.5).
    Placing,
    SpriteList,
    /// The sprite list while it's filtered (#313).
    FilteredList,
    Graph,
    Tree,
    Help,
    /// Help while it searches (#303).
    HelpSearching,
    /// The command palette (#303).
    Palette,
    /// A menu over the map: the Place menu and its genomes (design v28
    /// §6.5), saves to load and themes (§6.7).
    Menu,
    /// The title screen's menu (M2 design §8).
    TitleMenu,
    /// Naming a sprite (design §6.5).
    Naming,
    /// Naming a save (design §6.7).
    SaveNaming,
    /// The question before leaving a world (M2 design §8.4).
    Quitting,
}

/// A key with no action the palette can press.
const fn key(written: &'static str, does: &'static str) -> Key {
    Key {
        written,
        does,
        action: None,
    }
}

/// A key the palette can press, as `action`.
const fn presses(written: &'static str, does: &'static str, action: Action) -> Key {
    Key {
        written,
        does,
        action: Some(action),
    }
}

/// A control under `headings` that needs nothing, its hint its keys and
/// what it does.
const fn control(
    written: &'static str,
    keys: &'static [Key],
    does: &'static str,
    headings: &'static [Heading],
    words: &'static str,
) -> Control {
    Control {
        written,
        keys,
        does,
        headings,
        says: Says::Word(does),
        needs: Need::Nothing,
        words,
    }
}

/// A control only the status line shows, as its hint: `written` then
/// `does`.
const fn hint(written: &'static str, does: &'static str) -> Control {
    control(written, &[], does, &[], "")
}

impl Control {
    /// The same control, needing `need` to do anything.
    const fn needing(self, need: Need) -> Control {
        Control {
            needs: need,
            ..self
        }
    }

    /// The same control, its hint saying `says`.
    const fn saying(self, says: Says) -> Control {
        Control { says, ..self }
    }

    /// Its hint as a box's corner gives it: ` esc close `.
    pub fn corner(&self) -> (&'static str, &'static str) {
        let word = match self.says {
            Says::Word(word) | Says::WordGoingFirst(word) => word,
            Says::EachKey => self.does,
        };
        (self.written, word)
    }
}

// Time (design §6.6).
pub const PAUSE: Control = control(
    "space",
    &[presses("space", "pause or resume", Action::TogglePause)],
    "pause / resume",
    &[Heading::Time],
    "stop time run",
)
.saying(Says::Word("pause"));
pub const STEP: Control = control(
    ".",
    &[presses(".", "step", Action::StepOnce)],
    "step",
    &[Heading::Time],
    "one tick forward",
)
.needing(Need::Paused);
pub const SPEED: Control = control(
    "+/-",
    &[
        presses("+", "faster", Action::Faster { held: false }),
        presses("-", "slower", Action::Slower { held: false }),
    ],
    "faster / slower",
    &[Heading::Time],
    "quicker slow down up",
)
.saying(Says::Word("speed"));

// The view (design §6.1).
pub const SCROLL: Control = control(
    "WASD",
    &[],
    "scroll (Shift: 5)",
    &[Heading::View],
    "move the view arrows",
)
.saying(Says::Word("scroll"));
pub const TRACK: Control = control(
    "T",
    &[presses("T", "track the selected", Action::Track)],
    "track the selected",
    &[Heading::View],
    "follow centre the view",
);
pub const COLOURS: Control = control(
    "b",
    &[presses("b", "sprite colours", Action::CycleColours)],
    "sprite colours",
    &[Heading::View],
    "drive lineage plain",
);
pub const EVENT_FILTER: Control = control(
    "m",
    &[presses("m", "event filter", Action::CycleEventFilter)],
    "event filter",
    &[Heading::View],
    "log events major selected",
);
pub const DETAIL: Control = control(
    "v",
    &[presses("v", "exact detail", Action::ToggleDetail)],
    "exact detail",
    &[Heading::View],
    "numbers",
);
pub const HELP: Control = control(
    "?",
    &[presses("?", "help", Action::Help)],
    "this help",
    &[Heading::View],
    "keys legend",
)
.saying(Says::Word("help"))
.needing(Need::KeysAct);
pub const PALETTE: Control = control(
    ":",
    &[key(":", "commands")],
    "find a command",
    &[Heading::View],
    "palette search",
);
pub const BACK: Control = control(
    "esc",
    &[presses("esc", "let go, back, quit?", Action::Back)],
    "let go, back, quit?",
    &[Heading::View],
    "cancel leave",
)
.saying(Says::Word("quit"));
pub const QUIT: Control = control(
    "Ctrl+C",
    &[presses("Ctrl+C", "quit at once", Action::Quit)],
    "quit at once",
    &[Heading::View],
    "leave exit",
);

// Files (design §6.7).
pub const QUICKSAVE: Control = control(
    "F5 F9",
    &[
        presses("F5", "quicksave", Action::Quicksave),
        presses("F9", "load the quicksave", Action::Quickload),
    ],
    "quicksave / load",
    &[Heading::Files],
    "save",
);
pub const SAVE_AS: Control = control(
    "Ctrl+S",
    &[presses("Ctrl+S", "save as...", Action::SaveAs)],
    "save as...",
    &[Heading::Files],
    "name file",
);
pub const LOAD: Control = control(
    "Ctrl+O",
    &[presses("Ctrl+O", "load a save", Action::OpenSaves)],
    "load a save",
    &[Heading::Files],
    "open file",
);
pub const THEME: Control = control(
    "Ctrl+T",
    &[presses("Ctrl+T", "pick a theme", Action::OpenThemes)],
    "pick a theme",
    &[Heading::Files],
    "ascii glyphs colours look",
);
pub const TAKE_OVER: Control = control(
    "Ctrl+R",
    &[presses("Ctrl+R", "take over a replay", Action::TakeOver)],
    "take over a replay",
    &[Heading::Files],
    "play on",
);

// The Cursor (design §6.5).
pub const MODES: Control = control(
    "Z X C",
    &[
        presses("Z", "select", Action::Mode(CursorMode::Select)),
        presses("X", "train", Action::Mode(CursorMode::Train)),
        presses("C", "grab", Action::Mode(CursorMode::Grab)),
    ],
    "select/train/grab",
    &[Heading::Cursor],
    "cursor mode teach move",
)
.saying(Says::EachKey);
pub const CLICKS: Control = control(
    "Q E",
    &[
        presses(
            "Q",
            "left click",
            Action::Press {
                button: Button::Left,
                amplified: false,
            },
        ),
        presses(
            "E",
            "right click",
            Action::Press {
                button: Button::Right,
                amplified: false,
            },
        ),
    ],
    "left / right click",
    &[Heading::Cursor],
    "pet zap activate grab put down",
);
pub const STRONGER: Control = control(
    "Shift+Q E",
    &[
        presses(
            "Shift+Q",
            "hug",
            Action::Press {
                button: Button::Left,
                amplified: true,
            },
        ),
        presses(
            "Shift+E",
            "shock",
            Action::Press {
                button: Button::Right,
                amplified: true,
            },
        ),
    ],
    "hug / shock",
    &[Heading::Cursor],
    "train stronger",
);
pub const CTRL_CLICK: Control = control(
    "Ctrl+click",
    &[],
    "hug / shock",
    &[Heading::Cursor],
    "train stronger",
);
pub const SHOW: Control = control(
    "H",
    &[presses("H", "show to sprites", Action::ToggleVisible)],
    "show to sprites",
    &[Heading::Cursor],
    "hide seen visible",
);
pub const FOLLOW: Control = control(
    "F",
    &[presses("F", "follow a sprite", Action::Follow { at: None })],
    "follow a sprite",
    &[Heading::Cursor],
    "",
);
pub const MIDDLE: Control = control(
    "middle",
    &[],
    "follow a sprite",
    &[Heading::Cursor],
    "click",
);
pub const WHEEL: Control = control("wheel", &[], "change mode", &[Heading::Cursor], "cursor");
pub const PLACE_MENU: Control = control(
    "C again",
    &[],
    "the Place menu",
    &[Heading::Cursor],
    "put new object sprite egg genome",
);
pub const AIM: Control = control(
    "hold E",
    &[],
    "aim, let go to send",
    &[Heading::Cursor],
    "throw shove",
);

// Sprites (design §6.1).
pub const NEXT_SPRITE: Control = control(
    "tab",
    &[presses("tab", "next sprite", Action::SelectNext)],
    "next sprite",
    &[Heading::Sprites],
    "select",
);
pub const PREVIOUS_SPRITE: Control = control(
    "Shift+tab",
    &[presses(
        "Shift+tab",
        "previous sprite",
        Action::SelectPrevious,
    )],
    "previous sprite",
    &[Heading::Sprites],
    "select",
);
pub const SPRITE_LIST: Control = control(
    "l",
    &[presses("l", "sprite list", Action::SpriteList)],
    "sprite list",
    &[Heading::Sprites],
    "all find",
);
pub const LIST_THEN_FILTER: Control = control(
    "l then /",
    &[],
    "filter the list",
    &[Heading::Sprites],
    "find search",
);
pub const POPULATION: Control = control(
    "p",
    &[presses("p", "population graph", Action::Population)],
    "population graph",
    &[Heading::Sprites],
    "count history eggs",
);
pub const FAMILY_TREE: Control = control(
    "k",
    &[presses("k", "family tree", Action::FamilyTree)],
    "family tree",
    &[Heading::Sprites],
    "parents young eggs lineage relatives",
);
pub const RENAME: Control = control(
    "r",
    &[presses("r", "name it", Action::Rename)],
    "name it",
    &[Heading::Sprites],
    "rename",
);
pub const EXPORT_GENOME: Control = control(
    "g",
    &[presses("g", "save its genome", Action::ExportGenome)],
    "save its genome",
    &[Heading::Sprites],
    "genes file export egg",
);

// The inspector (design §6.1).
pub const TABS: Control = control(
    "[ ]",
    &[
        presses("[", "previous tab", Action::PreviousTab),
        presses("]", "next tab", Action::NextTab),
    ],
    "switch tabs",
    &[Heading::Inspector],
    "body brain chem genome world",
);
pub const TAB_SCROLL: Control = control(
    "PgUp PgDn",
    &[
        presses("PgUp", "scroll the tab up", Action::ScrollTab { pages: -1 }),
        presses(
            "PgDn",
            "scroll the tab down",
            Action::ScrollTab { pages: 1 },
        ),
    ],
    "scroll a tab",
    &[Heading::Inspector],
    "page",
);
pub const TAB_CLICK: Control = control(
    "click",
    &[],
    "open the tab clicked",
    &[Heading::Pointer],
    "body brain chem genome world",
);
pub const TAB_WHEEL: Control = control(
    "wheel",
    &[],
    "scroll the tab under it",
    &[Heading::Pointer],
    "",
);
pub const MEMORY: Control = control(
    "click",
    &[],
    "open or shut a memory",
    &[Heading::Pointer],
    "brain heading lesson learned",
);
pub const MEMORY_ENTER: Control = control(
    "enter",
    &[],
    "the same, where it points",
    &[Heading::Pointer],
    "brain heading lesson learned",
);

// The debug overlay (design §6.1, #311).
pub const DEBUG: Control = control(
    "F3",
    &[presses("F3", "debug overlay", Action::DebugOverlay)],
    "how it's running",
    &[Heading::Debug],
    "speed frames keys",
);

// The sprite list (design §6.1, #313).
pub const FILTER: Control = control("/", &[], "filter", &[Heading::SpriteList], "find search")
    .saying(Says::WordGoingFirst("filter"));
pub const CHOOSE: Control =
    control("↑↓", &[], "choose", &[Heading::SpriteList], "wheel").needing(Need::Rows);
pub const GO_TO: Control = control(
    "enter",
    &[],
    "go to it",
    &[Heading::SpriteList],
    "click select",
)
.needing(Need::Rows);
pub const SORT: Control = control("tab", &[], "sort", &[Heading::SpriteList], "order");
pub const SORT_BACK: Control = control(
    "Shift+tab",
    &[],
    "sort the other way",
    &[Heading::SpriteList],
    "order",
);
pub const CLOSE: Control = control(
    "esc",
    &[],
    "close",
    &[Heading::SpriteList, Heading::Graph, Heading::Tree],
    "",
);
pub const CLEAR: Control =
    control("esc", &[], "clear the filter", &[Heading::SpriteList], "").saying(Says::Word("clear"));

// The population graph (M2 design §14.3).
pub const PICK: Control = control(
    "←→",
    &[],
    "pick a moment",
    &[Heading::Graph],
    "pointer column",
);

// The family tree (M2 design §17.3).
pub const TREE_CHOOSE: Control =
    control("←→↑↓", &[], "choose", &[Heading::Tree], "pointer card").needing(Need::Selection);
pub const CENTRE: Control = control(
    "enter",
    &[],
    "centre on it",
    &[Heading::Tree],
    "click select walk",
)
.needing(Need::Selection);

// Help itself, and the palette (#303).
pub const HELP_PAGE: Control = hint("←→", "page");
pub const SEARCH: Control = hint("type", "to search");
pub const DO_IT: Control = hint("enter", "do it").needing(Need::Pressable);
/// `↑↓ choose` on the palette or a menu.
pub const CHOOSE_ROW: Control = hint("↑↓", "choose").needing(Need::Rows);

// The menus (design v28 §6.5, §6.7) and the title screen's (M2 design §8).
pub const PICK_ITEM: Control = hint("enter", "pick").needing(Need::Rows);
pub const TITLE_CHOOSE: Control = hint("↑↓", "choose");
pub const GO: Control = hint("enter", "go");

// While aiming (design v25 §6.5) or placing (design v28 §6.5).
pub const THROW: Control = hint("let go", "to throw");
pub const SHOVE: Control = hint("let go", "to shove");
pub const CANCEL: Control = hint("esc", "cancel");
pub const PLACE: Control = hint("click", "to place");
pub const PUT_AWAY: Control = hint("right-click", "put away");

// The prompts that take the status line over (design §6.5, §6.7, M2
// design §8.4).
pub const NAME_OK: Control = hint("enter", "ok");
pub const ANOTHER_NAME: Control = hint("tab", "another");
pub const SAVE: Control = hint("enter", "save");
pub const QUIT_GAME: Control = hint("y", "quit the game");
pub const TITLE_SCREEN: Control = hint("t", "title screen");
pub const STAY: Control = hint("any other key", "stays");

/// Every control in the help, in the order it lists them under each
/// heading.
pub const CONTROLS: &[Control] = &[
    PAUSE,
    STEP,
    SPEED,
    SCROLL,
    TRACK,
    COLOURS,
    EVENT_FILTER,
    DETAIL,
    HELP,
    PALETTE,
    BACK,
    QUIT,
    QUICKSAVE,
    SAVE_AS,
    LOAD,
    THEME,
    TAKE_OVER,
    MODES,
    CLICKS,
    STRONGER,
    CTRL_CLICK,
    SHOW,
    FOLLOW,
    MIDDLE,
    WHEEL,
    PLACE_MENU,
    AIM,
    NEXT_SPRITE,
    PREVIOUS_SPRITE,
    SPRITE_LIST,
    LIST_THEN_FILTER,
    POPULATION,
    FAMILY_TREE,
    RENAME,
    EXPORT_GENOME,
    TABS,
    TAB_SCROLL,
    TAB_CLICK,
    TAB_WHEEL,
    MEMORY,
    MEMORY_ENTER,
    DEBUG,
    FILTER,
    CHOOSE,
    GO_TO,
    SORT,
    SORT_BACK,
    CLEAR,
    PICK,
    TREE_CHOOSE,
    CENTRE,
    CLOSE,
];

/// The status line's hints in `context`, in order (design §6.1). Short of
/// room, a hint saying it goes first does, then whole hints from the end.
pub fn hints(context: Context) -> &'static [Control] {
    match context {
        Context::World => &[MODES, SCROLL, PAUSE, STEP, SPEED, BACK],
        Context::Throwing => &[THROW, CANCEL],
        Context::Shoving => &[SHOVE, CANCEL],
        Context::Placing => &[PLACE, PUT_AWAY],
        Context::SpriteList => &[FILTER, CHOOSE, GO_TO, SORT, CLOSE],
        Context::FilteredList => &[CHOOSE, GO_TO, SORT, CLEAR],
        Context::Graph => &[PICK, PAUSE, SPEED, CLOSE],
        Context::Tree => &[TREE_CHOOSE, CENTRE, PAUSE, CLOSE],
        Context::Help => &[HELP_PAGE, SEARCH, CLOSE],
        Context::HelpSearching => &[SEARCH, CLEAR],
        Context::Palette => &[CHOOSE_ROW, DO_IT, CLOSE],
        Context::Menu => &[CHOOSE_ROW, PICK_ITEM, CLOSE],
        Context::TitleMenu => &[TITLE_CHOOSE, GO, BACK],
        Context::Naming => &[NAME_OK, ANOTHER_NAME, CANCEL],
        Context::SaveNaming => &[SAVE, CANCEL],
        Context::Quitting => &[QUIT_GAME, TITLE_SCREEN, STAY],
    }
}

/// Where the player is in `app` on `world`, for the status line's hints.
pub fn context(app: &App, world: &World) -> Context {
    match app.screen() {
        Screen::Help if app.help_search().is_some() => Context::HelpSearching,
        Screen::Help => Context::Help,
        Screen::Palette => Context::Palette,
        Screen::PlaceMenu | Screen::GenomeMenu | Screen::LoadMenu | Screen::ThemeMenu => {
            Context::Menu
        }
        Screen::SpriteList if app.list_filter().is_some() => Context::FilteredList,
        Screen::SpriteList => Context::SpriteList,
        Screen::Population => Context::Graph,
        Screen::FamilyTree => Context::Tree,
        Screen::Naming => Context::Naming,
        Screen::SaveNaming => Context::SaveNaming,
        Screen::QuitPrompt => Context::Quitting,
        _ if app.aiming() => match app.grip(world) {
            Some(Grip::Leads(_)) => Context::Shoving,
            _ => Context::Throwing,
        },
        _ if app.placing().is_some() && app.mode() == CursorMode::Grab => Context::Placing,
        _ => Context::World,
    }
}

/// A hint on the status line: its key, what it does, and whether it does
/// anything just now; greyed if not (#303).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hint {
    pub key: &'static str,
    pub does: &'static str,
    pub ready: bool,
}

/// The hints `control` gives in `app` on `world`, as one: each key's when
/// it says each, which go together.
pub fn hints_of(control: &Control, app: &App, world: &World) -> Vec<Hint> {
    let ready = |keys: &[Key]| {
        meets(control.needs, app, world)
            && (keys.is_empty() || keys.iter().any(|key| key_works(key, app)))
    };
    match control.says {
        Says::Word(does) | Says::WordGoingFirst(does) => vec![Hint {
            key: control.written,
            does,
            ready: ready(control.keys),
        }],
        Says::EachKey => control
            .keys
            .iter()
            .map(|key| Hint {
                key: key.written,
                does: key.does,
                ready: ready(std::slice::from_ref(key)),
            })
            .collect(),
    }
}

/// Whether `need` is met in `app` on `world`.
fn meets(need: Need, app: &App, world: &World) -> bool {
    match need {
        Need::Nothing => true,
        Need::Paused => app.clock.is_paused(),
        Need::Rows => match app.screen() {
            Screen::Palette => !app.palette_rows().is_empty(),
            Screen::PlaceMenu | Screen::GenomeMenu | Screen::LoadMenu | Screen::ThemeMenu => {
                !app.menu_items(world).is_empty()
            }
            _ => !crate::sprite_list::order(app, world).is_empty(),
        },
        Need::Pressable => app
            .palette_rows()
            .get(app.palette_choice())
            .is_some_and(|row| pressable(&row.key, app)),
        Need::Selection => app.selection().is_some(),
        Need::KeysAct => {
            app.screen() == Screen::Help
                || !app.typing() && !matches!(app.screen(), Screen::QuitPrompt | Screen::LoadPrompt)
        }
    }
}

/// Whether `key` works: in a replay, nothing that would change the world
/// does (design §2.7), so its hint is greyed.
fn key_works(key: &Key, app: &App) -> bool {
    !app.replaying()
        || key
            .action
            .is_none_or(|action| !crate::app::changes_the_world(&action, false))
}

/// Whether the palette can press `key` just now: it has an action, and
/// a replay wouldn't refuse it.
pub fn pressable(key: &Key, app: &App) -> bool {
    key.action.is_some() && key_works(key, app)
}

/// A row of the command palette (#303): a control and one of its keys,
/// or for a control with none, such as the pointer's, the control itself,
/// which teaches its keys but can't press them.
#[derive(Debug, Clone, Copy)]
pub struct PaletteRow {
    pub control: &'static Control,
    pub key: Key,
}

/// The command palette's rows (#303): each key the palette can press, of
/// every control that works in the world, and each such control with no
/// keys of its own. The palette's own key isn't one.
pub fn palette_rows() -> impl Iterator<Item = PaletteRow> {
    CONTROLS
        .iter()
        .filter(|control| control.headings.iter().any(|h| h.in_the_world()))
        .flat_map(|control| {
            let keys = control.keys.iter().filter(|key| key.action.is_some());
            let whole = control.keys.is_empty().then_some(Key {
                written: control.written,
                does: control.does,
                action: None,
            });
            keys.copied()
                .chain(whole)
                .map(move |key| PaletteRow { control, key })
        })
}

/// What help's search and the palette find a control by (#303): its keys
/// as `written`, what each of `keys` does, what the help says it does, its
/// headings and its other words.
pub fn found_by(control: &Control, written: &str, keys: &[Key]) -> String {
    let does: Vec<&str> = keys.iter().map(|key| key.does).collect();
    let headings: Vec<&str> = control.headings.iter().map(|h| h.label()).collect();
    format!(
        "{written} {} {} {} {}",
        does.join(" "),
        control.does,
        headings.join(" "),
        control.words
    )
}
