//! Physiology (design §4.4, Appendix B): the body's fixed rules, which genes
//! can't change, from `physiology.ron`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::object_types::Size;
use crate::registry::{BrainParam, Chemical, ChemicalClass, Locus, LocusId, LocusKind, Trait};

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
    pub(crate) brain: BrainRanges,
    /// Each receptor target's range, by locus ID.
    pub(crate) receptor_targets: BTreeMap<LocusId, (f32, f32)>,
    pub(crate) nearby_sprites: NearbySprites,
    pub(crate) spawn_variation: f32,
    /// How far a learned value gets from 0 to be a lesson (design §5.6).
    pub(crate) lesson_threshold: f32,
    /// The least fall of a need in a tick that is relief (design §5.6).
    pub(crate) relief_deadband: f32,
    /// How many ticks after a try its target is still the thing touched.
    pub(crate) touch_window: u64,
    /// A remembered sprite is forgotten once everything learned about it is
    /// nearer 0 than this (design v18 §5.6).
    pub(crate) forget_below: f32,
    pub(crate) actions: Actions,
    pub(crate) movement: Movement,
    /// What the Cursor's touch does (design v21 §4.6).
    pub(crate) cursor: Cursor,
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
    brain: BTreeMap<String, ParamRange>,
    receptor_targets: BTreeMap<String, (f32, f32)>,
    nearby_sprites: NearbySprites,
    spawn_variation: f32,
    lesson_threshold: f32,
    relief_deadband: f32,
    touch_window: u64,
    forget_below: f32,
    actions: Actions,
    movement: Movement,
    cursor: Cursor,
}

/// A brain parameter's range and default (design §5.7, Appendix B).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ParamRange {
    /// What a `BrainParam` gene is clamped to.
    pub(crate) range: (f32, f32),
    /// The value with no gene for it.
    pub(crate) default: f32,
}

/// Every brain parameter's range and default, in `BrainParam::ALL` order.
#[derive(Debug, Clone)]
pub(crate) struct BrainRanges(Vec<ParamRange>);

impl BrainRanges {
    /// `param`'s range and default.
    pub(crate) fn of(&self, param: BrainParam) -> ParamRange {
        let index = BrainParam::ALL.iter().position(|&p| p == param);
        self.0[index.expect("every parameter is in ALL")]
    }
}

/// How sprites find their way (design §3.6–3.7).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Movement {
    /// How many ticks after a flood is made it's made again, if the sprite
    /// hasn't moved sooner.
    pub(crate) flood_refresh: u32,
    /// What the flood adds for a tile holding another sprite, in terrain units.
    pub(crate) occupied_penalty: u32,
    /// How many blocked ticks in a row start the search for a way round.
    pub(crate) replan_after: u32,
}

/// What the Cursor's touch does (design v21 §4.6): the levels it raises.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cursor {
    /// The reward a pet injects.
    pub(crate) pet: f32,
    /// The reward a hug, an amplified pet, injects.
    pub(crate) hug: f32,
    /// What a zap injects.
    pub(crate) zap: Correction,
    /// What a shock, an amplified zap, injects.
    pub(crate) shock: Correction,
    /// The longest a Reward looks back for the sprite's latest try, in ticks
    /// (design v21 §5.6). A Correct looks back only `touch_window`.
    pub(crate) max_reach_back: u64,
    /// The furthest the Cursor throws or shoves a thing of each size, in
    /// tiles (design v25 §3.5.4).
    pub(crate) furthest: Furthest,
}

/// The furthest the Cursor sends a thing of each size, in tiles (design v25
/// §3.5.4). Size stands in for weight until things have weights.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Furthest {
    pub(crate) small: u16,
    pub(crate) medium: u16,
    pub(crate) large: u16,
}

impl Furthest {
    /// The furthest for a thing of `size`.
    pub(crate) fn of(self, size: Size) -> u16 {
        match size {
            Size::Small => self.small,
            Size::Medium => self.medium,
            Size::Large => self.large,
        }
    }
}

/// What the Cursor's Correct injects (design v21 §4.6). It hurts, but adds
/// no injury.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Correction {
    pub(crate) punishment: f32,
    pub(crate) pain: f32,
}

/// How long actions last (design §5.5).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Actions {
    /// How long a Rest lasts, in ticks.
    pub(crate) rest_bout: u32,
    /// How many steps a Retreat takes.
    pub(crate) retreat_bout: u32,
    /// How long any action may last.
    pub(crate) timeout: u32,
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

        let mut brain = Vec::new();
        for param in BrainParam::ALL {
            let name = param.name();
            let &entry = self
                .brain
                .get(name)
                .ok_or_else(|| format!("`brain` has no range for `{name}`"))?;
            range(&format!("brain.{name}"), entry.range)?;
            if !(entry.range.0..=entry.range.1).contains(&entry.default) {
                return Err(format!(
                    "`brain.{name}` has the default {}, outside its range",
                    entry.default
                ));
            }
            brain.push(entry);
        }
        if let Some(name) = self
            .brain
            .keys()
            .find(|name| BrainParam::named(name).is_none())
        {
            return Err(format!(
                "`brain` names `{name}`, which isn't a brain parameter"
            ));
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

        for (name, ticks) in [
            ("actions.rest_bout", self.actions.rest_bout),
            ("actions.timeout", self.actions.timeout),
            ("movement.flood_refresh", self.movement.flood_refresh),
            ("movement.replan_after", self.movement.replan_after),
        ] {
            if ticks == 0 {
                return Err(format!("`{name}` must be at least 1 tick"));
            }
        }
        if self.actions.retreat_bout == 0 {
            return Err("`actions.retreat_bout` must be at least 1 step".into());
        }

        let cursor = self.cursor;
        for (name, level) in [
            ("cursor.pet", cursor.pet),
            ("cursor.hug", cursor.hug),
            ("cursor.zap.punishment", cursor.zap.punishment),
            ("cursor.zap.pain", cursor.zap.pain),
            ("cursor.shock.punishment", cursor.shock.punishment),
            ("cursor.shock.pain", cursor.shock.pain),
        ] {
            fraction(name, level)?;
        }
        if cursor.max_reach_back < self.touch_window {
            return Err(format!(
                "`cursor.max_reach_back` is {}, but must be at least `touch_window`, {}",
                cursor.max_reach_back, self.touch_window
            ));
        }
        let furthest = cursor.furthest;
        for (size, tiles) in [
            ("small", furthest.small),
            ("medium", furthest.medium),
            ("large", furthest.large),
        ] {
            if tiles == 0 {
                return Err(format!(
                    "`cursor.furthest.{size}` must be at least 1 tile: a throw or a shove goes somewhere"
                ));
            }
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
            brain: BrainRanges(brain),
            receptor_targets,
            nearby_sprites: self.nearby_sprites,
            spawn_variation: self.spawn_variation,
            lesson_threshold: self.lesson_threshold,
            relief_deadband: self.relief_deadband,
            touch_window: self.touch_window,
            forget_below: self.forget_below,
            actions: self.actions,
            movement: self.movement,
            cursor: self.cursor,
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
    /// The learning signals, which step 4 uses up (design §5.6).
    pub(crate) reward: usize,
    pub(crate) punishment: usize,
    /// The drive the Cursor's Correct raises (design v21 §4.6).
    pub(crate) pain: usize,
    pub(crate) always: usize,
    pub(crate) age: usize,
    pub(crate) nearby_sprites: usize,
    pub(crate) moving: usize,
    pub(crate) resting: usize,
    /// The receptor target that scales the brain's temperatures (design §5.3, §5.5).
    pub(crate) exploration_mod: usize,
    /// The receptor target that scales learning (design §5.6).
    pub(crate) learning_rate_mod: usize,
    /// The receptor target that scales curiosity (design §5.3).
    pub(crate) curiosity_mod: usize,
    /// The pulse a retreat that finds no step away fires (design §3.7).
    pub(crate) cornered: usize,
    /// The pulse whose source is the attacker, the Sprite candidate while
    /// it's live (design §3.6).
    pub(crate) was_hit: usize,
    /// The pulse a fruitless try fires (design §5.2).
    pub(crate) fruitless: usize,
    /// The pulses the Cursor's Reward and Correct fire (design v21 §4.6).
    pub(crate) petted: usize,
    pub(crate) shocked: usize,
}

impl Indices {
    /// Finds each physical chemical and body sensor physiology needs, the
    /// receptor target the brain reads, and the pulses a cornered retreat
    /// and a fruitless try fire and an attacker is known by, or says which
    /// file lacks one.
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
        let signal = |name: &str| {
            chemicals
                .iter()
                .position(|c| c.name == name && c.class == ChemicalClass::Signal)
                .ok_or_else(|| {
                    (
                        "chemicals.ron",
                        format!("sprites need a signal chemical called `{name}`"),
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
        let target = |name: &str| {
            loci.iter()
                .position(|l| l.name == name && l.kind == LocusKind::ReceptorTarget)
                .ok_or_else(|| {
                    (
                        "loci.ron",
                        format!("the brain needs a receptor target called `{name}`"),
                    )
                })
        };
        let pulse = |name: &str| {
            loci.iter()
                .position(|l| l.name == name && l.kind == LocusKind::Pulse)
                .ok_or_else(|| ("loci.ron", format!("sprites need a pulse called `{name}`")))
        };
        Ok(Indices {
            energy: chem("energy")?,
            hydration: chem("hydration")?,
            stamina: chem("stamina")?,
            food: chem("food")?,
            water: chem("water")?,
            injury: chem("injury")?,
            reward: signal("reward")?,
            punishment: signal("punishment")?,
            pain: signal("pain")?,
            always: sensor("always")?,
            age: sensor("age")?,
            nearby_sprites: sensor("nearby_sprites")?,
            moving: sensor("moving")?,
            resting: sensor("resting")?,
            exploration_mod: target("exploration_mod")?,
            learning_rate_mod: target("learning_rate_mod")?,
            curiosity_mod: target("curiosity_mod")?,
            cornered: pulse("cornered")?,
            was_hit: pulse("was_hit")?,
            fruitless: pulse("fruitless")?,
            petted: pulse("petted")?,
            shocked: pulse("shocked")?,
        })
    }
}
