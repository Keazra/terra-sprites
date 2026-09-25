//! The registries (design §2.8, Appendix A): stable, append-only IDs. Chemicals
//! and loci are data, listed in the pack. Categories, verbs and traits are closed
//! enums, because code gives each one its meaning.

use serde::{Deserialize, Serialize};

/// What brains perceive an object as. The discriminants are the stable `CategoryId`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) enum Category {
    BerryBush = 1,
    Berry = 2,
    Thornbush = 3,
    Water = 4,
    Ball = 5,
    Sprite = 6,
}

/// A brain output. The discriminants are the stable verb IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub(crate) enum Verb {
    Approach = 1,
    Eat = 2,
    Drink = 3,
    Hit = 4,
    Play = 5,
    Retreat = 6,
    Rest = 7,
    Wander = 8,
    /// Reserved for M2.
    Mate = 9,
    /// Reserved for M4.
    Speak = 10,
}

impl Verb {
    /// Whether the verb acts on its target through the target's verb table
    /// (design §5.2). The others move, rest or are reserved.
    pub(crate) fn is_interaction(self) -> bool {
        matches!(self, Verb::Eat | Verb::Drink | Verb::Hit | Verb::Play)
    }
}

/// An evolvable body trait (design §4.8). The discriminants are the stable trait IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Trait {
    Speed = 1,
    SenseRadius = 2,
    Lifespan = 3,
}

impl Trait {
    pub(crate) const ALL: [Trait; 3] = [Trait::Speed, Trait::SenseRadius, Trait::Lifespan];

    /// The trait's name in genome files and `physiology.ron`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Trait::Speed => "speed",
            Trait::SenseRadius => "sense_radius",
            Trait::Lifespan => "lifespan",
        }
    }

    /// The trait called `name`, if there is one.
    pub(crate) fn named(name: &str) -> Option<Trait> {
        Trait::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// What changes a chemical, and so who may change it (design §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ChemicalClass {
    /// Changed only by physics and object verbs.
    Physical,
    /// Drives and learning signals: changed by the genome and the hand.
    Signal,
    /// Spare channels the genome may put to use.
    Hormone,
}

/// One entry of `chemicals.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chemical {
    pub(crate) id: u16,
    pub(crate) name: String,
    pub(crate) class: ChemicalClass,
}

/// What fills a locus in (design §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum LocusKind {
    /// Filled in by physics every tick.
    BodySensor,
    /// An event that lasts one tick, written by the hand and by verbs.
    Pulse,
    /// Written by receptors.
    ReceptorTarget,
}

/// One entry of `loci.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Locus {
    pub(crate) id: u16,
    pub(crate) name: String,
    pub(crate) kind: LocusKind,
    #[expect(dead_code, reason = "the brain's inputs are built from it in slice 8")]
    pub(crate) brain_visible: bool,
}
