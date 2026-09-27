//! Step 4 (design §5.6): each sprite uses up its reward and punishment, and
//! learns from the difference.

use crate::data::DataPack;
use crate::world::WorldState;

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
