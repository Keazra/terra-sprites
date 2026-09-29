//! What a verb does to its target (design §3.5.2): the effects its verb
//! table lists for the verb, run in order, once per attempt.

use crate::action::{Hurt, Outcome};
use crate::data::DataPack;
use crate::ecology::{self, Turn};
use crate::events::{DeathCause, Event};
use crate::object_types::{Effect, Party};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::registry::Verb;
use crate::rolling;
use crate::world::WorldState;

/// Sprite `actor` applies `verb` to `target`, once: `failed` if the target
/// has no verb table, its verb table has no such verb, or a `RequireCounter`
/// isn't met, and then nothing after it happens; `applied` otherwise. With
/// the outcome comes which sprites it hurt.
pub(crate) fn attempt(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    verb: Verb,
    target: Target,
    events: &mut Vec<Event>,
) -> (Outcome, Hurt) {
    let mut hurt = Hurt::default();
    // Water or a sprite in a pack with no object type for it has no verb
    // table, so no rule for any verb (design v20 §5.2).
    let rule = state
        .kind_of(data, target)
        .and_then(|kind| Some((kind, data.object_types()[kind].verbs.get(&verb)?)));
    let Some((kind, effects)) = rule else {
        fruitless(state, data, actor);
        return (Outcome::Failed, hurt);
    };
    for effect in effects {
        match *effect {
            Effect::RequireCounter(counter, least) => {
                let Target::Object(id) = target else {
                    unreachable!("only objects have counters");
                };
                let object = state.objects.get(id).expect("the target");
                if object.counters[counter] < least {
                    fruitless(state, data, actor);
                    return (Outcome::Failed, hurt);
                }
            }
            Effect::Inject(party, chem, amount) => {
                if let Some(sprite) = party_sprite(actor, target, party) {
                    let index = data.chemical_index(chem).expect("a checked effect");
                    let injury = data.physiology().indices.injury;
                    if index == injury && amount > 0.0 {
                        match party {
                            Party::Actor => hurt.actor = true,
                            Party::Target => hurt.target = true,
                        }
                    }
                    let cause = DeathCause::HurtBy(data.object_types()[kind].id);
                    let body = &mut state.sprites.get_mut(sprite).expect("a sprite").body;
                    body.inject(index, amount, injury, cause);
                }
            }
            Effect::Signal(party, locus) => {
                if let Some(sprite) = party_sprite(actor, target, party) {
                    let index = data.locus_index(locus).expect("a checked effect");
                    let body = &mut state.sprites.get_mut(sprite).expect("a sprite").body;
                    // A pulse on the target records the actor as its source
                    // (design §3.5.2).
                    let source = (party == Party::Target).then_some(actor);
                    body.pulse(index, source);
                }
            }
            Effect::Push(tiles) => {
                let Target::Object(id) = target else {
                    unreachable!("a pseudo type's verb table only injects and signals");
                };
                rolling::push(state, actor, id, tiles);
            }
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
    (Outcome::Applied, hurt)
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

/// A try that did nothing (design §5.2): the actor feels a `fruitless` pulse.
fn fruitless(state: &mut WorldState, data: &DataPack, actor: EntityId) {
    let index = data.physiology().indices.fruitless;
    let body = &mut state.sprites.get_mut(actor).expect("the actor").body;
    body.pulse(index, None);
}
