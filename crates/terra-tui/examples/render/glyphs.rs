//! The characters a picture draws, a cell's worth each (#241): from the
//! font, or, for box-drawing and block characters, drawn to fill the cell
//! as terminals draw them, so lines join from cell to cell.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ab_glyph::{Font, FontArc, GlyphId, OutlineCurve, Point, PxScale, point};
use ratatui::layout::Size;

use crate::boxes::{self, Shape};

/// Cascadia Mono, Windows Terminal's default font, which has every glyph
/// the game draws. It's under the SIL Open Font License, in `fonts/OFL.txt`.
const CASCADIA: &[u8] = include_bytes!("fonts/CascadiaMono-Regular.ttf");
/// And its bold face.
const CASCADIA_BOLD: &[u8] = include_bytes!("fonts/CascadiaMono-Bold.ttf");

/// The faces a picture is drawn in: regular, and bold for what the game
/// draws in bold.
#[derive(Clone)]
pub struct Fonts {
    regular: FontArc,
    bold: FontArc,
}

impl Fonts {
    /// Cascadia Mono, as Windows Terminal draws the game.
    pub fn cascadia() -> Fonts {
        let font = |bytes| FontArc::try_from_slice(bytes).expect("Cascadia Mono reads");
        Fonts {
            regular: font(CASCADIA),
            bold: font(CASCADIA_BOLD),
        }
    }

    /// The font in the file `regular`, with its bold face in `bold`, or
    /// itself for bold too.
    pub fn from_files(regular: &Path, bold: Option<&Path>) -> Result<Fonts, String> {
        let read = |path: &Path| {
            let bytes = std::fs::read(path)
                .map_err(|err| format!("can't read the font {}: {err}", path.display()))?;
            FontArc::try_from_vec(bytes)
                .map_err(|_| format!("{} isn't a font the renderer can read", path.display()))
        };
        let font = read(regular)?;
        let bold = match bold {
            Some(path) => read(path)?,
            None => font.clone(),
        };
        Ok(Fonts {
            regular: font,
            bold,
        })
    }
}

/// How much of each pixel of a cell a character covers, out of 255, a row
/// at a time from the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ink {
    pub width: u16,
    pub height: u16,
    pub coverage: Vec<u8>,
}

impl Ink {
    /// Nothing, in a cell `cell` pixels in size.
    fn blank(cell: Size) -> Ink {
        Ink {
            width: cell.width,
            height: cell.height,
            coverage: vec![0; usize::from(cell.width) * usize::from(cell.height)],
        }
    }

    /// How much of pixel (`x`, `y`) is covered.
    #[cfg(test)]
    pub fn at(&self, x: u16, y: u16) -> u8 {
        self.coverage[usize::from(y) * usize::from(self.width) + usize::from(x)]
    }

    fn cover(&mut self, x: u16, y: u16, amount: u8) {
        let at = usize::from(y) * usize::from(self.width) + usize::from(x);
        self.coverage[at] = self.coverage[at].max(amount);
    }

    /// `shape`, filled in.
    fn of_shape(shape: &Shape, cell: Size) -> Ink {
        let mut ink = Ink::blank(cell);
        for rect in &shape.rects {
            for position in rect.positions() {
                ink.cover(position.x, position.y, shape.alpha);
            }
        }
        ink
    }
}

/// A font as a cell holds it: scaled so its characters' advance fills the
/// cell's width, or its height the cell's, whichever is smaller, and set
/// in the middle.
struct Face {
    font: FontArc,
    /// Pixels a font unit.
    scale: f32,
    /// Where a character starts, from the cell's left edge.
    x: f32,
    /// Where its baseline is, from the cell's top.
    baseline: f32,
}

impl Face {
    fn new(font: FontArc, cell: Size) -> Face {
        let advance = font.h_advance_unscaled(font.glyph_id('M'));
        let height = font.height_unscaled();
        let (width, cell_height) = (f32::from(cell.width), f32::from(cell.height));
        let scale = (width / advance).min(cell_height / height);
        Face {
            x: (width - advance * scale) / 2.0,
            baseline: (cell_height - height * scale) / 2.0 + font.ascent_unscaled() * scale,
            scale,
            font,
        }
    }

    /// The glyph `id`, rasterised in a cell `cell` pixels in size, cut off
    /// at its edges.
    fn ink(&self, id: GlyphId, cell: Size) -> Ink {
        let mut ink = Ink::blank(cell);
        let size = PxScale::from(self.scale * self.font.height_unscaled());
        let glyph = id.with_scale_and_position(size, point(self.x, self.baseline));
        if let Some(outlined) = self.font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, amount| {
                let x = bounds.min.x as i64 + i64::from(x);
                let y = bounds.min.y as i64 + i64::from(y);
                if let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y))
                    && x < cell.width
                    && y < cell.height
                {
                    ink.cover(x, y, (amount.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
            });
        }
        ink
    }
}

/// The characters drawn in cells of one size, each drawn once.
pub struct Glyphs {
    regular: Face,
    bold: Face,
    cell: Size,
    inks: BTreeMap<(char, bool), Ink>,
    /// The characters the font has no glyph for, that were asked for.
    missing: BTreeSet<char>,
}

impl Glyphs {
    /// Characters in `fonts`, in cells `cell` pixels in size.
    pub fn new(fonts: Fonts, cell: Size) -> Glyphs {
        Glyphs {
            regular: Face::new(fonts.regular, cell),
            bold: Face::new(fonts.bold, cell),
            cell,
            inks: BTreeMap::new(),
            missing: BTreeSet::new(),
        }
    }

    /// A cell's size, in pixels.
    pub fn cell(&self) -> Size {
        self.cell
    }

    /// `symbol`'s ink, in the bold face or not.
    pub fn ink(&mut self, symbol: char, bold: bool) -> &Ink {
        let key = (symbol, bold);
        if !self.inks.contains_key(&key) {
            let ink = self.draw(symbol, bold);
            self.inks.insert(key, ink);
        }
        &self.inks[&key]
    }

    fn draw(&mut self, symbol: char, bold: bool) -> Ink {
        if let Some(shape) = boxes::shape(symbol, self.cell) {
            return Ink::of_shape(&shape, self.cell);
        }
        let cell = self.cell;
        let (face, id) = self.glyph(symbol, bold);
        face.ink(id, cell)
    }

    /// The face `symbol` is drawn in, bold or not, and its glyph there,
    /// noting it as missing if the font has none.
    fn glyph(&mut self, symbol: char, bold: bool) -> (&Face, GlyphId) {
        let face = if bold { &self.bold } else { &self.regular };
        let id = face.font.glyph_id(symbol);
        if id.0 == 0 && !symbol.is_whitespace() {
            self.missing.insert(symbol);
        }
        (if bold { &self.bold } else { &self.regular }, id)
    }

    /// The characters asked for that the font has no glyph for: drawn as
    /// its blank box.
    pub fn missing(&self) -> &BTreeSet<char> {
        &self.missing
    }

    /// `symbol`'s outline, in the bold face or not, for an SVG; or `None`
    /// if it has no ink.
    pub fn outline(&mut self, symbol: char, bold: bool) -> Option<Outline> {
        if let Some(shape) = boxes::shape(symbol, self.cell) {
            let d: String = shape
                .rects
                .iter()
                .filter(|rect| !rect.is_empty())
                .map(|r| format!("M{} {}h{}v{}h-{}z", r.x, r.y, r.width, r.height, r.width))
                .collect();
            return (!d.is_empty()).then_some(Outline {
                d,
                alpha: shape.alpha,
                overflows: false,
            });
        }
        let cell = self.cell;
        let (face, id) = self.glyph(symbol, bold);
        face.outline(id, cell)
    }
}

/// A character as an SVG path, a cell's worth, in pixels from the cell's
/// top-left corner.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Outline {
    /// The path's data, its `d`.
    pub d: String,
    /// How much it covers what's under it, out of 255.
    pub alpha: u8,
    /// Whether it reaches past the cell's edges, where a picture cuts it
    /// off.
    pub overflows: bool,
}

impl Face {
    /// The glyph `id`'s outline in a cell `cell` pixels in size, or `None`
    /// if it has none.
    fn outline(&self, id: GlyphId, cell: Size) -> Option<Outline> {
        let outline = self.font.outline(id)?;
        let at = |p: Point| {
            let x = decimal(self.x + p.x * self.scale);
            let y = decimal(self.baseline - p.y * self.scale);
            format!("{x} {y}")
        };
        let mut d = String::new();
        let mut pen = None;
        for curve in &outline.curves {
            let (start, end) = match *curve {
                OutlineCurve::Line(start, end)
                | OutlineCurve::Quad(start, _, end)
                | OutlineCurve::Cubic(start, _, _, end) => (start, end),
            };
            if pen != Some(start) {
                if pen.is_some() {
                    d.push('Z');
                }
                d += &format!("M{}", at(start));
            }
            d += &match *curve {
                OutlineCurve::Line(_, end) => format!("L{}", at(end)),
                OutlineCurve::Quad(_, control, end) => format!("Q{} {}", at(control), at(end)),
                OutlineCurve::Cubic(_, one, two, end) => {
                    format!("C{} {} {}", at(one), at(two), at(end))
                }
            };
            pen = Some(end);
        }
        if d.is_empty() {
            return None;
        }
        d.push('Z');
        let bounds = outline.bounds;
        let (width, height) = (f32::from(cell.width), f32::from(cell.height));
        let overflows = self.x + bounds.min.x * self.scale < 0.0
            || self.x + bounds.max.x * self.scale > width
            || self.baseline - bounds.max.y * self.scale < 0.0
            || self.baseline - bounds.min.y * self.scale > height;
        Some(Outline {
            d,
            alpha: u8::MAX,
            overflows,
        })
    }
}

/// `value` to a tenth, without a needless ".0": "1.5", "-2", "0".
fn decimal(value: f32) -> String {
    let tenths = (value * 10.0).round() as i64;
    match (tenths / 10, (tenths % 10).abs()) {
        (whole, 0) => whole.to_string(),
        (0, tenth) if tenths < 0 => format!("-0.{tenth}"),
        (whole, tenth) => format!("{whole}.{tenth}"),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Size;

    use super::*;
    use crate::test_util::glyphs;

    /// The rows with ink in column `x`.
    fn rows_inked(ink: &Ink, x: u16) -> Vec<u16> {
        (0..ink.height).filter(|&y| ink.at(x, y) > 0).collect()
    }

    /// The columns with ink in row `y`.
    fn columns_inked(ink: &Ink, y: u16) -> Vec<u16> {
        (0..ink.width).filter(|&x| ink.at(x, y) > 0).collect()
    }

    fn total(ink: &Ink) -> u32 {
        ink.coverage.iter().map(|&c| u32::from(c)).sum()
    }

    #[test]
    fn single_lines_join_their_neighbours_at_the_cell_s_edges() {
        let mut glyphs = glyphs();
        let across = glyphs.ink('─', false).clone();
        let corner = glyphs.ink('┐', false).clone();
        let down = glyphs.ink('│', false).clone();
        let (right, bottom) = (9, 19);
        assert_eq!(rows_inked(&across, 0).len(), 1, "a line a pixel thick");
        assert_eq!(rows_inked(&across, right), rows_inked(&corner, 0));
        assert_eq!(columns_inked(&corner, bottom), columns_inked(&down, 0));
        assert_eq!(columns_inked(&down, bottom), columns_inked(&down, 0));
        // ┐ goes left and down only.
        assert!(rows_inked(&corner, right).is_empty());
        assert!(columns_inked(&corner, 0).is_empty());
        let tee = glyphs.ink('┼', false).clone();
        assert_eq!(rows_inked(&tee, 0), rows_inked(&across, right));
        assert_eq!(columns_inked(&tee, 0), columns_inked(&down, bottom));
    }

    #[test]
    fn double_lines_join_their_neighbours_two_lines_each() {
        let mut glyphs = glyphs();
        let across = glyphs.ink('═', false).clone();
        let down = glyphs.ink('║', false).clone();
        let (right, bottom) = (9, 19);
        assert_eq!(rows_inked(&across, 0).len(), 2);
        assert_eq!(columns_inked(&down, 0).len(), 2);
        // ╔═╗
        // ║ ║
        // ╚═╝
        let [top_left, top_right, bottom_left, bottom_right] =
            ['╔', '╗', '╚', '╝'].map(|corner| glyphs.ink(corner, false).clone());
        assert_eq!(rows_inked(&top_left, right), rows_inked(&across, 0));
        assert_eq!(rows_inked(&top_right, 0), rows_inked(&across, right));
        assert_eq!(columns_inked(&top_left, bottom), columns_inked(&down, 0));
        assert_eq!(columns_inked(&top_right, bottom), columns_inked(&down, 0));
        assert_eq!(columns_inked(&bottom_left, 0), columns_inked(&down, bottom));
        assert_eq!(rows_inked(&bottom_left, right), rows_inked(&across, 0));
        assert_eq!(rows_inked(&bottom_right, 0), rows_inked(&across, right));
        assert_eq!(
            columns_inked(&bottom_right, 0),
            columns_inked(&down, bottom)
        );
        // Each corner leaves the edges it doesn't join at alone.
        assert!(columns_inked(&top_left, 0).is_empty());
        assert!(rows_inked(&bottom_right, right).is_empty());
    }

    #[test]
    fn lines_thicken_with_the_cell() {
        let mut big = Glyphs::new(Fonts::cascadia(), Size::new(20, 40));
        assert_eq!(rows_inked(big.ink('─', false), 0).len(), 2);
    }

    #[test]
    fn blocks_fill_their_part_of_the_cell_and_shades_tint_it() {
        let mut glyphs = glyphs();
        assert!(glyphs.ink('█', false).coverage.iter().all(|&c| c == 255));
        let upper = glyphs.ink('▀', false).clone();
        assert!((0..10).all(|y| columns_inked(&upper, y).len() == 10));
        assert!((10..20).all(|y| columns_inked(&upper, y).is_empty()));
        let lower = glyphs.ink('▄', false).clone();
        assert!((0..10).all(|y| columns_inked(&lower, y).is_empty()));
        assert!((10..20).all(|y| columns_inked(&lower, y).len() == 10));
        let light = glyphs.ink('░', false);
        assert!(light.coverage.iter().all(|&c| c == 64), "a quarter");
    }

    #[test]
    fn letters_come_from_the_font_and_bold_from_its_bold_face() {
        let mut glyphs = glyphs();
        let regular = glyphs.ink('A', false).clone();
        let bold = glyphs.ink('A', true).clone();
        assert!(total(&regular) > 0);
        assert!(regular.coverage.contains(&0), "a letter isn't a block");
        assert!(total(&bold) > total(&regular));
        assert_eq!(total(glyphs.ink(' ', false)), 0);
        assert!(total(glyphs.ink('☺', false)) > 0);
    }

    #[test]
    fn outlines_are_the_same_characters_as_paths() {
        let mut glyphs = glyphs();
        // ─'s two arms, a pixel thick, meeting in the middle.
        let line = glyphs.outline('─', false).unwrap();
        assert_eq!(line.d, "M0 9h5v1h-5zM4 9h6v1h-6z");
        assert_eq!(glyphs.outline('░', false).unwrap().alpha, 64);
        let a = glyphs.outline('A', false).unwrap();
        assert!(a.d.starts_with('M') && a.d.ends_with('Z'));
        assert!(!a.overflows, "Cascadia's A fits its cell");
        assert_ne!(glyphs.outline('A', true), Some(a));
        assert_eq!(glyphs.outline(' ', false), None);
        assert_eq!(decimal(-0.46), "-0.5");
        assert_eq!(decimal(12.04), "12");
    }

    #[test]
    fn a_character_the_font_lacks_is_noted() {
        let mut glyphs = glyphs();
        glyphs.ink('☺', false);
        glyphs.ink('─', false);
        glyphs.ink('漢', false);
        assert_eq!(glyphs.missing().iter().collect::<Vec<_>>(), [&'漢']);
    }
}
