//! Body language (design §6.3): sprite colours and emotes, which show on
//! the map how a sprite is.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{
    ActionView, DataPack, EntityId, Event, EventKind, Genome, Hurt, Map, Outcome, Pos, Progress,
    Scenario, ScriptedAction, Target, Verb, World,
};
use terra_tui::app::{App, ColourMode, PLEASED_AT, Ticks};
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A genome whose sprite starts with the drives `levels`, and nothing else.
fn feeling(levels: &[(&str, f32)]) -> Genome {
    let genes: Vec<String> = levels
        .iter()
        .map(|(chem, value)| format!("InitialConcentration(chem: \"{chem}\", value: {value})"))
        .collect();
    let text = format!("(format: 1, genes: [{}])", genes.join(", "));
    Genome::from_ron(&text, &pack()).expect("a valid genome")
}

/// A field with a sprite on each of `sprites`' tiles, with its genome, before
/// its first tick.
fn field(sprites: &[(Pos, Genome)]) -> World {
    let pack = pack();
    let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites
        .iter()
        .map(|(pos, genome)| (*pos, Some(genome.clone())))
        .collect();
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, pack, 7).expect("valid scenario")
}

fn app_for(world: &World, theme: Theme) -> App {
    let areas = ui::areas(Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), theme, 7, areas);
    // The Cursor out of the way, on (9, 4).
    app.apply(Action::Point(Position::new(11, 7)), world);
    app
}

fn render(app: &App, world: &World) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// The screen cell tile `tile` is drawn on: the map view's tiles start at
/// (1, 2).
fn cell(tile: Pos) -> Position {
    Position::new(1 + tile.x, 2 + tile.y)
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

#[test]
fn a_sprite_is_drawn_in_the_colour_of_its_strongest_drive_above_half() {
    // Design §6.3: hunger yellow, thirst cyan, pain red, tiredness blue,
    // boredom dark grey, loneliness magenta, crowdedness light red; white
    // with none above .5.
    let cases = [
        ("hunger", Color::Yellow),
        ("thirst", Color::Cyan),
        ("pain", Color::Red),
        ("tiredness", Color::Blue),
        ("boredom", Color::DarkGray),
        ("loneliness", Color::Magenta),
        ("crowdedness", Color::LightRed),
    ];
    let sprites: Vec<(Pos, Genome)> = cases
        .iter()
        .enumerate()
        .map(|(i, (drive, _))| {
            // Each sprite's drive is strongest, above a weaker one also over .5.
            let genome = feeling(&[(drive, 0.9), ("hunger", 0.6), ("thirst", 0.6)]);
            (at(i as u16, 0), genome)
        })
        .chain([
            (at(0, 2), feeling(&[("hunger", 0.5), ("thirst", 0.3)])),
            (at(1, 2), feeling(&[])),
        ])
        .collect();
    let world = field(&sprites);
    for theme in [Theme::cp437(), Theme::ascii()] {
        let screen = render(&app_for(&world, theme), &world);
        for (i, (drive, colour)) in cases.iter().enumerate() {
            assert_eq!(screen[cell(at(i as u16, 0))].fg, *colour, "{drive}");
        }
        assert_eq!(screen[cell(at(0, 2))].fg, Color::White, "none above .5");
        assert_eq!(screen[cell(at(1, 2))].fg, Color::White, "no drives at all");
    }
}

#[test]
fn the_selected_sprite_takes_its_colour_too() {
    let world = field(&[(at(2, 3), feeling(&[("thirst", 0.8)]))]);
    let mut app = app_for(&world, Theme::cp437());
    app.apply(Action::SelectNext, &world);
    let screen = render(&app, &world);
    assert_eq!(screen[cell(at(2, 3))].symbol(), "☻");
    assert_eq!(screen[cell(at(2, 3))].fg, Color::Cyan);
}

#[test]
fn b_switches_between_the_strongest_drive_and_plain_and_says_so() {
    let world = field(&[(at(2, 3), feeling(&[("hunger", 0.8)]))]);
    let mut app = app_for(&world, Theme::cp437());
    assert_eq!(
        app.colour_mode(),
        ColourMode::Drive,
        "the game starts on drives"
    );
    app.apply(Action::CycleColours, &world);
    assert_eq!(app.colour_mode(), ColourMode::Plain);
    let screen = render(&app, &world);
    assert_eq!(screen[cell(at(2, 3))].fg, Color::White);
    let status: String = (0..100).map(|x| screen[(x, 29)].symbol()).collect();
    assert!(status.trim_end().ends_with("Colours: plain"), "{status}");

    app.apply(Action::CycleColours, &world);
    assert_eq!(app.colour_mode(), ColourMode::Drive);
    let screen = render(&app, &world);
    assert_eq!(screen[cell(at(2, 3))].fg, Color::Yellow);
    let status: String = (0..100).map(|x| screen[(x, 29)].symbol()).collect();
    assert!(
        status.trim_end().ends_with("Colours: strongest drive"),
        "{status}"
    );
}

/// One plain starter sprite on (2, 3), its ID, and an app on it.
fn one_sprite() -> (World, EntityId, App) {
    let world = field(&[(at(2, 3), feeling(&[]))]);
    let id = world.sprites().next().expect("a sprite").id();
    let app = app_for(&world, Theme::cp437());
    (world, id, app)
}

/// What sprite `id`'s `verb`, aimed at `target`, did when it ended on `tick`.
fn ended(tick: u64, id: EntityId, verb: Verb, outcome: Outcome, target: Option<Target>) -> Event {
    let action = ActionView {
        verb,
        destination: None,
        target,
        target_type: target.map(|_| 101),
        attempted: outcome == Outcome::Applied,
        target_gone: false,
        hurt: Hurt::default(),
        progress: Progress::Ended(outcome),
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

fn started(tick: u64, id: EntityId, verb: Verb) -> Event {
    Event {
        tick,
        kind: EventKind::ActionStarted { id, verb },
    }
}

/// The glyph drawn on tile (2, 3).
fn glyph(app: &App, world: &World) -> String {
    render(app, world)[cell(at(2, 3))].symbol().to_string()
}

const QUARTER: Duration = Duration::from_millis(250);

#[test]
fn a_sprite_that_gives_up_shows_a_question_mark_for_a_second() {
    // Design §6.3: failed, blocked or timed out; not changing its mind, or
    // being taken hold of.
    for outcome in [Outcome::Failed, Outcome::Blocked, Outcome::TimedOut] {
        let (world, id, mut app) = one_sprite();
        app.record(&[ended(1, id, Verb::Wander, outcome, None)], &world);
        assert_eq!(glyph(&app, &world), "?", "{outcome:?}");
        app.animate(QUARTER);
        assert_eq!(glyph(&app, &world), "☺", "taking turns with the sprite");
        app.animate(QUARTER);
        assert_eq!(glyph(&app, &world), "?");
        app.animate(QUARTER * 2);
        assert_eq!(glyph(&app, &world), "☺", "and then it's over");
    }
    for outcome in [Outcome::Applied, Outcome::Interrupted, Outcome::PulledAway] {
        let (world, id, mut app) = one_sprite();
        app.record(&[ended(1, id, Verb::Wander, outcome, None)], &world);
        assert_eq!(glyph(&app, &world), "☺", "{outcome:?}");
    }
}

#[test]
fn a_resting_sprite_shows_a_z_while_the_rest_lasts() {
    let (world, id, mut app) = one_sprite();
    app.record(&[started(1, id, Verb::Rest)], &world);
    // However long the rest lasts in real time.
    for _ in 0..8 {
        assert_eq!(glyph(&app, &world), "z");
        app.animate(QUARTER);
        assert_eq!(glyph(&app, &world), "☺");
        app.animate(QUARTER);
    }
    app.record(&[ended(10, id, Verb::Rest, Outcome::Applied, None)], &world);
    assert_eq!(glyph(&app, &world), "☺", "rested");
    app.animate(QUARTER);
    assert_eq!(glyph(&app, &world), "☺");
}

#[test]
fn a_rest_over_in_a_blink_still_shows_its_z_for_a_second() {
    // At 16x or Max a whole rest can start and end within one frame (design
    // §6.3): the emote is driven by the events, and lasts in real time.
    let (world, id, mut app) = one_sprite();
    app.record(
        &[
            started(1, id, Verb::Rest),
            ended(10, id, Verb::Rest, Outcome::Applied, None),
        ],
        &world,
    );
    assert_eq!(glyph(&app, &world), "z");
    app.animate(QUARTER);
    assert_eq!(glyph(&app, &world), "☺");
    app.animate(QUARTER);
    assert_eq!(glyph(&app, &world), "z");
    app.animate(QUARTER * 2);
    assert_eq!(glyph(&app, &world), "☺", "a second on, it's over");
    app.animate(QUARTER);
    assert_eq!(glyph(&app, &world), "☺");
}

#[test]
fn resting_gives_way_to_any_other_emote_and_comes_back_after() {
    let (world, id, mut app) = one_sprite();
    app.record(&[started(1, id, Verb::Rest)], &world);
    app.animate(QUARTER * 4);
    let mut hit = ended(
        5,
        EntityId(99),
        Verb::Hit,
        Outcome::Applied,
        Some(Target::Sprite(id)),
    );
    if let EventKind::ActionEnded { action, .. } = &mut hit.kind {
        action.hurt.target = true;
    }
    app.record(&[hit], &world);
    assert_eq!(glyph(&app, &world), "!", "hurt, while resting");
    app.animate(QUARTER * 2);
    assert_eq!(glyph(&app, &world), "!");
    app.animate(QUARTER * 2);
    assert_eq!(glyph(&app, &world), "z", "resting again");
}

#[test]
fn the_newest_emote_wins() {
    let (world, id, mut app) = one_sprite();
    app.record(&[ended(1, id, Verb::Wander, Outcome::Failed, None)], &world);
    app.animate(QUARTER * 2);
    let pet = Event {
        tick: 2,
        kind: EventKind::Rewarded {
            id,
            amplified: false,
        },
    };
    app.record(&[pet], &world);
    assert_eq!(glyph(&app, &world), "♥");
}

/// A sprite that feels a reward of `gain` every tick, from tick 1.
fn rewarded_every_tick(gain: f32) -> Genome {
    let text = format!(
        "(format: 1, genes: [Emitter(locus: Locus(\"always\"), mode: Level, gain: {gain}, chem: \"reward\")])"
    );
    Genome::from_ron(&text, &pack()).expect("a valid genome")
}

#[test]
fn a_sprite_that_feels_a_strong_reward_shows_a_heart() {
    // Design §6.3: Pleased on `last_r` above the UI's spike threshold, which
    // the frame's ticks pick up whichever tick it came on.
    let strong = rewarded_every_tick(PLEASED_AT + 0.2);
    let weak = rewarded_every_tick(PLEASED_AT - 0.1);
    let mut world = field(&[(at(2, 3), strong), (at(6, 3), weak)]);
    let mut app = app_for(&world, Theme::cp437());
    let mut ticks = Ticks::default();
    for _ in 0..3 {
        ticks.step(&mut world);
    }
    let felt: Vec<f32> = world.sprites().map(|sprite| sprite.felt()).collect();
    assert!(felt[0] > PLEASED_AT && felt[1] < PLEASED_AT, "{felt:?}");
    app.take_in(ticks, &world);
    let screen = render(&app, &world);
    let tiles: Vec<Pos> = world.sprites().map(|sprite| sprite.pos()).collect();
    assert_eq!(screen[cell(tiles[0])].symbol(), "♥");
    // It may show another emote: it may have given up wandering.
    assert_ne!(screen[cell(tiles[1])].symbol(), "♥", "a small comfort");
}

#[test]
fn emotes_last_a_second_of_real_time_at_any_speed() {
    // Design §6.3: many ticks a frame, as at Max, still show the emote for
    // a second, and frames with no ticks don't end it sooner.
    let (mut world, id, mut app) = one_sprite();
    let frame = |app: &mut App, world: &mut World, ticks: u32, events: Vec<Event>| {
        for _ in 0..ticks {
            world.step();
        }
        app.animate(Duration::from_millis(33));
        app.record(&events, world);
    };
    frame(
        &mut app,
        &mut world,
        50,
        vec![ended(1, id, Verb::Wander, Outcome::Blocked, None)],
    );
    let mut shown = Duration::ZERO;
    for _ in 0..40 {
        if render(&app, &world)[cell(world.sprite(id).unwrap().pos())].symbol() == "?" {
            shown += Duration::from_millis(33);
        }
        frame(&mut app, &mut world, 50, Vec::new());
    }
    // Half of a second's frames show it, the other half the sprite.
    assert!(
        (Duration::from_millis(400)..Duration::from_millis(600)).contains(&shown),
        "{shown:?}"
    );
}

#[test]
fn the_resting_emote_shows_for_a_second_at_16x_and_at_max_through_the_frame_loop() {
    // Design §6.3: emotes are driven by the sim's events, a frame's ticks
    // taken in through `Ticks`, so a rest over in a blink still shows.
    for speed in [Speed::X16, Speed::Max] {
        let pack = pack();
        let map = Map::from_ascii(&[".........."; 5], &pack).expect("valid drawing");
        let sprites = [(at(2, 3), Some(feeling(&[])))];
        let scripted = [(at(2, 3), ScriptedAction::Rest)];
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted: &scripted,
        };
        let mut world = World::from_scenario(scenario, pack, 7).expect("valid scenario");
        let id = world.sprites().next().expect("a sprite").id();
        let mut app = app_for(&world, Theme::cp437());
        while app.clock.speed() != speed {
            app.clock.faster();
        }
        // Frames of 33 ms, as the game draws them, at most 50 ticks each.
        let mut seen = Vec::new();
        for frame in 0..60u32 {
            let mut ticks = Ticks::default();
            let mut ran = 0;
            app.clock.advance(
                Duration::from_millis(33),
                || ticks.step(&mut world),
                || {
                    ran += 1;
                    ran >= 50
                },
            );
            app.take_in(ticks, &world);
            app.animate(Duration::from_millis(33));
            let pos = world.sprite(id).expect("still alive").pos();
            if render(&app, &world)[cell(pos)].symbol() == "z" {
                seen.push(frame);
            }
        }
        let first = *seen.first().unwrap_or_else(|| panic!("no z at {speed:?}"));
        let last = *seen.last().unwrap();
        assert!(
            first <= 1,
            "{speed:?}: it shows from the start, frame {first}"
        );
        assert!(
            (last - first) * 33 >= 900,
            "{speed:?}: shown from frame {first} to {last}"
        );
    }
}

#[test]
fn ascii_draws_failed_and_resting_with_the_same_letters() {
    // Design §6.3: `?` and `z` in both themes.
    let world = field(&[(at(2, 3), feeling(&[])), (at(6, 3), feeling(&[]))]);
    let ids: Vec<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
    let mut app = app_for(&world, Theme::ascii());
    app.record(
        &[
            ended(1, ids[0], Verb::Wander, Outcome::Failed, None),
            started(1, ids[1], Verb::Rest),
        ],
        &world,
    );
    let screen = render(&app, &world);
    assert_eq!(screen[cell(at(2, 3))].symbol(), "?");
    assert_eq!(screen[cell(at(6, 3))].symbol(), "z");
}
