//! What the screen may show (design §6.4): every display asks the app's
//! `InfoPolicy` first.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, Command, DataPack, DeathCause, EntityId, Event, EventKind, Grip, Hurt, Map,
    Outcome, Pos, Progress, Rejection, Scenario, ScriptedAction, Target, Verb, World,
};
use terra_tui::app::{App, CursorMode, Tab};
use terra_tui::input::Action;
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A policy that shows nothing.
struct Blind;

impl InfoPolicy for Blind {
    fn can_view(&self, _panel: Panel, _subject: Subject) -> bool {
        false
    }
}

/// A policy that shows everything but the Cursor's own lines about one
/// subject.
struct HideCursorLinesAbout(Subject);

impl InfoPolicy for HideCursorLinesAbout {
    fn can_view(&self, panel: Panel, subject: Subject) -> bool {
        panel != Panel::CursorLines || subject != self.0
    }
}

/// Two starter sprites in a field, the one at (2, 3) wandering to (8, 3),
/// and a berry bush at (1, 1).
fn field() -> World {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let objects = [(Pos { x: 1, y: 1 }, "berry_bush")];
    let sprites = [(Pos { x: 2, y: 3 }, None), (Pos { x: 6, y: 1 }, None)];
    let scripted = [(
        Pos { x: 2, y: 3 },
        ScriptedAction::Wander {
            destination: Pos { x: 8, y: 3 },
        },
    )];
    let scenario = Scenario {
        map,
        objects: &objects,
        sprites: &sprites,
        scripted: &scripted,
        newborns: &[],
    };
    let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    world.step();
    world
}

fn render(app: &App, world: &World) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn row(buffer: &Buffer, y: u16, xs: std::ops::Range<u16>) -> String {
    xs.map(|x| buffer[(x, y)].symbol()).collect()
}

/// The text inside the inspector's border, trimmed.
fn inspector_text(buffer: &Buffer) -> String {
    (2..23)
        .map(|y| row(buffer, y, 55..99).trim().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text inside the event log's border, trimmed.
fn event_log_text(buffer: &Buffer) -> String {
    (25..28)
        .map(|y| row(buffer, y, 1..99).trim().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The screen cell tile `tile` of a 10×5 field is drawn on, on a 100×30
/// screen: the field is small, so it's in the middle of the map view (M2
/// design §11.1).
fn cell(tile: Pos) -> Position {
    let map = Map::from_ascii(&[".........."; 5], &pack()).expect("valid drawing");
    let app = App::new(&map, Theme::cp437(), 7, ui::areas(Size::new(100, 30)));
    app.cell_of(tile).expect("in view")
}

/// The screen cell the pointer is on to point at `tile`: one down and right
/// of it (design v27 §6.5).
fn pointer_on(tile: Pos) -> Position {
    let at = cell(tile);
    Position::new(at.x + 1, at.y + 1)
}

/// Row `y` of a 10×5 field's tiles, as drawn.
fn map_row(buffer: &Buffer, y: u16) -> String {
    let start = cell(Pos { x: 0, y });
    row(buffer, start.y, start.x..start.x + 10)
}

/// The sprite at (2, 3).
const WANDERER: Pos = Pos { x: 2, y: 3 };

/// An app on `world` with the wanderer selected, hurt, a death and a hurt
/// in the event log, and the Cursor on the wanderer.
fn busy_app(world: &World) -> App {
    let areas = ui::areas(Size::new(100, 30));
    let mut app = App::new(world.map(), Theme::cp437(), 7, areas);
    let wanderer = world.sprite_at(Pos { x: 2, y: 3 }).unwrap().id();
    app.apply(Action::left_click(pointer_on(WANDERER)), world);
    let hurt = ActionView {
        verb: Verb::Hit,
        destination: None,
        target: Some(Target::Sprite(wanderer)),
        target_type: Some(101),
        attempted: true,
        target_gone: false,
        hurt: Hurt {
            actor: false,
            target: true,
        },
        progress: Progress::Ended(Outcome::Applied),
        remembered: false,
    };
    let events = [
        Event {
            tick: 1,
            kind: EventKind::Died {
                id: EntityId(99),
                name: None,
                cause: DeathCause::Starvation,
                age: 400,
            },
        },
        Event {
            tick: 1,
            kind: EventKind::ActionEnded {
                id: EntityId(98),
                verb: Verb::Hit,
                outcome: Outcome::Applied,
                action: hurt,
            },
        },
    ];
    app.record(&events, world);
    app
}

#[test]
fn every_display_asks_the_policy_and_shows_everything_in_m1() {
    let world = field();
    let mut app = busy_app(&world);
    let screen = render(&app, &world);
    assert!(row(&screen, 0, 0..100).contains("sprites 2"));
    assert!(event_log_text(&screen).contains("died"));
    assert!(row(&screen, 29, 0..100).starts_with(" (2,3) grass · Sprite #"));
    assert_eq!(screen[cell(WANDERER)].symbol(), "!", "the hurt emote");
    assert!(map_row(&screen, 3).contains('X'), "the Decision marker");
    for tab in Tab::ALL {
        while app.tab() != tab {
            app.apply(Action::NextTab, &world);
        }
        assert!(
            !inspector_text(&render(&app, &world)).is_empty(),
            "{tab:?} shows something"
        );
    }
    app.apply(Action::SpriteList, &world);
    let list = render(&app, &world);
    assert!(
        row(&list, 3, 0..100).starts_with("│ Sprite #"),
        "a row per sprite"
    );
    assert!(row(&list, 4, 0..100).starts_with("│ Sprite #"));
    app.apply(Action::Back, &world);
    app.apply(Action::Population, &world);
    let graph = render(&app, &world);
    assert!(
        row(&graph, 2, 0..100).starts_with("│ 100% ┤"),
        "the population graph: {}",
        row(&graph, 2, 0..100)
    );
}

#[test]
fn a_policy_that_denies_everything_blanks_every_display() {
    let world = field();
    let mut app = busy_app(&world);
    app.set_policy(Blind);
    let screen = render(&app, &world);
    let top = row(&screen, 0, 0..100);
    assert!(!top.contains("sprites"), "no population: {top}");
    assert_eq!(event_log_text(&screen).trim(), "", "an empty event log");
    let status = row(&screen, 29, 0..100);
    assert!(
        status.starts_with(" SELECT"),
        "no tile under the Cursor: {status}"
    );
    assert_eq!(screen[cell(WANDERER)].symbol(), "☻", "no emote");
    let title = row(&screen, 1, 54..100);
    assert!(
        !title.contains('#'),
        "the inspector's title names no sprite: {title}"
    );
    let map = (0..5)
        .map(|y| map_row(&screen, y))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!map.contains('X'), "no Decision marker:\n{map}");
    let shaded = (0..5)
        .any(|y| (0..10).any(|x| screen[cell(Pos { x, y })].bg == app.theme.attention_marker()));
    assert!(!shaded, "no Attention marker");
    for tab in Tab::ALL {
        while app.tab() != tab {
            app.apply(Action::NextTab, &world);
        }
        assert_eq!(
            inspector_text(&render(&app, &world)).trim(),
            "",
            "{tab:?} is blank"
        );
    }
    app.apply(Action::SpriteList, &world);
    let list = render(&app, &world);
    assert!(
        row(&list, 2, 0..100).starts_with("│ No sprites "),
        "no rows in the sprite list"
    );
    app.apply(Action::Back, &world);
    app.apply(Action::Population, &world);
    let graph = render(&app, &world);
    for y in 2..28 {
        let inside = row(&graph, y, 0..100);
        assert_eq!(inside.trim_matches(['│', ' ']), "", "no population graph");
    }
}

#[test]
fn a_policy_that_hides_map_colours_draws_sprites_in_their_own_colour() {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let hungry = terra_sim::Genome::from_ron(
        r#"(format: 1, genes: [InitialConcentration(chem: "hunger", value: 0.9)])"#,
        &pack,
    )
    .expect("a valid genome");
    let sprites = [(Pos { x: 2, y: 3 }, Some(hungry))];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
        newborns: &[],
    };
    let world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let areas = ui::areas(Size::new(100, 30));
    let mut app = App::new(world.map(), Theme::cp437(), 7, areas);
    app.apply(Action::Point(pointer_on(Pos { x: 9, y: 4 })), &world);
    assert_eq!(render(&app, &world)[cell(WANDERER)].fg, Color::Yellow);
    app.set_policy(Blind);
    assert_eq!(render(&app, &world)[cell(WANDERER)].fg, Color::White);
}

#[test]
fn the_cursors_own_lines_ask_the_policy_about_the_sprite_they_name() {
    // Design §6.4 (#174): what the Cursor follows, leads or holds, the
    // prompt for naming a sprite, and why a click was refused show only if
    // the policy lets the Cursor's own lines be about what they name.
    // Hiding something else leaves them be.
    let world = field();
    let wanderer = world.sprite_at(Pos { x: 2, y: 3 }).unwrap().id();
    let other = world.sprites().map(|s| s.id()).find(|&id| id != wanderer);
    let other = other.expect("a second sprite");
    let hide = |id| HideCursorLinesAbout(Subject::Sprite(id));
    let on_wanderer = pointer_on(WANDERER);
    let status = |app: &App| row(&render(app, &world), 29, 0..100);
    let areas = ui::areas(Size::new(100, 30));

    let mut following = App::new(world.map(), Theme::cp437(), 7, areas);
    following.apply(Action::left_click(on_wanderer), &world);
    following.apply(Action::Follow { at: None }, &world);
    assert!(status(&following).contains("│ following Sprite #"));
    following.set_policy(hide(other));
    assert!(status(&following).contains("│ following Sprite #"));
    following.set_policy(hide(wanderer));
    let line = status(&following);
    assert!(!line.contains("following"), "{line}");

    let mut leading = App::new(world.map(), Theme::cp437(), 7, areas);
    leading.apply(Action::Mode(CursorMode::Grab), &world);
    leading.apply(Action::left_click(on_wanderer), &world);
    assert!(status(&leading).contains("│ leading: Sprite #"));
    leading.set_policy(hide(other));
    assert!(status(&leading).contains("│ leading: Sprite #"));
    leading.set_policy(hide(wanderer));
    let line = status(&leading);
    assert!(!line.contains("leading"), "{line}");

    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[(Pos { x: 4, y: 1 }, "ball")],
        sprites: &[],
        scripted: &[],
        newborns: &[],
    };
    let ball_field = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let holding_line = |app: &App| row(&render(app, &ball_field), 29, 0..100);
    let mut holding = App::new(ball_field.map(), Theme::cp437(), 7, areas);
    holding.apply(Action::Mode(CursorMode::Grab), &ball_field);
    holding.apply(
        Action::left_click(pointer_on(Pos { x: 4, y: 1 })),
        &ball_field,
    );
    let line = holding_line(&holding);
    assert!(line.contains("│ holding: ball"), "{line}");
    holding.set_policy(hide(wanderer));
    let line = holding_line(&holding);
    assert!(line.contains("│ holding: ball"), "{line}");
    holding.set_policy(HideCursorLinesAbout(Subject::World));
    let line = holding_line(&holding);
    assert!(!line.contains("holding"), "{line}");

    let mut naming = App::new(world.map(), Theme::cp437(), 7, areas);
    naming.apply(Action::left_click(on_wanderer), &world);
    naming.apply(Action::Rename, &world);
    assert!(status(&naming).starts_with(" Name Sprite #"));
    naming.set_policy(hide(other));
    assert!(status(&naming).starts_with(" Name Sprite #"));
    naming.set_policy(hide(wanderer));
    let line = status(&naming);
    assert!(line.starts_with(" Name: "), "{line}");
    assert!(!line.contains("Sprite #"), "{line}");

    // A refusal names both sprites: the one the click took hold of, and the
    // one already led.
    let mut refused = App::new(world.map(), Theme::cp437(), 7, areas);
    let rejected = Event {
        tick: 1,
        kind: EventKind::CommandRejected {
            command: Command::TakeHold { sprite: wanderer },
            reason: Rejection::Busy(Grip::Leads(other)),
        },
    };
    refused.record(&[rejected], &world);
    assert!(status(&refused).contains("already leading Sprite #"));
    let third = HideCursorLinesAbout(Subject::Sprite(EntityId(999)));
    refused.set_policy(third);
    assert!(status(&refused).contains("already leading Sprite #"));
    for named in [wanderer, other] {
        refused.set_policy(hide(named));
        let line = status(&refused);
        assert!(!line.contains("leading"), "{named:?} hidden: {line}");
    }
}
