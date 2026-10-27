//! A palette with no colour, mono (design §6.2, #307): the finished screen
//! has no colour at all, its own greys included, only bold and reverse
//! video, so none can leak from anything drawn.

mod common;
use common::screen::{HUNGRY_GENOME, app_for, garden_with_sprites, one_sprite_among, render};
use common::{at, mono, render_on};

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Size;
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_tui::app::App;
use terra_tui::args::Args;
use terra_tui::input::Action;
use terra_tui::start;
use terra_tui::theme::Theme;
use terra_tui::title::{self, Title};
use terra_tui::ui;

/// Where `screen` has a colour, if anywhere: the first cell with one.
fn coloured(screen: &Buffer) -> Option<(u16, u16, Color, Color)> {
    let area = screen.area;
    (area.top()..area.bottom())
        .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
        .map(|(x, y)| {
            let cell = &screen[(x, y)];
            (x, y, cell.fg, cell.bg, cell.underline_color)
        })
        .find(|&(_, _, fg, bg, under)| [fg, bg, under] != [Color::Reset; 3])
        .map(|(x, y, fg, bg, _)| (x, y, fg, bg))
}

/// Whether some cell of `screen` has `modifier`.
fn has(screen: &Buffer, modifier: Modifier) -> bool {
    screen
        .content
        .iter()
        .any(|cell| cell.modifier.contains(modifier))
}

#[test]
fn in_mono_no_screen_of_a_world_has_any_colour_only_bold_and_reverse_video() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, mono(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let mut screens = vec![("the world", render(&app, &world, 100, 30))];
    for (what, action) in [
        ("the Brain tab", Action::NextTab),
        ("the Chem tab", Action::NextTab),
        ("the Genome tab", Action::NextTab),
        ("the World tab", Action::NextTab),
        ("help", Action::Help),
        ("the command palette", Action::Palette),
        ("the sprite list", Action::SpriteList),
        ("the population graph", Action::Population),
        ("the family tree", Action::FamilyTree),
        ("the debug overlay", Action::DebugOverlay),
    ] {
        app.apply(action, &world);
        screens.push((what, render(&app, &world, 100, 30)));
        if !matches!(action, Action::NextTab | Action::DebugOverlay) {
            app.apply(Action::Dismiss, &world);
        }
    }
    screens.push((
        "too small a screen",
        render_on(&app, &world, Size::new(10, 4)),
    ));
    for (what, screen) in &screens {
        assert_eq!(coloured(screen), None, "{what}");
    }
    let (_, world_screen) = &screens[0];
    assert!(has(world_screen, Modifier::REVERSED), "the open tab");
    assert!(has(world_screen, Modifier::BOLD), "headings and keys");
}

#[test]
fn in_mono_the_map_view_alone_has_no_colour() {
    let world = garden_with_sprites();
    let mut app = App::new(
        world.map(),
        mono(),
        world.seed(),
        ui::map_view_alone_areas(Size::new(30, 12)),
    );
    app.set_motion(false);
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    terminal
        .draw(|frame| ui::render_map_view_alone(frame, &app, &world))
        .unwrap();
    assert_eq!(coloured(terminal.backend().buffer()), None);
}

#[test]
fn in_mono_the_title_screen_has_no_colour_its_own_greys_included() {
    // Its scene's light and lettering fade in from grey, which the title
    // draws itself.
    let world = start::title_world(&Args::default(), 3, (100, 30)).expect("a world");
    let mut title = Title::new(world, mono(), 1, Size::new(100, 30));
    for step in [0, 500, 1500, 3000, 10_000] {
        title.animate(Duration::from_millis(step));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| title::render(frame, &title)).unwrap();
        assert_eq!(
            coloured(terminal.backend().buffer()),
            None,
            "at {step} ms more"
        );
    }
    let mut terminal = Terminal::new(TestBackend::new(10, 4)).unwrap();
    terminal.draw(|frame| title::render(frame, &title)).unwrap();
    assert_eq!(coloured(terminal.backend().buffer()), None, "too small");
}

#[test]
fn a_palette_with_colour_draws_its_colours() {
    // So what the tests above find is mono's doing.
    let world = garden_with_sprites();
    let app = app_for(&world, Theme::cp437(), 100, 30);
    assert!(coloured(&render(&app, &world, 100, 30)).is_some());
}

#[test]
fn in_mono_the_tile_the_selected_sprite_attends_to_is_bold() {
    // Design §6.2: the attention marker is a background, which mono can't
    // show, so bold marks the tile instead.
    let (berry, bush) = (at(0, 1), at(8, 4));
    let objects = [(bush, "berry_bush"), (berry, "berry")];
    let (world, mut app) = one_sprite_among(HUNGRY_GENOME, &objects, &[], 1);
    app.theme = mono();
    let (berry_cell, bush_cell) = (
        app.cell_of(berry).expect("in view"),
        app.cell_of(bush).expect("in view"),
    );
    let screen = render(&app, &world, 100, 30);
    assert_eq!(screen[berry_cell].symbol(), "•", "the berry, still drawn");
    assert!(
        screen[berry_cell].modifier.contains(Modifier::BOLD),
        "marked"
    );
    assert!(!screen[bush_cell].modifier.contains(Modifier::BOLD));
}
