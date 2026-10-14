//! The population record (M2 design §14): the world's counts of its sprites
//! by life stage, and its eggs, over its whole run, with the oldest and
//! newest generations alive (§17.1) and what happened between them, for the
//! population graph. It only watches: nothing in the world reads it.
//!
//! It counts at the first tick it steps from, then every 100 ticks. Once it
//! holds 512 counts it squeezes them: it keeps the first and every other
//! one after it, folding what happened in the dropped ones' stretches into
//! the next kept one, and counts half as often. So it always covers the
//! whole run, and a save stays small however long the world runs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ageing::LifeStage;
use crate::events::{DeathCause, Event, EventKind};
use crate::objects::Objects;
use crate::sprites::Sprites;

/// Ticks between counts, until the record first squeezes (M2 design §14).
pub const FIRST_COUNT_EVERY: u64 = 100;

/// The most counts the record holds; at this many, it squeezes them.
pub(crate) const MOST_COUNTS: usize = 512;

/// One count of the population (M2 design §14): the eggs and the sprites at
/// each life stage at a tick, the generations of the sprites, and what
/// happened since the count before. An egg is an item until it hatches (M2
/// design §10.4), on the map or in the Cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationCount {
    /// The tick it was taken at, before that tick's step.
    pub tick: u64,
    pub eggs: u32,
    pub young: u32,
    pub adults: u32,
    pub old: u32,
    /// The oldest and newest generations among the sprites (M2 design
    /// §17.1), not the eggs: none with no sprites, or in a count from a
    /// save made before the record kept them (#208).
    #[serde(default)]
    pub generations: Option<Generations>,
    /// What happened since the count before; for the first count, nothing.
    pub since: PopulationChanges,
}

/// The oldest and newest generations among a count's sprites (M2 design
/// §17.1, #208).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Generations {
    pub oldest: u32,
    pub newest: u32,
}

impl Generations {
    /// The oldest and newest of `generations`, or none if there are none.
    fn of(generations: impl IntoIterator<Item = u32>) -> Option<Generations> {
        let mut generations = generations.into_iter();
        let first = generations.next()?;
        let mut seen = Generations {
            oldest: first,
            newest: first,
        };
        for generation in generations {
            seen.oldest = seen.oldest.min(generation);
            seen.newest = seen.newest.max(generation);
        }
        Some(seen)
    }
}

impl PopulationCount {
    /// The sprites counted, at every life stage.
    pub fn sprites(&self) -> u32 {
        self.young + self.adults + self.old
    }

    /// The sprites and the eggs, as the world's limit counts them (M2
    /// design §10.6).
    pub fn with_eggs(&self) -> u32 {
        self.sprites() + self.eggs
    }

    /// The sprites in `sprites`, by life stage, with their generations, and
    /// the eggs in `objects`, at `tick`, with nothing happened yet.
    fn of(sprites: &Sprites, objects: &Objects, tick: u64) -> PopulationCount {
        let mut count = PopulationCount {
            tick,
            eggs: objects.eggs() as u32,
            generations: Generations::of(sprites.iter().map(|(_, sprite)| sprite.generation)),
            ..PopulationCount::default()
        };
        for (_, sprite) in sprites.iter() {
            *match sprite.stage() {
                LifeStage::Young => &mut count.young,
                LifeStage::Adult => &mut count.adults,
                LifeStage::Old => &mut count.old,
            } += 1;
        }
        count
    }
}

/// What changed the population over a stretch of time (M2 design §14).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationChanges {
    /// Eggs laid.
    pub laid: u32,
    /// Eggs that hatched.
    pub hatched: u32,
    /// Sprites the Cursor spawned (design §6.5).
    pub spawned: u32,
    /// Sprites that died, by cause.
    pub died: BTreeMap<DeathCause, u32>,
}

impl PopulationChanges {
    /// How many died, of every cause.
    pub fn deaths(&self) -> u32 {
        self.died.values().sum()
    }

    /// Adds what happened in `other`, a stretch of time next to this one.
    pub fn add(&mut self, other: &PopulationChanges) {
        self.laid += other.laid;
        self.hatched += other.hatched;
        self.spawned += other.spawned;
        for (&cause, &n) in &other.died {
            *self.died.entry(cause).or_insert(0) += n;
        }
    }

    /// Adds what `events` did to the population.
    fn note(&mut self, events: &[Event]) {
        for event in events {
            match event.kind {
                EventKind::Laid { .. } => self.laid += 1,
                EventKind::Hatched { .. } => self.hatched += 1,
                EventKind::Spawned { .. } => self.spawned += 1,
                EventKind::Died { cause, .. } => *self.died.entry(cause).or_insert(0) += 1,
                _ => {}
            }
        }
    }
}

/// The world's population over its whole run (M2 design §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PopulationRecord {
    /// Ticks between counts: 100, doubled each time the record squeezes.
    every: u64,
    /// The counts, oldest first.
    counts: Vec<PopulationCount>,
    /// What has happened since the last count.
    since: PopulationChanges,
}

impl Default for PopulationRecord {
    fn default() -> PopulationRecord {
        PopulationRecord {
            every: FIRST_COUNT_EVERY,
            counts: Vec::new(),
            since: PopulationChanges::default(),
        }
    }
}

impl PopulationRecord {
    /// The counts, oldest first.
    pub(crate) fn counts(&self) -> &[PopulationCount] {
        &self.counts
    }

    /// Ticks between counts now.
    pub(crate) fn every(&self) -> u64 {
        self.every
    }

    /// The population in `sprites` and `objects` at `tick`, with what has
    /// happened since the last count, as a count taken now would have it.
    pub(crate) fn now(&self, sprites: &Sprites, objects: &Objects, tick: u64) -> PopulationCount {
        PopulationCount {
            since: self.since.clone(),
            ..PopulationCount::of(sprites, objects, tick)
        }
    }

    /// Before a tick's step: if the record has no count yet, as for a new
    /// world or one loaded from a save made before it, counts the sprites
    /// and eggs as they are at `tick`.
    pub(crate) fn begin(&mut self, sprites: &Sprites, objects: &Objects, tick: u64) {
        if self.counts.is_empty() {
            self.counts
                .push(PopulationCount::of(sprites, objects, tick));
        }
    }

    /// After a tick's step, `tick` being the next: notes what `events` did
    /// to the population, and counts it if a count is due.
    pub(crate) fn end(
        &mut self,
        sprites: &Sprites,
        objects: &Objects,
        tick: u64,
        events: &[Event],
    ) {
        self.since.note(events);
        if tick.is_multiple_of(self.every) {
            self.counts.push(PopulationCount {
                since: std::mem::take(&mut self.since),
                ..PopulationCount::of(sprites, objects, tick)
            });
            if self.counts.len() >= MOST_COUNTS {
                self.squeeze();
            }
        }
    }

    /// Counts half as often: keeps the first count and those at a multiple
    /// of twice the old interval, folding what happened in each dropped
    /// count's stretch into the next kept one's, or the last's into what
    /// has happened since.
    fn squeeze(&mut self) {
        self.every *= 2;
        let every = self.every;
        let mut kept: Vec<PopulationCount> = Vec::with_capacity(self.counts.len() / 2 + 1);
        let mut dropped = PopulationChanges::default();
        for (i, mut count) in std::mem::take(&mut self.counts).into_iter().enumerate() {
            if i == 0 || count.tick.is_multiple_of(every) {
                let mut since = std::mem::take(&mut dropped);
                since.add(&count.since);
                count.since = since;
                kept.push(count);
            } else {
                dropped.add(&count.since);
            }
        }
        dropped.add(&self.since);
        self.since = dropped;
        self.counts = kept;
    }

    /// Why the record is broken, if it is (design §7.1): its counts must
    /// run forward in time, there must be no more than it holds, and a
    /// count's generations must run from the oldest to the newest, with
    /// none in a count of no sprites.
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.counts.len() >= MOST_COUNTS {
            return Err(format!(
                "the population record holds {} counts, past its {MOST_COUNTS}",
                self.counts.len()
            ));
        }
        for count in &self.counts {
            match count.generations {
                Some(Generations { oldest, newest }) if oldest > newest => {
                    return Err(format!(
                        "the population count at tick {} has generations {oldest} to {newest}",
                        count.tick
                    ));
                }
                Some(_) if count.sprites() == 0 => {
                    return Err(format!(
                        "the population count at tick {} has generations but no sprites",
                        count.tick
                    ));
                }
                _ => {}
            }
        }
        if let Some(pair) = self
            .counts
            .windows(2)
            .find(|pair| pair[0].tick >= pair[1].tick)
        {
            return Err(format!(
                "the population record's counts go back in time, from tick {} to {}",
                pair[0].tick, pair[1].tick
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record after counts at `ticks`, each with one hatching since the
    /// count before (none for the first).
    fn record_of(every: u64, ticks: &[u64]) -> PopulationRecord {
        let counts = ticks
            .iter()
            .enumerate()
            .map(|(i, &tick)| PopulationCount {
                tick,
                adults: tick as u32,
                generations: Some(Generations {
                    oldest: 1,
                    newest: 1 + i as u32,
                }),
                since: PopulationChanges {
                    hatched: u32::from(i > 0),
                    ..PopulationChanges::default()
                },
                ..PopulationCount::default()
            })
            .collect();
        PopulationRecord {
            every,
            counts,
            since: PopulationChanges::default(),
        }
    }

    #[test]
    fn squeezing_keeps_the_first_count_and_every_other_after_it() {
        let mut record = record_of(100, &[0, 100, 200, 300, 400, 500]);
        record.squeeze();
        assert_eq!(record.every, 200);
        let ticks: Vec<u64> = record.counts.iter().map(|c| c.tick).collect();
        assert_eq!(ticks, [0, 200, 400]);
    }

    #[test]
    fn squeezing_loses_nothing_that_happened() {
        let mut record = record_of(100, &[0, 100, 200, 300, 400, 500]);
        record.since.hatched = 1;
        record.squeeze();
        let hatched: Vec<u32> = record.counts.iter().map(|c| c.since.hatched).collect();
        assert_eq!(hatched, [0, 2, 2], "each kept count has its dropped one's");
        assert_eq!(
            record.since.hatched, 2,
            "the last count's goes to what's since"
        );
        let counted: u32 = record.counts.iter().map(|c| c.adults).sum();
        assert_eq!(counted, 600, "a kept count keeps its own numbers");
        let newest: Vec<u32> = record
            .counts
            .iter()
            .filter_map(|c| c.generations.map(|g| g.newest))
            .collect();
        assert_eq!(newest, [1, 3, 5], "and its own generations (#208)");
    }

    #[test]
    fn a_record_that_began_off_the_beat_keeps_its_first_count() {
        // A save from before the record began at tick 12,345.
        let mut record = record_of(100, &[12_345, 12_400, 12_500, 12_600]);
        record.squeeze();
        let ticks: Vec<u64> = record.counts.iter().map(|c| c.tick).collect();
        assert_eq!(ticks, [12_345, 12_400, 12_600]);
    }

    #[test]
    fn counts_out_of_order_are_caught() {
        let record = record_of(100, &[0, 200, 100]);
        assert!(record.check().is_err());
    }

    #[test]
    fn generations_out_of_order_or_without_sprites_are_caught() {
        let mut record = record_of(100, &[100, 200]);
        assert_eq!(record.check(), Ok(()));
        record.counts[1].generations = Some(Generations {
            oldest: 3,
            newest: 2,
        });
        assert!(record.check().is_err(), "newest before oldest");
        let mut record = record_of(100, &[100, 200]);
        record.counts[1].adults = 0;
        assert!(record.check().is_err(), "generations with no sprites");
        record.counts[1].generations = None;
        assert_eq!(record.check(), Ok(()));
    }

    #[test]
    fn generations_are_the_oldest_and_newest() {
        assert_eq!(Generations::of([]), None);
        assert_eq!(
            Generations::of([4, 2, 6, 3]),
            Some(Generations {
                oldest: 2,
                newest: 6
            })
        );
    }
}
