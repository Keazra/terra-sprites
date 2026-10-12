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

/// Whether the world has room for another sprite, egg or not (M2 design
/// §10). In a full world no sprite is fertile, so none is aroused: the
/// glass of the terrarium.
pub(crate) fn room(state: &WorldState) -> bool {
    state.sprites.len() < usize::from(state.most_sprites)
}

/// Whether `sprite` is fertile now (M2 design §10): grown up, fed and
/// watered, rested long enough since its last egg, in a world with `room`
/// for another sprite.
pub(crate) fn fertile(state: &WorldState, sprite: &Sprite, data: &DataPack) -> bool {
    let physiology = data.physiology();
    let (breeding, indices) = (&physiology.breeding, physiology.indices);
    let chems = &sprite.body.chems;
    room(state)
        && sprite.stage().is_grown()
        && chems[indices.energy] > breeding.fed
        && chems[indices.hydration] > breeding.fed
        && sprite
            .last_bred
            .is_none_or(|last| state.tick >= last.saturating_add(breeding.rest))
}

/// Sprite `actor`, beside `target`, tries to mate with it, once (M2 design
/// §10). It works if both are adults, fertile, one male and one female,
/// and the female has a free tile beside her: she lays an egg there, its
/// genes mixed from both. Otherwise it's a fruitless try.
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
            let (a, b) = (state.sprites.get(actor)?, state.sprites.get(partner)?);
            let both = fertile(state, a, data) && fertile(state, b, data);
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
/// it's a fertile adult female with a free tile beside her: she lays an egg
/// there, its genes hers. Otherwise it's a fruitless try.
pub(crate) fn breed_alone(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    events: &mut Vec<Event>,
) -> Outcome {
    let sprite = state.sprites.get(actor).expect("the actor");
    let able = sprite.sex == Sex::Female && fertile(state, sprite, data);
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

/// `mother`, fertile, lays an egg on a free tile beside her, by `father` or
/// alone (M2 design §10), if she has a free tile. The parents split the
/// egg's cost, or she pays it all alone, and each fires its `bred` pulse.
/// Returns the egg.
fn lay(
    state: &mut WorldState,
    data: &DataPack,
    mother: EntityId,
    father: Option<EntityId>,
    events: &mut Vec<Event>,
) -> Option<EntityId> {
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
    let (breeding, indices) = (&physiology.breeding, physiology.indices);
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

#[cfg(test)]
mod tests {
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::map::Map;
    use crate::test_util::at;
    use crate::world::{Scenario, World};

    /// A genome whose instincts all have `weight`, one per verb listed.
    fn weighted(weight: f32, data: &DataPack) -> Genome {
        let genes: String = [
            "Eat", "Drink", "Play", "Hit", "Rest", "Wander", "Mate", "Approach",
        ]
        .iter()
        .map(|verb| {
            format!(r#"Instinct(inputs: [("always", false)], verb: {verb}, weight: {weight:?}),"#)
        })
        .collect();
        Genome::from_ron(&format!("(format: 1, genes: [{genes}])"), data).expect("a valid genome")
    }

    #[test]
    fn a_child_takes_each_gene_from_one_parent_or_the_other() {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let (mine, theirs) = (weighted(0.25, &data), weighted(0.75, &data));
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let child = mixed(&mine, &theirs, &mut rng);
        assert_eq!(child.genes.len(), mine.genes.len());
        let from_mine = child
            .genes
            .iter()
            .zip(&mine.genes)
            .filter(|(gene, mine)| gene == mine)
            .count();
        let from_theirs = child
            .genes
            .iter()
            .zip(&theirs.genes)
            .filter(|(gene, theirs)| gene == theirs)
            .count();
        assert_eq!(from_mine + from_theirs, child.genes.len());
        assert!(
            from_mine > 0 && from_theirs > 0,
            "{from_mine} and {from_theirs}"
        );
    }

    #[test]
    fn genomes_that_dont_line_up_give_a_copy_of_the_mothers() {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let mine = weighted(0.25, &data);
        let mut theirs = weighted(0.75, &data);
        theirs.genes.pop();
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        assert_eq!(mixed(&mine, &theirs, &mut rng), mine);
    }

    #[test]
    fn a_full_world_is_infertile_and_lays_nothing() {
        // M2 design §10: the limit counts eggs, and a sprite in a full world
        // isn't fertile.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                InitialConcentration(chem: "feminiser", value: 1.0),
                Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
            ])"#,
            &data,
        )
        .expect("a valid genome");
        let map = Map::from_ascii(&[".....", ".....", "....."], &data).expect("valid drawing");
        let sprites = [(at(1, 1), Some(genome.clone())), (at(3, 1), Some(genome))];
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted: &[],
            newborns: &[],
        };
        let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
        world.set_most_sprites(2);
        let laid = |world: &mut World| {
            (0..50).any(|_| {
                world
                    .step()
                    .iter()
                    .any(|event| matches!(event.kind, EventKind::Laid { .. }))
            })
        };
        assert!(!laid(&mut world), "an egg in a full world");
        assert!(world.sprites().all(|sprite| !sprite.is_fertile()));
        world.set_most_sprites(3);
        assert!(laid(&mut world), "no egg once there was room");
        assert_eq!(world.sprites().count(), 3);
        assert!(!laid(&mut world), "a second egg over the limit");
    }
}
