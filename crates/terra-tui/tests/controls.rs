//! The controls as one list (design §6.1, #303): the keys it names do what
//! it says, every key is in it, the status line's hints come from it greyed
//! when they'd do nothing, and the command palette finds and runs them.

mod common;
use common::screen::{app_for, garden_with_sprites, hint_cells, render};
use common::{key, lines, status_line, type_in};

use ratatui::buffer::Cell;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use terra_tui::app::{CursorMode, Screen};
use terra_tui::controls::CONTROLS;
use terra_tui::input::{Action, Keys};
use terra_tui::theme::{TextLook, Theme};

/// The key event a key as the control list writes it stands for: `space`,
/// `Shift+tab`, `Ctrl+S`, `F5`, `PgUp`, or a key's own letter, which the
/// list writes as on the keyboard (`T`) and is pressed without Shift.
fn event(written: &str) -> KeyEvent {
    let (modifiers, rest) = if let Some(rest) = written.strip_prefix("Ctrl+") {
        (KeyModifiers::CONTROL, rest)
    } else if let Some(rest) = written.strip_prefix("Shift+") {
        (KeyModifiers::SHIFT, rest)
    } else {
        (KeyModifiers::NONE, written)
    };
    let code = match rest {
        "space" => KeyCode::Char(' '),
        "esc" => KeyCode::Esc,
        "tab" if modifiers == KeyModifiers::SHIFT => KeyCode::BackTab,
        "tab" => KeyCode::Tab,
        "enter" => KeyCode::Enter,
        "PgUp" => KeyCode::PageUp,
        "PgDn" => KeyCode::PageDown,
        f if f.starts_with('F') && f.len() > 1 => KeyCode::F(f[1..].parse().expect("an F key")),
        c if c.chars().count() == 1 => {
            let c = c.chars().next().expect("a character");
            match modifiers {
                KeyModifiers::SHIFT => KeyCode::Char(c.to_ascii_uppercase()),
                _ => KeyCode::Char(c.to_ascii_lowercase()),
            }
        }
        other => panic!("no key {other:?}"),
    };
    KeyEvent::new(code, modifiers)
}

/// Whether a hint's key, its first cell, is drawn as a key that works:
/// bold, where a greyed one is muted (#303).
fn ready(key: &Cell, look: TextLook) -> bool {
    key.modifier.contains(look.key.add_modifier)
}

#[test]
fn every_key_the_list_names_does_what_it_says() {
    let mut pressed = 0;
    for control in CONTROLS {
        for each in control.keys {
            let Some(action) = each.action else {
                continue;
            };
            let got = Keys::new().action_for(event(each.written));
            assert_eq!(got, Some(action), "{} ({})", each.written, each.does);
            pressed += 1;
        }
    }
    assert!(pressed > 30, "{pressed}");
    // The palette's own key, which it can't press itself.
    assert_eq!(Keys::new().action_for(event(":")), Some(Action::Palette));
}

#[test]
fn every_key_with_a_job_in_the_world_is_in_the_list() {
    let listed: Vec<Action> = CONTROLS
        .iter()
        .flat_map(|control| control.keys)
        .filter_map(|each| each.action)
        .collect();
    let mut events: Vec<KeyEvent> = (' '..='~')
        .flat_map(|c| {
            [
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
            ]
        })
        .collect();
    events.extend((1..=12).map(|n| key(KeyCode::F(n), KeyEventKind::Press)));
    events.extend(
        [
            KeyCode::Esc,
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Enter,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::Insert,
            KeyCode::Delete,
            KeyCode::Backspace,
        ]
        .map(|code| key(code, KeyEventKind::Press)),
    );
    for event in events {
        let Some(action) = Keys::new().action_for(event) else {
            continue;
        };
        // Keys the help gives in other words: scrolling (`WASD`), the
        // sprite list's (`/`, `enter`), a prompt's (`y`) and a menu's
        // (`1`–`9`); the palette's own (`:`), and keys with no job.
        let elsewhere = matches!(
            action,
            Action::Scroll { .. }
                | Action::FilterList
                | Action::Enter
                | Action::Confirm
                | Action::Pick(_)
                | Action::Palette
                | Action::Dismiss
        );
        assert!(
            elsewhere || listed.contains(&action),
            "{event:?} does {action:?}, which the list hasn't"
        );
    }
}

#[test]
fn a_hint_that_would_do_nothing_is_greyed_not_hidden() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let look = app.theme.text();
    let screen = render(&app, &world, 100, 30);
    // Time runs, so `.` would step nothing: its key is muted like its word,
    // where a key that works is bold.
    let step = hint_cells(&screen, ". step");
    assert!(step.iter().all(|cell| cell.fg == look.muted.fg.unwrap()));
    assert!(!step[0].modifier.contains(look.key.add_modifier));
    let pause = hint_cells(&screen, "space pause");
    assert!(pause[0].modifier.contains(look.key.add_modifier), "bold");

    app.apply(Action::TogglePause, &world);
    let screen = render(&app, &world, 100, 30);
    let step = hint_cells(&screen, ". step");
    assert!(step[0].modifier.contains(look.key.add_modifier), "bold");
    assert!(look.key.fg.is_none_or(|fg| step[0].fg == fg));
}

#[test]
fn the_world_s_hints_and_help_s_come_from_the_list() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let line = status_line(&app, &world);
    for hint in [
        "Z select",
        "X train",
        "C grab",
        "WASD scroll",
        "space pause",
        ". step",
        "+/- speed",
    ] {
        assert!(line.contains(hint), "{hint} in {line:?}");
    }
    app.apply(Action::Help, &world);
    let line = status_line(&app, &world);
    assert!(
        line.ends_with("←→ page  type to search  esc close"),
        "{line:?}"
    );
}

#[test]
fn colon_opens_the_palette_which_finds_a_control_by_the_starts_of_its_words() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Palette, &world);
    assert_eq!(app.screen(), Screen::Palette);
    let every = app.palette_rows().len();
    assert!(every > 30, "every key the world has, with nothing typed");
    let screen = lines(&render(&app, &world, 100, 30));
    let top = screen
        .iter()
        .position(|row| row.contains("┌─ Commands ─"))
        .expect("the palette");
    assert!(screen[top].contains(" esc close ─┐"), "{}", screen[top]);
    assert!(screen[top + 1].contains("│:_"), "{}", screen[top + 1]);

    type_in(&mut app, &world, "tree");
    let rows = app.palette_rows();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].key.does, "family tree");
    let screen = lines(&render(&app, &world, 100, 30));
    let found = &screen[top + 2];
    assert!(found.contains(" family tree · sprites"), "{found}");
    let key_at = found.find(" k │").expect("its key on the right");
    assert!(found[..key_at].ends_with("     "), "{found}");

    // Words in any order, each the start of one.
    app.apply(Action::Back, &world);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "gen sav");
    let names: Vec<&str> = app.palette_rows().iter().map(|row| row.key.does).collect();
    assert_eq!(names, ["save its genome"]);

    // And by other words for it (design §6.1).
    app.apply(Action::Back, &world);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "egg");
    let names: Vec<&str> = app.palette_rows().iter().map(|row| row.key.does).collect();
    assert_eq!(
        names,
        [
            "the Place menu",
            "population graph",
            "family tree",
            "save its genome"
        ]
    );
}

#[test]
fn the_palette_and_help_find_the_settings_by_each_built_in_palette_s_name() {
    // Design §6.7: the settings screen's Colours row goes through high
    // contrast, colour-blind, mono and Game Kid, so a player who has read
    // of one finds where it's chosen by its name.
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    for typed in ["contrast", "blind", "mono", "game kid", "kid", "green"] {
        app.apply(Action::Palette, &world);
        type_in(&mut app, &world, typed);
        let names: Vec<&str> = app.palette_rows().iter().map(|row| row.key.does).collect();
        assert!(names.contains(&"settings"), "{typed}: {names:?}");
        app.apply(Action::Back, &world);

        app.apply(Action::Help, &world);
        type_in(&mut app, &world, typed);
        let screen = lines(&render(&app, &world, 100, 30));
        assert!(
            screen.iter().any(|row| row.contains("Ctrl+T     settings")),
            "{typed}:\n{}",
            screen.join("\n")
        );
        app.apply(Action::Back, &world);
        app.apply(Action::Back, &world);
        assert_eq!(app.screen(), Screen::Normal);
    }
}

#[test]
fn enter_runs_the_chosen_control_and_esc_closes_the_palette() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "tree");
    app.apply(Action::Enter, &world);
    assert_eq!(app.screen(), Screen::FamilyTree);
    assert_eq!(app.palette_text(), None, "closed");

    app.apply(Action::Back, &world);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "pa");
    let names: Vec<&str> = app.palette_rows().iter().map(|row| row.key.does).collect();
    assert!(names.contains(&"pause or resume"), "{names:?}");
    let pause = names.iter().position(|&n| n == "pause or resume").unwrap();
    for _ in 0..pause {
        app.apply(Action::Scroll { dx: 0, dy: 1 }, &world);
    }
    assert_eq!(app.palette_choice(), pause);
    let paused = app.clock.is_paused();
    app.apply(Action::Enter, &world);
    assert_eq!(app.clock.is_paused(), !paused, "paused or resumed");
    assert_eq!(app.screen(), Screen::Normal);

    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "p");
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::Normal, "closed, not asking to quit");
}

#[test]
fn a_palette_that_finds_nothing_says_so_and_greys_its_enter() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "xyzzy");
    assert!(app.palette_rows().is_empty());
    let screen = render(&app, &world, 100, 30);
    assert!(
        lines(&screen)
            .iter()
            .any(|row| row.contains(" Nothing found"))
    );
    let look = app.theme.text();
    let enter = hint_cells(&screen, "enter do it");
    assert!(enter.iter().all(|cell| cell.fg == look.muted.fg.unwrap()));
    // Enter with nothing found just closes it.
    app.apply(Action::Enter, &world);
    assert_eq!(app.screen(), Screen::Normal);
}

#[test]
fn the_palette_lists_only_what_works_in_the_world() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Palette, &world);
    for row in app.palette_rows() {
        let control = row.control;
        assert!(
            control
                .headings
                .iter()
                .any(|heading| heading.in_the_world()),
            "{}",
            control.written
        );
    }
    // The sprite list's keys aren't there; opening the list is.
    type_in(&mut app, &world, "sort");
    assert!(app.palette_rows().is_empty());
}

#[test]
fn hints_with_nothing_to_choose_or_no_sprite_are_greyed() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let look = app.theme.text();
    // A filter that finds no sprite: nothing to choose or go to.
    app.apply(Action::SpriteList, &world);
    app.apply(Action::FilterList, &world);
    type_in(&mut app, &world, "zzz");
    let screen = render(&app, &world, 100, 30);
    assert!(!ready(hint_cells(&screen, "↑↓ choose")[0], look));
    assert!(!ready(hint_cells(&screen, "enter go to it")[0], look));
    assert!(ready(hint_cells(&screen, "tab sort")[0], look));
    app.apply(Action::Back, &world);
    let screen = render(&app, &world, 100, 30);
    assert!(ready(hint_cells(&screen, "↑↓ choose")[0], look), "the list");
    app.apply(Action::Back, &world);

    // The family tree with no sprite selected: nothing to choose on it.
    app.apply(Action::FamilyTree, &world);
    let screen = render(&app, &world, 100, 30);
    assert!(!ready(hint_cells(&screen, "←→↑↓ choose")[0], look));
    assert!(!ready(hint_cells(&screen, "enter centre on it")[0], look));
    assert!(ready(hint_cells(&screen, "esc close")[0], look));
    app.apply(Action::Back, &world);
    app.apply(Action::SelectNext, &world);
    app.apply(Action::FamilyTree, &world);
    let screen = render(&app, &world, 100, 30);
    assert!(ready(hint_cells(&screen, "←→↑↓ choose")[0], look));
}

#[test]
fn a_menu_gives_its_own_hints_and_question_mark_opens_help_over_it() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    app.apply(Action::Mode(CursorMode::Grab), &world);
    assert_eq!(app.screen(), Screen::PlaceMenu);
    let line = status_line(&app, &world);
    assert!(
        line.ends_with("↑↓ choose  enter pick  esc close"),
        "{line:?}"
    );
    app.apply(Action::Help, &world);
    assert_eq!(app.screen(), Screen::Help);
    app.apply(Action::Back, &world);
    assert_eq!(app.screen(), Screen::PlaceMenu, "back to the menu");
}

#[test]
fn the_top_bar_s_help_is_greyed_while_question_mark_would_type() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let look = app.theme.text();
    let help_key = |app: &terra_tui::app::App| {
        let screen = render(app, &world, 100, 30);
        let bar = lines(&screen)[0].clone();
        let at = bar.find("? help").expect("the top bar's help");
        screen[(bar[..at].chars().count() as u16, 0)].clone()
    };
    assert!(ready(&help_key(&app), look));
    app.apply(Action::Palette, &world);
    assert!(!ready(&help_key(&app), look), "`?` types into the palette");
    app.apply(Action::Back, &world);
    app.apply(Action::Help, &world);
    assert!(ready(&help_key(&app), look), "`?` closes help");
}

#[test]
fn the_palette_teaches_keys_it_can_t_press_and_the_wheel_chooses() {
    let world = garden_with_sprites();
    let mut app = app_for(&world, Theme::cp437(), 100, 30);
    let look = app.theme.text();
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "place");
    let rows = app.palette_rows();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].key.written, "C again");
    assert_eq!(rows[0].key.action, None);
    let screen = render(&app, &world, 100, 30);
    assert!(!ready(hint_cells(&screen, "enter do it")[0], look));
    // Enter on it just closes the palette: the player has the key.
    app.apply(Action::Enter, &world);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.mode(), CursorMode::Select);

    app.apply(Action::Palette, &world);
    let at = ratatui::layout::Position::new(20, 5);
    app.apply(Action::Wheel { at, notches: 2 }, &world);
    assert_eq!(app.palette_choice(), 2);
    app.apply(Action::Wheel { at, notches: -1 }, &world);
    assert_eq!(app.palette_choice(), 1);
    let screen = render(&app, &world, 100, 30);
    assert!(ready(hint_cells(&screen, "enter do it")[0], look));
}
