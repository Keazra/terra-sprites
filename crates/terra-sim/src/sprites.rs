//! The world's sprites and where they stand (design §2.3, §3.4, §4).

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::action::{Action, Did, ScriptedAction, Walk, chebyshev};
use crate::ageing::LifeStage;
use crate::biochem::{Body, Program};
use crate::brain::Brain;
use crate::data::DataPack;
use crate::genome::Genome;
use crate::map::{Dir, Map, Pos};
use crate::objects::{EntityId, Objects};
use crate::occupancy::Occupancy;
use crate::perception::Flood;
use crate::physiology::RememberedPlaces;
use crate::sliding::Slide;

/// One sprite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Sprite {
    pub(crate) pos: Pos,
    /// The tick it was born on: before the world began for a sprite that
    /// starts it grown (M2 design §9.1).
    pub(crate) born: i64,
    /// The name the player gave it, if any (design v28 §6.5).
    pub(crate) name: Option<String>,
    pub(crate) genome: Genome,
    /// The genome compiled for the chemistry step. It's derived from the
    /// genome, so it isn't hashed.
    #[serde(skip)]
    pub(crate) program: Program,
    /// The life stage its genes are expressed for (M2 design §9.1): the
    /// one it grew to at the latest step 3, or was made at. `None` only in
    /// a save from before life stages, until it's loaded.
    #[serde(default)]
    stage: Option<LifeStage>,
    pub(crate) body: Body,
    pub(crate) brain: Brain,
    /// What it's doing, or the action that last ended until the next starts.
    pub(crate) action: Option<Action>,
    /// Its way after the Cursor, while the Cursor leads it (design v23 §6.5).
    pub(crate) lead: Option<Walk>,
    /// Its slide, while a shove sends it (design v25 §3.5.4).
    pub(crate) slide: Option<Slide>,
    /// The actions a hand-made world starts it on, in order, until they start.
    pub(crate) scripted: VecDeque<ScriptedAction>,
    /// Its cached perception flood (design §3.6).
    pub(crate) flood: Option<Flood>,
    /// Move points banked towards its next step, in tenths (design §3.7).
    pub(crate) move_points: u32,
    /// What it did at the last step 6.
    pub(crate) did: Did,
    /// The direction of its latest step, if it has taken one: the way it
    /// pushes an item on its own tile (design §3.5.2).
    pub(crate) last_step: Option<Dir>,
}

impl Sprite {
    /// The sprite's age on the tick `tick`.
    pub(crate) fn age(&self, tick: u64) -> u64 {
        u64::try_from(tick as i64 - self.born).expect("a sprite is born by now")
    }

    /// The life stage its genes are expressed for (M2 design §9.1).
    pub(crate) fn stage(&self) -> LifeStage {
        self.stage.expect("a loaded sprite has a life stage")
    }

    /// How fast it walks on the tick `tick`, in move points (M2 design §9.4,
    /// §9.5): its speed trait, less while it's young, rising steadily to
    /// all of it as it grows up, and less as its old body declines.
    pub(crate) fn speed(&self, tick: u64, data: &DataPack) -> f32 {
        let ageing = data.physiology().ageing;
        let traits = self.program.traits;
        let growing = if self.stage() == LifeStage::Young {
            let grown = (self.age(tick) as f32 / traits.grows_up).min(1.0);
            ageing.young_speed + (1.0 - ageing.young_speed) * grown
        } else {
            1.0
        };
        traits.speed * growing * self.body.kept(ageing.worn_out.walks)
    }

    /// How far it perceives, in tiles (M2 design §9.4): its sense radius
    /// trait, less as its old body declines.
    pub(crate) fn sense_radius(&self, data: &DataPack) -> f32 {
        let sees = data.physiology().ageing.worn_out.sees;
        self.program.traits.sense_radius * self.body.kept(sees)
    }

    /// Grows the sprite to its life stage on the tick `tick` (M2 design
    /// §9.2): if that's a new one, its genes are expressed afresh for it,
    /// leaving what it has learned as it was. Returns the stage it was in
    /// and the new one, if it has one.
    pub(crate) fn grow(&mut self, tick: u64, data: &DataPack) -> Option<(LifeStage, LifeStage)> {
        let was = self.stage();
        let stage = self.program.traits.life_stage(self.age(tick));
        if stage == was {
            return None;
        }
        self.stage = Some(stage);
        self.program = Program::new(&self.genome, data, stage);
        self.brain.reach(&self.genome, data, stage);
        Some((was, stage))
    }

    /// A newborn with `genome` on `pos`, born on the tick `born` (design §4.7).
    pub(crate) fn newborn(genome: Genome, pos: Pos, born: u64, data: &DataPack) -> Sprite {
        Sprite::aged(genome, pos, born, 0, data)
    }

    /// A sprite with `genome` on `pos` that has just grown up on the tick
    /// `tick` (M2 design §9.1), with a newborn's body (design §4.7).
    pub(crate) fn grown_up(genome: Genome, pos: Pos, tick: u64, data: &DataPack) -> Sprite {
        let grows_up = Program::new(&genome, data, LifeStage::Young)
            .traits
            .grows_up;
        Sprite::aged(genome, pos, tick, libm::ceilf(grows_up) as u64, data)
    }

    /// A sprite with `genome` on `pos` that is `age` ticks old on the tick
    /// `tick`, its genes expressed for its life stage then (M2 design §9.1).
    /// Its body is a newborn's (design §4.7), and it has learned nothing.
    pub(crate) fn aged(genome: Genome, pos: Pos, tick: u64, age: u64, data: &DataPack) -> Sprite {
        // Its traits, which set its stage, don't depend on it.
        let traits = Program::new(&genome, data, LifeStage::Young).traits;
        let stage = traits.life_stage(age);
        let program = Program::new(&genome, data, stage);
        let body = Body::newborn(&program, data);
        let brain = Brain::new(&genome, data, stage);
        Sprite {
            pos,
            born: tick as i64 - age as i64,
            name: None,
            genome,
            program,
            stage: Some(stage),
            body,
            brain,
            action: None,
            lead: None,
            slide: None,
            scripted: VecDeque::new(),
            flood: None,
            move_points: 0,
            did: Did::default(),
            last_step: None,
        }
    }
}

/// Every sprite in the world, and the tile each stands on. A tile holds at
/// most one sprite (design §3.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Every sprite, in ascending ID order, to change. Where each stands
    /// isn't theirs to change: `move_to` and `swap` keep the tile index.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Sprite)> {
        self.by_id.iter_mut().map(|(&id, sprite)| (id, sprite))
    }

    /// The sprite `id`, if it exists.
    pub(crate) fn get(&self, id: EntityId) -> Option<&Sprite> {
        self.by_id.get(&id)
    }

    /// Every sprite's body, in ascending ID order, to change.
    pub(crate) fn bodies_mut(&mut self) -> impl Iterator<Item = &mut Body> {
        self.by_id.values_mut().map(|sprite| &mut sprite.body)
    }

    /// Every sprite's ID, body and brain, in ascending ID order, to change.
    pub(crate) fn minds_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Body, &mut Brain)> {
        self.by_id
            .iter_mut()
            .map(|(&id, sprite)| (id, &mut sprite.body, &mut sprite.brain))
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
        sprite.last_step = Dir::towards(sprite.pos, to).or(sprite.last_step);
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
        for (id, from, to) in [(a, pa, pb), (b, pb, pa)] {
            let sprite = self.by_id.get_mut(&id).expect("a swapping sprite");
            sprite.last_step = Dir::towards(from, to).or(sprite.last_step);
            sprite.pos = to;
        }
    }

    /// Puts a new sprite on its tile. The caller has checked the tile is free.
    pub(crate) fn place(&mut self, id: EntityId, sprite: Sprite) {
        self.on_tile.put(sprite.pos, id);
        self.by_id.insert(id, sprite);
    }

    /// Rebuilds what a save leaves out (design §2.8): which sprite stands on
    /// each tile of `map`, each one's genome compiled for the chemistry
    /// step, and its levels before the tick, taken as its levels now; its
    /// remembered places are brought within the pack's limits. Says
    /// what's wrong if one is off the map or shares a tile, was born after
    /// the tick `now`, its genome, body or brain doesn't fit `data`, or its
    /// flood doesn't fit `map`.
    pub(crate) fn rebuild(&mut self, map: &Map, data: &DataPack, now: u64) -> Result<(), String> {
        self.on_tile = Occupancy::new(map);
        for (&id, sprite) in &mut self.by_id {
            if !map.contains(sprite.pos) || self.on_tile.at(sprite.pos).is_some() {
                return Err(format!("sprite {} has no tile of its own", id.0));
            }
            if sprite.born > now as i64 {
                return Err(format!("sprite {} was born after the save", id.0));
            }
            // The genome first: the brain is checked against it, once it
            // has any brain parameter added since it was saved.
            let misfit = || format!("sprite {} doesn't fit its data pack", id.0);
            if !sprite.genome.fits(data) {
                return Err(misfit());
            }
            // A save from before life stages has none: its stage is the one
            // its age gives, from its traits, which don't depend on it.
            let stage = match sprite.stage {
                Some(stage) => stage,
                None => Program::new(&sprite.genome, data, LifeStage::Young)
                    .traits
                    .life_stage(sprite.age(now)),
            };
            sprite.stage = Some(stage);
            sprite.brain.params.catch_up(&sprite.genome, data, stage);
            // Places remembered under other limits, in a save from before
            // they were added, are brought within its pack's (M2 design §7.2).
            sprite
                .brain
                .experience
                .keep_within(data.physiology().remembered_places);
            if !sprite.body.fits(data) || !sprite.brain.fits(&sprite.genome, data, stage, now) {
                return Err(misfit());
            }
            if sprite.flood.as_ref().is_some_and(|flood| !flood.fits(map)) {
                return Err(format!("sprite {}'s flood doesn't fit the map", id.0));
            }
            self.on_tile.put(sprite.pos, id);
            sprite.program = Program::new(&sprite.genome, data, stage);
            sprite.body.chems_before_tick.clone_from(&sprite.body.chems);
        }
        Ok(())
    }

    /// Checks the sprites' invariants (design §7.1), or says which is broken:
    /// the tile index matches where the sprites are, so no two share a tile;
    /// none stands on a solid object; every chemical is within 0 to 1; and
    /// each remembers no more places than it may, all on the map (M2 design
    /// §7).
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
            if sprite.stage.is_none() {
                return Err(format!("{id:?} has no life stage"));
            }
            for (name, level) in [("wear", sprite.body.wear), ("decline", sprite.body.decline)] {
                if !(0.0..=1.0).contains(&level) {
                    return Err(format!("{id:?} has a {name} of {level}, outside 0 to 1"));
                }
            }
            if sprite.body.decline > 0.0 && sprite.stage() != LifeStage::Old {
                return Err(format!("{id:?} has declined, but isn't old"));
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
            sprite
                .brain
                .check()
                .map_err(|broken| format!("{id:?} {broken}"))?;
            // Its remembered places are few enough, no two of a kind within
            // the merge distance, and on the map (M2 design §7.2).
            let places = &sprite.brain.experience.places;
            let RememberedPlaces {
                held,
                per_kind,
                merge,
            } = data.physiology().remembered_places;
            if places.len() > usize::from(held) {
                return Err(format!(
                    "{id:?} remembers {} places, over {held}",
                    places.len()
                ));
            }
            for (i, place) in places.iter().enumerate() {
                let of_kind = places.iter().filter(|p| p.subject == place.subject).count();
                if of_kind > usize::from(per_kind) {
                    return Err(format!(
                        "{id:?} remembers {of_kind} places of one kind, over {per_kind}"
                    ));
                }
                let near = places[i + 1..]
                    .iter()
                    .find(|p| p.subject == place.subject && chebyshev(p.at, place.at) <= merge);
                if let Some(near) = near {
                    return Err(format!(
                        "{id:?} remembers two places of one kind within {merge} tiles, at {:?} and {:?}",
                        place.at, near.at
                    ));
                }
            }
            if let Some(place) = places.iter().find(|place| !map.contains(place.at)) {
                return Err(format!(
                    "{id:?} remembers a place off the map, at {:?}",
                    place.at
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::at;

    /// A sprite of speed 9 and sense radius 10 that grows up at 3,000 ticks
    /// and old at 6,000, `age` ticks old on tick 10,000.
    fn sprite(age: u64, data: &DataPack) -> Sprite {
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                Trait(trait: "speed", value: 9.0),
                Trait(trait: "sense_radius", value: 10.0),
                Trait(trait: "grows_up", value: 3000.0),
                Trait(trait: "grows_old", value: 6000.0),
            ])"#,
            data,
        )
        .expect("a valid genome");
        Sprite::aged(genome, at(0, 0), 10_000, age, data)
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "{actual} should be {expected}"
        );
    }

    #[test]
    fn the_young_walk_slower_until_they_grow_up() {
        // M2 design §9.5: from 0.67 of its speed at birth, rising steadily.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        assert_close(sprite(0, &data).speed(10_000, &data), 9.0 * 0.67);
        assert_close(
            sprite(1_500, &data).speed(10_000, &data),
            9.0 * (0.67 + 0.33 * 0.5),
        );
        assert_close(sprite(3_000, &data).speed(10_000, &data), 9.0);
        assert_close(sprite(0, &data).sense_radius(&data), 10.0);
    }

    #[test]
    fn an_old_body_walks_slower_and_sees_less_far_as_it_declines() {
        // M2 design §9.4: a worn-out body keeps 0.55 of its speed and 0.6 of
        // its sense radius.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let mut old = sprite(7_000, &data);
        assert_eq!(old.stage(), LifeStage::Old);
        assert_close(old.speed(10_000, &data), 9.0);
        old.body.decline = 0.5;
        assert_close(old.speed(10_000, &data), 9.0 * (1.0 - 0.5 * 0.45));
        old.body.decline = 1.0;
        assert_close(old.speed(10_000, &data), 9.0 * 0.55);
        assert_close(old.sense_radius(&data), 6.0);
    }
}
