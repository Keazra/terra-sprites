//! What a verb does to its target (design §3.5.2): the effects its verb
//! table lists for the verb, run in order, once per attempt.

use crate::action::Outcome;
use crate::data::DataPack;
use crate::ecology::{self, Turn};
use crate::events::{DeathCause, Event};
use crate::object_types::{Effect, Party};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::registry::Verb;
use crate::world::WorldState;

/// Sprite `actor` applies `verb` to `target`, once: `failed` if the target's
/// verb table has no such verb or a `RequireCounter` isn't met, and then
/// nothing after it happens; `applied` otherwise.
pub(crate) fn attempt(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    verb: Verb,
    target: Target,
    events: &mut Vec<Event>,
) -> Outcome {
    let Some(kind) = state.kind_of(data, target) else {
        return Outcome::Failed;
    };
    let Some(effects) = data.object_types()[kind].verbs.get(&verb) else {
        return Outcome::Failed;
    };
    for effect in effects {
        match *effect {
            Effect::RequireCounter(counter, least) => {
                let Target::Object(id) = target else {
                    unreachable!("only objects have counters");
                };
                let object = state.objects.get(id).expect("the target");
                if object.counters[counter] < least {
                    return Outcome::Failed;
                }
            }
            Effect::Inject(party, chem, amount) => {
                if let Some(sprite) = party_sprite(actor, target, party) {
                    let index = data.chemical_index(chem).expect("a checked effect");
                    let injury = data.physiology().indices.injury;
                    let cause = DeathCause::HurtBy(data.object_types()[kind].id);
                    let body = &mut state.sprites.get_mut(sprite).expect("a sprite").body;
                    body.inject(index, amount, injury, cause);
                }
            }
            Effect::Signal(party, locus) => {
                if let Some(sprite) = party_sprite(actor, target, party) {
                    let index = data.locus_index(locus).expect("a checked effect");
                    state
                        .sprites
                        .get_mut(sprite)
                        .expect("a sprite")
                        .body
                        .incoming[index] = 1.0;
                }
            }
            Effect::Push(_) => unreachable!("Hit and Play, the verbs that push, arrive in slice 7"),
            Effect::AddCounter(..)
            | Effect::SpawnNearby(..)
            | Effect::SpreadTo(..)
            | Effect::ReplaceWith(..)
            | Effect::DestroySelf => {
                let Target::Object(id) = target else {
                    unreachable!("a pseudo type's verb table only injects and signals");
                };
                if ecology::apply(state, data, id, effect, events) == Turn::Ends {
                    break;
                }
            }
        }
    }
    Outcome::Applied
}

/// The sprite an effect for `party` acts on: the actor, or a target that's
/// a sprite. `None` for a target that isn't one.
fn party_sprite(actor: EntityId, target: Target, party: Party) -> Option<EntityId> {
    match (party, target) {
        (Party::Actor, _) => Some(actor),
        (Party::Target, Target::Sprite(id)) => Some(id),
        (Party::Target, _) => None,
    }
}
