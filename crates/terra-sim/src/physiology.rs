//! Physiology (design §4.4, Appendix B): the body's fixed rules, which genes
//! can't change, from `physiology.ron`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::registry::{Chemical, ChemicalClass, Locus, LocusKind, Trait};

/// The physiology file, relative to the pack root.
pub(crate) const PHYSIOLOGY: &str = "physiology.ron";

/// The validated physiology.
#[derive(Debug, Clone)]
pub(crate) struct Physiology {
    pub(crate) newborn: Newborn,
    /// The fractions of the newborn level the first population's energy and
    /// hydration are drawn between.
    pub(crate) first_population: (f32, f32),
    pub(crate) metabolism: Metabolism,
    pub(crate) digestion: Digestion,
    pub(crate) hydration_loss: f32,
    pub(crate) stamina: Stamina,
    pub(crate) healing: f32,
    pub(crate) injury: Injury,
    /// What each cause-of-death tally is multiplied by every tick: the file's
    /// `cause_fade` is how many ticks it takes to halve.
    pub(crate) tally_fade: f32,
    pub(crate) traits: TraitRanges,
    /// Each receptor target's range, by locus ID.
    pub(crate) receptor_targets: BTreeMap<u16, (f32, f32)>,
    pub(crate) nearby_sprites: NearbySprites,
    pub(crate) spawn_variation: f32,
    pub(crate) indices: Indices,
}

/// `physiology.ron`, before validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PhysiologyEntry {
    newborn: Newborn,
    first_population: (f32, f32),
    metabolism: Metabolism,
    digestion: Digestion,
    hydration_loss: f32,
    stamina: Stamina,
    healing: f32,
    injury: Injury,
    cause_fade: u32,
    traits: TraitRanges,
    receptor_targets: BTreeMap<String, (f32, f32)>,
    nearby_sprites: NearbySprites,
    spawn_variation: f32,
}

/// How the `nearby_sprites` body sensor counts.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NearbySprites {
    /// How far away, in tiles in any direction, a sprite counts.
    pub(crate) radius: u16,
    /// How many sprites make the sensor read 1.
    pub(crate) full: u16,
}

/// What a newborn's physical chemicals start at.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Newborn {
    pub(crate) energy: f32,
    pub(crate) hydration: f32,
    pub(crate) stamina: f32,
}

/// Energy spent per tick.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metabolism {
    pub(crate) basal: f32,
    pub(crate) per_sense_tile: f32,
    /// Per step at speed 8; it scales with (speed / 8)².
    pub(crate) per_step: f32,
}

/// Gut contents moved into the body per tick.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Digestion {
    pub(crate) food: f32,
    pub(crate) water: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stamina {
    pub(crate) per_step: f32,
    pub(crate) idle: f32,
    pub(crate) resting: f32,
}

/// Injury per tick from each of physiology's causes.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Injury {
    pub(crate) starvation: f32,
    pub(crate) dehydration: f32,
    pub(crate) old_age: f32,
}

/// The range each trait is clamped to.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TraitRanges {
    pub(crate) speed: (f32, f32),
    pub(crate) sense_radius: (f32, f32),
    pub(crate) lifespan: (f32, f32),
}

impl TraitRanges {
    /// The range `which` is clamped to.
    pub(crate) fn range(&self, which: Trait) -> (f32, f32) {
        match which {
            Trait::Speed => self.speed,
            Trait::SenseRadius => self.sense_radius,
            Trait::Lifespan => self.lifespan,
        }
    }
}

impl PhysiologyEntry {
    /// The validated physiology, or what's wrong with it. Receptor targets are
    /// checked against the pack's `loci`.
    pub(crate) fn validate(self, indices: Indices, loci: &[Locus]) -> Result<Physiology, String> {
        let newborn = self.newborn;
        for (name, level) in [
            ("newborn.energy", newborn.energy),
            ("newborn.hydration", newborn.hydration),
            ("newborn.stamina", newborn.stamina),
        ] {
            fraction(name, level)?;
        }
        range("first_population", self.first_population)?;
        fraction("first_population", self.first_population.0)?;
        fraction("first_population", self.first_population.1)?;

        let (metabolism, digestion, stamina, injury) =
            (self.metabolism, self.digestion, self.stamina, self.injury);
        for (name, rate) in [
            ("metabolism.basal", metabolism.basal),
            ("metabolism.per_sense_tile", metabolism.per_sense_tile),
            ("metabolism.per_step", metabolism.per_step),
            ("digestion.food", digestion.food),
            ("digestion.water", digestion.water),
            ("hydration_loss", self.hydration_loss),
            ("stamina.per_step", stamina.per_step),
            ("stamina.idle", stamina.idle),
            ("stamina.resting", stamina.resting),
            ("healing", self.healing),
            ("injury.starvation", injury.starvation),
            ("injury.dehydration", injury.dehydration),
            ("injury.old_age", injury.old_age),
        ] {
            if !(rate.is_finite() && rate >= 0.0) {
                return Err(format!(
                    "`{name}` is {rate}, but must be a number, and can't be negative"
                ));
            }
        }
        if self.cause_fade == 0 {
            return Err("`cause_fade` must be at least 1 tick".into());
        }

        let traits = self.traits;
        for (name, bounds) in [
            ("traits.speed", traits.speed),
            ("traits.sense_radius", traits.sense_radius),
            ("traits.lifespan", traits.lifespan),
        ] {
            range(name, bounds)?;
            // A lifespan of 0 would make the age sensor 0 ÷ 0.
            if !(bounds.0 > 0.0 && bounds.1.is_finite()) {
                return Err(format!(
                    "`{name}` goes from {} to {}, but must be numbers above 0",
                    bounds.0, bounds.1
                ));
            }
        }

        let mut receptor_targets = BTreeMap::new();
        for locus in loci.iter().filter(|l| l.kind == LocusKind::ReceptorTarget) {
            let &bounds = self.receptor_targets.get(&locus.name).ok_or_else(|| {
                format!(
                    "`receptor_targets` has no range for `{}`, a receptor target in loci.ron",
                    locus.name
                )
            })?;
            range(&format!("receptor_targets.{}", locus.name), bounds)?;
            receptor_targets.insert(locus.id, bounds);
        }
        if let Some(name) = self.receptor_targets.keys().find(|name| {
            !loci
                .iter()
                .any(|l| &l.name == *name && l.kind == LocusKind::ReceptorTarget)
        }) {
            return Err(format!(
                "`receptor_targets` names `{name}`, which isn't a receptor target in loci.ron"
            ));
        }

        if self.nearby_sprites.radius == 0 || self.nearby_sprites.full == 0 {
            return Err("`nearby_sprites` needs a `radius` and a `full` of at least 1".into());
        }
        if !(0.0..1.0).contains(&self.spawn_variation) {
            return Err(format!(
                "`spawn_variation` is {}, but must be at least 0 and below 1",
                self.spawn_variation
            ));
        }

        Ok(Physiology {
            newborn,
            first_population: self.first_population,
            metabolism,
            digestion,
            hydration_loss: self.hydration_loss,
            stamina,
            healing: self.healing,
            injury,
            tally_fade: halving_factor(self.cause_fade as f32),
            traits,
            receptor_targets,
            nearby_sprites: self.nearby_sprites,
            spawn_variation: self.spawn_variation,
            indices,
        })
    }
}

/// What a level is multiplied by every tick so that it halves every `ticks` ticks.
pub(crate) fn halving_factor(ticks: f32) -> f32 {
    libm::powf(0.5, 1.0 / ticks)
}

/// Checks that `value` is a level: from 0 to 1.
fn fraction(name: &str, value: f32) -> Result<(), String> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!("`{name}` is {value}, but must be from 0 to 1"))
    }
}

/// Checks that `(low, high)` goes from low to high.
fn range(name: &str, (low, high): (f32, f32)) -> Result<(), String> {
    if low <= high {
        Ok(())
    } else {
        Err(format!(
            "`{name}` goes from {low} down to {high}, but must go from low to high"
        ))
    }
}

/// Where physiology finds the chemicals and body sensors it works on: indices
/// in the pack's chemical and locus order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Indices {
    pub(crate) energy: usize,
    pub(crate) hydration: usize,
    pub(crate) stamina: usize,
    pub(crate) food: usize,
    pub(crate) water: usize,
    pub(crate) injury: usize,
    pub(crate) always: usize,
    pub(crate) age: usize,
    pub(crate) nearby_sprites: usize,
    pub(crate) moving: usize,
    pub(crate) resting: usize,
}

impl Indices {
    /// Finds each physical chemical and body sensor physiology needs, or says
    /// which file lacks one.
    pub(crate) fn find(
        chemicals: &[Chemical],
        loci: &[Locus],
    ) -> Result<Indices, (&'static str, String)> {
        let chem = |name: &str| {
            chemicals
                .iter()
                .position(|c| c.name == name && c.class == ChemicalClass::Physical)
                .ok_or_else(|| {
                    (
                        "chemicals.ron",
                        format!("physiology needs a physical chemical called `{name}`"),
                    )
                })
        };
        let sensor = |name: &str| {
            loci.iter()
                .position(|l| l.name == name && l.kind == LocusKind::BodySensor)
                .ok_or_else(|| {
                    (
                        "loci.ron",
                        format!("physiology needs a body sensor called `{name}`"),
                    )
                })
        };
        Ok(Indices {
            energy: chem("energy")?,
            hydration: chem("hydration")?,
            stamina: chem("stamina")?,
            food: chem("food")?,
            water: chem("water")?,
            injury: chem("injury")?,
            always: sensor("always")?,
            age: sensor("age")?,
            nearby_sprites: sensor("nearby_sprites")?,
            moving: sensor("moving")?,
            resting: sensor("resting")?,
        })
    }
}
