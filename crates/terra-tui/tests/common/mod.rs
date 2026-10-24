//! Shared helpers for terra-tui integration tests: hand-drawn worlds, apps
//! on them, frames drawn as text, the player's clicks and ticks (#156), and
//! worlds where generations come quickly, for the family tree (M2 design
//! §17).
//!
//! Each test file is its own crate, so it `mod common;` and uses only the
//! pieces it needs. Helpers a given file never calls are allowed here so the
//! shared API stays one place, as in terra-sim's (#155).
#![allow(dead_code)]

use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Position, Rect, Size};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, Command, DataPack, EntityId, Event, EventKind, Genome, Hurt, Map, Outcome, Pos,
    Progress, Rejection, Scenario, ScriptedAction, Target, Verb, World,
};
use terra_tui::app::{App, Areas, CursorMode, Flow, Screen, Selection, Tab};
use terra_tui::input::{Action, Button};
use terra_tui::theme::Theme;
use terra_tui::ui;

/// The built-in data pack.
pub fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A position at `(x, y)`.
pub const fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// Builds a test world on a hand-drawn map: objects, sprites with their own
/// genes or the starter genome, scripted actions, newborns, the data and a
/// seed, as terra-sim's tests build theirs (#155).
///
/// Sprite and script methods append; [`Self::objects`] replaces.
pub struct WorldBuilder<'a> {
    rows: Vec<String>,
    objects: &'a [(Pos, &'a str)],
    sprites: Vec<(Pos, SpriteGenome<'a>)>,
    scripted: Vec<(Pos, ScriptedAction)>,
    newborns: Vec<Pos>,
    data: Option<DataPack>,
    seed: u64,
}

/// Where the genome of a [`WorldBuilder`] sprite comes from. Genes written
/// out are read with the world's data pack once the world is built, so the
/// pack can be given before or after them.
enum SpriteGenome<'a> {
    /// The starter genome, with spawn variation.
    Starter,
    /// A genome already made.
    Made(Genome),
    /// Genes written as in a genome file.
    Genes(&'a str),
}

impl<'a> WorldBuilder<'a> {
    /// Starts from an ascii drawing of the map.
    pub fn new(rows: &[&str]) -> Self {
        Self {
            rows: rows.iter().map(|row| row.to_string()).collect(),
            objects: &[],
            sprites: Vec::new(),
            scripted: Vec::new(),
            newborns: Vec::new(),
            data: None,
            seed: 1,
        }
    }

    /// Starts from a map all of grass, `width`×`height`.
    pub fn grass(width: usize, height: usize) -> Self {
        let row = ".".repeat(width);
        Self::new(&vec![row.as_str(); height])
    }

    /// Objects as `(tile, object type name)` (replaces any previous list).
    pub fn objects(mut self, objects: &'a [(Pos, &'a str)]) -> Self {
        self.objects = objects;
        self
    }

    /// A sprite of the starter genome, with spawn variation, on each of
    /// `tiles` (appends).
    pub fn starters(mut self, tiles: &[Pos]) -> Self {
        let genomes = tiles.iter().map(|&tile| (tile, SpriteGenome::Starter));
        self.sprites.extend(genomes);
        self
    }

    /// Sprites as `(tile, genes)`, the starter genome where there are none
    /// (appends).
    pub fn sprites_with(mut self, sprites: Vec<(Pos, Option<Genome>)>) -> Self {
        let genomes = sprites.into_iter().map(|(tile, genome)| {
            let genome = genome.map_or(SpriteGenome::Starter, SpriteGenome::Made);
            (tile, genome)
        });
        self.sprites.extend(genomes);
        self
    }

    /// Sprites as `(tile, genes)`, the genes written as in a genome file and
    /// read with the world's data pack, the built-in one or [`Self::data`]'s,
    /// once the world is built (appends).
    pub fn genes(mut self, sprites: &[(Pos, &'a str)]) -> Self {
        let genomes = sprites
            .iter()
            .map(|&(tile, genes)| (tile, SpriteGenome::Genes(genes)));
        self.sprites.extend(genomes);
        self
    }

    /// Scripted actions by starting tile (appends).
    pub fn scripts(mut self, scripted: &[(Pos, ScriptedAction)]) -> Self {
        self.scripted.extend_from_slice(scripted);
        self
    }

    /// The tiles of sprites that start as newborns, not grown up (M2 design
    /// §9.1) (appends).
    pub fn newborns(mut self, newborns: &[Pos]) -> Self {
        self.newborns.extend_from_slice(newborns);
        self
    }

    /// Uses this data pack instead of the built-in one.
    pub fn data(mut self, data: DataPack) -> Self {
        self.data = Some(data);
        self
    }

    /// The world seed (default 1).
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Builds the world, before its first tick.
    pub fn build(self) -> World {
        let data = self.data.unwrap_or_else(pack);
        let rows: Vec<&str> = self.rows.iter().map(String::as_str).collect();
        let map = Map::from_ascii(&rows, &data).expect("valid drawing");
        let sprites: Vec<(Pos, Option<Genome>)> = self
            .sprites
            .into_iter()
            .map(|(tile, source)| match source {
                SpriteGenome::Starter => (tile, None),
                SpriteGenome::Made(genome) => (tile, Some(genome)),
                SpriteGenome::Genes(genes) => {
                    let genome = Genome::from_ron(genes, &data).expect("valid genes");
                    (tile, Some(genome))
                }
            })
            .collect();
        let scenario = Scenario {
            map,
            objects: self.objects,
            sprites: &sprites,
            scripted: &self.scripted,
            newborns: &self.newborns,
        };
        World::from_scenario(scenario, data, self.seed).expect("valid scenario")
    }
}

/// An all-grass world, 10×6, with `objects`, and starter sprites on
/// `sprites`.
pub fn field(objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    WorldBuilder::grass(10, 6)
        .objects(objects)
        .starters(sprites)
        .build()
}

/// An 11×8 grass world with starter sprites on (2, 2) and (8, 3) and a
/// ball on (5, 5).
pub fn ball_world() -> World {
    WorldBuilder::grass(11, 8)
        .objects(&[(at(5, 5), "ball")])
        .starters(&[at(2, 2), at(8, 3)])
        .seed(7)
        .build()
}

/// Steps `world` `ticks` ticks.
pub fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
}

/// The ID of the sprite on `pos`.
pub fn sprite_on(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite").id()
}

/// The ID of the object on `pos`.
pub fn object_on(world: &World, pos: Pos) -> EntityId {
    world.object_at(pos).expect("an object").id()
}

/// The data folder the help screen shows, made up so the screen reads the
/// same on every machine.
pub const DATA_FOLDER: &str = "/home/player/.local/share/terra-sprites";

/// A fresh, empty folder for files, named for the test file and `name`.
pub fn scratch_folder(name: &str) -> PathBuf {
    let file = env!("CARGO_CRATE_NAME");
    let dir = std::env::temp_dir().join(format!("terra-{file}-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// Panels with only the map view, its tiles drawn on `tiles`.
pub fn map_only(tiles: Rect) -> Areas {
    Areas {
        tiles,
        inspector: None,
        event_log: None,
        overlay: None,
    }
}

/// The app on `world` in the default theme, with only the map view, its
/// tiles drawn from screen cell (0, 0), as many as the map has: so a
/// tile's cell is its position (see `pointing_at`).
pub fn app(world: &World) -> App {
    let map = world.map();
    map_app(world, Rect::new(0, 0, map.width(), map.height()))
}

/// The same app in Grab mode.
pub fn grab_app(world: &World) -> App {
    let mut app = app(world);
    apply(&mut app, world, Action::Mode(CursorMode::Grab));
    app
}

/// An app on `world` in the default theme, with only the map view, its
/// tiles drawn on `tiles`.
pub fn map_app(world: &World, tiles: Rect) -> App {
    App::new(world.map(), Theme::cp437(), world.seed(), map_only(tiles))
}

/// The smallest screen the game draws on (design §6.1).
pub const SCREEN: Size = Size::new(100, 30);

/// An app on `world` in the default theme, on a 100×30 screen.
pub fn app_on(world: &World) -> App {
    app_in(world, Theme::cp437())
}

/// An app on `world` in `theme`, on a 100×30 screen.
pub fn app_in(world: &World, theme: Theme) -> App {
    sized_app(world, theme, SCREEN)
}

/// An app on `world` in `theme`, with its panels laid out for a screen of
/// `size`. Its help screen shows [`DATA_FOLDER`].
pub fn sized_app(world: &World, theme: Theme, size: Size) -> App {
    let mut app = App::new(world.map(), theme, world.seed(), ui::areas(size));
    app.set_data_folder(PathBuf::from(DATA_FOLDER));
    app
}

/// One frame on a 100×30 screen.
pub fn render(app: &App, world: &World) -> Buffer {
    render_on(app, world, SCREEN)
}

/// One frame on a screen of `size`.
pub fn render_on(app: &App, world: &World, size: Size) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(size.width, size.height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// Row `y` of `screen` as text, without the spaces at its end.
pub fn row(screen: &Buffer, y: u16) -> String {
    let text: String = (0..screen.area.width)
        .map(|x| screen[(x, y)].symbol())
        .collect();
    text.trim_end().to_string()
}

/// The status line of a 100×30 screen, without the spaces at its end.
pub fn status_line(app: &App, world: &World) -> String {
    row(&render(app, world), SCREEN.height - 1)
}

/// The screen's text, a line per row, without the spaces at each line's
/// end.
pub fn lines(screen: &Buffer) -> Vec<String> {
    (0..screen.area.height).map(|y| row(screen, y)).collect()
}

/// A key event of `kind` with no modifiers: a press, a repeat or a
/// release.
pub fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
}

/// Carries out `action`, which leaves the game running.
pub fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

/// The screen cell the pointer is on to point at `tile` when the map view's
/// tiles are drawn from screen cell (0, 0), as `app` draws them: one down
/// and right of it (design v27 §6.5).
pub fn pointing_at(tile: Pos) -> Position {
    Position::new(tile.x + 1, tile.y + 1)
}

/// Points at `tile`, the map view's tiles drawn from screen cell (0, 0).
pub fn point(app: &mut App, world: &World, tile: Pos) {
    apply(app, world, Action::Point(pointing_at(tile)));
}

/// Clicks `button` on `tile`, the map view's tiles drawn from screen cell
/// (0, 0).
pub fn click(app: &mut App, world: &World, tile: Pos, button: Button) {
    let action = Action::Click {
        at: pointing_at(tile),
        button,
        amplified: false,
    };
    apply(app, world, action);
}

/// Has the Cursor follow the sprite on `tile`, with a middle click (design
/// v26 §6.5), the map view's tiles drawn from screen cell (0, 0).
pub fn follow(app: &mut App, world: &World, tile: Pos) {
    apply(app, world, Action::middle_click(pointing_at(tile)));
}

/// Hands the app's commands to the world and runs a tick, as a frame does.
pub fn tick(app: &mut App, world: &mut World) {
    for command in app.take_commands() {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, world);
}

/// Sprite `id` finished `verb`, aimed at nothing, with `outcome`, on `tick`.
pub fn finished(tick: u64, id: EntityId, verb: Verb, outcome: Outcome) -> Event {
    acted(tick, id, verb, None, outcome, UNHURT)
}

/// Nobody hurt.
pub const UNHURT: Hurt = Hurt {
    actor: false,
    target: false,
};

/// The actor hurt, by what it did.
pub const HURT_ITSELF: Hurt = Hurt {
    actor: true,
    target: false,
};

/// The sprite it was aimed at hurt, by what was done to it.
pub const HURT_TARGET: Hurt = Hurt {
    actor: false,
    target: true,
};

/// Aimed at sprite `id`: the target, and its type, a sprite's pseudo type
/// 101, for [`acted`].
pub fn at_sprite(id: EntityId) -> Option<(Target, u16)> {
    Some((Target::Sprite(id), 101))
}

/// Sprite `id` finished `verb` on `tick`, with `outcome`, aimed at
/// `target` of the object type given, if at anything, its attempt hurting
/// whom `hurt` says. An aimed action that applied made its attempt.
pub fn acted(
    tick: u64,
    id: EntityId,
    verb: Verb,
    target: Option<(Target, u16)>,
    outcome: Outcome,
    hurt: Hurt,
) -> Event {
    let action = ActionView {
        verb,
        destination: None,
        target: target.map(|(target, _)| target),
        target_type: target.map(|(_, target_type)| target_type),
        attempted: target.is_some() && outcome == Outcome::Applied,
        target_gone: false,
        hurt,
        progress: Progress::Ended(outcome),
        remembered: false,
        heard: None,
        egg_at: None,
    };
    Event {
        tick,
        kind: EventKind::ActionEnded {
            id,
            verb,
            outcome,
            action,
        },
    }
}

/// The Cursor touching sprite `id` on `tick`: a pet, hug, zap or shock by
/// name, or a pet or shock refused.
pub fn touched(tick: u64, id: EntityId, what: &str) -> Event {
    let kind = match what {
        "pet" => EventKind::Rewarded {
            id,
            amplified: false,
        },
        "hug" => EventKind::Rewarded {
            id,
            amplified: true,
        },
        "zap" => EventKind::Corrected {
            id,
            amplified: false,
        },
        "shock" => EventKind::Corrected {
            id,
            amplified: true,
        },
        "refused pet" => EventKind::CommandRejected {
            command: Command::Reward {
                sprite: id,
                amplified: false,
                reach_back: 3,
            },
            reason: Rejection::Gone,
        },
        "refused shock" => EventKind::CommandRejected {
            command: Command::Correct {
                sprite: id,
                amplified: true,
            },
            reason: Rejection::Gone,
        },
        _ => unreachable!("{what}"),
    };
    Event { tick, kind }
}

/// What to change in a data file: its name, and each text to replace in
/// it, with what.
pub type Changes = (&'static str, &'static [(&'static str, &'static str)]);

/// The built-in data pack with its files changed as `changes` say: a file's
/// name, and each text to replace in it, with what. Each file is one of the
/// pack's, and each text is in it, or the test fails, so a change can't
/// quietly miss and leave the test on the built-in data.
pub fn pack_changing(changes: &[(&str, &[(&str, &str)])]) -> DataPack {
    let sources = DataPack::builtin_sources();
    for (file, _) in changes {
        let known = sources.iter().any(|&(path, _)| path == *file);
        assert!(known, "{file} is no file of the pack");
    }
    let texts: Vec<(&'static str, String)> = sources
        .iter()
        .map(|&(path, builtin)| {
            let mut text = builtin.to_string();
            for (file, edits) in changes {
                if *file == path {
                    for (from, to) in edits.iter() {
                        text = changed(&text, from, to);
                    }
                }
            }
            (path, text)
        })
        .collect();
    let sources: Vec<(&'static str, &str)> = texts
        .iter()
        .map(|(path, text)| (*path, text.as_str()))
        .collect();
    DataPack::from_sources(&sources).expect("a valid pack")
}

/// `text`, a data file or genes, with `from` replaced by `to`. `from` is
/// in it, or the test fails, rather than going on with `text` unchanged.
pub fn changed(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "{from:?} isn't there to change");
    text.replace(from, to)
}

/// Changes that make generations quick: eggs copied exactly, hatching 20
/// ticks after they're laid, the young grown up as soon as their genes
/// say and thirsting as grown sprites do, and a mother free to lay again 30
/// ticks after her last egg.
pub const QUICK: [Changes; 2] = [
    (
        "physiology.ron",
        &[
            ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
            ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
            ("grows_up: (1500.0, 13500.0)", "grows_up: (10.0, 13500.0)"),
            ("thirst_childhood: 4500.0,", ""),
            ("rest: 1500,", "rest: 30,"),
            ("cost: 0.3,", "cost: 0.0,"),
        ],
    ),
    (
        "objects.ron",
        &[(
            "ticks: (750, 750), next: Hatch",
            "ticks: (20, 20), next: Hatch",
        )],
    ),
];

/// The change that lets a female breed alone at once, with a male in
/// sight or not (M2 design §15.7).
pub const LONE: Changes = (
    "physiology.ron",
    &[("alone_after: 4500,", "alone_after: 0,")],
);

/// Changes that make thirst kill a sprite with no water 85 ticks or so
/// after it's born.
pub const PARCHED: Changes = (
    "physiology.ron",
    &[
        ("hydration_loss: 0.00033", "hydration_loss: 0.012"),
        ("dehydration: 0.0011", "dehydration: 0.5"),
    ],
);

/// Changes that make thirst kill sooner: 35 ticks or so after a sprite's
/// born.
pub const SCORCHED: Changes = (
    "physiology.ron",
    &[
        ("hydration_loss: 0.00033", "hydration_loss: 0.03"),
        ("dehydration: 0.0011", "dehydration: 0.5"),
    ],
);

/// The change that makes a thornbush's thorns kill the sprite that hits
/// it, at once.
pub const LETHAL_THORNS: Changes = (
    "tags.ron",
    &[(
        r#"Hit:   [Inject(Actor, "injury", 0.03)"#,
        r#"Hit:   [Inject(Actor, "injury", 1.0)"#,
    )],
);

/// Genes for a sprite that turns female and lays alone whenever it can (M2
/// design §10.4).
pub const LAYS_ALONE: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "feminiser", value: 1.0),
    InitialConcentration(chem: "longing", value: 1.0),
    Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
])"#;

/// Genes for a sprite that grows up 10 ticks after it hatches, turns
/// female and lays alone whenever it can.
pub const LONE_MOTHER: &str = r#"(format: 1, genes: [
    Trait(trait: "speed", value: 7.0),
    Trait(trait: "sense_radius", value: 9.0),
    Trait(trait: "grows_up", value: 10.0),
    InitialConcentration(chem: "feminiser", value: 1.0),
    InitialConcentration(chem: "longing", value: 1.0),
    Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
])"#;

/// Genes for a sprite that grows up 10 ticks after it hatches and mates
/// with whatever sprite it sees, `female` or not, receptive from the start
/// as if courted (M2 design §15.3).
pub fn keen(female: bool) -> String {
    let feminiser = if female {
        r#"InitialConcentration(chem: "feminiser", value: 1.0),"#
    } else {
        ""
    };
    format!(
        r#"(format: 1, genes: [
    Trait(trait: "speed", value: 7.0),
    Trait(trait: "sense_radius", value: 9.0),
    Trait(trait: "grows_up", value: 10.0),
    {feminiser}
    InitialConcentration(chem: "receptivity", value: 1.0),
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
])"#
    )
}

/// The screen cell the pointer is on to point at map tile `tile` as `app`
/// shows it: one down and right of the cell the tile is drawn in (design
/// v27 §6.5).
pub fn pointer_on(app: &App, tile: Pos) -> Position {
    let cell = app.cell_of(tile).expect("in view");
    Position::new(cell.x + 1, cell.y + 1)
}

/// `n` with its thousands grouped, as the screen writes it: `13,489`.
pub fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A field of grass `width` by `height`, with the data changed as
/// `changes` say, and `sprites` the world began with, each with the genes
/// given, run `ticks` ticks.
pub fn grow(
    changes: &[Changes],
    (width, height): (usize, usize),
    sprites: &[(Pos, &str)],
    ticks: u64,
) -> World {
    let mut world = WorldBuilder::grass(width, height)
        .genes(sprites)
        .data(pack_changing(changes))
        .seed(7)
        .build();
    run(&mut world, ticks);
    world
}

/// One sprite the world began with, who lays alone, as do her young, run
/// `ticks` ticks. By tick 200, #1 has six young: #2, #3, #7, #12, #25 and
/// the egg #45.
pub fn lone_line(ticks: u64) -> World {
    let [quick, eggs] = QUICK;
    grow(
        &[quick, eggs, LONE],
        (12, 6),
        &[(at(1, 4), LONE_MOTHER)],
        ticks,
    )
}

/// The same, with no water, so each sprite dies of thirst about 85 ticks
/// after it's born. By tick 140, #1, #2, #3 and #4 have died, and #3's
/// young #5 lives.
pub fn parched_line(ticks: u64) -> World {
    let [quick, eggs] = QUICK;
    grow(
        &[quick, eggs, LONE, PARCHED],
        (12, 6),
        &[(at(1, 4), LONE_MOTHER)],
        ticks,
    )
}

/// Two pairs the world began with, #1 and #3 females, #2 and #4 males,
/// who mate with whatever sprite they see, kin or not, as their young do,
/// run `ticks` ticks. By tick 230, #14 is the young of #1 and #2, with
/// young by #1 and by its partner #8 (M2 design §15.4), and #16 is the
/// young of #3 and of #12, who is #3's young too.
pub fn pairs(ticks: u64) -> World {
    let (female, male) = (keen(true), keen(false));
    let sprites = [
        (at(2, 2), female.as_str()),
        (at(3, 2), male.as_str()),
        (at(12, 5), female.as_str()),
        (at(13, 5), male.as_str()),
    ];
    grow(&QUICK, (16, 8), &sprites, ticks)
}

/// Selects the sprite `id` with `Tab`, as the player would.
pub fn select(app: &mut App, world: &World, id: EntityId) {
    for _ in 0..world.sprites().count() {
        if app.selection() == Some(Selection::Living(id)) {
            return;
        }
        app.apply(Action::SelectNext, world);
    }
    assert_eq!(
        app.selection(),
        Some(Selection::Living(id)),
        "Tab reaches it"
    );
}

/// Selects `id` and opens its family tree (M2 design §17.2).
pub fn tree_of(app: &mut App, world: &World, id: EntityId) {
    select(app, world, id);
    app.apply(Action::FamilyTree, world);
    assert_eq!(app.screen(), Screen::FamilyTree);
}

/// Presses `]` until `tab` is open.
pub fn open(app: &mut App, world: &World, tab: Tab) {
    while app.tab() != tab {
        app.apply(Action::NextTab, world);
    }
}

/// The inspector on a 100×`height` screen: its title row, then each row
/// inside its border, the border and trailing spaces left off and the
/// margin kept. At 100 columns it's the right-hand 46, above the event
/// log's 7 rows.
pub fn inspector_rows(app: &App, world: &World, height: u16) -> (String, Vec<String>) {
    let size = Size::new(100, height);
    let screen = render_on(app, world, size);
    let rows: Vec<String> = (0..size.height)
        .map(|y| {
            let row: String = (size.width - 46..size.width)
                .map(|x| screen[(x, y)].symbol())
                .collect();
            row.trim_end().to_string()
        })
        .collect();
    let inside = rows[2..rows.len() - 7]
        .iter()
        .map(|row| {
            let row = row.trim_start_matches('│').trim_end_matches('│');
            row.trim_end().to_string()
        })
        .collect();
    (rows[1].clone(), inside)
}

/// `rows` from the first that starts with `start` after its margin.
pub fn rows_from<'a>(rows: &'a [String], start: &str) -> &'a [String] {
    let at = rows
        .iter()
        .position(|row| row.trim_start().starts_with(start))
        .unwrap_or_else(|| panic!("a row starting {start:?} in {rows:#?}"));
    &rows[at..]
}

/// The inspector's line on a 100×30 screen that starts with `start` after
/// its margin, without the margin.
pub fn inspector_line(app: &App, world: &World, start: &str) -> String {
    let (_, rows) = inspector_rows(app, world, 30);
    rows_from(&rows, start)[0].trim_start().to_string()
}

/// `Q`: what a plain left click does, where the Cursor is (design v21
/// §6.5).
pub const Q: Action = Action::Press {
    button: Button::Left,
    amplified: false,
};

/// `E`: what a plain right click does, where the Cursor is.
pub const E: Action = Action::Press {
    button: Button::Right,
    amplified: false,
};

/// Sprite `id`, unnamed, died of `cause` at `age`, on `tick`.
pub fn died(tick: u64, id: EntityId, cause: terra_sim::DeathCause, age: u64) -> Event {
    Event {
        tick,
        kind: EventKind::Died {
            id,
            name: None,
            cause,
            age,
        },
    }
}

/// Each drive of the built-in pack, in its order, with the colour a sprite
/// takes on the map in both built-in themes when it's the strongest
/// (design §6.3, arousal M2 design §10, and anxiety §15.8).
pub const DRIVE_COLOURS: [(&str, ratatui::style::Color); 9] = {
    use ratatui::style::Color;
    [
        ("hunger", Color::Yellow),
        ("thirst", Color::Cyan),
        ("pain", Color::Red),
        ("tiredness", Color::Blue),
        ("boredom", Color::DarkGray),
        ("loneliness", Color::Magenta),
        ("crowdedness", Color::LightRed),
        ("arousal", Color::LightMagenta),
        ("anxiety", Color::LightGreen),
    ]
};
