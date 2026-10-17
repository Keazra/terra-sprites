//! What the renderer's tests share (#241).

use ratatui::layout::Size;

use crate::glyphs::{Fonts, Glyphs};

/// Cascadia Mono in cells of 10×20 pixels, the renderer's own.
pub fn glyphs() -> Glyphs {
    Glyphs::new(Fonts::cascadia(), Size::new(10, 20))
}
