//! Actions in words (design §6.1): what a sprite is doing, as the Body
//! tab's first line says it, plainly or, in the detail view, exactly; and
//! what a finished action did, for the observed list and the event log.

use terra_sim::{ActionView, EntityId, Outcome, Progress, Target, Verb};

use super::{CURSOR, counted, with_article};
use crate::text::{Words, display_name};

/// What a sprite is doing, as the Body tab's first line says it (design
/// §6.1): in plain words, describing and never speaking as the sprite; or,
/// in the detail view, exactly, with its verb, destination or target, and
/// outcome.
pub(super) fn action_line(action: &ActionView, detail: bool, data: &Words) -> String {
    let plain = match action.verb {
        _ if detail => None,
        Verb::Wander => wander_line(action.progress),
        Verb::Rest => rest_line(action.progress),
        Verb::BreedAlone => breed_alone_line(action.progress),
        Verb::Call => call_line(action.progress),
        _ => aimed_line(action, data),
    };
    plain.unwrap_or_else(|| exact_line(action, data))
}

/// An aimed action in plain words, naming what it's aimed at, if it has
/// words for `progress`.
fn aimed_line(action: &ActionView, data: &Words) -> Option<String> {
    action.target?;
    let what = target_words(action, data);
    // Out of sight, it goes by what it remembers (M2 design §7).
    let aim = match action.remembered {
        true => format!("{what} it remembers"),
        false => what.clone(),
    };
    // Out of sight, it goes to where the call came from, or where it last
    // saw its egg (M2 design §15.8).
    let ended = matches!(action.progress, Progress::Ended(_));
    let going = match action.verb {
        _ if action.heard.is_some() && !ended => format!("Going to {what}'s call"),
        _ if action.egg_at.is_some() && !ended => "Going back to its egg".into(),
        Verb::Eat => format!("Going to eat {aim}"),
        Verb::Drink if action.remembered => format!("Going to drink at {aim}"),
        Verb::Drink => "Going to drink".into(),
        Verb::Approach => format!("Going over to {aim}"),
        Verb::Play => format!("Going to play with {aim}"),
        Verb::Hit => format!("Going to hit {aim}"),
        // Mating is pairing, in the player's words (M2 design §15.8).
        Verb::Mate => format!("Going to pair with {aim}"),
        Verb::Court => format!("Courting {aim}"),
        Verb::Retreat => format!("Backing away from {what}"),
        _ => return None,
    };
    Some(match action.progress {
        Progress::Walking { steps_left } => {
            format!(
                "{going} · {} to go",
                counted(steps_left, walked(action.verb))
            )
        }
        Progress::Waiting { .. } if action.verb == Verb::Retreat => {
            format!("{going} · waiting for room")
        }
        Progress::Waiting { .. } => format!("{going} · waiting to get past"),
        Progress::Resting { .. } => return None,
        Progress::Ended(Outcome::Applied) => done_line(action, &what, data),
        Progress::Ended(Outcome::Failed) if action.attempted => {
            let couldnt = match action.verb {
                Verb::Eat => format!("Couldn't eat from {what}"),
                Verb::Play => format!("Couldn't play with {what}"),
                Verb::Hit => format!("Couldn't hit {what}"),
                Verb::Mate => format!("Couldn't pair with {what}"),
                Verb::Court => format!("Couldn't dance for {what}"),
                _ => "Couldn't drink".into(),
            };
            and_if_hurt(couldnt, action)
        }
        Progress::Ended(Outcome::Failed) if action.target_gone => {
            format!("Gave up: {what} was gone")
        }
        Progress::Ended(Outcome::Blocked) if action.verb == Verb::Retreat => {
            "Backed into a corner".into()
        }
        // Its gut had no room (M2 design §19.4). A pack may feed a sprite
        // through any verb, so one that isn't Eat or Drink is just too full.
        Progress::Ended(Outcome::TooFull) => match action.verb {
            Verb::Eat if eaten_from(action, data) => format!("Too full to eat from {what}"),
            Verb::Eat => format!("Too full to eat {what}"),
            Verb::Drink => "Too full to drink".into(),
            _ => return ended_line(Outcome::TooFull),
        },
        Progress::Ended(outcome) => return ended_line(outcome),
    })
}

/// What a walking action counts down: a retreat, the steps of its bout;
/// any other, the tiles of its path.
fn walked(verb: Verb) -> &'static str {
    if verb == Verb::Retreat {
        "step"
    } else {
        "tile"
    }
}

/// A finished action in the past tense, for the Body tab's observed list
/// (design §6.1): what it did, or what it set out to do and how that went.
pub(crate) fn observed_line(action: &ActionView, data: &Words) -> String {
    let Progress::Ended(outcome) = action.progress else {
        return action_line(action, false, data);
    };
    let what = target_words(action, data);
    let set_out = match action.verb {
        Verb::Wander => "Wandered off".to_string(),
        Verb::Rest => "Rested".into(),
        Verb::Eat => format!("Went to eat {what}"),
        Verb::Drink => "Went to drink".into(),
        Verb::Approach => format!("Went over to {what}"),
        Verb::Play => format!("Went to play with {what}"),
        Verb::Hit => format!("Went to hit {what}"),
        Verb::Retreat => format!("Backed away from {what}"),
        Verb::Mate => format!("Went to pair with {what}"),
        Verb::Court => format!("Went to court {what}"),
        Verb::Call => "Called".into(),
        verb => verb_name(verb).to_lowercase(),
    };
    let how = match outcome {
        Outcome::Applied => {
            return match action.verb {
                Verb::Wander | Verb::Rest | Verb::Approach | Verb::Retreat | Verb::Call => set_out,
                _ => done_line(action, &what, data),
            };
        }
        Outcome::Blocked if action.verb == Verb::Retreat => "was cornered".into(),
        Outcome::Failed if action.attempted => match action.verb {
            Verb::Eat | Verb::Drink => "it was empty".to_string(),
            _ => "couldn't".into(),
        },
        Outcome::Failed if action.target_gone => match action.target {
            Some(Target::Sprite(_)) => format!("{what} was gone"),
            _ => "it was gone".into(),
        },
        Outcome::Interrupted => "changed its mind".into(),
        Outcome::PulledAway => "was pulled away".into(),
        Outcome::TooFull => "was too full".into(),
        outcome => {
            let reason = ended_line(outcome).expect("an outcome that isn't applied");
            reason.replacen("Gave up", "gave up", 1)
        }
    };
    and_if_hurt(format!("{set_out}, but {how}"), action)
}

/// What sprite `actor`'s finished `action` did to the sprite it was aimed
/// at, as that sprite's observed list says it (design §6.1): "Was hit by
/// Sprite #7". `None` if it did nothing worth a line.
pub(crate) fn done_to_line(actor: EntityId, action: &ActionView, data: &Words) -> Option<String> {
    if action.progress != Progress::Ended(Outcome::Applied) {
        return None;
    }
    let who = data.label(actor);
    match action.verb {
        Verb::Hit => Some(format!("Was hit by {who}")),
        Verb::Play => Some(format!("{who} played with it")),
        Verb::Court => Some(format!("{who} danced for it")),
        _ => None,
    }
}

/// What an aimed action that applied did to `what`, its target in words,
/// in the past tense: "Ate from the berry bush", "Kicked the ball", and
/// whether it got hurt doing it.
fn done_line(action: &ActionView, what: &str, data: &Words) -> String {
    let deed = deed(action, what, data);
    let mut letters = deed.chars();
    let first = letters.next().map(|c| c.to_ascii_uppercase());
    and_if_hurt(first.into_iter().chain(letters).collect(), action)
}

/// `line`, with ", and got hurt" if `action`'s attempt hurt its own sprite,
/// whatever hurt it.
fn and_if_hurt(line: String, action: &ActionView) -> String {
    if action.hurt.actor {
        format!("{line}, and got hurt")
    } else {
        line
    }
}

/// What an aimed action that applied did to `what`, its target in words,
/// in the past tense and lower case: "ate from the berry bush", "kicked the
/// ball". A kick is a Play that pushes, as the target's verb table says.
fn deed(action: &ActionView, what: &str, data: &Words) -> String {
    let pushes = |verb| action.target_type.is_some_and(|t| data.pushes(t, verb));
    match action.verb {
        // An Eat that hurts, a thornbush's say, gave no food.
        Verb::Eat if action.hurt.actor => format!("tried to eat {what}"),
        // A thing eaten whole is gone; one eaten from is still there.
        Verb::Eat if action.target_gone => format!("ate {what}"),
        Verb::Eat => format!("ate from {what}"),
        Verb::Drink => "drank".into(),
        Verb::Play if pushes(Verb::Play) => format!("kicked {what}"),
        Verb::Play => format!("played with {what}"),
        Verb::Hit => format!("hit {what}"),
        Verb::Retreat => format!("backed away from {what}"),
        Verb::Mate => format!("paired with {what}"),
        Verb::Court => format!("danced for {what}"),
        _ => format!("got to {what}"),
    }
}

/// Whether an Eat aimed at `action`'s target eats from it, as from a berry
/// bush, rather than eating it whole, as a berry: whether the target's
/// object type counts what's on it to eat, as a bush counts its fruit
/// (design §3.5.2). A try the gut has no room for leaves either there, so
/// its words can't go by whether the target's gone, as `deed`'s do.
fn eaten_from(action: &ActionView, data: &Words) -> bool {
    let name = action.target_type.and_then(|t| data.object_type_name(t));
    name.is_some_and(|name| !data.counter_names(name).is_empty())
}

/// Sprite `actor`'s finished `action` as the event log says it (design
/// §6.1), if the log shows it: every Play and Hit that applied, and any
/// attempt that hurt a sprite, even one that then failed. "Sprite #4
/// kicked a ball".
pub(crate) fn logged_line(actor: EntityId, action: &ActionView, data: &Words) -> Option<String> {
    let applied = action.progress == Progress::Ended(Outcome::Applied);
    let hurt = action.hurt.actor || action.hurt.target;
    let played = applied && matches!(action.verb, Verb::Play | Verb::Hit);
    if !played && !(action.attempted && hurt) {
        return None;
    }
    let what = match action.target? {
        Target::Sprite(id) => data.label(id),
        Target::Water(_) => "the water".into(),
        Target::Cursor => CURSOR.into(),
        Target::Object(_) => {
            let name = action.target_type.and_then(|id| data.object_type_name(id));
            with_article(&display_name(name.unwrap_or("?")))
        }
    };
    let deed = deed(action, &what, data);
    let hurt_itself = if action.hurt.actor {
        " and got hurt"
    } else {
        ""
    };
    Some(format!("{} {deed}{hurt_itself}", data.label(actor)))
}

/// What an aimed action is aimed at, in words: "the berry bush", "the
/// water", "Sprite #530". Empty for an action aimed at nothing.
fn target_words(action: &ActionView, data: &Words) -> String {
    match action.target {
        Some(Target::Sprite(id)) => data.label(id),
        Some(Target::Water(_)) => "the water".into(),
        Some(Target::Cursor) => CURSOR.into(),
        Some(Target::Object(_)) => {
            let name = action.target_type.and_then(|id| data.object_type_name(id));
            format!("the {}", display_name(name.unwrap_or("?")))
        }
        None => String::new(),
    }
}

/// How any action that ended without doing what it set out to ends, in plain words.
fn ended_line(outcome: Outcome) -> Option<String> {
    Some(
        match outcome {
            Outcome::Applied => return None,
            Outcome::Blocked => "Gave up: the way was blocked",
            Outcome::TimedOut => "Gave up: it took too long",
            Outcome::Failed => "Gave up: it couldn't get there",
            Outcome::Interrupted => "Changed its mind",
            Outcome::PulledAway => "Pulled away",
            Outcome::TooFull => "Too full",
        }
        .into(),
    )
}

/// A Wander in plain words, if it has any for `progress`.
fn wander_line(progress: Progress) -> Option<String> {
    Some(match progress {
        Progress::Walking { steps_left } => {
            format!("Wandering off · {} to go", counted(steps_left, "tile"))
        }
        Progress::Waiting { .. } => "Wandering off · waiting to get past".into(),
        Progress::Ended(Outcome::Applied) => "Arrived".into(),
        Progress::Ended(outcome) => return ended_line(outcome),
        Progress::Resting { .. } => return None,
    })
}

/// Breeding alone in plain words, if it has any for `progress` (M2 design
/// §10).
fn breed_alone_line(progress: Progress) -> Option<String> {
    Some(match progress {
        Progress::Ended(Outcome::Applied) => "Laid an egg alone".into(),
        Progress::Ended(Outcome::Failed) => "Couldn't breed alone".into(),
        Progress::Ended(outcome) => return ended_line(outcome),
        _ => "Breeding alone".into(),
    })
}

/// A Call in plain words (M2 design §15.8): calling while it listens.
fn call_line(progress: Progress) -> Option<String> {
    match progress {
        Progress::Resting { .. } => Some("Calling".into()),
        Progress::Ended(Outcome::Applied) => Some("Called".into()),
        Progress::Ended(outcome) => ended_line(outcome),
        _ => None,
    }
}

/// A Rest in plain words, if it has any for `progress`.
fn rest_line(progress: Progress) -> Option<String> {
    match progress {
        Progress::Resting { ticks, of } => Some(format!(
            "Resting · {} left",
            counted(of.saturating_sub(ticks), "tick")
        )),
        Progress::Ended(Outcome::Applied) => Some("Rested".into()),
        Progress::Ended(Outcome::Interrupted) => Some("Changed its mind".into()),
        _ => None,
    }
}

/// An action exactly: `WANDER → (61,40) · walking (5 tiles)`, or
/// `EAT → berry_bush #812 · applied`.
fn exact_line(action: &ActionView, data: &Words) -> String {
    let verb = verb_name(action.verb);
    let head = match (action.target, action.destination) {
        (Some(Target::Object(id)), _) => {
            let name = action.target_type.and_then(|t| data.object_type_name(t));
            format!("{verb} → {} #{}", name.unwrap_or("?"), id.0)
        }
        (Some(Target::Water(at)), _) => format!("{verb} → water ({},{})", at.x, at.y),
        (Some(Target::Sprite(id)), _) => format!("{verb} → sprite #{}", id.0),
        (Some(Target::Cursor), _) => format!("{verb} → cursor"),
        (None, Some(to)) => format!("{verb} → ({},{})", to.x, to.y),
        (None, None) => verb.to_string(),
    };
    let state = match action.progress {
        Progress::Walking { steps_left } => {
            format!("walking ({})", counted(steps_left, walked(action.verb)))
        }
        Progress::Waiting { blocked_ticks } => {
            format!("blocked ({})", counted(blocked_ticks, "tick"))
        }
        Progress::Resting { ticks, of } => format!("{ticks} of {of} ticks"),
        Progress::Ended(outcome) => outcome_name(outcome).to_string(),
    };
    // A trip to a remembered place (M2 design §7), or to a call (§15.1).
    let head = match (action.remembered, action.heard) {
        (true, Some(_)) => format!("{head} · by ear"),
        (true, None) => format!("{head} · from memory"),
        (false, _) => head,
    };
    format!("{head} · {state}")
}

/// A verb's name in the detail view.
pub(super) fn verb_name(verb: Verb) -> &'static str {
    match verb {
        Verb::Approach => "APPROACH",
        Verb::Eat => "EAT",
        Verb::Drink => "DRINK",
        Verb::Hit => "HIT",
        Verb::Play => "PLAY",
        Verb::Retreat => "RETREAT",
        Verb::Rest => "REST",
        Verb::Wander => "WANDER",
        Verb::Mate => "MATE",
        Verb::BreedAlone => "BREED_ALONE",
        Verb::Speak => "SPEAK",
        Verb::Court => "COURT",
        Verb::Call => "CALL",
    }
}

/// An outcome's name in the detail view, as the design names it (§5.5).
fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Applied => "applied",
        Outcome::Blocked => "blocked",
        Outcome::Failed => "failed",
        Outcome::Interrupted => "interrupted",
        Outcome::TimedOut => "timed_out",
        Outcome::PulledAway => "pulled_away",
        Outcome::TooFull => "too_full",
    }
}
