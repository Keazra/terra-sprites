//! How a screen cell looks in a picture: its character, and its colours in
//! Windows Terminal's default scheme, Campbell (#241).

use ratatui::buffer::Cell;
use ratatui::style::{Color, Modifier};

/// A colour, as red, green and blue.
pub type Rgb = [u8; 3];

/// The terminal's own text colour, where the game leaves it.
pub const FOREGROUND: Rgb = [0xCC, 0xCC, 0xCC];
/// The terminal's own background.
pub const BACKGROUND: Rgb = [0x0C, 0x0C, 0x0C];

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

/// `colour` in Campbell, or `None` for the terminal's own.
fn rgb(colour: Color) -> Option<Rgb> {
    Some(match colour {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(index) => indexed(index),
        named => CAMPBELL[named_index(named)],
    })
}

/// The 16 colours, in the order of their indexes: black, red, green,
/// yellow, blue, magenta, cyan and white (ratatui's gray), then their
/// bright forms (dark gray to white).
const CAMPBELL: [Rgb; 16] = [
    [0x0C, 0x0C, 0x0C],
    [0xC5, 0x0F, 0x1F],
    [0x13, 0xA1, 0x0E],
    [0xC1, 0x9C, 0x00],
    [0x00, 0x37, 0xDA],
    [0x88, 0x17, 0x98],
    [0x3A, 0x96, 0xDD],
    [0xCC, 0xCC, 0xCC],
    [0x76, 0x76, 0x76],
    [0xE7, 0x48, 0x56],
    [0x16, 0xC6, 0x0C],
    [0xF9, 0xF1, 0xA5],
    [0x3B, 0x78, 0xFF],
    [0xB4, 0x00, 0x9E],
    [0x61, 0xD6, 0xD6],
    [0xF2, 0xF2, 0xF2],
];

/// A named colour's index among the 16.
fn named_index(colour: Color) -> usize {
    match colour {
        Color::Black => 0,
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        _ => 15,
    }
}

/// One of the 256 indexed colours: the 16, then the 6×6×6 cube and the
/// grays, as xterm has them.
fn indexed(index: u8) -> Rgb {
    match index {
        0..16 => CAMPBELL[usize::from(index)],
        16..232 => {
            let step = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
            let n = index - 16;
            [step(n / 36), step(n / 6 % 6), step(n % 6)]
        }
        _ => {
            let gray = 8 + (index - 232) * 10;
            [gray; 3]
        }
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
