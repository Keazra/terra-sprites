//! The registries (design §2.8, Appendix A): stable, append-only IDs. Chemicals
//! and loci are data, listed in the pack. Categories, verbs and traits are closed
//! enums, because code gives each one its meaning.

use serde::{Deserialize, Serialize};

/// What brains perceive an object as. The discriminants are the stable `CategoryId`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Category {
    BerryBush = 1,
    Berry = 2,
    Thornbush = 3,
    Water = 4,
    Ball = 5,
    Sprite = 6,
}

impl Category {
    pub(crate) const ALL: [Category; 6] = [
        Category::BerryBush,
        Category::Berry,
        Category::Thornbush,
        Category::Water,
        Category::Ball,
        Category::Sprite,
    ];

    /// The category's name in brain input names, such as `attended_berry_bush`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Category::BerryBush => "berry_bush",
            Category::Berry => "berry",
            Category::Thornbush => "thornbush",
            Category::Water => "water",
            Category::Ball => "ball",
            Category::Sprite => "sprite",
        }
    }
}

/// A kind of action, and a brain output (design §5.2). The discriminants are
/// the stable verb IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Verb {
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
    pub(crate) const ALL: [Verb; 10] = [
        Verb::Approach,
        Verb::Eat,
        Verb::Drink,
        Verb::Hit,
        Verb::Play,
        Verb::Retreat,
        Verb::Rest,
        Verb::Wander,
        Verb::Mate,
        Verb::Speak,
    ];

    /// Whether the verb is aimed at a target (design §5.2): a movement or
    /// interaction verb.
    pub(crate) fn is_aimed(self) -> bool {
        matches!(self, Verb::Approach | Verb::Retreat) || self.is_interaction()
    }

    /// Whether the verb's ID is only reserved, for a later milestone.
    pub(crate) fn is_reserved(self) -> bool {
        matches!(self, Verb::Mate | Verb::Speak)
    }

    /// Whether the verb acts on its target through the target's verb table
    /// (design §5.2). The others move, rest or are reserved.
    pub(crate) fn is_interaction(self) -> bool {
        matches!(self, Verb::Eat | Verb::Drink | Verb::Hit | Verb::Play)
    }
}

/// A setting of how the brain works (design §5.7, Appendix B), set by a
/// `BrainParam` gene. The discriminants are the stable parameter IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum BrainParam {
    LearningRate = 1,
    TraceDecay = 2,
    RelaxRate = 3,
    ConsolidateRate = 4,
    TauBase = 5,
    TauAttBase = 6,
    SwitchMargin = 7,
    AttentionMargin = 8,
    SalienceGain = 9,
    PoolSize = 10,
    MaxArity = 11,
    RecruitThreshold = 12,
    NoveltyThreshold = 13,
    ForgetTicks = 14,
}

impl BrainParam {
    pub(crate) const ALL: [BrainParam; 14] = [
        BrainParam::LearningRate,
        BrainParam::TraceDecay,
        BrainParam::RelaxRate,
        BrainParam::ConsolidateRate,
        BrainParam::TauBase,
        BrainParam::TauAttBase,
        BrainParam::SwitchMargin,
        BrainParam::AttentionMargin,
        BrainParam::SalienceGain,
        BrainParam::PoolSize,
        BrainParam::MaxArity,
        BrainParam::RecruitThreshold,
        BrainParam::NoveltyThreshold,
        BrainParam::ForgetTicks,
    ];

    /// The parameter's name in genome files and `physiology.ron`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            BrainParam::LearningRate => "learning_rate",
            BrainParam::TraceDecay => "trace_decay",
            BrainParam::RelaxRate => "relax_rate",
            BrainParam::ConsolidateRate => "consolidate_rate",
            BrainParam::TauBase => "tau_base",
            BrainParam::TauAttBase => "tau_att_base",
            BrainParam::SwitchMargin => "switch_margin",
            BrainParam::AttentionMargin => "attention_margin",
            BrainParam::SalienceGain => "salience_gain",
            BrainParam::PoolSize => "pool_size",
            BrainParam::MaxArity => "max_arity",
            BrainParam::RecruitThreshold => "recruit_threshold",
            BrainParam::NoveltyThreshold => "novelty_threshold",
            BrainParam::ForgetTicks => "forget_ticks",
        }
    }

    /// The parameter called `name`, if there is one.
    pub(crate) fn named(name: &str) -> Option<BrainParam> {
        BrainParam::ALL.into_iter().find(|p| p.name() == name)
    }

    /// Whether it counts something, so spawn variation rounds it (design §4.9).
    pub(crate) fn is_whole(self) -> bool {
        matches!(
            self,
            BrainParam::PoolSize | BrainParam::MaxArity | BrainParam::ForgetTicks
        )
    }
}

/// An evolvable body trait (design §4.8). The discriminants are the stable trait IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Trait {
    /// Move points per tick.
    Speed = 1,
    /// How far the sprite perceives, in tiles.
    SenseRadius = 2,
    /// Ticks until old age.
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

/// A chemical's stable ID (Appendix A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ChemId(pub(crate) u16);

/// A locus's stable ID (Appendix A). Chemical levels, which are loci too, go
/// by their `ChemId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct LocusId(pub(crate) u16);

impl std::fmt::Display for ChemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for LocusId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// What changes a chemical, and so who may change it (design §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ChemicalClass {
    /// Changed only by physiology and object verbs.
    Physical,
    /// Drives and learning signals: changed by the genome and the hand.
    Signal,
    /// Spare channels the genome may put to use.
    Hormone,
}

/// What a chemical is to a sprite (design §4.1): its class, with the signal
/// chemicals told apart into drives and learning signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChemicalKind {
    /// The body's actual state, changed only by physiology and object verbs.
    Physical,
    /// A signal chemical the sprite feels as an urge.
    Drive,
    /// Reward or punishment, which learning uses up every tick.
    LearningSignal,
    /// A spare channel the genome may put to use.
    Hormone,
}

/// The signal chemicals that are learning signals. Every other one is a drive.
const LEARNING_SIGNALS: [&str; 2] = ["reward", "punishment"];

/// One entry of `chemicals.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chemical {
    pub(crate) id: ChemId,
    pub(crate) name: String,
    pub(crate) class: ChemicalClass,
}

impl Chemical {
    /// What the chemical is to a sprite.
    pub(crate) fn kind(&self) -> ChemicalKind {
        match self.class {
            ChemicalClass::Physical => ChemicalKind::Physical,
            ChemicalClass::Hormone => ChemicalKind::Hormone,
            ChemicalClass::Signal if LEARNING_SIGNALS.contains(&self.name.as_str()) => {
                ChemicalKind::LearningSignal
            }
            ChemicalClass::Signal => ChemicalKind::Drive,
        }
    }
}

/// What fills a locus in (design §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum LocusKind {
    /// Filled in by physiology every tick.
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
    pub(crate) id: LocusId,
    pub(crate) name: String,
    pub(crate) kind: LocusKind,
}
