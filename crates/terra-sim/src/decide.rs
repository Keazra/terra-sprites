//! Step 5a and 5b for one sprite (design §5.3, §5.5): attention picks what
//! to aim at, the brain picks a verb, and the step's activations are
//! snapshotted. All the brain's randomness is drawn here, and only at an
//! action boundary.

use std::collections::{BTreeMap, BTreeSet};

use crate::action::{Outcome, ScriptedAction, end, is_acting, penalty, set_off, start, trip};
use crate::brain::{
    self, Aim, Looks, Seen, Situation, Snapshot, SpriteScoring, available, best_above,
};
use crate::breeding;
use crate::courtship;
use crate::data::DataPack;
use crate::events::Event;
use crate::family;
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::{Ground, Occupied, Target};
use crate::registry::{CategoryId, Verb};
use crate::world::WorldState;

/// Something a category offers the sprite (design §3.6): a thing in sight
/// and the cost of the way there, or a remembered place out of sight, with
/// how well it's remembered (M2 design §7), or a sprite heard calling, out
/// of sight, with the tile it called from (§15.1), or an egg of its own out
/// of sight, with where it last saw it (§15.6).
#[derive(Debug, Clone, Copy)]
struct Offer {
    target: Target,
    cost: Option<u32>,
    recall: f32,
    seek: Option<Pos>,
}

/// Something the sprite could aim at, as it stands this tick.
#[derive(Debug, Clone)]
struct Candidate {
    target: Target,
    /// Its nearest reachable goal tile; none for a remembered place out of
    /// sight (M2 design §7), or a running action's target out of reach
    /// (#173).
    goal: Option<Pos>,
    /// Whether it's a remembered place out of sight (M2 design §7), a
    /// sprite heard calling (§15.1), or an egg of its own out of sight
    /// (§15.6).
    remembered: bool,
    /// The tile it's looked for on: where a sprite heard calling called
    /// from (M2 design §15.1), or where it last saw its egg (§15.6).
    seek: Option<Pos>,
    aim: Aim,
    /// The stable ID of its type: none for water or a sprite in a pack with
    /// no object type for them (design v19 §3.5.5).
    type_id: Option<u16>,
}

/// What `pick` picks out of every offer in `offered`, once each: what's
/// already there to aim at, so what's remembered or heard of it isn't
/// offered again.
fn offered_by<T: Ord>(
    offered: &BTreeMap<CategoryId, Vec<Offer>>,
    pick: impl FnMut(&Offer) -> Option<T>,
) -> BTreeSet<T> {
    offered.values().flatten().filter_map(pick).collect()
}

/// Step 5a and 5b for sprite `id`, after 5.0 (design §5.5). A scripted
/// action runs undisturbed: the brain neither attends nor decides while it
/// does, and the next scripted action starts before the brain chooses.
pub(crate) fn decide(
    state: &mut WorldState,
    data: &DataPack,
    id: EntityId,
    events: &mut Vec<Event>,
) {
    let sprite = state.sprites.get(id).expect("a sprite taking its turn");
    let running = is_acting(sprite);
    let action = sprite.action.as_ref().filter(|_| running);
    if action.is_some_and(|a| a.scripted) {
        return;
    }
    if !running && !sprite.scripted.is_empty() {
        start_scripted(state, data, id, events);
        return;
    }

    // What there is to aim at: each category's candidate (design §3.6).
    let flood = sprite.flood.as_ref().expect("step 5 made the flood");
    let ground = Ground {
        map: &state.map,
        objects: &state.objects,
        sprites: &state.sprites,
        data,
    };
    // What walking the flood's reach on grass costs: a grass step is 10
    // terrain units (design §5.2).
    let reach = 10.0 * f32::from(flood.reach());
    let was_hit = data.physiology().indices.was_hit;
    let attacker = sprite.body.sources.get(&was_hit).copied();
    // Each sprite in reach is weighed on its own, unless the attacker stands
    // for sprites while a hit is felt (design v18 §3.6).
    let mut offered: BTreeMap<CategoryId, Vec<Offer>> = flood
        .candidates(ground, id, attacker, state.cursor.seen_at())
        .into_iter()
        .map(|(category, things)| {
            let things = things.into_iter().map(|(target, cost)| Offer {
                target,
                cost: Some(cost),
                recall: 1.0,
                seek: None,
            });
            (category, things.collect())
        })
        .collect();
    // A remembered place joins its category's offers, as a thing at the
    // edge of sight, while nothing of its kind is in sight (M2 design §7).
    let in_view = offered_by(&offered, |offer| Some(state.subject_of(data, offer.target)));
    for place in &sprite.brain.experience.places {
        if in_view.contains(&place.subject) || state.whereabouts(data, place.target).is_none() {
            continue;
        }
        let offer = Offer {
            target: place.target,
            cost: None,
            recall: place.recall,
            seek: None,
        };
        let category = place.subject.category(data);
        offered.entry(category).or_default().push(offer);
    }
    // So does a sprite heard calling, out of sight, which draws the eye less
    // as its call fades (M2 design §15.1).
    let heard_for = data.physiology().calls.heard_for;
    let sprites_offered = offered_by(&offered, |offer| offer.target.sprite());
    for heard in &sprite.heard {
        let caller = heard.caller;
        if sprites_offered.contains(&caller) || state.sprites.get(caller).is_none() {
            continue;
        }
        let offer = Offer {
            target: Target::Sprite(caller),
            cost: None,
            recall: heard.freshness(state.tick, heard_for),
            seek: Some(heard.from),
        };
        offered
            .entry(data.sprite_category())
            .or_default()
            .push(offer);
    }
    // So does an egg of its own out of sight, where it last saw it, as a
    // remembered place is (M2 design §15.6).
    let objects_offered = match sprite.eggs.is_empty() {
        true => BTreeSet::new(),
        false => offered_by(&offered, |offer| match offer.target {
            Target::Object(id) => Some(id),
            _ => None,
        }),
    };
    for known in &sprite.eggs {
        let egg = Target::Object(known.egg);
        if objects_offered.contains(&known.egg) || state.whereabouts(data, egg).is_none() {
            continue;
        }
        let offer = Offer {
            target: egg,
            cost: None,
            recall: 1.0,
            seek: Some(known.at),
        };
        let category = state.category_of(data, egg);
        offered.entry(category).or_default().push(offer);
    }
    // The breeding drive works only between adults (M2 design §5): it reads
    // 0 in a sprite not grown up, and in a choice about one, or about an
    // egg, which holds a sprite not yet hatched (§10.4). Breeding alone is
    // offered only to a grown sprite too (§10).
    let grown = sprite.stage().is_grown();
    let not_grown = |target: Target| match target {
        Target::Sprite(other) => state
            .sprites
            .get(other)
            .is_some_and(|other| !other.stage().is_grown()),
        Target::Object(id) => state
            .objects
            .get(id)
            .is_some_and(|object| object.unhatched.is_some()),
        Target::Water(_) | Target::Cursor => false,
    };
    // Each category's candidate is the thing that draws the eye most (design
    // v19 §3.6); ties go to the lower ID, which comes first, and then to
    // what's in sight.
    let mut state_only = sprite.brain.inputs(&sprite.body, None, None, data);
    if !grown {
        brain::without_drive(&mut state_only, data);
    }
    let curiosity_mod = sprite.body.loci[data.physiology().indices.curiosity_mod];
    // Sprites in general, worked out once for everything the sprite weighs
    // this tick (design v18 §5.6).
    let general = sprite.brain.sprites_in_general(data);
    let found: BTreeMap<CategoryId, Offer> = offered
        .into_iter()
        .map(|(category, things)| {
            let drawn = things.into_iter().map(|offer| {
                let seen = Seen {
                    subject: state.subject_of(data, offer.target),
                    distance: offer.cost.map_or(1.0, |cost| normalized(cost, reach)),
                    recall: offer.recall,
                    not_grown: not_grown(offer.target),
                    relation: family::relation(state, data, id, offer.target),
                };
                let draw = sprite.brain.draw(
                    seen,
                    offer.target.sprite(),
                    &state_only,
                    curiosity_mod,
                    &general,
                    data,
                );
                (offer, draw)
            });
            let best = best_above(drawn, f32::NEG_INFINITY);
            (category, best.expect("a category offers something"))
        })
        .collect();
    let candidates: BTreeMap<CategoryId, Candidate> = found
        .into_iter()
        .map(|(category, offer)| {
            let target = offer.target;
            // A remembered place's way is found only if the sprite sets off
            // for it (M2 design §7).
            let goal = offer.cost.map(|_| {
                state
                    .goal_for(data, flood, target)
                    .expect("a candidate in sight is reachable")
            });
            let type_id = state.type_of(data, target);
            let aim = Aim {
                category,
                subject: state.subject_of(data, target),
                distance: offer.cost.map_or(1.0, |cost| normalized(cost, reach)),
                adjacent: state.on_goal_tile(data, sprite.pos, target),
                recall: offer.recall,
                not_grown: not_grown(target),
                relation: family::relation(state, data, id, target),
            };
            let candidate = Candidate {
                type_id,
                target,
                goal,
                remembered: offer.cost.is_none(),
                seek: offer.seek,
                aim,
            };
            (category, candidate)
        })
        .collect();
    // What a running aimed action is aimed at, which may not be its
    // category's candidate now (design §5.3).
    let aimed = action
        .and_then(|a| a.target.map(|target| (a, target)))
        .map(|(a, target)| {
            let (category, adjacent) = (
                state.category_of(data, target),
                state.on_goal_tile(data, sprite.pos, target),
            );
            // A caller's own tile is within the flood only once it's in
            // sight, so a trip to a call finds no goal here until then, and
            // keeps to the way it set off on (M2 design §15.1).
            let whereabouts = state.whereabouts(data, target);
            let nearest =
                whereabouts.and_then(|(there, own)| flood.nearest_goal(&state.map, there, own));
            let cost = nearest.map_or(u32::MAX, |(_, cost)| cost);
            let subject = state.subject_of(data, target);
            // A trip still out of sight is weighed as its place is
            // remembered, or one of its kind near it that it merged into (M2
            // design §7.3), or as the call it goes to (§15.1), or as an egg
            // of its own is (§15.6).
            let recall = match whereabouts {
                _ if !a.remembered => 1.0,
                Some(_) if a.seek.is_some() => target.sprite().map_or(1.0, |caller| {
                    sprite
                        .heard_from(caller, state.tick, heard_for)
                        .map_or(0.0, |heard| heard.freshness(state.tick, heard_for))
                }),
                Some((there, _)) => sprite.brain.experience.recall(
                    subject,
                    there,
                    data.physiology().remembered_places.merge,
                ),
                None => 0.0,
            };
            let aim = Aim {
                category,
                subject,
                distance: normalized(cost, reach),
                adjacent,
                recall,
                not_grown: not_grown(target),
                relation: family::relation(state, data, id, target),
            };
            Candidate {
                target,
                goal: nearest.map(|(goal, _)| goal),
                remembered: a.remembered,
                seek: a.seek,
                aim,
                type_id: a.target_type,
            }
        });
    let exploration = sprite.body.loci[data.physiology().indices.exploration_mod];
    // The sprite that stands for sprites while attention scores: a running
    // action's target, or else the candidate (design v18 §5.3).
    let candidate_sprite = candidates
        .get(&data.sprite_category())
        .and_then(|c| c.target.sprite());
    // Fear always catches the eye, so a sprite hit or cornered keeps it on
    // whoever did it (design v18 §5.3).
    let scoring = SpriteScoring {
        sprite: action
            .and_then(|a| a.target)
            .filter(|&target| state.category_of(data, target) == data.sprite_category())
            .and_then(Target::sprite)
            .or(candidate_sprite),
        quiet: false,
    };
    // But while a hit is felt or the sprite is cornered, what it does is
    // instinct's, and fear is quiet in the decision (design v18 §5.5).
    let quiet = sprite.body.loci[was_hit] > 0.0
        || sprite.body.loci[data.physiology().indices.cornered] > 0.0;
    // A running action's category is scored by the instance it's aimed at,
    // which a nearer one of the same category doesn't replace (design §5.3).
    let seen = |aim: &Aim| Seen {
        subject: aim.subject,
        distance: aim.distance,
        recall: aim.recall,
        not_grown: aim.not_grown,
        relation: aim.relation,
    };
    let mut in_sight: BTreeMap<CategoryId, Seen> = candidates
        .iter()
        .map(|(&category, c)| (category, seen(&c.aim)))
        .collect();
    if let Some(aimed) = &aimed {
        in_sight.insert(aimed.aim.category, seen(&aimed.aim));
    }

    // Mate is offered only when the sprite and the one it would aim at, a
    // running action's target or the candidate, are both adults (M2 design
    // §5).
    let partners: BTreeSet<EntityId> = candidate_sprite
        .into_iter()
        .chain(aimed.as_ref().and_then(|a| a.target.sprite()))
        .filter(|&other| breeding::adults(state, id, other))
        .collect();

    // What each sprite it may attend to shows it, and what it knows of
    // that sprite (M2 design §10, §15). A heard caller shows as if in sight.
    let known = &sprite.brain.experience.individuals;
    let looks: BTreeMap<EntityId, Looks> = candidates
        .values()
        .map(|c| c.target)
        .chain(action.and_then(|a| a.target))
        .filter_map(Target::sprite)
        .filter_map(|other| {
            let other_sprite = state.sprites.get(other)?;
            let looks = Looks {
                body: brain::shown(&other_sprite.body, data),
                familiar: known.get(&other).map_or(0.0, |known| known.familiarity),
                partner: sprite.partner() == Some(other),
                kin: courtship::kin(state, id, other),
            };
            Some((other, looks))
        })
        .collect();

    let sprite = state.sprites.get_mut(id).expect("the same sprite");
    let rng = &mut state.rng;
    let brain = &mut sprite.brain;

    // 5a: attention, from the State inputs alone.
    let attention = brain.attention_scores_with(
        &state_only,
        &in_sight,
        curiosity_mod,
        scoring,
        &general,
        data,
    );
    let scored = in_sight
        .iter()
        .map(|(&c, seen)| (c, seen.subject))
        .collect();
    let attended = brain.attend(&attention, running, exploration, rng);
    let candidate = attended.and_then(|c| candidates.get(&c));
    // Attention moved off the action's target: it changed its mind. The
    // action ends once the new one is sure to start, as a trip with no way
    // there doesn't (M2 design §7.4).
    let moved = aimed
        .as_ref()
        .is_some_and(|aimed| attended != Some(aimed.aim.category));
    let running = running && !moved;
    // What the decision weighs, and what a verb it chooses is aimed at: a
    // running action's target, which a nearer one of its category doesn't
    // replace, or else the candidate (design §5.3, #173).
    let weighed = aimed.as_ref().filter(|_| running).or(candidate);
    let aim = weighed.map(|c| c.aim);

    // 5b: the decision, with what the sprite it attends to shows (M2
    // design §10).
    let aimed_target = sprite
        .action
        .as_ref()
        .filter(|_| running)
        .and_then(|a| a.target);
    let seen = aimed_target
        .or(candidate.map(|c| c.target))
        .and_then(Target::sprite)
        .and_then(|other| looks.get(&other));
    let mut inputs = brain.inputs(&sprite.body, aim, seen, data);
    if !grown || aim.is_some_and(|aim| aim.not_grown) {
        brain::without_drive(&mut inputs, data);
    }
    let activations = brain.activations(&inputs);
    // The sprite the decision is about, as `aim` is: a running action's
    // target, or else the candidate (design v18 §5.5).
    let aimed_sprite = match aimed_target {
        Some(target) => target.sprite(),
        None => candidate_sprite,
    };
    let decision_scoring = SpriteScoring {
        sprite: aimed_sprite,
        quiet,
    };
    let scores = brain.scores_with(
        &activations,
        &inputs,
        aim.map(|a| a.subject),
        decision_scoring,
        &general,
        data,
    );
    // Verbs are offered on it only while there's a way to it: a running
    // action's target out of reach, unless it's a remembered place, offers
    // only what needs no target (design §5.5, #173).
    let reachable = weighed.filter(|c| c.goal.is_some() || c.remembered);
    let beside = reachable.is_some_and(|c| c.aim.adjacent);
    let remembered = reachable.is_some_and(|c| c.remembered);
    let adults = reachable
        .and_then(|c| c.target.sprite())
        .is_some_and(|other| partners.contains(&other));
    let offered = available(Situation {
        target: reachable.is_some(),
        beside,
        remembered,
        adults,
        grown,
        courts: data.physiology().indices.courtship.is_some(),
    });
    let current = sprite.action.as_ref().filter(|_| running).map(|a| a.verb);
    let chosen = match current {
        Some(verb) => brain.switch(verb, &scores, &offered),
        None => Some(brain.choose(&scores, &offered, exploration, rng)),
    };
    let target = weighed.map(|c| c.target);
    let verb = chosen.or(current);
    let motive = verb.and_then(|verb| brain.motive(verb, &activations, data));
    let snapshot = Snapshot {
        tick: state.tick,
        inputs,
        activations,
        attention,
        scored,
        attended,
        target,
        subject: aim.map(|a| a.subject),
        scores,
        verb,
        motive,
        sprite_seen: scoring.sprite,
        scoring: decision_scoring,
    };
    let Some(verb) = chosen else {
        brain.snapshot = Some(snapshot);
        return;
    };
    let (destination, target) = match verb {
        Verb::Wander => {
            let flood = sprite.flood.as_ref().expect("step 5 made the flood");
            let sense_radius = sprite.program.traits.sense_radius;
            (flood.wander_destination(sense_radius, rng), None)
        }
        Verb::Rest | Verb::BreedAlone | Verb::Call => (None, None),
        Verb::Retreat => {
            let weighed = weighed.expect("an aimed verb is offered only with a target");
            (None, Some((weighed.target, weighed.type_id)))
        }
        _ => {
            let weighed = weighed.expect("an aimed verb is offered only with a target");
            (weighed.goal, Some((weighed.target, weighed.type_id)))
        }
    };
    // Setting off for a remembered place, a call or its egg, it finds the
    // way there, which it keeps to; with none, it forgets the place, the
    // call or where it saw the egg, and keeps to what it was doing, deciding
    // nothing this tick, and chooses again at its next step 5 (M2 design
    // §7.4, §15.1, §15.6).
    let seek = weighed.and_then(|c| c.seek);
    let way = match (target, destination) {
        (Some((target, _)), None) if verb.heads_for_goal() => {
            let sprite = state.sprites.get(id).expect("the same sprite");
            let way = trip(
                state,
                data,
                sprite,
                target,
                seek,
                Occupied::Penalty(penalty(data)),
            );
            if way.is_none() {
                let sprite = state.sprites.get_mut(id).expect("the same sprite");
                match seek {
                    Some(_) => sprite.forget_sought(target),
                    None => sprite.brain.experience.forget_place(target),
                }
                return;
            }
            way
        }
        _ => None,
    };
    let sprite = state.sprites.get_mut(id).expect("the same sprite");
    sprite.brain.snapshot = Some(snapshot);
    if running || moved {
        let action = sprite.action.as_mut().expect("the running action");
        end(action, id, Outcome::Interrupted, state.tick, events);
    }
    start(
        sprite,
        id,
        verb,
        way.as_ref().map_or(destination, |way| Some(way.goal)),
        target,
        false,
        state.tick,
        events,
    );
    if let Some(way) = way {
        let speed = sprite.speed(state.tick, data);
        let action = sprite.action.as_mut().expect("the action just started");
        set_off(action, way, speed, state.tick);
    }
}

/// Starts sprite `id` on its next scripted action.
fn start_scripted(state: &mut WorldState, data: &DataPack, id: EntityId, events: &mut Vec<Event>) {
    let sprite = state.sprites.get_mut(id).expect("a sprite taking its turn");
    let script = sprite.scripted.pop_front().expect("a scripted action");
    let (verb, destination, target) = match script {
        ScriptedAction::Wander { destination } => (Verb::Wander, Some(destination), None),
        ScriptedAction::Rest => (Verb::Rest, None, None),
        ScriptedAction::Eat { at } => (Verb::Eat, None, state.object_target(at)),
        ScriptedAction::Drink { at } => (Verb::Drink, None, state.water_target(data, at)),
        ScriptedAction::Approach { at } => {
            let target = state
                .sprite_target(at)
                .or_else(|| state.object_target(at))
                .or_else(|| state.water_target(data, at));
            (Verb::Approach, None, target)
        }
        ScriptedAction::Play { at } => (Verb::Play, None, state.contact_target(id, at)),
        ScriptedAction::Hit { at } => (Verb::Hit, None, state.contact_target(id, at)),
        ScriptedAction::Retreat { at } => {
            let target = state
                .sprite_target(at)
                .filter(|&t| t != Target::Sprite(id))
                .or_else(|| state.object_target(at))
                .or_else(|| state.water_target(data, at));
            (Verb::Retreat, None, target)
        }
    };
    let flood = state
        .sprites
        .get(id)
        .expect("the same sprite")
        .flood
        .as_ref();
    let flood = flood.expect("step 5 made the flood");
    // A retreat heads for no goal tile: it backs away (design §5.5).
    let destination = match target {
        Some(target) if verb.heads_for_goal() => state.goal_for(data, flood, target),
        Some(_) => None,
        None => destination,
    };
    let target = target.map(|t| (t, state.type_of(data, t)));
    let sprite = state.sprites.get_mut(id).expect("the same sprite");
    start(
        sprite,
        id,
        verb,
        destination,
        target,
        true,
        state.tick,
        events,
    );
}

/// `cost` over the cost of walking the flood's `reach`, capped at 1.
fn normalized(cost: u32, reach: f32) -> f32 {
    (cost as f32 / reach).min(1.0)
}
