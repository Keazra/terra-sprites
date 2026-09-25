//! Which entity stands on each tile (design §3.4): an index an entity store
//! keeps for its own kind of entity, at most one per tile.

use crate::map::{Map, Pos};
use crate::objects::EntityId;

/// The entity on each tile of a map, for one kind of entity. It's derived from
/// where the store's entities stand, so stores don't hash it.
#[derive(Debug, Clone)]
pub(crate) struct Occupancy {
    /// By tile index, `y × width + x`.
    tiles: Vec<Option<EntityId>>,
    /// The map's width, to find a tile's index.
    width: u16,
}

impl Occupancy {
    /// Every tile of `map` empty.
    pub(crate) fn new(map: &Map) -> Occupancy {
        Occupancy {
            tiles: vec![None; map.tile_count()],
            width: map.width(),
        }
    }

    /// The entity on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.tiles[self.index(pos)]
    }

    /// Puts `id` on the tile at `pos`, which must be empty.
    pub(crate) fn put(&mut self, pos: Pos, id: EntityId) {
        let index = self.index(pos);
        debug_assert!(self.tiles[index].is_none(), "{pos:?} is already taken");
        self.tiles[index] = Some(id);
    }

    /// Empties the tile at `pos`.
    pub(crate) fn clear(&mut self, pos: Pos) {
        let index = self.index(pos);
        self.tiles[index] = None;
    }

    /// How many tiles hold an entity.
    pub(crate) fn count(&self) -> usize {
        self.tiles.iter().flatten().count()
    }

    fn index(&self, pos: Pos) -> usize {
        debug_assert!(
            pos.x < self.width,
            "{pos:?} is off a map {} wide",
            self.width
        );
        usize::from(pos.y) * usize::from(self.width) + usize::from(pos.x)
    }
}
