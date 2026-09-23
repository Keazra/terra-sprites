//! Decides how many simulation ticks to run each frame.

use std::time::Duration;

/// Rates are counted in eighths of a tick per second, so ⅛× (1.25 ticks/s) stays exact.
const EIGHTHS: u128 = 8;
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Simulation speed. 1× is 10 ticks per second; each step halves or doubles it.
/// Max runs as fast as the frame budget allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speed {
    Eighth,
    Quarter,
    Half,
    X1,
    X2,
    X4,
    X8,
    X16,
    Max,
}

impl Speed {
    /// Eighths of a tick per second, or `None` for Max.
    fn eighth_ticks_per_second(self) -> Option<u32> {
        match self {
            Speed::Eighth => Some(10),
            Speed::Quarter => Some(20),
            Speed::Half => Some(40),
            Speed::X1 => Some(80),
            Speed::X2 => Some(160),
            Speed::X4 => Some(320),
            Speed::X8 => Some(640),
            Speed::X16 => Some(1280),
            Speed::Max => None,
        }
    }

    fn faster(self) -> Speed {
        match self {
            Speed::Eighth => Speed::Quarter,
            Speed::Quarter => Speed::Half,
            Speed::Half => Speed::X1,
            Speed::X1 => Speed::X2,
            Speed::X2 => Speed::X4,
            Speed::X4 => Speed::X8,
            Speed::X8 => Speed::X16,
            Speed::X16 | Speed::Max => Speed::Max,
        }
    }

    fn slower(self) -> Speed {
        match self {
            Speed::Eighth | Speed::Quarter => Speed::Eighth,
            Speed::Half => Speed::Quarter,
            Speed::X1 => Speed::Half,
            Speed::X2 => Speed::X1,
            Speed::X4 => Speed::X2,
            Speed::X8 => Speed::X4,
            Speed::X16 => Speed::X8,
            Speed::Max => Speed::X16,
        }
    }
}

/// Paces the simulation against real time.
pub struct Clock {
    speed: Speed,
    paused: bool,
    /// A single step requested while paused, run on the next frame.
    step_requested: bool,
    /// Owed ticks, scaled by 8 × 10⁹ so pacing stays exact in integer maths.
    owed: u128,
}

impl Clock {
    /// A clock running at 1×.
    pub fn new() -> Clock {
        Clock {
            speed: Speed::X1,
            paused: false,
            step_requested: false,
            owed: 0,
        }
    }

    pub fn speed(&self) -> Speed {
        self.speed
    }

    /// One step faster, stopping at Max.
    pub fn faster(&mut self) {
        self.speed = self.speed.faster();
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Pauses or resumes. Time spent paused is never made up afterwards, and a
    /// single step still pending when time resumes is dropped.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.owed = 0;
        self.step_requested = false;
    }

    /// While paused, runs exactly one tick on the next frame. Does nothing while running.
    pub fn step_once(&mut self) {
        if self.paused {
            self.step_requested = true;
        }
    }

    /// One step slower, stopping at ⅛×.
    pub fn slower(&mut self) {
        self.speed = self.speed.slower();
    }

    /// Accounts for `elapsed` real time, calling `step` once per tick that is due.
    ///
    /// `out_of_time` is checked after each tick; once it returns true the frame
    /// stops and any backlog is dropped, so a slow frame can't snowball. At least
    /// one due tick always runs. Returns the number of ticks run.
    pub fn advance(
        &mut self,
        elapsed: Duration,
        mut step: impl FnMut(),
        mut out_of_time: impl FnMut() -> bool,
    ) -> u64 {
        if self.paused {
            if std::mem::take(&mut self.step_requested) {
                step();
                return 1;
            }
            return 0;
        }
        let due = match self.speed.eighth_ticks_per_second() {
            Some(rate) => {
                self.owed += elapsed.as_nanos() * u128::from(rate);
                let due = self.owed / (NANOS_PER_SECOND * EIGHTHS);
                self.owed %= NANOS_PER_SECOND * EIGHTHS;
                due
            }
            // Max: as many ticks as the frame budget allows.
            None => u128::MAX,
        };
        let mut ran = 0;
        while ran < due {
            step();
            ran += 1;
            if out_of_time() {
                break;
            }
        }
        ran as u64
    }
}

impl Default for Clock {
    fn default() -> Self {
        Clock::new()
    }
}
