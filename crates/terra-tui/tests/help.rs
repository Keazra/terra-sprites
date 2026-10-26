//! The help screen (`?`, design §6.1): the keys, the colour legend and the
//! game's folder, on pages for each screen, searched by typing (#303).

mod common;
use common::{
    DATA_FOLDER, DRIVE_COLOURS, WorldBuilder, app_in, app_on, at, lines, render, status_line,
    type_in,
};

use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::style::Color;
use terra_sim::World;
use terra_tui::app::{App, Flow, Screen};
use terra_tui::input::Action;
use terra_tui::theme::Theme;

/// A 10×5 field with a starter sprite on (2, 3).
fn field() -> World {
    WorldBuilder::grass(10, 5)
        .starters(&[at(2, 3)])
        .seed(7)
        .build()
}

#[test]
fn question_mark_opens_help_over_the_panels_and_esc_or_question_mark_closes_it() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    assert_eq!(app.screen(), Screen::Help);
    let screen = lines(&render(&app, &world));
    assert!(screen[0].starts_with(" Terra Sprites"), "the top bar stays");
    assert!(screen[1].starts_with("┌─ Help ─"), "{}", screen[1]);
    assert!(screen[1].ends_with(" esc close ─┐"), "{}", screen[1]);
    assert_eq!(
        screen[28].chars().next(),
        Some('└'),
        "down to the status line"
    );
    let all = screen.join("\n");
    for key in [
        "space",
        "pause / resume",
        "track the selected",
        "event filter",
        "sprite list",
        "show to sprites",
    ] {
        assert!(all.contains(key), "{key}");
    }
    assert!(!all.contains("Events"), "the event log is covered");
    assert!(all.contains(&format!("FILES    {DATA_FOLDER}")));

    assert_eq!(app.apply(Action::Back, &world), Flow::Continue);
    assert_eq!(
        app.screen(),
        Screen::Normal,
        "Esc closes it, not asks to quit"
    );
    app.apply(Action::Help, &world);
    app.apply(Action::Help, &world);
    assert_eq!(app.screen(), Screen::Normal);
}

#[test]
fn while_help_is_open_other_keys_do_nothing() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::Scroll { dx: 1, dy: 0 }, &world);
    assert_eq!(app.selection(), None);
    assert_eq!(app.screen(), Screen::Help);
    assert_eq!(
        app.apply(Action::Quit, &world),
        Flow::Quit,
        "Ctrl+C still quits"
    );
}

#[test]
fn the_legend_shows_each_drive_s_colour_and_each_emote() {
    for (theme, sprite, emotes) in [
        (Theme::cp437(), "☺", ["!", "‼", "?", "z", "♥", "♫", "♪"]),
        (Theme::ascii(), "@", ["!", "/", "?", "z", "+", "^", "\""]),
    ] {
        let world = field();
        let mut app = app_in(&world, theme);
        app.apply(Action::Help, &world);
        let screen = render(&app, &world);
        let rows = lines(&screen);
        let y = rows
            .iter()
            .position(|row| row.contains(" hunger "))
            .expect("the colour legend") as u16;
        // The legend wraps onto as many rows as it needs, up to the emotes.
        let emotes_at = rows
            .iter()
            .position(|row| row.contains(" EMOTES "))
            .expect("the emote legend") as u16;
        let colours: Vec<(String, Color)> = (y..emotes_at)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .filter(|&(x, y)| screen[(x, y)].symbol() == sprite)
            .map(|(x, y)| {
                let name: String = (x + 2..100)
                    .map(|x| screen[(x, y)].symbol())
                    .collect::<String>()
                    .split("  ")
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('│')
                    .trim_end()
                    .to_string();
                (name, screen[(x, y)].fg)
            })
            .collect();
        let expected: Vec<(String, Color)> = DRIVE_COLOURS
            .iter()
            .chain(&[("none", Color::White)])
            .map(|&(name, colour)| (name.to_string(), colour))
            .collect();
        assert_eq!(colours, expected);
        let emote_row = rows[usize::from(emotes_at)]
            .trim_start_matches('│')
            .trim_end_matches('│')
            .trim_end();
        // A dance and a call too (M2 design §15.8).
        let expected = format!(
            " EMOTES   {} hurt  {} zapped  {} gave up  {} resting  {} pleased  {} danced  {} called",
            emotes[0], emotes[1], emotes[2], emotes[3], emotes[4], emotes[5], emotes[6]
        );
        assert_eq!(emote_row, expected);
    }
}

#[test]
fn the_lineage_line_draws_each_sprite_in_its_colour() {
    // M2 design §17.4: `LINEAGE  ☺ parents, grandparents  ☺ brothers,
    // sisters  ☺ young, theirs  ☺ the rest`, each `☺` in its colour.
    for (theme, sprite) in [(Theme::cp437(), "☺"), (Theme::ascii(), "@")] {
        let world = field();
        let mut app = app_in(&world, theme);
        app.apply(Action::Help, &world);
        let screen = render(&app, &world);
        let y = lines(&screen)
            .iter()
            .position(|row| row.contains(" LINEAGE "))
            .expect("the lineage line") as u16;
        let colours: Vec<(String, Color)> = (0..100)
            .filter(|&x| screen[(x, y)].symbol() == sprite)
            .map(|x| {
                let who: String = (x + 2..100)
                    .map(|x| screen[(x, y)].symbol())
                    .collect::<String>()
                    .split("  ")
                    .next()
                    .unwrap_or("")
                    .to_string();
                (who, screen[(x, y)].fg)
            })
            .collect();
        let expected = [
            ("parents, grandparents", Color::LightBlue),
            ("brothers, sisters", Color::LightRed),
            ("young, theirs", Color::LightMagenta),
            ("the rest", Color::DarkGray),
        ]
        .map(|(who, colour)| (who.to_string(), colour));
        assert_eq!(colours, expected);
    }
}

/// The help's tabs, in the top edge of its box, the open one in the
/// highlight's style.
fn open_page(screen: &Buffer) -> String {
    let selected = Theme::cp437().text().selected;
    assert!(!selected.add_modifier.is_empty(), "reversed");
    (0..screen.area.width)
        .map(|x| &screen[(x, 1)])
        .filter(|cell| cell.modifier.contains(selected.add_modifier))
        .map(|cell| cell.symbol())
        .collect::<String>()
        .trim()
        .to_string()
}

#[test]
fn help_opens_at_the_page_for_where_the_player_is_and_goes_back_there() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    assert_eq!(app.help_page().label(), "World");
    assert_eq!(open_page(&render(&app, &world)), "World");
    app.apply(Action::Back, &world);

    // Over the sprite list, the list's page; Esc goes back to the list.
    app.apply(Action::SpriteList, &world);
    app.apply(Action::Help, &world);
    assert_eq!(app.help_page().label(), "Sprite list");
    let all = lines(&render(&app, &world)).join("\n");
    assert!(
        all.contains("SPRITE LIST") && all.contains("COLUMNS"),
        "{all}"
    );
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::SpriteList);
    app.apply(Action::Back, &world);

    for (open, page) in [
        (Action::Population, "Population graph"),
        (Action::FamilyTree, "Family tree"),
    ] {
        app.apply(Action::SelectNext, &world);
        app.apply(open, &world);
        let screen = app.screen();
        app.apply(Action::Help, &world);
        assert_eq!(app.help_page().label(), page);
        app.apply(Action::Help, &world);
        assert_eq!(app.screen(), screen, "? closes it, back to {page}");
        app.apply(Action::Back, &world);
    }

    // Once the player uses the inspector's keys, the inspector's page,
    // with the open tab's legend.
    app.apply(Action::NextTab, &world);
    app.apply(Action::Help, &world);
    assert_eq!(app.help_page().label(), "Inspector");
    let all = lines(&render(&app, &world)).join("\n");
    assert!(all.contains("THE POINTER"), "{all}");
    assert!(all.contains("open the tab clicked"), "{all}");
    assert!(all.contains("BRAIN TAB"), "{all}");
    app.apply(Action::Back, &world);
    // Back on the map, the world's page again.
    app.apply(Action::Scroll { dx: 1, dy: 0 }, &world);
    app.apply(Action::Help, &world);
    assert_eq!(app.help_page().label(), "World");
}

#[test]
fn the_top_bar_names_the_page_question_mark_would_open() {
    let world = field();
    let mut app = app_on(&world);
    let ends = |app: &App, hint: &str| {
        let bar = lines(&render(app, &world))[0].trim_end().to_string();
        assert!(bar.ends_with(&format!("  {hint}")), "{bar}");
    };
    ends(&app, "? help (World)");
    app.apply(Action::SpriteList, &world);
    ends(&app, "? help (Sprite list)");
    app.apply(Action::Back, &world);
    app.apply(Action::NextTab, &world);
    ends(&app, "? help (Inspector)");
    // Open, `?` closes it, so it names no page.
    app.apply(Action::Help, &world);
    ends(&app, "? help");
}

#[test]
fn pointing_at_the_inspector_opens_help_at_its_page_and_at_the_map_the_world_s() {
    let world = field();
    let mut app = app_on(&world);
    let page = |app: &App| app.help_page_here().label();
    // The inspector's lines, right of its left column at 54.
    app.apply(Action::Point(Position::new(70, 10)), &world);
    assert_eq!(page(&app), "Inspector");
    let bar = lines(&render(&app, &world))[0].trim_end().to_string();
    assert!(bar.ends_with("  ? help (Inspector)"), "{bar}");
    app.apply(Action::Help, &world);
    assert_eq!(app.help_page().label(), "Inspector");
    app.apply(Action::Back, &world);
    // A tile on the map.
    app.apply(Action::Point(Position::new(20, 10)), &world);
    assert_eq!(page(&app), "World");
    // A click on one of the inspector's tabs, in its title row.
    let title = lines(&render(&app, &world))[1].clone();
    let at = title.find("Brain").expect("the Brain tab");
    let x = title[..at].chars().count() as u16;
    app.apply(Action::left_click(Position::new(x, 1)), &world);
    assert_eq!(page(&app), "Inspector");
    // The wheel over the map changes mode, and is on the map.
    app.apply(
        Action::Wheel {
            at: Position::new(20, 10),
            notches: 1,
        },
        &world,
    );
    assert_eq!(page(&app), "World");
}

#[test]
fn a_click_on_a_page_s_name_opens_it_and_ends_a_search() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    let edge = lines(&render(&app, &world))[1].clone();
    let column = |name: &str| {
        let at = edge
            .find(name)
            .unwrap_or_else(|| panic!("{name} in {edge}"));
        edge[..at].chars().count() as u16
    };
    let click = |app: &mut App, x: u16, y: u16| {
        app.apply(Action::left_click(Position::new(x, y)), &world);
    };
    click(&mut app, column("Family tree") + 10, 1);
    assert_eq!(app.help_page().label(), "Family tree");
    type_in(&mut app, &world, "pause");
    click(&mut app, column("Inspector") - 1, 1);
    assert_eq!(app.help_page().label(), "Inspector", "the space before it");
    assert_eq!(app.help_search(), None, "the search ended");
    click(&mut app, column("Help"), 1);
    assert_eq!(app.help_page().label(), "Inspector", "the help's own name");
    click(&mut app, column("World"), 2);
    assert_eq!(app.help_page().label(), "Inspector", "below the edge");
    assert_eq!(app.screen(), Screen::Help);
}

#[test]
fn the_arrows_and_tab_turn_help_s_pages() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    let mut pages = Vec::new();
    for _ in 0..5 {
        app.apply(Action::Scroll { dx: 1, dy: 0 }, &world);
        pages.push(app.help_page().label());
    }
    assert_eq!(
        pages,
        [
            "Inspector",
            "Sprite list",
            "Population graph",
            "Family tree",
            "World"
        ]
    );
    app.apply(Action::Scroll { dx: -1, dy: 0 }, &world);
    assert_eq!(app.help_page().label(), "Family tree", "round the back");
    // Tab and Shift+Tab, as typing reads them.
    app.apply(Action::AnotherName, &world);
    assert_eq!(app.help_page().label(), "World");
    app.apply(Action::SelectPrevious, &world);
    assert_eq!(app.help_page().label(), "Family tree");
    assert_eq!(open_page(&render(&app, &world)), "Family tree");
}

#[test]
fn typing_in_help_searches_every_page_and_acts_on_nothing() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    let paused = app.clock.is_paused();
    type_in(&mut app, &world, "egg");
    assert_eq!(app.help_search(), Some("egg"));
    assert_eq!(app.clock.is_paused(), paused);
    let screen = lines(&render(&app, &world));
    assert!(
        screen[28].starts_with("└─ search: egg_ ─"),
        "{}",
        screen[28]
    );
    let all = screen.join("\n");
    // Rows and legend lines with a word starting `egg`, under their pages.
    for found in [
        "WORLD",
        "the Place menu",
        "INSPECTOR",
        "BODY TAB",
        "An egg shows who laid it",
        "FAMILY TREE",
    ] {
        assert!(all.contains(found), "{found} in\n{all}");
    }
    assert!(!all.contains("pause / resume"), "{all}");

    // Typing that would act in the world types: `p`, space.
    type_in(&mut app, &world, " p");
    assert_eq!(app.help_search(), Some("egg p"));
    assert_eq!(app.screen(), Screen::Help);
    app.apply(Action::Erase, &world);
    app.apply(Action::Erase, &world);
    assert_eq!(app.help_search(), Some("egg"));

    type_in(&mut app, &world, "zzq");
    let all = lines(&render(&app, &world)).join("\n");
    assert!(all.contains("Nothing in the help has \"eggzzq\""), "{all}");

    // Esc clears the search, then closes the help.
    app.apply(Action::Back, &world);
    assert_eq!(app.help_search(), None);
    assert_eq!(app.screen(), Screen::Help);
    let screen = lines(&render(&app, &world));
    assert!(
        screen[28].starts_with("└─ type to search ─"),
        "{}",
        screen[28]
    );
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal);
}

#[test]
fn while_help_searches_esc_says_it_clears() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    type_in(&mut app, &world, "egg");
    let line = status_line(&app, &world);
    assert!(line.ends_with("type to search  esc clear"), "{line:?}");
    let screen = lines(&render(&app, &world));
    assert!(screen[1].ends_with(" esc clear ─┐"), "{}", screen[1]);
}

#[test]
fn help_scrolls_a_page_too_long_to_fit_and_no_further() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    // The World page fits at 100×30.
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    assert_eq!(app.help_scroll(), 0);
    // Searching for `s` finds more than fits.
    type_in(&mut app, &world, "s");
    for _ in 0..200 {
        app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    }
    let last = app.help_scroll();
    assert!(last > 0, "it scrolls");
    let at = ratatui::layout::Position::new(10, 10);
    app.apply(Action::Wheel { at, notches: 5 }, &world);
    assert_eq!(app.help_scroll(), last, "no further than its last line");
    app.apply(Action::Scroll { dx: 0, dy: -1 }, &world);
    assert_eq!(app.help_scroll(), last - 1, "up at once");
    // Four rows taller, it's drawn from four lines further up, and goes up
    // from there at once.
    app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    app.fit(terra_tui::ui::areas(ratatui::layout::Size::new(100, 34)));
    app.apply(Action::Scroll { dx: 0, dy: -1 }, &world);
    assert_eq!(app.help_scroll(), last - 5, "up one from where it's drawn");
}

#[test]
fn turning_help_s_page_ends_a_search() {
    let world = field();
    let mut app = app_on(&world);
    app.apply(Action::Help, &world);
    type_in(&mut app, &world, "egg");
    app.apply(Action::Scroll { dx: 1, dy: 0 }, &world);
    assert_eq!(app.help_search(), None);
    assert_eq!(app.help_page().label(), "Inspector");
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal, "Esc closes it at once");
}
