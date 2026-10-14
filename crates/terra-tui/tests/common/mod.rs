//! Shared helpers for terra-tui integration tests: worlds where
//! generations come quickly, for the family tree (M2 design §16).
//!
//! Each test file is its own crate, so it `mod common;` and uses only the
//! pieces it needs. Helpers a given file never calls are allowed here so the
//! shared API stays one place, as in terra-sim's (#155).
#![allow(dead_code)]

use terra_sim::{DataPack, EntityId, Genome, Map, Pos, Scenario, World};
use terra_tui::app::{App, Screen, Selection};
use terra_tui::input::Action;

/// What to change in a data file: its name, and each text to replace in
/// it, with what.
pub type Changes = (&'static str, &'static [(&'static str, &'static str)]);

/// The built-in data pack with its files changed as `changes` say.
pub fn pack_changing(changes: &[Changes]) -> DataPack {
    let texts: Vec<(&'static str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, builtin)| {
            let mut text = builtin.to_string();
            for (file, edits) in changes {
                if *file == path {
                    for (from, to) in edits.iter() {
                        assert!(text.contains(from), "{file}: {from}");
                        text = text.replace(from, to);
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

/// Changes that make generations quick: eggs copied exactly, hatching 20
/// ticks after they're laid, the young grown up as soon as their genes
/// say, and a mother free to lay again 30 ticks after her last egg.
pub const QUICK: [Changes; 2] = [
    (
        "physiology.ron",
        &[
            ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
            ("flaws: (arise: 0.007,", "flaws: (arise: 0.0,"),
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
/// with whatever sprite it sees, `female` or not.
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
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
])"#
    )
}

pub fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
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
/// `ticks` ticks. By tick 200, #1 has seven young: #2, #4, #8, #10, #26,
/// #52 and the egg #72.
pub fn lone_line(ticks: u64) -> World {
    grow(&QUICK, (12, 6), &[(at(1, 4), LONE_MOTHER)], ticks)
}

/// The same, with no water, so each sprite dies of thirst about 85 ticks
/// after it's born. By tick 140, #1, #2, #3 and #4 have died, and #4's
/// young #5 lives.
pub fn parched_line(ticks: u64) -> World {
    let [quick, eggs] = QUICK;
    grow(
        &[quick, eggs, PARCHED],
        (12, 6),
        &[(at(1, 4), LONE_MOTHER)],
        ticks,
    )
}

/// Two pairs the world began with, #1 and #3 females, #2 and #4 males,
/// who mate with whatever sprite they see, as their young do, run `ticks`
/// ticks. By tick 200, #9 is the young of #1 and #2, with young by #10 and
/// by #3, and #16 is the young of #3 and of #13, who is #3's young too.
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

/// Selects `id` and opens its family tree (M2 design §16.2).
pub fn tree_of(app: &mut App, world: &World, id: EntityId) {
    for _ in 0..world.sprites().count() {
        if app.selection().map(Selection::id) == Some(id) {
            break;
        }
        app.apply(Action::SelectNext, world);
    }
    assert_eq!(app.selection().map(Selection::id), Some(id));
    app.apply(Action::FamilyTree, world);
    assert_eq!(app.screen(), Screen::FamilyTree);
}
