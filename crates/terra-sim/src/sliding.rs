//! Shoved sprites (design v25 §3.5.4): a shove lets go of the led sprite,
//! which slides a tile a tick at step 2, after the rolling items, choosing
//! nothing, until it has gone as far as the shove sent it or something stops
//! it. Stopping against a solid object or a sprite is a crash.

use serde::{Deserialize, Serialize};

use crate::brain::Thing;
use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::map::{Dir, Pos};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::verbs;
use crate::world::WorldState;

/// A sliding sprite's way on (design v25 §3.5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Slide {
    /// The way it's sliding.
    pub(crate) dir: Dir,
    /// The tiles it has left to go.
    pub(crate) left: u16,
    /// Whether it could see the Cursor that shoved it (design v29 §5.6).
    pub(crate) seen: bool,
}

/// What a sliding sprite meets a tile on.
enum Meeting {
    /// Nothing that stops it: it slides there.
    Open(Pos),
    /// Unwalkable terrain, or the wall: it stops, with no crash.
    Halt,
    /// A sprite or a solid object: it stops, and crashes into it.
    Crash(Target),
}

/// Every sliding sprite, in ascending ID order, slides a tile.
pub(crate) fn run(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let sliding: Vec<EntityId> = state
        .sprites
        .iter()
        .filter(|(_, sprite)| sprite.slide.is_some())
        .map(|(id, _)| id)
        .collect();
    for id in sliding {
        slide(state, data, id, events);
    }
}

/// Sprite `id` slides a tile on its way, unless whatever would stop a
/// rolling item stops it, when the slide ends where it is, with no bounce.
fn slide(state: &mut WorldState, data: &DataPack, id: EntityId, events: &mut Vec<Event>) {
    let sprite = state.sprites.get(id).expect("a sliding sprite");
    let (from, Some(Slide { dir, left, seen })) = (sprite.pos, sprite.slide) else {
        return;
    };
    let rest = match meet(state, data, from, dir) {
        Meeting::Open(to) => {
            state.sprites.move_to(id, to);
            (left > 1).then_some(Slide {
                dir,
                left: left - 1,
                seen,
            })
        }
        Meeting::Halt => None,
        Meeting::Crash(target) => {
            let into = match target {
                Target::Object(object) => {
                    let kind = state.objects.kind(object);
                    Thing::ObjectType(data.object_types()[kind].name.clone())
                }
                Target::Sprite(other) => Thing::Sprite(other),
                Target::Water(_) => unreachable!("water stops a slide without a crash"),
                Target::Cursor => unreachable!("the Cursor is light, and stops nothing"),
            };
            let hurt = verbs::crash(state, data, id, target);
            // Shoved by a Cursor it could see, the Cursor is part of the
            // lesson (design v29 §5.6).
            let brain = &mut state.sprites.get_mut(id).expect("the same sprite").brain;
            if let Some(touch) = brain.touched.as_mut() {
                touch.by_cursor = seen;
            }
            events.push(Event {
                tick: state.tick,
                kind: EventKind::Crashed {
                    sprite: id,
                    into,
                    hurt,
                },
            });
            None
        }
    };
    state.sprites.get_mut(id).expect("the same sprite").slide = rest;
}

/// What a sprite on `from`, sliding in direction `dir`, meets. It stops at
/// the map's edge, unwalkable terrain, a sprite, a solid object, or a corner
/// it may not cut (design §3.1); an item doesn't stop it. A diagonal slide
/// passes the corner before it reaches the tile ahead, so the corner comes
/// first: stopped there, it crashes into the solid object beside it, the one
/// to its east or west first. Past the corner, stopped by what's on the tile
/// ahead, it crashes into that (design v25 §3.5.4).
fn meet(state: &WorldState, data: &DataPack, from: Pos, dir: Dir) -> Meeting {
    let Some(to) = state.map.neighbour(from, dir) else {
        return Meeting::Halt;
    };
    let solid = |pos| {
        state
            .objects
            .at(pos)
            .filter(|_| state.objects.is_solid_at(data, pos))
    };
    if let Some((across, along)) = dir.parts() {
        let sides = [across, along].map(|side| {
            state
                .map
                .neighbour(from, side)
                .expect("beside a step on the map")
        });
        if let Some(object) = sides.into_iter().find_map(solid) {
            return Meeting::Crash(Target::Object(object));
        }
        if !sides.into_iter().all(|side| state.map.is_walkable(side)) {
            return Meeting::Halt;
        }
    }
    if let Some(other) = state.sprites.at(to) {
        return Meeting::Crash(Target::Sprite(other));
    }
    if let Some(object) = solid(to) {
        return Meeting::Crash(Target::Object(object));
    }
    if !state.map.is_walkable(to) {
        return Meeting::Halt;
    }
    Meeting::Open(to)
}
