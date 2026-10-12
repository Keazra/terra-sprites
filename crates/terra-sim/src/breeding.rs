//! Having young (M2 design §5): what a Mate attempt does, and the
//! content-safety rule the engine enforces for every genome and data pack,
//! that mating happens only between adults.

use crate::action::{Hurt, Outcome};
use crate::data::DataPack;
use crate::objects::EntityId;
use crate::world::WorldState;

/// Whether sprites `a` and `b` may mate (M2 design §5): two different
/// sprites, both grown up. Old sprites still count as adults.
pub(crate) fn adults(state: &WorldState, a: EntityId, b: EntityId) -> bool {
    let grown = |id| {
        state
            .sprites
            .get(id)
            .is_some_and(|sprite| sprite.stage().is_grown())
    };
    a != b && grown(a) && grown(b)
}

/// Sprite `actor`, beside `target`, tries to mate with it, once. Until
/// breeding's rules are settled (Slice 23's design round) every try is
/// fruitless.
pub(crate) fn mate(state: &mut WorldState, data: &DataPack, actor: EntityId) -> (Outcome, Hurt) {
    let index = data.physiology().indices.fruitless;
    let body = &mut state.sprites.get_mut(actor).expect("the actor").body;
    body.pulse(index, None);
    (Outcome::Failed, Hurt::default())
}
