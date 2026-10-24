//! What the renderer's tests share (#241).

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Style};

use crate::clip::Frame;
use crate::glyphs::{Fonts, Glyphs};

/// Cascadia Mono in cells of 10×20 pixels, the renderer's own.
pub fn glyphs() -> Glyphs {
    Glyphs::new(Fonts::cascadia(), Size::new(10, 20))
}

/// A frame of one row of `width` cells, `text` in `fg` from its left, that
/// shows for `millis` milliseconds.
pub fn frame(width: u16, text: &str, fg: Color, millis: u64) -> Frame {
    let mut screen = Buffer::empty(Rect::new(0, 0, width, 1));
    screen.set_string(0, 0, text, Style::default().fg(fg));
    Frame {
        screen,
        shows_for: Duration::from_millis(millis),
    }
}
