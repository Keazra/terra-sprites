//! Step 5a and 5b for one sprite (design §5.3, §5.5): attention picks what
//! to aim at, the brain picks a verb, and the step's activations are
//! snapshotted. All the brain's randomness is drawn here, and only at an
//! action boundary.

use std::collections::{BTreeMap, BTreeSet};

use crate::action::{Outcome, ScriptedAction, end, is_acting, penalty, set_off, start, trip};
use crate::brain::{Aim, Seen, Snapshot, SpriteScoring, available, best_above};
use crate::data::DataPack;
use crate::events::Event;
use crate::learning::Subject;
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::{Ground, Occupied, Target};
use crate::registry::{CategoryId, Verb};
use crate::world::WorldState;

/// Something a category offers the sprite (design §3.6): a thing in sight
/// and the cost of the way there, or a remembered place out of sight, with
/// how well it's remembered (M2 design §7).
#[derive(Debug, Clone, Copy)]
struct Offer {
    target: Target,
    cost: Option<u32>,
    recall: f32,
}

/// Something the sprite could aim at, as it stands this tick.
#[derive(Debug, Clone)]
struct Candidate {
    target: Target,
    /// Its nearest reachable goal tile; none for a remembered place out of
    /// sight (M2 design §7).
    goal: Option<Pos>,
    aim: Aim,
    /// The stable ID of its type: none for water or a sprite in a pack with
    /// no object type for them (design v19 §3.5.5).
    type_id: Option<u16>,
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
            });
            (category, things.collect())
        })
        .collect();
    // A remembered place joins its category's offers, as a thing at the
    // edge of sight, while nothing of its kind is in sight (M2 design §7).
    let in_view: BTreeSet<Subject> = offered
        .values()
        .flatten()
        .map(|offer| state.subject_of(data, offer.target))
        .collect();
    for place in &sprite.brain.experience.places {
        if in_view.contains(&place.subject) || state.whereabouts(data, place.target).is_none() {
            continue;
        }
        let offer = Offer {
            target: place.target,
            cost: None,
            recall: place.strength,
        };
        let category = place.subject.category(data);
        offered.entry(category).or_default().push(offer);
    }
    // Each category's candidate is the thing that draws the eye most (design
    // v19 §3.6); ties go to the lower ID, which comes first, and then to
    // what's in sight.
    let state_only = sprite.brain.inputs(&sprite.body, None, data);
    let curiosity_mod = sprite.body.loci[data.physiology().indices.curiosity_mod];
    let found: BTreeMap<CategoryId, Offer> = offered
        .into_iter()
        .map(|(category, things)| {
            let drawn = things.into_iter().map(|offer| {
                let seen = Seen {
                    subject: state.subject_of(data, offer.target),
                    distance: offer.cost.map_or(1.0, |cost| normalized(cost, reach)),
                    recall: offer.recall,
                };
                let draw = sprite.brain.draw(
                    seen,
                    offer.target.sprite(),
                    &state_only,
                    curiosity_mod,
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
            };
            let candidate = Candidate {
                type_id,
                target,
                goal,
                aim,
            };
            (category, candidate)
        })
        .collect();
    // What a running aimed action is aimed at, which may not be its
    // category's candidate now (design §5.3).
    let aimed = action.and_then(|a| a.target.map(|target| (a, target))).map(|(a, target)| {
        let (category, adjacent) = (
            state.category_of(data, target),
            state.on_goal_tile(data, sprite.pos, target),
        );
        let cost = state
            .whereabouts(data, target)
            .and_then(|(there, own)| flood.nearest_goal(&state.map, there, own))
            .map_or(u32::MAX, |(_, cost)| cost);
        // A trip still out of sight is weighed as its place is remembered
        // (M2 design §7).
        let recall = if a.remembered {
            sprite.brain.experience.recall(target)
        } else {
            1.0
        };
        Aim {
            category,
            subject: state.subject_of(data, target),
            distance: normalized(cost, reach),
            adjacent,
            recall,
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
    };
    let mut in_sight: BTreeMap<CategoryId, Seen> = candidates
        .iter()
        .map(|(&category, c)| (category, seen(&c.aim)))
        .collect();
    if let Some(aim) = aimed {
        in_sight.insert(aim.category, seen(&aim));
    }

    let sprite = state.sprites.get_mut(id).expect("the same sprite");
    let rng = &mut state.rng;
    let brain = &mut sprite.brain;

    // 5a: attention, from the State inputs alone.
    let attention = brain.attention_scores(&state_only, &in_sight, curiosity_mod, scoring, data);
    let scored = in_sight
        .iter()
        .map(|(&c, seen)| (c, seen.subject))
        .collect();
    let attended = brain.attend(&attention, running, exploration, rng);
    let mut running = running;
    if let Some(aim) = aimed
        && attended != Some(aim.category)
    {
        // Attention moved off the action's target: it changed its mind.
        let action = sprite.action.as_mut().expect("the running action");
        end(action, id, Outcome::Interrupted, state.tick, events);
        running = false;
    }
    let candidate = attended.and_then(|c| candidates.get(&c));
    let aim = if running { aimed } else { None }.or(candidate.map(|c| c.aim));

    // 5b: the decision.
    let inputs = brain.inputs(&sprite.body, aim, data);
    let activations = brain.activations(&inputs);
    // The sprite the decision is about, as `aim` is: a running action's
    // target, or else the candidate (design v18 §5.5).
    let aimed_target = sprite
        .action
        .as_ref()
        .filter(|_| running)
        .and_then(|a| a.target);
    let aimed_sprite = match aimed_target {
        Some(target) => target.sprite(),
        None => candidate_sprite,
    };
    let decision_scoring = SpriteScoring {
        sprite: aimed_sprite,
        quiet,
    };
    let scores = brain.scores(
        &activations,
        &inputs,
        aim.map(|a| a.subject),
        decision_scoring,
        data,
    );
    let beside = candidate.is_some_and(|c| c.aim.adjacent);
    let remembered = candidate.is_some_and(|c| c.goal.is_none());
    let offered = available(candidate.is_some(), beside, remembered);
    let current = sprite.action.as_ref().filter(|_| running).map(|a| a.verb);
    let chosen = match current {
        Some(verb) => brain.switch(verb, &scores, &offered),
        None => Some(brain.choose(&scores, &offered, exploration, rng)),
    };
    // A new action is aimed at the candidate; a running one keeps its target.
    let running_target = sprite
        .action
        .as_ref()
        .filter(|_| running && chosen.is_none());
    let target = running_target
        .and_then(|a| a.target)
        .or(candidate.map(|c| c.target));
    let verb = chosen.or(current);
    let motive = verb.and_then(|verb| brain.motive(verb, &activations, data));
    brain.snapshot = Some(Snapshot {
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
    });
    let Some(verb) = chosen else {
        return;
    };
    if running {
        let action = sprite.action.as_mut().expect("the running action");
        end(action, id, Outcome::Interrupted, state.tick, events);
    }
    let (destination, target) = match verb {
        Verb::Wander => {
            let flood = sprite.flood.as_ref().expect("step 5 made the flood");
            let sense_radius = sprite.program.traits.sense_radius;
            (flood.wander_destination(sense_radius, rng), None)
        }
        Verb::Rest => (None, None),
        Verb::Retreat => {
            let candidate = candidate.expect("an aimed verb is offered only with a target");
            (None, Some((candidate.target, candidate.type_id)))
        }
        _ => {
            let candidate = candidate.expect("an aimed verb is offered only with a target");
            (
                candidate.goal,
                Some((candidate.target, candidate.type_id)),
            )
        }
    };
    // Setting off for a remembered place, it finds the way there over the
    // whole map, which it keeps to; with none, it forgets the place (M2
    // design §7).
    let way = match (target, destination) {
        (Some((target, _)), None) if verb.heads_for_goal() => {
            let sprite = state.sprites.get(id).expect("the same sprite");
            let way = trip(state, data, sprite, target, Occupied::Penalty(penalty(data)));
            if way.is_none() {
                let sprite = state.sprites.get_mut(id).expect("the same sprite");
                sprite.brain.experience.forget_place(target);
            }
            way
        }
        _ => None,
    };
    let sprite = state.sprites.get_mut(id).expect("the same sprite");
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
        let action = sprite.action.as_mut().expect("the action just started");
        set_off(action, way, sprite.program.traits.speed, state.tick);
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
