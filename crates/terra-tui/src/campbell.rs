//! Windows Terminal's default colour scheme, Campbell: how a named colour
//! is drawn where the game needs it as red, green and blue (design §6.9).
//! Motion blends a theme's named colours from it, and the renderer draws
//! its pictures in it (#241).

use ratatui::style::Color;

/// A colour, as red, green and blue.
pub type Rgb = [u8; 3];

/// The terminal's own text colour, where the game leaves it.
pub const FOREGROUND: Rgb = [0xCC, 0xCC, 0xCC];
/// The terminal's own background.
pub const BACKGROUND: Rgb = [0x0C, 0x0C, 0x0C];

/// `colour` in Campbell, or `None` for the terminal's own (`Reset`).
pub fn rgb(colour: Color) -> Option<Rgb> {
    Some(match colour {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(index) => indexed(index),
        named => COLOURS[named_index(named)],
    })
}

/// `colour` as an exact colour, which a fade can blend: a named one as
/// Campbell draws it, and the terminal's own as `own` (design §6.9).
pub fn exact(colour: Color, own: Rgb) -> Color {
    let [r, g, b] = rgb(colour).unwrap_or(own);
    Color::Rgb(r, g, b)
}

/// The 16 colours, in the order of their indexes: black, red, green,
/// yellow, blue, magenta, cyan and white (ratatui's gray), then their
/// bright forms (dark gray to white).
const COLOURS: [Rgb; 16] = [
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
        0..16 => COLOURS[usize::from(index)],
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
    use super::*;

    #[test]
    fn a_colour_is_campbell_s_and_the_terminal_s_own_is_none() {
        assert_eq!(rgb(Color::Blue), Some([0x00, 0x37, 0xDA]));
        assert_eq!(rgb(Color::White), Some([0xF2, 0xF2, 0xF2]));
        assert_eq!(rgb(Color::Rgb(1, 2, 3)), Some([1, 2, 3]));
        assert_eq!(rgb(Color::Indexed(110)), Some([0x87, 0xAF, 0xD7]));
        assert_eq!(rgb(Color::Indexed(232)), Some([8, 8, 8]));
        assert_eq!(rgb(Color::Reset), None);
    }

    #[test]
    fn an_exact_colour_takes_the_given_one_for_the_terminal_s_own() {
        assert_eq!(exact(Color::Cyan, BACKGROUND), Color::Rgb(0x3A, 0x96, 0xDD));
        assert_eq!(
            exact(Color::Reset, FOREGROUND),
            Color::Rgb(0xCC, 0xCC, 0xCC)
        );
        assert_eq!(exact(Color::Reset, [9, 9, 9]), Color::Rgb(9, 9, 9));
    }
}
