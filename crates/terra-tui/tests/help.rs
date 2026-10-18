//! The help screen (`?`, design §6.1): the keys, the colour legend and the
//! game's folder.

mod common;
use common::{DATA_FOLDER, WorldBuilder, app_in, at, lines, render};

use std::path::PathBuf;

use ratatui::style::Color;
use terra_sim::World;
use terra_tui::app::{App, Flow, Screen};
use terra_tui::input::Action;
use terra_tui::theme::Theme;

/// A 10×5 field with a starter sprite on (2, 3).
fn field() -> World {
    WorldBuilder::grass(10, 5)
        .sprites(&[at(2, 3)])
        .seed(7)
        .build()
}

/// An app on `world` in `theme`, on a 100×30 screen, with a fixed folder
/// for its files.
fn app_for(world: &World, theme: Theme) -> App {
    let mut app = app_in(world, theme);
    app.set_data_folder(PathBuf::from(DATA_FOLDER));
    app
}

#[test]
fn question_mark_opens_help_over_the_panels_and_esc_or_question_mark_closes_it() {
    let world = field();
    let mut app = app_for(&world, Theme::cp437());
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
    let mut app = app_for(&world, Theme::cp437());
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
        let mut app = app_for(&world, theme);
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
        let expected = [
            ("hunger", Color::Yellow),
            ("thirst", Color::Cyan),
            ("pain", Color::Red),
            ("tiredness", Color::Blue),
            ("boredom", Color::DarkGray),
            ("loneliness", Color::Magenta),
            ("crowdedness", Color::LightRed),
            ("arousal", Color::LightMagenta),
            ("anxiety", Color::LightGreen),
            ("none", Color::White),
        ]
        .map(|(name, colour)| (name.to_string(), colour));
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
        let mut app = app_for(&world, theme);
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
