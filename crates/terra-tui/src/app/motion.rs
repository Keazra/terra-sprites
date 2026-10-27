//! Motion's switch, and what the app keeps for it (design §6.9): the looks
//! of the sprites and eggs on the map after each tick it takes in, the
//! deaths and hatchings those ticks set off, and the eggs about to hatch,
//! which crack; the levels of the selected sprite's drives, which its bars
//! ease to; and the tile it's on, for its trail. The tab sweeping in starts
//! as a tab opens (`open`), and a pop-up's fades as the app's clock moves
//! on (`animate`).

use std::time::Duration;

use terra_sim::{ChemicalKind, EntityId, EventKind, Pos, World};

use super::{App, Selection};
use crate::inspector::{self, Hatching};
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

    /// Starts the death or hatching `event` sets off, if it does: only
    /// those that could be drawn just now count towards how many play at
    /// once (design §6.9).
    pub(super) fn set_off(&mut self, event: &EventKind) {
        let drawn: Vec<(EntityId, Pos)> = self
            .motion
            .at_stake(event)
            .into_iter()
            .filter(|&(id, tile)| motion::could_draw(self, id, tile))
            .collect();
        let could_draw = |id, tile| drawn.contains(&(id, tile));
        self.motion
            .took_in(event, self.running_for, self.clock.speed(), could_draw);
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

    /// Notes the eggs on the map about to hatch on the next tick, after the
    /// ticks just taken in, for each to crack (design §6.9). The screen can
    /// tell: an egg hatches on the tick its stages end, unless the Cursor
    /// holds it or a sprite stands on it then, and sprites move after.
    pub(super) fn note_cracks(&mut self, world: &World) {
        if !self.motion.on() {
            return;
        }
        let due: Vec<EntityId> = world
            .eggs()
            .filter(|egg| {
                egg.pos()
                    .is_some_and(|tile| world.sprite_at(tile).is_none())
                    && matches!(inspector::hatching(egg, world), Hatching::In(1))
            })
            .map(|egg| egg.id())
            .collect();
        self.motion
            .note_cracks(&due, self.running_for, self.clock.speed());
    }

    /// Notes the levels of the selected sprite's drives, after the ticks
    /// just taken in, for their bars on the Body tab to ease to (design
    /// §6.9).
    pub(super) fn note_bars(&mut self, world: &World) {
        if !self.motion.on() {
            return;
        }
        let sprite = match self.selection {
            Some(Selection::Living(id)) => world.sprite(id),
            _ => None,
        };
        let drives = sprite.map(|sprite| {
            let drives = sprite
                .chemicals()
                .filter(|chemical| chemical.kind == ChemicalKind::Drive)
                .map(|drive| (drive.name.to_string(), inspector::blocks(drive.level)))
                .collect();
            (sprite.id(), drives)
        });
        self.motion
            .note_bars(drives, self.running_for, self.clock.speed());
    }

    /// Notes the tile the selected sprite is on, after the ticks just taken
    /// in, for the trail it leaves (design §6.9).
    pub(super) fn note_trail(&mut self, world: &World) {
        if !self.motion.on() {
            return;
        }
        let sprite = match self.selection {
            Some(Selection::Living(id)) => world.sprite(id).map(|sprite| (id, sprite.pos())),
            _ => None,
        };
        self.motion
            .note_trail(sprite, self.running_for, self.clock.speed());
    }

    /// How many blocks drive `drive`'s bar fills on sprite `id`'s Body tab
    /// just now, its level `level`: easing to its level while motion is on
    /// (design §6.9).
    pub(crate) fn drive_bar(&self, id: EntityId, drive: &str, level: f32) -> u8 {
        let live = inspector::blocks(level);
        self.motion
            .bar(id, drive, live, self.running_for, self.clock.speed())
    }
}
