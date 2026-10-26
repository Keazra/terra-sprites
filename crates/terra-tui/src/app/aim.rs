//! Aiming a throw or a shove (design v25 §6.5): a right press starts it,
//! the pull sets the way and how far, and letting go sends it.

use terra_sim::{Command, Dir, EntityId, Grip, Pos, World};

use super::cursor::{Reach, TO_SEND, item_look};
use super::{Aim, App};
use crate::inspector;
use crate::text::display_name;

impl App {
    /// A Grab-mode right click on `tile`, with `sprite` on it (design v25
    /// §6.5): it starts aiming what the Cursor has hold of, the pull measured
    /// from the Cursor to the pointer. With the Cursor empty, it grabs what's
    /// there first, as a left click would. Aiming already, it sends the aim,
    /// as `E` pressed again does in a terminal that doesn't report a key let
    /// go. An item its type says is never thrown, such as an egg, held or
    /// about to be picked up, aims nothing and is picked up by nothing (M2
    /// design §10.4).
    pub(super) fn aim_click(&mut self, tile: Pos, sprite: Option<EntityId>, world: &World) {
        if self.aim.is_some() {
            self.send_aimed(world);
            return;
        }
        let thrown = match self.grip(world) {
            Some(Grip::Holds(item)) => item_look(world, item),
            Some(Grip::Leads(_)) => None,
            None => match self.reach(tile, sprite, world) {
                Some(Reach::Item(item)) => Some(item),
                Some(Reach::Sprite(_)) | None => None,
            },
        };
        if let Some(item) = thrown.filter(|item| !item.throwable) {
            let name = inspector::with_article(&display_name(item.type_name));
            self.refuse(format!("Can't throw {name}"));
            return;
        }
        if self.grip(world).is_none() && !self.grab(tile, sprite, world, TO_SEND) {
            return;
        }
        // Leading, the Cursor goes onto the sprite, which then stands still,
        // since a led sprite walks towards the Cursor (design v25 §6.5).
        if let Some(Grip::Leads(id)) = self.grip(world)
            && let Some(led) = world.sprite(id)
        {
            self.cursor = led.pos();
        }
        let pointed = (i32::from(self.pointed.x), i32::from(self.pointed.y));
        let to = self.pointer.and_then(|cell| self.pointed_xy(cell));
        self.aim = Some(Aim {
            from: self.cursor,
            to: to.unwrap_or(pointed),
        });
    }

    /// Sends what the player aimed (design v25 §6.5): the thing goes the
    /// opposite way to the pull, from the Cursor to the pointer, snapped to
    /// the nearest of the 8 directions, as far as the pull, in the game's own
    /// measure. With the pointer on the Cursor, it sends nothing.
    pub(super) fn send_aimed(&mut self, world: &World) {
        let (aim, aimed) = (self.aim, self.aimed());
        self.end_aim();
        let (Some(Aim { from, .. }), Some((toward, tiles))) = (aim, aimed) else {
            return;
        };
        let command = match self.grip(world) {
            Some(Grip::Holds(_)) => Command::Throw {
                from,
                toward,
                tiles,
            },
            Some(Grip::Leads(_)) => Command::Shove { toward, tiles },
            None => return,
        };
        self.send(command, world);
    }

    /// While the player aims, the way the thing will go and how far, as
    /// the pull says (design v25 §6.5): the opposite way to the pull, from
    /// the Cursor to the pointer, snapped to the nearest of the 8
    /// directions, as far as the pull, in the game's own measure. `None` for
    /// a pull of nothing. Past the wall, it pulls on as if the map did (M2
    /// design §11.4).
    fn aimed(&self) -> Option<(Dir, u16)> {
        let (x, y) = self.aim?.to;
        let (dx, dy) = (i32::from(self.cursor.x) - x, i32::from(self.cursor.y) - y);
        let toward = Dir::nearest(dx, dy)?;
        let tiles = dx.unsigned_abs().max(dy.unsigned_abs());
        Some((toward, u16::try_from(tiles).unwrap_or(u16::MAX)))
    }

    /// Ends the aim, sent or not: the Cursor follows the pointer again, as
    /// far as Follow and the leash let it (design v25 §6.5). With the
    /// pointer past the wall, it stays where it is, out of reach, so a
    /// sprite it leads stands still (M2 design §11.4).
    pub(super) fn end_aim(&mut self) {
        if self.aim.take().is_some() && self.followed().is_none() && !self.pointer_past_wall {
            self.cursor = self.within_leash(self.pointed);
        }
    }

    /// Whether the player is aiming a throw or a shove (design v25 §6.5).
    pub fn aiming(&self) -> bool {
        self.aim.is_some()
    }

    /// While the player aims a held item, it and where it will be thrown
    /// from, under the Cursor, where it's drawn until it's thrown (design
    /// v25 §6.5).
    pub fn thrown_from(&self, world: &World) -> Option<(EntityId, Pos)> {
        match (self.aim, self.grip(world)) {
            (Some(Aim { from, .. }), Some(Grip::Holds(item))) => Some((item, from)),
            _ => None,
        }
    }

    /// While the player aims, the tiles the thing will cross, from the one
    /// after where it is to where it would stop if nothing's in the way:
    /// as far as the pull sends it, no further than the Cursor can send it,
    /// and not past the wall (design v25 §6.5). Empty otherwise. A held item
    /// starts where aiming began, and a led sprite where it stands.
    pub fn aim_line(&self, world: &World) -> Vec<Pos> {
        let (Some(Aim { from, .. }), Some((toward, tiles)), Some(grip)) =
            (self.aim, self.aimed(), self.grip(world))
        else {
            return Vec::new();
        };
        let start = match grip {
            Grip::Holds(_) => Some(from),
            Grip::Leads(id) => world.sprite(id).map(|sprite| sprite.pos()),
        };
        let Some(mut at) = start else {
            return Vec::new();
        };
        let mut line = Vec::new();
        for _ in 0..tiles.min(world.furthest(grip)) {
            match world.map().neighbour(at, toward) {
                Some(next) => at = next,
                None => break,
            }
            line.push(at);
        }
        line
    }
}
