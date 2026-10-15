//! Box-drawing and block characters, drawn to fill the cell as terminals
//! draw them, so lines join from cell to cell (#241): a font's own leave
//! gaps between cells.

use ratatui::layout::{Rect, Size};

/// A character drawn as rectangles of one strength, in pixels from the
/// cell's top-left corner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub rects: Vec<Rect>,
    /// How much each covers what's under it, out of 255: all of it but for
    /// the shades.
    pub alpha: u8,
}

/// How `symbol` is drawn in a cell `cell` pixels in size, if it's one of
/// the box-drawing or block characters drawn this way. Others come from
/// the font.
pub fn shape(symbol: char, cell: Size) -> Option<Shape> {
    let (w, h) = (i32::from(cell.width), i32::from(cell.height));
    // A line's thickness: a pixel in a 10×20 cell, as a terminal's, and
    // thicker in a bigger one.
    let t = ((h + 10) / 20).max(1);
    // Where a line runs through the middle of the cell.
    let (cx, cy) = ((w - t) / 2, (h - t) / 2);
    let rect = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let clamp = |v: i32, most: i32| v.clamp(0, most) as u16;
        let (x0, y0) = (clamp(x0, w), clamp(y0, h));
        Rect::new(x0, y0, clamp(x1, w) - x0, clamp(y1, h) - y0)
    };
    let solid = |rects: Vec<Rect>| Some(Shape { rects, alpha: 255 });
    let shaded = |alpha: u8| {
        Some(Shape {
            rects: vec![rect(0, 0, w, h)],
            alpha,
        })
    };
    if let Some([up, down, left, right]) = single_arms(symbol) {
        let arms = [
            (left, rect(0, cy, cx + t, cy + t)),
            (right, rect(cx, cy, w, cy + t)),
            (up, rect(cx, 0, cx + t, cy + t)),
            (down, rect(cx, cy, cx + t, h)),
        ];
        return solid(
            arms.into_iter()
                .filter(|&(arm, _)| arm)
                .map(|(_, r)| r)
                .collect(),
        );
    }
    // A double line's two strokes, a stroke's width apart: columns `p1` and
    // `p2`, or rows `q1` and `q2`. At a corner the outer stroke turns round
    // the inner one.
    let (p1, p2, q1, q2) = (cx - t, cx + t, cy - t, cy + t);
    let across = |y: i32, x0: i32, x1: i32| rect(x0, y, x1, y + t);
    let down = |x: i32, y0: i32, y1: i32| rect(x, y0, x + t, y1);
    match symbol {
        '═' => solid(vec![across(q1, 0, w), across(q2, 0, w)]),
        '║' => solid(vec![down(p1, 0, h), down(p2, 0, h)]),
        '╔' => solid(vec![
            across(q1, p1, w),
            down(p1, q1, h),
            across(q2, p2, w),
            down(p2, q2, h),
        ]),
        '╗' => solid(vec![
            across(q1, 0, p2 + t),
            down(p2, q1, h),
            across(q2, 0, p1 + t),
            down(p1, q2, h),
        ]),
        '╚' => solid(vec![
            across(q2, p1, w),
            down(p1, 0, q2 + t),
            across(q1, p2, w),
            down(p2, 0, q1 + t),
        ]),
        '╝' => solid(vec![
            across(q2, 0, p2 + t),
            down(p2, 0, q2 + t),
            across(q1, 0, p1 + t),
            down(p1, 0, q1 + t),
        ]),
        '█' => solid(vec![rect(0, 0, w, h)]),
        '▀' => solid(vec![rect(0, 0, w, h / 2)]),
        '▄' => solid(vec![rect(0, h / 2, w, h)]),
        '▌' => solid(vec![rect(0, 0, w / 2, h)]),
        '▐' => solid(vec![rect(w / 2, 0, w, h)]),
        '░' => shaded(64),
        '▒' => shaded(128),
        '▓' => shaded(191),
        _ => None,
    }
}

/// Which way a single-lined box-drawing character's lines go from the
/// middle: up, down, left and right.
fn single_arms(symbol: char) -> Option<[bool; 4]> {
    Some(match symbol {
        '─' => [false, false, true, true],
        '│' => [true, true, false, false],
        '┌' => [false, true, false, true],
        '┐' => [false, true, true, false],
        '└' => [true, false, false, true],
        '┘' => [true, false, true, false],
        '├' => [true, true, false, true],
        '┤' => [true, true, true, false],
        '┬' => [false, true, true, true],
        '┴' => [true, false, true, true],
        '┼' => [true, true, true, true],
        _ => return None,
    })
}
