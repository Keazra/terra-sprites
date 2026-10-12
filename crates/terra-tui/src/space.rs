//! What the map view shows past the wall (M2 design §11): the glass one
//! tile outside the map, and space beyond it, with stars going by. It
//! belongs to the screen, like the theme: the stars come from a pattern of
//! the front end's own, not the world's dice, so they're never part of the
//! world, its saves, its replays or its state hash.

use std::time::Duration;

use ratatui::layout::{Position, Size};
use serde::Deserialize;
use terra_sim::Pos;

use crate::app::Origin;

/// What the map view shows at a place counted in tiles from the map's
/// top-left corner, which may be past the wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// A tile of the map.
    Tile(Pos),
    /// The glass, one tile outside the map, all round (§11.2). It isn't a
    /// tile: nothing on the map reaches it.
    Glass(Pane),
    /// Space, past the glass (§11.3).
    Space,
}

impl Place {
    /// What's at (`x`, `y`), on or around a map of `map` tiles.
    pub fn at(x: i32, y: i32, map: Size) -> Place {
        let (width, height) = (i32::from(map.width), i32::from(map.height));
        if (0..width).contains(&x) && (0..height).contains(&y) {
            return Place::Tile(Pos {
                x: x as u16,
                y: y as u16,
            });
        }
        let (left, top) = (x == -1, y == -1);
        let side = (left || x == width) && (-1..=height).contains(&y);
        let end = (top || y == height) && (-1..=width).contains(&x);
        Place::Glass(match (side, end) {
            (true, true) => match (left, top) {
                (true, true) => Pane::TopLeft,
                (false, true) => Pane::TopRight,
                (true, false) => Pane::BottomLeft,
                (false, false) => Pane::BottomRight,
            },
            (true, false) => Pane::Vertical,
            (false, true) => Pane::Horizontal,
            (false, false) => return Place::Space,
        })
    }
}

/// A piece of the glass, each with a glyph of the theme's (§11.5): `═`
/// along the top and bottom, `║` down the sides, and the four corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pane {
    /// Along the top or the bottom.
    Horizontal,
    /// Down the left or the right.
    Vertical,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Pane {
    /// Every piece of the glass.
    pub const ALL: [Pane; 6] = [
        Pane::Horizontal,
        Pane::Vertical,
        Pane::TopLeft,
        Pane::TopRight,
        Pane::BottomLeft,
        Pane::BottomRight,
    ];
}

/// A depth of stars (§11.3). The numbers are those of the preview the owner
/// tried in the ship's design pass, round 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    Far,
    Near,
    Nearest,
}

impl Depth {
    /// Far to nearest: the order they're drawn in, so a nearer star hides a
    /// farther one on the same cell.
    pub const ALL: [Depth; 3] = [Depth::Far, Depth::Near, Depth::Nearest];

    /// The share of space's tiles with a star of this depth.
    fn share(self) -> f64 {
        match self {
            Depth::Far => 0.05,
            Depth::Near => 0.022,
            Depth::Nearest => 0.009,
        }
    }

    /// How far its stars drift right to left, in tiles a second at 1×: the
    /// ship flies towards the right of the screen.
    fn speed(self) -> f64 {
        match self {
            Depth::Far => 0.55,
            Depth::Near => 1.3,
            Depth::Nearest => 3.2,
        }
    }

    /// How far its stars shift against the view as it scrolls, as a share
    /// of the scroll: this sideways, and half of it up and down, which gives
    /// depth.
    fn shift(self) -> f64 {
        match self {
            Depth::Far => 0.08,
            Depth::Near => 0.2,
            Depth::Nearest => 0.45,
        }
    }
}

/// Where the stars are in a map view of `size` cells whose viewport starts
/// at `origin`, once they've drifted for `drift`, as time at 1×: by depth,
/// far ones first, in cells from the map view's top-left. They're fixed to
/// the map view and wrap round it. Some fall on the map or its glass, where
/// they aren't drawn.
pub fn stars(size: Size, origin: Origin, drift: Duration) -> Vec<(Depth, Position)> {
    if size.width == 0 || size.height == 0 {
        return Vec::new();
    }
    let (width, height) = (f64::from(size.width), f64::from(size.height));
    let drifted = drift.as_secs_f64();
    let mut stars = Vec::new();
    for (n, depth) in (0..).zip(Depth::ALL) {
        // Each depth draws on a pattern of its own, so a resize that
        // changes how many far stars there are leaves the nearer ones be.
        let mut pattern = Pattern(PATTERN + n);
        let count = (width * height * depth.share()).round() as usize;
        let back = drifted * depth.speed() + f64::from(origin.x) * depth.shift();
        let up = f64::from(origin.y) * depth.shift() / 2.0;
        for _ in 0..count {
            // Where it is as a share of the view, so a resize stretches the
            // pattern rather than shuffling it.
            let (x, y) = (pattern.share() * width, pattern.share() * height);
            let cell = Position::new(wrapped(x - back, size.width), wrapped(y - up, size.height));
            stars.push((depth, cell));
        }
    }
    stars
}

/// The cell `at` falls on, along a row or column `length` cells long,
/// wrapping round it.
fn wrapped(at: f64, length: u16) -> u16 {
    (at.round() as i64).rem_euclid(i64::from(length)) as u16
}

/// Where the stars' pattern starts: any number would do.
const PATTERN: u64 = 180;

/// The stars' pattern: SplitMix64, the same every time, and the front end's
/// own, never the world's dice.
struct Pattern(u64);

impl Pattern {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A share from 0 up to, but not including, 1.
    fn share(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_glass_is_one_tile_outside_the_map_all_round() {
        let map = Size::new(4, 3);
        let row = |y| -> String {
            (-2..6)
                .map(|x| match Place::at(x, y, map) {
                    Place::Tile(_) => '.',
                    Place::Glass(Pane::Horizontal) => '═',
                    Place::Glass(Pane::Vertical) => '║',
                    Place::Glass(Pane::TopLeft) => '╔',
                    Place::Glass(Pane::TopRight) => '╗',
                    Place::Glass(Pane::BottomLeft) => '╚',
                    Place::Glass(Pane::BottomRight) => '╝',
                    Place::Space => ' ',
                })
                .collect()
        };
        let rows: Vec<String> = (-2..5).map(row).collect();
        assert_eq!(
            rows,
            [
                "        ", //
                " ╔════╗ ", //
                " ║....║ ", //
                " ║....║ ", //
                " ║....║ ", //
                " ╚════╝ ", //
                "        ", //
            ]
        );
        assert_eq!(Place::at(3, 2, map), Place::Tile(Pos { x: 3, y: 2 }));
    }

    #[test]
    fn each_depth_takes_its_share_of_the_view() {
        let stars = stars(Size::new(100, 40), Origin { x: 0, y: 0 }, Duration::ZERO);
        let count = |depth| stars.iter().filter(|(d, _)| *d == depth).count();
        assert_eq!(
            [count(Depth::Far), count(Depth::Near), count(Depth::Nearest)],
            [200, 88, 36]
        );
    }

    #[test]
    fn stars_drift_right_to_left_the_nearer_the_quicker() {
        let size = Size::new(200, 50);
        let origin = Origin { x: 0, y: 0 };
        let before = stars(size, origin, Duration::ZERO);
        let after = stars(size, origin, Duration::from_secs(10));
        // In ten seconds at 1×, far stars drift 5.5 tiles, near 13 and the
        // nearest 32: each star is where it was, that far to the left,
        // give or take the rounding.
        for ((depth, was), (_, now)) in before.iter().zip(&after) {
            let moved = (i32::from(was.x) - i32::from(now.x)).rem_euclid(200);
            let expected = match depth {
                Depth::Far => 5..=6,
                Depth::Near => 13..=13,
                Depth::Nearest => 32..=32,
            };
            assert!(expected.contains(&moved), "{depth:?} moved {moved}");
            assert_eq!(was.y, now.y, "sideways only");
        }
    }

    #[test]
    fn scrolling_shifts_the_stars_a_little_the_far_ones_least() {
        let size = Size::new(200, 50);
        let before = stars(size, Origin { x: 0, y: 0 }, Duration::ZERO);
        let after = stars(size, Origin { x: 20, y: 20 }, Duration::ZERO);
        // Scrolled 20 tiles each way: far stars shift 1.6 tiles sideways
        // and 0.8 up, near 4 and 2, the nearest 9 and 4.5.
        for ((depth, was), (_, now)) in before.iter().zip(&after) {
            let across = (i32::from(was.x) - i32::from(now.x)).rem_euclid(200);
            let up = (i32::from(was.y) - i32::from(now.y)).rem_euclid(50);
            let (sideways, upward) = match depth {
                Depth::Far => (1..=2, 0..=1),
                Depth::Near => (4..=4, 2..=2),
                Depth::Nearest => (9..=9, 4..=5),
            };
            assert!(sideways.contains(&across), "{depth:?} {across}");
            assert!(upward.contains(&up), "{depth:?} {up}");
        }
    }
}
