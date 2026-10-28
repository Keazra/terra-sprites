//! How a screen cell looks in a picture: its character, and its colours in
//! Windows Terminal's default scheme, Campbell (#241).

use ratatui::buffer::Cell;
use ratatui::style::Modifier;

// The scheme is the library's, which motion blends named colours from too.
pub use terra_tui::campbell::{BACKGROUND, Rgb};
use terra_tui::campbell::{FOREGROUND, rgb};

/// How a screen cell is drawn: its character in its colours, in the bold
/// face or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Look {
    pub symbol: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
}

/// How `cell` is drawn. Reverse video swaps its colours, the terminal's
/// own included, as Windows Terminal does.
pub fn look(cell: &Cell) -> Look {
    let fg = rgb(cell.fg).unwrap_or(FOREGROUND);
    let bg = rgb(cell.bg).unwrap_or(BACKGROUND);
    let (fg, bg) = if cell.modifier.contains(Modifier::REVERSED) {
        (bg, fg)
    } else {
        (fg, bg)
    };
    Look {
        symbol: cell.symbol().chars().next().unwrap_or(' '),
        fg,
        bg,
        bold: cell.modifier.contains(Modifier::BOLD),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Cell;
    use ratatui::style::{Color, Modifier, Style};

    use super::*;

    fn cell(symbol: &str, style: Style) -> Cell {
        let mut cell = Cell::default();
        cell.set_symbol(symbol).set_style(style);
        cell
    }

    #[test]
    fn the_sixteen_colours_are_campbell_s() {
        let fg = |colour: Color| look(&cell("x", Style::default().fg(colour))).fg;
        assert_eq!(fg(Color::Black), [0x0C, 0x0C, 0x0C]);
        assert_eq!(fg(Color::Yellow), [0xC1, 0x9C, 0x00]);
        assert_eq!(fg(Color::Gray), [0xCC, 0xCC, 0xCC]);
        assert_eq!(fg(Color::DarkGray), [0x76, 0x76, 0x76]);
        assert_eq!(fg(Color::LightYellow), [0xF9, 0xF1, 0xA5]);
        assert_eq!(fg(Color::LightMagenta), [0xB4, 0x00, 0x9E]);
        assert_eq!(fg(Color::White), [0xF2, 0xF2, 0xF2]);
    }

    #[test]
    fn a_plain_cell_is_the_terminal_s_own_colours() {
        let plain = look(&cell("a", Style::default()));
        assert_eq!(
            plain,
            Look {
                symbol: 'a',
                fg: [0xCC, 0xCC, 0xCC],
                bg: [0x0C, 0x0C, 0x0C],
                bold: false,
            }
        );
    }

    #[test]
    fn reverse_video_swaps_the_colours_and_bold_takes_the_bold_face() {
        let style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD);
        let drawn = look(&cell("@", style));
        assert_eq!(
            (drawn.fg, drawn.bg, drawn.bold),
            ([0x0C, 0x0C, 0x0C], [0x13, 0xA1, 0x0E], true)
        );
        // The top bar: reverse video with the terminal's own colours.
        let bar = look(&cell(
            "T",
            Style::default().add_modifier(Modifier::REVERSED),
        ));
        assert_eq!((bar.fg, bar.bg), ([0x0C, 0x0C, 0x0C], [0xCC, 0xCC, 0xCC]));
    }
}
