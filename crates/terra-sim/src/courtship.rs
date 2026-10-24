//! Calls, getting to know each other, courtship, partners and kin (M2 design
//! §15). The world says who's whose, how far a call carries and who may
//! court whom; the genes say what a sprite feels about it.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use serde::{Deserialize, Serialize};

use crate::action::{Hurt, Outcome, chebyshev};
use crate::breeding::{self, Sex};
use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::family::FamilySenses;
use crate::map::{Dir, Map, Pos};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::registry::BrainParam;
use crate::sprites::Sprite;
use crate::terrain::Terrain;
use crate::world::WorldState;

/// A sprite's partner (M2 design §15.4): the sprite it last had an egg
/// with, while neither has died nor gone too long apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Partner {
    pub(crate) id: EntityId,
    /// The tick it last saw or heard its partner.
    pub(crate) seen: u64,
    /// Whether its partner has died. It's free then, and keeps the name
    /// until it pairs again, for the Body tab.
    pub(crate) died: bool,
}

/// A call a sprite heard (M2 design §15.1): who called, from where, and
/// when. It knows the latest call of each caller for `calls.heard_for`
/// ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Heard {
    pub(crate) caller: EntityId,
    pub(crate) from: Pos,
    pub(crate) tick: u64,
}

impl Heard {
    /// Whether it's still known at `tick`, within `heard_for` ticks of the
    /// call.
    pub(crate) fn fresh(&self, tick: u64, heard_for: u64) -> bool {
        tick < self.tick + heard_for
    }

    /// How fresh it is at `tick`, from 1 as it's heard to 0 once
    /// `heard_for` ticks have passed: what draws the eye to it, as a
    /// remembered place's recall does (M2 design §15.1).
    pub(crate) fn freshness(&self, tick: u64, heard_for: u64) -> f32 {
        let since = (tick - self.tick) as f32;
        (1.0 - since / heard_for as f32).max(0.0)
    }
}

impl Sprite {
    /// Its partner, while the partner lives (M2 design §15.4).
    pub(crate) fn partner(&self) -> Option<EntityId> {
        self.partner.filter(|p| !p.died).map(|p| p.id)
    }

    /// The latest call it heard from `caller` within `heard_for` ticks of
    /// `tick`, if any (M2 design §15.1).
    pub(crate) fn heard_from(&self, caller: EntityId, tick: u64, heard_for: u64) -> Option<Heard> {
        self.heard
            .iter()
            .find(|heard| heard.caller == caller && heard.fresh(tick, heard_for))
            .copied()
    }

    /// Forgets the call it heard from `caller` (M2 design §15.1): it went
    /// to where the call came from and didn't find the caller, or found no
    /// way there.
    pub(crate) fn forget_call(&mut self, caller: EntityId) {
        self.heard.retain(|heard| heard.caller != caller);
    }
}

/// Sprites `a` and `b` played together, so each knows the other better by
/// its own `befriend_play` (M2 design §15.2), while learning is on, in a
/// pack with courtship.
pub(crate) fn played(state: &mut WorldState, data: &DataPack, a: EntityId, b: EntityId) {
    let physiology = data.physiology();
    if !state.learning || physiology.indices.courtship.is_none() {
        return;
    }
    let needs = data.need_places().len();
    for (id, other) in [(a, b), (b, a)] {
        let Some(sprite) = state.sprites.get_mut(id) else {
            continue;
        };
        let brain = &mut sprite.brain;
        let gain = brain.params.get(BrainParam::BefriendPlay);
        let below = physiology.forget_below;
        brain.experience.befriend(other, gain, needs, below);
    }
}

/// Whether sprites `a` and `b` are close kin (M2 design §15.5): one a parent
/// of the other, or brothers or sisters, with a parent in common.
pub(crate) fn kin(state: &WorldState, a: EntityId, b: EntityId) -> bool {
    let parents = |id| {
        state
            .sprites
            .get(id)
            .map_or(&[][..], |s| s.parents.as_slice())
    };
    let (of_a, of_b) = (parents(a), parents(b));
    a != b && (of_a.contains(&b) || of_b.contains(&a) || of_a.iter().any(|p| of_b.contains(p)))
}

/// What a call from `from` weakens by on its way to each tile within
/// `reach` of it (M2 design §15.1): a step costs the sound of the tile it
/// enters, a diagonal step 14/10 of it, and it never crosses a tile that
/// stops sound, nor cuts a corner past one.
pub(crate) struct Sound {
    corner: Pos,
    width: u16,
    costs: Vec<u32>,
}

impl Sound {
    /// The furthest a call out to `reach` can go, in tiles in any
    /// direction: as far as the terrain that weakens it least carries it.
    pub(crate) fn radius(data: &DataPack, reach: u32) -> u16 {
        let least = Terrain::ALL
            .iter()
            .filter_map(|&terrain| data.terrain(terrain).sound())
            .min();
        least.map_or(0, |least| {
            u16::try_from(reach / u32::from(least)).unwrap_or(u16::MAX)
        })
    }

    /// The call from `from` on `map`, out to `reach`.
    pub(crate) fn spread(map: &Map, data: &DataPack, from: Pos, reach: u32) -> Sound {
        let sound = |pos: Pos| data.terrain(map.terrain(pos)).sound();
        let radius = Sound::radius(data, reach);
        let corner = Pos {
            x: from.x.saturating_sub(radius),
            y: from.y.saturating_sub(radius),
        };
        let far = Pos {
            x: from.x.saturating_add(radius).min(map.width() - 1),
            y: from.y.saturating_add(radius).min(map.height() - 1),
        };
        let (width, height) = (far.x - corner.x + 1, far.y - corner.y + 1);
        let mut field = Sound {
            corner,
            width,
            costs: vec![u32::MAX; usize::from(width) * usize::from(height)],
        };
        let start = field.index(from).expect("the caller's tile");
        field.costs[start] = 0;
        let mut waiting = BinaryHeap::from([Reverse((0, start))]);
        while let Some(Reverse((cost, index))) = waiting.pop() {
            if cost > field.costs[index] {
                continue;
            }
            let here = field.pos(index);
            for dir in Dir::ALL {
                let Some(next) = map.neighbour(here, dir) else {
                    continue;
                };
                let (Some(index), Some(step)) = (field.index(next), sound(next)) else {
                    continue;
                };
                let step = u32::from(step);
                let step = if dir.is_diagonal() {
                    // No corner cut past a tile that stops sound.
                    let beside = [
                        Pos {
                            x: next.x,
                            y: here.y,
                        },
                        Pos {
                            x: here.x,
                            y: next.y,
                        },
                    ];
                    if beside.iter().any(|&pos| sound(pos).is_none()) {
                        continue;
                    }
                    step * 14 / 10
                } else {
                    step
                };
                let cost = cost + step;
                if cost <= reach && cost < field.costs[index] {
                    field.costs[index] = cost;
                    waiting.push(Reverse((cost, index)));
                }
            }
        }
        field
    }

    /// Whether the call is heard on `pos`.
    pub(crate) fn heard_on(&self, pos: Pos) -> bool {
        self.index(pos)
            .is_some_and(|index| self.costs[index] != u32::MAX)
    }

    fn index(&self, pos: Pos) -> Option<usize> {
        let (x, y) = (
            pos.x.checked_sub(self.corner.x)?,
            pos.y.checked_sub(self.corner.y)?,
        );
        let height = self.costs.len() / usize::from(self.width);
        (x < self.width && usize::from(y) < height)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    fn pos(&self, index: usize) -> Pos {
        let width = usize::from(self.width);
        Pos {
            x: self.corner.x + (index % width) as u16,
            y: self.corner.y + (index / width) as u16,
        }
    }
}

/// Sprite `caller` calls, once, where it stands (M2 design §15.1): it pays
/// `calls.cost` energy, and every other sprite the call reaches hears it,
/// with a `heard` pulse whose source is the caller, and knows where it came
/// from.
pub(crate) fn call(state: &mut WorldState, data: &DataPack, caller: EntityId) {
    let physiology = data.physiology();
    let Some(indices) = physiology.indices.courtship else {
        return;
    };
    let calls = physiology.calls;
    let tick = state.tick;
    let sprite = state.sprites.get_mut(caller).expect("the caller");
    let energy = &mut sprite.body.chems[physiology.indices.energy];
    *energy = (*energy - calls.cost).max(0.0);
    let from = sprite.pos;
    // Only spread it if anyone may hear it: most calls go unheard.
    let radius = Sound::radius(data, calls.reach);
    let near: Vec<EntityId> = state
        .sprites
        .iter()
        .filter(|&(id, other)| id != caller && chebyshev(from, other.pos) <= radius)
        .map(|(id, _)| id)
        .collect();
    if near.is_empty() {
        return;
    }
    let sound = Sound::spread(&state.map, data, from, calls.reach);
    for id in near {
        let hearer = state.sprites.get_mut(id).expect("a hearer");
        if !sound.heard_on(hearer.pos) {
            continue;
        }
        hearer.body.pulse(indices.heard, Some(caller));
        hearer.heard.retain(|heard| heard.caller != caller);
        hearer.heard.push(Heard { caller, from, tick });
    }
}

/// Sprite `actor`, beside `target`, dances for it, once (M2 design §15.3):
/// the one danced for feels a `courted` pulse whose source is the dancer,
/// and the dancer a `danced` pulse. Only between adults (M2 design §5), in
/// a pack with courtship; otherwise it's a fruitless try.
pub(crate) fn court(
    state: &mut WorldState,
    data: &DataPack,
    actor: EntityId,
    target: Target,
) -> (Outcome, Hurt) {
    let courtship = data.physiology().indices.courtship;
    let courted = target
        .sprite()
        .filter(|&other| breeding::adults(state, actor, other));
    let (Some(indices), Some(other)) = (courtship, courted) else {
        let fruitless = data.physiology().indices.fruitless;
        let body = &mut state.sprites.get_mut(actor).expect("the actor").body;
        body.pulse(fruitless, None);
        return (Outcome::Failed, Hurt::default());
    };
    let them = &mut state
        .sprites
        .get_mut(other)
        .expect("the one danced for")
        .body;
    them.pulse(indices.courted, Some(actor));
    let body = &mut state.sprites.get_mut(actor).expect("the actor").body;
    body.pulse(indices.danced, None);
    (Outcome::Applied, Hurt::default())
}

/// Whether `mother` is receptive enough for a pair to mate (M2 design
/// §15.3): her `receptivity` at least `breeding.receptive`. A pack with no
/// courtship has no such rule.
pub(crate) fn receptive(mother: &Sprite, data: &DataPack) -> bool {
    let physiology = data.physiology();
    physiology.indices.courtship.is_none_or(|indices| {
        mother.body.chems[indices.receptivity] >= physiology.breeding.receptive
    })
}

/// `mother` and `father` have had an egg together, so they're partners
/// (M2 design §15.4), replacing any partners they had, who are freed: the
/// first egg together says so. In a pack with no courtship there are no
/// partners.
pub(crate) fn pair(
    state: &mut WorldState,
    data: &DataPack,
    mother: EntityId,
    father: EntityId,
    events: &mut Vec<Event>,
) {
    if data.physiology().indices.courtship.is_none() {
        return;
    }
    let tick = state.tick;
    let already = [(mother, father), (father, mother)].iter().all(|&(a, b)| {
        state
            .sprites
            .get(a)
            .is_some_and(|sprite| sprite.partner() == Some(b))
    });
    for (a, b) in [(mother, father), (father, mother)] {
        let left = state.sprites.get(a).and_then(Sprite::partner);
        if let Some(left) = left.filter(|&left| left != b) {
            let them = state.sprites.get_mut(left).expect("a living partner");
            them.partner = None;
            events.push(Event {
                tick,
                kind: EventKind::Parted { a, b: left },
            });
        }
        let sprite = state.sprites.get_mut(a).expect("a parent");
        sprite.partner = Some(Partner {
            id: b,
            seen: tick,
            died: false,
        });
    }
    if !already {
        events.push(Event {
            tick,
            kind: EventKind::Paired {
                a: mother,
                b: father,
            },
        });
    }
}

/// Each sprite whose partner is among the `dead` loses it (M2 design §15.4):
/// it's free, and feels a `lost_partner` pulse.
pub(crate) fn mourn(
    state: &mut WorldState,
    data: &DataPack,
    dead: &[EntityId],
    events: &mut Vec<Event>,
) {
    let Some(indices) = data.physiology().indices.courtship else {
        return;
    };
    let tick = state.tick;
    for (id, sprite) in state.sprites.iter_mut() {
        let Some(partner) = sprite.partner.as_mut().filter(|p| !p.died) else {
            continue;
        };
        if dead.contains(&partner.id) {
            partner.died = true;
            sprite.body.pulse(indices.lost_partner, None);
            events.push(Event {
                tick,
                kind: EventKind::LostPartner {
                    id,
                    partner: partner.id,
                },
            });
        }
    }
}

/// What each sprite senses of others this tick, by sight and by hearing
/// (M2 design §10.1, §15): whom it sees, whether it sees or has lately
/// heard a female adult, a male adult, and its partner, and what it senses
/// of its family (§15.6).
#[derive(Debug, Clone, Default)]
pub(crate) struct Company {
    /// The other sprites it sees, in ascending ID order: within its sense
    /// radius, in any direction.
    pub(crate) seen: Vec<EntityId>,
    pub(crate) female_near: bool,
    pub(crate) male_near: bool,
    pub(crate) partner_near: bool,
    pub(crate) family: FamilySenses,
}

/// Step 3's look round (M2 design §15): each sprite forgets calls heard
/// more than `calls.heard_for` ago or from a sprite gone, and senses its
/// company; one that sees or hears a male adult keeps the tick, for
/// breeding alone (§15.7), and one that sees or hears its partner keeps
/// that tick too. Partners who've gone `breeding.apart` ticks with neither
/// seeing nor hearing the other are freed together (§15.4). Returns each
/// sprite's company, in ascending ID order.
pub(crate) fn look_round(
    state: &mut WorldState,
    data: &DataPack,
    events: &mut Vec<Event>,
) -> BTreeMap<EntityId, Company> {
    let physiology = data.physiology();
    let heard_for = physiology.calls.heard_for;
    let tick = state.tick;
    let alive: BTreeSet<EntityId> = state.sprites.iter().map(|(id, _)| id).collect();
    for (_, sprite) in state.sprites.iter_mut() {
        sprite
            .heard
            .retain(|heard| heard.fresh(tick, heard_for) && alive.contains(&heard.caller));
    }
    let sprites = &state.sprites;
    // Each sprite's tile, and its sex if it's grown, in ascending ID order,
    // found once: every sprite looks at every other (#210).
    let everyone: Vec<(EntityId, Pos, Option<Sex>)> = sprites
        .iter()
        .map(|(id, sprite)| {
            (
                id,
                sprite.pos,
                sprite.stage().is_grown().then_some(sprite.sex),
            )
        })
        .collect();
    let grown_sex = |id: EntityId| {
        let place = everyone.binary_search_by_key(&id, |&(id, ..)| id);
        place.ok().and_then(|place| everyone[place].2)
    };
    let company: BTreeMap<EntityId, Company> = sprites
        .iter()
        .map(|(id, sprite)| {
            // Those it senses: those it sees and those it has heard.
            let sees = sprite.seeing();
            let mut seen = Vec::new();
            let (mut female_near, mut male_near) = (false, false);
            let mut sense = |sex: Option<Sex>| {
                female_near |= sex == Some(Sex::Female);
                male_near |= sex == Some(Sex::Male);
            };
            for &(other, pos, sex) in &everyone {
                if other != id && sees(pos) {
                    seen.push(other);
                    sense(sex);
                }
            }
            for heard in &sprite.heard {
                sense(grown_sex(heard.caller));
            }
            let partner_near = sprite.partner().is_some_and(|partner| {
                seen.binary_search(&partner).is_ok()
                    || sprite.heard.iter().any(|heard| heard.caller == partner)
            });
            let company = Company {
                female_near,
                male_near,
                partner_near,
                seen,
                family: FamilySenses::default(),
            };
            (id, company)
        })
        .collect();
    for (id, sprite) in state.sprites.iter_mut() {
        let company = &company[&id];
        if company.male_near {
            sprite.last_male = tick;
        }
        if let Some(partner) = sprite.partner.as_mut().filter(|_| company.partner_near) {
            partner.seen = tick;
        }
    }
    part(state, data, events);
    company
}

/// Frees the partners who've gone `breeding.apart` ticks with neither
/// seeing nor hearing the other (M2 design §15.4): each once its own time
/// is that long ago, and its partner's, if its partner still has it.
fn part(state: &mut WorldState, data: &DataPack, events: &mut Vec<Event>) {
    let Some(apart) = data.physiology().breeding.apart else {
        return;
    };
    let tick = state.tick;
    let long_ago = |seen: u64| tick >= seen.saturating_add(apart);
    let parting: Vec<(EntityId, EntityId)> = state
        .sprites
        .iter()
        .filter_map(|(id, sprite)| {
            let partner = sprite.partner.filter(|p| !p.died && long_ago(p.seen))?;
            let theirs = state.sprites.get(partner.id).and_then(|them| them.partner);
            let held = theirs.filter(|p| p.id == id && !p.died);
            held.is_none_or(|p| long_ago(p.seen))
                .then_some((id, partner.id))
        })
        .collect();
    for &(id, other) in &parting {
        state.sprites.get_mut(id).expect("a parting sprite").partner = None;
        // Both freed together are told once, by the lower ID.
        if id < other || !parting.contains(&(other, id)) {
            events.push(Event {
                tick,
                kind: EventKind::Parted { a: id, b: other },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::Genome;
    use crate::test_util::{WorldBuilder, at, builtin_changing, id_at};
    use crate::world::World;

    /// A sprite that sees `sight` tiles and calls all the time, or with
    /// `calls` false, rests.
    fn genes(sight: f32, calls: bool, data: &DataPack) -> Genome {
        let verb = if calls { "Call" } else { "Rest" };
        let text = format!(
            r#"(format: 1, genes: [
                Trait(trait: "speed", value: 10.0),
                Trait(trait: "sense_radius", value: {sight:?}),
                Instinct(inputs: [("always", false)], verb: {verb}, weight: 1.0),
                BrainParam(param: "tau_base", value: 0.02),
            ])"#
        );
        Genome::from_ron(&text, data).expect("a valid genome")
    }

    /// Two sprites, as `a` and `b` make them, on a row of grass 30 tiles
    /// long, at its start and `gap` tiles on, in a pack where partners
    /// part after `breeding.apart` 50 ticks: the world and the two.
    fn two(
        a: impl Fn(&DataPack) -> Genome,
        b: impl Fn(&DataPack) -> Genome,
        gap: u16,
    ) -> (World, EntityId, EntityId) {
        let data = builtin_changing("physiology.ron", &[("apart: 4500,", "apart: 50,")]);
        let row = ".".repeat(30);
        let world = WorldBuilder::new(&[row.as_str()])
            .sprites_with(vec![
                (at(0, 0), Some(a(&data))),
                (at(gap, 0), Some(b(&data))),
            ])
            .data(data)
            .build();
        let (a, b) = (id_at(&world, at(0, 0)), id_at(&world, at(gap, 0)));
        (world, a, b)
    }

    /// The ticks, in its first 100, on which the partners of [`two`] parted,
    /// paired as an egg of theirs would pair them.
    fn partings(
        a: impl Fn(&DataPack) -> Genome,
        b: impl Fn(&DataPack) -> Genome,
        gap: u16,
    ) -> Vec<u64> {
        let (mut world, a, b) = two(a, b, gap);
        let (state, data) = world.parts();
        pair(state, data, a, b, &mut Vec::new());
        let mut parted = Vec::new();
        for _ in 0..100 {
            for event in world.step() {
                if let EventKind::Parted { .. } = event.kind {
                    parted.push(event.tick);
                }
            }
        }
        parted
    }

    #[test]
    fn a_heard_call_pulses_heard_with_the_caller_as_its_source() {
        // M2 design §15.1: 20 tiles off, out of sight; the pulse goes live
        // at the next step 3.
        let (mut world, caller, hearer) = two(
            |data| genes(10.0, true, data),
            |data| genes(10.0, false, data),
            20,
        );
        world.step();
        world.step();
        let (state, data) = world.parts();
        let heard = data.physiology().indices.courtship.expect("calls").heard;
        let body = &state.sprites.get(hearer).expect("the hearer").body;
        assert_eq!(body.loci[heard], 1.0, "heard");
        assert_eq!(body.sources.get(&heard), Some(&caller));
    }

    #[test]
    fn partners_who_only_hear_each_other_never_part() {
        // M2 design §15.4: 20 tiles apart, out of sight, a call heard
        // counts as the partner seen. Resting, they part after 50 ticks.
        let parted = |calls: bool| {
            let partner = |data: &DataPack| genes(10.0, calls, data);
            partings(partner, partner, 20)
        };
        assert_eq!(parted(true), [] as [u64; 0], "calling");
        assert_eq!(parted(false), [50], "resting");
    }

    #[test]
    fn partners_part_only_once_neither_has_sensed_the_other_for_long_enough() {
        // M2 design §15.4: "both times". 12 tiles apart, one sees 14 tiles
        // and the other 6, so only one of them sees the other: they stay
        // partners. Each seeing 6 tiles, they part.
        let parted = |sight: f32| {
            let far = |data: &DataPack| genes(sight, false, data);
            partings(far, |data| genes(6.0, false, data), 12)
        };
        assert_eq!(parted(14.0), [] as [u64; 0], "one sees the other");
        assert_eq!(parted(6.0), [50], "neither sees the other");
    }
}
