//! Whether a world left alone lasted (#222, M2 design §6, G1), for the
//! examples that run one for a long time (`childhood`, `generations`). A
//! seed lasts when all three hold:
//!
//! - at the end it still has sprites or eggs;
//! - it never had fewer than 10 sprites, eggs not counted, at any count of
//!   its population record (M2 design §14);
//! - its sprites over its last 50 hours averaged at least half of what they
//!   averaged over hours 50–100, which catches a fix that only delays the
//!   die-out. A run shorter than 150 hours can't judge it.
//!
//! At least 9 seeds of 10 must last. #224's bushes are judged the same way,
//! by the same averages, and in at least 9 seeds of 10.
//!
//! Examples include this module via `#[path]`, so each example is its own
//! crate and may not call every helper.
#![allow(dead_code)]

use terra_sim::World;

/// Ticks in an hour at 1×, 1.25 ticks a second (design §6.6).
pub const HOUR: u64 = 4_500;

/// The fewest sprites a world that lasts ever has, at any count of its
/// population record (#222).
pub const FLOOR: usize = 10;

/// The hours at the end whose averages are judged.
pub const LATE_HOURS: u64 = 50;

/// The hours they're set against: the ends of hours 51 to 100.
pub const EARLY_HOURS: (u64, u64) = (50, 100);

/// The fewest hours a run needs for its 50-hour averages to be judged.
pub const JUDGED_FROM: u64 = EARLY_HOURS.1 + LATE_HOURS;

/// A world's sprites, eggs not counted, as it runs: the fewest at any
/// count of its population record (M2 design §14), from its first, and
/// how many at the end of each hour.
#[derive(Debug, Clone, Default)]
pub struct Census {
    pub fewest: usize,
    pub hourly: Vec<usize>,
}

impl Census {
    /// Starts with the record's first count: the sprites `world` starts
    /// with, before its first step.
    pub fn new(world: &World) -> Census {
        Census {
            fewest: world.sprites().count(),
            hourly: Vec::new(),
        }
    }

    /// Counts `world` after a step if its population record counted then,
    /// `every` being the record's ticks between counts before the step (a
    /// squeeze doubles it once the step has counted), and at the end of
    /// each hour.
    pub fn count(&mut self, world: &World, every: u64) {
        self.note(world.tick(), world.sprites().count(), every);
    }

    /// Notes `sprites` at `tick`, after a step, as `count` does.
    pub fn note(&mut self, tick: u64, sprites: usize, every: u64) {
        if tick.is_multiple_of(every) {
            self.fewest = self.fewest.min(sprites);
        }
        if tick.is_multiple_of(HOUR) {
            self.hourly.push(sprites);
        }
    }
}

/// The mean of `hourly`, counted at the ends of hours, over the ends of
/// hours `from` + 1 to `to`; none if the counts don't reach them.
pub fn mean(hourly: &[usize], from: u64, to: u64) -> Option<f64> {
    let hours = hourly.get(from as usize..to as usize)?;
    Some(hours.iter().sum::<usize>() as f64 / hours.len() as f64)
}

/// Whether `hourly`, counted at the end of each hour of a run `hours` long,
/// held: averaged at least half over its last 50 hours of what it did over
/// hours 50–100 (#222, #224). None if the run is shorter than 150 hours, or
/// the counts stop short of its end, as a world's that died out do.
pub fn held(hourly: &[usize], hours: u64) -> Option<bool> {
    if hours < JUDGED_FROM {
        return None;
    }
    let early = mean(hourly, EARLY_HOURS.0, EARLY_HOURS.1)?;
    let late = mean(hourly, hours - LATE_HOURS, hours)?;
    Some(late >= early / 2.0)
}

/// Why a seed's world didn't last (#222), in a few words each: empty if it
/// lasted. `died_out` says it had no sprites and no eggs left; `census`
/// counted its sprites over a run `hours` long.
pub fn not_lasted(died_out: bool, census: &Census, hours: u64) -> Vec<&'static str> {
    [
        (died_out, "died out"),
        (census.fewest < FLOOR, "under 10 sprites"),
        (
            held(&census.hourly, hours) == Some(false),
            "sprites fell by half",
        ),
    ]
    .into_iter()
    .filter_map(|(missed, why)| missed.then_some(why))
    .collect()
}

/// Whether a bar judged across seeds is met when `missed` of the `judged`
/// seeds missed it: at most one seed in ten may (#222, #224), so 9 of 10.
pub fn pooled_met(missed: usize, judged: usize) -> bool {
    10 * missed <= judged
}
