//! What the event log says (design §6.1): a line for each event it
//! shows, and the sprites each one is about.

use terra_sim::{
    Blocker, Command, CursorTouch, Emptied, EntityId, Event, EventKind, Grip, Learned,
    MAX_NAME_CHARS, NameProblem, PlaceRule, Rejection, Removal, Target, Terrain, Thing,
};

use super::actions::logged_line;
use super::body::parents_text;
use super::brain::learned_name;
use super::{CURSOR, with_article};
use crate::text;
use crate::text::{ROOTED, Words, cause_name, display_name, terrain_name, thousands};

/// A lesson as the event log says it (design §6.1): "Sprite #12 learned:
/// thornbushes are bad", "… water is good for thirst", "… eating balls is
/// bad"; one learned by `watching` another, "Sprite #21 learned from
/// watching Sprite #12: …" (M2 design §18.4).
pub(crate) fn learned_line(
    id: EntityId,
    learned: &Learned,
    good: bool,
    watching: Option<EntityId>,
    data: &Words,
) -> String {
    let verdict = if good { "good" } else { "bad" };
    let what = match learned {
        Learned::Worth { .. } | Learned::Bad { .. } | Learned::Fear { .. } => {
            learned_name(learned, data)
        }
        Learned::Habit { .. } => format!("{} is {verdict}", learned_name(learned, data)),
        Learned::NewThings => format!("new things are {verdict}"),
    };
    match watching {
        Some(other) => format!(
            "{} learned from watching {}: {what}",
            data.label(id),
            data.label(other)
        ),
        None => format!("{} learned: {what}", data.label(id)),
    }
}

/// Why the world refused `command`, spoken to the player, as the event log
/// and the status line say it (design v22, v23 §6.1): "Couldn't pet Sprite
/// #12: it's gone", "Couldn't put the ball down: a berry is there",
/// "Couldn't throw the ball: a berry is there" (design v25).
fn refusal_line(command: &Command, reason: Rejection, data: &Words) -> Option<String> {
    let name = |id: u16| display_name(data.object_type_name(id).unwrap_or("?"));
    let what = match command.clone() {
        Command::Reward { sprite, .. } | Command::Correct { sprite, .. } => {
            let touch = CursorTouch::of(command).expect("a touch");
            format!("{} {}", touch.name(), data.label(sprite))
        }
        Command::TakeHold { sprite } => format!("take hold of {}", data.label(sprite)),
        Command::PickUp { .. } => "pick it up".into(),
        Command::PutDown { .. } => match reason {
            Rejection::InTheWay { item_type, .. } => format!("put the {} down", name(item_type)),
            _ => "put it down".into(),
        },
        Command::LetGo => "let go".into(),
        // The app sends it only while leading or visible, and only onto
        // the map; and the world never refuses showing or hiding it.
        Command::MoveCursor { .. } | Command::ShowCursor { .. } => return None,
        // Refused as putting it down would be (design v25 §2.5).
        Command::Throw { .. } => match reason {
            Rejection::InTheWay { item_type, .. } | Rejection::NotThrowable { item_type } => {
                format!("throw the {}", name(item_type))
            }
            _ => "throw it".into(),
        },
        Command::Shove { .. } => "shove".into(),
        Command::Place { object_type, .. } => format!("place the {}", name(object_type)),
        Command::SpawnSprite { .. } => "put a new sprite there".into(),
        Command::Rename { sprite, .. } => format!("name {}", data.label(sprite)),
    };
    let why = match reason {
        Rejection::Gone => "it's gone".into(),
        Rejection::Busy(Grip::Leads(led)) => {
            format!("you're already leading {}", data.label(led))
        }
        Rejection::Busy(Grip::Holds(_)) => "you're already holding something".into(),
        Rejection::NotLeading => "you're not leading a sprite".into(),
        Rejection::Rooted => ROOTED.into(),
        Rejection::NotHolding => "you're not holding anything".into(),
        // An egg (M2 design §10.4).
        Rejection::NotThrowable { .. } => "it can only be put down".into(),
        Rejection::OffTheMap => "that's off the map".into(),
        Rejection::InTheWay { blocker, .. } | Rejection::NoRoom(blocker) => match blocker {
            Blocker::Object(there) => format!("{} is there", with_article(&name(there))),
            Blocker::Sprite => "a sprite is there".into(),
            Blocker::Terrain(terrain) => {
                let into = match terrain {
                    Terrain::ShallowWater | Terrain::DeepWater => "in",
                    _ => "on",
                };
                format!("it can't go {into} {}", terrain_name(terrain))
            }
        },
        Rejection::NotPlaceable => "the Cursor can't make one of those".into(),
        Rejection::PlaceRule { rule, .. } => match rule {
            PlaceRule::KeepsPathsOpen => "it would block the way".into(),
            PlaceRule::Fertility => "the ground there doesn't suit it".into(),
            PlaceRule::DensityBelow(crowding) => {
                let crowd = data.object_type_name(crowding).unwrap_or("?");
                let crowd = data
                    .plural_of(crowd)
                    .map_or_else(|| display_name(crowd), str::to_string);
                format!("there are too many {crowd} near")
            }
            PlaceRule::Itself => "it can't go there".into(),
        },
        Rejection::BadName(NameProblem::Empty) => "a name needs a letter in it".into(),
        Rejection::BadName(NameProblem::TooLong) => {
            format!("a name has at most {MAX_NAME_CHARS} letters")
        }
        Rejection::BadName(NameProblem::NotCp437) => {
            "the game can't show some of its letters".into()
        }
        Rejection::BadGenome => "its genome doesn't fit this world".into(),
    };
    Some(format!("Couldn't {what}: {why}"))
}

/// What an event says in the event log, if the log shows it (design §6.1).
pub(crate) fn event_line(event: &Event, data: &Words) -> Option<String> {
    match &event.kind {
        EventKind::Died {
            id,
            name,
            cause,
            age,
        } => Some(format!(
            "{} died ({}, age {})",
            name.as_deref()
                .map_or_else(|| data.label(*id), |name| text::label(*id, Some(name))),
            cause_name(*cause, data),
            thousands(*age)
        )),
        EventKind::ActionEnded { id, action, .. } => logged_line(*id, action, data),
        // Life stages (M2 design §9.6).
        EventKind::GrewUp { id } => Some(format!("{} grew up", data.label(*id))),
        EventKind::GrewOld { id } => Some(format!("{} grew old", data.label(*id))),
        // Breeding (M2 design §10).
        EventKind::Laid { by, with, .. } => Some(match with {
            Some(with) => format!(
                "{} laid an egg (with {})",
                data.label(*by),
                data.label(*with)
            ),
            None => format!("{} laid an egg alone", data.label(*by)),
        }),
        // With the genes copying changed in what they do (M2 design §13.2).
        EventKind::Hatched { id, parents } => {
            let new = data
                .news
                .get(id)
                .map_or(String::new(), |new| format!(", with something new: {new}"));
            Some(match parents_text(parents, data) {
                Some(parents) => format!("{} hatched ({parents}'s){new}", data.label(*id)),
                None => format!("{} hatched{new}", data.label(*id)),
            })
        }
        EventKind::TurnedFemale { id } => Some(format!("{} turned female", data.label(*id))),
        // Partners (M2 design §15.8).
        EventKind::Paired { a, b } => Some(format!(
            "{} and {} are now partners",
            data.label(*a),
            data.label(*b)
        )),
        EventKind::LostPartner { id, partner } => {
            Some(format!("{} lost {}", data.label(*id), data.label(*partner)))
        }
        EventKind::Parted { a, b } => {
            Some(format!("{} and {} parted", data.label(*a), data.label(*b)))
        }
        EventKind::LearnedMilestone {
            id,
            learned,
            good,
            watching,
        } => Some(learned_line(*id, learned, *good, *watching, data)),
        // The Cursor's touch, spoken to the player (design v21 §6.1).
        EventKind::Rewarded { id, .. } | EventKind::Corrected { id, .. } => {
            let touch = CursorTouch::reported(&event.kind).expect("the Cursor's touch");
            let done = match touch {
                CursorTouch::Pet => "petted",
                CursorTouch::Hug => "hugged",
                CursorTouch::Zap => "zapped",
                CursorTouch::Shock => "shocked",
            };
            Some(format!("You {done} {}", data.label(*id)))
        }
        EventKind::CommandRejected { command, reason } => refusal_line(command, *reason, data),
        // Grabbing, spoken to the player (design v23 §6.1).
        EventKind::TookHold { sprite } => Some(format!("You took hold of {}", data.label(*sprite))),
        EventKind::LetGo { sprite } => Some(format!("You let go of {}", data.label(*sprite))),
        EventKind::PickedUp { object_type, .. } => Some(format!(
            "You picked up {}",
            with_article(&display_name(object_type))
        )),
        EventKind::PutDown { object_type, .. } => {
            Some(format!("You put the {} down", display_name(object_type)))
        }
        EventKind::CursorEmptied {
            reason:
                Emptied::Removed {
                    object_type,
                    reason,
                    ..
                },
        } => {
            let went = match reason {
                Removal::Expired => "expired",
                Removal::Destroyed => "was destroyed",
                Removal::Replaced => "was replaced",
            };
            let name = display_name(object_type);
            Some(format!("The {name} you were holding {went}"))
        }
        // A led sprite's death has its own line.
        EventKind::CursorEmptied {
            reason: Emptied::Died { .. },
        } => None,
        EventKind::ObjectSpawned { .. }
        | EventKind::ObjectRemoved { .. }
        | EventKind::ActionStarted { .. } => None,
        // Throwing and shoving, spoken to the player, and a crash only if it
        // hurt (design v25 §6.1).
        EventKind::Threw { object_type, .. } => {
            Some(format!("You threw the {}", display_name(object_type)))
        }
        EventKind::Shoved { sprite } => Some(format!("You shoved {}", data.label(*sprite))),
        EventKind::Crashed {
            sprite,
            into,
            hurt: true,
        } => Some(format!(
            "{} was shoved into {} and got hurt",
            data.label(*sprite),
            crashed_into(into, data)
        )),
        EventKind::Crashed { hurt: false, .. } => None,
        // The Place menu and naming, spoken to the player (design v28 §6.5).
        EventKind::Placed { object_type, .. } => Some(format!(
            "You placed {}",
            with_article(&display_name(object_type))
        )),
        EventKind::Spawned { id, .. } => Some(format!("You made {}", data.label(*id))),
        EventKind::Renamed { id, name } => Some(format!("You named Sprite #{} {name}", id.0)),
    }
}

/// The sprites an event is about, first the one it's mostly about: who
/// did it, then who it was done to. Empty for an event about no sprite.
pub(crate) fn event_sprites(event: &Event) -> Vec<EntityId> {
    match &event.kind {
        EventKind::Died { id, .. }
        | EventKind::LearnedMilestone { id, .. }
        | EventKind::Rewarded { id, .. }
        | EventKind::Corrected { id, .. }
        | EventKind::Spawned { id, .. }
        | EventKind::Renamed { id, .. }
        | EventKind::GrewUp { id }
        | EventKind::GrewOld { id }
        | EventKind::TurnedFemale { id }
        | EventKind::ActionStarted { id, .. } => vec![*id],
        EventKind::Paired { a, b } | EventKind::Parted { a, b } => vec![*a, *b],
        EventKind::LostPartner { id, partner } => vec![*id, *partner],
        EventKind::Laid { egg, by, with } => [Some(*by), *with, Some(*egg)]
            .into_iter()
            .flatten()
            .collect(),
        EventKind::Hatched { id, parents } => std::iter::once(*id)
            .chain(parents.iter().copied())
            .collect(),
        EventKind::ActionEnded { id, action, .. } => match action.target {
            Some(Target::Sprite(target)) => vec![*id, target],
            _ => vec![*id],
        },
        EventKind::TookHold { sprite }
        | EventKind::LetGo { sprite }
        | EventKind::Shoved { sprite } => vec![*sprite],
        EventKind::Crashed { sprite, into, .. } => match into {
            Thing::Sprite(other) => vec![*sprite, *other],
            _ => vec![*sprite],
        },
        EventKind::CursorEmptied {
            reason: Emptied::Died { sprite },
        } => vec![*sprite],
        EventKind::CommandRejected { command, .. } => match command {
            Command::Reward { sprite, .. }
            | Command::Correct { sprite, .. }
            | Command::TakeHold { sprite }
            | Command::Rename { sprite, .. } => vec![*sprite],
            _ => Vec::new(),
        },
        EventKind::ObjectSpawned { .. }
        | EventKind::ObjectRemoved { .. }
        | EventKind::Placed { .. }
        | EventKind::PickedUp { .. }
        | EventKind::PutDown { .. }
        | EventKind::Threw { .. }
        | EventKind::CursorEmptied { .. } => Vec::new(),
    }
}

/// The sprites a refused command's line names: the one it was aimed at,
/// and the one the Cursor already leads if that's why (design §6.4, #174).
pub(crate) fn refusal_sprites(event: &Event) -> Vec<EntityId> {
    let mut sprites = event_sprites(event);
    if let EventKind::CommandRejected {
        reason: Rejection::Busy(Grip::Leads(led)),
        ..
    } = event.kind
    {
        sprites.push(led);
    }
    sprites
}

/// What a sprite crashed into, as a sentence says it: "a thornbush", or
/// "Sprite #7" (design v25 §6.1).
pub(crate) fn crashed_into(into: &Thing, data: &Words) -> String {
    match into {
        Thing::ObjectType(name) | Thing::Category(name) => with_article(&display_name(name)),
        Thing::Sprite(id) => data.label(*id),
        Thing::Cursor => CURSOR.into(),
    }
}
