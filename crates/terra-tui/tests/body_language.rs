//! Body language (design §6.3): sprite colours and emotes, which show on
//! the map how a sprite is.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, Genome, Map, Pos, Scenario, World};
use terra_tui::app::{App, ColourMode};
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
