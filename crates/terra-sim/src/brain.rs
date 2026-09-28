//! The brain (design §5): what a sprite attends to (5a) and what it chooses
//! to do about it (5b), from its inputs through its concepts.

use std::collections::{BTreeMap, VecDeque};

use rand_chacha::ChaCha8Rng;
use serde::Serialize;

use crate::biochem::Body;
use crate::brain_io::Source;
use crate::data::DataPack;
use crate::expression::{Expression, expressions};
use crate::genome::{Gene, Genome, LocusRef};
use crate::learning::{
    Experience, Links, Signals, TRACE_CAP, Touch, TraceEntry, kind, still_counts, weight,
};
use crate::perception::Target;
use crate::random::unit;
use crate::registry::{BrainParam, Category, Verb};

/// The verbs the brain scores, in verb ID order: every verb but the reserved
/// ones. A verb's place here is its column in the decision links.
pub(crate) const VERBS: [Verb; 8] = [
    Verb::Approach,
    Verb::Eat,
    Verb::Drink,
    Verb::Hit,
    Verb::Play,
    Verb::Retreat,
    Verb::Rest,
    Verb::Wander,
];

/// Where `verb` is in `VERBS`.
fn column(verb: Verb) -> usize {
    VERBS
        .iter()
        .position(|&v| v == verb)
        .expect("a verb the brain scores")
}

/// Where `category` is in `Category::ALL`, its column in the attention links.
fn category_column(category: Category) -> usize {
    Category::ALL
        .iter()
        .position(|&c| c == category)
        .expect("every category is in ALL")
}

/// The brain's parameters (design §5.7): each from the first `BrainParam`
/// gene that sets it, clamped to physiology's range, or physiology's
/// default where no gene does.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct BrainParams {
    /// In `BrainParam::ALL` order.
    values: [f32; BrainParam::ALL.len()],
}

impl BrainParams {
    pub(crate) fn express(genome: &Genome, data: &DataPack) -> BrainParams {
        let ranges = &data.physiology().brain;
        let mut values = BrainParam::ALL.map(|param| ranges.of(param).default);
        for (gene, expression) in genome.genes.iter().zip(expressions(genome, data)) {
            if let (&Gene::BrainParam { param, value }, Expression::Expressed) = (gene, expression)
            {
                let (low, high) = ranges.of(param).range;
                values[index(param)] = value.clamp(low, high);
            }
        }
        BrainParams { values }
    }

    /// The value of `param`.
    pub(crate) fn get(&self, param: BrainParam) -> f32 {
        self.values[index(param)]
    }
}

/// Where `param` is in `BrainParam::ALL`.
fn index(param: BrainParam) -> usize {
    BrainParam::ALL
        .iter()
        .position(|&p| p == param)
        .expect("every parameter is in ALL")
}

/// A concept (design §5.4): the inputs it combines, by their place in the
/// pack's brain inputs, each maybe negated, in that order. It's identified
/// by this signature, never by its place in the brain.
pub(crate) type Signature = Vec<(usize, bool)>;

/// What a sprite knows about the thing it attends to, for the Target inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Aim {
    pub(crate) category: Category,
    /// Its path cost, normalized (design §5.2): 0 on it, 1 at the edge of sight.
    pub(crate) distance: f32,
    /// Whether the sprite stands on one of its goal tiles.
    pub(crate) adjacent: bool,
}

/// A sprite's brain: its parameters, concepts and links (design §5.1).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Brain {
    pub(crate) params: BrainParams,
    /// Singletons, one per input in input order, then the innate
    /// conjunctions in genome order.
    pub(crate) concepts: Vec<Signature>,
    /// Decision instincts W: each concept's link to each verb, in `VERBS` order.
    decision: Links<{ VERBS.len() }>,
    /// Attention instincts A: each State input's link to each category, in
    /// `Category::ALL` order, by the input's place in the pack.
    attention: Links<{ Category::ALL.len() }>,
    /// The category attention is on, if any.
    pub(crate) attended: Option<Category>,
    /// What the brain did at the latest step 5 (design §5.5), for the trace
    /// entry learning commits and for the Brain tab.
    pub(crate) snapshot: Option<Snapshot>,
    /// The reward less the punishment step 4 last used up: `last_r` (design §5.6).
    pub(crate) felt: f32,
    /// What it felt and chose on its recent ticks, oldest first (design §5.6).
    pub(crate) trace: VecDeque<TraceEntry>,
    /// What it has learned (design §5.6).
    pub(crate) experience: Experience,
    /// What it last tried a verb on, and when (design §5.6).
    pub(crate) touched: Option<Touch>,
    /// Each need's singleton, in the pack's needs order, to find a motive.
    need_rows: Vec<usize>,
}

/// What the brain saw and did at a step 5.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Snapshot {
    /// The tick of that step 5.
    pub(crate) tick: u64,
    /// Every input's value, in the pack's input order.
    pub(crate) inputs: Vec<f32>,
    /// Every concept's activation, in the brain's concept order.
    pub(crate) activations: Vec<f32>,
    /// Each candidate category's attention score.
    pub(crate) attention: BTreeMap<Category, f32>,
    pub(crate) attended: Option<Category>,
    /// The one thing attention is on: a running action's target, or else the
    /// attended category's candidate.
    pub(crate) target: Option<Target>,
    /// Each verb's score, in `VERBS` order.
    pub(crate) scores: [f32; VERBS.len()],
    /// The verb it's doing, if any.
    pub(crate) verb: Option<Verb>,
}

/// Something a sprite has learned, named (design §5.6, §5.9). Things are
/// named as brain inputs name categories (`berry_bush`), needs as brain
/// inputs name them (`hunger`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Learned {
    /// What a kind of thing is worth: for a need, or in general (`None`).
    Worth { thing: String, need: Option<String> },
    /// How bad a kind of thing is.
    Bad { thing: String },
    /// A habit: doing a verb to a kind of thing.
    Habit { thing: String, verb: Verb },
    /// The worth of new things.
    NewThings,
}

/// One thing a sprite has learned, and how far it has got from nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub learned: Learned,
    pub value: f32,
}

/// How many learned things the memory lists (design §5.9).
const MEMORY_SIZE: usize = 5;

/// What a brain did at its latest step 5, explained (design §5.9): what it
/// could attend to, and why it's doing what it's doing.
#[derive(Debug, Clone, PartialEq)]
pub struct Explanation<'a> {
    /// Each candidate category's attention score, as `(category, score)`,
    /// highest first, ties to the lower category.
    pub attention: Vec<(&'static str, f32)>,
    /// The category attention is on, if any.
    pub attended: Option<&'static str>,
    /// The verb it chose or kept doing, with its score.
    pub decision: Option<(Verb, f32)>,
    /// The concepts adding to or taking from that verb's score, largest
    /// first whatever the sign, ties in concept order. A concept adding
    /// nothing is left out.
    pub contributions: Vec<Contribution<'a>>,
}

/// How much one concept adds to a verb's score (design §5.9).
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution<'a> {
    /// The concept's inputs by name, each with whether it's negated.
    pub inputs: Vec<(&'a str, bool)>,
    /// Its activation times its link to the verb.
    pub amount: f32,
}

impl Brain {
    /// A newborn's brain from its genome (design §5.4, §5.7): a singleton per
    /// input, a conjunction per expressed multi-input `Instinct` signature,
    /// and each link at its instinct's weight, or 0.
    pub(crate) fn new(genome: &Genome, data: &DataPack) -> Brain {
        let inputs = data.brain_inputs_in_order();
        let place = |id| {
            inputs
                .iter()
                .position(|input| input.id == id)
                .expect("a checked gene")
        };
        let mut concepts: Vec<Signature> = (0..inputs.len()).map(|i| vec![(i, false)]).collect();
        let mut decision = vec![[0.0; VERBS.len()]; inputs.len()];
        let mut attention = vec![[0.0; Category::ALL.len()]; inputs.len()];
        for (gene, expression) in genome.genes.iter().zip(expressions(genome, data)) {
            if expression != Expression::Expressed {
                continue;
            }
            match *gene {
                Gene::Instinct {
                    ref inputs,
                    verb,
                    weight,
                } => {
                    let mut signature: Signature = inputs
                        .iter()
                        .map(|&(id, negated)| (place(id), negated))
                        .collect();
                    signature.sort();
                    let concept = match concepts.iter().position(|c| *c == signature) {
                        Some(concept) => concept,
                        None => {
                            concepts.push(signature);
                            decision.push([0.0; VERBS.len()]);
                            concepts.len() - 1
                        }
                    };
                    decision[concept][column(verb)] = weight;
                }
                Gene::AttentionInstinct {
                    input,
                    category,
                    weight,
                } => attention[place(input)][category_column(category)] = weight,
                _ => {}
            }
        }
        Brain {
            params: BrainParams::express(genome, data),
            concepts,
            decision: Links::new(decision),
            attention: Links::new(attention),
            attended: None,
            snapshot: None,
            felt: 0.0,
            trace: VecDeque::new(),
            experience: Experience::new(data.need_places().len()),
            touched: None,
            need_rows: data.need_places().to_vec(),
        }
    }

    /// Step 4 at `tick` (design §2.4, §5.6), from `signals`, every rate
    /// scaled by `learning_rate_mod`. Instinct links never change. Each
    /// need's relief teaches what the thing touched is worth for it. Returns
    /// the lessons it learned.
    pub(crate) fn learn(
        &mut self,
        tick: u64,
        signals: &Signals,
        learning_rate_mod: f32,
        data: &DataPack,
    ) -> Vec<(Learned, bool)> {
        let physiology = data.physiology();
        let relief: Vec<f32> = match &self.experience.needs_before {
            Some(before) => before
                .iter()
                .zip(&signals.needs)
                .map(|(&was, &now)| was - now)
                .map(|fall| {
                    if fall >= physiology.relief_deadband {
                        fall
                    } else {
                        0.0
                    }
                })
                .collect(),
            None => vec![0.0; signals.needs.len()],
        };
        self.experience.needs_before = Some(signals.needs.clone());
        let touched = self
            .touched
            .filter(|t| tick - t.tick <= physiology.touch_window)
            .map(|t| (t.category, t.novelty))
            .or(signals
                .attacked
                .then(|| (Category::Sprite, self.novelty(Category::Sprite))));
        if let Some((category, novelty)) = touched {
            let good = self.params.get(BrainParam::WorthRateGood) * learning_rate_mod;
            let bad = self.params.get(BrainParam::WorthRateBad) * learning_rate_mod;
            let c = kind(category);
            let experience = &mut self.experience;
            for (worth, &relief) in experience.worth.iter_mut().zip(&relief) {
                worth[c] = (worth[c] + good * relief).min(1.0);
            }
            experience.good[c] = (experience.good[c] + good * signals.reward).min(1.0);
            experience.bad[c] = (experience.bad[c] - bad * signals.punishment).max(-1.0);
            // How it went with something new teaches the worth of new things.
            let relieved: f32 = relief.iter().sum();
            let felt = good * (relieved + signals.reward) - bad * signals.punishment;
            experience.new_things = (experience.new_things + felt * novelty).clamp(-1.0, 1.0);
        }
        // Habits, along the trace: each aimed verb and the thing it attended.
        // A fruitless try disappoints the need that chose it.
        let r = signals.reward - signals.punishment;
        let tried = self.touched.filter(|_| signals.fruitless).map(|t| t.tick);
        if r != 0.0 || tried.is_some() {
            let rate = self.params.get(BrainParam::HabitRate) * learning_rate_mod;
            let disappointment = self.params.get(BrainParam::Disappointment);
            let decay = self.params.get(BrainParam::TraceDecay);
            for entry in &self.trace {
                let (Some(verb), Some(category)) = (entry.verb, entry.attended) else {
                    continue;
                };
                if !verb.is_aimed() {
                    continue;
                }
                let disappointed = match entry.motive {
                    Some(need) if tried == Some(entry.tick) => disappointment * signals.needs[need],
                    _ => 0.0,
                };
                let step = rate * (r - disappointed) * weight(decay, tick, entry.tick);
                let habit = &mut self.experience.habits[kind(category)][column(verb)];
                *habit = (*habit + step).clamp(-1.0, 1.0);
            }
        }
        self.fade();
        Vec::new()
    }

    /// One tick of fading (design §5.6): good and bad and habits each by
    /// their own rate.
    fn fade(&mut self) {
        let keep = |param| 1.0 - self.params.get(param);
        let (good, bad, habit) = (
            keep(BrainParam::WorthFadeGood),
            keep(BrainParam::WorthFadeBad),
            keep(BrainParam::HabitFade),
        );
        let experience = &mut self.experience;
        let good_values = experience
            .worth
            .iter_mut()
            .flatten()
            .chain(&mut experience.good);
        for value in good_values {
            *value *= good;
        }
        experience.new_things *= good;
        for value in &mut experience.bad {
            *value *= bad;
        }
        for value in experience.habits.iter_mut().flatten() {
            *value *= habit;
        }
    }

    /// How new `category` is to the sprite (design §5.6): 1 − familiarity.
    pub(crate) fn novelty(&self, category: Category) -> f32 {
        1.0 - self.experience.familiarity[kind(category)]
    }

    /// Each need's level (design §5.2), in the pack's needs order.
    pub(crate) fn need_levels(&self, body: &Body, data: &DataPack) -> Vec<f32> {
        let inputs = data.brain_inputs_in_order();
        data.need_places()
            .iter()
            .map(|&place| match inputs[place].source {
                Source::State(LocusRef::Chem(id)) => {
                    body.chems[data.chemical_index(id).expect("a checked need")]
                }
                _ => unreachable!("a need reads a drive"),
            })
            .collect()
    }

    /// What the sprite has learned, furthest from nothing first, up to
    /// five (design §5.9); ties keep the order they're listed in.
    pub(crate) fn memory(&self, data: &DataPack) -> Vec<Memory> {
        let inputs = data.brain_inputs_in_order();
        let mut memory: Vec<Memory> = Vec::new();
        for (worth, &place) in self.experience.worth.iter().zip(data.need_places()) {
            for (&category, &value) in Category::ALL.iter().zip(worth) {
                memory.push(Memory {
                    learned: Learned::Worth {
                        thing: category.name().to_string(),
                        need: Some(inputs[place].name.clone()),
                    },
                    value,
                });
            }
        }
        let thing = |category: Category| category.name().to_string();
        for (&category, &value) in Category::ALL.iter().zip(&self.experience.good) {
            let learned = Learned::Worth {
                thing: thing(category),
                need: None,
            };
            memory.push(Memory { learned, value });
        }
        for (&category, &value) in Category::ALL.iter().zip(&self.experience.bad) {
            let learned = Learned::Bad {
                thing: thing(category),
            };
            memory.push(Memory { learned, value });
        }
        for (&category, habits) in Category::ALL.iter().zip(&self.experience.habits) {
            for (&verb, &value) in VERBS.iter().zip(habits) {
                let learned = Learned::Habit {
                    thing: thing(category),
                    verb,
                };
                memory.push(Memory { learned, value });
            }
        }
        memory.push(Memory {
            learned: Learned::NewThings,
            value: self.experience.new_things,
        });
        memory.retain(|m| m.value != 0.0);
        // A stable sort keeps a tie in listed order.
        memory.sort_by(|a, b| b.value.abs().total_cmp(&a.value.abs()));
        memory.truncate(MEMORY_SIZE);
        memory
    }

    /// Checks the brain's learned state (design §5.6): links within [−1, 1],
    /// a felt value that's a number, and a trace within its cap.
    pub(crate) fn check(&self) -> Result<(), String> {
        self.decision.check()?;
        self.attention.check()?;
        if !self.felt.is_finite() {
            return Err(format!("felt {}, which isn't a number", self.felt));
        }
        if self.trace.len() > TRACE_CAP {
            return Err(format!(
                "has {} trace entries, over {TRACE_CAP}",
                self.trace.len()
            ));
        }
        Ok(())
    }

    /// Sets a decision link to `w`, unchecked, to test the checks.
    #[cfg(test)]
    pub(crate) fn break_a_link_for_test(&mut self, w: f32) {
        self.decision.set(0, 0, w);
    }

    /// Commits this tick's trace entry at the end of step 6 (design §5.6),
    /// if the brain decided at this tick's step 5, then drops the entries the
    /// next step 4 would credit too little, whether it decided or not.
    pub(crate) fn commit(&mut self, tick: u64) {
        if let Some(snapshot) = self.snapshot.as_ref().filter(|s| s.tick == tick) {
            let motive = snapshot
                .verb
                .and_then(|verb| self.motive(verb, &snapshot.activations));
            if let Some(category) = snapshot.attended {
                // Attending to a kind of thing makes it familiar (design §5.6).
                let rate = self.params.get(BrainParam::FamiliarityRate);
                let familiar = &mut self.experience.familiarity[kind(category)];
                *familiar = (*familiar + rate).min(1.0);
            }
            self.trace.push_back(TraceEntry {
                motive,
                tick,
                activations: snapshot.activations.clone(),
                verb: snapshot.verb,
                attended: snapshot.attended,
            });
        }
        // Oldest first.
        let decay = self.params.get(BrainParam::TraceDecay);
        while self
            .trace
            .front()
            .is_some_and(|e| !still_counts(decay, tick + 1, e.tick))
            || self.trace.len() > TRACE_CAP
        {
            self.trace.pop_front();
        }
    }

    /// The motive for choosing `verb` with these concept `activations`
    /// (design §5.5): of the needs, the one whose singleton pushed the verb
    /// most through its instinct, by its place in the pack's needs; ties to
    /// the lower input ID. None if no need pushed it.
    fn motive(&self, verb: Verb, activations: &[f32]) -> Option<usize> {
        let v = column(verb);
        // The singletons come first, one per input, so a need's place in the
        // inputs is its singleton's place in the concepts.
        let mut pushes: Vec<(usize, usize, f32)> = self
            .need_rows
            .iter()
            .enumerate()
            .map(|(need, &row)| {
                let a = activations.get(row).copied().unwrap_or(0.0);
                (row, need, a * self.decision.get(row, v))
            })
            .filter(|&(_, _, part)| part > 0.0)
            .collect();
        // Inputs are in ID order, so the lower row is the lower input ID.
        pushes.sort_by_key(|&(row, ..)| row);
        pushes
            .into_iter()
            .fold(
                None,
                |best: Option<(usize, f32)>, (_, need, part)| match best {
                    Some((_, top)) if top >= part => best,
                    _ => Some((need, part)),
                },
            )
            .map(|(need, _)| need)
    }

    /// What the brain did at its latest step 5, explained (design §5.9), or
    /// `None` if it hasn't decided anything yet.
    pub(crate) fn explain<'a>(&self, data: &'a DataPack) -> Option<Explanation<'a>> {
        let snapshot = self.snapshot.as_ref()?;
        let mut attention: Vec<(&'static str, f32)> = snapshot
            .attention
            .iter()
            .map(|(category, &score)| (category.name(), score))
            .collect();
        // A stable sort keeps a tie in category order.
        attention.sort_by(|a, b| b.1.total_cmp(&a.1));
        let decision = snapshot
            .verb
            .map(|verb| (verb, snapshot.scores[column(verb)]));
        let names = data.brain_inputs_in_order();
        let mut contributions: Vec<Contribution> = match decision {
            Some((verb, _)) => self
                .concepts
                .iter()
                .zip(&snapshot.activations)
                .zip(self.decision.rows())
                .map(|((signature, &activation), links)| Contribution {
                    inputs: signature
                        .iter()
                        .map(|&(i, negated)| (names[i].name.as_str(), negated))
                        .collect(),
                    amount: activation * links[column(verb)],
                })
                .filter(|c| c.amount != 0.0)
                .collect(),
            None => Vec::new(),
        };
        // A stable sort keeps a tie in concept order.
        contributions.sort_by(|a, b| b.amount.abs().total_cmp(&a.amount.abs()));
        Some(Explanation {
            attention,
            attended: snapshot.attended.map(Category::name),
            decision,
            contributions,
        })
    }

    /// Every input's value (design §5.2), in the pack's input order: State
    /// inputs from `body`, Target inputs from `aim`.
    pub(crate) fn inputs(&self, body: &Body, aim: Option<Aim>, data: &DataPack) -> Vec<f32> {
        let flag = |on: bool| if on { 1.0 } else { 0.0 };
        data.brain_inputs_in_order()
            .iter()
            .map(|input| match input.source {
                Source::State(LocusRef::Chem(id)) => {
                    body.chems[data.chemical_index(id).expect("a checked input")]
                }
                Source::State(LocusRef::Locus(id)) => {
                    body.loci[data.locus_index(id).expect("a checked input")]
                }
                Source::Attended(category) => flag(aim.is_some_and(|a| a.category == category)),
                Source::TargetDistance => aim.map_or(0.0, |a| a.distance),
                Source::TargetAdjacent => flag(aim.is_some_and(|a| a.adjacent)),
            })
            .collect()
    }

    /// Each concept's activation (design §5.4): the product of its inputs,
    /// each negated one as 1 − x.
    pub(crate) fn activations(&self, inputs: &[f32]) -> Vec<f32> {
        self.concepts
            .iter()
            .map(|signature| {
                signature
                    .iter()
                    .map(|&(i, negated)| if negated { 1.0 - inputs[i] } else { inputs[i] })
                    .product()
            })
            .collect()
    }

    /// Each candidate category's attention score (design §5.3): the State
    /// inputs through the attention links, plus salience for nearness.
    /// `candidates` gives each category's normalized distance. Target
    /// inputs never count, whatever `inputs` holds for them.
    pub(crate) fn attention_scores(
        &self,
        inputs: &[f32],
        candidates: &BTreeMap<Category, f32>,
        curiosity_mod: f32,
        data: &DataPack,
    ) -> BTreeMap<Category, f32> {
        let state: Vec<usize> = data
            .brain_inputs_in_order()
            .iter()
            .enumerate()
            .filter(|(_, input)| matches!(input.source, Source::State(_)))
            .map(|(i, _)| i)
            .collect();
        let salience = self.params.get(BrainParam::SalienceGain);
        let value_gain = self.params.get(BrainParam::ValueGain);
        let curiosity = self.params.get(BrainParam::Curiosity);
        let boldness = (1.0 + self.experience.new_things).max(0.0) * curiosity_mod;
        candidates
            .iter()
            .map(|(&category, &distance)| {
                let c = category_column(category);
                let instinct: f32 = state
                    .iter()
                    .map(|&i| inputs[i] * self.attention.get(i, c))
                    .sum();
                let worth = value_gain * self.value(category, inputs, data);
                let curious = curiosity * self.novelty(category) * boldness;
                (
                    category,
                    instinct + salience * (1.0 - distance) + worth + curious,
                )
            })
            .collect()
    }

    /// Where attention goes (design §5.3). Choosing afresh, with no action
    /// running, it samples a category from softmax(score / τ_att), one draw.
    /// With an action running it draws nothing: it stays unless a rival
    /// beats it by `attention_margin`, and then goes to the best rival, ties
    /// to the lower category; with nothing attended, to the best candidate.
    pub(crate) fn attend(
        &mut self,
        scores: &BTreeMap<Category, f32>,
        running: bool,
        exploration: f32,
        rng: &mut ChaCha8Rng,
    ) -> Option<Category> {
        if scores.is_empty() {
            self.attended = None;
        } else if !running {
            let tau = self.params.get(BrainParam::TauAttBase) * exploration;
            let categories: Vec<Category> = scores.keys().copied().collect();
            let values: Vec<f32> = scores.values().copied().collect();
            self.attended = Some(categories[sample(&values, tau, rng)]);
        } else {
            let current = self.attended.and_then(|c| scores.get(&c).copied());
            let bar = current.map_or(f32::NEG_INFINITY, |s| {
                s + self.params.get(BrainParam::AttentionMargin)
            });
            if let Some(best) = best_above(scores.iter().map(|(&c, &s)| (c, s)), bar) {
                self.attended = Some(best);
            } else if current.is_none() {
                self.attended = None;
            }
        }
        self.attended
    }

    /// Each verb's score (design §5.5), in `VERBS` order: the concepts'
    /// activations through the instinct links, and for the thing `aimed`
    /// at, its worth, which draws the sprite to it or makes it back away.
    pub(crate) fn scores(
        &self,
        activations: &[f32],
        inputs: &[f32],
        aimed: Option<Category>,
        data: &DataPack,
    ) -> [f32; VERBS.len()] {
        let mut scores = [0.0; VERBS.len()];
        for (a, links) in activations.iter().zip(self.decision.rows()) {
            for (score, w) in scores.iter_mut().zip(links) {
                *score += a * w;
            }
        }
        if let Some(category) = aimed {
            let worth = self.params.get(BrainParam::ValueGain) * self.value(category, inputs, data);
            let habits = &self.experience.habits[kind(category)];
            for ((score, verb), habit) in scores.iter_mut().zip(VERBS).zip(habits) {
                *score += habit
                    + match verb {
                        Verb::Retreat => -worth,
                        Verb::Rest | Verb::Wander => 0.0,
                        _ => worth,
                    };
            }
        }
        scores
    }

    /// What `category` is worth to the sprite now (design §5.6): each need's
    /// level, from `inputs`, times its worth for that need, plus its general
    /// good and its bad.
    fn value(&self, category: Category, inputs: &[f32], data: &DataPack) -> f32 {
        let c = kind(category);
        let experience = &self.experience;
        let for_needs: f32 = data
            .need_places()
            .iter()
            .zip(&experience.worth)
            .map(|(&place, worth)| inputs[place] * worth[c])
            .sum();
        for_needs + experience.good[c] + experience.bad[c]
    }

    /// A verb sampled from the `available` ones, by softmax(score / τ), τ
    /// being `tau_base` × `exploration` (design §5.5): one draw.
    pub(crate) fn choose(
        &self,
        scores: &[f32; VERBS.len()],
        available: &[Verb],
        exploration: f32,
        rng: &mut ChaCha8Rng,
    ) -> Verb {
        let tau = self.params.get(BrainParam::TauBase) * exploration;
        let values: Vec<f32> = available.iter().map(|&v| scores[column(v)]).collect();
        available[sample(&values, tau, rng)]
    }

    /// The verb a sprite doing `current` switches to, if any (design §5.5):
    /// the best available verb scoring more than `switch_margin` above it,
    /// ties to the lower verb ID. It draws nothing.
    pub(crate) fn switch(
        &self,
        current: Verb,
        scores: &[f32; VERBS.len()],
        available: &[Verb],
    ) -> Option<Verb> {
        let bar = scores[column(current)] + self.params.get(BrainParam::SwitchMargin);
        best_above(available.iter().map(|&v| (v, scores[column(v)])), bar)
    }
}

/// The verbs a sprite may choose (design §5.2): Rest and Wander always;
/// with a target, every other verb, since a sprite may try anything on
/// anything (v16), but Approach only while it isn't already `beside` the
/// target, on one of its goal tiles, where Approach would do nothing.
pub(crate) fn available(target: bool, beside: bool) -> Vec<Verb> {
    VERBS
        .into_iter()
        .filter(|&verb| match verb {
            Verb::Rest | Verb::Wander => true,
            Verb::Approach => target && !beside,
            _ => target,
        })
        .collect()
}

/// The item scoring more than `bar` that scores most, ties to the first.
fn best_above<T: Copy>(scored: impl Iterator<Item = (T, f32)>, bar: f32) -> Option<T> {
    scored
        .filter(|&(_, score)| score > bar)
        .fold(None, |best: Option<(T, f32)>, (item, score)| match best {
            Some((_, top)) if top >= score => best,
            _ => Some((item, score)),
        })
        .map(|(item, _)| item)
}

/// An index into `scores`, sampled by softmax(score / `tau`): one draw.
fn sample(scores: &[f32], tau: f32, rng: &mut ChaCha8Rng) -> usize {
    let top = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let weights: Vec<f32> = scores
        .iter()
        .map(|&s| libm::expf((s - top) / tau))
        .collect();
    let total: f32 = weights.iter().sum();
    let mut left = unit(rng) * total;
    for (i, &w) in weights.iter().enumerate() {
        if left < w {
            return i;
        }
        left -= w;
    }
    // Rounding can leave a sliver past the last.
    scores.len() - 1
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::learning::kind;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    fn brain(genes: &[&str]) -> Brain {
        let data = builtin();
        let text = format!("(format: 1, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        Brain::new(&genome, &data)
    }

    /// A brain of `genes` whose learned values never fade, so they read exactly.
    fn unfading(genes: &[&str]) -> Brain {
        let no_fade = [
            r#"BrainParam(param: "worth_fade_good", value: 0.0)"#,
            r#"BrainParam(param: "worth_fade_bad", value: 0.0)"#,
            r#"BrainParam(param: "habit_fade", value: 0.0)"#,
        ];
        brain(&[genes, &no_fade].concat())
    }

    fn params(genes: &[&str]) -> BrainParams {
        brain(genes).params
    }

    /// Where the input called `name` is in the pack's input order.
    fn input(name: &str) -> usize {
        let data = builtin();
        data.brain_inputs()
            .position(|(_, n)| n == name)
            .expect(name)
    }

    /// Inputs all 0 but those `set` names.
    fn inputs(set: &[(&str, f32)]) -> Vec<f32> {
        let mut x = vec![0.0; builtin().brain_inputs().count()];
        for &(name, value) in set {
            x[input(name)] = value;
        }
        x
    }

    #[test]
    fn a_brain_parameter_is_clamped_to_physiology_s_range() {
        let params = params(&[
            r#"BrainParam(param: "tau_base", value: 5.0)"#,
            r#"BrainParam(param: "switch_margin", value: 0.4)"#,
            r#"BrainParam(param: "switch_margin", value: 0.9)"#,
        ]);
        // physiology.ron: tau_base 0.05–2.0.
        assert_eq!(params.get(BrainParam::TauBase), 2.0);
        assert_eq!(
            params.get(BrainParam::SwitchMargin),
            0.4,
            "the first gene wins"
        );
    }

    #[test]
    fn a_brain_parameter_no_gene_sets_takes_physiology_s_default() {
        let params = params(&[]);
        // physiology.ron's defaults.
        assert_eq!(params.get(BrainParam::TauBase), 0.2);
        assert_eq!(params.get(BrainParam::SalienceGain), 0.5);
        assert_eq!(params.get(BrainParam::ForgetTicks), 5000.0);
    }

    #[test]
    fn every_input_has_a_singleton_and_instincts_add_conjunctions() {
        let brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0)"#,
            r#"Instinct(inputs: [("target_adjacent", true), ("hunger", false)], verb: Approach, weight: 0.5)"#,
            r#"Instinct(inputs: [("hunger", false), ("target_adjacent", true)], verb: Eat, weight: 0.3)"#,
        ]);
        let n = builtin().brain_inputs().count();
        assert_eq!(n, 45, "37 State inputs and 8 Target inputs");
        // One singleton per input, and one conjunction for both genes naming it.
        assert_eq!(brain.concepts.len(), n + 1);
        assert_eq!(
            brain.concepts[n],
            vec![(input("hunger"), false), (input("target_adjacent"), true)]
        );
    }

    #[test]
    fn a_concept_is_the_product_of_its_inputs_with_negation() {
        let brain = brain(&[
            r#"Instinct(inputs: [("hunger", false), ("target_adjacent", true)], verb: Approach, weight: 0.5)"#,
        ]);
        let conjunction = brain.concepts.len() - 1;
        let x = inputs(&[("hunger", 0.8), ("target_adjacent", 0.25)]);
        let a = brain.activations(&x);
        assert_eq!(a[input("hunger")], 0.8, "a singleton is its input");
        assert_eq!(a[conjunction], 0.8 * 0.75, "hunger and not adjacent");
        assert_eq!(a[input("always")], 0.0, "inputs is all 0 but those set");
    }

    #[test]
    fn verb_scores_are_activations_through_the_instinct_links() {
        let data = builtin();
        let brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0)"#,
            r#"Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3)"#,
        ]);
        let x = inputs(&[("hunger", 0.6), ("always", 1.0)]);
        let scores = brain.scores(&brain.activations(&x), &x, None, &data);
        assert_eq!(scores[column(Verb::Eat)], 0.6);
        assert_eq!(scores[column(Verb::Wander)], 0.3);
        assert_eq!(scores[column(Verb::Drink)], 0.0);
    }

    #[test]
    fn target_inputs_never_feed_attention() {
        let data = builtin();
        let brain = brain(&[
            r#"AttentionInstinct(input: "hunger", category: BerryBush, weight: 0.8)"#,
            r#"BrainParam(param: "salience_gain", value: 0.5)"#,
        ]);
        let candidates = BTreeMap::from([(Category::BerryBush, 0.4), (Category::Water, 0.0)]);
        let hungry = inputs(&[("hunger", 0.5)]);
        let also_aimed = inputs(&[
            ("hunger", 0.5),
            ("attended_water", 1.0),
            ("target_distance", 1.0),
            ("target_adjacent", 1.0),
        ]);
        let scores = brain.attention_scores(&hungry, &candidates, 0.0, &data);
        // 0.5 × 0.8 learned, plus 0.5 × (1 − 0.4) salience.
        assert_eq!(scores[&Category::BerryBush], 0.4 + 0.3);
        assert_eq!(scores[&Category::Water], 0.5, "salience alone");
        assert_eq!(
            brain.attention_scores(&also_aimed, &candidates, 0.0, &data),
            scores
        );
    }

    #[test]
    fn with_a_target_every_verb_can_be_tried_whatever_its_verb_table_says() {
        use Verb::*;
        assert_eq!(
            available(false, false),
            [Rest, Wander],
            "no target: targetless only"
        );
        // Design v16 §5.2: a verb table says what a try does, not what's allowed.
        assert_eq!(
            available(true, false),
            [Approach, Eat, Drink, Hit, Play, Retreat, Rest, Wander]
        );
    }

    #[test]
    fn approach_is_not_offered_to_a_sprite_already_beside_its_target() {
        use Verb::*;
        assert_eq!(
            available(true, true),
            [Eat, Drink, Hit, Play, Retreat, Rest, Wander]
        );
    }

    #[test]
    fn a_running_action_switches_only_past_the_margin_to_the_best_ties_to_the_lower_id() {
        let brain = brain(&[r#"BrainParam(param: "switch_margin", value: 0.2)"#]);
        let mut scores = [0.0; VERBS.len()];
        scores[column(Verb::Rest)] = 0.5;
        scores[column(Verb::Wander)] = 0.7;
        let all = [Verb::Approach, Verb::Eat, Verb::Rest, Verb::Wander];
        assert_eq!(
            brain.switch(Verb::Rest, &scores, &all),
            None,
            "0.7 isn't past 0.5 + 0.2"
        );
        scores[column(Verb::Wander)] = 0.75;
        assert_eq!(brain.switch(Verb::Rest, &scores, &all), Some(Verb::Wander));
        scores[column(Verb::Eat)] = 0.75;
        scores[column(Verb::Approach)] = 0.75;
        assert_eq!(
            brain.switch(Verb::Rest, &scores, &all),
            Some(Verb::Approach),
            "a tie goes to the lower verb ID"
        );
        assert_eq!(
            brain.switch(Verb::Rest, &scores, &[Verb::Rest, Verb::Wander]),
            Some(Verb::Wander),
            "only available verbs"
        );
    }

    #[test]
    fn running_attention_moves_only_past_the_margin_ties_to_the_lower_category() {
        let mut brain = brain(&[r#"BrainParam(param: "attention_margin", value: 0.2)"#]);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        brain.attended = Some(Category::Water);
        let mut scores = BTreeMap::from([(Category::Water, 0.3), (Category::Ball, 0.45)]);
        assert_eq!(
            brain.attend(&scores, true, 1.0, &mut rng),
            Some(Category::Water)
        );
        scores.insert(Category::Ball, 0.55);
        scores.insert(Category::Berry, 0.55);
        assert_eq!(
            brain.attend(&scores, true, 1.0, &mut rng),
            Some(Category::Berry),
            "past the margin, to the best; a tie to the lower category"
        );
        scores.remove(&Category::Berry);
        scores.remove(&Category::Water);
        brain.attended = Some(Category::Water);
        assert_eq!(
            brain.attend(&scores, true, 1.0, &mut rng),
            Some(Category::Ball),
            "its category gone, the best of the rest"
        );
        assert_eq!(brain.attend(&BTreeMap::new(), true, 1.0, &mut rng), None);
    }

    #[test]
    fn the_brain_draws_from_the_rng_only_when_choosing_afresh() {
        let mut brain = brain(&[]);
        let scores = BTreeMap::from([(Category::Water, 0.3), (Category::Ball, 0.5)]);
        let verbs = [0.1; VERBS.len()];
        let available = available(true, false);
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let untouched = rng.clone();
        // An action running: attention and switching draw nothing.
        brain.attended = Some(Category::Water);
        brain.attend(&scores, true, 1.0, &mut rng);
        brain.switch(Verb::Rest, &verbs, &available);
        assert_eq!(rng, untouched);
        // Choosing afresh: one draw for attention, one for the verb.
        let mut expected = untouched.clone();
        unit(&mut expected);
        unit(&mut expected);
        brain.attend(&scores, false, 1.0, &mut rng);
        brain.choose(&verbs, &available, 1.0, &mut rng);
        assert_eq!(rng, expected);
    }

    #[test]
    fn softmax_sampling_favours_the_higher_score_as_tau_falls() {
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let pick_first = |tau: f32, rng: &mut ChaCha8Rng| {
            (0..1000)
                .filter(|_| sample(&[1.0, 0.0], tau, rng) == 0)
                .count()
        };
        // At τ = 1, e¹ : e⁰ is about 73% : 27%; at τ = 0.1, almost all.
        let warm = pick_first(1.0, &mut rng);
        let cold = pick_first(0.1, &mut rng);
        assert!((650..800).contains(&warm), "{warm}");
        assert!(cold > 990, "{cold}");
    }

    /// A snapshot of step 5 at `tick`, having chosen `verb` while attending
    /// to `attended`, with `activations`.
    fn snapshot(
        tick: u64,
        activations: Vec<f32>,
        verb: Verb,
        attended: Option<Category>,
    ) -> Snapshot {
        Snapshot {
            tick,
            inputs: Vec::new(),
            activations,
            attention: BTreeMap::new(),
            attended,
            target: None,
            scores: [0.0; VERBS.len()],
            verb: Some(verb),
        }
    }

    /// Has `brain` decide and commit on each of `ticks`.
    fn decide_on(brain: &mut Brain, ticks: std::ops::Range<u64>) {
        for tick in ticks {
            brain.snapshot = Some(snapshot(tick, Vec::new(), Verb::Rest, None));
            brain.commit(tick);
        }
    }

    #[test]
    fn the_trace_keeps_an_entry_while_its_weight_at_the_next_step_4_is_at_least_one_in_a_hundred() {
        // 0.9^43 is about .0108 and 0.9^44 about .0097, so after committing
        // tick 99 the next step 4 (tick 100) credits ticks 57 to 99.
        let mut quick = brain(&[r#"BrainParam(param: "trace_decay", value: 0.9)"#]);
        decide_on(&mut quick, 0..100);
        let ticks: Vec<u64> = quick.trace.iter().map(|e| e.tick).collect();
        assert_eq!(ticks, (57..100).collect::<Vec<u64>>());
        // 0.99^458 is about .0100 and 0.99^459 about .0099.
        let mut slow = brain(&[r#"BrainParam(param: "trace_decay", value: 0.99)"#]);
        decide_on(&mut slow, 0..1000);
        assert_eq!(slow.trace.len(), 458);
        assert_eq!(slow.trace.front().map(|e| e.tick), Some(542));
    }

    /// Activations for `brain`'s concepts, all 0 but the singletons of
    /// the inputs `set` names.
    fn activations(brain: &Brain, set: &[(&str, f32)]) -> Vec<f32> {
        let mut a = vec![0.0; brain.concepts.len()];
        for &(name, value) in set {
            a[input(name)] = value;
        }
        a
    }

    /// Has `brain` decide `verb` at `tick`, attending to `attended`, with
    /// the singletons `set` names active, and commit it.
    fn decide(
        brain: &mut Brain,
        tick: u64,
        verb: Verb,
        attended: Option<Category>,
        set: &[(&str, f32)],
    ) {
        let a = activations(brain, set);
        brain.snapshot = Some(snapshot(tick, a, verb, attended));
        brain.commit(tick);
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_reward_or_a_punishment_leaves_the_instinct_links_as_they_were_born() {
        // Design v16 §5.1: instinct never changes in a sprite's life.
        let data = builtin();
        let mut brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 0.7)"#,
            r#"AttentionInstinct(input: "hunger", category: Berry, weight: 0.4)"#,
        ]);
        let set = [("hunger", 1.0), ("always", 1.0)];
        decide(&mut brain, 9, Verb::Eat, Some(Category::Berry), &set);
        brain.touched = Some(Touch {
            tick: 9,
            category: Category::Berry,
            novelty: 1.0,
        });
        let needs = data.need_places().len();
        let hungry = |level| Signals {
            needs: [vec![level], vec![0.0; needs - 1]].concat(),
            attacked: true,
            ..Default::default()
        };
        brain.learn(10, &hungry(1.0), 1.0, &data);
        brain.learn(11, &hungry(0.0), 1.0, &data);
        let eat = |brain: &Brain, name| brain.decision.get(input(name), column(Verb::Eat));
        assert_eq!(eat(&brain, "hunger"), 0.7);
        assert_eq!(eat(&brain, "always"), 0.0);
        let berry = brain
            .attention
            .get(input("hunger"), category_column(Category::Berry));
        assert_eq!(berry, 0.4);
    }

    /// Step 4's signals with hunger at `hunger`, every other need at 0.
    fn hunger_at(data: &DataPack, hunger: f32, attacked: bool) -> Signals {
        let needs = data.need_places().len();
        Signals {
            needs: [vec![hunger], vec![0.0; needs - 1]].concat(),
            attacked,
            ..Default::default()
        }
    }

    /// What `brain` has learned berry bushes are worth for hunger.
    fn bush_for_hunger(brain: &Brain) -> f32 {
        brain.experience.worth[0][kind(Category::BerryBush)]
    }

    #[test]
    fn a_need_s_fall_is_relief_only_at_or_past_the_deadband() {
        // physiology.ron: relief_deadband .02, worth_rate_good .5 by default.
        let data = builtin();
        let mut brain = unfading(&[]);
        let touch = Touch {
            tick: 1,
            category: Category::BerryBush,
            novelty: 1.0,
        };
        brain.touched = Some(touch);
        brain.learn(1, &hunger_at(&data, 0.5, false), 1.0, &data);
        brain.learn(2, &hunger_at(&data, 0.49, false), 1.0, &data);
        assert_eq!(bush_for_hunger(&brain), 0.0, "a slow drift isn't relief");
        brain.learn(3, &hunger_at(&data, 0.44, false), 1.0, &data);
        assert!(
            close(bush_for_hunger(&brain), 0.5 * 0.05),
            "{}",
            bush_for_hunger(&brain)
        );
    }

    #[test]
    fn with_nothing_touched_relief_teaches_nothing() {
        let data = builtin();
        let mut brain = brain(&[]);
        brain.learn(1, &hunger_at(&data, 1.0, false), 1.0, &data);
        brain.learn(2, &hunger_at(&data, 0.5, false), 1.0, &data);
        assert_eq!(bush_for_hunger(&brain), 0.0);
        // A touch longer ago than the window (3 ticks) is forgotten too.
        brain.touched = Some(Touch {
            tick: 2,
            category: Category::BerryBush,
            novelty: 1.0,
        });
        brain.learn(6, &hunger_at(&data, 0.0, false), 1.0, &data);
        assert_eq!(bush_for_hunger(&brain), 0.0);
    }

    #[test]
    fn an_attacker_is_the_thing_touched_when_the_sprite_touched_nothing() {
        let data = builtin();
        let mut brain = unfading(&[]);
        brain.learn(1, &hunger_at(&data, 1.0, true), 1.0, &data);
        brain.learn(2, &hunger_at(&data, 0.5, true), 1.0, &data);
        let sprite_for_hunger = brain.experience.worth[0][kind(Category::Sprite)];
        assert!(close(sprite_for_hunger, 0.25), "{sprite_for_hunger}");
    }

    #[test]
    fn a_thing_s_worth_for_a_need_counts_as_much_as_the_sprite_needs_it() {
        // Design v16 §5.3: value_gain (1) × Σ need × worth, plus good and bad.
        let data = builtin();
        let mut brain = brain(&[]);
        brain.experience.worth[0][kind(Category::Berry)] = 0.5;
        brain.experience.bad[kind(Category::Thornbush)] = -0.4;
        let candidates = BTreeMap::from([(Category::Berry, 1.0), (Category::Thornbush, 1.0)]);
        let full = brain.attention_scores(&inputs(&[]), &candidates, 0.0, &data);
        assert_eq!(full[&Category::Berry], 0.0, "no hunger, no pull");
        assert_eq!(full[&Category::Thornbush], -0.4, "bad counts always");
        let hungry = brain.attention_scores(&inputs(&[("hunger", 0.8)]), &candidates, 0.0, &data);
        assert!(close(hungry[&Category::Berry], 0.4), "{hungry:?}");
    }

    #[test]
    fn a_worthwhile_target_draws_the_sprite_to_it_and_a_bad_one_makes_it_back_away() {
        // Design v16 §5.5: + worth for Approach and the interactions, − for
        // Retreat, nothing for Rest and Wander.
        use Verb::*;
        let data = builtin();
        let mut brain = brain(&[]);
        brain.experience.good[kind(Category::Ball)] = 0.3;
        let x = inputs(&[]);
        let scores = brain.scores(&brain.activations(&x), &x, Some(Category::Ball), &data);
        for verb in [Approach, Eat, Drink, Hit, Play] {
            assert_eq!(scores[column(verb)], 0.3, "{verb:?}");
        }
        assert_eq!(scores[column(Retreat)], -0.3);
        assert_eq!(scores[column(Rest)], 0.0);
        assert_eq!(scores[column(Wander)], 0.0);
        let unaimed = brain.scores(&brain.activations(&x), &x, None, &data);
        assert_eq!(unaimed, [0.0; VERBS.len()], "nothing attended, no worth");
    }

    #[test]
    fn good_less_bad_teaches_habits_along_the_trace_the_most_recent_most() {
        // Design v16 §5.6: habit_rate (.3) × (reward − punishment) × λ^age,
        // for entries whose verb was aimed at the thing they attended.
        let data = builtin();
        let mut brain = unfading(&[r#"BrainParam(param: "trace_decay", value: 0.5)"#]);
        decide(&mut brain, 7, Verb::Rest, Some(Category::Berry), &[]);
        decide(&mut brain, 8, Verb::Approach, Some(Category::Berry), &[]);
        decide(&mut brain, 9, Verb::Eat, Some(Category::Berry), &[]);
        let hurt = Signals {
            needs: vec![0.0; data.need_places().len()],
            punishment: 1.0,
            ..Default::default()
        };
        brain.learn(10, &hurt, 1.0, &data);
        let habit = |verb| brain.experience.habits[kind(Category::Berry)][column(verb)];
        assert!(close(habit(Verb::Eat), -0.3 * 0.5), "{}", habit(Verb::Eat));
        assert!(close(habit(Verb::Approach), -0.3 * 0.25));
        assert_eq!(habit(Verb::Rest), 0.0, "Rest aims at nothing");
        let eating_berries = Learned::Habit {
            thing: "berry".into(),
            verb: Verb::Eat,
        };
        let memory = brain.memory(&data);
        assert_eq!(memory[0].learned, eating_berries, "{memory:?}");
    }

    #[test]
    fn a_habit_counts_for_its_verb_on_the_thing_attended() {
        let data = builtin();
        let mut brain = brain(&[]);
        brain.experience.habits[kind(Category::Ball)][column(Verb::Eat)] = -0.4;
        let x = inputs(&[]);
        let scores = brain.scores(&brain.activations(&x), &x, Some(Category::Ball), &data);
        assert_eq!(scores[column(Verb::Eat)], -0.4);
        assert_eq!(scores[column(Verb::Play)], 0.0);
        let at_a_berry = brain.scores(&brain.activations(&x), &x, Some(Category::Berry), &data);
        assert_eq!(at_a_berry[column(Verb::Eat)], 0.0);
    }

    #[test]
    fn a_fruitless_try_disappoints_the_need_that_chose_it_in_the_habit() {
        // Design v16 §5.5–5.6: hunger's .8 × 1 pushed Eat more than boredom's
        // 1 × .2, so hunger is the motive: disappointment (.3) × hunger (.8),
        // at habit_rate .3 and λ .9 for the entry a tick old.
        let data = builtin();
        let mut brain = unfading(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0)"#,
            r#"Instinct(inputs: [("boredom", false)], verb: Eat, weight: 0.2)"#,
        ]);
        let set = [("hunger", 0.8), ("boredom", 1.0)];
        decide(&mut brain, 9, Verb::Eat, Some(Category::Ball), &set);
        brain.touched = Some(Touch {
            tick: 9,
            category: Category::Ball,
            novelty: 1.0,
        });
        let mut needs = vec![0.0; data.need_places().len()];
        needs[0] = 0.8;
        let fruitless = Signals {
            needs,
            fruitless: true,
            ..Default::default()
        };
        brain.learn(10, &fruitless, 1.0, &data);
        let habit = brain.experience.habits[kind(Category::Ball)][column(Verb::Eat)];
        assert!(close(habit, -0.3 * 0.3 * 0.8 * 0.9), "{habit}");
        assert_eq!(bush_for_hunger(&brain), 0.0, "worth isn't touched");
    }

    #[test]
    fn a_fruitless_try_no_need_chose_costs_nothing() {
        // Curiosity or a habit, not a need, chose to play with the ball.
        let data = builtin();
        let mut brain =
            brain(&[r#"Instinct(inputs: [("always", false)], verb: Play, weight: 0.5)"#]);
        decide(
            &mut brain,
            9,
            Verb::Play,
            Some(Category::Ball),
            &[("always", 1.0)],
        );
        brain.touched = Some(Touch {
            tick: 9,
            category: Category::Ball,
            novelty: 1.0,
        });
        let fruitless = Signals {
            needs: vec![1.0; data.need_places().len()],
            fruitless: true,
            ..Default::default()
        };
        brain.learn(10, &fruitless, 1.0, &data);
        assert_eq!(
            brain.experience.habits[kind(Category::Ball)][column(Verb::Play)],
            0.0
        );
    }

    #[test]
    fn learned_values_fade_each_tick_at_their_channel_s_rate() {
        // Design v16 §5.6: good by worth_fade_good, bad by worth_fade_bad,
        // habits by habit_fade.
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "worth_fade_good", value: 0.01)"#,
            r#"BrainParam(param: "worth_fade_bad", value: 0.001)"#,
            r#"BrainParam(param: "habit_fade", value: 0.005)"#,
        ]);
        let (berry, thorn) = (kind(Category::Berry), kind(Category::Thornbush));
        brain.experience.worth[0][berry] = 0.5;
        brain.experience.good[berry] = 0.4;
        brain.experience.bad[thorn] = -0.5;
        brain.experience.habits[thorn][column(Verb::Eat)] = -0.2;
        let quiet = Signals {
            needs: vec![0.0; data.need_places().len()],
            ..Default::default()
        };
        brain.learn(1, &quiet, 1.0, &data);
        let experience = &brain.experience;
        assert!(close(experience.worth[0][berry], 0.5 * 0.99));
        assert!(close(experience.good[berry], 0.4 * 0.99));
        assert!(close(experience.bad[thorn], -0.5 * 0.999));
        assert!(close(
            experience.habits[thorn][column(Verb::Eat)],
            -0.2 * 0.995
        ));
    }

    #[test]
    fn attending_to_a_kind_of_thing_makes_it_familiar() {
        // Design v16 §5.6: familiarity_rate (.002) each tick attended.
        let mut brain = brain(&[]);
        for tick in 0..10 {
            decide(&mut brain, tick, Verb::Rest, Some(Category::Ball), &[]);
        }
        let familiar = brain.experience.familiarity[kind(Category::Ball)];
        assert!(close(familiar, 0.02), "{familiar}");
        assert_eq!(brain.experience.familiarity[kind(Category::Berry)], 0.0);
    }

    #[test]
    fn unfamiliar_kinds_of_thing_draw_the_eye_as_boldly_as_the_sprite_is_bold() {
        // Design v16 §5.3: curiosity (.3) × novelty × max(0, 1 + new things)
        // × curiosity_mod; salience is 0 at distance 1.
        let data = builtin();
        let mut brain = brain(&[]);
        brain.experience.familiarity[kind(Category::Ball)] = 0.5;
        let candidates = BTreeMap::from([(Category::Berry, 1.0), (Category::Ball, 1.0)]);
        let x = inputs(&[]);
        let score = |brain: &Brain, mood: f32| brain.attention_scores(&x, &candidates, mood, &data);
        let calm = score(&brain, 1.0);
        assert!(close(calm[&Category::Berry], 0.3), "{calm:?}");
        assert!(
            close(calm[&Category::Ball], 0.15),
            "half familiar, half the pull"
        );
        let wary = score(&brain, 0.5);
        assert!(close(wary[&Category::Berry], 0.15), "wariness halves it");
        brain.experience.new_things = -0.5;
        let shy = score(&brain, 1.0);
        assert!(close(shy[&Category::Berry], 0.15), "a shy sprite, half");
    }

    #[test]
    fn a_hurt_from_something_new_teaches_that_new_things_are_bad() {
        // Design v16 §5.6: (good − worth_rate_bad (.8) × punishment) × how
        // new the thing touched was.
        let data = builtin();
        let mut brain = unfading(&[]);
        brain.touched = Some(Touch {
            tick: 1,
            category: Category::Thornbush,
            novelty: 0.5,
        });
        let hurt = Signals {
            needs: vec![0.0; data.need_places().len()],
            punishment: 1.0,
            ..Default::default()
        };
        brain.learn(1, &hurt, 1.0, &data);
        assert!(
            close(brain.experience.new_things, -0.8 * 0.5),
            "{}",
            brain.experience.new_things
        );
        let memory = brain.memory(&data);
        assert!(
            memory.iter().any(|m| m.learned == Learned::NewThings),
            "{memory:?}"
        );
    }

    #[test]
    fn an_instinct_past_one_is_born_at_one() {
        let brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.5)"#,
            r#"AttentionInstinct(input: "hunger", category: Berry, weight: -1.2)"#,
        ]);
        assert_eq!(brain.decision.get(input("hunger"), column(Verb::Eat)), 1.0);
        assert_eq!(
            brain
                .attention
                .get(input("hunger"), category_column(Category::Berry)),
            -1.0
        );
    }

    #[test]
    fn a_brain_that_stops_deciding_still_drops_what_no_longer_counts() {
        // A sprite held by the hand, or on a scripted action, commits no
        // entries, but the ones it has still fade out: 0.9^43 is the last
        // that counts.
        let mut brain = brain(&[r#"BrainParam(param: "trace_decay", value: 0.9)"#]);
        decide_on(&mut brain, 0..3);
        brain.commit(40);
        assert_eq!(brain.trace.len(), 3, "0.9^(41 − 0) still counts");
        brain.commit(43);
        let ticks: Vec<u64> = brain.trace.iter().map(|e| e.tick).collect();
        assert_eq!(
            ticks,
            [1, 2],
            "at tick 44's step 4, tick 0's weight is 0.9^44"
        );
        brain.commit(100);
        assert!(brain.trace.is_empty());
    }
}
