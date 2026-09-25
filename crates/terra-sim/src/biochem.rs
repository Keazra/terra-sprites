//! Biochemistry (design §4): a genome compiled for the chemistry step, and
//! the step itself, a pure function of a body and what it sensed.

use serde::Serialize;

use crate::data::DataPack;
use crate::events::DeathCause;
use crate::expression::{Expression, expressions};
use crate::genome::{Gene, Genome, LocusRef, Mode, Term};
use crate::registry::{LocusKind, Trait};

/// A sprite's traits (design §4.8), clamped to physiology's ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Traits {
    pub(crate) speed: f32,
    pub(crate) sense_radius: f32,
    pub(crate) lifespan: f32,
}

/// A genome as the chemistry step runs it: its expressed genes only, with
/// chemicals and loci as slots in the pack's order.
#[derive(Debug, Clone)]
pub(crate) struct Program {
    pub(crate) traits: Traits,
    /// What each chemical is multiplied by every tick: 1 where no half-life is set.
    decay: Vec<f32>,
    reactions: Vec<Reaction>,
    /// Level emitters, in genome order.
    level_emitters: Vec<Emitter>,
    /// Rise and Fall emitters, in genome order.
    change_emitters: Vec<(Change, Emitter)>,
    /// Each receptor target's slot and range, and the receptors that write it.
    targets: Vec<Target>,
    /// Each signal chemical's slot and level at birth, where a gene sets one.
    initial: Vec<(usize, f32)>,
}

/// A receptor target and the receptors aimed at it.
#[derive(Debug, Clone)]
struct Target {
    slot: usize,
    range: (f32, f32),
    /// Each receptor's chemical slot, threshold and gain.
    receptors: Vec<(usize, f32, f32)>,
}

/// Which way a Rise or Fall emitter looks.
#[derive(Debug, Clone, Copy)]
enum Change {
    Rise,
    Fall,
}

/// An emitter, compiled.
#[derive(Debug, Clone, Copy)]
struct Emitter {
    read: Read,
    invert: bool,
    threshold: f32,
    gain: f32,
    chem: usize,
}

/// A value the chemistry step can read: a chemical's level or a locus's value, by slot.
#[derive(Debug, Clone, Copy)]
enum Read {
    Chem(usize),
    Locus(usize),
}

impl Read {
    fn value(self, body: &Body) -> f32 {
        match self {
            Read::Chem(slot) => body.chems[slot],
            Read::Locus(slot) => body.loci[slot],
        }
    }

    /// The value as the previous tick's step 3 left it.
    fn last(self, body: &Body) -> f32 {
        match self {
            Read::Chem(slot) => body.last_chems[slot],
            Read::Locus(slot) => body.last_loci[slot],
        }
    }
}

/// A reaction, compiled.
#[derive(Debug, Clone)]
struct Reaction {
    /// Each reactant's slot and coefficient.
    reactants: Vec<(usize, f32)>,
    /// How much each chemical the reaction changes gains per unit of extent:
    /// products minus reactants. A catalyst, the same on both sides, isn't
    /// listed, so it's left exactly as it was.
    net: Vec<(usize, f32)>,
    rate: f32,
}

impl Reaction {
    fn new(
        reactants: &[Term],
        products: &[Term],
        rate: f32,
        slot: impl Fn(u16) -> usize,
    ) -> Reaction {
        let mut net: Vec<(usize, f32)> = Vec::new();
        let terms = reactants
            .iter()
            .map(|t| (t, -1.0))
            .chain(products.iter().map(|t| (t, 1.0)));
        for (term, sign) in terms {
            let change = sign * f32::from(term.coefficient);
            match net.iter_mut().find(|(s, _)| *s == slot(term.chem)) {
                Some((_, gain)) => *gain += change,
                None => net.push((slot(term.chem), change)),
            }
        }
        net.retain(|&(_, gain)| gain != 0.0);
        Reaction {
            reactants: reactants
                .iter()
                .map(|t| (slot(t.chem), f32::from(t.coefficient)))
                .collect(),
            net,
            rate,
        }
    }
}

impl Program {
    /// Compiles `genome`'s expressed genes.
    pub(crate) fn new(genome: &Genome, data: &DataPack) -> Program {
        let range = |which: Trait| data.physiology().traits.range(which);
        // A trait no gene sets takes the middle of its range.
        let value = |which: Trait| {
            let (low, high) = range(which);
            low + (high - low) / 2.0
        };
        let mut traits = [
            value(Trait::Speed),
            value(Trait::SenseRadius),
            value(Trait::Lifespan),
        ];
        let chem = |id: u16| data.chemical_slot(id).expect("a checked gene");
        let mut decay = vec![1.0; data.chemicals().len()];
        let mut reactions = Vec::new();
        let mut level_emitters = Vec::new();
        let mut change_emitters = Vec::new();
        let mut initial = Vec::new();
        let mut targets: Vec<Target> = data
            .physiology()
            .receptor_targets
            .iter()
            .map(|(&id, &range)| Target {
                slot: data.locus_slot(id).expect("a validated receptor target"),
                range,
                receptors: Vec::new(),
            })
            .collect();
        for (gene, expression) in genome.genes.iter().zip(expressions(genome, data)) {
            if expression != Expression::Expressed {
                continue;
            }
            match *gene {
                Gene::HalfLife { chem: id, ticks } => {
                    decay[chem(id)] = libm::powf(0.5, 1.0 / ticks as f32);
                }
                Gene::Reaction {
                    ref reactants,
                    ref products,
                    rate,
                } => reactions.push(Reaction::new(reactants, products, rate, chem)),
                Gene::Emitter {
                    locus,
                    mode,
                    invert,
                    threshold,
                    gain,
                    chem: id,
                } => {
                    let emitter = Emitter {
                        read: match locus {
                            LocusRef::Chem(id) => Read::Chem(chem(id)),
                            LocusRef::Locus(id) => {
                                Read::Locus(data.locus_slot(id).expect("a checked gene"))
                            }
                        },
                        invert,
                        threshold,
                        gain,
                        chem: chem(id),
                    };
                    match mode {
                        Mode::Level => level_emitters.push(emitter),
                        Mode::Rise => change_emitters.push((Change::Rise, emitter)),
                        Mode::Fall => change_emitters.push((Change::Fall, emitter)),
                    }
                }
                Gene::Receptor {
                    chem: id,
                    threshold,
                    gain,
                    target,
                } => {
                    let slot = data.locus_slot(target).expect("a checked gene");
                    let target = targets
                        .iter_mut()
                        .find(|t| t.slot == slot)
                        .expect("an expressed receptor writes a receptor target");
                    target.receptors.push((chem(id), threshold, gain));
                }
                Gene::InitialConcentration { chem: id, value } => initial.push((chem(id), value)),
                Gene::Trait { which, value } => {
                    let (low, high) = range(which);
                    traits[which as usize - 1] = value.clamp(low, high);
                }
                _ => {}
            }
        }
        let [speed, sense_radius, lifespan] = traits;
        Program {
            traits: Traits {
                speed,
                sense_radius,
                lifespan,
            },
            decay,
            reactions,
            level_emitters,
            change_emitters,
            targets,
            initial,
        }
    }
}

/// A sprite's chemistry (design §4.1–§4.2): every chemical's level and every
/// locus's value, as the chemistry step leaves them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Body {
    /// Each chemical's level, in the pack's chemical order.
    pub(crate) chems: Vec<f32>,
    /// Each locus's value, in the pack's locus order: body sensors, live
    /// pulses and receptor targets.
    pub(crate) loci: Vec<f32>,
    /// Pulses written since the last latch, by locus: they go live at the next one.
    pub(crate) incoming: Vec<f32>,
    /// `chems` and `loci` as the previous tick's step 3 left them, for Rise
    /// and Fall emitters.
    last_chems: Vec<f32>,
    last_loci: Vec<f32>,
    /// The injury each of physiology's causes added lately, fading, in
    /// `DeathCause::ALL` order (design §4.10).
    pub(crate) tallies: [f32; 3],
}

impl Body {
    /// A newborn's body (design §4.7).
    pub(crate) fn newborn(program: &Program, data: &DataPack) -> Body {
        let physiology = data.physiology();
        let (slots, newborn) = (physiology.slots, physiology.newborn);
        let mut chems = vec![0.0; data.chemicals().len()];
        chems[slots.energy] = newborn.energy;
        chems[slots.hydration] = newborn.hydration;
        chems[slots.stamina] = newborn.stamina;
        for &(slot, level) in &program.initial {
            chems[slot] = level;
        }
        // Receptor targets rest at 1 (design §4.2).
        let loci: Vec<f32> = data
            .loci()
            .iter()
            .map(|l| {
                if l.kind == LocusKind::ReceptorTarget {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        Body {
            last_chems: chems.clone(),
            last_loci: loci.clone(),
            incoming: vec![0.0; loci.len()],
            chems,
            loci,
            tallies: [0.0; 3],
        }
    }

    /// What caused most of the body's recent injury (design §4.10). Ties go
    /// to the cause listed first.
    pub(crate) fn cause_of_death(&self) -> DeathCause {
        let mut causes = DeathCause::ALL.into_iter().zip(self.tallies);
        let first = causes.next().expect("there are causes");
        causes
            .fold(
                first,
                |most, next| if next.1 > most.1 { next } else { most },
            )
            .0
    }
}

/// What the world tells the chemistry step about a sprite.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Senses {
    /// Ticks since it was born.
    pub(crate) age: u64,
    /// Sprites within 3 tiles, divided by 4, capped at 1 (design §4.2).
    pub(crate) nearby_sprites: f32,
    /// The steps it took during the previous tick.
    pub(crate) steps: u32,
    /// Whether a Rest action is running.
    pub(crate) resting: bool,
}

/// Step 3 for one sprite (design §4.4). Returns whether its injury has
/// reached 1, so it's dying.
pub(crate) fn step(program: &Program, body: &mut Body, senses: &Senses, data: &DataPack) -> bool {
    // (a) The pulse latch: what came in goes live, and the buffer empties.
    for (slot, locus) in data.loci().iter().enumerate() {
        if locus.kind == LocusKind::Pulse {
            body.loci[slot] = std::mem::take(&mut body.incoming[slot]);
        }
    }
    physiology(program, body, senses, data);
    // (c) Reactions, in genome order.
    for reaction in &program.reactions {
        let most = reaction
            .reactants
            .iter()
            .map(|&(slot, coefficient)| body.chems[slot] / coefficient)
            .fold(f32::INFINITY, f32::min);
        let extent = reaction.rate * most;
        for &(slot, gain) in &reaction.net {
            body.chems[slot] += gain * extent;
        }
    }
    // (d) Half-life decay.
    for (level, factor) in body.chems.iter_mut().zip(&program.decay) {
        *level *= factor;
    }
    // (e1) Level emitters, in genome order, each seeing those before it.
    for emitter in &program.level_emitters {
        let value = emitter.read.value(body);
        let signal = if emitter.invert { 1.0 - value } else { value };
        body.chems[emitter.chem] += emitter.gain * (signal - emitter.threshold).max(0.0);
    }
    // (e2) Rise and Fall emitters, in genome order, after every Level emitter.
    for &(change, emitter) in &program.change_emitters {
        let delta = emitter.read.value(body) - emitter.read.last(body);
        let signal = match change {
            Change::Rise => delta.max(0.0),
            Change::Fall => (-delta).max(0.0),
        };
        body.chems[emitter.chem] += emitter.gain * (signal - emitter.threshold).max(0.0);
    }
    // (f) Clamp.
    for level in &mut body.chems {
        *level = level.clamp(0.0, 1.0);
    }
    // (g) Receptors.
    for target in &program.targets {
        let pull: f32 = target
            .receptors
            .iter()
            .map(|&(chem, threshold, gain)| gain * (body.chems[chem] - threshold).max(0.0))
            .sum();
        body.loci[target.slot] = (1.0 + pull).clamp(target.range.0, target.range.1);
    }
    body.last_chems.clone_from(&body.chems);
    body.last_loci.clone_from(&body.loci);
    body.chems[data.physiology().slots.injury] >= 1.0
}

/// Step 3(b), physiology (design §4.4): the body's fixed rules, which read
/// only the traits, the physical chemicals and what the sprite sensed, and
/// write only the physical chemicals and the body sensors.
fn physiology(program: &Program, body: &mut Body, senses: &Senses, data: &DataPack) {
    let physiology = data.physiology();
    let (slots, traits) = (physiology.slots, program.traits);
    let chems = &mut body.chems;
    let steps = senses.steps as f32;
    let pace = traits.speed / 8.0;
    let metabolism = physiology.metabolism;
    chems[slots.energy] -= metabolism.basal
        + metabolism.per_sense_tile * traits.sense_radius
        + steps * metabolism.per_step * pace * pace;
    for (gut, into, rate) in [
        (slots.food, slots.energy, physiology.digestion.food),
        (slots.water, slots.hydration, physiology.digestion.water),
    ] {
        let digested = chems[gut].min(rate);
        chems[gut] -= digested;
        chems[into] += digested;
    }
    chems[slots.hydration] -= physiology.hydration_loss;
    chems[slots.stamina] += if senses.steps > 0 {
        -steps * physiology.stamina.per_step
    } else if senses.resting {
        physiology.stamina.resting
    } else {
        physiology.stamina.idle
    };
    chems[slots.injury] -= physiology.healing;

    let age = senses.age as f32;
    let injury = physiology.injury;
    let harms = [
        (chems[slots.energy] <= 0.0, injury.starvation),
        (chems[slots.hydration] <= 0.0, injury.dehydration),
        (age > traits.lifespan, injury.old_age),
    ];
    for (tally, (harmed, amount)) in body.tallies.iter_mut().zip(harms) {
        *tally *= physiology.cause_fade;
        if harmed {
            chems[slots.injury] += amount;
            *tally += amount;
        }
    }

    let loci = &mut body.loci;
    loci[slots.always] = 1.0;
    loci[slots.age] = (age / traits.lifespan).min(1.0);
    loci[slots.nearby_sprites] = senses.nearby_sprites;
    loci[slots.moving] = if senses.steps > 0 { 1.0 } else { 0.0 };
    loci[slots.resting] = if senses.resting { 1.0 } else { 0.0 };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    /// The built-in physiology with every rate at 0, so only genes change a body.
    fn quiet() -> DataPack {
        let mut physiology = include_str!("../../../data/physiology.ron").to_string();
        for (rate, zero) in [
            ("basal: 0.0001,", "basal: 0.0,"),
            ("per_sense_tile: 0.0000067,", "per_sense_tile: 0.0,"),
            ("per_step: 0.0002,", "per_step: 0.0,"),
            (
                "digestion: (food: 0.001, water: 0.002)",
                "digestion: (food: 0.0, water: 0.0)",
            ),
            ("hydration_loss: 0.00033", "hydration_loss: 0.0"),
            (
                "stamina: (per_step: 0.002, idle: 0.0005, resting: 0.002)",
                "stamina: (per_step: 0.0, idle: 0.0, resting: 0.0)",
            ),
            ("healing: 0.0001", "healing: 0.0"),
            (
                "injury: (starvation: 0.0011, dehydration: 0.0011, old_age: 0.0006)",
                "injury: (starvation: 0.0, dehydration: 0.0, old_age: 0.0)",
            ),
        ] {
            assert!(physiology.contains(rate), "{rate} is in physiology.ron");
            physiology = physiology.replace(rate, zero);
        }
        let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
            .iter()
            .map(|&(path, text)| {
                let text = if path == "physiology.ron" {
                    physiology.as_str()
                } else {
                    text
                };
                (path, text)
            })
            .collect();
        DataPack::from_sources(&sources).expect("the quiet pack is valid")
    }

    /// A sprite with a genome of `genes`, newborn in `data`.
    struct Subject {
        data: DataPack,
        program: Program,
        body: Body,
    }

    impl Subject {
        fn new(data: DataPack, genes: &[&str]) -> Subject {
            let text = format!("(format: 1, genes: [{}])", genes.join(", "));
            let program = Program::new(
                &Genome::from_ron(&text, &data).expect("a valid genome"),
                &data,
            );
            let body = Body::newborn(&program, &data);
            Subject {
                data,
                program,
                body,
            }
        }

        fn quiet(genes: &[&str]) -> Subject {
            Subject::new(quiet(), genes)
        }

        fn chem_slot(&self, name: &str) -> usize {
            self.data
                .chemicals()
                .iter()
                .position(|c| c.name == name)
                .expect("a chemical")
        }

        fn locus_slot(&self, name: &str) -> usize {
            self.data
                .loci()
                .iter()
                .position(|l| l.name == name)
                .expect("a locus")
        }

        fn level(&self, chemical: &str) -> f32 {
            self.body.chems[self.chem_slot(chemical)]
        }

        fn set(&mut self, chemical: &str, level: f32) {
            let slot = self.chem_slot(chemical);
            self.body.chems[slot] = level;
        }

        fn locus(&self, name: &str) -> f32 {
            self.body.loci[self.locus_slot(name)]
        }

        fn pulse(&mut self, name: &str) {
            let slot = self.locus_slot(name);
            self.body.incoming[slot] = 1.0;
        }

        /// Runs step 3 with what the sprite sensed. Says whether it's dying.
        fn step_with(&mut self, senses: Senses) -> bool {
            step(&self.program, &mut self.body, &senses, &self.data)
        }

        /// Runs step 3 with the sprite at rest, as if `age` ticks old. Says whether it's dying.
        fn step_at(&mut self, age: u64) -> bool {
            self.step_with(Senses {
                age,
                ..Senses::default()
            })
        }

        fn step(&mut self) -> bool {
            self.step_at(0)
        }
    }

    /// Asserts that `actual` is `expected`, give or take rounding.
    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "{actual} should be {expected}"
        );
    }

    #[test]
    fn a_reaction_runs_at_its_rate_of_the_most_its_reactants_allow() {
        let mut sprite = Subject::quiet(&[
            r#"Reaction(reactants: [("h0", 2)], products: [("h1", 1), ("h2", 1)], rate: 0.5)"#,
        ]);
        sprite.set("h0", 0.4);
        sprite.step();
        // At most 0.4 / 2 = 0.2, so the extent is 0.5 × 0.2 = 0.1.
        assert_close(sprite.level("h0"), 0.2);
        assert_close(sprite.level("h1"), 0.1);
        assert_close(sprite.level("h2"), 0.1);
    }

    #[test]
    fn the_scarcest_reactant_limits_a_reaction() {
        let mut sprite = Subject::quiet(&[
            r#"Reaction(reactants: [("h0", 1), ("h3", 1)], products: [("h1", 1)], rate: 1.0)"#,
        ]);
        sprite.set("h0", 0.5);
        sprite.set("h3", 0.1);
        sprite.step();
        assert_close(sprite.level("h0"), 0.4);
        assert_close(sprite.level("h3"), 0.0);
        assert_close(sprite.level("h1"), 0.1);
    }

    #[test]
    fn a_catalyst_is_left_exactly_as_it_was() {
        let mut sprite = Subject::quiet(&[
            r#"Reaction(reactants: [("hunger", 1), ("food", 1)], products: [("food", 1)], rate: 0.5)"#,
        ]);
        sprite.set("hunger", 0.6);
        sprite.set("food", 0.3);
        sprite.step();
        assert_close(sprite.level("hunger"), 0.45);
        assert_eq!(sprite.level("food").to_bits(), 0.3f32.to_bits());
    }

    #[test]
    fn a_level_emitter_emits_gain_times_how_far_its_locus_is_past_the_threshold() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Chem("h1"), mode: Level, threshold: 0.2, gain: 0.5, chem: "h0")"#,
            r#"Emitter(locus: Chem("h1"), mode: Level, invert: true, threshold: 0.5, gain: 0.2, chem: "h2")"#,
            r#"Emitter(locus: Chem("h1"), mode: Level, threshold: 0.9, gain: 1.0, chem: "h3")"#,
        ]);
        sprite.set("h1", 0.6);
        sprite.step();
        assert_close(sprite.level("h0"), 0.2); // 0.5 × (0.6 − 0.2)
        assert_close(sprite.level("h2"), 0.0); // inverted, 0.4 isn't past 0.5
        assert_eq!(sprite.level("h3"), 0.0, "not past the threshold");
        sprite.set("h1", 0.2);
        sprite.step();
        assert_close(sprite.level("h2"), 0.06); // 0.2 × ((1 − 0.2) − 0.5)
    }

    #[test]
    fn a_negative_gain_removes_chemical() {
        let mut sprite =
            Subject::quiet(&[r#"Emitter(locus: Chem("h1"), mode: Level, gain: -0.5, chem: "h0")"#]);
        sprite.set("h0", 0.5);
        sprite.set("h1", 0.6);
        sprite.step();
        assert_close(sprite.level("h0"), 0.2);
    }

    #[test]
    fn a_level_emitter_reads_a_pulse_on_the_tick_it_goes_live() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Locus("ate"), mode: Level, gain: 0.4, chem: "h0")"#,
        ]);
        sprite.pulse("ate");
        sprite.step();
        assert_close(sprite.level("h0"), 0.4);
        sprite.step();
        assert_close(sprite.level("h0"), 0.4);
    }

    #[test]
    fn rise_and_fall_emitters_see_the_change_since_the_last_tick() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Chem("h1"), mode: Rise, gain: 1.0, chem: "h0")"#,
            r#"Emitter(locus: Chem("h1"), mode: Fall, gain: 1.0, chem: "h2")"#,
        ]);
        sprite.set("h1", 0.2);
        sprite.step();
        assert_close(sprite.level("h0"), 0.2); // up from the newborn's 0
        sprite.set("h1", 0.5);
        sprite.step();
        assert_close(sprite.level("h0"), 0.5); // up 0.3 more
        sprite.set("h1", 0.1);
        sprite.step();
        assert_close(sprite.level("h0"), 0.5);
        assert_close(sprite.level("h2"), 0.4); // down 0.4
        sprite.step();
        assert_close(sprite.level("h2"), 0.4); // no change, no signal
    }

    #[test]
    fn a_change_emitter_s_threshold_is_a_deadband() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Chem("h1"), mode: Fall, threshold: 0.02, gain: 1.0, chem: "h0")"#,
        ]);
        sprite.set("h1", 0.5);
        sprite.step();
        sprite.set("h1", 0.49);
        sprite.step();
        assert_eq!(
            sprite.level("h0"),
            0.0,
            "a fall of 0.01 is within the deadband"
        );
        sprite.set("h1", 0.39);
        sprite.step();
        assert_close(sprite.level("h0"), 0.08); // a fall of 0.1, past 0.02
    }

    #[test]
    fn a_pulse_that_drops_a_drive_releases_its_fall_reward_in_the_same_tick() {
        // The Fall emitter comes first in the genome, but runs in the second pass.
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Chem("hunger"), mode: Fall, threshold: 0.02, gain: 1.0, chem: "reward")"#,
            r#"Emitter(locus: Locus("ate"), mode: Level, gain: -0.6, chem: "hunger")"#,
        ]);
        sprite.set("hunger", 0.8);
        sprite.step();
        sprite.pulse("ate");
        sprite.step();
        assert_close(sprite.level("hunger"), 0.2);
        assert_close(sprite.level("reward"), 0.58); // a fall of 0.6, past the 0.02 deadband
    }

    #[test]
    fn a_chain_of_change_emitters_in_chain_order_completes_in_one_tick() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Chem("injury"), mode: Rise, gain: 1.0, chem: "pain")"#,
            r#"Emitter(locus: Chem("pain"), mode: Rise, gain: 1.0, chem: "punishment")"#,
        ]);
        sprite.step();
        sprite.set("injury", 0.1);
        sprite.step();
        assert_close(sprite.level("pain"), 0.1);
        assert_close(sprite.level("punishment"), 0.1);
    }

    #[test]
    fn every_level_is_clamped_from_0_to_1() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Chem("h1"), mode: Level, gain: 2.0, chem: "h0")"#,
            r#"Emitter(locus: Chem("h1"), mode: Level, gain: -2.0, chem: "h2")"#,
        ]);
        sprite.set("h1", 0.9);
        sprite.set("h2", 0.5);
        sprite.step();
        assert_eq!(sprite.level("h0"), 1.0);
        assert_eq!(sprite.level("h2"), 0.0);
    }

    #[test]
    fn receptors_move_their_target_from_1_by_how_far_their_chemical_is_past_the_threshold() {
        let mut sprite = Subject::quiet(&[
            r#"Receptor(chem: "h0", threshold: 0.2, gain: 0.5, target: "learning_rate_mod")"#,
            r#"Receptor(chem: "h1", gain: 0.5, target: "learning_rate_mod")"#,
            r#"Receptor(chem: "h2", gain: 10.0, target: "exploration_mod")"#,
        ]);
        sprite.set("h0", 0.6);
        sprite.set("h1", 0.4);
        sprite.set("h2", 1.0);
        sprite.step();
        assert_close(sprite.locus("learning_rate_mod"), 1.4); // 1 + 0.5 × 0.4 + 0.5 × 0.4
        assert_eq!(
            sprite.locus("exploration_mod"),
            4.0,
            "clamped to its range, 0.25–4"
        );
    }

    #[test]
    fn a_receptor_target_no_receptor_writes_rests_at_1() {
        let mut sprite = Subject::quiet(&[]);
        assert_eq!(sprite.locus("learning_rate_mod"), 1.0, "from birth");
        sprite.step();
        assert_eq!(sprite.locus("learning_rate_mod"), 1.0);
        assert_eq!(sprite.locus("exploration_mod"), 1.0);
    }

    /// A sprite with the built-in physiology, at speed 8, sense radius 10 and
    /// a lifespan of 20,000 ticks, whose genome also holds `genes`.
    fn physical(genes: &[&str]) -> Subject {
        let mut all = vec![
            r#"Trait(trait: "speed", value: 8.0)"#,
            r#"Trait(trait: "sense_radius", value: 10.0)"#,
            r#"Trait(trait: "lifespan", value: 20000.0)"#,
        ];
        all.extend_from_slice(genes);
        Subject::new(builtin(), &all)
    }

    #[test]
    fn a_newborn_starts_at_physiology_s_levels_and_its_genes_initial_concentrations() {
        let sprite = physical(&[r#"InitialConcentration(chem: "boredom", value: 0.2)"#]);
        for (chemical, level) in [
            ("energy", 1.0),
            ("hydration", 1.0),
            ("stamina", 1.0),
            ("food", 0.0),
            ("water", 0.0),
            ("injury", 0.0),
            ("boredom", 0.2),
            ("thirst", 0.0),
        ] {
            assert_eq!(sprite.level(chemical), level, "{chemical}");
        }
    }

    #[test]
    fn a_resting_sprite_spends_basal_energy_and_the_cost_of_its_sense_radius() {
        let mut sprite = physical(&[]);
        sprite.step();
        assert_close(sprite.level("energy"), 1.0 - (0.0001 + 10.0 * 0.0000067));
    }

    #[test]
    fn each_step_costs_energy_by_the_square_of_speed_and_drains_stamina() {
        let mut sprite = Subject::new(
            builtin(),
            &[
                r#"Trait(trait: "speed", value: 12.0)"#,
                r#"Trait(trait: "sense_radius", value: 10.0)"#,
            ],
        );
        sprite.step_with(Senses {
            steps: 2,
            ..Senses::default()
        });
        let steps = 2.0 * 0.0002 * (12.0f32 / 8.0) * (12.0 / 8.0);
        assert_close(
            sprite.level("energy"),
            1.0 - (0.0001 + 10.0 * 0.0000067) - steps,
        );
        assert_close(sprite.level("stamina"), 1.0 - 2.0 * 0.002);
    }

    #[test]
    fn stamina_recovers_while_idle_and_faster_while_resting() {
        let mut sprite = physical(&[]);
        sprite.set("stamina", 0.5);
        sprite.step();
        assert_close(sprite.level("stamina"), 0.5005);
        sprite.step_with(Senses {
            resting: true,
            ..Senses::default()
        });
        assert_close(sprite.level("stamina"), 0.5025);
    }

    #[test]
    fn digestion_moves_gut_contents_into_the_body_and_hydration_is_lost() {
        let mut sprite = physical(&[]);
        sprite.set("energy", 0.5);
        sprite.set("food", 0.3);
        sprite.set("hydration", 0.5);
        sprite.set("water", 0.2);
        sprite.step();
        assert_close(sprite.level("food"), 0.299);
        assert_close(
            sprite.level("energy"),
            0.5 + 0.001 - (0.0001 + 10.0 * 0.0000067),
        );
        assert_close(sprite.level("water"), 0.198);
        assert_close(sprite.level("hydration"), 0.5 + 0.002 - 0.00033);
        sprite.set("food", 0.0004);
        sprite.step();
        assert_eq!(
            sprite.level("food"),
            0.0,
            "what's left is less than a tick's digestion"
        );
    }

    #[test]
    fn injury_heals_slowly() {
        let mut sprite = physical(&[]);
        sprite.set("injury", 0.5);
        sprite.step();
        assert_close(sprite.level("injury"), 0.4999);
    }

    #[test]
    fn starvation_dehydration_and_old_age_injure_a_sprite() {
        let net = 0.0011 - 0.0001; // injury added, less healing
        let mut starving = physical(&[]);
        starving.set("energy", 0.0);
        starving.step();
        assert_close(starving.level("injury"), net);

        let mut parched = physical(&[]);
        parched.set("hydration", 0.0);
        parched.step();
        assert_close(parched.level("injury"), net);

        let mut old = physical(&[]);
        old.step_at(20_000);
        assert_eq!(old.level("injury"), 0.0, "at its lifespan, not yet past it");
        old.step_at(20_001);
        assert_close(old.level("injury"), 0.0006 - 0.0001);
    }

    #[test]
    fn body_sensors_report_the_sprite_s_state() {
        let mut sprite = physical(&[]);
        sprite.step_with(Senses {
            age: 5_000,
            nearby_sprites: 0.5,
            steps: 1,
            resting: false,
        });
        assert_eq!(sprite.locus("always"), 1.0);
        assert_eq!(sprite.locus("age"), 0.25, "5,000 of a 20,000-tick lifespan");
        assert_eq!(sprite.locus("nearby_sprites"), 0.5);
        assert_eq!(sprite.locus("moving"), 1.0);
        assert_eq!(sprite.locus("resting"), 0.0);
        sprite.step_with(Senses {
            age: 40_000,
            resting: true,
            ..Senses::default()
        });
        assert_eq!(sprite.locus("age"), 1.0, "capped at 1 past the lifespan");
        assert_eq!(sprite.locus("moving"), 0.0);
        assert_eq!(sprite.locus("resting"), 1.0);
    }

    #[test]
    fn a_sprite_whose_injury_reaches_1_is_dying() {
        let mut sprite = physical(&[]);
        sprite.set("injury", 0.5);
        assert!(!sprite.step());
        sprite.set("injury", 0.9995);
        sprite.set("energy", 0.0);
        assert!(sprite.step(), "0.9995 − 0.0001 + 0.0011 reaches 1");
    }

    mod properties {
        use proptest::prelude::*;
        use rand_chacha::ChaCha8Rng;
        use rand_chacha::rand_core::{Rng, SeedableRng};

        use super::*;
        use crate::registry::ChemicalClass;

        /// Any valid gene of types 1–5, naming any chemical or locus in the built-in pack.
        fn gene() -> impl Strategy<Value = Gene> {
            let data = builtin();
            let chems: Vec<u16> = data.chemicals().iter().map(|c| c.id).collect();
            let loci: Vec<u16> = data.loci().iter().map(|l| l.id).collect();
            let chem = prop::sample::select(chems);
            let locus = prop::sample::select(loci);
            let read = prop_oneof![
                chem.clone().prop_map(LocusRef::Chem),
                locus.clone().prop_map(LocusRef::Locus),
            ];
            let mode = prop_oneof![Just(Mode::Level), Just(Mode::Rise), Just(Mode::Fall)];
            let term =
                (chem.clone(), 1u8..=3).prop_map(|(chem, coefficient)| Term { chem, coefficient });
            prop_oneof![
                (chem.clone(), 1u32..200).prop_map(|(chem, ticks)| Gene::HalfLife { chem, ticks }),
                (
                    prop::collection::vec(term.clone(), 1..=2),
                    prop::collection::vec(term, 0..=2),
                    0.0f32..=1.0,
                )
                    .prop_map(|(reactants, products, rate)| Gene::Reaction {
                        reactants,
                        products,
                        rate
                    }),
                (
                    read,
                    mode,
                    any::<bool>(),
                    0.0f32..=1.0,
                    -2.0f32..2.0,
                    chem.clone()
                )
                    .prop_map(|(locus, mode, invert, threshold, gain, chem)| {
                        Gene::Emitter {
                            locus,
                            mode,
                            invert,
                            threshold,
                            gain,
                            chem,
                        }
                    }),
                (chem.clone(), 0.0f32..=1.0, -5.0f32..5.0, locus).prop_map(
                    |(chem, threshold, gain, target)| Gene::Receptor {
                        chem,
                        threshold,
                        gain,
                        target
                    }
                ),
                (chem, 0.0f32..=1.0)
                    .prop_map(|(chem, value)| Gene::InitialConcentration { chem, value }),
            ]
        }

        /// Trait genes: the one gene type allowed to change physical costs.
        fn traits() -> impl Strategy<Value = Vec<Gene>> {
            (4.0f32..12.0, 6.0f32..14.0, 20_000.0f32..30_000.0).prop_map(
                |(speed, sense, lifespan)| {
                    vec![
                        Gene::Trait {
                            which: Trait::Speed,
                            value: speed,
                        },
                        Gene::Trait {
                            which: Trait::SenseRadius,
                            value: sense,
                        },
                        Gene::Trait {
                            which: Trait::Lifespan,
                            value: lifespan,
                        },
                    ]
                },
            )
        }

        /// A newborn with `genes`.
        fn newborn(genes: Vec<Gene>, data: &DataPack) -> (Program, Body) {
            let program = Program::new(&Genome { genes }, data);
            let body = Body::newborn(&program, data);
            (program, body)
        }

        /// Runs both sprites through 1,000 ticks of the same scripted inputs,
        /// calling `check` after each tick.
        fn run_side_by_side(
            seed: u64,
            sprites: &mut [(Program, Body)],
            data: &DataPack,
            mut check: impl FnMut(&[(Program, Body)]) -> Result<(), TestCaseError>,
        ) -> Result<(), TestCaseError> {
            let slots = data.physiology().slots;
            let pulses: Vec<usize> = data
                .loci()
                .iter()
                .enumerate()
                .filter(|(_, l)| l.kind == LocusKind::Pulse)
                .map(|(slot, _)| slot)
                .collect();
            let mut script = ChaCha8Rng::seed_from_u64(seed);
            for age in 0..1_000 {
                let roll = script.next_u32();
                let senses = Senses {
                    age: age * 30,
                    nearby_sprites: (roll % 5) as f32 / 4.0,
                    steps: roll % 3,
                    resting: roll % 7 == 0,
                };
                // What verbs did last tick: food, water or injury, and a pulse.
                let injection = [(slots.food, 0.3), (slots.water, 0.2), (slots.injury, 0.03)]
                    [(roll / 8 % 3) as usize];
                let pulse = pulses[(roll / 32) as usize % pulses.len()];
                for (program, body) in sprites.iter_mut() {
                    if roll % 11 == 0 {
                        body.chems[injection.0] += injection.1;
                        body.incoming[pulse] = 1.0;
                    }
                    step(program, body, &senses, data);
                }
                check(sprites)?;
            }
            Ok(())
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(64))]

            #[test]
            fn genes_can_never_change_physical_chemicals(
                traits in traits(),
                genes in prop::collection::vec(gene(), 0..24),
                seed: u64,
            ) {
                let data = builtin();
                let physical: Vec<usize> = data
                    .chemicals()
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.class == ChemicalClass::Physical)
                    .map(|(slot, _)| slot)
                    .collect();
                let random = traits.iter().cloned().chain(genes).collect();
                let mut sprites = [newborn(random, &data), newborn(traits, &data)];
                run_side_by_side(seed, &mut sprites, &data, |sprites| {
                    for &slot in &physical {
                        let (random, control) = (sprites[0].1.chems[slot], sprites[1].1.chems[slot]);
                        prop_assert_eq!(random.to_bits(), control.to_bits(), "chemical slot {}", slot);
                    }
                    Ok(())
                })?;
            }

            #[test]
            fn every_level_stays_from_0_to_1(
                traits in traits(),
                genes in prop::collection::vec(gene(), 0..24),
                seed: u64,
            ) {
                let data = builtin();
                let genes = traits.into_iter().chain(genes).collect();
                let mut sprites = [newborn(genes, &data)];
                run_side_by_side(seed, &mut sprites, &data, |sprites| {
                    for &level in &sprites[0].1.chems {
                        prop_assert!((0.0..=1.0).contains(&level), "{}", level);
                    }
                    Ok(())
                })?;
            }
        }
    }

    #[test]
    fn the_cause_of_death_is_the_biggest_recent_source_of_injury() {
        let mut sprite = physical(&[]);
        sprite.set("energy", 0.0);
        for _ in 0..600 {
            sprite.step();
        }
        assert_eq!(sprite.body.cause_of_death(), DeathCause::Starvation);
        // Fed, then parched: 0.3 of injury from thirst against 0.6 from
        // hunger, but the hunger is older and has faded more.
        sprite.set("energy", 1.0);
        sprite.set("hydration", 0.0);
        for _ in 0..100 {
            sprite.step();
        }
        assert_eq!(sprite.body.cause_of_death(), DeathCause::Starvation);
        for _ in 0..200 {
            sprite.step();
        }
        assert_eq!(sprite.body.cause_of_death(), DeathCause::Dehydration);
        assert!(sprite.level("injury") < 1.0, "still alive");
    }

    #[test]
    fn healing_lowers_injury_but_not_the_tallies() {
        let mut sprite = physical(&[]);
        sprite.set("hydration", 0.0);
        sprite.step();
        sprite.set("hydration", 1.0);
        let tallies = sprite.body.tallies;
        sprite.set("injury", 0.5);
        sprite.step();
        let fade = libm::powf(0.5, 1.0 / 350.0);
        assert_eq!(
            sprite.body.tallies[1],
            tallies[1] * fade,
            "only fading lowers it"
        );
    }

    #[test]
    fn a_half_life_halves_a_level_in_that_many_ticks() {
        let mut sprite = Subject::quiet(&[r#"HalfLife(chem: "pain", ticks: 10)"#]);
        sprite.set("pain", 0.8);
        sprite.set("thirst", 0.8);
        for _ in 0..10 {
            sprite.step();
        }
        assert!(
            (sprite.level("pain") - 0.4).abs() < 1e-5,
            "{}",
            sprite.level("pain")
        );
        assert_eq!(sprite.level("thirst"), 0.8, "no half-life, no decay");
    }

    #[test]
    fn a_pulse_is_live_for_exactly_one_step() {
        let mut sprite = Subject::quiet(&[]);
        sprite.pulse("ate");
        assert_eq!(sprite.locus("ate"), 0.0, "not live until the latch");
        sprite.step();
        assert_eq!(sprite.locus("ate"), 1.0);
        sprite.step();
        assert_eq!(sprite.locus("ate"), 0.0);
    }

    /// The program of a genome holding `genes`, with the built-in pack.
    fn program(genes: &[&str]) -> Program {
        let data = builtin();
        let text = format!("(format: 1, genes: [{}])", genes.join(", "));
        Program::new(
            &Genome::from_ron(&text, &data).expect("a valid genome"),
            &data,
        )
    }

    #[test]
    fn traits_are_clamped_to_physiology_s_ranges() {
        let traits = program(&[
            r#"Trait(trait: "speed", value: 30.0)"#,
            r#"Trait(trait: "sense_radius", value: 9.0)"#,
            r#"Trait(trait: "lifespan", value: 100.0)"#,
        ])
        .traits;
        // Physiology's ranges: speed 4–12, sense_radius 6–14, lifespan 20,000–200,000.
        assert_eq!(
            traits,
            Traits {
                speed: 12.0,
                sense_radius: 9.0,
                lifespan: 20_000.0
            }
        );
    }

    #[test]
    fn a_trait_no_gene_sets_takes_the_middle_of_its_range() {
        assert_eq!(
            program(&[]).traits,
            Traits {
                speed: 8.0,
                sense_radius: 10.0,
                lifespan: 110_000.0
            }
        );
    }

    #[test]
    fn only_the_first_trait_gene_counts() {
        let traits = program(&[
            r#"Trait(trait: "speed", value: 5.0)"#,
            r#"Trait(trait: "speed", value: 11.0)"#,
        ])
        .traits;
        assert_eq!(traits.speed, 5.0);
    }
}
