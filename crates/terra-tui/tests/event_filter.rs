//! The event log's filter (design §6.1): all events, the selected sprite's,
//! or major ones only, switched with `m` or a click on its label.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, Command, DataPack, DeathCause, EntityId, Event, EventKind, Hurt, Learned, Map,
    Outcome, Pos, Progress, Rejection, Scenario, Target, Verb, World,
};
use terra_tui::app::{App, EventFilter};
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// Sprites #1 and #2 in a field.
fn field() -> World {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let sprites = [(Pos { x: 2, y: 3 }, None), (Pos { x: 6, y: 1 }, None)];
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, pack, 7).expect("valid scenario")
}

fn render(app: &App, world: &World) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn row(screen: &Buffer, y: u16) -> String {
    (0..100).map(|x| screen[(x, y)].symbol()).collect()
}

/// The event log's lines, without their ticks.
fn log(app: &App, world: &World) -> Vec<String> {
    let screen = render(app, world);
    (25..28)
        .map(|y| (1..99).map(|x| screen[(x, y)].symbol()).collect::<String>())
        .map(|line| {
            line.trim()
                .split_once("  ")
                .map_or("", |(_, text)| text)
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn hit(tick: u64, hitter: u64, hit: u64) -> Event {
    let action = ActionView {
        verb: Verb::Hit,
        destination: None,
        target: Some(Target::Sprite(EntityId(hit))),
        target_type: Some(101),
        attempted: true,
        target_gone: false,
        hurt: Hurt::default(),
        progress: Progress::Ended(Outcome::Applied),
    };
    Event {
        tick,
        kind: EventKind::ActionEnded {
            id: EntityId(hitter),
            verb: Verb::Hit,
            outcome: Outcome::Applied,
            action,
        },
    }
}

/// An app on `world` whose log has, oldest first: #2 learning a lesson, a
/// refused pet of #2, #1 hitting #2, a pet of #1, #3 hitting #4, and #3's
/// death.
fn logged(world: &World) -> App {
    let areas = ui::areas(Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), Theme::cp437(), 7, areas);
    let events = [
        Event {
            tick: 1,
            kind: EventKind::LearnedMilestone {
                id: EntityId(2),
                learned: Learned::Fear {
                    thing: terra_sim::Thing::Sprite(EntityId(1)),
                },
                good: false,
            },
        },
        Event {
            tick: 2,
            kind: EventKind::CommandRejected {
                command: Command::Correct {
                    sprite: EntityId(2),
                    amplified: false,
                },
                reason: Rejection::Gone,
            },
        },
        hit(3, 1, 2),
        Event {
            tick: 4,
            kind: EventKind::Rewarded {
                id: EntityId(1),
                amplified: false,
            },
        },
        hit(5, 3, 4),
        Event {
            tick: 6,
            kind: EventKind::Died {
                id: EntityId(3),
                name: None,
                cause: DeathCause::OldAge,
                age: 900,
            },
        },
    ];
    // The log shows three lines, so the filters are checked in turn.
    app.record(&events, world);
    app
}

#[test]
fn the_filter_starts_on_all_and_m_goes_round_selected_and_major() {
    let world = field();
    let mut app = logged(&world);
    assert_eq!(app.event_filter(), EventFilter::All);
    assert_eq!(
        log(&app, &world),
        [
            "Sprite #3 died (old age, age 900)",
            "Sprite #3 hit Sprite #4",
            "You petted Sprite #1",
        ]
    );
    app.apply(Action::CycleEventFilter, &world);
    assert_eq!(app.event_filter(), EventFilter::Selected);
    app.apply(Action::CycleEventFilter, &world);
    assert_eq!(app.event_filter(), EventFilter::Major);
    app.apply(Action::CycleEventFilter, &world);
    assert_eq!(app.event_filter(), EventFilter::All);
}

#[test]
fn selected_shows_what_the_selected_sprite_did_and_had_done_to_it() {
    let world = field();
    let mut app = logged(&world);
    app.apply(Action::CycleEventFilter, &world);
    assert!(log(&app, &world).is_empty(), "nothing selected");
    // Sprite #2.
    app.apply(Action::SelectNext, &world);
    app.apply(Action::SelectNext, &world);
    assert_eq!(
        log(&app, &world),
        [
            "Sprite #1 hit Sprite #2",
            "Couldn't zap Sprite #2: it's gone",
            "Sprite #2 learned: Sprite #1 is frightening",
        ]
    );
}

#[test]
fn major_shows_deaths_lessons_and_refusals() {
    let world = field();
    let mut app = logged(&world);
    app.apply(Action::CycleEventFilter, &world);
    app.apply(Action::CycleEventFilter, &world);
    assert_eq!(
        log(&app, &world),
        [
            "Sprite #3 died (old age, age 900)",
            "Couldn't zap Sprite #2: it's gone",
            "Sprite #2 learned: Sprite #1 is frightening",
        ]
    );
}

#[test]
fn the_log_s_border_shows_the_filters_with_the_one_in_use_in_brackets() {
    let world = field();
    let mut app = logged(&world);
    let border = |app: &App| row(&render(app, &world), 24);
    assert!(
        border(&app).ends_with(" [all] selected major ─┐"),
        "{}",
        border(&app)
    );
    app.apply(Action::CycleEventFilter, &world);
    assert!(
        border(&app).ends_with(" all [selected] major ─┐"),
        "{}",
        border(&app)
    );
}

#[test]
fn a_click_on_the_filters_goes_to_the_next() {
    let world = field();
    let mut app = logged(&world);
    let border = row(&render(&app, &world), 24);
    let at = border.chars().position(|c| c == '[').expect("the label") as u16;
    app.apply(Action::left_click(Position::new(at + 3, 24)), &world);
    assert_eq!(app.event_filter(), EventFilter::Selected);
    app.apply(Action::left_click(Position::new(10, 24)), &world);
    assert_eq!(
        app.event_filter(),
        EventFilter::Selected,
        "the border elsewhere"
    );
    app.apply(Action::left_click(Position::new(at + 3, 26)), &world);
    assert_eq!(app.event_filter(), EventFilter::Selected, "the log's lines");
}
