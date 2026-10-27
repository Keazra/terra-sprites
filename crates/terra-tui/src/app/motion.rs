//! Motion's switch, and what the app keeps for it (design §6.9): the looks
//! of the sprites and eggs on the map after each tick it takes in, and the
//! deaths and hatchings those ticks set off.

use std::time::Duration;

use terra_sim::{EventKind, World};

use super::App;
use crate::motion::{self, Motion};

impl App {
    /// Whether motion is on (design §6.9): on unless the player turned it
    /// off, as `--no-motion` does.
    pub fn motion(&self) -> bool {
        self.motion.on()
    }

    /// Turns motion on or off (design §6.9). Off, the screen is drawn as
    /// it was before there was motion.
    pub fn set_motion(&mut self, on: bool) {
        self.motion.switch(on);
    }

    /// What the app keeps for motion, for the screen to draw it.
    pub(crate) fn motion_kept(&self) -> &Motion {
        &self.motion
    }

    /// How long the app has run, in the real time the frame loop hands it
    /// (design §6.6): motion's clock (§6.9).
    pub(crate) fn running_for(&self) -> Duration {
        self.running_for
    }

    /// Starts the death or hatching `event` sets off, if it does.
    pub(super) fn set_off(&mut self, event: &EventKind) {
        self.motion
            .took_in(event, self.running_for, self.clock.speed());
    }

    /// Notes how each sprite and egg on `world`'s map is drawn, after the
    /// ticks just taken in, while motion is on: the world removes a dead
    /// sprite before it says so, so the screen remembers where it was
    /// (design §6.9).
    pub(super) fn note_looks(&mut self, world: &World) {
        if self.motion.on() {
            let seen = motion::seen(self, world);
            self.motion.note(seen);
        }
    }
}
