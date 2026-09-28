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
use crate::learning::{LearnableLinks, TRACE_CAP, TraceEntry, still_counts, weight};
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
    /// Decision links W: each concept's weight towards each verb, in `VERBS` order.
    decision: LearnableLinks<{ VERBS.len() }>,
    /// Attention links A: each State input's weight towards each category,
    /// in `Category::ALL` order, by the input's place in the pack.
    attention: LearnableLinks<{ Category::ALL.len() }>,
    /// The category attention is on, if any.
    pub(crate) attended: Option<Category>,
    /// What the brain did at the latest step 5 (design §5.5), for the trace
    /// entry learning commits and for the Brain tab.
    pub(crate) snapshot: Option<Snapshot>,
    /// The reward less the punishment step 4 last used up: `last_r` (design §5.6).
    pub(crate) felt: f32,
    /// What it felt and chose on its recent ticks, oldest first (design §5.6).
    pub(crate) trace: VecDeque<TraceEntry>,
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

/// A link, named (design §5.6): from a concept to a verb, or from a State
/// input to a category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// A concept's link to a verb: the concept's inputs by name, each with
    /// whether it's negated.
    Decision {
        inputs: Vec<(String, bool)>,
        verb: Verb,
    },
    /// A State input's link to a category, as brain inputs name it (`thornbush`).
    Attention { input: String, category: String },
}

/// The link from the input at `input` in the pack to `category`, named.
fn attention_link(input: usize, category: Category, data: &DataPack) -> Link {
    Link::Attention {
        input: data.brain_inputs_in_order()[input].name.clone(),
        category: category.name().to_string(),
    }
}

/// How far one link has come from birth (design §5.9).
#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub link: Link,
    /// Its working weight now.
    pub now: f32,
    /// The instinct it was born with.
    pub birth: f32,
}

/// How many links the memory lists (design §5.9).
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
    /// The links, of both kinds, furthest from birth, largest first, ties
    /// in link order: decision links, then attention links. A link that
    /// hasn't moved is left out.
    pub memory: Vec<Memory>,
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
            decision: LearnableLinks::new(decision),
            attention: LearnableLinks::new(attention),
            attended: None,
            snapshot: None,
            felt: 0.0,
            trace: VecDeque::new(),
        }
    }

    /// Step 4 at `tick` (design §2.4, §5.6): `r`, scaled by
    /// `learning_rate_mod`, is credited back along the trace, then every
    /// link relaxes a tick. Each entry's verb gains η × mod × r × λ^(tick −
    /// its tick) × each concept's activation from that concept; if the verb
    /// was aimed at a target, its attended category gains the same from each
    /// State input. Links stay within [−1, 1]. Returns the lessons it learned, decision links
    /// first: each link that moved `lesson_threshold` from birth for the first
    /// time, with whether it rose.
    pub(crate) fn learn(
        &mut self,
        tick: u64,
        r: f32,
        learning_rate_mod: f32,
        data: &DataPack,
    ) -> Vec<(Link, bool)> {
        if r == 0.0 {
            // Relaxing only takes a link back towards birth: no lessons.
            self.relax();
            return Vec::new();
        }
        let rate = self.params.get(BrainParam::LearningRate) * learning_rate_mod * r;
        let decay = self.params.get(BrainParam::TraceDecay);
        let inputs = data.brain_inputs_in_order();
        for entry in &self.trace {
            let Some(verb) = entry.verb else {
                continue;
            };
            let step = rate * weight(decay, tick, entry.tick);
            for (k, &a) in entry.activations.iter().enumerate() {
                self.decision.nudge(k, column(verb), step * a);
            }
            let Some(category) = entry.attended.filter(|_| verb.is_aimed()) else {
                continue;
            };
            // The singletons come first, one per input, so their
            // activations are the inputs.
            for (i, input) in inputs.iter().enumerate() {
                if matches!(input.source, Source::State(_)) {
                    let x = entry.activations[i];
                    self.attention.nudge(i, category_column(category), step * x);
                }
            }
        }
        self.relax();
        let threshold = data.physiology().lesson_threshold;
        let decisions: Vec<(Link, bool)> = self
            .decision
            .lessons(threshold)
            .into_iter()
            .map(|(k, v, good)| (self.decision_link(k, VERBS[v], data), good))
            .collect();
        let attention = self
            .attention
            .lessons(threshold)
            .into_iter()
            .map(|(i, c, good)| (attention_link(i, Category::ALL[c], data), good));
        decisions.into_iter().chain(attention).collect()
    }

    /// One tick of the two timescales for every link (design §5.6).
    fn relax(&mut self) {
        let (relax, consolidate) = (
            self.params.get(BrainParam::RelaxRate),
            self.params.get(BrainParam::ConsolidateRate),
        );
        self.decision.relax(relax, consolidate);
        self.attention.relax(relax, consolidate);
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
            self.trace.push_back(TraceEntry {
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
                .zip(self.decision.working())
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
            memory: self.memory(data),
        })
    }

    /// The links furthest from birth (design §5.9), of those that have moved.
    fn memory(&self, data: &DataPack) -> Vec<Memory> {
        let decisions = self
            .decision
            .moved()
            .map(|(k, v, now, birth)| (self.decision_link(k, VERBS[v], data), now, birth));
        let attention = self
            .attention
            .moved()
            .map(|(i, c, now, birth)| (attention_link(i, Category::ALL[c], data), now, birth));
        let mut memory: Vec<Memory> = decisions
            .chain(attention)
            .map(|(link, now, birth)| Memory { link, now, birth })
            .collect();
        // A stable sort keeps a tie in link order.
        memory.sort_by(|a, b| (b.now - b.birth).abs().total_cmp(&(a.now - a.birth).abs()));
        memory.truncate(MEMORY_SIZE);
        memory
    }

    /// The link from the concept at `concept` to `verb`, named.
    fn decision_link(&self, concept: usize, verb: Verb, data: &DataPack) -> Link {
        let names = data.brain_inputs_in_order();
        let inputs = self.concepts[concept]
            .iter()
            .map(|&(i, negated)| (names[i].name.clone(), negated))
            .collect();
        Link::Decision { inputs, verb }
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
        candidates
            .iter()
            .map(|(&category, &distance)| {
                let c = category_column(category);
                let learned: f32 = state
                    .iter()
                    .map(|&i| inputs[i] * self.attention.get(i, c))
                    .sum();
                (category, learned + salience * (1.0 - distance))
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

    /// Each verb's score (design §5.5): the concepts' activations through
    /// the decision links, in `VERBS` order.
    pub(crate) fn scores(&self, activations: &[f32]) -> [f32; VERBS.len()] {
        let mut scores = [0.0; VERBS.len()];
        for (a, links) in activations.iter().zip(self.decision.working()) {
            for (score, w) in scores.iter_mut().zip(links) {
                *score += a * w;
            }
        }
        scores
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

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    fn brain(genes: &[&str]) -> Brain {
        let data = builtin();
        let text = format!("(format: 1, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        Brain::new(&genome, &data)
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
        assert_eq!(n, 44, "36 State inputs and 8 Target inputs");
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
        let brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0)"#,
            r#"Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3)"#,
        ]);
        let x = inputs(&[("hunger", 0.6), ("always", 1.0)]);
        let scores = brain.scores(&brain.activations(&x));
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
        let scores = brain.attention_scores(&hungry, &candidates, &data);
        // 0.5 × 0.8 learned, plus 0.5 × (1 − 0.4) salience.
        assert_eq!(scores[&Category::BerryBush], 0.4 + 0.3);
        assert_eq!(scores[&Category::Water], 0.5, "salience alone");
        assert_eq!(
            brain.attention_scores(&also_aimed, &candidates, &data),
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
    fn a_reward_credits_what_came_before_it_the_most_recent_most() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.1)"#,
            r#"BrainParam(param: "trace_decay", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.0)"#,
            r#"BrainParam(param: "consolidate_rate", value: 0.0)"#,
        ]);
        let always = [("always", 1.0)];
        decide(&mut brain, 7, Verb::Eat, None, &always);
        decide(&mut brain, 8, Verb::Drink, None, &always);
        decide(&mut brain, 9, Verb::Play, None, &always);
        brain.learn(10, 1.0, 1.0, &data);
        let link = |brain: &Brain, verb| brain.decision.get(input("always"), column(verb));
        // η × r × λ^(10 − t_j) × activation.
        assert!(
            close(link(&brain, Verb::Play), 0.1 * 0.5),
            "{}",
            link(&brain, Verb::Play)
        );
        assert!(close(link(&brain, Verb::Drink), 0.1 * 0.25));
        assert!(close(link(&brain, Verb::Eat), 0.1 * 0.125));
        // What it does after the reward gets none of it: the reward was used up.
        decide(&mut brain, 10, Verb::Wander, None, &always);
        brain.learn(11, 0.0, 1.0, &data);
        assert_eq!(link(&brain, Verb::Wander), 0.0);
        assert!(close(link(&brain, Verb::Play), 0.1 * 0.5));
    }

    #[test]
    fn a_reward_moves_each_link_by_how_active_its_concept_was() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.1)"#,
            r#"BrainParam(param: "trace_decay", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.0)"#,
        ]);
        decide(
            &mut brain,
            9,
            Verb::Eat,
            None,
            &[("hunger", 0.8), ("always", 1.0)],
        );
        brain.learn(10, -1.0, 2.0, &data);
        let eat = |name| brain.decision.get(input(name), column(Verb::Eat));
        // η × learning_rate_mod × r × λ × activation.
        assert!(
            close(eat("hunger"), -0.1 * 2.0 * 0.5 * 0.8),
            "{}",
            eat("hunger")
        );
        assert!(close(eat("always"), -0.1 * 2.0 * 0.5));
        assert_eq!(eat("thirst"), 0.0, "an inactive concept learns nothing");
    }

    #[test]
    fn a_reward_moves_attention_only_for_actions_aimed_at_a_target() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.1)"#,
            r#"BrainParam(param: "trace_decay", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.0)"#,
        ]);
        let set = [("hunger", 0.8), ("attended_thornbush", 1.0)];
        let thornbush = Some(Category::Thornbush);
        decide(&mut brain, 8, Verb::Eat, thornbush, &set);
        decide(&mut brain, 9, Verb::Rest, thornbush, &set);
        brain.learn(10, -1.0, 1.0, &data);
        let attends = |name| {
            brain
                .attention
                .get(input(name), category_column(Category::Thornbush))
        };
        // Only the Eat, at λ²: the Rest used no target.
        assert!(
            close(attends("hunger"), -0.1 * 0.25 * 0.8),
            "{}",
            attends("hunger")
        );
        assert_eq!(
            attends("attended_thornbush"),
            0.0,
            "a Target input never feeds attention"
        );
    }

    #[test]
    fn no_link_goes_past_minus_one_or_plus_one() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.0)"#,
            r#"Instinct(inputs: [("always", false)], verb: Eat, weight: 0.9)"#,
            r#"AttentionInstinct(input: "always", category: Berry, weight: -0.9)"#,
        ]);
        decide(
            &mut brain,
            9,
            Verb::Eat,
            Some(Category::Berry),
            &[("always", 1.0)],
        );
        brain.learn(10, 1.0, 2.0, &data);
        assert_eq!(brain.decision.get(input("always"), column(Verb::Eat)), 1.0);
        brain.learn(11, -1.0, 2.0, &data);
        brain.learn(12, -1.0, 2.0, &data);
        brain.learn(13, -1.0, 2.0, &data);
        assert_eq!(
            brain
                .attention
                .get(input("always"), category_column(Category::Berry)),
            -1.0
        );
    }

    #[test]
    fn a_learned_change_fades_towards_the_settled_weight_which_creeps_towards_it() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.5)"#,
            r#"BrainParam(param: "trace_decay", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.01)"#,
            r#"BrainParam(param: "consolidate_rate", value: 0.001)"#,
        ]);
        decide(&mut brain, 9, Verb::Eat, None, &[("always", 1.0)]);
        // The reward lifts w from 0 to .25, then the links relax a tick
        // (design §2.4): w += .01 × (0 − .25) and w_long += .001 × (.25 − 0),
        // both from the values before relaxing.
        brain.learn(10, 1.0, 1.0, &data);
        let eat = |brain: &Brain| brain.decision.get(input("always"), column(Verb::Eat));
        assert!(close(eat(&brain), 0.2475), "{}", eat(&brain));
        let long = brain.decision.settled(input("always"), column(Verb::Eat));
        assert!(close(long, 0.00025), "{long}");
        // In the end both settle on the same weight: c·w + r·w_long is kept,
        // so (.001 × .25 + .01 × 0) / .011.
        for tick in 11..20_000 {
            brain.learn(tick, 0.0, 1.0, &data);
        }
        assert!((eat(&brain) - 0.25 / 11.0).abs() < 1e-4, "{}", eat(&brain));
    }

    #[test]
    fn a_link_that_moves_half_a_point_from_birth_is_a_lesson_once() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.4)"#,
            r#"BrainParam(param: "trace_decay", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.0)"#,
            r#"AttentionInstinct(input: "hunger", category: Thornbush, weight: 0.3)"#,
        ]);
        let set = [("hunger", 1.0), ("attended_thornbush", 1.0)];
        decide(&mut brain, 9, Verb::Eat, Some(Category::Thornbush), &set);
        // −.4 × .5 = −.2 a time: the third takes both links past −.5 from birth.
        assert_eq!(brain.learn(10, -1.0, 1.0, &data), []);
        assert_eq!(brain.learn(10, -1.0, 1.0, &data), []);
        let eat = |input: &str| Link::Decision {
            inputs: vec![(input.to_string(), false)],
            verb: Verb::Eat,
        };
        let attends = Link::Attention {
            input: "hunger".into(),
            category: "thornbush".into(),
        };
        assert_eq!(
            brain.learn(10, -1.0, 1.0, &data),
            [
                (eat("hunger"), false),
                (eat("attended_thornbush"), false),
                (attends, false),
            ]
        );
        assert_eq!(brain.learn(10, -1.0, 1.0, &data), [], "each lesson once");
    }

    #[test]
    fn memory_is_the_five_links_furthest_from_birth_largest_first() {
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "learning_rate", value: 0.4)"#,
            r#"BrainParam(param: "trace_decay", value: 0.5)"#,
            r#"BrainParam(param: "relax_rate", value: 0.0)"#,
            r#"AttentionInstinct(input: "hunger", category: Thornbush, weight: 0.3)"#,
        ]);
        let set = [
            ("hunger", 1.0),
            ("attended_thornbush", 1.0),
            ("always", 0.5),
            ("thirst", 0.05),
        ];
        decide(&mut brain, 9, Verb::Eat, Some(Category::Thornbush), &set);
        let memory = |brain: &Brain| brain.explain(&data).expect("a decision").memory;
        assert_eq!(memory(&brain), [], "nothing has moved yet");
        // −.4 × .5 = −.2 for each input at 1, −.1 at .5 and −.01 at .05.
        brain.learn(10, -1.0, 1.0, &data);
        let eat = |inputs: &[&str]| Link::Decision {
            inputs: inputs.iter().map(|&i| (i.to_string(), false)).collect(),
            verb: Verb::Eat,
        };
        let attends = |input: &str| Link::Attention {
            input: input.into(),
            category: "thornbush".into(),
        };
        let got: Vec<(Link, f32, f32)> = memory(&brain)
            .into_iter()
            .map(|m| (m.link, m.now, m.birth))
            .collect();
        // Ties keep link order: decision links in concept order, then attention.
        let expected = [
            (eat(&["hunger"]), -0.2, 0.0),
            (eat(&["attended_thornbush"]), -0.2, 0.0),
            (attends("hunger"), 0.1, 0.3),
            (eat(&["always"]), -0.1, 0.0),
            (attends("always"), -0.1, 0.0),
        ];
        assert_eq!(got.len(), expected.len(), "{got:?}");
        for ((link, now, birth), (want, want_now, want_birth)) in got.iter().zip(&expected) {
            assert_eq!(link, want);
            assert!(
                close(*now, *want_now) && close(*birth, *want_birth),
                "{link:?} {now} {birth}"
            );
        }
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
