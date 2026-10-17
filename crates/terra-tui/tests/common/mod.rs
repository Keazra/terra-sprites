//! Shared helpers for terra-tui integration tests: worlds where
//! generations come quickly, for the family tree (M2 design §17).
//!
//! Each test file is its own crate, so it `mod common;` and uses only the
//! pieces it needs. Helpers a given file never calls are allowed here so the
//! shared API stays one place, as in terra-sim's (#155).
#![allow(dead_code)]

use ratatui::layout::{Position, Size};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, EntityId, Genome, Map, Pos, Scenario, World};
use terra_tui::app::{App, Screen, Selection, Tab};
use terra_tui::input::Action;
use terra_tui::ui;

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
/// say, and a mother free to lay again 30 ticks after her last egg.
pub const QUICK: [Changes; 2] = [
    (
        "physiology.ron",
        &[
            ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
            ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
            ("grows_up: (1500.0, 13500.0)", "grows_up: (10.0, 13500.0)"),
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

pub fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
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
    let pack = pack_changing(changes);
    let row = ".".repeat(width);
    let map = Map::from_ascii(&vec![row.as_str(); height], &pack).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|&(pos, genes)| {
            (
                pos,
                Some(Genome::from_ron(genes, &pack).expect("valid genes")),
            )
        })
        .collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
        newborns: &[],
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    for _ in 0..ticks {
        world.step();
    }
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
    let mut terminal = Terminal::new(TestBackend::new(size.width, size.height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    let screen = terminal.backend().buffer();
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
