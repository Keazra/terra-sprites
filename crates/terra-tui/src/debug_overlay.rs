//! The debug overlay (`F3`, design §6.1, #311): how the game is running on
//! this computer, in the map view's top-right corner, so "it feels slow"
//! becomes numbers in a screenshot. `main.rs` times each frame and hands the
//! times in; the overlay reads them about once a second. The speed example
//! (A5) measures the simulation alone, where this measures the game as it's
//! played, drawing included.

use std::time::Duration;

use crate::clock::{Clock, SIM_BUDGET, Speed};
use crate::input::KeyReports;
use crate::text::group_thousands;

/// How long the meter gathers frames before it reads them: about a second,
/// so the numbers hold still long enough to read and stay fresh.
const READ_EVERY: Duration = Duration::from_secs(1);

/// How wide the overlay is, border included: room for its widest line,
/// whatever the numbers, so it doesn't change size as they do.
pub(crate) const WIDTH: u16 = 29;

/// What `main.rs` measured of one frame (design §6.6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameTimes {
    /// Real time since the frame before.
    pub elapsed: Duration,
    /// Ticks the simulation ran in it.
    pub ticks: u64,
    /// The time those ticks took: what the frame's budget counts.
    pub simulating: Duration,
    /// The time drawing the frame took.
    pub drawing: Duration,
}

/// The overlay: whether it's shown, and the frames and keys it tells of.
#[derive(Debug, Clone, Default)]
pub(crate) struct DebugOverlay {
    shown: bool,
    meter: Meter,
    keys: KeyReports,
}

impl DebugOverlay {
    pub(crate) fn shown(&self) -> bool {
        self.shown
    }

    /// Shows it, or hides it.
    pub(crate) fn toggle(&mut self) {
        self.shown = !self.shown;
    }

    /// Takes in one frame's times, and what the terminal reports of keys.
    pub(crate) fn note(&mut self, times: FrameTimes, keys: KeyReports) {
        self.meter.note(times);
        self.keys = keys;
    }

    /// Its lines, inside its border, at the speed `clock` asks for.
    pub(crate) fn lines(&self, clock: &Clock) -> Vec<String> {
        let reading = self.meter.reading;
        let achieved = reading.map_or("-".to_string(), |reading| {
            achieved_rate(reading.ticks_per_second)
        });
        let asked = match clock.speed().ticks_per_second() {
            _ if clock.is_paused() => "paused".to_string(),
            Some(rate) => format!("of {}", asked_rate(rate)),
            None => format!("at {}", Speed::Max.label()),
        };
        let budget = SIM_BUDGET.as_millis();
        let (simulating, drawing) = match reading {
            Some(reading) => (
                milliseconds(reading.simulating),
                format!(
                    "{} ms, {:.0} fps",
                    milliseconds(reading.drawing),
                    reading.frames_per_second
                ),
            ),
            None => ("-".to_string(), "-".to_string()),
        };
        vec![
            format!(" ticks/s {achieved} {asked}"),
            format!(" sim     {simulating} of {budget} ms"),
            format!(" draw    {drawing}"),
            format!(" keys    {}", self.keys.label()),
        ]
    }
}

/// The frames of the last second or so, read.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Reading {
    ticks_per_second: f64,
    frames_per_second: f64,
    /// The simulation's time in a frame, on average.
    simulating: Duration,
    /// The drawing's time in a frame, on average.
    drawing: Duration,
}

/// Gathers frames' times, and reads them once a second's worth is in.
#[derive(Debug, Clone, Default)]
struct Meter {
    frames: u32,
    gathered: FrameTimes,
    /// The latest reading, once a second's worth of frames is in.
    reading: Option<Reading>,
}

impl Meter {
    /// Takes in one frame's times; a second's worth in, reads them and
    /// starts again.
    fn note(&mut self, frame: FrameTimes) {
        self.frames += 1;
        let gathered = &mut self.gathered;
        gathered.elapsed += frame.elapsed;
        gathered.ticks += frame.ticks;
        gathered.simulating += frame.simulating;
        gathered.drawing += frame.drawing;
        if gathered.elapsed < READ_EVERY {
            return;
        }
        let seconds = gathered.elapsed.as_secs_f64();
        self.reading = Some(Reading {
            ticks_per_second: gathered.ticks as f64 / seconds,
            frames_per_second: f64::from(self.frames) / seconds,
            simulating: gathered.simulating / self.frames,
            drawing: gathered.drawing / self.frames,
        });
        (self.frames, self.gathered) = (0, FrameTimes::default());
    }
}

/// Ticks a second achieved: to a tenth, and whole from 100 up.
fn achieved_rate(rate: f64) -> String {
    if rate < 100.0 {
        format!("{rate:.1}")
    } else {
        group_thousands(rate.round() as u64)
    }
}

/// The ticks a second a speed asks for: whole where it's whole, as 20 at
/// 16×, and else to two places, as 1.25 at 1×.
fn asked_rate(rate: f64) -> String {
    if rate.fract() == 0.0 {
        format!("{rate:.0}")
    } else {
        let places = format!("{rate:.2}");
        places.trim_end_matches('0').to_string()
    }
}

/// A frame's time, in milliseconds to a tenth.
fn milliseconds(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1000.0)
}
