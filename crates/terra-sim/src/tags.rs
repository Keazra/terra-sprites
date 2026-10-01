//! Tags defined in the data (design v23 §3.5.6): what each contact with a
//! thing that has one does to the sprite making it. `Solid` and `Fixture` are
//! built in, as physics (design §3.5.1), and aren't defined here.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::data::DataError;
use crate::object_types::{Effect, EffectEntry, Party, effect_name, injectable, signallable};
use crate::registry::{Chemical, Locus, Verb};

/// The tags file, relative to the pack root.
pub(crate) const TAGS: &str = "tags.ron";

/// The built-in tags (design §3.5.1): physics, not data.
pub(crate) const SOLID: &str = "Solid";
pub(crate) const FIXTURE: &str = "Fixture";

/// A contact (design v23 §3.5.6): a sprite touching a thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) enum Contact {
    Eat,
    Hit,
    Play,
    /// A sliding sprite stopping against the thing (design §3.5.4).
    Crash,
}

impl Contact {
    /// The contact a sprite makes by doing `verb` to a thing: none for a
    /// drink, since water has no tags, nor for a verb that doesn't touch.
    pub(crate) fn of(verb: Verb) -> Option<Contact> {
        match verb {
            Verb::Eat => Some(Contact::Eat),
            Verb::Hit => Some(Contact::Hit),
            Verb::Play => Some(Contact::Play),
            _ => None,
        }
    }
}

/// A validated tag: what each contact with a thing that has it does to the
/// sprite making it, only ever an `Inject` or a `Signal` on the Actor.
#[derive(Debug, Clone)]
pub(crate) struct Tag {
    pub(crate) name: String,
    pub(crate) contact: BTreeMap<Contact, Vec<Effect>>,
}

/// One entry of `tags.ron`, before validation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TagEntry {
    name: String,
    #[serde(default)]
    contact: BTreeMap<Contact, Vec<EffectEntry>>,
}

/// Parses and validates `tags.ron`, keeping the tags in listed order.
pub(crate) fn tags(
    entries: Vec<TagEntry>,
    chemicals: &[Chemical],
    loci: &[Locus],
) -> Result<Vec<Tag>, DataError> {
    let mut seen = Vec::new();
    entries
        .into_iter()
        .map(|entry| {
            let name = entry.name.clone();
            let invalid = |message: String| DataError::Invalid {
                file: TAGS.into(),
                message,
            };
            if name == SOLID || name == FIXTURE {
                return Err(invalid(format!(
                    "`{name}` is built in, as physics, so it isn't defined here"
                )));
            }
            if seen.contains(&name) {
                return Err(invalid(format!("`{name}` is defined more than once")));
            }
            seen.push(name.clone());
            entry
                .resolve(chemicals, loci)
                .map_err(|problem| DataError::Invalid {
                    file: TAGS.into(),
                    message: format!("`{name}`: {problem}"),
                })
        })
        .collect()
}

impl TagEntry {
    /// The validated tag, or what's wrong with the entry.
    fn resolve(self, chemicals: &[Chemical], loci: &[Locus]) -> Result<Tag, String> {
        let contact = self
            .contact
            .into_iter()
            .map(|(contact, effects)| {
                let effects = effects
                    .iter()
                    .map(|effect| match *effect {
                        EffectEntry::Inject(Party::Actor, ref name, amount) => Ok(Effect::Inject(
                            Party::Actor,
                            injectable(chemicals, name)?,
                            amount,
                        )),
                        EffectEntry::Signal(Party::Actor, ref name) => {
                            Ok(Effect::Signal(Party::Actor, signallable(loci, name)?))
                        }
                        EffectEntry::Inject(Party::Target, ..)
                        | EffectEntry::Signal(Party::Target, ..) => Err(format!(
                            "`{}` acts on the Target, but a contact acts only on the Actor, the sprite making it",
                            effect_name(effect)
                        )),
                        _ => Err(format!(
                            "`{}` isn't allowed: a contact may only Inject and Signal",
                            effect_name(effect)
                        )),
                    })
                    .collect::<Result<Vec<_>, String>>()
                    .map_err(|e| format!("the {contact:?} contact: {e}"))?;
                Ok((contact, effects))
            })
            .collect::<Result<_, String>>()?;
        Ok(Tag {
            name: self.name,
            contact,
        })
    }
}
