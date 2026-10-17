//! A screen painted into pixels (#241): each cell's character over its
//! background, in its colours.

use ratatui::buffer::Buffer;
use ratatui::layout::Size;

use crate::glyphs::Glyphs;
use crate::look::{Look, Rgb, look};

/// A picture's size in pixels, of a screen of `screen` cells.
pub fn pixels(glyphs: &Glyphs, screen: Size) -> (usize, usize) {
    let cell = glyphs.cell();
    (
        usize::from(screen.width) * usize::from(cell.width),
        usize::from(screen.height) * usize::from(cell.height),
    )
}

/// Goes over every pixel of `screen`, a cell at a time, with the pixel's
/// place in the picture, counted a row at a time from the top left, the
/// cell's look, and how much of the pixel its character covers, out of
/// 255.
pub fn each_pixel(glyphs: &mut Glyphs, screen: &Buffer, mut paint: impl FnMut(usize, &Look, u8)) {
    let cell = glyphs.cell();
    let (width, _) = pixels(glyphs, screen.area.as_size());
    let (cell_width, cell_height) = (usize::from(cell.width), usize::from(cell.height));
    for position in screen.area.positions() {
        let look = look(&screen[position]);
        let ink = glyphs.ink(look.symbol, look.bold);
        let left = usize::from(position.x - screen.area.x) * cell_width;
        let top = usize::from(position.y - screen.area.y) * cell_height;
        for y in 0..cell_height {
            let row = (top + y) * width + left;
            let inked = &ink.coverage[y * cell_width..(y + 1) * cell_width];
            for (x, &coverage) in inked.iter().enumerate() {
                paint(row + x, &look, coverage);
            }
        }
    }
}

/// `screen` in full colour: red, green and blue a pixel, a row at a time
/// from the top left.
pub fn rgb(glyphs: &mut Glyphs, screen: &Buffer) -> Vec<u8> {
    let (width, height) = pixels(glyphs, screen.area.as_size());
    let mut rgb = vec![0; width * height * 3];
    each_pixel(glyphs, screen, |at, look, coverage| {
        rgb[at * 3..at * 3 + 3].copy_from_slice(&mix(look.bg, look.fg, coverage));
    });
    rgb
}

/// `fg` over `bg`, covering `coverage` out of 255 of it.
pub fn mix(bg: Rgb, fg: Rgb, coverage: u8) -> Rgb {
    let (over, under) = (u32::from(coverage), 255 - u32::from(coverage));
    let channel =
        |i: usize| ((u32::from(fg[i]) * over + u32::from(bg[i]) * under + 127) / 255) as u8;
    [channel(0), channel(1), channel(2)]
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};

    use super::*;
    use crate::test_util::glyphs;

    /// The colour of pixel (`x`, `y`) of a picture `width` pixels wide.
    fn pixel(rgb: &[u8], width: usize, x: usize, y: usize) -> [u8; 3] {
        let at = (y * width + x) * 3;
        [rgb[at], rgb[at + 1], rgb[at + 2]]
    }

    #[test]
    fn each_cell_is_its_character_in_its_colours_over_its_background() {
        // A red block, a space on blue, and a green A: three 10×20 cells.
        let mut screen = Buffer::empty(Rect::new(0, 0, 3, 1));
        screen.set_string(0, 0, "█", Style::default().fg(Color::Red));
        screen.set_string(1, 0, " ", Style::default().bg(Color::Blue));
        screen.set_string(2, 0, "A", Style::default().fg(Color::Green));
        let mut glyphs = glyphs();
        let rgb = rgb(&mut glyphs, &screen);
        assert_eq!(rgb.len(), 30 * 20 * 3);
        let all = |x0: usize| (x0..x0 + 10).flat_map(|x| (0..20).map(move |y| (x, y)));
        assert!(all(0).all(|(x, y)| pixel(&rgb, 30, x, y) == [0xC5, 0x0F, 0x1F]));
        assert!(all(10).all(|(x, y)| pixel(&rgb, 30, x, y) == [0x00, 0x37, 0xDA]));
        let a: Vec<[u8; 3]> = all(20).map(|(x, y)| pixel(&rgb, 30, x, y)).collect();
        assert!(a.contains(&[0x13, 0xA1, 0x0E]), "the letter, in green");
        assert!(
            a.contains(&[0x0C, 0x0C, 0x0C]),
            "on the terminal's background"
        );
    }
}
