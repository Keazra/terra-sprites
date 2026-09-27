//! Step 4 (design §5.6): each sprite uses up its reward and punishment, and
//! learns from the difference.

use serde::Serialize;

use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::registry::{Category, Verb};
use crate::world::WorldState;

/// Links that learn (design §5.6): a row per concept or input, a column per
/// verb or category. Each link has a working weight `w`, which learning
/// moves, and a consolidated one `w_long`; both start at the instinct.
#[derive(Debug, Clone, Serialize)]
#[serde(bound(serialize = "[f32; N]: Serialize, [bool; N]: Serialize"))]
pub(crate) struct LearnableLinks<const N: usize> {
    w: Vec<[f32; N]>,
    long: Vec<[f32; N]>,
    /// The instinct each link was born with.
    birth: Vec<[f32; N]>,
    /// Whether each link has been a lesson yet.
    taught: Vec<[bool; N]>,
}

impl<const N: usize> LearnableLinks<N> {
    /// Links born at the weights `birth` gives.
    pub(crate) fn new(birth: Vec<[f32; N]>) -> LearnableLinks<N> {
        LearnableLinks {
            w: birth.clone(),
            long: birth.clone(),
            taught: vec![[false; N]; birth.len()],
            birth,
        }
    }

    /// Every row's working weights.
    pub(crate) fn rows(&self) -> &[[f32; N]] {
        &self.w
    }

    /// The working weight of the link in `row` and `column`.
    pub(crate) fn get(&self, row: usize, column: usize) -> f32 {
        self.w[row][column]
    }

    /// The consolidated weight of the link in `row` and `column`.
    #[cfg(test)]
    pub(crate) fn settled(&self, row: usize, column: usize) -> f32 {
        self.long[row][column]
    }

    /// Moves the link in `row` and `column` by `by`, keeping it within
    /// [−1, 1].
    pub(crate) fn nudge(&mut self, row: usize, column: usize, by: f32) {
        let w = &mut self.w[row][column];
        *w = (*w + by).clamp(-1.0, 1.0);
    }

    /// The links whose working weight is `threshold` or more from birth for
    /// the first time, as `(row, column, rose)`, in row then column order.
    /// Each is marked, so it's a lesson only once (design §5.6).
    pub(crate) fn lessons(&mut self, threshold: f32) -> impl Iterator<Item = (usize, usize, bool)> {
        let mut found = Vec::new();
        for (row, taught) in self.taught.iter_mut().enumerate() {
            for (column, taught) in taught.iter_mut().enumerate() {
                let moved = self.w[row][column] - self.birth[row][column];
                if !*taught && moved.abs() >= threshold {
                    *taught = true;
                    found.push((row, column, moved > 0.0));
                }
            }
        }
        found.into_iter()
    }

    /// One tick of the two timescales: `w` relaxes towards `w_long` by
    /// `relax_rate`, and `w_long` consolidates towards `w` by
    /// `consolidate_rate`, both from the values before the tick.
    pub(crate) fn relax(&mut self, relax_rate: f32, consolidate_rate: f32) {
        for (w, long) in self.w.iter_mut().zip(&mut self.long) {
            for (w, long) in w.iter_mut().zip(long.iter_mut()) {
                let (was, settled) = (*w, *long);
                *w = was + relax_rate * (settled - was);
                *long = settled + consolidate_rate * (was - settled);
            }
        }
    }
}

/// The most entries a trace keeps (design §5.6).
pub(crate) const TRACE_CAP: usize = 512;

/// The least weight an entry keeps its place in the trace with.
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

/// Step 4 for every sprite: reads `r = reward − punishment`, resets both to
/// 0, keeps `r` as what the sprite felt, and learns from it, reporting each
/// lesson.
pub(crate) fn run(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let indices = &data.physiology().indices;
    let tick = state.tick;
    for (id, body, brain) in state.sprites.minds_mut() {
        let r = body.chems[indices.reward] - body.chems[indices.punishment];
        body.chems[indices.reward] = 0.0;
        body.chems[indices.punishment] = 0.0;
        brain.felt = r;
        for lesson in brain.learn(tick, r, body.loci[indices.learning_rate_mod], data) {
            let (link, good) = lesson.named(brain, data);
            events.push(Event {
                tick,
                kind: EventKind::LearnedMilestone { id, link, good },
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
