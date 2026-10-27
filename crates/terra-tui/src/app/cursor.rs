//! The Cursor (design v23 §6.5): where it is, the sprite it follows (v26)
//! or leads on its leash, what it holds, what the world is told of it, and
//! what a click on the map does in each cursor mode.

use std::collections::BTreeMap;

use terra_sim::{Command, CursorTouch, EntityId, Grip, ObjectView, Pos, World};

use super::{App, CursorMode, Selection};
use crate::input::Button;
use crate::text::{ROOTED, display_name};

/// An item as the screen shows it, on the map or held by the Cursor.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ItemLook<'w> {
    pub(crate) id: EntityId,
    pub(crate) type_name: &'w str,
    pub(crate) visual_state: &'w str,
    /// Its stage, if its type has stages.
    pub(crate) stage: Option<&'w str>,
    /// A fixture, rooted to the ground (design §3.3).
    pub(crate) solid: bool,
    pub(crate) egg: bool,
    /// False for an egg (M2 design §10.4).
    pub(crate) throwable: bool,
}

impl<'w> ItemLook<'w> {
    /// How the screen shows `object`, on the map or held.
    fn of(object: &ObjectView<'w>) -> ItemLook<'w> {
        ItemLook {
            id: object.id(),
            type_name: object.type_name(),
            visual_state: object.visual_state(),
            stage: object.stage(),
            solid: object.is_solid(),
            egg: object.is_egg(),
            throwable: object.is_throwable(),
        }
    }
}

/// The item `id`, held by the Cursor or, while a pick-up waits in the queue,
/// still on the map (design §6.5).
pub(crate) fn item_look(world: &World, id: EntityId) -> Option<ItemLook<'_>> {
    let held = world.cursor().holds().filter(|held| held.id() == id);
    let object = held.or_else(|| world.objects().find(|object| object.id() == id))?;
    Some(ItemLook::of(&object))
}

/// Where a Grab-mode command waiting for the next tick leaves an item it
/// moves (design §6.5, #203).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Moved {
    PickedUp,
    PutDown(Pos),
}

/// The items on the map as the screen shows them, with the Grab-mode
/// commands waiting for the next tick done, as the Cursor's marks show them
/// (design §6.5, #203): an item a pick-up takes is gone from its tile, and
/// one put down or thrown shows where it lands. If the world refuses one of
/// them, they go back to the world's word.
pub(crate) struct ShownItems<'w> {
    world: &'w World,
    moved: BTreeMap<EntityId, Moved>,
    /// Each item put down, and the tile it's put down on.
    put_down: Vec<(Pos, ItemLook<'w>)>,
}

impl<'w> ShownItems<'w> {
    /// The item shown on `tile`, if any.
    pub(crate) fn at(&self, tile: Pos) -> Option<ItemLook<'w>> {
        let on_map = self.world.object_at(tile);
        let stays = on_map.filter(|object| !self.moved.contains_key(&object.id()));
        stays.map(|object| ItemLook::of(&object)).or_else(|| {
            let (_, look) = self.put_down.iter().find(|&&(at, _)| at == tile)?;
            Some(*look)
        })
    }
}

/// What the Cursor will have hold of, and where the items it moves will
/// be, once the world applies the Grab-mode commands waiting for the next
/// tick (design §6.5).
struct AfterQueue {
    grip: Option<Grip>,
    moved: BTreeMap<EntityId, Moved>,
}

/// What a grab with the Cursor empty takes (design §6.5).
pub(super) enum Reach<'w> {
    Sprite(EntityId),
    Item(ItemLook<'w>),
}

/// How far the Cursor goes from a sprite it leads, in tiles, in a square: a
/// UI setting (design v23 §6.5).
const LEASH: u16 = 5;

/// How a grab with nothing to grab is refused: "Nothing here to grab", or
/// on a fixture, "Can't grab the berry bush" (design v22 §6.1).
pub(super) struct Refused {
    nothing_to: &'static str,
    cant: &'static str,
}

/// A left click's grab.
const TO_GRAB: Refused = Refused {
    nothing_to: "grab",
    cant: "grab",
};

/// A right press's grab, to aim what it grabs (design v25 §6.1).
pub(super) const TO_SEND: Refused = Refused {
    nothing_to: "throw or shove",
    cant: "throw",
};

impl App {
    /// Settles the Cursor after an action or a tick: what it leads, then
    /// where it is, then telling the world if it leads a sprite.
    pub(super) fn settle_cursor(&mut self, world: &World) {
        self.track_selected(world);
        self.led = match self.grip(world) {
            Some(Grip::Leads(id)) => world.sprite(id).map(|sprite| (id, sprite.pos())),
            _ => None,
        };
        self.track(world);
        self.tell(world);
    }

    /// While the Cursor leads a sprite, the tile it stands on: where the
    /// leash runs to from the Cursor (design v23 §6.5).
    pub fn leash(&self) -> Option<Pos> {
        self.led.map(|(_, at)| at)
    }

    /// The tile within the leash nearest `tile`: while the Cursor leads a
    /// sprite, it goes no further from it than `LEASH` tiles, in a square
    /// (design v23 §6.5).
    pub(super) fn within_leash(&self, tile: Pos) -> Pos {
        let Some((_, at)) = self.led else {
            return tile;
        };
        let near =
            |to: u16, from: u16| to.clamp(from.saturating_sub(LEASH), from.saturating_add(LEASH));
        Pos {
            x: near(tile.x, at.x),
            y: near(tile.y, at.y),
        }
    }

    /// While the Cursor leads a sprite, tells the world each new tile it
    /// moves onto, in every mode (design v23 §6.5): the led sprite heads
    /// there. So too while sprites can see it, where they see it; and
    /// whether they can, as the cursor mode changes (design v29 §6.5).
    fn tell(&mut self, world: &World) {
        let leading = matches!(self.grip(world), Some(Grip::Leads(_)));
        let visible = self.visible();
        if !leading && !visible {
            self.told_tile = None;
        } else if self.told_tile != Some(self.cursor) {
            self.commands
                .push(Command::MoveCursor { tile: self.cursor });
            self.told_tile = Some(self.cursor);
        }
        if self.told_visible != visible {
            self.commands.push(Command::ShowCursor { visible });
            self.told_visible = visible;
        }
    }

    /// Whether sprites can see the Cursor: the current cursor mode's switch
    /// (design v29 §6.5).
    pub fn visible(&self) -> bool {
        self.visible_in.contains(&self.mode)
    }

    /// What the Cursor has hold of as the player sees it: what the world
    /// says, as the commands queued since will leave it (design v23 §6.5).
    /// If the world refuses one, this goes back to the world's word.
    /// In a replay the Cursor is the player's own, which holds nothing: the
    /// recorded one isn't drawn (design §2.7).
    pub fn grip(&self, world: &World) -> Option<Grip> {
        self.after_queue(world).grip
    }

    /// The items on the map as the screen shows them, with the commands
    /// queued since done (design §6.5, #203).
    pub(crate) fn shown_items<'w>(&self, world: &'w World) -> ShownItems<'w> {
        let moved = self.after_queue(world).moved;
        let put_down = moved
            .iter()
            .filter_map(|(&id, &moved)| match moved {
                Moved::PutDown(tile) => Some((tile, item_look(world, id)?)),
                Moved::PickedUp => None,
            })
            .collect();
        ShownItems {
            world,
            moved,
            put_down,
        }
    }

    /// What the Cursor will have hold of, and where the items it moves will
    /// be, once the world applies the commands queued since. Nothing, in a
    /// replay.
    fn after_queue(&self, world: &World) -> AfterQueue {
        let mut moved = BTreeMap::new();
        if self.replay.is_some() {
            return AfterQueue { grip: None, moved };
        }
        let now = world.cursor();
        let held = now.holds().map(|item| Grip::Holds(item.id()));
        let start = now.leads().map(Grip::Leads).or(held);
        let grip = self
            .queued
            .iter()
            .fold(start, |grip, (_, command)| match *command {
                Command::TakeHold { sprite } => grip.or(Some(Grip::Leads(sprite))),
                Command::PickUp { item } if grip.is_none() => {
                    moved.insert(item, Moved::PickedUp);
                    Some(Grip::Holds(item))
                }
                Command::PickUp { .. } => grip,
                // A thrown item is put down where it's thrown from, and
                // rolls on from there in the tick (design §2.5).
                Command::PutDown { tile } | Command::Throw { from: tile, .. } => {
                    if let Some(Grip::Holds(item)) = grip {
                        moved.insert(item, Moved::PutDown(tile));
                    }
                    None
                }
                Command::LetGo | Command::Shove { .. } => None,
                Command::Reward { .. }
                | Command::Correct { .. }
                | Command::MoveCursor { .. }
                | Command::ShowCursor { .. }
                | Command::Place { .. }
                | Command::SpawnSprite { .. }
                | Command::Rename { .. } => grip,
            });
        AfterQueue { grip, moved }
    }

    /// The sprite the Cursor follows, if any (design v26 §6.5). While the
    /// Cursor leads that sprite, Follow steps aside, in every mode, and the
    /// Cursor follows the pointer, within the leash (design v23 §6.5).
    pub fn followed(&self) -> Option<EntityId> {
        if self.follow_waits() {
            return None;
        }
        self.follow
    }

    /// Whether Follow steps aside: while the Cursor leads the followed
    /// sprite (design v23 §6.5).
    fn follow_waits(&self) -> bool {
        matches!((self.follow, self.led), (Some(followed), Some((led, _))) if followed == led)
    }

    /// Keeps the Cursor on the sprite it follows, wherever it has walked; and
    /// a leading one on the pointer, so a keyboard click lands where the
    /// player points. Either way, within the leash (design v23 §6.5). While
    /// the player aims, it stays on what it aims: a led sprite, which a
    /// slide may carry on, or where a held item will be thrown from (design
    /// v25 §6.5).
    fn track(&mut self, world: &World) {
        let wanted = if self.aim.is_some() {
            self.leash()
        } else {
            match self.followed().and_then(|id| world.sprite(id)) {
                Some(sprite) => Some(sprite.pos()),
                // Out of reach past the wall, it stays where it is, so the
                // sprite it leads isn't led on (M2 design §11.4).
                None if self.led.is_some() && !self.out_of_reach() => Some(self.pointed),
                None => None,
            }
        };
        if let Some(tile) = wanted {
            self.cursor = self.within_leash(tile);
        }
    }

    /// The tile the Cursor is on: its followed sprite's, or else the
    /// pointer's.
    pub fn cursor(&self) -> Pos {
        self.cursor
    }

    /// Whether the Cursor is out of reach past the wall (M2 design §11.4):
    /// it follows the pointer, and the pointer points at the glass or
    /// space. It stays on its last tile but isn't drawn, and a click, `Q`
    /// or `E` does nothing. A Cursor that follows a sprite, or sits on what
    /// the player aims, carries on as before.
    pub fn out_of_reach(&self) -> bool {
        self.pointer_past_wall && self.followed().is_none() && self.aim.is_none()
    }

    /// Withdraws the Cursor: out of reach, as a pointer over the glass or
    /// space takes it, with no pointer anywhere (M2 design §11.4). For
    /// pictures of the game with no player in them, such as the renderer's
    /// clips (#241). The pointer coming back over the map brings it back.
    pub fn withdraw_cursor(&mut self) {
        self.pointer = None;
        self.pointer_past_wall = true;
    }

    /// Whether the Cursor is withdrawn (`withdraw_cursor`). Only that
    /// leaves it past the wall with no pointer: a pointer past the wall is
    /// over the glass or space.
    pub fn cursor_withdrawn(&self) -> bool {
        self.pointer.is_none() && self.out_of_reach()
    }

    /// The tile the Cursor is drawn on, unless it's out of reach past the
    /// wall, where it isn't drawn at all (M2 design §11.4), or hidden
    /// under the inspector open over the map view, where it follows the
    /// pointer (design §6.1, #305). `Q`, `E` and `F` reach it only here.
    pub fn shown_cursor(&self) -> Option<Pos> {
        let hidden = self.followed().is_none()
            && self.aim.is_none()
            && self.under_the_inspector(self.cursor);
        (!self.out_of_reach() && !hidden).then_some(self.cursor)
    }

    /// What a click with `button` on `tile` does, in the cursor mode (design
    /// v21 §6.5).
    ///
    /// In Select, a left click selects the sprite there, or else the egg
    /// (M2 design §10.7); on empty ground it clears the selection, which
    /// leaves Follow as it is. A right
    /// click activates what's there, and nothing can be activated yet
    /// (design v26 §6.5).
    ///
    /// In Train, a left click rewards the target and a right click corrects
    /// it: the followed sprite, or else the one there. With none, nothing is
    /// sent.
    pub(super) fn act(&mut self, tile: Pos, button: Button, amplified: bool, world: &World) {
        let sprite = world.sprite_at(tile).map(|sprite| sprite.id());
        match (self.mode, button) {
            (CursorMode::Select, Button::Left) => {
                // As the map shows it, a queued put-down done (#203).
                let egg = || self.shown_items(world).at(tile).filter(|item| item.egg);
                match sprite.or_else(|| egg().map(|egg| egg.id)) {
                    Some(id) => self.select(id),
                    None => self.selection = None,
                }
            }
            // Until there are devices, nothing can be activated (design v26
            // §6.5).
            (CursorMode::Select, Button::Right) => {
                self.refuse("Nothing here to activate".into());
            }
            // A Place menu item waiting on the Cursor takes the next click,
            // where the Cursor is: at the followed sprite's feet, as a held
            // item is put down, or else on the tile clicked. A right click
            // puts it away (design v28 §6.5).
            (CursorMode::Grab, Button::Left) if self.placing.is_some() => {
                let tile = if self.followed().is_some() {
                    self.cursor
                } else {
                    tile
                };
                self.place(tile);
            }
            (CursorMode::Grab, Button::Right) if self.placing.is_some() => self.placing = None,
            (CursorMode::Grab, Button::Left) => self.grab_click(tile, sprite, world),
            (CursorMode::Grab, Button::Right) => self.aim_click(tile, sprite, world),
            (CursorMode::Train, button) => {
                let touch = match (button, amplified) {
                    (Button::Left, false) => CursorTouch::Pet,
                    (Button::Left, true) => CursorTouch::Hug,
                    (Button::Right, false) => CursorTouch::Zap,
                    (Button::Right, true) => CursorTouch::Shock,
                };
                // With nothing to act on, nothing is sent, so `?` flashes at
                // once, and the status line says why (design v22 §6.5).
                let Some(sprite) = self.followed().or(sprite) else {
                    self.refuse(format!("No sprite here to {}", touch.name()));
                    return;
                };
                self.commands
                    .push(touch.command(sprite, self.clock.reach_back()));
                // The marks follow the latest click: an earlier click's
                // report, not shown yet, is behind it (design v22 §6.5).
                self.sent_at = Some(self.running_for);
                self.report = None;
            }
        }
    }

    /// A Grab-mode click on `tile`, with `sprite` on it (design v23 §6.5).
    /// Leading, it lets go; holding, it puts the item down there, or at the
    /// followed sprite's feet, wherever the click lands (design v26 §6.5).
    /// Empty, it takes hold of the followed sprite, or else the sprite there,
    /// or else picks up the item there; a fixture is rooted to the ground.
    fn grab_click(&mut self, tile: Pos, sprite: Option<EntityId>, world: &World) {
        match self.grip(world) {
            Some(Grip::Leads(_)) => self.send(Command::LetGo, world),
            Some(Grip::Holds(_)) => {
                let tile = if self.followed().is_some() {
                    self.cursor
                } else {
                    tile
                };
                self.send(Command::PutDown { tile }, world);
            }
            None => {
                self.grab(tile, sprite, world, TO_GRAB);
            }
        }
    }

    /// Lets go of what the Cursor has hold of, where the Cursor is, as `Esc`
    /// does (design v33 §6.5): a led sprite is let go, and a held item put
    /// down on the Cursor's tile, as a click there would.
    pub(super) fn let_go(&mut self, world: &World) {
        match self.grip(world) {
            Some(Grip::Leads(_)) => self.send(Command::LetGo, world),
            Some(Grip::Holds(_)) => self.send(Command::PutDown { tile: self.cursor }, world),
            None => {}
        }
    }

    /// With the Cursor empty, grabs what's on `tile`, with `sprite` on it
    /// (design v23 §6.5): takes hold of the followed sprite, or else the
    /// sprite there, or else picks up the item there; a fixture is rooted to
    /// the ground, and with nothing there, it's `refused`. Says whether it
    /// grabbed anything.
    pub(super) fn grab(
        &mut self,
        tile: Pos,
        sprite: Option<EntityId>,
        world: &World,
        refused: Refused,
    ) -> bool {
        let command = match self.reach(tile, sprite, world) {
            Some(Reach::Sprite(sprite)) => {
                // Taking hold puts the Cursor, as the world knows it, on the
                // sprite.
                self.told_tile = world.sprite(sprite).map(|s| s.pos());
                Command::TakeHold { sprite }
            }
            // In M1 every solid object is a fixture (design §3.3).
            Some(Reach::Item(item)) if item.solid => {
                let (cant, name) = (refused.cant, display_name(item.type_name));
                self.refuse(format!("Can't {cant} the {name}: {ROOTED}"));
                return false;
            }
            Some(Reach::Item(item)) => Command::PickUp { item: item.id },
            None => {
                self.refuse(format!("Nothing here to {}", refused.nothing_to));
                return false;
            }
        };
        self.send(command, world);
        true
    }

    /// What a grab with the Cursor empty takes on `tile`, with `sprite` on
    /// it (design §6.5): the followed sprite, or else that one, or else the
    /// item there.
    pub(super) fn reach<'w>(
        &self,
        tile: Pos,
        sprite: Option<EntityId>,
        world: &'w World,
    ) -> Option<Reach<'w>> {
        match self.followed().or(sprite) {
            Some(sprite) => Some(Reach::Sprite(sprite)),
            // As the map shows it, a queued put-down done (#203).
            None => self.shown_items(world).at(tile).map(Reach::Item),
        }
    }

    /// Sends a Grab-mode command, queued, so the marks and the next click
    /// follow it (design v23 §6.5).
    pub(super) fn send(&mut self, command: Command, world: &World) {
        self.commands.push(command.clone());
        self.queued.push((world.tick(), command));
        // The marks follow the latest click (design v22 §6.5).
        self.report = None;
    }

    /// `F`, or a middle click, on `tile`, in every mode (design v26 §6.5),
    /// or on no tile, past the wall (M2 design §11.4). Following, it stops,
    /// wherever `tile` is. Otherwise the Cursor follows the sprite on
    /// `tile`, or else the selected sprite, leaving the selection as it is;
    /// with neither, it's refused. An egg isn't followed.
    pub(super) fn toggle_follow(&mut self, tile: Option<Pos>, world: &World) {
        if self.follow.is_some() {
            self.follow = None;
            // It follows the pointer again, unless the player is aiming:
            // then it stays on what it aims until the aim ends (design v25
            // §6.5).
            if self.aim.is_none() {
                self.cursor = self.pointed;
            }
            return;
        }
        let selected = match self.selection {
            Some(Selection::Living(id)) => world.sprite(id).map(|sprite| sprite.id()),
            _ => None,
        };
        let there = tile.and_then(|tile| world.sprite_at(tile));
        match there.map(|sprite| sprite.id()).or(selected) {
            Some(id) => self.follow = Some(id),
            None => self.refuse("No sprite here to follow".into()),
        }
    }
}
