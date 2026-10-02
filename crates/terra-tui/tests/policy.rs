//! What the screen may show (design §6.4): every display asks the app's
//! `InfoPolicy` first.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, DataPack, DeathCause, EntityId, Event, EventKind, Hurt, Map, Outcome, Pos,
    Progress, Scenario, ScriptedAction, Target, Verb, World,
};
use terra_tui::app::{App, Tab};
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

/// The sprite at (2, 3), on the screen cell (3, 5).
const WANDERER: Position = Position::new(3, 5);

/// An app on `world` with the wanderer selected, hurt, a death and a hurt
/// in the event log, and the Cursor on the wanderer.
fn busy_app(world: &World) -> App {
    let areas = ui::areas(Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), Theme::cp437(), 7, areas);
    let wanderer = world.sprite_at(Pos { x: 2, y: 3 }).unwrap().id();
    app.apply(Action::left_click(Position::new(4, 6)), world);
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
    assert_eq!(screen[WANDERER].symbol(), "!", "the hurt emote");
    assert!(row(&screen, 5, 1..11).contains('X'), "the Decision marker");
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
    assert_eq!(screen[WANDERER].symbol(), "☻", "no emote");
    let title = row(&screen, 1, 54..100);
    assert!(
        !title.contains('#'),
        "the inspector's title names no sprite: {title}"
    );
    let map = (2..7)
        .map(|y| row(&screen, y, 1..11))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!map.contains('X'), "no Decision marker:\n{map}");
    let shaded = (2..7).any(|y| (1..11).any(|x| screen[(x, y)].bg == app.theme.attention_marker()));
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
    };
    let world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
    let areas = ui::areas(Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), Theme::cp437(), 7, areas);
    app.apply(Action::Point(Position::new(11, 7)), &world);
    assert_eq!(render(&app, &world)[WANDERER].fg, Color::Yellow);
    app.set_policy(Blind);
    assert_eq!(render(&app, &world)[WANDERER].fg, Color::White);
}
