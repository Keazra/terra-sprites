//! Every screen's text stays within CP437, so any CP437 font or tileset can
//! draw every panel (design §6.2).

use ratatui::buffer::Buffer;
use ratatui::layout::Size;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, World, WorldConfig};
use terra_tui::app::{App, CursorMode, Ticks};
use terra_tui::input::Action;
use terra_tui::theme::Theme;
use terra_tui::ui;

fn render(app: &App, world: &World, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// Fails naming the screen and the row if a cell holds anything CP437
/// can't show.
fn assert_cp437(screen: &Buffer, what: &str) {
    for y in 0..screen.area.height {
        let row: String = (0..screen.area.width)
            .map(|x| screen[(x, y)].symbol())
            .collect();
        if let Some(c) = row.chars().find(|&c| !terra_tui::cp437::contains(c)) {
            panic!("{c:?} on {what}, row {y}: {row:?}");
        }
    }
}

/// The game on the default world, `ticks` ticks on, with the app having
/// seen each tick's events, in `theme`.
fn played(ticks: u32, theme: Theme) -> (World, App) {
    let pack = DataPack::builtin().expect("built-in data pack is valid");
    let mut world = World::new(WorldConfig::builtin(&pack), pack, 7);
    let areas = ui::areas(Size::new(100, 30), world.map());
    let mut app = App::new(world.map(), theme, 7, areas);
    app.set_data_folder("/home/kel/.local/share/terra-sprites".into());
    for _ in 0..ticks {
        let mut tick = Ticks::default();
        tick.step(&mut world);
        app.take_in(tick, &world);
    }
    (world, app)
}

#[test]
fn every_screen_s_text_is_within_cp437() {
    for theme in [Theme::cp437(), Theme::ascii()] {
        let (world, mut app) = played(600, theme);
        let check = |app: &mut App, action: Option<Action>, what: &str| {
            if let Some(action) = action {
                app.apply(action, &world);
            }
            assert_cp437(&render(app, &world, 100, 30), what);
        };
        check(&mut app, None, "the main screen");
        check(&mut app, Some(Action::Track), "a refusal");
        check(&mut app, Some(Action::CycleColours), "a notice");
        check(&mut app, Some(Action::SelectNext), "the Body tab");
        for tab in ["Brain", "Chem", "Genome", "World"] {
            check(&mut app, Some(Action::NextTab), tab);
        }
        check(&mut app, Some(Action::NextTab), "the Body tab again");
        check(&mut app, Some(Action::ToggleDetail), "the detail view");
        for filter in ["selected", "major", "all"] {
            check(&mut app, Some(Action::CycleEventFilter), filter);
        }
        check(&mut app, Some(Action::Track), "Track");
        check(&mut app, Some(Action::Help), "the help screen");
        check(&mut app, Some(Action::Help), "the help screen closed");
        check(&mut app, Some(Action::SpriteList), "the sprite list");
        for sort in ["name", "age", "drive"] {
            check(&mut app, Some(Action::SelectNext), sort);
        }
        check(&mut app, Some(Action::Back), "the sprite list closed");
        check(&mut app, Some(Action::Rename), "naming");
        check(&mut app, Some(Action::Back), "naming given up");
        check(&mut app, Some(Action::Mode(CursorMode::Grab)), "Grab mode");
        check(
            &mut app,
            Some(Action::Mode(CursorMode::Grab)),
            "the Place menu",
        );
        check(&mut app, Some(Action::Back), "the Place menu closed");
        check(&mut app, Some(Action::Back), "back to Select");
        check(&mut app, Some(Action::Back), "the quit prompt");
        assert_cp437(&render(&app, &world, 80, 20), "a screen too small");
    }
}
