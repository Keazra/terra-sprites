//! What a verb does to its target (design §3.5.2): the effects its tags
//! give the contact (design v23 §3.5.6), then those its verb table lists for
//! the verb, run in order, once per attempt.

use crate::action::{Hurt, Outcome};
use crate::data::DataPack;
use crate::ecology::{self, Turn};
use crate::events::{DeathCause, Event};
use crate::object_types::{Effect, Party};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::registry::{ChemId, LocusId, Verb};
use crate::rolling;
use crate::tags::Contact;
use crate::world::WorldState;

/// Sprite `actor` applies `verb` to `target`, once: `failed` if neither the
/// target's tags nor its verb table have a rule for it, or a `RequireCounter`
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
    let Some(kind) = state.kind_of(data, target) else {
        fruitless(state, data, actor);
        return (Outcome::Failed, hurt);
    };
    // A contact runs the thing's tags first, in the order it lists them,
    // then its own verb table; a try a tag answers isn't fruitless (design
    // v23 §3.5.6).
    let tagged = Contact::of(verb).map_or_else(Vec::new, |contact| tagged(data, kind, contact));
    let table = data.object_types()[kind]
        .verbs
        .get(&verb)
        .map(Vec::as_slice);
    if tagged.is_empty() && table.is_none() {
        fruitless(state, data, actor);
        return (Outcome::Failed, hurt);
    }
    for effect in tagged.into_iter().chain(table).flatten() {
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
                if let Some(sprite) = party_sprite(actor, target, party)
                    && inject(state, data, sprite, chem, amount, kind)
                {
                    match party {
                        Party::Actor => hurt.actor = true,
                        Party::Target => hurt.target = true,
                    }
                }
            }
            Effect::Signal(party, locus) => {
                if let Some(sprite) = party_sprite(actor, target, party) {
                    // A pulse on the target records the actor as its source
                    // (design §3.5.2).
                    let source = (party == Party::Target).then_some(actor);
                    signal(state, data, sprite, locus, source);
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

/// Sprite `actor`, sliding, crashes into `into` (design v25 §3.5.4): a
/// contact, so the Crash effects of its tags run on `actor` (design v23
/// §3.5.6). Returns whether it hurt. A sprite has no tags, so a crash into
/// one does nothing to either.
pub(crate) fn crash(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    into: Target,
) -> bool {
    let Some(kind) = state.kind_of(data, into) else {
        return false;
    };
    let mut hurt = false;
    for effect in tagged(data, kind, Contact::Crash).into_iter().flatten() {
        match *effect {
            Effect::Inject(_, chem, amount) => {
                hurt |= inject(state, data, actor, chem, amount, kind)
            }
            Effect::Signal(_, locus) => signal(state, data, actor, locus, None),
            _ => unreachable!("a tag only injects into and signals the actor"),
        }
    }
    hurt
}

/// The effects the tags of a thing of the object type `kind` give
/// `contact`, tag by tag, in the order the type lists them (design v23
/// §3.5.6). Empty if no tag has a rule for it.
fn tagged(data: &DataPack, kind: usize, contact: Contact) -> Vec<&[Effect]> {
    let tags = data.object_types()[kind].tags.iter();
    tags.filter_map(|&tag| data.tags()[tag].contact.get(&contact))
        .map(Vec::as_slice)
        .collect()
}

/// Injects `amount` of `chem` into `sprite`, from a thing of the object type
/// `kind`, to which any injury it adds is put down (design §3.5.2). Returns
/// whether it injured the sprite.
fn inject(
    state: &mut WorldState,
    data: &DataPack,
    sprite: EntityId,
    chem: ChemId,
    amount: f32,
    kind: usize,
) -> bool {
    let index = data.chemical_index(chem).expect("a checked effect");
    let injury = data.physiology().indices.injury;
    let cause = DeathCause::HurtBy(data.object_types()[kind].id);
    let body = &mut state.sprites.get_mut(sprite).expect("a sprite").body;
    body.inject(index, amount, injury, cause);
    index == injury && amount > 0.0
}

/// Writes a pulse of `locus` to `sprite`, from `source` if it has one.
fn signal(
    state: &mut WorldState,
    data: &DataPack,
    sprite: EntityId,
    locus: LocusId,
    source: Option<EntityId>,
) {
    let index = data.locus_index(locus).expect("a checked effect");
    let body = &mut state.sprites.get_mut(sprite).expect("a sprite").body;
    body.pulse(index, source);
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
