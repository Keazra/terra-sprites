//! The world's sprites and where they stand (design §2.3, §3.4, §4).

use std::collections::{BTreeMap, VecDeque};

use serde::Serialize;

use crate::action::{Action, Did, ScriptedAction};
use crate::biochem::{Body, Program};
use crate::data::DataPack;
use crate::genome::Genome;
use crate::map::{Map, Pos};
use crate::objects::{EntityId, Objects};
use crate::occupancy::Occupancy;
use crate::perception::Flood;

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
    /// What it's doing, or the action that last ended until the next starts.
    pub(crate) action: Option<Action>,
    /// The actions a hand-made world starts it on, in order, until they start.
    pub(crate) scripted: VecDeque<ScriptedAction>,
    /// Its cached perception flood (design §3.6).
    pub(crate) flood: Option<Flood>,
    /// Move points banked towards its next step, in tenths (design §3.7).
    pub(crate) move_points: u32,
    /// What it did at the last step 6.
    pub(crate) did: Did,
}

impl Sprite {
    /// The sprite's age on the tick `tick`.
    pub(crate) fn age(&self, tick: u64) -> u64 {
        tick - self.born
    }

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
            action: None,
            scripted: VecDeque::new(),
            flood: None,
            move_points: 0,
            did: Did::default(),
        }
    }
}

/// Every sprite in the world, and the tile each stands on. A tile holds at
/// most one sprite (design §3.4).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Sprites {
    /// In ascending ID order, the order every per-sprite pass takes (design §2.3).
    by_id: BTreeMap<EntityId, Sprite>,
    /// The sprite on each tile. It's derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    on_tile: Occupancy,
}

impl Sprites {
    /// No sprites, on `map`.
    pub(crate) fn new(map: &Map) -> Sprites {
        Sprites {
            by_id: BTreeMap::new(),
            on_tile: Occupancy::new(map),
        }
    }

    /// The sprite on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.on_tile.at(pos)
    }

    /// Every sprite, in ascending ID order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (EntityId, &Sprite)> {
        self.by_id.iter().map(|(&id, sprite)| (id, sprite))
    }

    /// The sprite `id`, if it exists.
    pub(crate) fn get(&self, id: EntityId) -> Option<&Sprite> {
        self.by_id.get(&id)
    }

    /// Every sprite's body, in ascending ID order, to change.
    pub(crate) fn bodies_mut(&mut self) -> impl Iterator<Item = &mut Body> {
        self.by_id.values_mut().map(|sprite| &mut sprite.body)
    }

    /// The sprite `id`, if it exists, to change.
    pub(crate) fn get_mut(&mut self, id: EntityId) -> Option<&mut Sprite> {
        self.by_id.get_mut(&id)
    }

    /// Takes the sprite `id`, which must exist, out of the world.
    pub(crate) fn remove(&mut self, id: EntityId) -> Sprite {
        let sprite = self.by_id.remove(&id).expect("the sprite to remove");
        self.on_tile.clear(sprite.pos);
        sprite
    }

    /// Moves the sprite `id`, which must exist, onto the tile at `to`. The
    /// caller has checked the tile is free.
    pub(crate) fn move_to(&mut self, id: EntityId, to: Pos) {
        let sprite = self.by_id.get_mut(&id).expect("the sprite to move");
        self.on_tile.clear(sprite.pos);
        self.on_tile.put(to, id);
        sprite.pos = to;
    }

    /// Swaps the tiles of sprites `a` and `b`, which must exist.
    pub(crate) fn swap(&mut self, a: EntityId, b: EntityId) {
        let pa = self.by_id[&a].pos;
        let pb = self.by_id[&b].pos;
        self.on_tile.clear(pa);
        self.on_tile.clear(pb);
        self.on_tile.put(pa, b);
        self.on_tile.put(pb, a);
        self.by_id.get_mut(&a).expect("sprite a").pos = pb;
        self.by_id.get_mut(&b).expect("sprite b").pos = pa;
    }

    /// Puts a new sprite on its tile. The caller has checked the tile is free.
    pub(crate) fn place(&mut self, id: EntityId, sprite: Sprite) {
        self.on_tile.put(sprite.pos, id);
        self.by_id.insert(id, sprite);
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
        let indexed = self.on_tile.count();
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
            if objects.is_solid_at(data, pos) {
                return Err(format!("{id:?} stands on a solid object at {pos:?}"));
            }
            if sprite.body.chems_before_tick.len() != sprite.body.chems.len() {
                return Err(format!(
                    "{id:?} has {} levels from a tick ago, but {} chemicals",
                    sprite.body.chems_before_tick.len(),
                    sprite.body.chems.len()
                ));
            }
            if let Some(level) = sprite.body.chems.iter().find(|l| !(0.0..=1.0).contains(*l)) {
                return Err(format!("{id:?} has a chemical at {level}, outside 0 to 1"));
            }
            if let Some(value) = sprite.body.loci.iter().find(|v| !v.is_finite()) {
                return Err(format!(
                    "{id:?} has a locus at {value}, which isn't a number"
                ));
            }
        }
        Ok(())
    }
}
