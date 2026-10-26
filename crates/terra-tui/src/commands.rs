//! The commands (design §6.1, #303): every key the player has, what it does,
//! and where it works, as one list. The status line's hints, the help
//! screen and the command palette are all built from it, so they can't
//! drift apart.

use crate::app::{App, CursorMode, Screen};
use crate::input::{Action, Button};
use crate::listed::listed_enum;
use terra_sim::{Grip, World};

/// A key, what it does in a word or two, as its hint and the command
/// palette name it, and the action it stands for, if the palette can press
/// it.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub key: &'static str,
    pub name: &'static str,
    pub action: Option<Action>,
}

/// One row of the help: a key or keys, and what it does, under one or more
/// of help's headings; with the hint it gives on the status line, and what
/// it needs to do anything there.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    /// The keys as the help and the hint write them: `+/-`, `Z X C`.
    pub written: &'static str,
    /// Each key on its own. The pointer's doing, such as `hold E`, has none.
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

/// What a command's hint on the status line says.
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

/// What a command needs to do anything just now (#303).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    Nothing,
    /// Time paused: `.` steps only then.
    Paused,
    /// Something to choose: a sprite on the list, or a command the palette
    /// found.
    Rows,
    /// A sprite selected, for the family tree to be drawn about.
    Selection,
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
    /// Whether its commands work in the world, with nothing open over it,
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
    /// The command palette (#303).
    Palette,
    /// Naming a sprite (design §6.5).
    Naming,
    /// Naming a save (design §6.7).
    SaveNaming,
    /// The question before leaving a world (M2 design §8.4).
    Quitting,
}

/// A key with no action the palette can press.
const fn key(key: &'static str, name: &'static str) -> Key {
    Key {
        key,
        name,
        action: None,
    }
}

/// A key the palette can press, as `action`.
const fn presses(key: &'static str, name: &'static str, action: Action) -> Key {
    Key {
        key,
        name,
        action: Some(action),
    }
}

/// A command under `headings` that needs nothing, its hint its keys and
/// what it does.
const fn command(
    written: &'static str,
    keys: &'static [Key],
    does: &'static str,
    headings: &'static [Heading],
    words: &'static str,
) -> Command {
    Command {
        written,
        keys,
        does,
        headings,
        says: Says::Word(does),
        needs: Need::Nothing,
        words,
    }
}

/// A command only the status line shows, as its hint: `written` then
/// `does`.
const fn hint(written: &'static str, does: &'static str) -> Command {
    command(written, &[], does, &[], "")
}

impl Command {
    /// The same command, needing `need` to do anything.
    const fn needing(self, need: Need) -> Command {
        Command {
            needs: need,
            ..self
        }
    }

    /// The same command, its hint saying `says`.
    const fn saying(self, says: Says) -> Command {
        Command { says, ..self }
    }
}

// Time (design §6.6).
pub const PAUSE: Command = command(
    "space",
    &[presses("space", "pause or resume", Action::TogglePause)],
    "pause / resume",
    &[Heading::Time],
    "stop time run",
)
.saying(Says::Word("pause"));
pub const STEP: Command = command(
    ".",
    &[presses(".", "step", Action::StepOnce)],
    "step",
    &[Heading::Time],
    "one tick forward",
)
.needing(Need::Paused);
pub const SPEED: Command = command(
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
pub const SCROLL: Command = command(
    "WASD",
    &[key("WASD", "scroll")],
    "scroll (Shift: 5)",
    &[Heading::View],
    "move the view arrows",
)
.saying(Says::Word("scroll"));
pub const TRACK: Command = command(
    "T",
    &[presses("T", "track the selected", Action::Track)],
    "track the selected",
    &[Heading::View],
    "follow centre the view",
);
pub const COLOURS: Command = command(
    "b",
    &[presses("b", "sprite colours", Action::CycleColours)],
    "sprite colours",
    &[Heading::View],
    "drive lineage plain",
);
pub const EVENT_FILTER: Command = command(
    "m",
    &[presses("m", "event filter", Action::CycleEventFilter)],
    "event filter",
    &[Heading::View],
    "log events major selected",
);
pub const DETAIL: Command = command(
    "v",
    &[presses("v", "exact detail", Action::ToggleDetail)],
    "exact detail",
    &[Heading::View],
    "numbers",
);
pub const HELP: Command = command(
    "?",
    &[presses("?", "help", Action::Help)],
    "this help",
    &[Heading::View],
    "keys legend",
)
.saying(Says::Word("help"));
pub const PALETTE: Command = command(
    ":",
    &[key(":", "commands")],
    "find a command",
    &[Heading::View],
    "palette search",
);
pub const BACK: Command = command(
    "esc",
    &[presses("esc", "let go, back, quit?", Action::Back)],
    "let go, back, quit?",
    &[Heading::View],
    "cancel leave",
)
.saying(Says::Word("quit"));
pub const QUIT: Command = command(
    "Ctrl+C",
    &[presses("Ctrl+C", "quit at once", Action::Quit)],
    "quit at once",
    &[Heading::View],
    "leave exit",
);

// Files (design §6.7).
pub const QUICKSAVE: Command = command(
    "F5 F9",
    &[
        presses("F5", "quicksave", Action::Quicksave),
        presses("F9", "load the quicksave", Action::Quickload),
    ],
    "quicksave / load",
    &[Heading::Files],
    "save",
);
pub const SAVE_AS: Command = command(
    "Ctrl+S",
    &[presses("Ctrl+S", "save as...", Action::SaveAs)],
    "save as...",
    &[Heading::Files],
    "name file",
);
pub const LOAD: Command = command(
    "Ctrl+O",
    &[presses("Ctrl+O", "load a save", Action::OpenSaves)],
    "load a save",
    &[Heading::Files],
    "open file",
);
pub const THEME: Command = command(
    "Ctrl+T",
    &[presses("Ctrl+T", "pick a theme", Action::OpenThemes)],
    "pick a theme",
    &[Heading::Files],
    "ascii glyphs colours look",
);
pub const TAKE_OVER: Command = command(
    "Ctrl+R",
    &[presses("Ctrl+R", "take over a replay", Action::TakeOver)],
    "take over a replay",
    &[Heading::Files],
    "play on",
);

// The Cursor (design §6.5).
pub const MODES: Command = command(
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
pub const CLICKS: Command = command(
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
pub const STRONGER: Command = command(
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
pub const CTRL_CLICK: Command = command(
    "Ctrl+click",
    &[],
    "hug / shock",
    &[Heading::Cursor],
    "train stronger",
);
pub const SHOW: Command = command(
    "H",
    &[presses("H", "show to sprites", Action::ToggleVisible)],
    "show to sprites",
    &[Heading::Cursor],
    "hide seen visible",
);
pub const FOLLOW: Command = command(
    "F",
    &[presses("F", "follow a sprite", Action::Follow { at: None })],
    "follow a sprite",
    &[Heading::Cursor],
    "",
);
pub const MIDDLE: Command = command(
    "middle",
    &[],
    "follow a sprite",
    &[Heading::Cursor],
    "click",
);
pub const WHEEL: Command = command("wheel", &[], "change mode", &[Heading::Cursor], "cursor");
pub const PLACE_MENU: Command = command(
    "C again",
    &[],
    "the Place menu",
    &[Heading::Cursor],
    "put new object sprite egg genome",
);
pub const AIM: Command = command(
    "hold E",
    &[],
    "aim, let go to send",
    &[Heading::Cursor],
    "throw shove",
);

// Sprites (design §6.1).
pub const NEXT_SPRITE: Command = command(
    "tab",
    &[presses("tab", "next sprite", Action::SelectNext)],
    "next sprite",
    &[Heading::Sprites],
    "select",
);
pub const PREVIOUS_SPRITE: Command = command(
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
pub const SPRITE_LIST: Command = command(
    "l",
    &[presses("l", "sprite list", Action::SpriteList)],
    "sprite list",
    &[Heading::Sprites],
    "all find",
);
pub const LIST_THEN_FILTER: Command = command(
    "l then /",
    &[],
    "filter the list",
    &[Heading::Sprites],
    "find search",
);
pub const POPULATION: Command = command(
    "p",
    &[presses("p", "population graph", Action::Population)],
    "population graph",
    &[Heading::Sprites],
    "count history eggs",
);
pub const FAMILY_TREE: Command = command(
    "k",
    &[presses("k", "family tree", Action::FamilyTree)],
    "family tree",
    &[Heading::Sprites],
    "parents young eggs lineage relatives",
);
pub const RENAME: Command = command(
    "r",
    &[presses("r", "name it", Action::Rename)],
    "name it",
    &[Heading::Sprites],
    "rename",
);
pub const EXPORT_GENOME: Command = command(
    "g",
    &[presses("g", "save its genome", Action::ExportGenome)],
    "save its genome",
    &[Heading::Sprites],
    "genes file export egg",
);

// The inspector (design §6.1).
pub const TABS: Command = command(
    "[ ]",
    &[
        presses("[", "previous tab", Action::PreviousTab),
        presses("]", "next tab", Action::NextTab),
    ],
    "switch tabs",
    &[Heading::Inspector],
    "body brain chem genome world",
);
pub const TAB_SCROLL: Command = command(
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
pub const TAB_WHEEL: Command = command(
    "wheel",
    &[],
    "scroll the tab under it",
    &[Heading::Pointer],
    "",
);
pub const MEMORY: Command = command(
    "click",
    &[],
    "open or shut a memory",
    &[Heading::Pointer],
    "brain heading lesson learned",
);
pub const MEMORY_ENTER: Command = command(
    "enter",
    &[],
    "the same, where it points",
    &[Heading::Pointer],
    "brain heading lesson learned",
);

// The debug overlay (design §6.1, #311).
pub const DEBUG: Command = command(
    "F3",
    &[presses("F3", "debug overlay", Action::DebugOverlay)],
    "how it's running",
    &[Heading::Debug],
    "speed frames keys",
);

// The sprite list (design §6.1, #313).
pub const FILTER: Command = command("/", &[], "filter", &[Heading::SpriteList], "find search")
    .saying(Says::WordGoingFirst("filter"));
pub const CHOOSE: Command =
    command("↑↓", &[], "choose", &[Heading::SpriteList], "wheel").needing(Need::Rows);
pub const GO_TO: Command = command(
    "enter",
    &[],
    "go to it",
    &[Heading::SpriteList],
    "click select",
)
.needing(Need::Rows);
pub const SORT: Command = command("tab", &[], "sort", &[Heading::SpriteList], "order");
pub const SORT_BACK: Command = command(
    "Shift+tab",
    &[],
    "sort the other way",
    &[Heading::SpriteList],
    "order",
);
pub const CLOSE: Command = command(
    "esc",
    &[],
    "close",
    &[Heading::SpriteList, Heading::Graph, Heading::Tree],
    "",
);
pub const CLEAR: Command =
    command("esc", &[], "clear the filter", &[Heading::SpriteList], "").saying(Says::Word("clear"));

// The population graph (M2 design §14.3).
pub const PICK: Command = command(
    "←→",
    &[],
    "pick a moment",
    &[Heading::Graph],
    "pointer column",
);

// The family tree (M2 design §17.3).
pub const TREE_CHOOSE: Command =
    command("←→↑↓", &[], "choose", &[Heading::Tree], "pointer card").needing(Need::Selection);
pub const CENTRE: Command = command(
    "enter",
    &[],
    "centre on it",
    &[Heading::Tree],
    "click select walk",
)
.needing(Need::Selection);

// Help itself, and the palette (#303).
pub const HELP_PAGE: Command = hint("←→", "page");
pub const SEARCH: Command = hint("type", "to search");
pub const DO_IT: Command = hint("enter", "do it").needing(Need::Rows);
pub const PALETTE_CHOOSE: Command = hint("↑↓", "choose").needing(Need::Rows);

// While aiming (design v25 §6.5) or placing (design v28 §6.5).
pub const THROW: Command = hint("let go", "to throw");
pub const SHOVE: Command = hint("let go", "to shove");
pub const CANCEL: Command = hint("esc", "cancel");
pub const PLACE: Command = hint("click", "to place");
pub const PUT_AWAY: Command = hint("right-click", "put away");

// The prompts that take the status line over (design §6.5, §6.7, M2
// design §8.4).
pub const NAME_OK: Command = hint("enter", "ok");
pub const ANOTHER_NAME: Command = hint("tab", "another");
pub const SAVE: Command = hint("enter", "save");
pub const QUIT_GAME: Command = hint("y", "quit the game");
pub const TITLE_SCREEN: Command = hint("t", "title screen");
pub const STAY: Command = hint("any other key", "stays");

/// Every command in the help, in the order it lists them under each
/// heading.
pub const COMMANDS: &[Command] = &[
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
pub fn hints(context: Context) -> &'static [Command] {
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
        Context::Palette => &[PALETTE_CHOOSE, DO_IT, CLOSE],
        Context::Naming => &[NAME_OK, ANOTHER_NAME, CANCEL],
        Context::SaveNaming => &[SAVE, CANCEL],
        Context::Quitting => &[QUIT_GAME, TITLE_SCREEN, STAY],
    }
}

/// Where the player is in `app` on `world`, for the status line's hints.
pub fn context(app: &App, world: &World) -> Context {
    match app.screen() {
        Screen::Help => Context::Help,
        Screen::Palette => Context::Palette,
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

/// The hints `command` gives in `app` on `world`, as one: each key's when
/// it says each, which go together.
pub fn hints_of(command: &Command, app: &App, world: &World) -> Vec<Hint> {
    let ready = |keys: &[Key]| {
        meets(command.needs, app, world)
            && (keys.is_empty() || keys.iter().any(|key| key_works(key, app)))
    };
    match command.says {
        Says::Word(does) | Says::WordGoingFirst(does) => vec![Hint {
            key: command.written,
            does,
            ready: ready(command.keys),
        }],
        Says::EachKey => command
            .keys
            .iter()
            .map(|key| Hint {
                key: key.key,
                does: key.name,
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
            _ => !crate::sprite_list::order(app, world).is_empty(),
        },
        Need::Selection => app.selection().is_some(),
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

/// The keys the command palette can press (#303): each of a command that
/// works in the world, with what it's found by.
pub fn palette_keys() -> impl Iterator<Item = (&'static Command, &'static Key)> {
    COMMANDS
        .iter()
        .filter(|command| command.headings.iter().any(|h| h.in_the_world()))
        .flat_map(|command| {
            command
                .keys
                .iter()
                .filter(|key| key.action.is_some())
                .map(move |key| (command, key))
        })
}

/// What a command and one of its keys are found by: its name, what the
/// help says it does, its heading and its other words.
pub fn found_by(command: &Command, key: &Key) -> String {
    let headings: Vec<&str> = command.headings.iter().map(|h| h.label()).collect();
    format!(
        "{} {} {} {}",
        key.name,
        command.does,
        headings.join(" "),
        command.words
    )
}
