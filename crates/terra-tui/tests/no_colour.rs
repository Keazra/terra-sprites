//! A palette with no colour, mono (design §6.2, #307): the finished screen
//! has no colour at all, its own greys included, only bold and reverse
//! video, so none can leak from anything drawn.

mod common;
use common::screen::{HUNGRY_GENOME, app_for, garden_with_sprites, one_sprite_among, render};
use common::{WorldBuilder, at, mono, render_on, run};

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
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

/// Each row of `screen`'s characters, without their styles.
fn characters(screen: &Buffer) -> Vec<String> {
    common::lines(screen)
}

#[test]
fn a_palette_without_colour_draws_lines_between_the_panels_even_with_shades() {
    // Design §6.1, §6.2: its shades couldn't be seen, so it draws lines,
    // as mono does.
    let cp437 = include_str!("../../../themes/cp437.ron");
    let colourless = cp437.replacen("(\n    tiles: {", "(\n    colour: false,\n    tiles: {", 1);
    assert_ne!(colourless, cp437);
    let world = garden_with_sprites();
    let theme = Theme::from_ron(&colourless, world.data()).expect("a theme");
    assert!(!theme.has_colour());
    let drawn =
        |theme: Theme| characters(&render(&app_for(&world, theme, 100, 30), &world, 100, 30));
    let lines = drawn(mono());
    assert_ne!(drawn(Theme::cp437()), lines, "shades in place of lines");
    assert_eq!(drawn(theme), lines);
}

#[test]
fn in_mono_the_tile_the_selected_sprite_attends_to_is_underlined() {
    // Design §6.2: the attention marker is a background, which mono can't
    // show, so the tile is underlined instead, which no glyph is.
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
    let berry_style = screen[berry_cell].modifier;
    assert!(berry_style.contains(Modifier::UNDERLINED), "marked");
    assert!(!berry_style.contains(Modifier::BOLD), "{berry_style:?}");
    assert!(!screen[bush_cell].modifier.contains(Modifier::UNDERLINED));
}

/// A hungry sprite on (2, 3) that attends to bushes, the nearest berry
/// bush on (4, 3) and another on (8, 1), with `fruit` each, after a tick,
/// and an app on it in mono with the sprite selected; and the two bushes'
/// screen cells.
fn attending_a_bush(fruit: u16) -> (Buffer, Position, Position) {
    let genome = HUNGRY_GENOME.replace("category: \"fruit\"", "category: \"bush\"");
    let (near, far) = (at(4, 3), at(8, 1));
    let mut world = WorldBuilder::grass(10, 5)
        .objects(&[(near, "berry_bush"), (far, "berry_bush")])
        .genes(&[(at(2, 3), genome.as_str())])
        .seed(7)
        .build();
    for bush in [near, far] {
        world
            .start_object(bush, "mature", &[("fruit", fruit)])
            .expect("the bush");
    }
    run(&mut world, 1);
    let mut app = app_for(&world, mono(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let cells = (
        app.cell_of(near).expect("in view"),
        app.cell_of(far).expect("in view"),
    );
    (render(&app, &world, 100, 30), cells.0, cells.1)
}

#[test]
fn in_mono_the_attention_marker_never_reads_as_a_bush_s_fruit() {
    // Design §6.2: bold is what tells a fruiting bush from a bare one in
    // mono, so the marker leaves it as it is.
    let (screen, near, far) = attending_a_bush(3);
    let (near, far) = (screen[near].modifier, screen[far].modifier);
    assert!(
        near.contains(Modifier::BOLD | Modifier::UNDERLINED),
        "{near:?}"
    );
    assert!(far.contains(Modifier::BOLD), "{far:?}");
    assert!(!far.contains(Modifier::UNDERLINED), "{far:?}");
    // A bare bush attended to isn't drawn as if it bore fruit.
    let (screen, near, _) = attending_a_bush(0);
    let near = screen[near].modifier;
    assert!(near.contains(Modifier::UNDERLINED), "{near:?}");
    assert!(!near.contains(Modifier::BOLD), "{near:?}");
}

#[test]
fn in_mono_b_says_the_palette_can_t_show_a_sprite_s_colours_and_where_they_re_read() {
    // Design §6.2, §6.3: mono can't draw a sprite in a drive's colour or the
    // lineage's, so `b` says where they're read instead.
    let world = garden_with_sprites();
    let mut app = app_for(&world, mono(), 100, 30);
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
fn in_mono_help_says_where_a_sprite_s_colours_are_read_and_leaves_the_lineage_out() {
    // Design §6.2: the help's COLOURS and LINEAGE legends are drawn in
    // colours mono can't show, so they'd be a row of the same sprite.
    let world = garden_with_sprites();
    let mut app = app_for(&world, mono(), 100, 30);
    app.apply(Action::Help, &world);
    let rows = characters(&render(&app, &world, 100, 30));
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
