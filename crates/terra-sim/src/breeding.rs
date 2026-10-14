//! Having young (M2 design §5, §10): sexes, fertility, laying an egg with a
//! partner or alone, the sprite an egg holds until it hatches, and the
//! content-safety rule the engine enforces for every genome and data pack,
//! that mating happens only between adults.

use serde::{Deserialize, Serialize};

use crate::action::{self, Hurt, Outcome};
use crate::data::DataPack;
use crate::ecology;
use crate::events::{Event, EventKind};
use crate::genome::Genome;
use crate::inheritance;
use crate::map::Dir;
use crate::mutation::{self, Copied, CopyChange, CopyingError};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::random::uniform;
use crate::registry::Verb;
use crate::sprites::{Genes, Sprite, check_genes};
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

/// The sprite an egg holds until it hatches (M2 design §10.4, #196): its
/// genes as copying left them, who laid it, when, and the name the player
/// gave it, if any. The egg is an item; the sprite is born as it hatches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Unhatched {
    pub(crate) genome: Genome,
    /// How many of its two copies of each gene are flawed (M2 design §12):
    /// empty when none are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) flaws: Vec<u8>,
    /// The genes copying changed (M2 design §12).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) copying_errors: Vec<CopyingError>,
    /// The whole genes copying doubled, lost or changed (M2 design §13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) copy_changes: Vec<CopyChange>,
    /// The one that laid it, then its partner, if it had one.
    pub(crate) parents: Vec<EntityId>,
    /// The tick it was laid on.
    pub(crate) laid: u64,
    /// The name the player gave it, which the sprite keeps (design §2.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
}

impl Unhatched {
    /// Checks that the egg `id`'s genes fit together, as a sprite's must
    /// (design §7.1), or says how they don't.
    pub(crate) fn check(&self, id: EntityId, data: &DataPack) -> Result<(), String> {
        check_genes(
            id,
            Genes {
                genome: &self.genome,
                flaws: &self.flaws,
                copying_errors: &self.copying_errors,
                copy_changes: &self.copy_changes,
                parents: &self.parents,
            },
            data,
        )
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

/// Whether the world has room for another sprite, counting each egg as
/// the sprite it holds (M2 design §10.4). In a full world no sprite is
/// fertile, so none is aroused: the glass of the terrarium.
pub(crate) fn room(state: &WorldState) -> bool {
    state.sprites.len() + state.objects.eggs() < usize::from(state.most_sprites)
}

/// Whether `sprite` is fertile now (M2 design §10): in a pack with
/// breeding and an egg type, grown up, fed and watered, rested long enough
/// since its last egg, in a world with `room` for another sprite. A save
/// from before breeding keeps its pack, which has none.
pub(crate) fn fertile(state: &WorldState, sprite: &Sprite, data: &DataPack) -> bool {
    let physiology = data.physiology();
    let (breeding, indices) = (&physiology.breeding, physiology.indices);
    let chems = &sprite.body.chems;
    indices.breeding.is_some()
        && data.egg_type().is_some()
        && room(state)
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
/// it's a fertile adult female whose longing has reached
/// `breeding.breeds_alone`, with a free tile beside her: she lays an egg
/// there, its genes hers. Otherwise it's a fruitless try.
pub(crate) fn breed_alone(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    events: &mut Vec<Event>,
) -> Outcome {
    let sprite = state.sprites.get(actor).expect("the actor");
    let physiology = data.physiology();
    let longing = physiology
        .indices
        .breeding
        .map_or(0.0, |indices| sprite.body.chems[indices.longing]);
    let able = sprite.sex == Sex::Female
        && longing >= physiology.breeding.breeds_alone
        && fertile(state, sprite, data);
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
/// alone (M2 design §10), if she has one: a tile a sprite could stand on,
/// with no item on it, for the egg is an item until it hatches (§10.4). Its
/// genes are theirs mixed, or hers doubled, copied with errors (§12). The
/// parents split the egg's cost, or she pays it all alone, and each fires
/// its `bred` pulse. Returns the egg.
fn lay(
    state: &mut WorldState,
    data: &DataPack,
    mother: EntityId,
    father: Option<EntityId>,
    events: &mut Vec<Event>,
) -> Option<EntityId> {
    let kind = data.egg_type()?;
    let pos = state.sprites.get(mother).expect("the mother").pos;
    let free: Vec<_> = Dir::ALL
        .into_iter()
        .filter_map(|dir| state.map.neighbour(pos, dir))
        .filter(|&tile| state.can_lie(data, kind, tile))
        .collect();
    if free.is_empty() {
        return None;
    }
    let tile = free[uniform(&mut state.rng, free.len() as u64) as usize];
    let Copied {
        genome,
        flaws,
        errors,
        changes,
    } = {
        let sprites = &state.sprites;
        let hers = sprites.get(mother).expect("the mother").as_parent();
        let inherited = match father.and_then(|father| sprites.get(father)) {
            Some(him) => inheritance::of_pair(hers, him.as_parent(), data, &mut state.rng),
            None => inheritance::alone(hers, &mut state.rng),
        };
        // Copied with errors (M2 design §12).
        mutation::copied(inherited, data, &mut state.rng)
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
    let mut egg = ecology::started_object(data, &mut state.rng, kind, tile, state.tick);
    egg.unhatched = Some(Box::new(Unhatched {
        genome,
        flaws,
        copying_errors: errors,
        copy_changes: changes,
        parents,
        laid: state.tick,
        name: None,
    }));
    let egg = state.add_object(egg);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::map::Map;
    use crate::test_util::at;
    use crate::world::{Scenario, World};

    #[test]
    fn a_full_world_is_infertile_and_lays_nothing() {
        // M2 design §10.4: the limit counts eggs, and a sprite in a full
        // world isn't fertile.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                InitialConcentration(chem: "feminiser", value: 1.0),
                InitialConcentration(chem: "longing", value: 1.0),
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
        assert_eq!((world.sprites().count(), world.eggs().count()), (2, 1));
        assert!(!laid(&mut world), "a second egg over the limit");
        // Held by the Cursor, the egg still counts.
        let egg = world.eggs().next().expect("the egg").id();
        world.submit(Command::PickUp { item: egg });
        world.step();
        assert!(world.cursor().holds().is_some_and(|held| held.id() == egg));
        assert!(!laid(&mut world), "a second egg while the first is held");
    }

    #[test]
    fn a_partner_keeps_the_touch_of_its_mate_whatever_it_touches_later_that_tick() {
        // M2 design §10: each learns about the other, even when the one
        // mated with acts after, on the same tick.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let map = Map::from_ascii(&[".....", ".....", "....."], &data).expect("valid drawing");
        let sprites = [(at(1, 1), None), (at(2, 1), None), (at(3, 1), None)];
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted: &[],
            newborns: &[],
        };
        let mut world = World::from_scenario(scenario, data, 1).expect("valid scenario");
        let (state, data) = world.parts();
        let ids: Vec<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
        let (she, he, other) = (ids[0], ids[1], ids[2]);
        let indices = data.physiology().indices;
        for &id in &ids {
            let sprite = state.sprites.get_mut(id).expect("a sprite");
            sprite.body.chems[indices.energy] = 1.0;
            sprite.body.chems[indices.hydration] = 1.0;
        }
        state.sprites.get_mut(she).expect("her").sex = Sex::Female;
        let mut events = Vec::new();
        mate(state, data, he, Target::Sprite(she), &mut events);
        assert!(
            events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Laid { .. })),
            "they mated"
        );
        // Her own try, later that tick, is on another sprite.
        let subject = state.subject_of(data, Target::Sprite(other));
        action::touched(
            state,
            data,
            she,
            Target::Sprite(other),
            subject,
            Some(Verb::Mate),
        );
        let touch = state.sprites.get(she).expect("her").brain.touched;
        let touch = touch.expect("a touch");
        assert_eq!((touch.verb, touch.sprite), (Some(Verb::Mate), Some(he)));
        // The next tick, she touches what she touches.
        state.tick += 1;
        action::touched(
            state,
            data,
            she,
            Target::Sprite(other),
            subject,
            Some(Verb::Mate),
        );
        let touch = state.sprites.get(she).expect("her").brain.touched;
        assert_eq!(touch.expect("a touch").sprite, Some(other));
    }
}
