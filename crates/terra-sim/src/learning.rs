//! Step 4 (design §5.6): each sprite reads its needs' relief and uses up its
//! reward and punishment, and learns what things are worth and its habits.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::brain::{Learned, VERBS};
use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::objects::EntityId;
use crate::registry::{CategoryId, Verb};
use crate::world::WorldState;

/// Instinct links (design §5.1, §5.4): a row per concept or input, a column
/// per verb or category, each at the weight the genome gave it. They never
/// change in a sprite's life; learning is worth and habits (§5.6).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Links {
    w: Vec<Vec<f32>>,
}

impl Links {
    /// Links at the weights `birth` gives, each clamped to [−1, 1]: spawn
    /// variation can take a 1.0 instinct past 1.
    pub(crate) fn new(mut birth: Vec<Vec<f32>>) -> Links {
        for w in birth.iter_mut().flatten() {
            *w = w.clamp(-1.0, 1.0);
        }
        Links { w: birth }
    }

    /// Every row's weights.
    pub(crate) fn rows(&self) -> &[Vec<f32>] {
        &self.w
    }

    /// The weight of the link in `row` and `column`.
    pub(crate) fn get(&self, row: usize, column: usize) -> f32 {
        self.w[row][column]
    }

    /// Checks every weight is a number within [−1, 1], or says which isn't.
    pub(crate) fn check(&self) -> Result<(), String> {
        match self
            .w
            .iter()
            .flatten()
            .copied()
            .find(|w| !(-1.0..=1.0).contains(w))
        {
            Some(w) => Err(format!("a link at {w}, outside -1 to 1")),
            None => Ok(()),
        }
    }

    /// Sets the weight in `row` and `column`, unchecked.
    #[cfg(test)]
    pub(crate) fn set(&mut self, row: usize, column: usize, w: f32) {
        self.w[row][column] = w;
    }
}

/// What a sprite learns about a thing as (design v19 §5.6): its object type,
/// by its stable ID, or its category, for water or sprites in a pack with no
/// object type for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) enum Subject {
    ObjectType(u16),
    Category(CategoryId),
}

impl Subject {
    /// What a thing of `category`, and of the object type with the stable ID
    /// `object_type` if it has one, is learned about as.
    pub(crate) fn of(category: CategoryId, object_type: Option<u16>) -> Subject {
        object_type.map_or(Subject::Category(category), Subject::ObjectType)
    }

    /// The category it's in.
    pub(crate) fn category(self, data: &DataPack) -> CategoryId {
        match self {
            Subject::ObjectType(id) => {
                data.object_type(id)
                    .expect("an object type in the pack")
                    .category
            }
            Subject::Category(category) => category,
        }
    }
}

/// What a sprite has learned about one object type (design v19 §5.6): its
/// worth for each need and in general, how bad it is, its habits, how
/// familiar it is, and whether it knows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TypeMemory {
    /// Worth for each need, in the pack's needs order (0 to 1).
    pub(crate) worth: Vec<f32>,
    /// General good, from `reward` (0 to 1).
    pub(crate) good: f32,
    /// Bad, from `punishment` (−1 to 0).
    pub(crate) bad: f32,
    /// Habits: doing each verb to it, in `VERBS` order.
    pub(crate) habits: [f32; VERBS.len()],
    /// How familiar it is, from attending to it (0 to 1).
    pub(crate) familiarity: f32,
    /// Whether it has touched one, and so knows it: until then, it's
    /// judged by its category's summary.
    pub(crate) touched: bool,
}

impl TypeMemory {
    /// An object type newly learned about, for `needs` needs: nothing yet.
    pub(crate) fn new(needs: usize) -> TypeMemory {
        TypeMemory {
            worth: vec![0.0; needs],
            good: 0.0,
            bad: 0.0,
            habits: [0.0; VERBS.len()],
            familiarity: 0.0,
            touched: false,
        }
    }
}

/// What a brain has learned (design §5.6), all starting at 0: what each
/// object type is worth for each need and in general, how bad it is, its
/// habits and how familiar it is; the sprites it remembers; and the worth of
/// new things.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Experience {
    /// What it has learned about each object type (design v19 §5.6). Object
    /// types are never forgotten.
    pub(crate) types: BTreeMap<Subject, TypeMemory>,
    /// The worth of new things.
    pub(crate) new_things: f32,
    /// The sprites it remembers (design v18 §5.6), by ID.
    pub(crate) individuals: BTreeMap<EntityId, SpriteMemory>,
    /// Which learned values have been lessons (design §5.6).
    pub(crate) taught: BTreeSet<Learned>,
    /// Each need's level at the last step 4, to read its relief from.
    pub(crate) needs_before: Option<Vec<f32>>,
}

impl Experience {
    /// Checks every learned value is a number within its range (design
    /// §5.6): worth, good and familiarity 0 to 1, bad −1 to 0, habits and
    /// the worth of new things −1 to 1. Says which isn't.
    pub(crate) fn check(&self) -> Result<(), String> {
        for known in self.types.values() {
            within(&known.worth, (0.0, 1.0), "a worth")?;
            within([&known.good], (0.0, 1.0), "a good")?;
            within([&known.bad], (-1.0, 0.0), "a bad")?;
            within(&known.habits, (-1.0, 1.0), "a habit")?;
            within([&known.familiarity], (0.0, 1.0), "a familiarity")?;
        }
        within([&self.new_things], (-1.0, 1.0), "the worth of new things")?;
        for individual in self.individuals.values() {
            within(&individual.worth, (0.0, 1.0), "a sprite's worth")?;
            within([&individual.good], (0.0, 1.0), "a sprite's good")?;
            within([&individual.bad], (-1.0, 0.0), "a sprite's bad")?;
            within([&individual.fear], (-1.0, 0.0), "a sprite's fear")?;
        }
        Ok(())
    }
}

impl Experience {
    /// What it has learned about `subject`, starting afresh, for `needs`
    /// needs, if it has learned nothing yet (design v19 §5.6).
    pub(crate) fn learn_about(&mut self, subject: Subject, needs: usize) -> &mut TypeMemory {
        self.types
            .entry(subject)
            .or_insert_with(|| TypeMemory::new(needs))
    }

    /// What it remembers of `sprite`, remembering it afresh, for `needs`
    /// needs, if it doesn't yet (design v18 §5.6).
    pub(crate) fn remember(&mut self, sprite: EntityId, needs: usize) -> &mut SpriteMemory {
        self.individuals
            .entry(sprite)
            .or_insert_with(|| SpriteMemory::new(needs))
    }

    /// Forgets every remembered sprite that everything learned about is
    /// nearer 0 than `below` (design v18 §5.6).
    pub(crate) fn forget_faded(&mut self, below: f32) {
        self.individuals.retain(|_, memory| !memory.faded(below));
    }

    /// Forgets every remembered sprite not in `alive`: a sprite that has
    /// died is forgotten (design v18 §5.6).
    pub(crate) fn forget_dead(&mut self, alive: &BTreeSet<EntityId>) {
        self.individuals.retain(|sprite, _| alive.contains(sprite));
    }
}

/// What a sprite has learned about one other sprite (design v18 §5.6): its
/// worth for each need and in general, how bad it is, and how frightening.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SpriteMemory {
    /// Worth for each need, in the pack's needs order (0 to 1).
    pub(crate) worth: Vec<f32>,
    /// General good, from `reward` (0 to 1).
    pub(crate) good: f32,
    /// Bad, from hurting itself on it (−1 to 0).
    pub(crate) bad: f32,
    /// Fear, from its hurting the sprite (−1 to 0).
    pub(crate) fear: f32,
}

impl SpriteMemory {
    /// Whether everything learned about it is nearer 0 than `below`.
    pub(crate) fn faded(&self, below: f32) -> bool {
        let values = self.worth.iter().chain([&self.good, &self.bad, &self.fear]);
        values.into_iter().all(|v| v.abs() < below)
    }

    /// A sprite newly remembered, for `needs` needs: nothing learned yet.
    pub(crate) fn new(needs: usize) -> SpriteMemory {
        SpriteMemory {
            worth: vec![0.0; needs],
            good: 0.0,
            bad: 0.0,
            fear: 0.0,
        }
    }
}

/// Checks each of `values` is a number from `low` to `high`, or says which
/// of `what` isn't.
fn within<'a>(
    values: impl IntoIterator<Item = &'a f32>,
    (low, high): (f32, f32),
    what: &str,
) -> Result<(), String> {
    match values.into_iter().find(|v| !(low..=high).contains(*v)) {
        Some(v) => Err(format!("{what} at {v}, outside {low} to {high}")),
        None => Ok(()),
    }
}

/// What a sprite tried a verb on, and when (design §5.6): the thing a
/// feeling is about, for `touch_window` ticks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct Touch {
    pub(crate) tick: u64,
    /// What it's learned about as (design v19 §5.6).
    pub(crate) subject: Subject,
    /// Which sprite, if it was one (design v18 §5.6).
    pub(crate) sprite: Option<EntityId>,
    /// How new its object type was to the sprite then (design v19 §5.6).
    pub(crate) novelty: f32,
}

/// What step 4 reads for one sprite (design §5.6).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Signals {
    /// Each need's level now, in the pack's needs order.
    pub(crate) needs: Vec<f32>,
    /// Each need's relief this tick, in the same order.
    pub(crate) relief: Vec<f32>,
    /// The general good and bad channels: `reward` and `punishment`.
    pub(crate) reward: f32,
    pub(crate) punishment: f32,
    /// Whether a `fruitless` pulse is live: its latest try did nothing.
    pub(crate) fruitless: bool,
    /// Whether a `was_hit` pulse is live: its punishment teaches fear of
    /// the attacker and habits, not what anything is worth (design v18 §5.6).
    pub(crate) hit: bool,
    /// Who hit it, the `was_hit` pulse's source, if one is live.
    pub(crate) attacker: Option<EntityId>,
}

/// The most entries a trace keeps (design §5.6).
pub(crate) const TRACE_CAP: usize = 512;

/// The least weight an entry keeps its place in the trace with (design §5.6).
const TRACE_FLOOR: f32 = 0.01;

/// The weight step 4 at tick `now` gives the trace entry from tick `then`
/// (design §5.6): λ^(now − then).
pub(crate) fn weight(trace_decay: f32, now: u64, then: u64) -> f32 {
    libm::powf(trace_decay, (now - then) as f32)
}

/// Whether the entry from tick `then` still counts at tick `now`'s step 4.
pub(crate) fn still_counts(trace_decay: f32, now: u64, then: u64) -> bool {
    weight(trace_decay, now, then) >= TRACE_FLOOR
}

/// One tick of a brain's trace (design §5.6): what it felt and chose at
/// that tick's step 5.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TraceEntry {
    pub(crate) tick: u64,
    /// The verb it chose or kept doing, if any.
    pub(crate) verb: Option<Verb>,
    /// What the thing attention was on, if any, is learned about as (design
    /// v19 §5.6).
    pub(crate) subject: Option<Subject>,
    /// The verb's motive (design §5.5): the need, by its place in the pack's
    /// needs, whose instinct did most to choose it.
    pub(crate) motive: Option<usize>,
}

/// Step 4 for every sprite not `dying` (design §2.4): reads each need's
/// relief, reward and punishment, resets the last two to 0, keeps what the
/// sprite felt, and learns from them, reporting each lesson.
pub(crate) fn run(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    events: &mut Vec<Event>,
) {
    let indices = &data.physiology().indices;
    let tick = state.tick;
    let learning = state.learning;
    // A sprite that has died is forgotten (design v18 §5.6).
    let alive: BTreeSet<EntityId> = state
        .sprites
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !dying.contains(id))
        .collect();
    let living = state
        .sprites
        .minds_mut()
        .filter(|(id, ..)| !dying.contains(id));
    for (id, body, brain) in living {
        let (reward, punishment) = (body.chems[indices.reward], body.chems[indices.punishment]);
        body.chems[indices.reward] = 0.0;
        body.chems[indices.punishment] = 0.0;
        brain.experience.forget_dead(&alive);
        let needs = brain.need_levels(body, data);
        let relief = brain.relief(&needs, data);
        brain.felt = relief.iter().sum::<f32>() + reward - punishment;
        if !learning {
            continue;
        }
        let signals = Signals {
            needs,
            relief,
            reward,
            punishment,
            fruitless: body.loci[indices.fruitless] > 0.0,
            hit: body.loci[indices.was_hit] > 0.0,
            attacker: body.sources.get(&indices.was_hit).copied(),
        };
        let rate = body.loci[indices.learning_rate_mod];
        for (learned, good) in brain.learn(tick, &signals, rate, data) {
            events.push(Event {
                tick,
                kind: EventKind::LearnedMilestone { id, learned, good },
            });
        }
    }
}

/// The end of step 6 for every sprite: each brain that decided this tick
/// commits its trace entry.
pub(crate) fn commit(state: &mut WorldState, data: &DataPack) {
    let tick = state.tick;
    for (_, _, brain) in state.sprites.minds_mut() {
        brain.commit(tick, data);
    }
}
