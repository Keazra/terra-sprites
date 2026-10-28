//! A palette the player picks draws its colours with `NO_COLOR` set
//! (design §6.2): the game answers `NO_COLOR` itself, so it has crossterm
//! write the colours crossterm would drop. crossterm's switch for that is
//! global, so this test has a file, and so a process, of its own.

use ratatui::crossterm::Command;
use ratatui::crossterm::style::{Color, Colored, SetForegroundColor};
use terra_tui::start;

/// What crossterm writes to turn the text `colour`.
fn written(colour: Color) -> String {
    let mut text = String::new();
    SetForegroundColor(colour).write_ansi(&mut text).unwrap();
    text
}

#[test]
fn with_no_color_set_the_game_still_writes_the_colours_it_draws() {
    // As crossterm starts with `NO_COLOR` set: no colour, and a bare reset
    // in its place, which would end bold and reverse video too.
    Colored::set_ansi_color_disabled(true);
    assert_eq!(written(Color::Red), "\x1b[m");
    start::write_colours();
    assert_eq!(written(Color::Red), "\x1b[38;5;9m");
}
