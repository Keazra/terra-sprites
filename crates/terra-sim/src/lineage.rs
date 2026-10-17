//! Lineage (M2 design §17): who descends from whom. Each sprite and egg
//! knows its parents and its generation, and once a sprite dies the world
//! keeps a short record of it, so the family tree can show the dead.
//! Nothing in the world reads the record: it's the player's, like a
//! breeder's studbook, so sprites live exactly as they would without it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ageing::LifeStage;
use crate::breeding::Sex;
use crate::events::DeathCause;
use crate::objects::EntityId;
use crate::sprites::{Sprite, Sprites};

/// How many dead sprites the world remembers (M2 design §17.1): past it, the
/// longest dead are forgotten first, but never one the player named.
pub(crate) const REMEMBERED: usize = 10_000;

/// The generation of a sprite with no parents: one the world began with,
/// or the Cursor placed (M2 design §17.1).
pub(crate) const FIRST_GENERATION: u32 = 1;

/// Where a sprite came from (M2 design §17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// One the world began with.
    First,
    /// One the Cursor placed.
    Placed,
    /// One that hatched from an egg.
    Hatched,
}

impl Origin {
    /// Where a sprite with `parents` came from, if the Cursor `placed` it
    /// or not.
    pub(crate) fn of(parents: &[EntityId], placed: bool) -> Origin {
        match (parents.is_empty(), placed) {
            (false, _) => Origin::Hatched,
            (true, true) => Origin::Placed,
            (true, false) => Origin::First,
        }
    }
}

/// The generation of a sprite whose parents are of `parents` generations:
/// one more than the newer parent's, or the first, with none (M2 design
/// §17.1).
pub(crate) fn generation_after(parents: impl IntoIterator<Item = u32>) -> u32 {
    parents
        .into_iter()
        .max()
        .map_or(FIRST_GENERATION, |newest| newest + 1)
}

/// What the world keeps of a sprite once it dies (M2 design §17.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Record {
    /// Its parents: the one that laid it, then its partner, if it had one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) parents: Vec<EntityId>,
    /// The name the player gave it, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// Whether the Cursor placed it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) placed: bool,
    pub(crate) generation: u32,
    /// The tick it was born on, as a sprite's is.
    pub(crate) born: i64,
    /// The tick it died on.
    pub(crate) died: u64,
    pub(crate) cause: DeathCause,
    /// Its life stage when it died.
    pub(crate) stage: LifeStage,
    /// Its sex when it died.
    pub(crate) sex: Sex,
}

impl Record {
    /// The record of `sprite`, dying of `cause` on the tick `tick`.
    pub(crate) fn of(sprite: &Sprite, cause: DeathCause, tick: u64) -> Record {
        Record {
            parents: sprite.parents.clone(),
            name: sprite.name.clone(),
            placed: sprite.placed,
            generation: sprite.generation,
            born: sprite.born,
            died: tick,
            cause,
            stage: sprite.stage(),
            sex: sprite.sex,
        }
    }

    /// Its age when it died, in ticks.
    pub(crate) fn age(&self) -> u64 {
        u64::try_from(i128::from(self.died) - i128::from(self.born)).expect("born before it died")
    }
}

/// The world's record of the dead (M2 design §17.1), by ID: the newest
/// `REMEMBERED`, and every one the player named.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct RecordOfTheDead {
    dead: BTreeMap<EntityId, Record>,
}

impl RecordOfTheDead {
    /// Remembers the dead sprite `id`, forgetting the longest dead the
    /// player didn't name while more than `REMEMBERED` are kept.
    pub(crate) fn remember(&mut self, id: EntityId, record: Record) {
        self.remember_within(id, record, REMEMBERED);
    }

    /// The same, keeping no more than `most` but the named.
    fn remember_within(&mut self, id: EntityId, record: Record, most: usize) {
        self.dead.insert(id, record);
        while self.dead.len() > most {
            let longest_dead = self
                .dead
                .iter()
                .filter(|(_, record)| record.name.is_none())
                .min_by_key(|&(&id, record)| (record.died, id))
                .map(|(&id, _)| id);
            match longest_dead {
                Some(id) => self.dead.remove(&id),
                None => break,
            };
        }
    }

    /// The record of the dead sprite `id`, if the world remembers it.
    pub(crate) fn get(&self, id: EntityId) -> Option<&Record> {
        self.dead.get(&id)
    }

    /// Every dead sprite the world remembers, in ascending ID order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (EntityId, &Record)> {
        self.dead.iter().map(|(&id, record)| (id, record))
    }

    /// Checks the record (design §7.1): each of the dead was born before it
    /// died, of parents older than itself, in a generation after theirs
    /// where the world knows them, living or remembered, and no more are
    /// kept than `REMEMBERED` and the named. That none of them is still in
    /// the world is checked from the world's side, every tick
    /// (`World::check_invariants`).
    pub(crate) fn check(&self, sprites: &Sprites) -> Result<(), String> {
        let named = self.dead.values().filter(|r| r.name.is_some()).count();
        if self.dead.len() > REMEMBERED.max(named) {
            return Err(format!(
                "the record of the dead holds {} sprites, past {REMEMBERED}",
                self.dead.len()
            ));
        }
        for (&id, record) in &self.dead {
            if i128::from(record.born) > i128::from(record.died) {
                return Err(format!("{id:?} is remembered as dying before it was born"));
            }
            if let Some(parent) = record.parents.iter().find(|parent| **parent >= id) {
                return Err(format!(
                    "{id:?} has a parent, {parent:?}, younger than itself"
                ));
            }
            let older = record
                .parents
                .iter()
                .filter_map(|&parent| match sprites.get(parent) {
                    Some(sprite) => Some(sprite.generation),
                    None => self.get(parent).map(|parent| parent.generation),
                })
                .find(|&generation| generation >= record.generation);
            if record.generation < FIRST_GENERATION || older.is_some() {
                return Err(format!(
                    "{id:?} is remembered in generation {}, not after its parents'",
                    record.generation
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Map;
    use crate::test_util::builtin;

    fn record(died: u64, name: Option<&str>) -> Record {
        Record {
            parents: Vec::new(),
            name: name.map(str::to_string),
            placed: false,
            generation: FIRST_GENERATION,
            born: 0,
            died,
            cause: DeathCause::OldAge,
            stage: LifeStage::Old,
            sex: Sex::Male,
        }
    }

    fn remembered(dead: &RecordOfTheDead) -> Vec<u64> {
        dead.iter().map(|(id, _)| id.0).collect()
    }

    #[test]
    fn past_the_most_the_longest_dead_are_forgotten_first() {
        let mut dead = RecordOfTheDead::default();
        // Sprite 1 died last, so it's kept the longest, whatever its ID.
        for (id, died) in [(1, 30), (2, 10), (3, 20), (4, 40)] {
            dead.remember_within(EntityId(id), record(died, None), 3);
        }
        assert_eq!(remembered(&dead), [1, 3, 4]);
    }

    #[test]
    fn a_named_sprite_is_never_forgotten() {
        let mut dead = RecordOfTheDead::default();
        dead.remember_within(EntityId(1), record(10, Some("Pip")), 2);
        for (id, died) in [(2, 20), (3, 30), (4, 40)] {
            dead.remember_within(EntityId(id), record(died, None), 2);
        }
        assert_eq!(remembered(&dead), [1, 4]);
        // With only the named left to forget, it keeps them all.
        for id in 5..8 {
            dead.remember_within(EntityId(id), record(50, Some("Bo")), 2);
        }
        assert_eq!(remembered(&dead), [1, 5, 6, 7]);
    }

    #[test]
    fn the_world_remembers_the_newest_ten_thousand() {
        let mut dead = RecordOfTheDead::default();
        for id in 0..=10_000 {
            dead.remember(EntityId(id), record(id, None));
        }
        let kept = remembered(&dead);
        assert_eq!(kept.len(), 10_000);
        assert_eq!(kept[0], 1, "the first to die is forgotten");
    }

    /// No sprites, to check a record alone against.
    fn no_sprites() -> Sprites {
        let map = Map::from_ascii(&["."], &builtin()).expect("a map");
        Sprites::new(&map)
    }

    #[test]
    fn one_remembered_as_dying_before_it_was_born_is_caught() {
        // M2 design §17.1, checked every tick (design §7.1).
        let sprites = no_sprites();
        let mut dead = RecordOfTheDead::default();
        let mut on_the_tick = record(10, None);
        on_the_tick.born = 10;
        dead.remember(EntityId(1), on_the_tick);
        assert_eq!(dead.check(&sprites), Ok(()));
        let mut early = record(10, None);
        early.born = 11;
        dead.remember(EntityId(2), early);
        assert!(
            dead.check(&sprites)
                .is_err_and(|why| why.contains("dying before it was born"))
        );
    }

    #[test]
    fn a_record_far_from_any_world_s_ticks_reads_without_overflowing() {
        // A damaged save could hold any ticks: an age, and the check that
        // it died after it was born, count them all as they are.
        let sprites = no_sprites();
        let mut ancient = record(10, None);
        ancient.born = i64::MIN;
        assert_eq!(ancient.age(), 10 + (1 << 63));
        let mut last = record(u64::MAX, None);
        last.born = 0;
        assert_eq!(last.age(), u64::MAX);
        let mut dead = RecordOfTheDead::default();
        dead.remember(EntityId(1), ancient);
        dead.remember(EntityId(2), last);
        assert_eq!(dead.check(&sprites), Ok(()));
    }

    #[test]
    fn more_remembered_than_ten_thousand_is_caught_unless_they_re_named() {
        // M2 design §17.1: the newest 10,000, and the named however many.
        let sprites = no_sprites();
        let (mut unnamed, mut named) = (RecordOfTheDead::default(), RecordOfTheDead::default());
        for id in 0..=REMEMBERED as u64 {
            unnamed.remember_within(EntityId(id), record(id, None), REMEMBERED + 1);
            named.remember_within(EntityId(id), record(id, Some("Pip")), REMEMBERED + 1);
        }
        assert!(
            unnamed
                .check(&sprites)
                .is_err_and(|why| why.contains("past 10000"))
        );
        assert_eq!(named.check(&sprites), Ok(()));
    }

    #[test]
    fn a_generation_is_one_after_the_newer_parent() {
        assert_eq!(generation_after([]), FIRST_GENERATION);
        assert_eq!(generation_after([3]), 4);
        assert_eq!(generation_after([2, 5]), 6);
    }
}
