//! Step 4 (design §5.6): each sprite uses up its reward and punishment, and
//! learns from the difference.

use serde::Serialize;

use crate::data::DataPack;
use crate::registry::{Category, Verb};
use crate::world::WorldState;

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
/// 0, and keeps `r` as what the sprite felt.
pub(crate) fn run(state: &mut WorldState, data: &DataPack) {
    let indices = &data.physiology().indices;
    for (body, brain) in state.sprites.minds_mut() {
        let r = body.chems[indices.reward] - body.chems[indices.punishment];
        body.chems[indices.reward] = 0.0;
        body.chems[indices.punishment] = 0.0;
        brain.felt = r;
    }
}

/// The end of step 6 for every sprite: each brain that decided this tick
/// commits its trace entry.
pub(crate) fn commit(state: &mut WorldState) {
    let tick = state.tick;
    for (_, brain) in state.sprites.minds_mut() {
        brain.commit(tick);
    }
}
