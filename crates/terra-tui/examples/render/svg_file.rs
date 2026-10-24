//! A clip as an animated SVG that loops (#241): small enough for a
//! README, and sharp at any size.
//!
//! Each character the clip draws is one path, used in every cell it's in.
//! The first frame is drawn whole. Each later frame is a group of only the
//! cells that changed, hidden until its time comes round by a CSS
//! animation, so the frames build up over the loop as the screen did.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;
use std::time::Duration;

use ratatui::layout::Size;

use crate::clip::Clip;
use crate::glyphs::{Glyphs, Outline};
use crate::look::{BACKGROUND, Look, Rgb, look};
use crate::picture;

/// The id of the cell's outline that cuts off what a character draws past
/// it. Characters' ids are letters only, and frames' "f" and a number.
const CELL_CLIP: &str = "c0";

/// Writes `clip` to `out` as an SVG that loops forever, its characters
/// drawn by `glyphs`.
pub fn write(clip: &Clip, glyphs: &mut Glyphs, mut out: impl Write) -> Result<(), String> {
    let first = clip.frames.first().ok_or("the clip has no frames")?;
    let screen = first.screen.area.as_size();
    // Each screen's looks, from when it starts showing; a screen that
    // looks the same as the one before is part of it.
    let mut screens: Vec<(Duration, Vec<Look>)> = Vec::new();
    let mut start = Duration::ZERO;
    for frame in &clip.frames {
        let looks: Vec<Look> = frame.screen.content().iter().map(look).collect();
        if screens.last().is_none_or(|(_, before)| *before != looks) {
            screens.push((start, looks));
        }
        start += frame.shows_for;
    }
    let length = start;
    let mut drawing = Drawing {
        cell: glyphs.cell(),
        columns: usize::from(screen.width),
        glyphs,
        colours: BTreeMap::new(),
        characters: BTreeMap::new(),
        outlines: BTreeMap::new(),
    };
    let mut body = String::new();
    let mut animations = String::new();
    let mut shown: Option<&[Look]> = None;
    for (n, (start, looks)) in screens.iter().enumerate() {
        match shown {
            None => {
                let (width, height) = picture::pixels(drawing.glyphs, screen);
                let class = drawing.colour(BACKGROUND);
                let _ = writeln!(
                    body,
                    r#"<rect width="{width}" height="{height}" class="{class}"/>"#
                );
                body += &drawing.cells(looks, |_| true, Some(BACKGROUND));
            }
            Some(before) => {
                let changed = |at: usize| before[at] != looks[at];
                let _ = writeln!(body, r#"<g id="f{n}">"#);
                body += &drawing.cells(looks, changed, None);
                body += "</g>\n";
                let _ = writeln!(
                    animations,
                    "@keyframes k{n}{{0%{{opacity:0}}{}%{{opacity:1}}}}\n#f{n}{{animation-name:k{n}}}",
                    percent(*start, length)
                );
            }
        }
        shown = Some(looks);
    }
    let svg = drawing.svg(screen, length, &animations, &body);
    out.write_all(svg.as_bytes())
        .map_err(|err| format!("couldn't write the SVG: {err}"))
}

/// What's drawn so far: the colours and characters it's in, by name.
struct Drawing<'g> {
    glyphs: &'g mut Glyphs,
    cell: Size,
    /// Cells a row of the screen.
    columns: usize,
    /// Each colour's class.
    colours: BTreeMap<Rgb, String>,
    /// Each character's outline's id, in the bold face or not; `None` for
    /// one with no ink.
    characters: BTreeMap<(char, bool), Option<String>>,
    /// Each outline's id: characters drawn the same share one.
    outlines: BTreeMap<Outline, String>,
}

impl Drawing<'_> {
    /// The class that fills in `colour`.
    fn colour(&mut self, colour: Rgb) -> String {
        let next = self.colours.len();
        self.colours
            .entry(colour)
            .or_insert_with(|| name(next))
            .clone()
    }

    /// The id of `look`'s character's outline, or `None` if it has no ink.
    fn character(&mut self, look: &Look) -> Option<String> {
        let key = (look.symbol, look.bold);
        if let Some(id) = self.characters.get(&key) {
            return id.clone();
        }
        let id = self.glyphs.outline(look.symbol, look.bold).map(|outline| {
            let next = self.outlines.len();
            self.outlines
                .entry(outline)
                .or_insert_with(|| name(next))
                .clone()
        });
        self.characters.insert(key, id.clone());
        id
    }

    /// The cells of `looks` that are `drawn`: their backgrounds, but for
    /// those already in the colour `under`, then their characters. Each run
    /// of cells in a row in one background is one rectangle.
    fn cells(
        &mut self,
        looks: &[Look],
        drawn: impl Fn(usize) -> bool,
        under: Option<Rgb>,
    ) -> String {
        let (width, height) = (usize::from(self.cell.width), usize::from(self.cell.height));
        let columns = self.columns;
        let mut rects: BTreeMap<String, String> = BTreeMap::new();
        let mut uses: BTreeMap<String, String> = BTreeMap::new();
        for (row, cells) in looks.chunks(columns).enumerate() {
            let at = |column: usize| row * columns + column;
            let place = |column: usize| at_place(column * width, row * height);
            let mut column = 0;
            while column < cells.len() {
                let background = cells[column].bg;
                if !drawn(at(column)) || Some(background) == under {
                    column += 1;
                    continue;
                }
                let run = cells[column..]
                    .iter()
                    .enumerate()
                    .take_while(|&(more, cell)| drawn(at(column + more)) && cell.bg == background)
                    .count();
                let rect = format!(
                    r#"<rect{} width="{}" height="{height}"/>"#,
                    place(column),
                    run * width
                );
                *rects.entry(self.colour(background)).or_default() += &rect;
                column += run;
            }
            for (column, cell) in cells.iter().enumerate() {
                if drawn(at(column))
                    && let Some(id) = self.character(cell)
                {
                    let used = format!(r##"<use href="#{id}"{}/>"##, place(column));
                    *uses.entry(self.colour(cell.fg)).or_default() += &used;
                }
            }
        }
        let mut cells = String::new();
        for (class, drawn) in rects.iter().chain(&uses) {
            let _ = writeln!(cells, r#"<g class="{class}">{drawn}</g>"#);
        }
        cells
    }

    /// The whole SVG, of a screen `screen` cells in size, its frames'
    /// `animations` over `length` and its `body`.
    fn svg(&self, screen: Size, length: Duration, animations: &str, body: &str) -> String {
        let (width, height) = picture::pixels(self.glyphs, screen);
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n<style>\n"
        );
        if !animations.is_empty() {
            // The frames' groups, all that have an id, hidden until
            // their keyframe.
            let _ = writeln!(
                svg,
                "g[id]{{opacity:0;animation-duration:{};animation-timing-function:step-end;animation-iteration-count:infinite}}",
                seconds(length)
            );
        }
        svg += "rect{shape-rendering:crispEdges}\n";
        for (&[r, g, b], class) in &self.colours {
            let _ = writeln!(svg, ".{class}{{fill:#{r:02X}{g:02X}{b:02X}}}");
        }
        svg += animations;
        svg += "</style>\n<defs>\n";
        if self.outlines.keys().any(|outline| outline.overflows) {
            let _ = writeln!(
                svg,
                r#"<clipPath id="{CELL_CLIP}"><rect width="{}" height="{}"/></clipPath>"#,
                self.cell.width, self.cell.height
            );
        }
        let mut outlines: Vec<(&String, &Outline)> = self
            .outlines
            .iter()
            .map(|(outline, id)| (id, outline))
            .collect();
        outlines.sort();
        for (id, outline) in outlines {
            let _ = write!(svg, r#"<path id="{id}""#);
            if outline.alpha < u8::MAX {
                let _ = write!(svg, r#" fill-opacity="{}""#, fraction(outline.alpha));
            }
            if outline.overflows {
                let _ = write!(svg, r##" clip-path="url(#{CELL_CLIP})""##);
            }
            let _ = writeln!(svg, r#" d="{}"/>"#, outline.d);
        }
        svg += "</defs>\n";
        svg += body;
        svg += "</svg>\n";
        svg
    }
}

/// The `n`th short name, in letters alone: "a" to "z", "A" to "Z", "ba"...
fn name(mut n: usize) -> String {
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut name = vec![LETTERS[n % LETTERS.len()]];
    n /= LETTERS.len();
    while n > 0 {
        name.push(LETTERS[n % LETTERS.len()]);
        n /= LETTERS.len();
    }
    name.reverse();
    String::from_utf8(name).expect("letters")
}

/// The attributes that put something at (`x`, `y`), leaving out a 0.
fn at_place(x: usize, y: usize) -> String {
    let mut place = String::new();
    if x > 0 {
        let _ = write!(place, r#" x="{x}""#);
    }
    if y > 0 {
        let _ = write!(place, r#" y="{y}""#);
    }
    place
}

/// How far `time` is through `length`, in percent to a thousandth, without
/// needless zeros: "30", "33.333".
fn percent(time: Duration, length: Duration) -> String {
    let (time, length) = (time.as_millis(), length.as_millis().max(1));
    let thousandths = (time * 100_000 + length / 2) / length;
    trimmed(thousandths / 1000, thousandths % 1000)
}

/// `time` in seconds, for CSS: "1s", "12.34s".
fn seconds(time: Duration) -> String {
    let millis = time.as_millis();
    format!("{}s", trimmed(millis / 1000, millis % 1000))
}

/// `alpha` out of 255, as a fraction: "0.251".
fn fraction(alpha: u8) -> String {
    let thousandths = (u32::from(alpha) * 1000 + 127) / 255;
    trimmed(0, u128::from(thousandths))
}

/// `whole` and `thousandths` of it, without needless zeros.
fn trimmed(whole: u128, thousandths: u128) -> String {
    if thousandths == 0 {
        return whole.to_string();
    }
    let fraction = format!("{thousandths:03}");
    format!("{whole}.{}", fraction.trim_end_matches('0'))
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crate::clip::Frame;
    use crate::test_util::{self, glyphs};

    /// A frame of four 10×20 cells, `text` in green, for `millis`.
    fn frame(text: &str, millis: u64) -> Frame {
        test_util::frame(4, text, Color::Green, millis)
    }

    fn svg_of(clip: &Clip) -> String {
        let mut glyphs = glyphs();
        let mut bytes = Vec::new();
        write(clip, &mut glyphs, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    /// The part of `svg` from the group of frame `n`, from 1, to the next.
    fn group(svg: &str, n: usize) -> &str {
        let start = svg
            .find(&format!("<g id=\"f{n}\""))
            .expect("the frame's group");
        let end = svg[start + 1..]
            .find("<g id=\"f")
            .map_or(svg.len(), |end| start + 1 + end);
        &svg[start..end]
    }

    #[test]
    fn each_character_is_drawn_once_and_used_in_every_cell_it_s_in() {
        let svg = svg_of(&Clip {
            frames: vec![frame("A═A ", 100)],
        });
        assert!(
            svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"20\"")
        );
        assert!(svg.trim_end().ends_with("</svg>"));
        assert_eq!(svg.matches("<path id=").count(), 2, "A and ═");
        assert_eq!(svg.matches("<use ").count(), 3);
        assert!(svg.contains("fill:#13A10E"), "Campbell's green");
    }

    #[test]
    fn later_frames_show_at_their_time_and_hold_only_what_changed() {
        // 0.3 s of "AB", 0.2 s with the B gone, then 0.5 s with a C too.
        let svg = svg_of(&Clip {
            frames: vec![frame("AB", 300), frame("A", 200), frame("A  C", 500)],
        });
        assert!(svg.contains("animation-duration:1s"), "a second in all");
        assert!(svg.contains("@keyframes k1{0%{opacity:0}30%{opacity:1}}"));
        assert!(svg.contains("@keyframes k2{0%{opacity:0}50%{opacity:1}}"));
        // The B's cell blanked, and the C's drawn.
        assert_eq!(group(&svg, 1).matches("<rect").count(), 1);
        assert_eq!(group(&svg, 1).matches("<use").count(), 0);
        assert_eq!(group(&svg, 2).matches("<rect").count(), 1);
        assert_eq!(group(&svg, 2).matches("<use").count(), 1);
    }

    #[test]
    fn the_same_clip_makes_the_same_bytes() {
        let clip = Clip {
            frames: vec![frame("A☺─", 100), frame("B☻═", 100)],
        };
        assert_eq!(svg_of(&clip), svg_of(&clip));
    }
}
