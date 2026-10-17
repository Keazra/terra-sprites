//! Life stages and ageing (M2 design §9): a sprite is young, then an adult,
//! then old, at ages its genes set, and a gene may switch on or off at a
//! life stage.

use serde::{Deserialize, Deserializer, Serialize};

use crate::genome::Gene;
use crate::registry::Trait;

/// A phase of a sprite's life (M2 design §9.1). Objects have stages of their
/// own; a sprite's are life stages. They come in this order.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum LifeStage {
    /// From hatching, or for a sprite placed young, being born, until it
    /// grows up. An egg is an item until it hatches (M2 design §10.4, #196),
    /// so a save or genome file from before that may say `Egg`, which reads
    /// as young: an egg's genes were expressed as a young sprite's.
    #[default]
    #[serde(alias = "Egg")]
    Young,
    /// From growing up until it grows old.
    Adult,
    /// From growing old until it dies.
    Old,
}

impl LifeStage {
    /// Whether a sprite at this stage is grown up: an adult, or old, which
    /// still counts as one for mating (M2 design §5, §9.1).
    pub fn is_grown(self) -> bool {
        matches!(self, LifeStage::Adult | LifeStage::Old)
    }

    /// The stage's name, as genome files and the player see it.
    pub fn name(self) -> &'static str {
        match self {
            LifeStage::Young => "young",
            LifeStage::Adult => "adult",
            LifeStage::Old => "old",
        }
    }
}

/// When a gene works (M2 design §9.2): from the life stage `from` on, until
/// the stage `until`, if it has one, when it switches off. A gene with the
/// default switch works all its life.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Switch {
    /// The stage it switches on at.
    pub from: LifeStage,
    /// The stage it switches off at, if any.
    pub until: Option<LifeStage>,
}

impl Switch {
    /// Whether the gene works in `stage`.
    pub fn is_on(self, stage: LifeStage) -> bool {
        self.from <= stage && self.until.is_none_or(|until| stage < until)
    }

    /// Whether the gene works all its life.
    pub fn is_always(self) -> bool {
        self == Switch::default()
    }

    /// Whether it ever works: a switch off at or before the stage it
    /// switches on at never does.
    pub(crate) fn ever_on(self) -> bool {
        self.until.is_none_or(|until| self.from < until)
    }

    /// The switch as a genome file writes it after a gene's own fields:
    /// nothing for one that's always on.
    pub(crate) fn to_ron(self) -> String {
        let mut text = String::new();
        if self.from != LifeStage::Young {
            text += &format!(", from: {:?}", self.from);
        }
        if let Some(until) = self.until {
            text += &format!(", until: {until:?}");
        }
        text
    }
}

/// How much of a hit of damage `gene` takes (M2 design §9.4): all of it for
/// a gene that's a strength of the body, half for an instinct, so the mind
/// outlasts the body, and none for a gene that's a setting, not a strength,
/// which doesn't weaken.
pub(crate) fn frailty(gene: &Gene) -> f32 {
    match gene {
        Gene::Emitter { .. } | Gene::Receptor { .. } | Gene::Reaction { .. } => 1.0,
        Gene::Trait {
            which: Trait::Speed | Trait::SenseRadius,
            ..
        } => 1.0,
        Gene::Instinct { .. } | Gene::AttentionInstinct { .. } => 0.5,
        Gene::Trait { .. }
        | Gene::HalfLife { .. }
        | Gene::InitialConcentration { .. }
        | Gene::BrainParam { .. }
        | Gene::Unknown { .. }
        | Gene::Unmatched(_) => 0.0,
    }
}

/// The health of the gene at `index`, from 1 (whole) to 0 (M2 design
/// §9.4): whole where `health` has no entry for it.
pub(crate) fn health_of(health: &[f32], index: usize) -> f32 {
    health.get(index).copied().unwrap_or(1.0)
}

/// Reads a bare life stage as `Some`, for a genome file's `until`.
pub(crate) fn some_stage<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<LifeStage>, D::Error> {
    LifeStage::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_works_from_its_stage_until_its_end() {
        let adult_only = Switch {
            from: LifeStage::Adult,
            until: Some(LifeStage::Old),
        };
        assert!(!adult_only.is_on(LifeStage::Young));
        assert!(adult_only.is_on(LifeStage::Adult));
        assert!(!adult_only.is_on(LifeStage::Old));
        let always = Switch::default();
        assert!(always.is_always());
        for stage in [LifeStage::Young, LifeStage::Adult, LifeStage::Old] {
            assert!(always.is_on(stage));
        }
    }

    #[test]
    fn a_switch_off_before_it_is_on_never_works() {
        let never = Switch {
            from: LifeStage::Old,
            until: Some(LifeStage::Adult),
        };
        assert!(!never.ever_on());
        assert!(
            Switch {
                from: LifeStage::Young,
                until: Some(LifeStage::Adult)
            }
            .ever_on()
        );
    }

    #[test]
    fn an_old_sprite_still_counts_as_grown_up() {
        // M2 design §5, §9.1: for mating, an old sprite is an adult.
        assert!(!LifeStage::Young.is_grown());
        assert!(LifeStage::Adult.is_grown());
        assert!(LifeStage::Old.is_grown());
    }
}
