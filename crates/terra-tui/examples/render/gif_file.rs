//! A clip as a GIF that loops (#241).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::time::Duration;

use gif::{DisposalMethod, Encoder, Repeat};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::clip::Clip;
use crate::glyphs::Glyphs;
use crate::look::{Look, Rgb, look};
use crate::picture::{self, mix};

/// The most steps of anti-aliasing between a character's colour and its
/// background: fewer if the clip's colours wouldn't fit in a GIF's 256.
const MOST_STEPS: u32 = 8;

/// The longest a GIF frame shows, in hundredths of a second.
const LONGEST_DELAY: u64 = u16::MAX as u64;

/// Writes `clip` to `out` as a GIF that loops forever, its characters
/// drawn by `glyphs`. After the first frame, each covers only what
/// changed, the rest left see-through, which keeps the file small.
pub fn write(clip: &Clip, glyphs: &mut Glyphs, out: impl Write) -> Result<(), String> {
    let first = clip.frames.first().ok_or("the clip has no frames")?;
    let (width, height) = picture::pixels(glyphs, first.screen.area.as_size());
    let palette = Palette::for_clip(clip)?;
    let failed = |err: gif::EncodingError| format!("couldn't write the GIF: {err}");
    let mut encoder =
        Encoder::new(out, width as u16, height as u16, &palette.bytes()).map_err(failed)?;
    encoder.set_repeat(Repeat::Infinite).map_err(failed)?;
    let mut writer = Writer {
        encoder,
        written: Duration::ZERO,
    };
    let mut shown: Option<Vec<u8>> = None;
    let mut pending: Option<(Change, Duration)> = None;
    for frame in &clip.frames {
        let image = palette.paint(glyphs, &frame.screen, width * height);
        let change = match &shown {
            None => Some(Change::whole(&image, width, height)),
            Some(before) => Change::between(before, &image, width, palette.see_through()),
        };
        match (change, &mut pending) {
            // Painted the same, as two screens may be: it shows for longer.
            (None, Some((_, shows_for))) => *shows_for += frame.shows_for,
            (change, _) => {
                if let Some((change, shows_for)) = pending.take() {
                    writer.frame(&change, shows_for).map_err(failed)?;
                }
                let change = change.unwrap_or_else(|| Change::nothing(palette.see_through()));
                pending = Some((change, frame.shows_for));
            }
        }
        shown = Some(image);
    }
    if let Some((change, shows_for)) = pending {
        writer.frame(&change, shows_for).map_err(failed)?;
    }
    Ok(())
}

/// The GIF being written, and how much of the clip it has shown.
struct Writer<W: Write> {
    encoder: Encoder<W>,
    written: Duration,
}

impl<W: Write> Writer<W> {
    /// Writes `change`, shown for `shows_for`. A GIF counts in hundredths of
    /// a second, so each frame's delay is rounded where it ends, and the
    /// clip doesn't drift; one longer than a frame can show goes on in
    /// frames that change nothing.
    fn frame(&mut self, change: &Change, shows_for: Duration) -> Result<(), gif::EncodingError> {
        let hundredths = |time: Duration| (time.as_millis() as u64 + 5) / 10;
        let start = hundredths(self.written);
        self.written += shows_for;
        let mut delay = hundredths(self.written) - start;
        let mut change = change.clone();
        loop {
            let now = delay.min(LONGEST_DELAY);
            let frame = gif::Frame {
                delay: now as u16,
                dispose: DisposalMethod::Keep,
                transparent: change.see_through,
                left: change.area.x,
                top: change.area.y,
                width: change.area.width,
                height: change.area.height,
                buffer: Cow::Borrowed(&change.indexes),
                ..gif::Frame::default()
            };
            self.encoder.write_frame(&frame)?;
            delay -= now;
            if delay == 0 {
                return Ok(());
            }
            change = Change::nothing(change.see_through.unwrap_or(0));
        }
    }
}

/// What a GIF frame paints over the one before: a rectangle of palette
/// indexes, where `see_through`'s leave what's under them.
#[derive(Debug, Clone)]
struct Change {
    /// Where it's painted, in pixels.
    area: Rect,
    indexes: Vec<u8>,
    see_through: Option<u8>,
}

impl Change {
    /// The whole of `image`.
    fn whole(image: &[u8], width: usize, height: usize) -> Change {
        Change {
            area: Rect::new(0, 0, width as u16, height as u16),
            indexes: image.to_vec(),
            see_through: None,
        }
    }

    /// A pixel that changes nothing.
    fn nothing(see_through: u8) -> Change {
        Change {
            area: Rect::new(0, 0, 1, 1),
            indexes: vec![see_through],
            see_through: Some(see_through),
        }
    }

    /// The smallest rectangle of `after` that covers what's changed since
    /// `before`, with what's the same see-through; or `None` if nothing has.
    fn between(before: &[u8], after: &[u8], width: usize, see_through: u8) -> Option<Change> {
        let (mut left, mut top, mut right, mut bottom) = (usize::MAX, usize::MAX, 0, 0);
        for (at, _) in before
            .iter()
            .zip(after)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
        {
            let (x, y) = (at % width, at / width);
            (left, top) = (left.min(x), top.min(y));
            (right, bottom) = (right.max(x), bottom.max(y));
        }
        if left == usize::MAX {
            return None;
        }
        let indexes = (top..=bottom)
            .flat_map(|y| (left..=right).map(move |x| y * width + x))
            .map(|at| {
                if before[at] == after[at] {
                    see_through
                } else {
                    after[at]
                }
            })
            .collect();
        Some(Change {
            area: Rect::new(
                left as u16,
                top as u16,
                (right - left + 1) as u16,
                (bottom - top + 1) as u16,
            ),
            indexes,
            see_through: Some(see_through),
        })
    }
}

/// A GIF's colours: each pair of colours a cell is drawn in, with `steps`
/// of anti-aliasing between them, and one index left over for
/// see-through.
struct Palette {
    colours: Vec<Rgb>,
    steps: u32,
    /// Each pair, foreground and background, with its colours' indexes
    /// from the background, step by step, to the foreground.
    pairs: BTreeMap<(Rgb, Rgb), Vec<u8>>,
}

impl Palette {
    /// The palette for `clip`'s colours, with as many steps as fit.
    fn for_clip(clip: &Clip) -> Result<Palette, String> {
        let pairs: BTreeSet<(Rgb, Rgb)> = clip
            .frames
            .iter()
            .flat_map(|frame| frame.screen.content().iter().map(look))
            .map(|look| (look.fg, look.bg))
            .collect();
        for steps in (1..=MOST_STEPS).rev() {
            let shades =
                |&(fg, bg): &(Rgb, Rgb)| (0..=steps).map(move |k| mix(bg, fg, level(k, steps)));
            let colours: BTreeSet<Rgb> = pairs.iter().flat_map(shades).collect();
            if colours.len() >= 256 {
                continue;
            }
            let colours: Vec<Rgb> = colours.into_iter().collect();
            let index = |colour: Rgb| colours.binary_search(&colour).expect("in the palette") as u8;
            let pairs = pairs
                .iter()
                .map(|pair| (*pair, shades(pair).map(index).collect()))
                .collect();
            return Ok(Palette {
                colours,
                steps,
                pairs,
            });
        }
        Err("the clip has more colours than a GIF can hold".into())
    }

    /// The index that leaves what's under it.
    fn see_through(&self) -> u8 {
        self.colours.len() as u8
    }

    /// The palette as a GIF holds it, red, green and blue a colour.
    fn bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = self.colours.iter().flatten().copied().collect();
        bytes.extend([0, 0, 0]);
        bytes
    }

    /// `screen` in palette indexes, `pixels` of them.
    fn paint(&self, glyphs: &mut Glyphs, screen: &Buffer, pixels: usize) -> Vec<u8> {
        let mut image = vec![0; pixels];
        let mut last: Option<(Look, &Vec<u8>)> = None;
        picture::each_pixel(glyphs, screen, |at, look, coverage| {
            let shades = match last {
                Some((before, shades)) if before == *look => shades,
                _ => {
                    let shades = &self.pairs[&(look.fg, look.bg)];
                    last = Some((*look, shades));
                    shades
                }
            };
            let step = (u32::from(coverage) * self.steps + 127) / 255;
            image[at] = shades[step as usize];
        });
        image
    }
}

/// Step `k` of `steps` from a background to a foreground, out of 255.
fn level(k: u32, steps: u32) -> u8 {
    ((k * 255 + steps / 2) / steps) as u8
}

#[cfg(test)]
mod tests {
    use gif::{ColorOutput, DecodeOptions, Repeat};
    use ratatui::style::Color;

    use super::*;
    use crate::clip::Frame;
    use crate::test_util::{self, glyphs};

    /// A frame of three 10×20 cells, `text` in red, for `millis`.
    fn frame(text: &str, millis: u64) -> Frame {
        test_util::frame(3, text, Color::Red, millis)
    }

    fn gif_of(clip: &Clip) -> Vec<u8> {
        let mut bytes = Vec::new();
        write(clip, &mut glyphs(), &mut bytes).unwrap();
        bytes
    }

    /// What's read back of each frame: its delay, where it's drawn and its
    /// pixels' palette indexes.
    struct Read {
        delay: u16,
        area: (u16, u16, u16, u16),
        indexes: Vec<u8>,
    }

    fn read(bytes: &[u8]) -> (Vec<u8>, Repeat, Vec<Read>) {
        let mut options = DecodeOptions::new();
        options.set_color_output(ColorOutput::Indexed);
        let mut decoder = options.read_info(bytes).unwrap();
        let mut frames = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            frames.push(Read {
                delay: frame.delay,
                area: (frame.left, frame.top, frame.width, frame.height),
                indexes: frame.buffer.to_vec(),
            });
        }
        let palette = decoder.global_palette().unwrap().to_vec();
        (palette, decoder.repeat(), frames)
    }

    #[test]
    fn each_frame_shows_for_its_time_and_after_the_first_only_what_changed() {
        // A block, then a second block beside it, then the first one gone.
        let clip = Clip {
            frames: vec![frame("█", 300), frame("██", 200), frame(" █", 1000)],
        };
        let (palette, repeat, frames) = read(&gif_of(&clip));
        assert_eq!(repeat, Repeat::Infinite);
        let delays: Vec<u16> = frames.iter().map(|frame| frame.delay).collect();
        assert_eq!(delays, [30, 20, 100], "in hundredths of a second");
        let areas: Vec<_> = frames.iter().map(|frame| frame.area).collect();
        assert_eq!(areas, [(0, 0, 30, 20), (10, 0, 10, 20), (0, 0, 10, 20)]);
        let colour = |index: u8| {
            let at = usize::from(index) * 3;
            [palette[at], palette[at + 1], palette[at + 2]]
        };
        let first = &frames[0].indexes;
        assert_eq!(colour(first[0]), [0xC5, 0x0F, 0x1F], "Campbell's red");
        assert_eq!(colour(first[29]), [0x0C, 0x0C, 0x0C], "its background");
        assert!(
            frames[1]
                .indexes
                .iter()
                .all(|&index| colour(index) == [0xC5, 0x0F, 0x1F])
        );
    }

    #[test]
    fn the_same_clip_makes_the_same_bytes() {
        let clip = Clip {
            frames: vec![frame("A☺─", 100), frame("B☻═", 100)],
        };
        assert_eq!(gif_of(&clip), gif_of(&clip));
    }
}
