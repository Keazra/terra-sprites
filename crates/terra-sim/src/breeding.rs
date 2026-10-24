//! Having young (M2 design §5, §10): sexes, fertility, laying an egg with a
//! partner or alone, the sprite an egg holds until it hatches, and the
//! content-safety rule the engine enforces for every genome and data pack,
//! that mating happens only between adults.

use serde::{Deserialize, Serialize};

use crate::action::{self, Hurt, Outcome};
use crate::courtship;
use crate::data::DataPack;
use crate::ecology;
use crate::events::{Event, EventKind};
use crate::genome::Genome;
use crate::inheritance;
use crate::lineage;
use crate::listed::listed_enum;
use crate::map::Dir;
use crate::mutation::{self, Copied, CopyChange, CopyingError};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::physiology::RoomNearby;
use crate::random::uniform;
use crate::registry::Verb;
use crate::sprites::{Genes, Sprite, check_genes};
use crate::world::WorldState;

listed_enum! {
    /// A sprite's sex (M2 design §10). Every sprite hatches male; an adult male
    /// turns female for good once its genes have it feel no female near for
    /// long enough. Only a female lays eggs.
    #[derive(
        Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
    )]
    pub enum Sex {
        #[default]
        Male => "male",
        Female => "female",
    }

    /// Both sexes, male first.
    pub const ALL;

    /// The sex's name, as the player sees it.
    pub fn name(self) -> &'static str;
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
    /// The generation it hatches into (M2 design §17.1): one after its newer
    /// parent's. 0 only in a save from before generations, until it's
    /// loaded.
    #[serde(default)]
    pub(crate) generation: u32,
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

/// Whether a sprite can breed now, and if its body could but room stops it,
/// which room (M2 design §10.2, §10.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fertility {
    /// It can breed.
    Fertile,
    /// Its body could breed, but the world is full (§10.6): no sprite can
    /// until one dies.
    WorldFull,
    /// Its body could breed, and the world has room, but too many other
    /// grown sprites are near it: it's too crowded to breed, with no room
    /// nearby (§10.2, §10.9).
    TooCrowded,
    /// Its body can't: it's young, hungry, thirsty or resting since its last
    /// egg, or its pack has no breeding.
    Infertile,
}

/// Whether `sprite` is fertile now (M2 design §10): see `fertility`.
pub(crate) fn fertile(state: &WorldState, sprite: &Sprite, data: &DataPack) -> bool {
    fertility(state, sprite, data) == Fertility::Fertile
}

/// Whether `sprite` can breed now (M2 design §10.2). It's fertile in a pack
/// that `breeds`, grown up, fed and watered, rested long enough since its
/// last egg, in a world with `room` for another sprite, and with room
/// nearby. A save from before breeding keeps its pack, which has none. The
/// world's room is asked first, since only a death makes it, and the room
/// nearby last, since it's the one that counts other sprites.
pub(crate) fn fertility(state: &WorldState, sprite: &Sprite, data: &DataPack) -> Fertility {
    let physiology = data.physiology();
    let (breeding, indices) = (&physiology.breeding, physiology.indices);
    let chems = &sprite.body.chems;
    let able = data.breeds()
        && sprite.stage().is_grown()
        && chems[indices.energy] > breeding.fed
        && chems[indices.hydration] > breeding.fed
        && sprite
            .last_bred
            .is_none_or(|last| state.tick >= last.saturating_add(breeding.rest));
    if !able {
        Fertility::Infertile
    } else if !room(state) {
        Fertility::WorldFull
    } else if breeding
        .room_nearby
        .is_some_and(|room| too_crowded(state, sprite, room))
    {
        Fertility::TooCrowded
    } else {
        Fertility::Fertile
    }
}

/// Whether `room`'s `fewer_than` or more other grown sprites, adult or old,
/// a partner included, are within its `within` tiles of `sprite` in a
/// straight line (M2 design §10.2, §10.9). Young sprites and eggs don't
/// count.
fn too_crowded(state: &WorldState, sprite: &Sprite, room: RoomNearby) -> bool {
    let reach = u64::from(room.within).pow(2);
    let near = state
        .sprites
        .iter()
        .filter(|&(_, other)| !std::ptr::eq(other, sprite) && other.stage().is_grown())
        .filter(|(_, other)| other.pos.distance_squared(sprite.pos) <= reach)
        .count();
    near >= usize::from(room.fewer_than)
}

/// Sprite `actor`, beside `target`, tries to mate with it, once (M2 design
/// §10). It works if both are adults, fertile, one male and one female,
/// she's receptive enough (§15.3), and she has a free tile beside her: she
/// lays an egg there, its genes mixed from both, and they're partners
/// (§15.4). Otherwise it's a fruitless try.
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
            let her = state.sprites.get(mother)?;
            let able = both && courtship::receptive(her, data);
            let egg = able.then(|| lay(state, data, mother, Some(father), events))??;
            courtship::pair(state, data, mother, father, events);
            Some(egg)
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
/// it's a fertile adult female who has gone `breeding.alone_after` ticks
/// with no male adult seen or heard (§15.7), with a free tile beside her:
/// she lays an egg there, its genes hers. Otherwise it's a fruitless try.
/// A pack from before that rule has her longing reach
/// `breeding.breeds_alone` instead.
pub(crate) fn breed_alone(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    events: &mut Vec<Event>,
) -> Outcome {
    let sprite = state.sprites.get(actor).expect("the actor");
    let physiology = data.physiology();
    let breeding = &physiology.breeding;
    let longing = physiology
        .indices
        .breeding
        .map_or(0.0, |indices| sprite.body.chems[indices.longing]);
    let alone = breeding
        .alone_after
        .is_none_or(|ticks| state.tick >= sprite.last_male.saturating_add(ticks));
    let longs = breeding.breeds_alone.is_none_or(|level| longing >= level);
    let able = sprite.sex == Sex::Female && alone && longs && fertile(state, sprite, data);
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
    let generation = lineage::generation_after(
        parents
            .iter()
            .map(|&parent| state.sprites.get(parent).expect("a parent").generation),
    );
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
        generation,
        laid: state.tick,
        name: None,
    }));
    let next_tick = state.tick + 1;
    let egg = state.add_object_from(egg, data, next_tick);
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
    use crate::test_util::{WorldBuilder, at, lone};
    use crate::world::World;

    /// The drawing of a small field of grass, 5 by 3 tiles.
    const FIELD: [&str; 3] = [".....", ".....", "....."];

    #[test]
    fn a_full_world_is_infertile_and_lays_nothing() {
        // M2 design §10.4: the limit counts eggs, and a sprite in a full
        // world isn't fertile. In a pack that lets a female breed alone at
        // once (§15.7).
        let data = lone();
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                InitialConcentration(chem: "feminiser", value: 1.0),
                InitialConcentration(chem: "longing", value: 1.0),
                Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
            ])"#,
            &data,
        )
        .expect("a valid genome");
        let mut world = WorldBuilder::new(&FIELD)
            .data(data)
            .sprites_with(vec![
                (at(1, 1), Some(genome.clone())),
                (at(3, 1), Some(genome)),
            ])
            .build();
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
    fn a_full_world_says_so_before_room_nearby() {
        // M2 design §10.6, §10.9: five sprites close together, each with
        // four others near, are too crowded to breed while the world has
        // room, and in a full world it's the world that's full, since only
        // a death can make room.
        let tiles = [at(0, 0), at(2, 0), at(4, 0), at(1, 2), at(3, 2)];
        let mut world = WorldBuilder::new(&FIELD).starters(&tiles).build();
        let fertility = |world: &World| world.sprites().map(|s| s.fertility()).collect::<Vec<_>>();
        assert_eq!(fertility(&world), [Fertility::TooCrowded; 5]);
        world.set_most_sprites(5);
        assert_eq!(fertility(&world), [Fertility::WorldFull; 5]);
    }

    #[test]
    fn the_cursor_may_spawn_a_sprite_past_most_sprites() {
        // M2 design §10.6: most sprites is the sprites' glass, not the
        // player's. In a full world the Cursor still places a sprite, and
        // none is fertile.
        let mut world = WorldBuilder::new(&FIELD)
            .starters(&[at(1, 1), at(3, 1)])
            .build();
        world.set_most_sprites(2);
        world.submit(Command::SpawnSprite {
            tile: at(2, 2),
            genome: None,
        });
        let events = world.step();
        let spawned = |event: &Event| matches!(event.kind, EventKind::Spawned { .. });
        assert!(events.iter().any(spawned), "{events:?}");
        assert_eq!(world.sprites().count(), 3);
        assert!(world.sprites().all(|sprite| !sprite.is_fertile()));
    }

    #[test]
    fn a_partner_keeps_the_touch_of_its_mate_whatever_it_touches_later_that_tick() {
        // M2 design §10: each learns about the other, even when the one
        // mated with acts after, on the same tick.
        let mut world = WorldBuilder::new(&FIELD)
            .starters(&[at(1, 1), at(2, 1), at(3, 1)])
            .build();
        let (state, data) = world.parts();
        let ids: Vec<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
        let (she, he, other) = (ids[0], ids[1], ids[2]);
        let indices = data.physiology().indices;
        for &id in &ids {
            let sprite = state.sprites.get_mut(id).expect("a sprite");
            sprite.body.chems[indices.energy] = 1.0;
            sprite.body.chems[indices.hydration] = 1.0;
        }
        let her = state.sprites.get_mut(she).expect("her");
        her.sex = Sex::Female;
        // Courted till she's receptive (M2 design §15.3).
        let courtship = indices.courtship.expect("a pack with courtship");
        her.body.chems[courtship.receptivity] = 1.0;
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
