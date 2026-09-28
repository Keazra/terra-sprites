//! Step 4 (design §5.6): each sprite uses up its reward and punishment, and
//! learns from the difference.

use serde::Serialize;

use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::objects::EntityId;
use crate::registry::{Category, Verb};
use crate::world::WorldState;

/// Instinct links (design §5.1, §5.4): a row per concept or input, a column
/// per verb or category, each at the weight the genome gave it. They never
/// change in a sprite's life; learning is worth and habits (§5.6).
#[derive(Debug, Clone, Serialize)]
#[serde(bound(serialize = "[f32; N]: Serialize"))]
pub(crate) struct Links<const N: usize> {
    w: Vec<[f32; N]>,
}

impl<const N: usize> Links<N> {
    /// Links at the weights `birth` gives, each clamped to [−1, 1]: spawn
    /// variation can take a 1.0 instinct past 1.
    pub(crate) fn new(mut birth: Vec<[f32; N]>) -> Links<N> {
        for w in birth.iter_mut().flatten() {
            *w = w.clamp(-1.0, 1.0);
        }
        Links { w: birth }
    }

    /// Every row's weights.
    pub(crate) fn rows(&self) -> &[[f32; N]] {
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

/// How many kinds of thing there are to learn about.
const KINDS: usize = Category::ALL.len();

/// What a brain has learned (design §5.6), all starting at 0: what each kind
/// of thing is worth for each need and in general, and how bad it is; its
/// habits; how familiar each kind is; and the worth of new things.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct Experience {
    /// Worth for each need, in the pack's needs order, by category (0 to 1).
    pub(crate) worth: Vec<[f32; KINDS]>,
    /// Each need's level at the last step 4, to read its relief from.
    pub(crate) needs_before: Option<Vec<f32>>,
}

impl Experience {
    /// A newborn's: nothing learned, for `needs` needs.
    pub(crate) fn new(needs: usize) -> Experience {
        Experience {
            worth: vec![[0.0; KINDS]; needs],
            needs_before: None,
        }
    }
}

/// What a sprite tried a verb on, and when (design §5.6): the thing a
/// feeling is about, for `touch_window` ticks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct Touch {
    pub(crate) tick: u64,
    pub(crate) category: Category,
}

/// What step 4 reads for one sprite (design §5.6).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Signals {
    /// Each need's level now, in the pack's needs order.
    pub(crate) needs: Vec<f32>,
    /// Whether a `was_hit` pulse is live: its attacker is the thing touched
    /// if the sprite touched nothing itself.
    pub(crate) attacked: bool,
}

/// Where `category` is in `Category::ALL`.
pub(crate) fn kind(category: Category) -> usize {
    Category::ALL
        .iter()
        .position(|&c| c == category)
        .expect("every category is in ALL")
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
    /// Every concept's activation, in the brain's concept order. The
    /// singletons come first, one per input, so they're the inputs too.
    pub(crate) activations: Vec<f32>,
    /// The verb it chose or kept doing, if any.
    pub(crate) verb: Option<Verb>,
    /// The category attention was on, if any.
    pub(crate) attended: Option<Category>,
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
    let living = state
        .sprites
        .minds_mut()
        .filter(|(id, ..)| !dying.contains(id));
    for (id, body, brain) in living {
        let r = body.chems[indices.reward] - body.chems[indices.punishment];
        body.chems[indices.reward] = 0.0;
        body.chems[indices.punishment] = 0.0;
        brain.felt = r;
        let signals = Signals {
            needs: brain.need_levels(body, data),
            attacked: body.loci[indices.was_hit] > 0.0,
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
pub(crate) fn commit(state: &mut WorldState) {
    let tick = state.tick;
    for (_, _, brain) in state.sprites.minds_mut() {
        brain.commit(tick);
    }
}
