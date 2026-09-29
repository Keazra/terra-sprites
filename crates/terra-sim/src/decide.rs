//! Step 5a and 5b for one sprite (design §5.3, §5.5): attention picks what
//! to aim at, the brain picks a verb, and the step's activations are
//! snapshotted. All the brain's randomness is drawn here, and only at an
//! action boundary.

use std::collections::BTreeMap;

use crate::action::{Outcome, ScriptedAction, end, is_acting, start};
use crate::brain::{Aim, Snapshot, SpriteScoring, available, best_above};
use crate::data::DataPack;
use crate::events::Event;
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::{Ground, Target};
use crate::registry::{CategoryId, Verb};
use crate::world::WorldState;

/// Something the sprite could aim at, as it stands this tick.
#[derive(Debug, Clone)]
struct Candidate {
    target: Target,
    /// Its nearest reachable goal tile.
    goal: Pos,
    aim: Aim,
    /// The stable ID of its type.
    type_id: u16,
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
    let mut found = flood.candidates(ground, id, attacker);
    // Each sprite in reach is weighed on its own, and the one that draws the
    // eye most stands for sprites, unless the attacker does while a hit is
    // felt (design v18 §3.6).
    let is_attacker = |target: Target| attacker.is_some_and(|a| target == Target::Sprite(a));
    if !found
        .get(&data.sprite_category())
        .is_some_and(|&(target, _)| is_attacker(target))
    {
        let inputs = sprite.brain.inputs(&sprite.body, None, data);
        let drawn = flood.sprites(ground, id).into_iter().map(|(other, cost)| {
            let draw = sprite
                .brain
                .draw(other, normalized(cost, reach), &inputs, data);
            ((other, cost), draw)
        });
        // Ties go to the lower ID, which comes first.
        if let Some((other, cost)) = best_above(drawn, f32::NEG_INFINITY) {
            found.insert(data.sprite_category(), (Target::Sprite(other), cost));
        }
    }
    let candidates: BTreeMap<CategoryId, Candidate> = found
        .into_iter()
        .map(|(category, (target, cost))| {
            let goal = state
                .goal_for(data, flood, target)
                .expect("a candidate is reachable");
            let aim = Aim {
                category,
                distance: normalized(cost, reach),
                adjacent: state.on_goal_tile(data, sprite.pos, target),
            };
            let kind = state.kind_of(data, target).expect("a candidate is there");
            let object_type = &data.object_types()[kind];
            let type_id = object_type.id;
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
    let aimed = action.and_then(|a| a.target).map(|target| {
        let (category, adjacent) = (
            target_category(state, data, target),
            state.on_goal_tile(data, sprite.pos, target),
        );
        let cost = state
            .whereabouts(data, target)
            .and_then(|(there, own)| flood.nearest_goal(&state.map, there, own))
            .map_or(u32::MAX, |(_, cost)| cost);
        Aim {
            category,
            distance: normalized(cost, reach),
            adjacent,
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
    let mut distances: BTreeMap<CategoryId, f32> = candidates
        .iter()
        .map(|(&category, candidate)| (category, candidate.aim.distance))
        .collect();
    if let Some(aim) = aimed {
        distances.insert(aim.category, aim.distance);
    }

    let sprite = state.sprites.get_mut(id).expect("the same sprite");
    let rng = &mut state.rng;
    let brain = &mut sprite.brain;

    // 5a: attention, from the State inputs alone.
    let state_only = brain.inputs(&sprite.body, None, data);
    let curiosity_mod = sprite.body.loci[data.physiology().indices.curiosity_mod];
    let attention = brain.attention_scores(&state_only, &distances, curiosity_mod, scoring, data);
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
        aim.map(|a| a.category),
        decision_scoring,
        data,
    );
    let beside = candidate.is_some_and(|c| c.aim.adjacent);
    let offered = available(candidate.is_some(), beside);
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
        attended,
        target,
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
                Some(candidate.goal),
                Some((candidate.target, candidate.type_id)),
            )
        }
    };
    start(
        sprite,
        id,
        verb,
        destination,
        target,
        false,
        state.tick,
        events,
    );
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
    let target = target.and_then(|t| Some((t, state.type_of(data, t)?)));
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

/// The category `target` is perceived as.
pub(crate) fn target_category(state: &WorldState, data: &DataPack, target: Target) -> CategoryId {
    match target {
        Target::Object(id) => data.object_types()[state.objects.kind(id)].category,
        Target::Water(_) => data.water_category(),
        Target::Sprite(_) => data.sprite_category(),
    }
}

/// `cost` over the cost of walking the flood's `reach`, capped at 1.
fn normalized(cost: u32, reach: f32) -> f32 {
    (cost as f32 / reach).min(1.0)
}
