//! The commands as one list (design §6.1, #303): the keys it names do what
//! it says, every key is in it, the status line's hints come from it greyed
//! when they'd do nothing, and the command palette finds and runs them.

mod common;
use common::screen::{app_for, garden_with_sprites, render};
use common::{key, lines, status_line};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use terra_tui::app::Screen;
use terra_tui::commands::COMMANDS;
use terra_tui::input::{Action, Keys};
use terra_tui::theme::Theme;

/// The key event a key as the command list writes it stands for: `space`,
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

#[test]
fn every_key_the_list_names_does_what_it_says() {
    let mut pressed = 0;
    for command in COMMANDS {
        for each in command.keys {
            let Some(action) = each.action else {
                continue;
            };
            let got = Keys::new().action_for(event(each.key));
            assert_eq!(got, Some(action), "{} ({})", each.key, each.name);
            pressed += 1;
        }
    }
    assert!(pressed > 30, "{pressed}");
    // The palette's own key, which it can't press itself.
    assert_eq!(Keys::new().action_for(event(":")), Some(Action::Palette));
}

#[test]
fn every_key_with_a_job_in_the_world_is_in_the_list() {
    let listed: Vec<Action> = COMMANDS
        .iter()
        .flat_map(|command| command.keys)
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

/// The status line's cells of `hint` (`. step`), as drawn on `screen`.
fn hint_cells<'a>(screen: &'a Buffer, hint: &str) -> Vec<&'a ratatui::buffer::Cell> {
    let line = lines(screen)[29].clone();
    let at = line
        .find(hint)
        .unwrap_or_else(|| panic!("no {hint:?} in {line:?}"));
    let x = line[..at].chars().count() as u16;
    (0..hint.chars().count() as u16)
        .map(|i| &screen[(x + i, 29)])
        .collect()
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

/// Types `text` into whatever's open.
fn type_in(app: &mut terra_tui::app::App, world: &terra_sim::World, text: &str) {
    for c in text.chars() {
        app.apply(Action::Type(c), world);
    }
}

#[test]
fn colon_opens_the_palette_which_finds_a_command_by_the_starts_of_its_words() {
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
    assert_eq!(rows[0].1.name, "family tree");
    let screen = lines(&render(&app, &world, 100, 30));
    let found = &screen[top + 2];
    assert!(found.contains(" family tree · sprites"), "{found}");
    let key_at = found.find(" k │").expect("its key on the right");
    assert!(found[..key_at].ends_with("     "), "{found}");

    // Words in any order, each the start of one.
    app.apply(Action::Back, &world);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "gen sav");
    let names: Vec<&str> = app.palette_rows().iter().map(|(_, k)| k.name).collect();
    assert_eq!(names, ["save its genome"]);

    // And by other words for it (design §6.1).
    app.apply(Action::Back, &world);
    app.apply(Action::Palette, &world);
    type_in(&mut app, &world, "egg");
    let names: Vec<&str> = app.palette_rows().iter().map(|(_, k)| k.name).collect();
    assert_eq!(
        names,
        ["population graph", "family tree", "save its genome"]
    );
}

#[test]
fn enter_runs_the_chosen_command_and_esc_closes_the_palette() {
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
    let names: Vec<&str> = app.palette_rows().iter().map(|(_, k)| k.name).collect();
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
    for (command, _) in app.palette_rows() {
        assert!(
            command
                .headings
                .iter()
                .any(|heading| heading.in_the_world()),
            "{}",
            command.written
        );
    }
    // The sprite list's keys aren't there; opening the list is.
    type_in(&mut app, &world, "sort");
    assert!(app.palette_rows().is_empty());
}
