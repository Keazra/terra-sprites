//! The Game Kid palette (design §6.2): the screen in greens on the
//! terminal's own black, after the original Game Boy's four. Greens can't
//! tell the drives or the lineage apart, so, as in mono, a sprite keeps its
//! own colour, and `b` and the help say where they're read instead.

mod common;
use common::screen::{app_for, garden_with_sprites, render};
use common::{app_in, frame_on, game_kid, lines, on_the_ramp, pairs, render_on, select};

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::EntityId;
use terra_tui::app::{App, ColourMode};
use terra_tui::args::Args;
use terra_tui::campbell;
use terra_tui::input::Action;
use terra_tui::start;
use terra_tui::title::{self, Title};
use terra_tui::ui;

/// Whether `colour`, drawn as a foreground, a background or an underline,
/// is a green on the ramp, or as dark as the terminal's black, as a fade on
/// its way to black passes through: no channel above 25, a tenth of the way
/// from Campbell's black to white. The terminal's own background is its
/// black; its own text colour is Campbell's grey, which isn't green.
fn green_or_black(colour: Color, foreground: bool) -> bool {
    let own = foreground.then_some(campbell::FOREGROUND);
    let Some(rgb) = campbell::rgb(colour).or(own) else {
        return true;
    };
    rgb.iter().all(|&channel| channel <= 25) || on_the_ramp(rgb)
}

/// The first cell of `screen` drawn in a colour that's neither green nor
/// black, with the colour: its foreground, background or underline.
fn not_green(screen: &Buffer) -> Option<(Position, Color)> {
    screen.area.positions().find_map(|at| {
        let cell = &screen[at];
        [
            (cell.fg, true),
            (cell.bg, false),
            (cell.underline_color, false),
        ]
        .into_iter()
        .find(|&(colour, foreground)| !green_or_black(colour, foreground))
        .map(|(colour, _)| (at, colour))
    })
}

#[test]
fn in_game_kid_every_screen_of_a_world_is_drawn_in_greens_on_black_as_it_moves_too() {
    // Design §6.2, §6.9: its own colours, the text the game leaves in the
    // terminal's own and every grey it draws itself, in a tab's sweep and
    // a pop-up's fade as well.
    let world = garden_with_sprites();
    let mut app = app_for(&world, game_kid(), 100, 30);
    app.set_motion(true);
    let size = Size::new(100, 30);
    let step = Duration::from_millis(60);
    app.apply(Action::SelectNext, &world);
    let mut screens = vec![("the world", frame_on(&mut app, &world, step, size))];
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
        ("the settings screen", Action::OpenSettings),
        ("the debug overlay", Action::DebugOverlay),
    ] {
        app.apply(action, &world);
        screens.push((what, frame_on(&mut app, &world, step, size)));
        if !matches!(action, Action::NextTab | Action::DebugOverlay) {
            app.apply(Action::Dismiss, &world);
            screens.push((what, frame_on(&mut app, &world, step, size)));
        }
    }
    screens.push((
        "too small a screen",
        render_on(&app, &world, Size::new(10, 4)),
    ));
    for (what, screen) in &screens {
        assert_eq!(not_green(screen), None, "{what}");
    }
}

#[test]
fn in_game_kid_the_map_view_alone_is_drawn_in_greens_on_black() {
    let world = garden_with_sprites();
    let mut app = App::new(
        world.map(),
        game_kid(),
        world.seed(),
        ui::map_view_alone_areas(Size::new(30, 12)),
    );
    app.set_motion(false);
    let mut terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    terminal
        .draw(|frame| ui::render_map_view_alone(frame, &app, &world))
        .unwrap();
    assert_eq!(not_green(terminal.backend().buffer()), None);
}

#[test]
fn in_game_kid_the_title_screen_is_drawn_in_greens_on_black_its_own_greys_included() {
    // Its scene's light and lettering fade in from grey, which the title
    // draws itself.
    let world = start::title_world(&Args::default(), 3, (100, 30)).expect("a world");
    let mut title = Title::new(world, game_kid(), 1, Size::new(100, 30));
    for step in [0, 500, 1500, 3000, 10_000] {
        title.animate(Duration::from_millis(step));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| title::render(frame, &title)).unwrap();
        assert_eq!(
            not_green(terminal.backend().buffer()),
            None,
            "at {step} ms more"
        );
    }
}

#[test]
fn another_palette_draws_greys() {
    // So what the tests above find is Game Kid's doing.
    let world = garden_with_sprites();
    let app = app_for(&world, common::mono(), 100, 30);
    assert!(not_green(&render(&app, &world, 100, 30)).is_some());
    let app = app_for(&world, terra_tui::theme::Theme::cp437(), 100, 30);
    assert!(not_green(&render(&app, &world, 100, 30)).is_some());
}

#[test]
fn in_game_kid_b_says_the_palette_can_t_show_a_sprite_s_colours_and_where_they_re_read() {
    // Design §6.2, §6.3: greens can't tell nine drives apart, or the
    // lineage, so `b` says where they're read instead, as in mono.
    let world = garden_with_sprites();
    let mut app = app_for(&world, game_kid(), 100, 30);
    let mut said = Vec::new();
    for _ in 0..3 {
        app.apply(Action::CycleColours, &world);
        said.push(app.notice().map(str::to_string));
    }
    assert_eq!(
        said,
        [
            Some("Colours: plain".to_string()),
            Some("Colours: lineage, which this palette can't show: see the family tree (k)".into()),
            Some(
                "Colours: strongest drive, which this palette can't show: see the sprite list (l)"
                    .into()
            ),
        ]
    );
}

#[test]
fn in_game_kid_help_says_where_a_sprite_s_colours_are_read_and_leaves_the_lineage_out() {
    // Design §6.2: its legends would be a row of the same green sprite.
    let world = garden_with_sprites();
    let mut app = app_for(&world, game_kid(), 100, 30);
    app.apply(Action::Help, &world);
    let rows = lines(&render(&app, &world, 100, 30));
    let colours = rows
        .iter()
        .find(|row| row.contains(" COLOURS"))
        .expect("the COLOURS heading");
    assert!(
        colours.contains(
            "COLOURS: none in this palette: drives in the sprite list (l), lineage in the \
             family tree (k)"
        ),
        "{colours}"
    );
    assert!(
        !rows.iter().any(|row| row.contains(" hunger ")),
        "{rows:#?}"
    );
    assert!(!rows.iter().any(|row| row.contains("LINEAGE")), "{rows:#?}");
    assert!(rows.iter().any(|row| row.contains(" EMOTES ")), "{rows:#?}");
}

#[test]
fn in_game_kid_a_sprite_keeps_its_own_colour_in_the_lineage_colours() {
    // Design §6.3: its lineage greens would tell no one apart, so every
    // sprite is drawn in its own; #1 and #2 are #14's parents.
    let world = pairs(230);
    let mut app = app_in(&world, game_kid());
    select(&mut app, &world, EntityId(14));
    // The Decision marker off, so it draws over no one.
    app.animate(Duration::from_millis(500));
    let colour = |app: &App, id: u64| {
        let pos = world.sprite(EntityId(id)).expect("alive").pos();
        let cell = app.cell_of(pos).expect("in view");
        let screen = render(app, &world, 100, 30);
        let cell = &screen[(cell.x, cell.y)];
        assert_eq!(cell.symbol(), "☺", "#{id} drawn");
        cell.fg
    };
    let own = app.theme.glyph(terra_tui::theme::SemanticTile::Sprite).fg;
    while app.colour_mode() != ColourMode::Lineage {
        app.apply(Action::CycleColours, &world);
    }
    assert_ne!(app.theme.lineage().elders, own);
    assert_eq!((colour(&app, 1), colour(&app, 2)), (own, own));
    assert_eq!(own, Color::Rgb(0xdc, 0xf0, 0x90));
}
