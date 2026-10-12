//! Having young (M2 design §5, §10): sexes, fertility, laying an egg with a
//! partner or alone, and the content-safety rule the engine enforces for
//! every genome and data pack, that mating happens only between adults.

use std::mem::discriminant;

use serde::{Deserialize, Serialize};

use crate::action::{self, Hurt, Outcome};
use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::genome::Genome;
use crate::map::Dir;
use crate::objects::EntityId;
use crate::perception::Target;
use crate::random::{chance, uniform};
use crate::registry::Verb;
use crate::sprites::Sprite;
use crate::world::WorldState;

/// A sprite's sex (M2 design §10). Every sprite hatches male; an adult male
/// turns female for good once its genes have it feel no female near for
/// long enough. Only a female lays eggs.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Sex {
    #[default]
    Male,
    Female,
}

impl Sex {
    /// The sex's name, as the player sees it.
    pub fn name(self) -> &'static str {
        match self {
            Sex::Male => "male",
            Sex::Female => "female",
        }
    }
}

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

/// Whether `sprite` is fertile on the tick `tick` (M2 design §10): grown up,
/// fed and watered, and rested long enough since its last egg.
pub(crate) fn fertile(sprite: &Sprite, tick: u64, data: &DataPack) -> bool {
    let physiology = data.physiology();
    let (breeding, indices) = (physiology.breeding, physiology.indices);
    let chems = &sprite.body.chems;
    sprite.stage().is_grown()
        && chems[indices.energy] > breeding.fed
        && chems[indices.hydration] > breeding.fed
        && sprite
            .last_bred
            .is_none_or(|last| tick >= last + breeding.rest)
}

/// Sprite `actor`, beside `target`, tries to mate with it, once (M2 design
/// §10). It works if both are adults, fertile, one male and one female,
/// the world has room for one more sprite, and the female has a free tile
/// beside her: she lays an egg there, its genes mixed from both. Otherwise
/// it's a fruitless try.
pub(crate) fn mate(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    target: Target,
    events: &mut Vec<Event>,
) -> (Outcome, Hurt) {
    let laid = target
        .sprite()
        .filter(|&partner| adults(state, actor, partner))
        .and_then(|partner| {
            let (tick, sprites) = (state.tick, &state.sprites);
            let (a, b) = (sprites.get(actor)?, sprites.get(partner)?);
            let both = fertile(a, tick, data) && fertile(b, tick, data);
            let (mother, father) = match (a.sex, b.sex) {
                (Sex::Female, Sex::Male) => (actor, partner),
                (Sex::Male, Sex::Female) => (partner, actor),
                _ => return None,
            };
            both.then(|| lay(state, data, mother, Some(father), events))?
        });
    // The partner touched the actor too, so each learns about the other
    // (M2 design §10).
    if let (Some(_), Some(partner)) = (laid, target.sprite()) {
        let me = Target::Sprite(actor);
        let subject = state.subject_of(data, me);
        action::touched(state, data, partner, me, subject, Some(Verb::Mate));
    }
    outcome(state, data, actor, laid)
}

/// Sprite `actor` tries to breed alone, once (M2 design §10). It works if
/// it's a fertile adult female, the world has room for one more sprite,
/// and she has a free tile beside her: she lays an egg there, its genes
/// hers. Otherwise it's a fruitless try.
pub(crate) fn breed_alone(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    events: &mut Vec<Event>,
) -> Outcome {
    let sprite = state.sprites.get(actor).expect("the actor");
    let able = sprite.sex == Sex::Female && fertile(sprite, state.tick, data);
    let laid = able
        .then(|| lay(state, data, actor, None, events))
        .flatten();
    outcome(state, data, actor, laid).0
}

/// What a try to breed came to: a fruitless one, if no egg was `laid`.
fn outcome(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    laid: Option<EntityId>,
) -> (Outcome, Hurt) {
    if laid.is_some() {
        return (Outcome::Applied, Hurt::default());
    }
    let index = data.physiology().indices.fruitless;
    let body = &mut state.sprites.get_mut(actor).expect("the actor").body;
    body.pulse(index, None);
    (Outcome::Failed, Hurt::default())
}

/// `mother` lays an egg on a free tile beside her, by `father` or alone (M2
/// design §10), if the world has room for another sprite and she has a free
/// tile. The parents split the egg's cost, or she pays it all alone, and
/// each fires its `bred` pulse. Returns the egg.
fn lay(
    state: &mut WorldState,
    data: &DataPack,
    mother: EntityId,
    father: Option<EntityId>,
    events: &mut Vec<Event>,
) -> Option<EntityId> {
    if state.sprites.len() >= usize::from(state.most_sprites) {
        return None;
    }
    let pos = state.sprites.get(mother).expect("the mother").pos;
    let free: Vec<_> = Dir::ALL
        .into_iter()
        .filter_map(|dir| state.map.neighbour(pos, dir))
        .filter(|&tile| state.can_stand(data, tile))
        .collect();
    if free.is_empty() {
        return None;
    }
    let tile = free[uniform(&mut state.rng, free.len() as u64) as usize];
    let genome = {
        let sprites = &state.sprites;
        let mine = &sprites.get(mother).expect("the mother").genome;
        match father.and_then(|father| sprites.get(father)) {
            Some(theirs) => mixed(mine, &theirs.genome, &mut state.rng),
            None => mine.clone(),
        }
    };
    let parents: Vec<EntityId> = std::iter::once(mother).chain(father).collect();
    let physiology = data.physiology();
    let (breeding, indices) = (physiology.breeding, physiology.indices);
    let share = breeding.cost / parents.len() as f32;
    for &parent in &parents {
        let sprite = state.sprites.get_mut(parent).expect("a parent");
        let energy = &mut sprite.body.chems[indices.energy];
        *energy = (*energy - share).max(0.0);
        sprite.last_bred = Some(state.tick);
        if let Some(breeding) = indices.breeding {
            sprite.body.pulse(breeding.bred, None);
        }
    }
    let egg = Sprite::egg(genome, tile, state.tick, parents, data);
    let egg = state.add_sprite(egg);
    events.push(Event {
        tick: state.tick,
        kind: EventKind::Laid {
            egg,
            by: mother,
            with: father,
        },
    });
    Some(egg)
}

/// A child's genome from its parents', `mine` and `theirs` (M2 design §10):
/// each gene, with its switch, from one or the other at random, where the
/// two line up gene for gene, as genomes from the starter do; or else a
/// copy of `mine`, the mother's. The damage old age did to a parent's genes
/// isn't passed on, as it's in the body, not the genome.
pub(crate) fn mixed(mine: &Genome, theirs: &Genome, rng: &mut rand_chacha::ChaCha8Rng) -> Genome {
    let lined_up = mine.genes.len() == theirs.genes.len()
        && mine
            .genes
            .iter()
            .zip(&theirs.genes)
            .all(|(a, b)| discriminant(a) == discriminant(b));
    if !lined_up {
        return mine.clone();
    }
    let switch =
        |genome: &Genome, index: usize| genome.switches.get(index).copied().unwrap_or_default();
    let (mut genes, mut switches) = (Vec::new(), Vec::new());
    for index in 0..mine.genes.len() {
        let from = if chance(rng, 0.5) { mine } else { theirs };
        genes.push(from.genes[index].clone());
        switches.push(switch(from, index));
    }
    // An empty list of switches means none switch.
    if switches.iter().all(|switch| switch.is_always()) {
        switches.clear();
    }
    Genome { genes, switches }
}
