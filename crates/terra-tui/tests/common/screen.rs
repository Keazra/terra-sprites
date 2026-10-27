//! Helpers for the tests of what the screen draws, which `ui.rs`,
//! `inspector_tabs.rs` and `event_log.rs` share (#136): screens of any
//! size, the worlds and sprites they show, and the inspector's lines.

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect, Size};
use terra_sim::{DataPack, EntityId, Event, EventKind, Pos, ScriptedAction, World, WorldConfig};
use terra_tui::app::App;
use terra_tui::input::Action;
use terra_tui::theme::Theme;

use super::{
    LONE, SCREEN, WorldBuilder, at, inspector_rows, pack, pack_changing, render_on, run, sized_app,
};

pub fn generated_world() -> World {
    World::new(WorldConfig::builtin(&pack()), pack(), 7)
}

/// A UI for `world` sized for a `width`×`height` screen. A smaller screen
/// than 100×30 is a corner of one that size, with every panel: see
/// `render`.
pub fn app_for(world: &World, theme: Theme, width: u16, height: u16) -> App {
    sized_app(world, theme, at_least_min(width, height))
}

/// A screen of at least the smallest size with every panel.
pub fn at_least_min(width: u16, height: u16) -> Size {
    Size::new(width.max(SCREEN.width), height.max(SCREEN.height))
}

/// One frame on a `width`×`height` screen. Smaller than 100×30, where the
/// panels step down (design §6.1), it's the top-left `width`×`height` of a
/// frame that size, with its status line as the last row: so the tests of
/// small maps read as they did before the minimum. A test of the steps
/// down draws on a screen its own size, with `render_on`.
pub fn render(app: &App, world: &World, width: u16, height: u16) -> Buffer {
    let full = at_least_min(width, height);
    let drawn = render_on(app, world, full);
    if Size::new(width, height) == full {
        return drawn;
    }
    let mut corner = Buffer::empty(Rect::new(0, 0, width, height));
    for y in 0..height {
        let from = if y + 1 == height { full.height - 1 } else { y };
        for x in 0..width {
            corner[(x, y)] = drawn[(x, from)].clone();
        }
    }
    corner
}

/// The cells of `hint`, such as `. step`, where it first shows on the
/// status line, the bottom row of `screen`.
pub fn hint_cells<'a>(screen: &'a Buffer, hint: &str) -> Vec<&'a Cell> {
    let y = screen.area.height - 1;
    let line: String = (0..screen.area.width)
        .map(|x| screen[(x, y)].symbol())
        .collect();
    let at = line
        .find(hint)
        .unwrap_or_else(|| panic!("no {hint:?} in {line:?}"));
    let x = line[..at].chars().count() as u16;
    (0..hint.chars().count() as u16)
        .map(|i| &screen[(x + i, y)])
        .collect()
}

/// The screen cell the pointer is on to point at `world`'s map tile (x, y)
/// on a 100×30 screen, not scrolled, as `cell` places the tile.
pub fn pointer_on(world: &World, x: u16, y: u16) -> Position {
    let app = app_for(world, Theme::cp437(), 100, 30);
    super::pointer_on(&app, at(x, y))
}

/// The garden's objects: a berry bush, a thornbush, a berry and a ball.
pub const GARDEN: [(Pos, &str); 4] = [
    (at(1, 1), "berry_bush"),
    (at(4, 1), "thornbush"),
    (at(7, 1), "berry"),
    (at(6, 3), "ball"),
];

/// A 10×5 field of grass with a berry bush, a thornbush, a berry and a ball.
pub fn garden(pack: DataPack) -> World {
    WorldBuilder::grass(10, 5)
        .objects(&GARDEN)
        .data(pack)
        .seed(7)
        .build()
}

/// The garden, with starter sprites at (2, 3) and on the berry at (7, 1).
pub fn garden_with_sprites() -> World {
    WorldBuilder::grass(10, 5)
        .objects(&GARDEN)
        .starters(&[at(2, 3), at(7, 1)])
        .seed(7)
        .build()
}

/// The text inside a bordered row, without the borders or trailing spaces.
pub fn inside(row: &str) -> &str {
    row.trim_start_matches('│').trim_end_matches('│').trim()
}

/// The event log's three lines on a screen of `rows`, as from `lines`:
/// the rows between its title row and the status line, trimmed (design
/// §6.1).
pub fn log_rows(rows: &[String]) -> Vec<String> {
    rows[rows.len() - 4..rows.len() - 1]
        .iter()
        .map(|row| row.trim().to_string())
        .collect()
}

/// The inspector's rows on a 100×30 screen: its title row, then the open
/// tab's lines, trimmed.
pub fn inspector(app: &App, world: &World) -> (String, Vec<String>) {
    inspector_on(app, world, 30)
}

/// `inspector`, on a screen `height` rows tall.
pub fn inspector_on(app: &App, world: &World, height: u16) -> (String, Vec<String>) {
    let (title, rows) = inspector_rows(app, world, height);
    let text = rows
        .iter()
        .map(|row| row.trim_start().to_string())
        .collect();
    (title, text)
}

/// A 10×5 field of grass with one sprite at (2, 3), made from `genome`, and
/// an app on it that has selected the sprite and stepped the world `ticks`
/// times.
pub fn one_sprite(genome: &str, ticks: u64) -> (World, App) {
    one_sprite_doing(genome, &[], ticks)
}

/// `one_sprite`, starting on the `scripted` actions.
pub fn one_sprite_doing(genome: &str, scripted: &[ScriptedAction], ticks: u64) -> (World, App) {
    one_sprite_among(genome, &[], scripted, ticks)
}

/// `one_sprite_doing`, with `objects` in the field.
pub fn one_sprite_among(
    genome: &str,
    objects: &[(Pos, &str)],
    scripted: &[ScriptedAction],
    ticks: u64,
) -> (World, App) {
    one_sprite_in(pack(), genome, objects, scripted, ticks)
}

/// `one_sprite_among`, in `pack`.
pub fn one_sprite_in(
    pack: DataPack,
    genome: &str,
    objects: &[(Pos, &str)],
    scripted: &[ScriptedAction],
    ticks: u64,
) -> (World, App) {
    let start = at(2, 3);
    let scripted: Vec<(Pos, ScriptedAction)> = scripted.iter().map(|&a| (start, a)).collect();
    let mut world = WorldBuilder::grass(10, 5)
        .objects(objects)
        .genes(&[(start, genome)])
        .scripts(&scripted)
        .data(pack)
        .seed(7)
        .build();
    run(&mut world, ticks);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    (world, app)
}

/// A genome that walks a grass step a tick.
pub const SPEED_10: &str = r#"(format: 1, genes: [Trait(trait: "speed", value: 10.0)])"#;

/// A hungry sprite whose instincts point it at berries and to eating, with
/// a nudge against eating for no reason and a mild habit of wandering, and
/// no curiosity, so its attention is its instincts and nearness alone.
pub const HUNGRY_GENOME: &str = r#"(format: 1, genes: [
    InitialConcentration(chem: "hunger", value: 0.8),
    BrainParam(param: "curiosity", value: 0.0),
    BrainParam(param: "tau_base", value: 0.05),
    BrainParam(param: "tau_att_base", value: 0.05),
    AttentionInstinct(input: "hunger", category: "fruit", weight: 1.0),
    Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
    Instinct(inputs: [("hunger", false), ("target_adjacent", true)], verb: Eat, weight: 0.5),
    Instinct(inputs: [("always", false)], verb: Eat, weight: -0.1),
    Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
    Instinct(inputs: [("target_adjacent", true)], verb: Eat, weight: 0.004),
])"#;

/// The Cursor's events for sprite 12, at `tick`: a pet, hug, zap or shock by
/// name, or one refused.
pub fn touched(tick: u64, what: &str) -> Event {
    super::touched(tick, EntityId(12), what)
}

/// A world whose one sprite, by `physiology`'s rules, the built-in one's
/// changed as `changes` say, has laid an egg alone, and the egg. She may
/// breed alone at once (M2 design §15.7).
pub fn a_lone_egg(changes: &[(&str, &str)]) -> (World, EntityId) {
    let genome = r#"(format: 1, genes: [
        Trait(trait: "speed", value: 7.0),
        Trait(trait: "sense_radius", value: 9.0),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "hunger"),
        Emitter(locus: Chem("hydration"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "thirst"),
        InitialConcentration(chem: "feminiser", value: 1.0),
        InitialConcentration(chem: "longing", value: 1.0),
        Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
    ])"#;
    let mut world = WorldBuilder::grass(10, 5)
        .genes(&[(at(1, 4), genome)])
        .data(pack_changing(&[LONE, ("physiology.ron", changes)]))
        .seed(7)
        .build();
    let egg = (0..50)
        .flat_map(|_| world.step())
        .find_map(|event| match event.kind {
            EventKind::Laid { egg, .. } => Some(egg),
            _ => None,
        })
        .expect("she laid an egg");
    (world, egg)
}

/// The built-in physiology's copy and change, as its file begins it.
pub const COPY_AND_CHANGE: &str =
    "copy_and_change: (doubled: 0.00029, lost: 0.00029, changed: 0.00029,";
