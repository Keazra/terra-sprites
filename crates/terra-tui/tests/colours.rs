//! Panels without boxes and colours with jobs (design §6.1, §6.2, #302): the
//! inspector and the event log on the theme's shades, or lines between them
//! in a theme without, and the screen's text coloured by what it is.

mod common;
use common::screen::{
    HUNGRY_GENOME, app_for, garden, garden_with_sprites, one_sprite, one_sprite_among, pointer_on,
    render,
};
use common::{at, died, open, pack, row, run};

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Position;
use ratatui::style::{Color, Modifier};
use terra_sim::{DeathCause, EntityId};
use terra_tui::app::Tab;
use terra_tui::input::Action;
use terra_tui::theme::{TextLook, Theme};

/// Where `text` first starts on `screen` from row `from` down.
fn find_from(screen: &Buffer, text: &str, from: u16) -> Position {
    (from..screen.area.height)
        .find_map(|y| {
            let cells: Vec<&str> = (0..screen.area.width)
                .map(|x| screen[(x, y)].symbol())
                .collect();
            let wanted: Vec<String> = text.chars().map(String::from).collect();
            (0..cells.len().saturating_sub(wanted.len() - 1))
                .find(|&x| cells[x..x + wanted.len()] == wanted[..])
                .map(|x| Position::new(x as u16, y))
        })
        .unwrap_or_else(|| panic!("no {text:?} on the screen"))
}

/// The cells of `text` where it first starts on `screen`, from row `from`.
fn cells<'a>(screen: &'a Buffer, text: &str, from: u16) -> Vec<&'a Cell> {
    let at = find_from(screen, text, from);
    (0..text.chars().count() as u16)
        .map(|i| &screen[(at.x + i, at.y)])
        .collect()
}

/// Whether `cell` is drawn in `style`'s colour and with its modifiers.
fn styled(cell: &Cell, style: ratatui::style::Style) -> bool {
    style.fg.is_none_or(|fg| cell.fg == fg) && cell.modifier.contains(style.add_modifier)
}

/// The default theme's text styles.
fn look() -> TextLook {
    Theme::cp437().text()
}

#[test]
fn the_default_theme_shades_the_inspector_and_the_log_and_not_the_map() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    let panels = Theme::cp437().panels().expect("shaded");
    // Their title rows a shade lighter than the rest.
    assert_eq!(
        screen[(99, 1)].bg,
        panels.title,
        "the inspector's title row"
    );
    assert_eq!(screen[(54, 1)].bg, panels.title, "its left column");
    assert_eq!(screen[(70, 12)].bg, panels.shade, "the open tab");
    assert_eq!(screen[(54, 24)].bg, panels.shade, "its last row");
    assert_eq!(screen[(10, 25)].bg, panels.title, "the log's title row");
    assert_eq!(screen[(10, 27)].bg, panels.shade, "the log's lines");
    // The map view has no frame and no shade: its tiles run to its edges.
    for at in [(0, 1), (53, 1), (0, 24), (53, 24), (20, 12)] {
        assert!(
            ![panels.shade, panels.title].contains(&screen[at].bg),
            "{at:?}"
        );
    }
    // No lines between them either, nor round the map view.
    let lines = ['┌', '┐', '└', '┘', '│', '─', '┴'];
    for y in 1..29 {
        let edge: String = (0..55).map(|x| screen[(x, y)].symbol()).collect();
        assert!(!edge.contains(lines), "{edge}");
    }
}

#[test]
fn the_ascii_theme_draws_a_line_beside_the_inspector_and_over_the_log() {
    let world = garden(pack());
    let app = app_for(&world, Theme::ascii(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    for y in 1..25 {
        assert_eq!(screen[(54, y)].symbol(), "│", "row {y}");
    }
    let rule = row(&screen, 25);
    assert!(rule.starts_with("─ Events ─"), "{rule}");
    assert_eq!(screen[(54, 25)].symbol(), "┴", "where the two lines meet");
    assert!(rule.ends_with(" [all] selected major ──"), "{rule}");
    assert_eq!(screen[(70, 12)].bg, Color::Reset, "no shade");
}

#[test]
fn the_open_tab_is_in_reverse_video_and_the_others_are_not() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SelectNext, &world);
    let screen = render(&app, &world, 100, 30);
    let open: Vec<&Cell> = cells(&screen, " Body ", 1);
    assert!(open.iter().all(|cell| styled(cell, look().selected)));
    let shut = cells(&screen, "Brain", 1);
    assert!(
        shut.iter()
            .all(|cell| !cell.modifier.contains(Modifier::REVERSED))
    );
}

#[test]
fn the_brain_tab_s_headings_gains_and_losses_have_their_colours() {
    let objects = [(at(8, 3), "berry_bush"), (at(6, 1), "berry")];
    let (world, mut app) = one_sprite_among(HUNGRY_GENOME, &objects, &[], 1);
    open(&mut app, &world, Tab::Brain);
    let screen = render(&app, &world, 100, 30);
    let look = look();
    for heading in ["ATTENTION", "DECISION: EAT"] {
        let heading = cells(&screen, heading, 2);
        assert!(heading.iter().all(|cell| styled(cell, look.heading)));
    }
    let gain = cells(&screen, "+.80", 2);
    assert!(gain.iter().all(|cell| styled(cell, look.good)), "a gain");
    let loss = cells(&screen, "-.10", 2);
    assert!(loss.iter().all(|cell| styled(cell, look.bad)), "a loss");
    // What it's for stays plain beside them.
    let name = cells(&screen, "hunger", 2);
    assert!(name.iter().all(|cell| cell.fg == Color::Reset));
}

#[test]
fn the_body_tab_draws_each_drive_s_bar_in_its_colour_on_a_muted_track() {
    let (world, app) = one_sprite(HUNGRY_GENOME, 0);
    let screen = render(&app, &world, 100, 30);
    let at = find_from(&screen, "hunger ", 2);
    let bar: Vec<&Cell> = (at.x + 12..at.x + 22).map(|x| &screen[(x, at.y)]).collect();
    let hunger = Theme::cp437().drive_colour("hunger").expect("a colour");
    let (full, empty): (Vec<&&Cell>, Vec<&&Cell>) =
        bar.iter().partition(|cell| cell.symbol() == "█");
    assert_eq!((full.len(), empty.len()), (8, 2), "hunger .8");
    assert!(full.iter().all(|cell| cell.fg == hunger));
    assert!(empty.iter().all(|cell| styled(cell, look().muted)));
    // The dots between a line's parts are minor too.
    let dot = find_from(&screen, " · ", 2);
    assert!(styled(&screen[(dot.x + 1, dot.y)], look().muted));
}

#[test]
fn the_chem_tab_colours_what_was_felt_and_leaves_each_change_plain() {
    let (world, mut app) = one_sprite(HUNGRY_GENOME, 1);
    open(&mut app, &world, Tab::Chem);
    let screen = render(&app, &world, 100, 40);
    let look = look();
    let mut changes = 0;
    for y in 2..35 {
        let text: String = (55..100).map(|x| screen[(x, y)].symbol()).collect();
        let Some(at) = text
            .find(['+', '-'])
            .filter(|&at| text[at + 1..].starts_with('.'))
        else {
            continue;
        };
        let number = text[at..].trim_end();
        let cell = &screen[(55 + text[..at].chars().count() as u16, y)];
        if text[..at].ends_with("felt ") {
            // A reward or a punishment, or plain when nothing was.
            assert!(
                styled(cell, look.signed(number, Default::default())),
                "{text}"
            );
        } else {
            // A rise is good for food and bad for hunger, so it's plain.
            assert_eq!(cell.fg, Color::Reset, "{text}");
            changes += 1;
        }
    }
    assert!(changes > 0, "energy and water fall every tick");
    let heading = cells(&screen, "HORMONES", 2);
    assert!(heading.iter().all(|cell| styled(cell, look.heading)));
}

#[test]
fn the_world_tab_s_graph_heading_is_a_heading() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    let heading = cells(&screen, "population, ticks", 2);
    assert!(heading.iter().all(|cell| styled(cell, look().heading)));
}

#[test]
fn the_event_log_s_ticks_are_muted_beside_what_happened() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.record(
        &[died(4_012, EntityId(31), DeathCause::Dehydration, 4_012)],
        &world,
    );
    let screen = render(&app, &world, 100, 30);
    let tick = cells(&screen, "4,012  Sprite #31 d", 26);
    assert!(tick[..5].iter().all(|cell| styled(cell, look().muted)));
    // The name, which a tooltip explains, is in the spot colour (#304).
    let name = "Sprite #31".len();
    assert!(
        tick[7..7 + name]
            .iter()
            .all(|cell| styled(cell, look().spot))
    );
    assert!(tick[7 + name..].iter().all(|cell| cell.fg == Color::Reset));
}

#[test]
fn a_hint_s_key_is_bold_and_what_it_does_is_muted() {
    let world = garden(pack());
    let app = app_for(&world, Theme::cp437(), 100, 30);
    let screen = render(&app, &world, 100, 30);
    let hint = cells(&screen, "Z select", 29);
    assert!(styled(hint[0], look().key), "the key");
    assert!(hint[1..].iter().all(|cell| styled(cell, look().muted)));
    // The top bar's own hint, in the bar's reverse video.
    let help = cells(&screen, "? help", 0);
    assert!(
        help[0]
            .modifier
            .contains(Modifier::BOLD | Modifier::REVERSED)
    );
}

#[test]
fn the_dots_between_the_status_line_s_parts_are_muted() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Point(pointer_on(&world, 1, 1)), &world);
    let screen = render(&app, &world, 100, 30);
    let tile = cells(&screen, "grass · berry bush", 29);
    assert!(styled(tile[6], look().muted), "the dot");
    assert!(tile[..5].iter().all(|cell| cell.fg == Color::Reset));
}

#[test]
fn the_quit_prompt_s_keys_and_an_overlay_s_corner_key_are_bold() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Back, &world);
    let screen = render(&app, &world, 100, 30);
    for answer in ["y quit the game", "t title screen"] {
        let answer = cells(&screen, answer, 29);
        assert!(styled(answer[0], look().key));
        assert!(answer[2..].iter().all(|cell| styled(cell, look().muted)));
    }
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Help, &world);
    let screen = render(&app, &world, 100, 30);
    let corner = cells(&screen, "esc close", 1);
    assert!(corner[..3].iter().all(|cell| styled(cell, look().key)));
    assert!(corner[4..].iter().all(|cell| styled(cell, look().muted)));
}

#[test]
fn help_s_headings_and_keys_have_their_styles() {
    let world = garden(pack());
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Help, &world);
    let screen = render(&app, &world, 100, 30);
    let look = look();
    for heading in ["TIME", "CURSOR", "COLOURS", "EMOTES"] {
        let heading = cells(&screen, heading, 1);
        assert!(heading.iter().all(|cell| styled(cell, look.heading)));
    }
    let key = cells(&screen, "space", 1);
    assert!(key.iter().all(|cell| styled(cell, look.key)));
    let does = cells(&screen, "pause / resume", 1);
    assert!(does.iter().all(|cell| cell.fg == Color::Reset), "plain");
}

#[test]
fn the_sprite_list_s_heading_and_highlight_have_their_styles() {
    let mut world = garden_with_sprites();
    run(&mut world, 3);
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::SpriteList, &world);
    let screen = render(&app, &world, 100, 30);
    let heading = cells(&screen, "Life stage", 2);
    assert!(heading.iter().all(|cell| styled(cell, look().heading)));
    let first = find_from(&screen, "Sprite #", 3);
    assert!(styled(&screen[first], look().selected), "the highlight");
    // The dots in what each is doing are minor.
    let dot = find_from(&screen, " · ", 3);
    assert!(styled(&screen[(dot.x + 1, dot.y)], look().muted));
}
