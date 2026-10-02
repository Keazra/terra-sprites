//! The player's commands (design §2.5): what the Cursor does to the world,
//! applied at step 1 of the tick they're stamped for.

use serde::{Deserialize, Serialize};

use crate::action::{self, Outcome, Walk};
use crate::cursor::Grip;
use crate::data::DataPack;
use crate::ecology::{holds_without_drawing, new_object};
use crate::events::{Event, EventKind};
use crate::genome::Genome;
use crate::map::{Dir, Pos};
use crate::names::{self, NameProblem};
use crate::object_types::{Condition, Size};
use crate::objects::{EntityId, Roll};
use crate::perception::Target;
use crate::sliding::Slide;
use crate::sprites::Sprite;
use crate::terrain::Terrain;
use crate::variation::varied;
use crate::world::WorldState;

/// Something the player does to the world through the Cursor. It carries
/// values, never references (design §2.5).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Command {
    /// The Cursor's good touch (design v21 §4.6): a pet, or amplified, a hug.
    Reward {
        sprite: EntityId,
        amplified: bool,
        /// How many ticks back its feeling looks for the sprite's latest
        /// attempt (design v21 §5.6).
        reach_back: u64,
    },
    /// The Cursor's bad touch (design v21 §4.6): a zap, or amplified, a
    /// shock. It hurts without injuring. Its feeling looks back only the
    /// touch window, whatever the speed, so a late shock can't land on the
    /// wrong thing (design v21 §5.6).
    Correct { sprite: EntityId, amplified: bool },
    /// Takes hold of a sprite, which the Cursor then leads (design v23 §6.5).
    TakeHold { sprite: EntityId },
    /// Picks up an item, which the Cursor then holds, off the map (design
    /// v23 §6.5).
    PickUp { item: EntityId },
    /// Puts the item the Cursor holds down on a tile, at rest (design v23
    /// §6.5).
    PutDown { tile: Pos },
    /// Lets go of the sprite the Cursor leads, which chooses for itself
    /// again at its next step 5 (design v23 §6.5).
    LetGo,
    /// Where the Cursor is: sent while it leads a sprite, which heads there
    /// (design v23 §2.5).
    MoveCursor { tile: Pos },
    /// Throws the item the Cursor holds (design v25 §3.5.4): puts it down on
    /// `from`, where aiming began, rolling `tiles` tiles `toward`.
    Throw { from: Pos, toward: Dir, tiles: u16 },
    /// Shoves the sprite the Cursor leads (design v25 §3.5.4): lets go of
    /// it, and it slides `tiles` tiles `toward`.
    Shove { toward: Dir, tiles: u16 },
    /// Places a new object of the type with this stable ID on a tile, from
    /// the Place menu (design v26 §2.5). It starts at the beginning of its first
    /// stage.
    Place { tile: Pos, object_type: u16 },
    /// Spawns a new sprite on a tile, from the Place menu (design v26 §2.5): from
    /// the genome it carries in full, or with `None`, from the starter
    /// genome with spawn variation (design §4.9).
    SpawnSprite { tile: Pos, genome: Option<Genome> },
    /// Names a sprite (design §6.5).
    Rename { sprite: EntityId, name: String },
}

/// One of the Cursor's four touches (design v21 §4.6): a Reward or a
/// Correct, amplified or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum CursorTouch {
    Pet,
    /// An amplified pet.
    Hug,
    Zap,
    /// An amplified zap.
    Shock,
}

impl CursorTouch {
    /// The touch `command` gives, if it's a touch.
    pub fn of(command: &Command) -> Option<CursorTouch> {
        match *command {
            Command::Reward { amplified, .. } => Some(CursorTouch::rewarding(amplified)),
            Command::Correct { amplified, .. } => Some(CursorTouch::correcting(amplified)),
            Command::TakeHold { .. }
            | Command::PickUp { .. }
            | Command::PutDown { .. }
            | Command::LetGo
            | Command::MoveCursor { .. }
            | Command::Throw { .. }
            | Command::Shove { .. }
            | Command::Place { .. }
            | Command::SpawnSprite { .. }
            | Command::Rename { .. } => None,
        }
    }

    /// The touch an event reports, if it reports one.
    pub fn reported(kind: &EventKind) -> Option<CursorTouch> {
        match *kind {
            EventKind::Rewarded { amplified, .. } => Some(CursorTouch::rewarding(amplified)),
            EventKind::Corrected { amplified, .. } => Some(CursorTouch::correcting(amplified)),
            _ => None,
        }
    }

    fn rewarding(amplified: bool) -> CursorTouch {
        if amplified {
            CursorTouch::Hug
        } else {
            CursorTouch::Pet
        }
    }

    fn correcting(amplified: bool) -> CursorTouch {
        if amplified {
            CursorTouch::Shock
        } else {
            CursorTouch::Zap
        }
    }

    /// Whether it's a Correct: a zap or a shock.
    pub fn corrects(self) -> bool {
        matches!(self, CursorTouch::Zap | CursorTouch::Shock)
    }

    /// The command that gives `sprite` this touch, a pet or hug looking
    /// `reach_back` ticks back.
    pub fn command(self, sprite: EntityId, reach_back: u64) -> Command {
        let amplified = matches!(self, CursorTouch::Hug | CursorTouch::Shock);
        if self.corrects() {
            Command::Correct { sprite, amplified }
        } else {
            Command::Reward {
                sprite,
                amplified,
                reach_back,
            }
        }
    }

    /// Its name: `pet`, `hug`, `zap` or `shock`.
    pub fn name(self) -> &'static str {
        match self {
            CursorTouch::Pet => "pet",
            CursorTouch::Hug => "hug",
            CursorTouch::Zap => "zap",
            CursorTouch::Shock => "shock",
        }
    }
}

/// Why a command was refused (design §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The sprite isn't in the world: it never was, or it has died.
    Gone,
    /// The Cursor already has hold of something (design v23 §6.5).
    Busy(Grip),
    /// The Cursor leads no sprite to let go of.
    NotLeading,
    /// A fixture can't be picked up: it's attached to the ground (design
    /// §3.3).
    Rooted,
    /// The Cursor holds no item to put down.
    NotHolding,
    /// The tile isn't on the map.
    OffTheMap,
    /// Something on the tile stops the item going there (design §3.4).
    InTheWay {
        /// The stable ID of the held or placed item's type.
        item_type: u16,
        blocker: Blocker,
    },
    /// Something on the tile stops a new sprite standing there (design
    /// §3.4).
    NoRoom(Blocker),
    /// The data doesn't let the Cursor place objects of this type, or the
    /// pack has no such type (design v26 §2.5).
    NotPlaceable,
    /// The tile doesn't meet a condition the type's data asks of where it's
    /// placed (design v26 §2.5).
    PlaceRule {
        /// The stable ID of the placed type.
        object_type: u16,
        rule: PlaceRule,
    },
    /// A sprite's name must be 1 to 16 CP437 characters (design §2.5).
    BadName(NameProblem),
    /// The genome doesn't fit the world's data pack (design §2.5).
    BadGenome,
}

/// A condition of where a type may be placed that the tile didn't meet
/// (design v26 §2.5), as the data names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceRule {
    /// It would cut a path (design §3.3).
    KeepsPathsOpen,
    /// The ground isn't fertile enough, or is too fertile.
    Fertility,
    /// Too many objects of the type with this stable ID are near.
    DensityBelow(u16),
    /// A condition on the new object itself, its stage or a counter.
    Itself,
}

/// What stops an item being put down on a tile (design §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocker {
    /// An object is there already: the stable ID of its type.
    Object(u16),
    /// A sprite stands there.
    Sprite,
    /// The tile's terrain isn't walkable.
    Terrain(Terrain),
}

/// Step 1 (design §2.4): applies the commands stamped for this tick, in the
/// order they were submitted.
pub(crate) fn apply(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    for command in std::mem::take(&mut state.commands) {
        let applied = match command.clone() {
            Command::Reward {
                sprite,
                amplified,
                reach_back,
            } => reward(state, data, sprite, amplified, reach_back),
            Command::Correct { sprite, amplified } => correct(state, data, sprite, amplified),
            Command::TakeHold { sprite } => take_hold(state, sprite, events),
            Command::PickUp { item } => pick_up(state, data, item),
            Command::PutDown { tile } => put_down(state, data, tile),
            Command::LetGo => let_go(state),
            // Nothing to report: it moves many times a second while leading.
            Command::MoveCursor { tile } if state.map.contains(tile) => {
                state.cursor.tile = Some(tile);
                continue;
            }
            Command::MoveCursor { .. } => Err(Rejection::OffTheMap),
            Command::Throw {
                from,
                toward,
                tiles,
            } => throw(state, data, from, toward, tiles),
            Command::Shove { toward, tiles } => shove(state, data, toward, tiles),
            Command::Place { tile, object_type } => place(state, data, tile, object_type),
            Command::SpawnSprite { tile, genome } => spawn_sprite(state, data, tile, genome),
            Command::Rename { sprite, name } => rename(state, sprite, &name),
        };
        let kind = match applied {
            Ok(kind) => kind,
            Err(reason) => EventKind::CommandRejected { command, reason },
        };
        events.push(Event {
            tick: state.tick,
            kind,
        });
    }
}

/// The Cursor's good touch on `sprite` (design v21 §4.6): a pet, or
/// amplified, a hug. Its feeling looks back its reach back, within the
/// bounds, and several in a tick as far as the furthest (design v21 §2.5,
/// §5.6).
fn reward(
    state: &mut WorldState,
    data: &DataPack,
    sprite: EntityId,
    amplified: bool,
    reach_back: u64,
) -> Result<EventKind, Rejection> {
    let physiology = data.physiology();
    let (cursor, indices) = (&physiology.cursor, &physiology.indices);
    let touched = state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?;
    let reach_back = reach_back.clamp(physiology.touch_window, cursor.max_reach_back);
    let brain = &mut touched.brain;
    brain.reach_back = brain.reach_back.max(Some(reach_back));
    let body = &mut touched.body;
    let reward = if amplified { cursor.hug } else { cursor.pet };
    body.raise(indices.reward, reward);
    body.pulse(indices.petted, None);
    Ok(EventKind::Rewarded {
        id: sprite,
        amplified,
    })
}

/// The Cursor's bad touch on `sprite` (design v21 §4.6): a zap, or
/// amplified, a shock. Its feeling looks back only the touch window, which
/// its `shocked` pulse tells learning (design v21 §5.6).
fn correct(
    state: &mut WorldState,
    data: &DataPack,
    sprite: EntityId,
    amplified: bool,
) -> Result<EventKind, Rejection> {
    let physiology = data.physiology();
    let (cursor, indices) = (&physiology.cursor, &physiology.indices);
    let body = &mut state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?.body;
    let correction = if amplified { cursor.shock } else { cursor.zap };
    body.raise(indices.punishment, correction.punishment);
    body.raise(indices.pain, correction.pain);
    body.pulse(indices.shocked, None);
    Ok(EventKind::Corrected {
        id: sprite,
        amplified,
    })
}

/// The Cursor takes hold of `sprite`, and leads it (design v23 §6.5):
/// whatever it was doing ends, pulled away.
fn take_hold(
    state: &mut WorldState,
    sprite: EntityId,
    events: &mut Vec<Event>,
) -> Result<EventKind, Rejection> {
    state.cursor.free()?;
    let led = state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?;
    if let Some(doing) = led.action.as_mut().filter(|a| a.ended.is_none()) {
        action::end(doing, sprite, Outcome::PulledAway, state.tick, events);
    }
    led.lead = Some(Walk::default());
    state.cursor.take(Grip::Leads(sprite));
    state.cursor.tile = Some(led.pos);
    Ok(EventKind::TookHold { sprite })
}

/// The Cursor lets go of the sprite it leads (design v23 §6.5).
fn let_go(state: &mut WorldState) -> Result<EventKind, Rejection> {
    let sprite = state.cursor.leads().ok_or(Rejection::NotLeading)?;
    state.cursor.release();
    let led = state.sprites.get_mut(sprite).expect("the led sprite");
    led.lead = None;
    Ok(EventKind::LetGo { sprite })
}

/// The Cursor picks up `item`, and holds it off the map, at rest (design
/// v23 §6.5).
fn pick_up(
    state: &mut WorldState,
    data: &DataPack,
    item: EntityId,
) -> Result<EventKind, Rejection> {
    state.cursor.free()?;
    let object = state.objects.get(item).ok_or(Rejection::Gone)?;
    let object_type = &data.object_types()[object.kind];
    // In M1 every fixture is solid, and every solid object a fixture
    // (design §3.3).
    if object_type.solid {
        return Err(Rejection::Rooted);
    }
    let object_type = object_type.name.clone();
    state.objects.pick_up(item);
    state.cursor.take(Grip::Holds(item));
    Ok(EventKind::PickedUp { item, object_type })
}

/// The Cursor puts the item it holds down on `tile`, at rest, where an item
/// may go: a walkable tile holding no object, a sprite there or not (design
/// §3.4).
fn put_down(state: &mut WorldState, data: &DataPack, tile: Pos) -> Result<EventKind, Rejection> {
    let (item, object_type) = let_go_of_held(state, data, tile)?;
    Ok(EventKind::PutDown {
        item,
        object_type,
        pos: tile,
    })
}

/// The Cursor throws the item it holds (design v25 §3.5.4): it's put down on
/// `from`, as `PutDown` would put it, and rolls `tiles` tiles `toward`, from
/// this tick's step 2: at least 1, and at most the furthest for its size.
fn throw(
    state: &mut WorldState,
    data: &DataPack,
    from: Pos,
    toward: Dir,
    tiles: u16,
) -> Result<EventKind, Rejection> {
    let (item, object_type) = let_go_of_held(state, data, from)?;
    let left = sent(state, data, Target::Object(item), tiles);
    let thrown = state.objects.get_mut(item).expect("the thrown item");
    thrown.roll = Some(Roll { dir: toward, left });
    Ok(EventKind::Threw { item, object_type })
}

/// The Cursor shoves the sprite it leads (design v25 §3.5.4): lets go of
/// it, and it slides `tiles` tiles `toward`, from this tick's step 2: at
/// least 1, and at most the furthest for its size. Sprites are large.
fn shove(
    state: &mut WorldState,
    data: &DataPack,
    toward: Dir,
    tiles: u16,
) -> Result<EventKind, Rejection> {
    let sprite = state.cursor.leads().ok_or(Rejection::NotLeading)?;
    let left = sent(state, data, Target::Sprite(sprite), tiles);
    state.cursor.release();
    let shoved = state.sprites.get_mut(sprite).expect("the led sprite");
    shoved.lead = None;
    shoved.slide = Some(Slide { dir: toward, left });
    Ok(EventKind::Shoved { sprite })
}

/// How far a throw or a shove asked to send `thing` `tiles` tiles sends it:
/// at least 1, and at most the furthest for its size (design v25 §2.5).
fn sent(state: &WorldState, data: &DataPack, thing: Target, tiles: u16) -> u16 {
    // The pack guarantees the furthest is at least 1.
    tiles.clamp(1, furthest(state, data, thing))
}

/// The furthest the Cursor throws or shoves `thing`, by its size (design
/// v25 §3.5.4). Sprites are large, in a pack that doesn't say.
pub(crate) fn furthest(state: &WorldState, data: &DataPack, thing: Target) -> u16 {
    let size = state
        .kind_of(data, thing)
        .and_then(|kind| data.object_types()[kind].build)
        .map_or(Size::Large, |build| build.size);
    data.physiology().cursor.furthest.of(size)
}

/// The Cursor lets go of the item it holds onto `tile`, at rest, where an
/// item may go: a walkable tile holding no object, a sprite there or not
/// (design §3.4). Returns the item and its type's name.
fn let_go_of_held(
    state: &mut WorldState,
    data: &DataPack,
    tile: Pos,
) -> Result<(EntityId, String), Rejection> {
    let item = state.cursor.holds().ok_or(Rejection::NotHolding)?;
    if !state.map.contains(tile) {
        return Err(Rejection::OffTheMap);
    }
    let held = &data.object_types()[state.objects.kind(item)];
    let in_the_way = |blocker| Rejection::InTheWay {
        item_type: held.id,
        blocker,
    };
    if let Some(there) = state.objects.at(tile) {
        let kind = state.objects.kind(there);
        return Err(in_the_way(Blocker::Object(data.object_types()[kind].id)));
    }
    if !state.map.is_walkable(tile) {
        return Err(in_the_way(Blocker::Terrain(state.map.terrain(tile))));
    }
    state.objects.put_down(item, tile);
    state.cursor.release();
    Ok((item, held.name.clone()))
}

/// The Cursor places a new object of the type with stable ID `object_type`
/// on `tile` (design v26 §2.5), at the beginning of its first stage: where the
/// space rules let it go (design §3.3–3.4), and where the tile meets what
/// the type's data asks of it.
fn place(
    state: &mut WorldState,
    data: &DataPack,
    tile: Pos,
    object_type: u16,
) -> Result<EventKind, Rejection> {
    let kind = data
        .object_types()
        .iter()
        .position(|t| t.id == object_type)
        .ok_or(Rejection::NotPlaceable)?;
    let placed = &data.object_types()[kind];
    let placement = placed.place.as_ref().ok_or(Rejection::NotPlaceable)?;
    if !state.map.contains(tile) {
        return Err(Rejection::OffTheMap);
    }
    let in_the_way = |blocker| Rejection::InTheWay {
        item_type: object_type,
        blocker,
    };
    if let Some(there) = state.objects.at(tile) {
        let kind = state.objects.kind(there);
        return Err(in_the_way(Blocker::Object(data.object_types()[kind].id)));
    }
    let terrain = state.map.terrain(tile);
    if !state.can_place(data, kind, tile) {
        return Err(in_the_way(
            if placed.solid && state.sprites.at(tile).is_some() {
                Blocker::Sprite
            } else {
                Blocker::Terrain(terrain)
            },
        ));
    }
    let object = new_object(data, kind, tile);
    for condition in &placement.conditions {
        let held =
            holds_without_drawing(&state.map, &state.objects, data, &object, tile, condition)
                .expect("the data allows no Chance in a placement");
        if !held {
            return Err(Rejection::PlaceRule {
                object_type,
                rule: place_rule(data, condition),
            });
        }
    }
    let id = state.add_object(object);
    Ok(EventKind::Placed {
        id,
        object_type: placed.name.clone(),
        pos: tile,
    })
}

/// How a refusal names the placement `condition` a tile didn't meet.
fn place_rule(data: &DataPack, condition: &Condition) -> PlaceRule {
    match *condition {
        Condition::KeepsPathsOpen => PlaceRule::KeepsPathsOpen,
        Condition::Fertility(..) => PlaceRule::Fertility,
        Condition::DensityBelow(kind, ..) => PlaceRule::DensityBelow(data.object_types()[kind].id),
        Condition::InStage(..) | Condition::Counter(..) | Condition::Chance(..) => {
            PlaceRule::Itself
        }
    }
}

/// The Cursor spawns a new sprite on `tile` (design v26 §2.5): from `genome`, or
/// from the starter genome with spawn variation (design §4.9), on a tile a
/// sprite may stand on (design §3.4).
fn spawn_sprite(
    state: &mut WorldState,
    data: &DataPack,
    tile: Pos,
    genome: Option<Genome>,
) -> Result<EventKind, Rejection> {
    if !state.map.contains(tile) {
        return Err(Rejection::OffTheMap);
    }
    let in_the_way = Rejection::NoRoom;
    if state.sprites.at(tile).is_some() {
        return Err(in_the_way(Blocker::Sprite));
    }
    if let Some(there) = state.objects.at(tile)
        && data.object_types()[state.objects.kind(there)].solid
    {
        let kind = state.objects.kind(there);
        return Err(in_the_way(Blocker::Object(data.object_types()[kind].id)));
    }
    if !state.map.is_walkable(tile) {
        return Err(in_the_way(Blocker::Terrain(state.map.terrain(tile))));
    }
    let genome = match genome {
        Some(genome) if !genome.fits(data) => return Err(Rejection::BadGenome),
        Some(genome) => genome,
        None => varied(data.starter(), data, &mut state.rng),
    };
    let sprite = Sprite::newborn(genome, tile, state.tick, data);
    let id = state.add_sprite(sprite);
    Ok(EventKind::Spawned { id, pos: tile })
}

/// The player names `sprite` (design §6.5): 1 to 16 CP437 characters,
/// trimmed of spaces at either end.
fn rename(state: &mut WorldState, sprite: EntityId, name: &str) -> Result<EventKind, Rejection> {
    let named = state.sprites.get_mut(sprite).ok_or(Rejection::Gone)?;
    let name = names::checked(name).map_err(Rejection::BadName)?;
    named.name = Some(name.to_string());
    Ok(EventKind::Renamed {
        id: sprite,
        name: name.to_string(),
    })
}
