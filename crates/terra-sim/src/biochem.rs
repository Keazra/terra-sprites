//! Biochemistry (design §4): a genome compiled for the chemistry step, and
//! the step itself, a pure function of a body and what it sensed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ageing::{LifeStage, frailty, health_of};
use crate::data::DataPack;
use crate::events::DeathCause;
use crate::expression::{Expression, expressions};
use crate::family::FamilySenses;
use crate::genome::{EmitterMode, Gene, Genome, LocusRef, Term};
use crate::objects::EntityId;
use crate::physiology::halving_factor;
use crate::registry::{ChemId, LocusKind, Trait};

/// A sprite's traits (design §4.8), clamped to physiology's ranges.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Traits {
    /// Move points per tick.
    pub speed: f32,
    /// How far it perceives, in tiles.
    pub sense_radius: f32,
    /// The age it grows old at, in ticks (M2 design §9.3). Genome files
    /// from before M2 call it `lifespan`.
    pub grows_old: f32,
    /// The age it grows up at, in ticks (M2 design §9.3).
    pub grows_up: f32,
}

impl Traits {
    fn set(&mut self, which: Trait, value: f32) {
        match which {
            Trait::Speed => self.speed = value,
            Trait::SenseRadius => self.sense_radius = value,
            Trait::GrowsOld => self.grows_old = value,
            Trait::GrowsUp => self.grows_up = value,
        }
    }

    /// The age at which a sprite with these traits grows old, in ticks (M2
    /// design §9.3, §9.4): `grows_old`, brought `sooner` by a hard life,
    /// but never before it grows up (§9.1).
    pub(crate) fn grows_old_at(&self, sooner: f32) -> f32 {
        (self.grows_old * (1.0 - sooner)).max(self.grows_up)
    }

    /// The life stage of a sprite with these traits at `age` (M2 design
    /// §9.1): young until it grows up, an adult until it grows old, then
    /// old. A hard life brings old age `sooner`, by that share of the age
    /// its genes say (§9.4). One that grows old before it grows up grows
    /// old as it grows up, and is never an adult.
    pub(crate) fn life_stage(&self, age: u64, sooner: f32) -> LifeStage {
        let age = age as f32;
        if age < self.grows_up {
            LifeStage::Young
        } else if age < self.grows_old_at(sooner) {
            LifeStage::Adult
        } else {
            LifeStage::Old
        }
    }
}

/// A genome as the chemistry step runs it: its expressed genes only, with
/// chemicals and loci as indices in the pack's order.
#[derive(Debug, Clone, Default)]
pub(crate) struct Program {
    pub(crate) traits: Traits,
    /// The average health of its genes that weaken (M2 design §9.4): 1
    /// while they're whole.
    pub(crate) vigour: f32,
    /// What each chemical is multiplied by every tick: 1 where no half-life is set.
    decay: Vec<f32>,
    reactions: Vec<Reaction>,
    /// Level emitters, in genome order.
    level_emitters: Vec<Emitter>,
    /// Rise and Fall emitters, in genome order.
    change_emitters: Vec<(Change, Emitter)>,
    /// Each receptor target's index and range, and the receptors that write it.
    targets: Vec<Target>,
    /// Each signal chemical's index and level at birth, where a gene sets one.
    initial: Vec<(usize, f32)>,
    /// How each gene is expressed at the life stage it was compiled for, in
    /// genome order (design §4.3): the same until the stage or the genome
    /// changes, so genes that only weakened are compiled again without
    /// finding it afresh (#210).
    expressed: Vec<Expression>,
}

/// A receptor target and the receptors aimed at it.
#[derive(Debug, Clone)]
struct Target {
    index: usize,
    range: (f32, f32),
    /// Each receptor's chemical index, threshold and gain.
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
    source: Source,
    invert: bool,
    threshold: f32,
    gain: f32,
    chem: usize,
}

impl Emitter {
    /// Adds `gain` times how far `signal` is past the threshold to the
    /// emitter's chemical, kept within 0 to 1.
    fn emit(&self, body: &mut Body, signal: f32) {
        let level = &mut body.chems[self.chem];
        *level = clamped(*level + self.gain * (signal - self.threshold).max(0.0));
    }
}

/// `level` kept within 0 to 1.
fn clamped(level: f32) -> f32 {
    level.clamp(0.0, 1.0)
}

/// Where an emitter reads its value from: a chemical's level or a locus's value, by index.
#[derive(Debug, Clone, Copy)]
enum Source {
    Chem(usize),
    Locus(usize),
}

impl Source {
    fn value(self, body: &Body) -> f32 {
        match self {
            Source::Chem(index) => body.chems[index],
            Source::Locus(index) => body.loci[index],
        }
    }

    /// The value as the previous tick's step 3 left it.
    fn last(self, body: &Body) -> f32 {
        match self {
            Source::Chem(index) => body.last_chems[index],
            Source::Locus(index) => body.last_loci[index],
        }
    }
}

/// A reaction, compiled.
#[derive(Debug, Clone)]
struct Reaction {
    /// Each reactant's index and coefficient.
    reactants: Vec<(usize, f32)>,
    /// How much each chemical the reaction changes gains per unit of extent:
    /// products minus reactants. A catalyst, the same on both sides, isn't
    /// listed, so it's left exactly as it was.
    net: Vec<(usize, f32)>,
    rate: f32,
}

impl Reaction {
    /// A chemical listed twice on one side counts as one term with both
    /// coefficients, so the reaction never uses more than there is.
    fn new(
        reactants: &[Term],
        products: &[Term],
        rate: f32,
        index: impl Fn(ChemId) -> usize,
    ) -> Reaction {
        let mut net: Vec<(usize, f32)> = Vec::new();
        let terms = reactants
            .iter()
            .map(|t| (t, -1.0))
            .chain(products.iter().map(|t| (t, 1.0)));
        for (term, sign) in terms {
            let change = sign * f32::from(term.coefficient);
            match net.iter_mut().find(|(s, _)| *s == index(term.chem)) {
                Some((_, gain)) => *gain += change,
                None => net.push((index(term.chem), change)),
            }
        }
        net.retain(|&(_, gain)| gain != 0.0);
        let mut merged: Vec<(usize, f32)> = Vec::new();
        for term in reactants {
            let coefficient = f32::from(term.coefficient);
            match merged.iter_mut().find(|(i, _)| *i == index(term.chem)) {
                Some((_, total)) => *total += coefficient,
                None => merged.push((index(term.chem), coefficient)),
            }
        }
        Reaction {
            reactants: merged,
            net,
            rate,
        }
    }
}

impl Program {
    /// Compiles `genome`'s genes expressed at `stage`.
    pub(crate) fn new(genome: &Genome, data: &DataPack, stage: LifeStage) -> Program {
        Program::weakened(genome, data, stage, &[])
    }

    /// The same, each gene's effect weakened to its `health` (M2 design
    /// §9.4): a feeling's gain, a receptor's, a reaction's rate, and a speed
    /// or sense trait's reach above the least it can be.
    pub(crate) fn weakened(
        genome: &Genome,
        data: &DataPack,
        stage: LifeStage,
        health: &[f32],
    ) -> Program {
        Program::weakened_as(
            genome,
            data,
            stage,
            &expressions(genome, data, stage),
            health,
        )
    }

    /// The same, from how each gene is expressed at `stage`, in genome
    /// order, which it keeps. Finding that is most of the work, so a sprite
    /// finds it once for its brain and body, and keeps it while its genes
    /// only weaken (#210).
    pub(crate) fn weakened_as(
        genome: &Genome,
        data: &DataPack,
        stage: LifeStage,
        expressed: &[Expression],
        health: &[f32],
    ) -> Program {
        let range = |which: Trait| data.physiology().traits.range(which);
        // A trait no gene sets takes the middle of its range.
        let middle = |which: Trait| {
            let (low, high) = range(which);
            low + (high - low) / 2.0
        };
        let mut traits = Traits {
            speed: middle(Trait::Speed),
            sense_radius: middle(Trait::SenseRadius),
            grows_old: middle(Trait::GrowsOld),
            grows_up: middle(Trait::GrowsUp),
        };
        let chem = |id: ChemId| data.chemical_index(id).expect("a checked gene");
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
                index: data.locus_index(id).expect("a validated receptor target"),
                range,
                receptors: Vec::new(),
            })
            .collect();
        let mut weakening = (0.0, 0);
        for (index, (gene, expression)) in genome.genes.iter().zip(expressed).enumerate() {
            let strength = health_of(health, index);
            if frailty(gene) > 0.0 {
                weakening = (weakening.0 + strength, weakening.1 + 1);
            }
            if *expression != Expression::Expressed {
                continue;
            }
            match *gene {
                Gene::HalfLife { chem: id, ticks } => {
                    decay[chem(id)] = halving_factor(ticks as f32);
                }
                Gene::Reaction {
                    ref reactants,
                    ref products,
                    rate,
                } => reactions.push(Reaction::new(reactants, products, rate * strength, chem)),
                Gene::Emitter {
                    locus,
                    mode,
                    invert,
                    threshold,
                    gain,
                    chem: id,
                } => {
                    let emitter = Emitter {
                        source: match locus {
                            LocusRef::Chem(id) => Source::Chem(chem(id)),
                            LocusRef::Locus(id) => {
                                Source::Locus(data.locus_index(id).expect("a checked gene"))
                            }
                        },
                        invert,
                        threshold,
                        gain: gain * strength,
                        chem: chem(id),
                    };
                    match mode {
                        EmitterMode::Level => level_emitters.push(emitter),
                        EmitterMode::Rise => change_emitters.push((Change::Rise, emitter)),
                        EmitterMode::Fall => change_emitters.push((Change::Fall, emitter)),
                    }
                }
                Gene::Receptor {
                    chem: id,
                    threshold,
                    gain,
                    target,
                } => {
                    let index = data.locus_index(target).expect("a checked gene");
                    let target = targets
                        .iter_mut()
                        .find(|t| t.index == index)
                        .expect("an expressed receptor writes a receptor target");
                    target
                        .receptors
                        .push((chem(id), threshold, gain * strength));
                }
                Gene::InitialConcentration { chem: id, value } => initial.push((chem(id), value)),
                Gene::Trait { which, value } => {
                    let (low, high) = range(which);
                    let value = value.clamp(low, high);
                    let weakened = low + (value - low) * strength;
                    traits.set(which, if frailty(gene) > 0.0 { weakened } else { value });
                }
                // The brain's genes (design §5.7) don't touch the chemistry.
                Gene::BrainParam { .. }
                | Gene::Instinct { .. }
                | Gene::AttentionInstinct { .. } => {}
                Gene::Unknown { .. } | Gene::Unmatched(_) => {
                    unreachable!("an unknown or unmatched gene isn't expressed")
                }
            }
        }
        let (sum, count) = weakening;
        // A genome with no gene that weakens has nothing to wear down, so
        // its body gives out as soon as it's old (M2 design §9.4).
        let vigour = match (count, stage) {
            (0, LifeStage::Old) => data.physiology().ageing.decay.gives_out,
            (0, _) => 1.0,
            _ => sum / count as f32,
        };
        Program {
            traits,
            vigour,
            decay,
            reactions,
            level_emitters,
            change_emitters,
            targets,
            initial,
            expressed: expressed.to_vec(),
        }
    }

    /// How each gene is expressed at the life stage it was compiled for, in
    /// genome order.
    pub(crate) fn expressed(&self) -> &[Expression] {
        &self.expressed
    }
}

/// A sprite's chemistry (design §4.1–§4.2): every chemical's level and every
/// locus's value, as the chemistry step leaves them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Body {
    /// Each chemical's level, in the pack's chemical order.
    pub(crate) chems: Vec<f32>,
    /// Each locus's value, in the pack's locus order: body sensors, live
    /// pulses and receptor targets.
    pub(crate) loci: Vec<f32>,
    /// Pulses written since the last latch, by locus: they go live at the next one.
    pub(crate) incoming: Vec<f32>,
    /// The sprite that caused each live pulse another's verb wrote, by
    /// locus index: the attacker, for `was_hit` (design §4.2).
    pub(crate) sources: BTreeMap<usize, EntityId>,
    /// The sources of the pulses in `incoming`, which go live with them.
    pub(crate) incoming_sources: BTreeMap<usize, EntityId>,
    /// `chems` and `loci` as the previous tick's step 3 left them, for Rise
    /// and Fall emitters.
    last_chems: Vec<f32>,
    last_loci: Vec<f32>,
    /// `chems` as they were before the latest tick began, for the change the
    /// Chem tab shows (design §6.1). Nothing in the sim reads them, so they
    /// aren't hashed. Not to be confused with `last_chems`, which step 3's
    /// Rise and Fall emitters read.
    #[serde(skip)]
    pub(crate) chems_before_tick: Vec<f32>,
    /// The injury each cause added lately, fading (design §4.10). A cause
    /// that has never added any has no entry.
    pub(crate) tallies: BTreeMap<DeathCause, f32>,
    /// How worn a hard life has left the body, from 0 to 1 (M2 design
    /// §9.4). It never heals. A world saved before it had none.
    #[serde(default)]
    pub(crate) wear: f32,
}

impl Body {
    /// Whether the body has a level for each of `data`'s chemicals and a
    /// value for each of its loci, as a body read from a save must (design
    /// §2.8).
    pub(crate) fn fits(&self, data: &DataPack) -> bool {
        let (chems, loci) = (data.chemicals().len(), data.loci().len());
        self.chems.len() == chems
            && self.last_chems.len() == chems
            && self.loci.len() == loci
            && self.last_loci.len() == loci
            && self.incoming.len() == loci
    }

    /// A newborn's body (design §4.7).
    pub(crate) fn newborn(program: &Program, data: &DataPack) -> Body {
        let physiology = data.physiology();
        let (indices, newborn) = (physiology.indices, physiology.newborn);
        let mut chems = vec![0.0; data.chemicals().len()];
        chems[indices.energy] = newborn.energy;
        chems[indices.hydration] = newborn.hydration;
        chems[indices.stamina] = newborn.stamina;
        for &(index, level) in &program.initial {
            chems[index] = level;
        }
        // Receptor targets rest at 1 (design §4.2).
        let mut loci: Vec<f32> = data
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
        // `always` is 1 from birth, so nothing ever sees it rise.
        loci[indices.always] = 1.0;
        Body {
            last_chems: chems.clone(),
            last_loci: loci.clone(),
            chems_before_tick: chems.clone(),
            incoming: vec![0.0; loci.len()],
            sources: BTreeMap::new(),
            incoming_sources: BTreeMap::new(),
            chems,
            loci,
            tallies: BTreeMap::new(),
            wear: 0.0,
        }
    }

    /// Starts the chemical at `index` at `level`, as if it had been there
    /// since the end of the previous tick's step 3, so no Rise or Fall emitter
    /// sees a change, and since before that tick, so the Chem tab shows none.
    pub(crate) fn start_at(&mut self, index: usize, level: f32) {
        self.chems[index] = level;
        self.last_chems[index] = level;
        self.chems_before_tick[index] = level;
    }

    /// What caused most of the body's recent injury (design §4.10). Ties go
    /// to the cause that comes first; with no injury at all, starvation.
    pub(crate) fn cause_of_death(&self) -> DeathCause {
        self.tallies
            .iter()
            .fold((DeathCause::Starvation, 0.0), |most, (&cause, &tally)| {
                if tally > most.1 { (cause, tally) } else { most }
            })
            .0
    }

    /// Writes the pulse at `index` for step 3 to latch (design §4.2), with
    /// `source` as its source: the sprite whose verb caused it, or none, which
    /// clears an earlier one. The latest written counts.
    pub(crate) fn pulse(&mut self, index: usize, source: Option<EntityId>) {
        self.incoming[index] = 1.0;
        match source {
            Some(source) => self.incoming_sources.insert(index, source),
            None => self.incoming_sources.remove(&index),
        };
    }

    /// Raises the chemical at `index` by `amount`, no further than 1, as the
    /// Cursor's touch does (design v21 §4.6).
    pub(crate) fn raise(&mut self, index: usize, amount: f32) {
        self.chems[index] = (self.chems[index] + amount).min(1.0);
    }

    /// Adds `amount` to the chemical at `index`, within 0 to 1. Injury,
    /// at `injury`, is put down to `cause` in full, as physiology's is,
    /// even where the level stops at 1.
    pub(crate) fn inject(&mut self, index: usize, amount: f32, injury: usize, cause: DeathCause) {
        let before = self.chems[index];
        self.chems[index] = (before + amount).clamp(0.0, 1.0);
        if index == injury && amount > 0.0 {
            *self.tallies.entry(cause).or_insert(0.0) += amount;
        }
    }
}

/// What the world tells the chemistry step about a sprite.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Senses {
    /// Ticks since it was born.
    pub(crate) age: u64,
    /// How crowded it is around the sprite, from 0 to 1, as physiology's
    /// `nearby_sprites` counts it (design §4.2).
    pub(crate) nearby_sprites: f32,
    /// The steps it took during the previous tick.
    pub(crate) steps: u32,
    /// Whether it rested during the previous tick.
    pub(crate) resting: bool,
    /// Whether it has no action, standing where it is (M2 design §19.2).
    pub(crate) standing: bool,
    /// Its life stage (M2 design §9.1).
    pub(crate) stage: LifeStage,
    /// The share of the age its genes say that wear and broken genes have
    /// brought its old age sooner by (M2 design §9.4, §12.4).
    pub(crate) sooner: f32,
    /// Whether it's fertile, female, and sees a female adult or a male one
    /// (M2 design §10).
    pub(crate) fertile: bool,
    pub(crate) female: bool,
    pub(crate) female_near: bool,
    pub(crate) male_near: bool,
    /// Whether it sees its partner or has lately heard it call, and whether
    /// it has a partner (M2 design §15.4).
    pub(crate) partner_near: bool,
    pub(crate) paired: bool,
    /// What it senses of its family (M2 design §15.6).
    pub(crate) family: FamilySenses,
}

/// Step 3 for one sprite (design §4.4). Returns whether its injury has
/// reached 1, so it's dying.
///
/// No level ever leaves 0 to 1, even partway through: physiology, each
/// reaction and each emitter clamp what they write (decay can't leave the
/// range). So no gene reads a level outside it, and a Rise or Fall emitter
/// only sees a change that really happened.
pub(crate) fn step(program: &Program, body: &mut Body, senses: &Senses, data: &DataPack) -> bool {
    // (a) The pulse latch: what came in goes live, with its sources, and
    // the buffer empties.
    for (index, locus) in data.loci().iter().enumerate() {
        if locus.kind == LocusKind::Pulse {
            body.loci[index] = std::mem::take(&mut body.incoming[index]);
        }
    }
    body.sources = std::mem::take(&mut body.incoming_sources);
    // (b) Physiology. It works on its own chemicals below 0 or above 1, to
    // tell when energy or hydration has run out, and clamps them when done.
    physiology(program, body, senses, data);
    for level in &mut body.chems {
        *level = clamped(*level);
    }
    // (c) Reactions, in genome order.
    for reaction in &program.reactions {
        let most = reaction
            .reactants
            .iter()
            .map(|&(index, coefficient)| body.chems[index] / coefficient)
            .fold(f32::INFINITY, f32::min);
        let extent = reaction.rate * most;
        for &(index, gain) in &reaction.net {
            body.chems[index] = clamped(body.chems[index] + gain * extent);
        }
    }
    // (d) Half-life decay.
    for (level, factor) in body.chems.iter_mut().zip(&program.decay) {
        *level *= factor;
    }
    // (e1) Level emitters, in genome order, each seeing those before it.
    for emitter in &program.level_emitters {
        let value = emitter.source.value(body);
        let signal = if emitter.invert { 1.0 - value } else { value };
        emitter.emit(body, signal);
    }
    // (e2) Rise and Fall emitters, in genome order, after every Level emitter.
    for &(change, emitter) in &program.change_emitters {
        let delta = emitter.source.value(body) - emitter.source.last(body);
        let signal = match change {
            Change::Rise => delta.max(0.0),
            Change::Fall => (-delta).max(0.0),
        };
        emitter.emit(body, signal);
    }
    // (f) Receptors.
    for target in &program.targets {
        let pull: f32 = target
            .receptors
            .iter()
            .map(|&(chem, threshold, gain)| gain * (body.chems[chem] - threshold).max(0.0))
            .sum();
        body.loci[target.index] = (1.0 + pull).clamp(target.range.0, target.range.1);
    }
    body.last_chems.clone_from(&body.chems);
    body.last_loci.clone_from(&body.loci);
    body.chems[data.physiology().indices.injury] >= 1.0
}

/// Step 3(b), physiology (design §4.4): the body's fixed rules, which read
/// only the traits, the physical chemicals and what the sprite sensed, and
/// write only the physical chemicals and the body sensors.
fn physiology(program: &Program, body: &mut Body, senses: &Senses, data: &DataPack) {
    let physiology = data.physiology();
    let (indices, traits) = (physiology.indices, program.traits);
    let chems = &mut body.chems;
    let steps = senses.steps as f32;
    let pace = traits.speed / 8.0;
    let metabolism = physiology.metabolism;
    // A rest costs less than standing awake (M2 design §19.2).
    let idle = if senses.resting {
        metabolism.resting
    } else {
        1.0
    };
    chems[indices.energy] -= idle
        * (metabolism.basal + metabolism.per_sense_tile * traits.sense_radius)
        + steps * metabolism.per_step * pace * pace;
    // Growing up takes the same energy however long it takes, and a later
    // old age costs a little all life (M2 design §9.3).
    let ageing = physiology.ageing;
    if senses.stage == LifeStage::Young {
        chems[indices.energy] -= ageing.growth / traits.grows_up;
    }
    chems[indices.energy] -= ageing.upkeep * traits.grows_old / 10_000.0;
    // Each thing in the gut moves into the body at its own pace, one that
    // waits only as the body has room for it (M2 design §19.4).
    for (gut, into, content) in [
        (indices.food, indices.energy, physiology.digestion.food),
        (indices.water, indices.hydration, physiology.digestion.water),
    ] {
        let mut digested = chems[gut].min(content.pace);
        if content.waits {
            digested = digested.min((1.0 - chems[into]).max(0.0));
        }
        chems[gut] -= digested;
        chems[into] += digested;
    }
    // A young sprite's thirst, and its dying once it has dried out, are
    // paced to its childhood, so one that grows up sooner needs as much
    // water (M2 design §19.5).
    let paced = match ageing.thirst_childhood {
        Some(childhood) if senses.stage == LifeStage::Young => childhood / traits.grows_up,
        _ => 1.0,
    };
    chems[indices.hydration] -= physiology.hydration_loss * paced;
    chems[indices.stamina] += if senses.steps > 0 {
        -steps * physiology.stamina.per_step
    } else if senses.resting {
        physiology.stamina.resting
    } else {
        physiology.stamina.idle
    };
    // Repair is what fails as genes weaken, so a body heals slower as they
    // do (M2 design §9.4).
    chems[indices.injury] -= physiology.healing * program.vigour;

    let age = senses.age as f32;
    let injury = physiology.injury;
    let harms = [
        (
            DeathCause::Starvation,
            chems[indices.energy] <= 0.0,
            injury.starvation,
        ),
        (
            DeathCause::Dehydration,
            chems[indices.hydration] <= 0.0,
            injury.dehydration * paced,
        ),
        // Once an old sprite's genes have weakened to where its body
        // gives out (M2 design §9.4).
        (
            DeathCause::OldAge,
            senses.stage == LifeStage::Old && program.vigour <= ageing.decay.gives_out,
            injury.old_age,
        ),
    ];
    for tally in body.tallies.values_mut() {
        *tally *= physiology.tally_fade;
    }
    for (cause, harmed, amount) in harms {
        if harmed {
            chems[indices.injury] += amount;
            *body.tallies.entry(cause).or_insert(0.0) += amount;
        }
    }

    // A hard life wears the body for good (M2 design §9.4), which brings
    // old age sooner and speeds the decay of its genes then. It reads the
    // body, not feelings, which genes could switch off.
    let grows_old_at = traits.grows_old_at(senses.sooner);
    let wear = ageing.wear;
    let mut worn = wear.injured * chems[indices.injury].clamp(0.0, 1.0);
    if chems[indices.energy] < wear.low {
        worn += wear.starving;
    }
    if chems[indices.hydration] < wear.low {
        worn += wear.parched;
    }
    body.wear = (body.wear + worn).min(1.0);

    let loci = &mut body.loci;
    loci[indices.always] = 1.0;
    // It reaches 1 as the sprite grows old, sooner for a worn one (§9.3).
    loci[indices.age] = (age / grows_old_at).min(1.0);
    loci[indices.nearby_sprites] = senses.nearby_sprites;
    loci[indices.moving] = if senses.steps > 0 { 1.0 } else { 0.0 };
    loci[indices.resting] = if senses.resting { 1.0 } else { 0.0 };
    if let Some(standing) = indices.standing {
        loci[standing] = if senses.standing { 1.0 } else { 0.0 };
    }
    if let Some(breeding) = indices.breeding {
        let level = |sensed: bool| if sensed { 1.0 } else { 0.0 };
        loci[breeding.fertile] = level(senses.fertile);
        loci[breeding.female] = level(senses.female);
        loci[breeding.female_near] = level(senses.female_near);
        loci[breeding.male_near] = level(senses.male_near);
    }
    if let Some(courtship) = indices.courtship {
        let level = |sensed: bool| if sensed { 1.0 } else { 0.0 };
        loci[courtship.partner_near] = level(senses.partner_near);
        loci[courtship.paired] = level(senses.paired);
        // How healthy it looks, which a mate may choose by (M2 design
        // §15.3): fed, watered and unhurt, and how whole its genes are.
        let fraction = |chem: usize| chems[chem].clamp(0.0, 1.0);
        loci[courtship.condition] = fraction(indices.energy)
            * fraction(indices.hydration)
            * (1.0 - fraction(indices.injury))
            * program.vigour;
    }
    if let Some(families) = indices.families {
        let level = |sensed: bool| if sensed { 1.0 } else { 0.0 };
        let family = senses.family;
        loci[families.egg_away] = level(family.egg_away);
        loci[families.egg_alone] = level(family.egg_alone);
        loci[families.young_away] = level(family.young_away);
        loci[families.parent_away] = level(family.parent_away);
        loci[families.partner_away] = level(family.partner_away);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{builtin, builtin_changing, builtin_with};

    /// The built-in physiology with every rate at 0, so only genes change a body.
    fn quiet() -> DataPack {
        let mut physiology = include_str!("../../../data/physiology.ron").to_string();
        let rates = [
            "basal",
            "per_sense_tile",
            "per_step",
            "pace",
            "hydration_loss",
            "idle",
            "resting",
            "healing",
            "starvation",
            "dehydration",
            "old_age",
        ];
        for rate in rates {
            // Every `rate: <number>` in the file, whatever it's tuned to.
            let key = format!("{rate}: ");
            let mut from = 0;
            while let Some(at) = physiology[from..].find(&key) {
                let start = from + at + key.len();
                let end = start + physiology[start..].find([',', ')']).expect("a value");
                physiology.replace_range(start..end, "0.0");
                from = start;
            }
            assert!(from > 0, "{rate} is in physiology.ron");
        }
        builtin_with("physiology.ron", &physiology)
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
                LifeStage::Adult,
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

        fn chem_index(&self, name: &str) -> usize {
            self.data
                .chemicals()
                .iter()
                .position(|c| c.name == name)
                .expect("a chemical")
        }

        fn locus_index(&self, name: &str) -> usize {
            self.data
                .loci()
                .iter()
                .position(|l| l.name == name)
                .expect("a locus")
        }

        fn level(&self, chemical: &str) -> f32 {
            self.body.chems[self.chem_index(chemical)]
        }

        fn set(&mut self, chemical: &str, level: f32) {
            let index = self.chem_index(chemical);
            self.body.chems[index] = level;
        }

        fn locus(&self, name: &str) -> f32 {
            self.body.loci[self.locus_index(name)]
        }

        fn pulse(&mut self, name: &str) {
            let index = self.locus_index(name);
            self.body.incoming[index] = 1.0;
        }

        /// Runs step 3 with what the sprite sensed. Says whether it's dying.
        fn step_with(&mut self, senses: Senses) -> bool {
            step(&self.program, &mut self.body, &senses, &self.data)
        }

        /// Runs step 3 with the sprite at rest, an adult `age` ticks old. Says whether it's dying.
        fn step_at(&mut self, age: u64) -> bool {
            self.step_with(Senses { age, ..adult() })
        }

        fn step(&mut self) -> bool {
            self.step_at(0)
        }

        /// Runs step 3 with the sprite old and at rest. Says whether it's dying.
        fn step_old(&mut self) -> bool {
            self.step_with(Senses {
                age: 30_000,
                stage: LifeStage::Old,
                ..Senses::default()
            })
        }
    }

    /// What an adult at rest senses.
    fn adult() -> Senses {
        Senses {
            stage: LifeStage::Adult,
            ..Senses::default()
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

    /// A sprite with the built-in physiology, at speed 8 and sense radius
    /// 10, growing up at 2,000 ticks and old at 20,000, whose genome also
    /// holds `genes`.
    fn physical(genes: &[&str]) -> Subject {
        let mut all = vec![
            r#"Trait(trait: "speed", value: 8.0)"#,
            r#"Trait(trait: "sense_radius", value: 10.0)"#,
            r#"Trait(trait: "grows_up", value: 2000.0)"#,
            r#"Trait(trait: "grows_old", value: 20000.0)"#,
        ];
        all.extend_from_slice(genes);
        Subject::new(builtin(), &all)
    }

    #[test]
    fn of_two_pulses_in_a_tick_the_last_written_counts_and_one_with_no_source_clears_it() {
        // Design §3.5.2: a pulse on a target records the actor as its
        // source; one a sprite's own verb writes has none.
        let mut sprite = physical(&[]);
        let was_hit = sprite.locus_index("was_hit");
        sprite.body.pulse(was_hit, Some(EntityId(4)));
        sprite.body.pulse(was_hit, Some(EntityId(9)));
        sprite.step();
        assert_eq!(sprite.body.sources.get(&was_hit), Some(&EntityId(9)));
        sprite.body.pulse(was_hit, Some(EntityId(4)));
        sprite.body.pulse(was_hit, None);
        sprite.step();
        assert_eq!(sprite.locus("was_hit"), 1.0, "the pulse is live");
        assert_eq!(sprite.body.sources.get(&was_hit), None, "with no source");
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
                r#"Trait(trait: "grows_old", value: 20000.0)"#,
            ],
        );
        sprite.step_with(Senses {
            steps: 2,
            ..adult()
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
            ..adult()
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
        old.program.vigour = 0.51;
        old.step_old();
        assert_eq!(old.level("injury"), 0.0, "its body hasn't given out yet");
        old.program.vigour = 0.5;
        old.step_old();
        // Genes at half their health heal at half the rate.
        assert_close(old.level("injury"), 0.0006 - 0.0001 * 0.5);

        let mut adult = physical(&[]);
        adult.program.vigour = 0.5;
        adult.step();
        assert_eq!(adult.level("injury"), 0.0, "only an old body gives out");
    }

    #[test]
    fn body_sensors_report_the_sprite_s_state() {
        let mut sprite = physical(&[]);
        sprite.step_with(Senses {
            age: 5_000,
            nearby_sprites: 0.5,
            steps: 1,
            resting: false,
            stage: LifeStage::Adult,
            ..Senses::default()
        });
        assert_eq!(sprite.locus("always"), 1.0);
        assert_eq!(
            sprite.locus("age"),
            0.25,
            "5,000 of the 20,000 ticks to old age"
        );
        assert_eq!(sprite.locus("nearby_sprites"), 0.5);
        assert_eq!(sprite.locus("moving"), 1.0);
        assert_eq!(sprite.locus("resting"), 0.0);
        sprite.step_with(Senses {
            age: 40_000,
            resting: true,
            ..adult()
        });
        assert_eq!(sprite.locus("age"), 1.0, "capped at 1 once old");
        assert_eq!(sprite.locus("moving"), 0.0);
        assert_eq!(sprite.locus("resting"), 1.0);
    }

    #[test]
    fn courtship_sensors_report_a_partner_and_how_healthy_the_sprite_looks() {
        // M2 design §15.3, §15.4: `condition` is energy × hydration × (1 −
        // injury) × the genes' average health.
        let mut sprite = Subject::quiet(&[]);
        sprite.set("energy", 0.8);
        sprite.set("hydration", 0.5);
        sprite.set("injury", 0.5);
        sprite.program.vigour = 0.5;
        sprite.step_with(Senses {
            paired: true,
            partner_near: true,
            ..adult()
        });
        assert_eq!(sprite.locus("paired"), 1.0);
        assert_eq!(sprite.locus("partner_near"), 1.0);
        assert_close(sprite.locus("condition"), 0.8 * 0.5 * 0.5 * 0.5);
        sprite.step_with(adult());
        assert_eq!(sprite.locus("paired"), 0.0);
        assert_eq!(sprite.locus("partner_near"), 0.0);
    }

    #[test]
    fn family_sensors_report_family_out_of_sight() {
        // M2 design §15.6.
        let mut sprite = Subject::quiet(&[]);
        sprite.step_with(Senses {
            family: FamilySenses {
                egg_away: true,
                egg_alone: true,
                young_away: true,
                parent_away: true,
                partner_away: true,
            },
            ..adult()
        });
        for sensor in [
            "egg_away",
            "egg_alone",
            "young_away",
            "parent_away",
            "partner_away",
        ] {
            assert_eq!(sprite.locus(sensor), 1.0, "{sensor}");
        }
        sprite.step_with(Senses {
            family: FamilySenses {
                egg_away: true,
                ..FamilySenses::default()
            },
            ..adult()
        });
        assert_eq!(sprite.locus("egg_away"), 1.0);
        for sensor in ["egg_alone", "young_away", "parent_away", "partner_away"] {
            assert_eq!(sprite.locus(sensor), 0.0, "{sensor}");
        }
    }

    #[test]
    fn a_worn_sprite_s_age_sensor_reaches_1_as_it_grows_old() {
        // M2 design §9.3, §9.4: the most worn sprite grows old at (1 −
        // `sooner`) of its `grows_old`, here .67 of 20,000 ticks.
        let mut sprite = physical(&[]);
        sprite.step_with(Senses {
            age: 6_700,
            sooner: 0.33,
            ..adult()
        });
        assert_close(sprite.locus("age"), 0.5);
    }

    #[test]
    fn a_hard_life_brings_old_age_sooner() {
        // M2 design §9.4: `sooner` is a share of the age its genes say.
        let traits = physical(&[]).program.traits;
        assert_eq!(traits.life_stage(19_999, 0.0), LifeStage::Adult);
        assert_eq!(traits.life_stage(20_000, 0.0), LifeStage::Old);
        assert_eq!(traits.life_stage(14_999, 0.25), LifeStage::Adult);
        assert_eq!(traits.life_stage(15_000, 0.25), LifeStage::Old);
        assert_eq!(
            traits.life_stage(1_000, 0.99),
            LifeStage::Young,
            "never before it grows up"
        );
        assert_eq!(
            traits.grows_old_at(0.99),
            2_000.0,
            "never before it grows up"
        );
    }

    #[test]
    fn growing_up_costs_energy_spread_over_childhood() {
        // Growing up at 2,000 ticks, it spends 0.3 / 2,000 each young tick
        // on growing (M2 design §9.3).
        let mut young = physical(&[]);
        young.step_with(Senses::default());
        let mut adult = physical(&[]);
        adult.step();
        assert_close(adult.level("energy") - young.level("energy"), 0.3 / 2000.0);
    }

    #[test]
    fn a_later_old_age_costs_a_little_energy_every_tick() {
        // The built-in pack's upkeep is 0 until genes evolve (M2 design
        // §9.3), so this sets one.
        let data = builtin_changing("physiology.ron", &[("upkeep: 0.0,", "upkeep: 0.00001,")]);
        let early = r#"Trait(trait: "grows_old", value: 20000.0)"#;
        let late = r#"Trait(trait: "grows_old", value: 50000.0)"#;
        let with = |grows_old| {
            [
                r#"Trait(trait: "speed", value: 8.0)"#,
                r#"Trait(trait: "sense_radius", value: 10.0)"#,
                r#"Trait(trait: "grows_up", value: 2000.0)"#,
                grows_old,
            ]
        };
        let mut early = Subject::new(data.clone(), &with(early));
        let mut late = Subject::new(data, &with(late));
        early.step();
        late.step();
        // 0.00001 a tick for every 10,000 ticks: 0.00002 against 0.00005.
        assert_close(early.level("energy") - late.level("energy"), 0.00003);
    }

    #[test]
    fn a_hard_life_wears_the_body_for_good() {
        let mut sprite = physical(&[]);
        sprite.step();
        assert_eq!(sprite.body.wear, 0.0, "a full, unhurt body doesn't wear");
        sprite.set("energy", 0.2);
        sprite.step();
        assert_close(sprite.body.wear, 0.0002);
        sprite.set("energy", 1.0);
        sprite.set("hydration", 0.2);
        sprite.step();
        assert_close(sprite.body.wear, 0.0004);
        sprite.set("hydration", 1.0);
        sprite.set("injury", 0.5001);
        sprite.step();
        // 0.001 for each unit of injury, after this tick's healing.
        assert_close(sprite.body.wear, 0.0004 + 0.0005);
        sprite.set("injury", 0.0);
        for _ in 0..100 {
            sprite.step();
        }
        assert_close(sprite.body.wear, 0.0009);
    }

    #[test]
    fn a_weakened_gene_has_less_effect() {
        // M2 design §9.4: a reaction runs, a feeling rises and speed reaches
        // above its least in proportion to the gene's health; a setting,
        // such as when it grows old, doesn't weaken.
        let data = quiet();
        let text = r#"(format: 1, genes: [
            Reaction(reactants: [("h0", 1)], products: [("h1", 1)], rate: 0.5),
            Emitter(locus: Chem("h3"), mode: Level, threshold: 0.0, gain: 0.2, chem: "h2"),
            Trait(trait: "speed", value: 8.0),
            Trait(trait: "grows_old", value: 20000.0),
        ])"#;
        let genome = Genome::from_ron(text, &data).expect("a valid genome");
        let health = [0.5, 0.25, 0.5, 0.0];
        let program = Program::weakened(&genome, &data, LifeStage::Adult, &health);
        assert_close(program.traits.speed, 4.0 + (8.0 - 4.0) * 0.5);
        assert_eq!(program.traits.grows_old, 20_000.0);
        // The average of the three genes that weaken.
        assert_close(program.vigour, (0.5 + 0.25 + 0.5) / 3.0);
        let mut sprite = Subject {
            body: Body::newborn(&program, &data),
            program,
            data,
        };
        sprite.set("h0", 0.4);
        sprite.set("h3", 0.4);
        sprite.step();
        // The reaction at a rate of 0.25 moves 0.1, and the feeling at a
        // gain of 0.05 emits 0.02.
        assert_close(sprite.level("h1"), 0.1);
        assert_close(sprite.level("h2"), 0.02);
    }

    #[test]
    fn a_weakened_receptor_moves_its_target_less() {
        // M2 design §9.4: a receptor's gain, too, scales with its gene's
        // health.
        let data = quiet();
        let text = r#"(format: 1, genes: [
            Receptor(chem: "h0", gain: 0.5, target: "learning_rate_mod"),
        ])"#;
        let genome = Genome::from_ron(text, &data).expect("a valid genome");
        let program = Program::weakened(&genome, &data, LifeStage::Adult, &[0.5]);
        let mut sprite = Subject {
            body: Body::newborn(&program, &data),
            program,
            data,
        };
        sprite.set("h0", 0.8);
        sprite.step();
        // 1 + 0.8 × a gain of 0.25.
        assert_close(sprite.locus("learning_rate_mod"), 1.2);
    }

    #[test]
    fn a_body_with_nothing_to_weaken_gives_out_when_old() {
        // M2 design §9.4: settings alone never weaken, so an old sprite
        // with no other genes would otherwise live for ever.
        let data = quiet();
        let text = r#"(format: 1, genes: [Trait(trait: "grows_old", value: 20000.0)])"#;
        let genome = Genome::from_ron(text, &data).expect("a valid genome");
        let adult = Program::weakened(&genome, &data, LifeStage::Adult, &[1.0]);
        assert_eq!(adult.vigour, 1.0);
        let old = Program::weakened(&genome, &data, LifeStage::Old, &[1.0]);
        assert_eq!(old.vigour, data.physiology().ageing.decay.gives_out);
    }

    #[test]
    fn whole_genes_are_as_strong_as_ever() {
        let data = builtin();
        let genome = data.starter().clone();
        let whole = vec![1.0; genome.genes.len()];
        let weakened = Program::weakened(&genome, &data, LifeStage::Adult, &whole);
        let born = Program::new(&genome, &data, LifeStage::Adult);
        assert_eq!(weakened.traits, born.traits);
        assert_eq!(weakened.vigour, 1.0);
    }

    #[test]
    fn weakened_genes_heal_slower_but_recover_stamina_as_ever() {
        let mut sprite = physical(&[]);
        sprite.program.vigour = 0.5;
        sprite.set("injury", 0.5);
        sprite.set("stamina", 0.5);
        sprite.step();
        assert_close(sprite.level("injury"), 0.5 - 0.00005);
        assert_close(sprite.level("stamina"), 0.5 + 0.0005);
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
        use crate::registry::{ChemicalClass, LocusId};

        /// Any valid gene of types 1–5, naming any chemical or locus in the built-in pack.
        fn gene() -> impl Strategy<Value = Gene> {
            let data = builtin();
            let chems: Vec<ChemId> = data.chemicals().iter().map(|c| c.id).collect();
            let loci: Vec<LocusId> = data.loci().iter().map(|l| l.id).collect();
            let chem = prop::sample::select(chems);
            let locus = prop::sample::select(loci);
            let read = prop_oneof![
                chem.clone().prop_map(LocusRef::Chem),
                locus.clone().prop_map(LocusRef::Locus),
            ];
            let mode = prop_oneof![
                Just(EmitterMode::Level),
                Just(EmitterMode::Rise),
                Just(EmitterMode::Fall)
            ];
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
                            // Only a Level emitter can be inverted.
                            invert: invert && mode == EmitterMode::Level,
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
                |(speed, sense, grows_old)| {
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
                            which: Trait::GrowsOld,
                            value: grows_old,
                        },
                    ]
                },
            )
        }

        /// A newborn with `genes`.
        fn newborn(genes: Vec<Gene>, data: &DataPack) -> (Program, Body) {
            let program =
                Program::new(&Genome::switched(genes, Vec::new()), data, LifeStage::Adult);
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
            let indices = data.physiology().indices;
            let pulses: Vec<usize> = data
                .loci()
                .iter()
                .enumerate()
                .filter(|(_, l)| l.kind == LocusKind::Pulse)
                .map(|(index, _)| index)
                .collect();
            let mut script = ChaCha8Rng::seed_from_u64(seed);
            for age in 0..1_000 {
                let roll = script.next_u32();
                let senses = Senses {
                    age: age * 30,
                    nearby_sprites: (roll % 5) as f32 / 4.0,
                    steps: roll % 3,
                    resting: roll % 7 == 0,
                    stage: LifeStage::Adult,
                    ..Senses::default()
                };
                // What verbs did last tick: food, water or injury, and a pulse.
                let injection = [
                    (indices.food, 0.3),
                    (indices.water, 0.2),
                    (indices.injury, 0.03),
                ][(roll / 8 % 3) as usize];
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
                    .map(|(index, _)| index)
                    .collect();
                let random = traits.iter().cloned().chain(genes).collect();
                let mut sprites = [newborn(random, &data), newborn(traits, &data)];
                run_side_by_side(seed, &mut sprites, &data, |sprites| {
                    for &index in &physical {
                        let (random, control) = (sprites[0].1.chems[index], sprites[1].1.chems[index]);
                        prop_assert_eq!(random.to_bits(), control.to_bits(), "chemical index {}", index);
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
    fn injected_injury_counts_in_full_towards_its_cause_even_past_1() {
        let mut sprite = physical(&[]);
        let injury = sprite.chem_index("injury");
        sprite.set("injury", 0.98);
        let thornbush = DeathCause::HurtBy(3);
        sprite.body.inject(injury, 0.05, injury, thornbush);
        assert_eq!(sprite.level("injury"), 1.0);
        assert_eq!(sprite.body.tallies[&thornbush], 0.05);
    }

    #[test]
    fn healing_lowers_injury_but_not_the_tallies() {
        let mut sprite = physical(&[]);
        sprite.set("hydration", 0.0);
        sprite.step();
        sprite.set("hydration", 1.0);
        let tallies = sprite.body.tallies.clone();
        sprite.set("injury", 0.5);
        sprite.step();
        let fade = libm::powf(0.5, 1.0 / 350.0);
        assert_eq!(
            sprite.body.tallies[&DeathCause::Dehydration],
            tallies[&DeathCause::Dehydration] * fade,
            "only fading lowers it"
        );
    }

    /// A newborn made from the built-in starter genome, unvaried.
    fn starter() -> Subject {
        let data = builtin();
        let program = Program::new(data.starter(), &data, LifeStage::Adult);
        let body = Body::newborn(&program, &data);
        Subject {
            data,
            program,
            body,
        }
    }

    #[test]
    fn a_sprite_at_rest_with_nothing_happening_gets_no_reward() {
        let mut sprite = starter();
        for tick in 0..200 {
            sprite.step_at(tick);
            assert_eq!(sprite.level("reward"), 0.0, "tick {tick}");
        }
    }

    #[test]
    fn eating_drops_a_hungry_starter_s_hunger_at_once_and_makes_no_reward() {
        // Design v16 §4.5: the brain reads hunger's fall as relief itself, so
        // the starter genome turns no fall into reward.
        let mut hungry = starter();
        hungry.set("hunger", 0.8);
        hungry.step();
        let before = hungry.level("hunger");
        hungry.pulse("ate");
        hungry.step();
        assert!((before - hungry.level("hunger") - 0.5).abs() < 0.01);
        assert_eq!(hungry.level("reward"), 0.0);
    }

    #[test]
    fn a_starter_that_loses_its_partner_grieves_and_is_held_back_from_breeding() {
        // M2 design §15.4: grief holds arousal down while it lasts, though
        // the sprite is fertile.
        let (mut grieving, mut free) = (starter(), starter());
        for sprite in [&mut grieving, &mut free] {
            sprite.set("arousal", 0.5);
        }
        grieving.pulse("lost_partner");
        for _ in 0..100 {
            for sprite in [&mut grieving, &mut free] {
                sprite.step_with(Senses {
                    fertile: true,
                    ..adult()
                });
            }
        }
        assert!(grieving.level("grief") > 0.3, "it grieves");
        assert_eq!(grieving.level("arousal"), 0.0);
        assert!(free.level("arousal") > 0.5, "fertile, it's aroused");
    }

    #[test]
    fn a_reaction_never_runs_backwards_when_a_level_runs_out() {
        let mut sprite = Subject::new(
            builtin(),
            &[
                r#"Reaction(reactants: [("hunger", 1), ("energy", 1)], products: [("energy", 1)], rate: 0.5)"#,
            ],
        );
        sprite.set("energy", 0.0);
        sprite.set("hunger", 0.4);
        sprite.step();
        // Basal metabolism takes energy below 0 before the reaction runs,
        // but no level is ever read outside 0 to 1.
        assert_eq!(sprite.level("hunger"), 0.4);
    }

    #[test]
    fn always_is_1_from_birth_so_nothing_sees_it_rise() {
        let mut sprite = Subject::quiet(&[
            r#"Emitter(locus: Locus("always"), mode: Rise, gain: 1.0, chem: "h0")"#,
        ]);
        assert_eq!(sprite.locus("always"), 1.0);
        sprite.step();
        assert_eq!(sprite.level("h0"), 0.0);
    }

    #[test]
    fn a_reactant_listed_twice_counts_as_one_with_both_coefficients() {
        let mut sprite = Subject::quiet(&[
            r#"Reaction(reactants: [("h0", 1), ("h0", 1)], products: [("h1", 1)], rate: 1.0)"#,
        ]);
        sprite.set("h0", 0.4);
        sprite.step();
        // As 2 h0 → h1: at most 0.4 / 2, so it never uses more h0 than there is.
        assert_close(sprite.level("h1"), 0.2);
        assert_close(sprite.level("h0"), 0.0);
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
            LifeStage::Adult,
        )
    }

    #[test]
    fn traits_are_clamped_to_physiology_s_ranges() {
        let traits = program(&[
            r#"Trait(trait: "speed", value: 30.0)"#,
            r#"Trait(trait: "sense_radius", value: 9.0)"#,
            r#"Trait(trait: "grows_old", value: 100.0)"#,
            r#"Trait(trait: "grows_up", value: 99000.0)"#,
        ])
        .traits;
        // Physiology's ranges: speed 4–12, sense_radius 6–14, grows_old
        // 6,000–60,000, grows_up 1,500–13,500.
        assert_eq!(
            traits,
            Traits {
                speed: 12.0,
                sense_radius: 9.0,
                grows_old: 6_000.0,
                grows_up: 13_500.0,
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
                grows_old: 33_000.0,
                grows_up: 7_500.0,
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
