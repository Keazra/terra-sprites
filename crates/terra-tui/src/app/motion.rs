//! Motion's switch, and what the app keeps for it (design §6.9): the looks
//! of the sprites and eggs on the map after each tick it takes in, the
//! deaths and hatchings those ticks set off, and the eggs about to hatch,
//! which flash; the levels of the selected sprite's drives, which its bars
//! ease to, and how much each changed, for their waves; and the tile it's
//! on, for its trail. The tab sweeping in starts as a tab opens (`open`),
//! and a pop-up's fades as the app's clock moves on (`animate`).

use std::time::Duration;

use terra_sim::{ChemicalKind, EntityId, EventKind, Pos, World};

use super::{App, Selection};
use crate::inspector::{self, Hatching};
use crate::motion::{self, DriveLevel, Motion};

impl App {
    /// Whether motion is on (design §6.9): on unless the player turned it
    /// off, as `--no-motion` does.
    pub fn motion(&self) -> bool {
        self.motion.on()
    }

    /// Turns motion on or off (design §6.9). Off, the screen is drawn as
    /// it was before there was motion, and a pan under way ends at once,
    /// the Cursor going to the tile under a still pointer, as when one
    /// ends.
    pub fn set_motion(&mut self, on: bool) {
        if self.motion.switch(on) {
            self.repoint();
        }
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
    /// once (design §6.9). A death is nothing but colour, so where motion's
    /// colours don't play, as in mono, it isn't started, and takes no place
    /// from a hatching, whose shell plays there.
    pub(super) fn set_off(&mut self, event: &EventKind) {
        if matches!(event, EventKind::Died { .. }) && !motion::colours_play(self) {
            return;
        }
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

    /// Notes the eggs on the map within `motion::FLASH_FROM` ticks of
    /// hatching, with no sprite on them, and the ticks each has left, after
    /// the ticks just taken in, for each to flash (design §6.9). The screen
    /// can tell: an egg hatches on the tick its stages end, unless the
    /// Cursor holds it or a sprite stands on it then, and sprites move
    /// after. One a sprite stood on then, which has since stepped off, is
    /// past its tick and hatches on the next, so it has one tick left.
    pub(super) fn note_flashing(&mut self, world: &World) {
        if !self.motion.on() {
            return;
        }
        let due: Vec<(EntityId, u64)> = world
            .eggs()
            .filter(|egg| {
                egg.pos()
                    .is_some_and(|tile| world.sprite_at(tile).is_none())
            })
            .filter_map(|egg| match inspector::hatching(&egg, world) {
                Hatching::In(left) if left <= motion::FLASH_FROM => Some((egg.id(), left)),
                Hatching::OnceClear => Some((egg.id(), 1)),
                _ => None,
            })
            .collect();
        self.motion.note_flashing(&due, self.running_for);
    }

    /// Notes the levels of the selected sprite's drives, after `ticks`
    /// ticks just taken in, for their bars on the Body tab to ease to, and
    /// how much each changed over the last of them, for their waves
    /// (design §6.9).
    pub(super) fn note_bars(&mut self, world: &World, ticks: usize) {
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
                .map(|drive| DriveLevel {
                    name: drive.name.to_string(),
                    thirds: inspector::thirds(drive.level),
                    change: drive.change,
                })
                .collect();
            (sprite.id(), drives)
        });
        let at = (self.running_for, self.clock.speed());
        self.motion
            .note_bars(drives, ticks, at, self.theme.motion().bar);
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

    /// How many thirds of a block drive `drive`'s bar fills on sprite
    /// `id`'s Body tab just now, its level `level`: easing to its level a
    /// third at a time while motion is on (design §6.1, §6.9).
    pub(crate) fn drive_bar(&self, id: EntityId, drive: &str, level: f32) -> u8 {
        let live = inspector::thirds(level);
        self.motion
            .bar(id, drive, live, self.running_for, self.clock.speed())
    }
}
