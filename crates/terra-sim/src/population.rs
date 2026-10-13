//! The population record (M2 design §14): the world's counts of its sprites
//! by life stage over its whole run, with what happened between them, for
//! the population graph. It only watches: nothing in the world reads it.
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
use crate::sprites::Sprites;

/// Ticks between counts, until the record first squeezes (M2 design §14).
pub(crate) const FIRST_EVERY: u64 = 100;

/// The most counts the record holds; at this many, it squeezes them.
pub(crate) const MOST_COUNTS: usize = 512;

/// One count of the population (M2 design §14): the sprites at each life
/// stage at a tick, and what happened since the count before.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationCount {
    /// The tick it was taken at, before that tick's step.
    pub tick: u64,
    pub eggs: u32,
    pub young: u32,
    pub adults: u32,
    pub old: u32,
    /// What happened since the count before; for the first count, nothing.
    pub since: Changes,
}

impl PopulationCount {
    /// Every sprite counted, eggs included, as the world's limit counts
    /// them (M2 design §10.6).
    pub fn sprites(&self) -> u32 {
        self.eggs + self.young + self.adults + self.old
    }

    /// How many were at `stage`.
    pub fn at(&self, stage: LifeStage) -> u32 {
        match stage {
            LifeStage::Egg => self.eggs,
            LifeStage::Young => self.young,
            LifeStage::Adult => self.adults,
            LifeStage::Old => self.old,
        }
    }

    /// The sprites in `sprites`, by life stage, at `tick`, with nothing
    /// happened yet.
    fn of(sprites: &Sprites, tick: u64) -> PopulationCount {
        let mut count = PopulationCount {
            tick,
            ..PopulationCount::default()
        };
        for (_, sprite) in sprites.iter() {
            *match sprite.stage() {
                LifeStage::Egg => &mut count.eggs,
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
pub struct Changes {
    /// Eggs that hatched.
    pub hatched: u32,
    /// Sprites the Cursor placed (design v28 §6.5).
    pub placed: u32,
    /// Sprites that died, by cause.
    pub died: BTreeMap<DeathCause, u32>,
}

impl Changes {
    /// How many died, of every cause.
    pub fn deaths(&self) -> u32 {
        self.died.values().sum()
    }

    /// Adds what happened in `other`, a stretch of time next to this one.
    pub fn add(&mut self, other: &Changes) {
        self.hatched += other.hatched;
        self.placed += other.placed;
        for (&cause, &n) in &other.died {
            *self.died.entry(cause).or_insert(0) += n;
        }
    }

    /// Adds what `events` did to the population.
    fn note(&mut self, events: &[Event]) {
        for event in events {
            match event.kind {
                EventKind::Hatched { .. } => self.hatched += 1,
                EventKind::Spawned { .. } => self.placed += 1,
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
    since: Changes,
}

impl Default for PopulationRecord {
    fn default() -> PopulationRecord {
        PopulationRecord {
            every: FIRST_EVERY,
            counts: Vec::new(),
            since: Changes::default(),
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

    /// The population in `sprites` at `tick`, with what has happened since
    /// the last count, as a count taken now would have it.
    pub(crate) fn now(&self, sprites: &Sprites, tick: u64) -> PopulationCount {
        PopulationCount {
            since: self.since.clone(),
            ..PopulationCount::of(sprites, tick)
        }
    }

    /// Before a tick's step: if the record has no count yet, as for a new
    /// world or one loaded from a save made before it, counts the sprites
    /// as they are at `tick`.
    pub(crate) fn begin(&mut self, sprites: &Sprites, tick: u64) {
        if self.counts.is_empty() {
            self.counts.push(PopulationCount::of(sprites, tick));
        }
    }

    /// After a tick's step, `tick` being the next: notes what `events` did
    /// to the population, and counts it if a count is due.
    pub(crate) fn end(&mut self, sprites: &Sprites, tick: u64, events: &[Event]) {
        self.since.note(events);
        if tick.is_multiple_of(self.every) {
            self.counts.push(PopulationCount {
                since: std::mem::take(&mut self.since),
                ..PopulationCount::of(sprites, tick)
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
        let mut dropped = Changes::default();
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
    /// run forward in time, and there must be no more than it holds.
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.counts.len() >= MOST_COUNTS {
            return Err(format!(
                "the population record holds {} counts, past its {MOST_COUNTS}",
                self.counts.len()
            ));
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
                since: Changes {
                    hatched: u32::from(i > 0),
                    ..Changes::default()
                },
                ..PopulationCount::default()
            })
            .collect();
        PopulationRecord {
            every,
            counts,
            since: Changes::default(),
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
}
