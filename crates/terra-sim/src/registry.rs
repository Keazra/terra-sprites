//! The registries (design §2.8, Appendix A): stable, append-only IDs.
//! Chemicals, loci and categories are data, listed in the pack. Verbs and
//! traits are closed enums, because code gives each one its meaning.

use serde::{Deserialize, Serialize};

use crate::listed::listed_enum;

/// A category's permanent ID (design v19 §3.5.5): what brains perceive a
/// thing as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct CategoryId(pub(crate) u16);

listed_enum! {
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
        /// Having young with another adult (M2 design §5).
        Mate = 9,
        /// Reserved for M6 (Words).
        Speak = 10,
        /// Laying an egg alone, as a female can (M2 design §10).
        BreedAlone = 11,
        /// Dancing for another adult, to court it (M2 design §15.3).
        Court = 12,
        /// Calling, for whoever hears to come (M2 design §15.1).
        Call = 13,
    }

    pub(crate) const ALL;
}

impl Verb {
    /// Whether the verb is aimed at a target (design §5.2): a movement or
    /// contact verb.
    pub(crate) fn is_aimed(self) -> bool {
        matches!(self, Verb::Approach | Verb::Retreat) || self.is_contact()
    }

    /// Whether the verb walks to a goal tile of its target (design §3.6,
    /// §5.5): an aimed verb other than Retreat, which backs away from it.
    pub(crate) fn heads_for_goal(self) -> bool {
        self.is_aimed() && self != Verb::Retreat
    }

    /// Whether the verb's ID is only reserved, for a later milestone.
    pub(crate) fn is_reserved(self) -> bool {
        self == Verb::Speak
    }

    /// Whether the verb acts on its target through the target's verb table
    /// (design §5.2). The others move, rest, mate or are reserved.
    pub(crate) fn is_interaction(self) -> bool {
        matches!(self, Verb::Eat | Verb::Drink | Verb::Hit | Verb::Play)
    }

    /// Whether the verb walks to its target and makes one attempt on it,
    /// which the sprite learns from as a touch (design §5.5, §5.6): an
    /// interaction, or mating or courting, which the engine itself resolves
    /// (M2 design §5, §15.3).
    pub(crate) fn is_contact(self) -> bool {
        self.is_interaction() || self.is_mating()
    }

    /// Whether the verb is mating or courting, which only ever happen
    /// between adults and are never learned by watching (M2 design §5,
    /// §18.3).
    pub(crate) fn is_mating(self) -> bool {
        matches!(self, Verb::Mate | Verb::Court)
    }
}

listed_enum! {
    /// A setting of how the brain works (design §5.7, Appendix B), set by a
    /// `BrainParam` gene. The discriminants are the stable parameter IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    pub(crate) enum BrainParam {
        /// Does nothing since v16, when learning became worth and habits; kept
        /// for its ID, since genomes may carry it.
        LearningRate = 1 => "learning_rate",
        TraceDecay = 2 => "trace_decay",
        /// Does nothing since v16; kept for its ID.
        RelaxRate = 3 => "relax_rate",
        /// Does nothing since v16; kept for its ID.
        ConsolidateRate = 4 => "consolidate_rate",
        TauBase = 5 => "tau_base",
        TauAttBase = 6 => "tau_att_base",
        SwitchMargin = 7 => "switch_margin",
        AttentionMargin = 8 => "attention_margin",
        SalienceGain = 9 => "salience_gain",
        /// This and the next four do nothing while the recruitable pool is on
        /// hold (design v16 §5.4); kept for their IDs.
        PoolSize = 10 => "pool_size",
        MaxArity = 11 => "max_arity",
        RecruitThreshold = 12 => "recruit_threshold",
        NoveltyThreshold = 13 => "novelty_threshold",
        ForgetTicks = 14 => "forget_ticks",
        WorthRateGood = 15 => "worth_rate_good",
        WorthRateBad = 16 => "worth_rate_bad",
        WorthFadeGood = 17 => "worth_fade_good",
        WorthFadeBad = 18 => "worth_fade_bad",
        HabitRate = 19 => "habit_rate",
        HabitFade = 20 => "habit_fade",
        ValueGain = 21 => "value_gain",
        Curiosity = 22 => "curiosity",
        FamiliarityRate = 23 => "familiarity_rate",
        Disappointment = 24 => "disappointment",
        /// How fast a particular sprite is learned good (design v18 §5.6).
        IndividualRateGood = 25 => "individual_rate_good",
        /// How fast a particular sprite is learned bad (design v18 §5.6).
        IndividualRateBad = 26 => "individual_rate_bad",
        /// How fast whoever hurt it is feared (design v18 §5.6).
        FearRate = 27 => "fear_rate",
        /// How much fear fades each tick (design v18 §5.6).
        FearFade = 28 => "fear_fade",
        /// How many sprites it knows before sprites in general are judged in
        /// full (design v18 §5.6).
        Generalise = 29 => "generalise",
        /// How strongly fear catches the eye (design v18 §5.3).
        Vigilance = 30 => "vigilance",
        /// How strongly fear pulls towards backing away (design v18 §5.5).
        Flight = 31 => "flight",
        /// The distance, as a share of sight, at which fear stops pulling
        /// (design v18 §5.3).
        FearReach = 32 => "fear_reach",
        /// How many object types in a category it knows before the category's
        /// summary counts in full (design v19 §5.6).
        GeneraliseTypes = 33 => "generalise_types",
        /// How much a pressing first-order need quiets what the sprite merely
        /// likes (design v21 §5.6).
        Quieting = 34 => "quieting",
        /// How much a bad habit fades each tick (design v21 §5.6).
        HabitFadeBad = 35 => "habit_fade_bad",
        /// How fast fear of the Cursor wears off while it stays near the sprite
        /// and does nothing to it (design v29 §5.6).
        CursorCalming = 36 => "cursor_calming",
        /// How much a remembered place fades each tick (M2 design §7).
        PlaceFade = 37 => "place_fade",
        /// How much faster than its other rates say a sprite learns: the
        /// young learn faster (M2 design §9.5).
        Plasticity = 38 => "plasticity",
        /// How much a Reward or Correct teaches the taught habit of the
        /// sprite's latest try (design §5.6, #187).
        TaughtRate = 39 => "taught_rate",
        /// How much a taught habit fades each tick (design §5.6, #187).
        TaughtFade = 40 => "taught_fade",
        /// How much familiarity with a sprite grows each tick it's in sight
        /// (M2 design §15.2).
        BefriendRate = 41 => "befriend_rate",
        /// How much a social play adds to familiarity (M2 design §15.2).
        BefriendPlay = 42 => "befriend_play",
        /// How much familiarity with a sprite fades each tick (M2 design
        /// §15.2).
        BefriendFade = 43 => "befriend_fade",
        /// The share of the way a sprite's view moves towards what the
        /// sprite it watches knows (M2 design §18.1).
        WatchRate = 44 => "watch_rate",
        /// How strong a verb's pull must be for a sprite with nothing to do
        /// to start it; below it, it stands where it is (M2 design §19.2).
        ActThreshold = 45 => "act_threshold",
    }

    pub(crate) const ALL;

    /// The parameter's name in genome files and `physiology.ron`.
    pub(crate) fn name(self) -> &'static str;

    /// The parameter called `name`, if there is one.
    pub(crate) fn named;
}

impl BrainParam {
    /// Whether it counts something, so spawn variation rounds it (design §4.9).
    pub(crate) fn is_whole(self) -> bool {
        matches!(
            self,
            BrainParam::PoolSize | BrainParam::MaxArity | BrainParam::ForgetTicks
        )
    }
}

listed_enum! {
    /// An evolvable body trait (design §4.8). The discriminants are the stable trait IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    pub enum Trait {
        /// Move points per tick.
        Speed = 1 => "speed",
        /// How far the sprite perceives, in tiles.
        SenseRadius = 2 => "sense_radius",
        /// The age it grows old at, in ticks (M2 design §9.3). It was
        /// `lifespan` before M2, when it was the age old age harmed it from,
        /// and saves from then call it that.
        #[serde(alias = "Lifespan")]
        GrowsOld = 3 => "grows_old",
        /// The age it grows up at, in ticks (M2 design §9.3).
        GrowsUp = 4 => "grows_up",
    }

    pub(crate) const ALL;

    /// The trait's name in genome files and `physiology.ron`.
    pub(crate) fn name(self) -> &'static str;

    /// The trait called `name`, if there is one.
    pub(crate) fn named;
}

impl Trait {
    /// The trait a genome file calls `name`: by its name, or by the name it
    /// had before M2 (M2 design §9.3), so older files still load.
    pub(crate) fn read(name: &str) -> Option<Trait> {
        Trait::named(name).or(match name {
            "lifespan" => Some(Trait::GrowsOld),
            _ => None,
        })
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
    /// Drives and learning signals: changed by the genome and the Cursor.
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
    /// What it is, in a sentence, which the screen shows when the pointer
    /// is on it (design §6.1): for a drive, what raises it and what lowers it.
    /// Empty if the pack doesn't say.
    #[serde(default)]
    pub(crate) about: String,
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
    /// An event that lasts one tick, written by the Cursor and by verbs.
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

#[cfg(test)]
mod tests {
    use super::{BrainParam, Trait, Verb};

    #[test]
    fn stable_ids_stay_put_and_names_round_trip() {
        for (i, verb) in Verb::ALL.into_iter().enumerate() {
            assert_eq!(verb as u16, (i + 1) as u16);
        }

        assert_eq!(
            Trait::ALL.map(Trait::name),
            ["speed", "sense_radius", "grows_old", "grows_up"]
        );
        for (i, one) in Trait::ALL.into_iter().enumerate() {
            assert_eq!(one as u16, (i + 1) as u16);
            assert_eq!(Trait::named(one.name()), Some(one));
        }
        assert_eq!(Trait::named("no_such"), None);
        assert_eq!(
            Trait::read("lifespan"),
            Some(Trait::GrowsOld),
            "its old name"
        );

        // `ALL`'s order is the ID order, from 1. Renaming a parameter still
        // round-trips here. Loading `physiology.ron` is what rejects a name
        // the data doesn't list, and a missing range for one it does.
        assert_eq!(BrainParam::ALL.len(), 45);
        for (i, param) in BrainParam::ALL.into_iter().enumerate() {
            assert_eq!(param as u16, (i + 1) as u16);
            assert_eq!(BrainParam::named(param.name()), Some(param));
        }
        assert_eq!(
            BrainParam::named("learning_rate"),
            Some(BrainParam::LearningRate)
        );
        assert_eq!(BrainParam::named("place_fade"), Some(BrainParam::PlaceFade));
        assert_eq!(
            BrainParam::named("taught_rate"),
            Some(BrainParam::TaughtRate)
        );
        assert_eq!(
            BrainParam::named("taught_fade"),
            Some(BrainParam::TaughtFade)
        );
        assert_eq!(
            BrainParam::named("befriend_fade"),
            Some(BrainParam::BefriendFade)
        );
        assert_eq!(BrainParam::named(""), None);
    }
}
