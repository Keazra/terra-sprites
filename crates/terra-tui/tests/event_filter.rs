//! The event log's filter (design §6.1): all events, the selected sprite's,
//! or major ones only, switched with `m` or a click on its label.

mod common;
use common::{UNHURT, WorldBuilder, acted, app_on, at, at_sprite, died, render, row};

use ratatui::layout::Position;
use terra_sim::{
    Command, DeathCause, EntityId, Event, EventKind, Learned, Outcome, Rejection, Verb, World,
};
use terra_tui::app::{App, EventFilter};
use terra_tui::input::Action;

/// Sprites #1 and #2 in a field.
fn field() -> World {
    WorldBuilder::grass(10, 5)
        .starters(&[at(2, 3), at(6, 1)])
        .seed(7)
        .build()
}

/// The event log's lines, without their ticks: the three rows above the
/// status line.
fn log(app: &App, world: &World) -> Vec<String> {
    let screen = render(app, world);
    (26..29)
        .map(|y| {
            (0..100)
                .map(|x| screen[(x, y)].symbol())
                .collect::<String>()
        })
        .map(|line| {
            line.trim()
                .split_once("  ")
                .map_or("", |(_, text)| text)
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

/// Sprite #`hitter` hit sprite #`hit` on `tick`, hurting nobody.
fn hit(tick: u64, hitter: u64, hit: u64) -> Event {
    let (hitter, target) = (EntityId(hitter), at_sprite(EntityId(hit)));
    acted(tick, hitter, Verb::Hit, target, Outcome::Applied, UNHURT)
}

/// An app on `world` whose log has, oldest first: #2 learning a lesson, a
/// refused pet of #2, #1 hitting #2, a pet of #1, #3 hitting #4, and #3's
/// death.
fn logged(world: &World) -> App {
    let mut app = app_on(world);
    let events = [
        Event {
            tick: 1,
            kind: EventKind::LearnedMilestone {
                id: EntityId(2),
                learned: Learned::Fear {
                    thing: terra_sim::Thing::Sprite(EntityId(1)),
                },
                good: false,
                watching: None,
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
        died(6, EntityId(3), DeathCause::OldAge, 900),
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
fn the_log_s_title_row_shows_the_filters_with_the_one_in_use_in_brackets() {
    let world = field();
    let mut app = logged(&world);
    let title = |app: &App| row(&render(app, &world), 25);
    assert!(
        title(&app).ends_with(" [all] selected major"),
        "{}",
        title(&app)
    );
    app.apply(Action::CycleEventFilter, &world);
    assert!(
        title(&app).ends_with(" all [selected] major"),
        "{}",
        title(&app)
    );
}

#[test]
fn a_click_on_the_filters_goes_to_the_next() {
    let world = field();
    let mut app = logged(&world);
    let title = row(&render(&app, &world), 25);
    let at = title.chars().position(|c| c == '[').expect("the label") as u16;
    app.apply(Action::left_click(Position::new(at + 3, 25)), &world);
    assert_eq!(app.event_filter(), EventFilter::Selected);
    app.apply(Action::left_click(Position::new(10, 25)), &world);
    assert_eq!(
        app.event_filter(),
        EventFilter::Selected,
        "the title row elsewhere"
    );
    app.apply(Action::left_click(Position::new(at + 3, 27)), &world);
    assert_eq!(app.event_filter(), EventFilter::Selected, "the log's lines");
}
