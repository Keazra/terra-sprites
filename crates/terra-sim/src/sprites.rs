//! The world's sprites and where they stand (design §2.3, §3.4, §4).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::biochem::{Body, Program};
use crate::data::DataPack;
use crate::genome::Genome;
use crate::map::{Map, Pos};
use crate::objects::{EntityId, Objects};

/// One sprite.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Sprite {
    pub(crate) pos: Pos,
    /// The tick it was born on.
    pub(crate) born: u64,
    pub(crate) genome: Genome,
    /// The genome compiled for the chemistry step. It's derived from the
    /// genome, so it isn't hashed.
    #[serde(skip)]
    pub(crate) program: Program,
    pub(crate) body: Body,
}

impl Sprite {
    /// A newborn with `genome` on `pos`, born on the tick `born` (design §4.7).
    pub(crate) fn newborn(genome: Genome, pos: Pos, born: u64, data: &DataPack) -> Sprite {
        let program = Program::new(&genome, data);
        let body = Body::newborn(&program, data);
        Sprite {
            pos,
            born,
            genome,
            program,
            body,
        }
    }
}

/// Every sprite in the world, and the tile each stands on. A tile holds at
/// most one sprite (design §3.4).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Sprites {
    /// In ascending ID order, the order every per-sprite pass takes (design §2.3).
    by_id: BTreeMap<EntityId, Sprite>,
    /// The sprite on each tile, by tile index. It's derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    on_tile: Vec<Option<EntityId>>,
    /// The map's width, to find a tile's index.
    #[serde(skip)]
    width: u16,
}

impl Sprites {
    /// No sprites, on `map`.
    pub(crate) fn new(map: &Map) -> Sprites {
        Sprites {
            by_id: BTreeMap::new(),
            on_tile: vec![None; map.tile_count()],
            width: map.width(),
        }
    }

    /// The sprite on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.on_tile[self.index(pos)]
    }

    /// Every sprite, in ascending ID order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (EntityId, &Sprite)> {
        self.by_id.iter().map(|(&id, sprite)| (id, sprite))
    }

    /// The sprite `id`, if it exists.
    pub(crate) fn get(&self, id: EntityId) -> Option<&Sprite> {
        self.by_id.get(&id)
    }

    /// The sprite `id`, if it exists, to change.
    pub(crate) fn get_mut(&mut self, id: EntityId) -> Option<&mut Sprite> {
        self.by_id.get_mut(&id)
    }

    /// Takes the sprite `id`, which must exist, out of the world.
    pub(crate) fn remove(&mut self, id: EntityId) -> Sprite {
        let sprite = self.by_id.remove(&id).expect("the sprite to remove");
        let index = self.index(sprite.pos);
        self.on_tile[index] = None;
        sprite
    }

    /// Puts a new sprite on its tile. The caller has checked the tile is free.
    pub(crate) fn place(&mut self, id: EntityId, sprite: Sprite) {
        let index = self.index(sprite.pos);
        debug_assert!(
            self.on_tile[index].is_none(),
            "{:?} already holds a sprite",
            sprite.pos
        );
        self.on_tile[index] = Some(id);
        self.by_id.insert(id, sprite);
    }

    fn index(&self, pos: Pos) -> usize {
        usize::from(pos.y) * usize::from(self.width) + usize::from(pos.x)
    }

    /// Checks the sprites' invariants (design §7.1), or says which is broken:
    /// the tile index matches where the sprites are, so no two share a tile;
    /// none stands on a solid object; and every chemical is within 0 to 1.
    pub(crate) fn check(
        &self,
        map: &Map,
        objects: &Objects,
        data: &DataPack,
    ) -> Result<(), String> {
        let indexed = self.on_tile.iter().filter(|id| id.is_some()).count();
        if indexed != self.by_id.len() {
            return Err(format!(
                "the tile index holds {indexed} sprites, but there are {}",
                self.by_id.len()
            ));
        }
        for (&id, sprite) in &self.by_id {
            let pos = sprite.pos;
            if !map.contains(pos) || self.at(pos) != Some(id) {
                return Err(format!("the tile index doesn't have {id:?} on {pos:?}"));
            }
            if let Some(object) = objects.at(pos)
                && data.object_types()[objects.kind(object)].solid
            {
                return Err(format!("{id:?} stands on the solid {object:?}"));
            }
            if let Some(level) = sprite.body.chems.iter().find(|l| !(0.0..=1.0).contains(*l)) {
                return Err(format!("{id:?} has a chemical at {level}, outside 0 to 1"));
            }
        }
        Ok(())
    }
}

/// Syllables for sprite names: consonant then vowel, easy to say.
const SYLLABLES: [&str; 32] = [
    "ka", "ke", "ki", "ko", "ku", "la", "le", "li", "lo", "ma", "me", "mi", "mo", "mu", "na", "ne",
    "ni", "no", "ra", "re", "ri", "ro", "sa", "se", "si", "so", "ta", "te", "ti", "to", "va", "vi",
];
/// What a name ends with after its syllables: nothing, half the time.
const ENDINGS: [&str; 8] = ["", "", "", "", "n", "l", "r", "s"];

/// A sprite's name (design §6.5): a fixed syllable generator applied to its
/// ID, so it takes no random draws and the same ID always gets the same name.
/// Names can repeat; the ID tells sprites apart.
pub(crate) fn name(id: EntityId) -> String {
    // SplitMix64's finaliser, so neighbouring IDs get unrelated names.
    let mut bits = id.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
    bits = (bits ^ (bits >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    bits = (bits ^ (bits >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    bits ^= bits >> 31;
    let syllable = |shift: u32| SYLLABLES[(bits >> shift) as usize % SYLLABLES.len()];
    let ending = ENDINGS[(bits >> 10) as usize % ENDINGS.len()];
    let mut name = format!("{}{}{ending}", syllable(0), syllable(5));
    name[..1].make_ascii_uppercase();
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_the_same_for_the_same_id_every_time() {
        assert_eq!(name(EntityId(12)), name(EntityId(12)));
    }

    #[test]
    fn names_are_capitalised_ascii_letters_of_4_to_5_characters() {
        for id in 1..500 {
            let name = name(EntityId(id));
            assert!((4..=5).contains(&name.len()), "{name}");
            assert!(name.chars().all(|c| c.is_ascii_alphabetic()), "{name}");
            assert!(name.starts_with(|c: char| c.is_ascii_uppercase()), "{name}");
        }
    }

    #[test]
    fn neighbouring_ids_get_different_names_mostly() {
        let names: std::collections::BTreeSet<String> =
            (1..=100).map(|id| name(EntityId(id))).collect();
        assert!(
            names.len() > 80,
            "only {} different names in 100",
            names.len()
        );
    }
}
