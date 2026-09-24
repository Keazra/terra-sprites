use serde::{Deserialize, Serialize, Serializer};

use crate::data::DataPack;
use crate::terrain::Terrain;

/// A tile's position: `x` grows to the east, `y` to the south.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub x: u16,
    pub y: u16,
}

/// One of the eight step directions. North is up the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dir {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl Dir {
    /// Every direction, clockwise from north.
    pub const ALL: [Dir; 8] = [
        Dir::N,
        Dir::NE,
        Dir::E,
        Dir::SE,
        Dir::S,
        Dir::SW,
        Dir::W,
        Dir::NW,
    ];

    /// The change in `(x, y)` a step in this direction makes.
    fn offset(self) -> (i32, i32) {
        match self {
            Dir::N => (0, -1),
            Dir::NE => (1, -1),
            Dir::E => (1, 0),
            Dir::SE => (1, 1),
            Dir::S => (0, 1),
            Dir::SW => (-1, 1),
            Dir::W => (-1, 0),
            Dir::NW => (-1, -1),
        }
    }

    fn is_diagonal(self) -> bool {
        matches!(self, Dir::NE | Dir::SE | Dir::SW | Dir::NW)
    }
}

/// Why a map could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// A side of the map is outside 1 to 1024 tiles.
    BadSize { width: usize, height: usize },
    /// A row of a drawing is not as long as the first row.
    RaggedRow { row: usize },
    /// A drawing uses a character that isn't in the legend.
    UnknownGlyph { glyph: char, pos: Pos },
    /// A world's map must form exactly one region.
    NotOneRegion { regions: usize },
}

/// The longest a side of any map can be, in tiles.
const MAX_SIDE: u16 = 1024;

/// The world's fixed-size grid of tiles, with the terrain movement rules (design §3.1).
#[derive(Debug, Clone, Serialize)]
pub struct Map {
    width: u16,
    height: u16,
    /// Row-major: the tile at `(x, y)` is at `y × width + x`.
    #[serde(serialize_with = "terrain_bytes")]
    tiles: Vec<Terrain>,
    /// Each terrain's step cost, indexed by `Terrain as usize`; `None` if unwalkable.
    step_costs: [Option<u16>; 6],
}

impl Map {
    /// Draws a map from rows of text, using the ascii theme's glyphs as the legend:
    /// `.` grass, `,` dirt, `:` sand, `~` shallow water, `=` deep water, `#` rock.
    /// Each side must be from 1 to 1024 tiles.
    pub fn from_ascii(rows: &[&str], data: &DataPack) -> Result<Map, MapError> {
        let width = rows.first().map_or(0, |row| row.chars().count());
        let height = rows.len();
        let side = 1..=usize::from(MAX_SIDE);
        if !side.contains(&width) || !side.contains(&height) {
            return Err(MapError::BadSize { width, height });
        }
        let mut tiles = Vec::with_capacity(width * height);
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return Err(MapError::RaggedRow { row: y });
            }
            for (x, glyph) in row.chars().enumerate() {
                let terrain = match glyph {
                    '.' => Terrain::Grass,
                    ',' => Terrain::Dirt,
                    ':' => Terrain::Sand,
                    '~' => Terrain::ShallowWater,
                    '=' => Terrain::DeepWater,
                    '#' => Terrain::Rock,
                    _ => {
                        let pos = Pos {
                            x: x as u16,
                            y: y as u16,
                        };
                        return Err(MapError::UnknownGlyph { glyph, pos });
                    }
                };
                tiles.push(terrain);
            }
        }
        Ok(Map {
            width: width as u16,
            height: height as u16,
            tiles,
            step_costs: Terrain::ALL.map(|terrain| data.terrain(terrain).step_cost()),
        })
    }

    /// A map of one terrain throughout.
    pub(crate) fn filled(width: u16, height: u16, terrain: Terrain, data: &DataPack) -> Map {
        Map {
            width,
            height,
            tiles: vec![terrain; usize::from(width) * usize::from(height)],
            step_costs: Terrain::ALL.map(|terrain| data.terrain(terrain).step_cost()),
        }
    }

    /// The map's width, in tiles.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// The map's height, in tiles.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// The terrain of the tile at `pos`, which must be on the map.
    pub fn terrain(&self, pos: Pos) -> Terrain {
        self.tiles[self.index(pos)]
    }

    /// What a step from `from` in direction `dir` costs, or `None` if the step isn't allowed.
    pub fn step_cost(&self, from: Pos, dir: Dir) -> Option<u32> {
        let to = self.neighbour(from, dir)?;
        let cost = u32::from(self.cost_onto(to)?);
        if !dir.is_diagonal() {
            return Some(cost);
        }
        // No corner-cutting: both tiles beside a diagonal step must be walkable.
        let beside = [Pos { x: to.x, y: from.y }, Pos { x: from.x, y: to.y }];
        beside
            .iter()
            .all(|&pos| self.cost_onto(pos).is_some())
            .then_some(cost * 14 / 10)
    }

    /// The step cost of the terrain at `pos`, or `None` if it isn't walkable.
    fn cost_onto(&self, pos: Pos) -> Option<u16> {
        self.step_costs[self.terrain(pos) as usize]
    }

    /// Changes the terrain of the tile at `pos`, which must be on the map.
    pub(crate) fn set(&mut self, pos: Pos, terrain: Terrain) {
        let index = self.index(pos);
        self.tiles[index] = terrain;
    }

    /// Whether sprites can stand on the tile at `pos`.
    pub(crate) fn is_walkable(&self, pos: Pos) -> bool {
        self.cost_onto(pos).is_some()
    }

    /// How many tiles the map has.
    pub(crate) fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Every position on the map, in tile-index order.
    pub(crate) fn positions(&self) -> impl Iterator<Item = Pos> + use<> {
        let (width, height) = (self.width, self.height);
        (0..height).flat_map(move |y| (0..width).map(move |x| Pos { x, y }))
    }

    /// The tile one step from `pos` in direction `dir`, or `None` past the wall.
    pub(crate) fn neighbour(&self, pos: Pos, dir: Dir) -> Option<Pos> {
        let (dx, dy) = dir.offset();
        let x = i32::from(pos.x) + dx;
        let y = i32::from(pos.y) + dy;
        let on_map =
            (0..i32::from(self.width)).contains(&x) && (0..i32::from(self.height)).contains(&y);
        on_map.then_some(Pos {
            x: x as u16,
            y: y as u16,
        })
    }

    /// The position of the tile at `index`.
    pub(crate) fn pos(&self, index: usize) -> Pos {
        let width = usize::from(self.width);
        Pos {
            x: (index % width) as u16,
            y: (index / width) as u16,
        }
    }

    /// The tile index of `pos`: `y × width + x`.
    pub(crate) fn index(&self, pos: Pos) -> usize {
        usize::from(pos.y) * usize::from(self.width) + usize::from(pos.x)
    }
}

/// Serializes terrain compactly, one byte per tile, for the state hash.
fn terrain_bytes<S: Serializer>(tiles: &[Terrain], serializer: S) -> Result<S::Ok, S::Error> {
    let bytes: Vec<u8> = tiles.iter().map(|&terrain| terrain as u8).collect();
    serializer.serialize_bytes(&bytes)
}
