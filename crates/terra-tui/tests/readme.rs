//! The README's controls table matches the keybindings in code (design
//! §7.4, A9): every key the game binds is in the table, and every key the
//! table's first column names does something.
//!
//! The keys come from the code itself: each key a keyboard can press goes
//! through `Keys::action_for`, and any that doesn't come back as `Dismiss`
//! is bound. So a key added to `input.rs` fails this test until the README
//! lists it.

use std::collections::BTreeSet;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terra_tui::input::{Action, Keys};

const README: &str = include_str!("../../../README.md");

/// The controls table's rows, as their cells: from the row after
/// `| Key | Action |` and its rule, until the table ends.
fn controls_rows() -> Vec<Vec<String>> {
    let start = README
        .find("| Key | Action |")
        .expect("the README has a controls table");
    README[start..]
        .lines()
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            line.trim_matches('|')
                .split(" | ")
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

/// The keys a cell names: each `` `code` `` span, with `` `1`–`9` `` read as
/// every digit between, and the word "arrows" as the arrow keys.
fn keys_in(cell: &str) -> BTreeSet<String> {
    let spans: Vec<&str> = cell.split('`').skip(1).step_by(2).collect();
    let mut keys: BTreeSet<String> = spans.iter().map(|span| canonical(span)).collect();
    for pair in spans.windows(2) {
        let range = format!("`{}`–`{}`", pair[0], pair[1]);
        if let (true, [from], [to]) = (
            cell.contains(&range),
            pair[0].as_bytes(),
            pair[1].as_bytes(),
        ) {
            keys.extend((*from..=*to).map(|c| (c as char).to_string()));
        }
    }
    if cell.contains("arrows") {
        keys.insert("arrows".to_string());
    }
    keys
}

/// A key as the README and this test both name it: a letter in capitals,
/// since the game reads either case the same.
fn canonical(key: &str) -> String {
    match key.chars().collect::<Vec<_>>()[..] {
        [c] if c.is_ascii_alphabetic() => c.to_ascii_uppercase().to_string(),
        _ => key.to_string(),
    }
}

/// Every key the game binds outside a prompt, by its README name.
fn bound_keys() -> BTreeSet<String> {
    let plain = (' '..='~').map(|c| (KeyCode::Char(c), KeyModifiers::NONE));
    let ctrl = ('a'..='z').map(|c| (KeyCode::Char(c), KeyModifiers::CONTROL));
    let special = [
        KeyCode::Esc,
        KeyCode::Enter,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Insert,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
    ]
    .into_iter()
    .chain((1..=12).map(KeyCode::F))
    .map(|code| (code, KeyModifiers::NONE));
    plain
        .chain(ctrl)
        .chain(special)
        .filter(|&(code, modifiers)| {
            let action =
                Keys::with_release_reporting(false).action_for(KeyEvent::new(code, modifiers));
            action.is_some_and(|action| action != Action::Dismiss)
        })
        .map(|(code, modifiers)| name(code, modifiers))
        .collect()
}

/// The README's name for a key.
fn name(code: KeyCode, modifiers: KeyModifiers) -> String {
    let key = match code {
        KeyCode::Char(' ') => "space".to_string(),
        // The `+` key without Shift.
        KeyCode::Char('=') => "+".to_string(),
        KeyCode::Char(c) => canonical(&c.to_string()),
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => "arrows".to_string(),
        KeyCode::BackTab => "Shift+Tab".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        other => other.to_string(),
    };
    if modifiers.contains(KeyModifiers::CONTROL) {
        format!("Ctrl+{key}")
    } else {
        key
    }
}

#[test]
fn every_key_the_game_binds_is_in_the_readme_controls_table() {
    let mentioned: BTreeSet<String> = controls_rows()
        .iter()
        .flatten()
        .flat_map(|cell| keys_in(cell))
        .collect();
    let missing: Vec<String> = bound_keys().difference(&mentioned).cloned().collect();
    assert!(
        missing.is_empty(),
        "the README's controls table doesn't mention {missing:?}"
    );
}

#[test]
fn every_key_in_the_readme_controls_table_does_something() {
    let bound = bound_keys();
    for row in controls_rows() {
        for key in keys_in(&row[0]) {
            assert!(
                bound.contains(&key),
                "the README lists `{key}`, which the game doesn't bind"
            );
        }
    }
}

#[test]
fn the_check_reads_the_keys_the_game_binds() {
    // A guard on the guard: if `bound_keys` found nothing, the first test
    // would pass with any README.
    let bound = bound_keys();
    for key in [
        "space",
        "arrows",
        "W",
        "Q",
        "E",
        "F",
        "+",
        "-",
        "1",
        "9",
        "?",
        "Tab",
        "Shift+Tab",
        "PgUp",
        "Esc",
        "F5",
        "F9",
        "Ctrl+C",
        "Ctrl+S",
        "Ctrl+O",
    ] {
        assert!(bound.contains(key), "{key}");
    }
    for key in ["F1", "Home", "Ctrl+A", "Backspace", "!"] {
        assert!(!bound.contains(key), "{key}");
    }
}
