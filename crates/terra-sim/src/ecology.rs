//! Step 2 of the tick: objects run their lifecycle rules (design §2.4, §3.5.2).

use rand_chacha::ChaCha8Rng;

use crate::data::DataPack;
use crate::events::{Event, EventKind, Removal};
use crate::map::{Map, Pos};
use crate::object_types::{Condition, Effect, Stage, Trigger};
use crate::objects::{EntityId, Object, Objects};
use crate::random::{chance, uniform};
use crate::world::WorldState;

/// A new object of type `kind` on `pos`, in its first stage. It enters that
/// stage on its first turn, which is when the stage's length is drawn.
pub(crate) fn new_object(data: &DataPack, kind: usize, pos: Pos) -> Object {
    let object_type = &data.object_types()[kind];
    Object {
        kind,
        pos,
        stage: (!object_type.stages.is_empty()).then_some(0),
        stage_ends: 0,
        counters: vec![0; object_type.counters.len()],
        fresh: true,
    }
}

/// A new object of type `kind` on `pos` at a random point in its life, for
/// world generation (design §3.2): every stage's length is drawn, then an age
/// within their total, and the object starts in the stage that age falls in,
/// with that stage's remaining time. Its counters start at 0, and no
/// `OnStageEnter` fires for the stage it starts in.
pub(crate) fn aged_object(data: &DataPack, rng: &mut ChaCha8Rng, kind: usize, pos: Pos) -> Object {
    let object_type = &data.object_types()[kind];
    let mut object = new_object(data, kind, pos);
    object.fresh = false;
    if object_type.stages.is_empty() {
        return object;
    }
    // The stages it lives through: from the first, until it expires or a stage repeats.
    let mut life: Vec<(usize, u64)> = Vec::new();
    let mut stage = Some(0);
    while let Some(index) = stage.filter(|&s| life.iter().all(|&(seen, _)| seen != s)) {
        life.push((index, duration(rng, &object_type.stages[index])));
        stage = object_type.stages[index].next;
    }
    let total: u64 = life.iter().map(|&(_, ticks)| ticks).sum();
    let mut age = uniform(rng, total);
    for (index, ticks) in life {
        if age < ticks {
            object.stage = Some(index);
            object.stage_ends = ticks - age;
            break;
        }
        age -= ticks;
    }
    object
}

/// How long a stage lasts this time: from its minimum to its maximum, uniformly.
fn duration(rng: &mut ChaCha8Rng, stage: &Stage) -> u64 {
    let (min, max) = stage.ticks;
    u64::from(min) + uniform(rng, u64::from(max - min) + 1)
}

/// Runs step 2: every object that exists as it begins takes a turn, in
/// ascending ID order. Objects created during it first run next tick.
pub(crate) fn run(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let ids: Vec<EntityId> = state.objects.iter().map(|(id, _)| id).collect();
    for id in ids {
        turn(state, data, id, events);
    }
}

/// Whether an object's turn goes on after an effect, or ends because the object is gone.
#[derive(PartialEq, Eq)]
enum Turn {
    Continues,
    Ends,
}

/// One object's turn: the stage clock, then its rules (design §3.5.2).
fn turn(state: &mut WorldState, data: &DataPack, id: EntityId, events: &mut Vec<Event>) {
    let tick = state.tick;
    let Some(object) = state.objects.get_mut(id) else {
        return;
    };
    let object_type = &data.object_types()[object.kind];
    let mut entered = None;
    let mut expiring = false;
    // A new object enters its first stage on its first turn.
    if std::mem::take(&mut object.fresh)
        && let Some(first) = object.stage
    {
        object.stage_ends = tick + duration(&mut state.rng, &object_type.stages[first]);
        entered = Some(first);
    }
    if let Some(stage) = object.stage
        && tick >= object.stage_ends
    {
        match object_type.stages[stage].next {
            Some(next) => {
                object.stage = Some(next);
                object.stage_ends = tick + duration(&mut state.rng, &object_type.stages[next]);
                entered = Some(next);
            }
            None => expiring = true,
        }
    }

    for rule in &object_type.rules {
        let fires = match rule.trigger {
            Trigger::Every(n) => !expiring && (tick + id.0).is_multiple_of(u64::from(n)),
            Trigger::OnStageEnter(stage) => !expiring && entered == Some(stage),
            Trigger::OnExpire => expiring,
        };
        let pos = state
            .objects
            .get(id)
            .expect("the object taking its turn")
            .pos;
        if !fires || !conditions_hold(state, data, id, pos, &rule.conditions) {
            continue;
        }
        for effect in &rule.effects {
            if apply(state, data, id, effect, events) == Turn::Ends {
                return;
            }
        }
    }

    if expiring {
        let old = state.objects.remove(id);
        removed(state, data, id, old.kind, Removal::Expired, events);
    }
}

/// Whether every condition holds for the object `id`, with location conditions
/// judged at `pos`. They're checked left to right, stopping at the first false
/// one, so the conditions after it draw nothing from the RNG.
fn conditions_hold(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    pos: Pos,
    conditions: &[Condition],
) -> bool {
    conditions
        .iter()
        .all(|condition| holds(state, data, id, pos, condition))
}

fn holds(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    pos: Pos,
    condition: &Condition,
) -> bool {
    if let Condition::Chance(p) = *condition {
        return chance(&mut state.rng, p);
    }
    let object = state.objects.get(id).expect("the object taking its turn");
    holds_without_drawing(&state.map, &state.objects, data, object, pos, condition)
        .expect("only Chance draws")
}

/// Whether `condition` holds for `object`, with location conditions judged at
/// `pos`, or `None` for `Chance`, the one condition that draws from the RNG.
pub(crate) fn holds_without_drawing(
    map: &Map,
    objects: &Objects,
    data: &DataPack,
    object: &Object,
    pos: Pos,
    condition: &Condition,
) -> Option<bool> {
    Some(match *condition {
        Condition::InStage(stage) => object.stage == Some(stage),
        Condition::Counter(counter, cmp, value) => cmp.holds(object.counters[counter], value),
        Condition::Chance(_) => return None,
        Condition::Fertility(cmp, value) => {
            cmp.holds(data.terrain(map.terrain(pos)).fertility(), value)
        }
        Condition::DensityBelow(kind, radius, max) => {
            count_near(map, objects, pos, kind, radius) < usize::from(max)
        }
        Condition::KeepsPathsOpen => objects.keeps_paths_open(map, data, pos),
    })
}

/// How many objects of type `kind` stand within Chebyshev distance `radius` of `pos`.
fn count_near(map: &Map, objects: &Objects, pos: Pos, kind: usize, radius: u16) -> usize {
    square(map, pos, radius)
        .filter(|&tile| objects.at(tile).is_some_and(|id| objects.kind(id) == kind))
        .count()
}

/// The tiles within Chebyshev distance `radius` of `pos` that are on the map,
/// row by row.
fn square(map: &Map, pos: Pos, radius: u16) -> impl Iterator<Item = Pos> + use<> {
    let (origin, width, height) = square_bounds(map, pos, radius);
    (origin.y..origin.y + height)
        .flat_map(move |y| (origin.x..origin.x + width).map(move |x| Pos { x, y }))
}

/// The square of tiles within Chebyshev distance `radius` of `pos`, cut down
/// to the map: its top-left tile, width and height.
fn square_bounds(map: &Map, pos: Pos, radius: u16) -> (Pos, u16, u16) {
    let x0 = pos.x.saturating_sub(radius);
    let y0 = pos.y.saturating_sub(radius);
    let x1 = pos.x.saturating_add(radius).min(map.width() - 1);
    let y1 = pos.y.saturating_add(radius).min(map.height() - 1);
    (Pos { x: x0, y: y0 }, x1 - x0 + 1, y1 - y0 + 1)
}

/// Applies one effect of a lifecycle rule to the object `id`.
fn apply(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    effect: &Effect,
    events: &mut Vec<Event>,
) -> Turn {
    let object = state
        .objects
        .get_mut(id)
        .expect("the object taking its turn");
    let object_type = &data.object_types()[object.kind];
    let pos = object.pos;
    match *effect {
        Effect::AddCounter(counter, delta) => {
            let max = i64::from(object_type.counters[counter].max);
            let value = i64::from(object.counters[counter]) + i64::from(delta);
            object.counters[counter] = value.clamp(0, max) as u16;
        }
        Effect::SpawnNearby(kind, radius) => {
            let candidates: Vec<Pos> = square(&state.map, pos, radius)
                .filter(|&tile| state.objects.can_place(&state.map, data, kind, tile))
                .collect();
            if !candidates.is_empty() {
                let choice = uniform(&mut state.rng, candidates.len() as u64) as usize;
                create(state, data, kind, candidates[choice], events);
            }
        }
        Effect::SpreadTo(kind, radius, ref conditions) => {
            let (origin, width, height) = square_bounds(&state.map, pos, radius);
            let choice = uniform(&mut state.rng, u64::from(width) * u64::from(height));
            let target = Pos {
                x: origin.x + (choice % u64::from(width)) as u16,
                y: origin.y + (choice / u64::from(width)) as u16,
            };
            if state.objects.can_place(&state.map, data, kind, target)
                && conditions_hold(state, data, id, target, conditions)
            {
                create(state, data, kind, target, events);
            }
        }
        Effect::ReplaceWith(kind) => {
            // Off the tile first, so the new object's placement is judged without it.
            let old = state.objects.remove(id);
            if !state.objects.can_place(&state.map, data, kind, pos) {
                state.objects.place(id, old);
                return Turn::Continues;
            }
            removed(state, data, id, old.kind, Removal::Replaced, events);
            create(state, data, kind, pos, events);
            return Turn::Ends;
        }
        Effect::DestroySelf => {
            let old = state.objects.remove(id);
            removed(state, data, id, old.kind, Removal::Destroyed, events);
            return Turn::Ends;
        }
        Effect::RequireCounter(..) | Effect::Inject(..) | Effect::Signal(..) | Effect::Push(..) => {
            unreachable!("verb-only effects are rejected in lifecycle rules when the pack loads")
        }
    }
    Turn::Continues
}

/// Reports that the object `id`, of type `kind`, has left the world.
fn removed(
    state: &WorldState,
    data: &DataPack,
    id: EntityId,
    kind: usize,
    reason: Removal,
    events: &mut Vec<Event>,
) {
    events.push(Event {
        tick: state.tick,
        kind: EventKind::ObjectRemoved {
            id,
            object_type: data.object_types()[kind].name.clone(),
            reason,
        },
    });
}

/// Creates an object of type `kind` on `pos` during step 2, where the caller
/// has checked it may go. It takes its first turn next tick.
fn create(state: &mut WorldState, data: &DataPack, kind: usize, pos: Pos, events: &mut Vec<Event>) {
    let id = state.add_object(new_object(data, kind, pos));
    events.push(Event {
        tick: state.tick,
        kind: EventKind::ObjectSpawned {
            id,
            object_type: data.object_types()[kind].name.clone(),
            pos,
        },
    });
}
