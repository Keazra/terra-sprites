//! The brain (design §5): what a sprite attends to (5a) and what it chooses
//! to do about it (5b), from its inputs through its concepts.

use std::collections::{BTreeMap, VecDeque};

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::ageing::{LifeStage, health_of};
use crate::biochem::Body;
use crate::brain_io::{InputId, Source};
use crate::data::DataPack;
use crate::expression::{Expression, expressions};
use crate::family::Relation;
use crate::genome::{Gene, Genome, LocusRef, Whom};
use crate::learning::{
    Experience, Knowledge, Links, Signals, SpriteMemory, Subject, TRACE_CAP, Topic, Touch,
    TraceEntry, TriedAt, TypeMemory, per_verb, settled, still_counts, toward, weight,
};
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::Target;
use crate::random::unit;
use crate::registry::{BrainParam, CategoryId, Verb};

/// The verbs the brain scores, in verb ID order: every verb but the reserved
/// ones. A verb's place here is its column in the decision links. A save
/// from before Mate and BreedAlone, or Court and Call, has columns fewer,
/// which it gains at 0 (M2 design §5, §10, §15).
pub(crate) const VERBS: [Verb; 12] = [
    Verb::Approach,
    Verb::Eat,
    Verb::Drink,
    Verb::Hit,
    Verb::Play,
    Verb::Retreat,
    Verb::Rest,
    Verb::Wander,
    Verb::Mate,
    Verb::BreedAlone,
    Verb::Court,
    Verb::Call,
];

/// Which way a thing's worth pushes a verb aimed at it (design v18 §5.5):
/// towards it for Approach and the interactions, and not at all for
/// Retreat, which is fear's (a bad thing is left alone, not backed away
/// from), or the targetless verbs.
fn side(verb: Verb) -> f32 {
    match verb {
        Verb::Retreat | Verb::Rest | Verb::Wander | Verb::BreedAlone | Verb::Call => 0.0,
        _ => 1.0,
    }
}

/// How fear pushes a verb aimed at a frightening thing (design v18 §5.5):
/// `flight` towards Retreat, `value_gain` away from Approach, Eat, Drink,
/// Play, Mate and Court, and never towards or away from Hit, since fear
/// that pushed Hit started feuds (#62).
fn fear_push(verb: Verb, flight: f32, value_gain: f32) -> f32 {
    match verb {
        Verb::Retreat => flight,
        Verb::Approach | Verb::Eat | Verb::Drink | Verb::Play | Verb::Mate | Verb::Court => {
            -value_gain
        }
        _ => 0.0,
    }
}

/// The signature of the concept an instinct on `inputs` links from (design
/// §5.4): each input by its place in the pack, in order.
fn signature_of(inputs: &[(InputId, bool)], data: &DataPack) -> Signature {
    let mut signature: Signature = inputs
        .iter()
        .map(|&(id, negated)| (data.brain_input_index(id).expect("a checked gene"), negated))
        .collect();
    signature.sort();
    signature
}

/// Where `verb` is in `VERBS`.
fn column(verb: Verb) -> usize {
    VERBS
        .iter()
        .position(|&v| v == verb)
        .expect("a verb the brain scores")
}

/// The brain's parameters (design §5.7): each from the first `BrainParam`
/// gene that sets it, clamped to physiology's range, or physiology's
/// default where no gene does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct BrainParams {
    /// In `BrainParam::ALL` order. A `Vec`, since serde serialises arrays
    /// only up to 32 long.
    values: Vec<f32>,
}

impl BrainParams {
    pub(crate) fn express(genome: &Genome, data: &DataPack, stage: LifeStage) -> BrainParams {
        BrainParams::express_as(genome, data, &expressions(genome, data, stage))
    }

    /// The same, from how each gene is expressed, in genome order (#210).
    fn express_as(genome: &Genome, data: &DataPack, expressed: &[Expression]) -> BrainParams {
        let ranges = &data.physiology().brain;
        let mut values: Vec<f32> = BrainParam::ALL
            .iter()
            .map(|&param| ranges.of(param).default)
            .collect();
        for (gene, expression) in genome.genes.iter().zip(expressed) {
            if let (&Gene::BrainParam { param, value }, Expression::Expressed) = (gene, expression)
            {
                let (low, high) = ranges.of(param).range;
                values[index(param)] = value.clamp(low, high);
            }
        }
        BrainParams { values }
    }

    /// Gives parameters added since a brain was saved the values its
    /// `genome` expresses now, at `stage` (design §2.8), so an older save
    /// plays on.
    pub(crate) fn catch_up(&mut self, genome: &Genome, data: &DataPack, stage: LifeStage) {
        let now = BrainParams::express(genome, data, stage);
        if let Some(added) = now.values.get(self.values.len()..) {
            self.values.extend_from_slice(added);
        }
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

/// Which sprite stands for sprites while the brain scores (design v18
/// §5.3, §5.5), and whether fear is quiet: a hit is still felt, or the
/// sprite is cornered, moments that are instinct's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct SpriteScoring {
    pub(crate) sprite: Option<EntityId>,
    pub(crate) quiet: bool,
}

/// A thing attention weighs (design §5.3): what it's learned about as, its
/// normalized distance, and how much of its worth counts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Seen {
    pub(crate) subject: Subject,
    pub(crate) distance: f32,
    /// 1 for a thing in sight; for a remembered place, how well it's
    /// remembered (M2 design §7).
    pub(crate) recall: f32,
    /// Whether it's a sprite not grown up, an egg or young, which the
    /// breeding drive never draws the eye to (M2 design §5).
    pub(crate) not_grown: bool,
    /// Who it is to the sprite (M2 design §15.6).
    pub(crate) relation: Relation,
}

#[cfg(test)]
impl Seen {
    /// A thing in sight, at normalized `distance`.
    pub(crate) fn in_sight(subject: Subject, distance: f32) -> Seen {
        Seen {
            subject,
            distance,
            recall: 1.0,
            not_grown: false,
            relation: Relation::default(),
        }
    }
}

/// What a sprite knows about the thing it attends to, for the Target inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Aim {
    pub(crate) category: CategoryId,
    /// What it's learned about as (design v19 §5.6).
    pub(crate) subject: Subject,
    /// Its path cost, normalized (design §5.2): 0 on it, 1 at the edge of sight.
    pub(crate) distance: f32,
    /// Whether the sprite stands on one of its goal tiles.
    pub(crate) adjacent: bool,
    /// How much of its worth draws the eye: 1 in sight, or how well a
    /// remembered place is remembered (M2 design §7).
    pub(crate) recall: f32,
    /// Whether it's a sprite not grown up, an egg or young, which the
    /// breeding drive never moves the sprite to act on (M2 design §5).
    pub(crate) not_grown: bool,
    /// Who it is to the sprite (M2 design §15.6).
    pub(crate) relation: Relation,
}

/// An attention instinct about whom (M2 design §15.6): a State input's
/// link to attending to the sprite's family, or its kin, in a category.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct WhomLink {
    /// The input, by its place in the pack.
    input: usize,
    /// The category, by its index in the pack.
    category: usize,
    whom: Whom,
    /// Held within [−1, 1], as every instinct link is (M1 design §5.4).
    weight: f32,
}

/// A sprite's brain: its parameters, concepts and links (design §5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Brain {
    pub(crate) params: BrainParams,
    /// Singletons, one per input in input order, then the innate
    /// conjunctions in genome order.
    pub(crate) concepts: Vec<Signature>,
    /// Decision instincts W: each concept's link to each verb, in `VERBS` order.
    decision: Links,
    /// Attention instincts A: each State input's link to each category, in
    /// the pack's category order, by the input's place in the pack.
    attention: Links,
    /// Attention instincts about whom (M2 design §15.6), in genome order:
    /// none in a save from before them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    whom: Vec<WhomLink>,
    /// The category attention is on, if any.
    pub(crate) attended: Option<CategoryId>,
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
    /// How far back the Cursor's touch this tick looks, from step 1 to
    /// step 4 (design v21 §5.6); `None` untouched.
    pub(crate) reach_back: Option<u64>,
    /// Whether the sprite could see the Cursor that touched it this tick,
    /// from step 1 to step 4 (design v29 §5.6).
    pub(crate) seen_cursor: bool,
    /// What its summaries were made of when it last looked for lessons, if
    /// it has since it was made or loaded (design §5.6). Derived, so it
    /// isn't saved or hashed.
    #[serde(skip)]
    looked: Option<SummaryBasis>,
}

/// What a brain's summaries are made of (design v18, v19 §5.6): the sprites
/// and object types they're over, and how strongly it generalises. While
/// that holds and it learns nothing, fading only brings a summary nearer 0.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SummaryBasis {
    sprites: usize,
    types: usize,
    generalise: u32,
    generalise_types: u32,
}

/// What the brain saw and did at a step 5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    /// The tick of that step 5.
    pub(crate) tick: u64,
    /// Every input's value, in the pack's input order.
    pub(crate) inputs: Vec<f32>,
    /// Every concept's activation, in the brain's concept order.
    pub(crate) activations: Vec<f32>,
    /// Each candidate category's attention score.
    pub(crate) attention: BTreeMap<CategoryId, f32>,
    /// What each category's score was for, learned about as (design v19
    /// §5.9): its candidate, or the thing a running action is aimed at.
    pub(crate) scored: BTreeMap<CategoryId, Subject>,
    pub(crate) attended: Option<CategoryId>,
    /// The one thing attention is on: a running action's target, or else the
    /// attended category's candidate.
    pub(crate) target: Option<Target>,
    /// What that thing is learned about as (design v19 §5.6).
    pub(crate) subject: Option<Subject>,
    /// Each verb's score, in `VERBS` order.
    #[serde(deserialize_with = "per_verb")]
    pub(crate) scores: [f32; VERBS.len()],
    /// The verb it's doing, if any.
    pub(crate) verb: Option<Verb>,
    /// The verb's motive (design §5.5): the need, by its place in the pack's
    /// needs, whose instinct did most to choose it.
    pub(crate) motive: Option<usize>,
    /// The sprite attention scored sprites by (design v18 §5.3).
    pub(crate) sprite_seen: Option<EntityId>,
    /// The sprite the decision was about, and whether fear was quiet (§5.5).
    pub(crate) scoring: SpriteScoring,
}

/// What something learned is about (design v19 §5.9, §6.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Thing {
    /// An object type, by name (`berry_bush`).
    ObjectType(String),
    /// A category, by name (`bush`): its summary, what the sprite thinks of
    /// the things in it it hasn't met (design v19 §5.6). The sprite
    /// category's is sprites in general (design v18 §5.6). Water or sprites
    /// in a pack with no object type for them are learned about as their
    /// category too.
    Category(String),
    /// A particular sprite (design v18 §5.6).
    Sprite(EntityId),
    /// The Cursor, which sprites learn about as one individual while they
    /// can see it (design v29 §5.6).
    Cursor,
}

impl From<&str> for Thing {
    /// The object type called `object_type`.
    fn from(object_type: &str) -> Thing {
        Thing::ObjectType(object_type.to_string())
    }
}

/// Something a sprite has learned, named (design §5.6, §5.9). Needs are
/// named as brain inputs name them (`hunger`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Learned {
    /// What a thing is worth: for a need, or in general (`None`).
    Worth { thing: Thing, need: Option<String> },
    /// How bad a thing is: it hurt the sprite when it touched it.
    Bad { thing: Thing },
    /// How frightening a thing is (design v18 §5.6): it hurt the sprite by
    /// its own doing.
    Fear { thing: Thing },
    /// A habit: doing a verb to an object type (design v19 §5.6).
    Habit { thing: Thing, verb: Verb },
    /// The worth of new things.
    NewThings,
}

/// One thing a sprite has learned, and how far it has got from nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub learned: Learned,
    pub amount: f32,
}

/// A place a sprite remembers (M2 design §7): what it found there, the
/// thing's tile, and its recall, how well it remembers it, from 1 just after
/// it was eased there, fading towards 0.
#[derive(Debug, Clone, PartialEq)]
pub struct RememberedPlace {
    pub thing: Thing,
    pub at: Pos,
    pub recall: f32,
}

/// What a sprite thinks of the things in a category it hasn't met (design
/// v19 §5.6): the category's summary of each need's worth, of general good
/// and of bad.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Summary {
    pub(crate) worth: Vec<f32>,
    pub(crate) good: f32,
    pub(crate) bad: f32,
}

/// What a brain did at its latest step 5, explained (design §5.9): what it
/// could attend to, and why it's doing what it's doing.
#[derive(Debug, Clone, PartialEq)]
pub struct Explanation<'a> {
    /// Each candidate category's attention score, highest first, ties to
    /// the lower category, named by what was scored: its object type, or
    /// for sprites the one scored (design v19 §5.9).
    pub attention: Vec<(Thing, f32)>,
    /// What attention is on, if anything.
    pub attended: Option<Thing>,
    /// The verb it chose or kept doing, with its score.
    pub decision: Option<(Verb, f32)>,
    /// The concepts adding to or taking from that verb's score, largest
    /// first whatever the sign, ties in concept order. A concept adding
    /// nothing is left out.
    pub contributions: Vec<Contribution<'a>>,
}

/// How much one part of the brain adds to a verb's score (design §5.9).
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution<'a> {
    pub part: Part<'a>,
    /// What it adds, or takes away if negative.
    pub amount: f32,
}

/// A part of the brain that adds to a verb's score (design §5.5, §5.9).
#[derive(Debug, Clone, PartialEq)]
pub enum Part<'a> {
    /// An instinct: a concept's inputs by name, each with whether it's
    /// negated, through its link to the verb.
    Concept(Vec<(&'a str, bool)>),
    /// What the thing attended to is worth: an object type, or a particular
    /// sprite.
    Worth(Thing),
    /// How frightening the thing attended to is (design v18 §5.5).
    Fear(Thing),
    /// The habit of doing the verb to the thing attended to.
    Habit(Thing),
}

impl Brain {
    /// A newborn's brain from its genome (design §5.4, §5.7): a singleton per
    /// input, a conjunction per expressed multi-input `Instinct` signature,
    /// and each link at its instinct's weight, or 0.
    pub(crate) fn new(genome: &Genome, data: &DataPack, stage: LifeStage) -> Brain {
        Brain::weakened(genome, data, stage, &[])
    }

    /// The same, each instinct's weight weakened to its gene's `health`
    /// (M2 design §9.4).
    pub(crate) fn weakened(
        genome: &Genome,
        data: &DataPack,
        stage: LifeStage,
        health: &[f32],
    ) -> Brain {
        Brain::weakened_as(genome, data, &expressions(genome, data, stage), health)
    }

    /// The same, from how each gene is expressed, in genome order (#210).
    fn weakened_as(
        genome: &Genome,
        data: &DataPack,
        expressed: &[Expression],
        health: &[f32],
    ) -> Brain {
        let inputs = data.brain_inputs_in_order();
        let place = |id| data.brain_input_index(id).expect("a checked gene");
        let mut concepts: Vec<Signature> = (0..inputs.len()).map(|i| vec![(i, false)]).collect();
        let mut decision = vec![vec![0.0; VERBS.len()]; inputs.len()];
        let mut attention = vec![vec![0.0; data.categories().len()]; inputs.len()];
        let mut whom = Vec::new();
        for (index, (gene, expression)) in genome.genes.iter().zip(expressed).enumerate() {
            if *expression != Expression::Expressed {
                continue;
            }
            let strength = health_of(health, index);
            match *gene {
                Gene::Instinct {
                    ref inputs,
                    verb,
                    weight,
                } => {
                    let signature = signature_of(inputs, data);
                    let concept = match concepts.iter().position(|c| *c == signature) {
                        Some(concept) => concept,
                        None => {
                            concepts.push(signature);
                            decision.push(vec![0.0; VERBS.len()]);
                            concepts.len() - 1
                        }
                    };
                    decision[concept][column(verb)] = weight * strength;
                }
                Gene::AttentionInstinct {
                    input,
                    category,
                    whom: Whom::Any,
                    weight,
                } => attention[place(input)][data.category_index(category)] = weight * strength,
                Gene::AttentionInstinct {
                    input,
                    category,
                    whom: about,
                    weight,
                } => whom.push(WhomLink {
                    input: place(input),
                    category: data.category_index(category),
                    whom: about,
                    weight: (weight * strength).clamp(-1.0, 1.0),
                }),
                _ => {}
            }
        }
        Brain {
            params: BrainParams::express_as(genome, data, expressed),
            concepts,
            decision: Links::new(decision),
            attention: Links::new(attention),
            whom,
            attended: None,
            snapshot: None,
            felt: 0.0,
            trace: VecDeque::new(),
            experience: Experience::default(),
            touched: None,
            reach_back: None,
            seen_cursor: false,
            looked: None,
        }
    }

    /// Sets the instinct links of `genes`, each given with how it's
    /// expressed and its health now, to their weakened weights (M2 design
    /// §9.4), as `weakened` would. Only for genes that have weakened since
    /// the brain was expressed, at the same stage: weakening changes no
    /// parameter or concept, so the rest of the brain stays as it is (#210).
    /// That includes its snapshot of the last step 5, which still fits, so
    /// at step 4 it still watches from where its eye was then, and shows a
    /// sprite watching it what it chose its try for (§18.1, §18.2).
    pub(crate) fn reweaken<'a>(
        &mut self,
        genes: impl IntoIterator<Item = (&'a Gene, &'a Expression, f32)>,
        data: &DataPack,
    ) {
        let place = |id| data.brain_input_index(id).expect("a checked gene");
        for (gene, expression, strength) in genes {
            if *expression != Expression::Expressed {
                continue;
            }
            // Each link is one expressed gene's: a gene that would set it
            // again isn't expressed (design §4.3).
            match *gene {
                Gene::Instinct {
                    ref inputs,
                    verb,
                    weight,
                } => {
                    let signature = signature_of(inputs, data);
                    let concept = self
                        .concepts
                        .iter()
                        .position(|c| *c == signature)
                        .expect("an expressed instinct's concept");
                    self.decision
                        .set_clamped(concept, column(verb), weight * strength);
                }
                Gene::AttentionInstinct {
                    input,
                    category,
                    whom: Whom::Any,
                    weight,
                } => self.attention.set_clamped(
                    place(input),
                    data.category_index(category),
                    weight * strength,
                ),
                Gene::AttentionInstinct {
                    input,
                    category,
                    whom: about,
                    weight,
                } => {
                    let (input, category) = (place(input), data.category_index(category));
                    let link = self
                        .whom
                        .iter_mut()
                        .find(|l| l.input == input && l.category == category && l.whom == about)
                        .expect("an expressed attention instinct's link");
                    link.weight = (weight * strength).clamp(-1.0, 1.0);
                }
                _ => {}
            }
        }
    }

    /// Whether its parameters, concepts and instinct links are `other`'s.
    pub(crate) fn instincts_match(&self, other: &Brain) -> bool {
        self.params == other.params
            && self.concepts == other.concepts
            && self.decision == other.decision
            && self.attention == other.attention
            && self.whom == other.whom
    }

    /// Expresses `genome` afresh for a life stage (M2 design §9.2), each
    /// gene as `expressed` at that stage, in genome order: its parameters,
    /// concepts and instinct links become those of its genes working then,
    /// as weakened as their `health` says (§9.4). What it has learned,
    /// attends to and remembers doing stays as it was; the snapshot, whose
    /// concepts may be gone, is taken again at step 5.
    pub(crate) fn reach(
        &mut self,
        genome: &Genome,
        data: &DataPack,
        expressed: &[Expression],
        health: &[f32],
    ) {
        let grown = Brain::weakened_as(genome, data, expressed, health);
        self.params = grown.params;
        self.concepts = grown.concepts;
        self.decision = grown.decision;
        self.attention = grown.attention;
        self.whom = grown.whom;
        self.snapshot = None;
    }

    /// `learn_with_moment`'s lessons alone.
    #[cfg(test)]
    pub(crate) fn learn(
        &mut self,
        tick: u64,
        signals: &Signals,
        learning_rate_mod: f32,
        data: &DataPack,
    ) -> Vec<(Learned, bool)> {
        self.learn_with_moment(tick, signals, learning_rate_mod, data)
            .0
    }

    /// Step 4 at `tick` (design §2.4, §5.6), from `signals`, every rate
    /// scaled by `learning_rate_mod` and its plasticity (M2 design §9.5). Instinct links never change. Each
    /// need's relief teaches what the thing touched is worth for it. Returns
    /// the lessons it learned, and what its moment was about, which a sprite
    /// watching it can take on (M2 design §18.2).
    pub(crate) fn learn_with_moment(
        &mut self,
        tick: u64,
        signals: &Signals,
        learning_rate_mod: f32,
        data: &DataPack,
    ) -> (Vec<(Learned, bool)>, Vec<Topic>) {
        let physiology = data.physiology();
        let relief = &signals.relief;
        let learning_rate_mod = learning_rate_mod * self.params.get(BrainParam::Plasticity);
        // A hurt done to it teaches fear of whoever did it (design v18 §5.6).
        let hitter = signals.attacker.filter(|_| signals.hit);
        if let Some(attacker) = hitter {
            let rate = self.params.get(BrainParam::FearRate) * learning_rate_mod;
            let attacker = self.experience.remember(attacker, relief.len());
            attacker.fear = (attacker.fear - rate * signals.punishment).max(-1.0);
        }
        // What the feelings are about: the latest try, while it's recent
        // (design §5.6). Relief and punishment look back the touch window, a
        // zap's or shock's too; reward, in a tick the Cursor rewarded the
        // sprite, looks back the Reward's reach back (design v21 §5.6). The
        // reach back is never shorter than the touch window, so whatever is
        // near is reached too.
        let within = |ticks: u64| self.touched.filter(|t| tick - t.tick <= ticks);
        let near = within(physiology.touch_window);
        let reached = signals.reach_back.map_or(near, within);
        // A Cursor it can see is part of the lesson (design v29 §5.6): a pet
        // teaches it to like the Cursor, and a zap, a shock or a crash after
        // its shove to fear it, as a hit teaches fear of the hitter, as well
        // as what each teaches about what it touched. A pet for a try on the
        // Cursor itself is the lesson about the thing touched, below, and a
        // crash teaches only in its own tick. In a tick the Cursor corrected
        // it, the pain is the correction's, so it's about the Cursor only if
        // the sprite could see it.
        let touched_the_cursor =
            reached.is_some_and(|touch| touch.subject.category(data) == data.cursor_category());
        let liked = signals.seen_cursor && signals.reach_back.is_some() && !touched_the_cursor;
        let feared = if signals.corrected {
            signals.seen_cursor
        } else {
            near.is_some_and(|touch| touch.by_cursor && touch.tick == tick)
        };
        if liked || feared {
            let good = self.params.get(BrainParam::IndividualRateGood) * learning_rate_mod;
            let fear = self.params.get(BrainParam::FearRate) * learning_rate_mod;
            let cursor = self.experience.remember_the_cursor(relief.len());
            if liked {
                cursor.good = (cursor.good + good * signals.reward).min(1.0);
            }
            if feared {
                cursor.fear = (cursor.fear - fear * signals.punishment).max(-1.0);
            }
        }
        // A feared Cursor that stays near and does nothing wears the fear
        // off, fastest on the sprite's own tile: it comes to see it can't do
        // anything about it (design v29 §5.6).
        if !signals.corrected
            && !feared
            && let (Some(distance), Some(cursor)) =
                (signals.cursor_distance, self.experience.cursor.as_mut())
        {
            let reach = self.params.get(BrainParam::FearReach);
            let nearness = (1.0 - distance / reach).max(0.0);
            cursor.fear *= 1.0 - self.params.get(BrainParam::CursorCalming) * nearness;
        }
        let needs = data.need_places().len();
        let mut eased_at = None;
        if let Some(Touch {
            subject,
            sprite,
            novelty,
            ..
        }) = reached
        {
            let good = self.params.get(BrainParam::WorthRateGood) * learning_rate_mod;
            let bad = self.params.get(BrainParam::WorthRateBad) * learning_rate_mod;
            let relief: &[f32] = if near.is_some() { relief } else { &[] };
            let reward = if reached.is_some() {
                signals.reward
            } else {
                0.0
            };
            // A hit's punishment teaches fear and habits, not worth (design
            // v18 §5.6).
            let punishment = if near.is_some() && !signals.hit {
                signals.punishment
            } else {
                0.0
            };
            // A sprite is learned about as that one sprite, fast; sprites in
            // general only through the ones it knows (design v18 §5.6). So is
            // the Cursor, the one there is (design v29 §5.6).
            let individual = match sprite {
                Some(sprite) => Some(self.experience.remember(sprite, needs)),
                None if subject.category(data) == data.cursor_category() => {
                    Some(self.experience.remember_the_cursor(needs))
                }
                None => None,
            };
            match individual {
                Some(known) => {
                    let good = self.params.get(BrainParam::IndividualRateGood) * learning_rate_mod;
                    let bad = self.params.get(BrainParam::IndividualRateBad) * learning_rate_mod;
                    for (worth, &relief) in known.worth.iter_mut().zip(relief) {
                        *worth = (*worth + good * relief).min(1.0);
                    }
                    known.good = (known.good + good * reward).min(1.0);
                    known.bad = (known.bad - bad * punishment).max(-1.0);
                }
                // Anything else is learned about as its object type (design
                // v19 §5.6).
                None => {
                    let known = self.experience.learn_about(subject, needs);
                    for (worth, &relief) in known.worth.iter_mut().zip(relief) {
                        *worth = (*worth + good * relief).min(1.0);
                    }
                    known.good = (known.good + good * reward).min(1.0);
                    known.bad = (known.bad - bad * punishment).max(-1.0);
                }
            }
            // How it went with something new teaches the worth of new things,
            // but the breeding drive eased teaches only about the partner (M2
            // design §5).
            let drive = physiology.indices.drive;
            let relieved: f32 = relief
                .iter()
                .enumerate()
                .filter(|&(need, _)| Some(need) != drive)
                .map(|(_, relief)| relief)
                .sum();
            // Relief from something that stays put makes where it is a
            // remembered place (M2 design §7), and relief of a need other
            // than the drive makes it part of its moment (§18.2, §18.3).
            if let Some(Touch {
                place: Some((target, at)),
                ..
            }) = near
            {
                if relief.iter().any(|&relief| relief > 0.0) {
                    let limits = physiology.remembered_places;
                    self.experience.remember_place(subject, target, at, limits);
                }
                if relieved > 0.0 {
                    eased_at = Some(Topic::Place {
                        subject,
                        target,
                        at,
                    });
                }
            }
            let experience = &mut self.experience;
            let felt = good * (relieved + reward) - bad * punishment;
            experience.new_things = (experience.new_things + felt * novelty).clamp(-1.0, 1.0);
        }
        // Habits, along the trace: each aimed verb and the thing it attended.
        // A fruitless try disappoints the need that chose it. In a tick the
        // Cursor rewarded the sprite, the reward goes instead to the taught
        // habit of the latest try within the Reward's reach back, at
        // `taught_rate` and full weight; in a tick it corrected it, the
        // punishment likewise to the latest try within the touch window. The
        // taught habit is kept apart so what the sprite learns itself doesn't
        // wash it out (design §5.6, #187).
        let rate = self.params.get(BrainParam::HabitRate) * learning_rate_mod;
        let taught_rate = self.params.get(BrainParam::TaughtRate) * learning_rate_mod;
        let rewarded = reached.filter(|_| signals.reach_back.is_some());
        let corrected = near.filter(|_| signals.corrected);
        let mut r = 0.0;
        for (tried_on, felt) in [(rewarded, signals.reward), (corrected, -signals.punishment)] {
            match tried_on {
                // A crash has no verb, so teaches no habit (design v23 §5.6).
                Some(Touch { verb: None, .. }) => {}
                Some(Touch {
                    verb: Some(verb),
                    subject,
                    ..
                }) => {
                    let known = self.experience.learn_about(subject, needs);
                    let habit = &mut known.taught_habits[column(verb)];
                    *habit = (*habit + taught_rate * felt).clamp(-1.0, 1.0);
                }
                None => r += felt,
            }
        }
        // While a crash is the thing touched, the trace teaches nothing
        // either: it holds what the sprite chose before it was taken hold
        // of, which the crash's pain isn't about (design v25 §5.6).
        let crashed = near.is_some_and(|touch| touch.verb.is_none());
        let tried = self.touched.filter(|_| signals.fruitless).map(|t| t.tick);
        let trace_teaches = !crashed && (r != 0.0 || tried.is_some());
        if trace_teaches {
            let disappointment = self.params.get(BrainParam::Disappointment);
            let decay = self.params.get(BrainParam::TraceDecay);
            for entry in &self.trace {
                let (Some(verb), Some(subject)) = (entry.verb, entry.subject) else {
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
                let known = self.experience.learn_about(subject, needs);
                let habit = &mut known.habits[column(verb)];
                *habit = (*habit + step).clamp(-1.0, 1.0);
            }
        }
        // What its moment was about (M2 design §18.2): the thing it tried at
        // its latest step 6, the try the Cursor rewarded or corrected, the
        // sprite that hit it, the Cursor it learned to fear, and where it
        // was eased; never a try at mating or courting (§18.3).
        let mut moment = Vec::new();
        let tries = [self.latest_try(tick), rewarded, corrected];
        for topic in tries.into_iter().flatten().filter_map(Topic::touched) {
            if !moment.contains(&topic) {
                moment.push(topic);
            }
        }
        if let Some(attacker) = hitter {
            moment.push(Topic::Hitter(attacker));
        }
        if feared {
            moment.push(Topic::Cursor);
        }
        moment.extend(eased_at);
        // A sprite touched that taught it nothing isn't remembered, so it
        // doesn't count towards sprites in general (design v18 §5.6).
        self.experience.forget_faded(physiology.forget_below);
        // Only learning can tip a lesson, and between ticks only fading,
        // which brings every learned value nearer 0 while each fade and the
        // Cursor's calming stay from 0 to 1, as the built-in pack's ranges
        // keep them, but for two things: a summary whose sprites, object types
        // or generalising changed, and a learned habit and a taught one
        // pulling opposite ways, as one fades faster than the other. So
        // with none of these there's no lesson to look for. Every kind of
        // learning above is counted here.
        let learned_anything = hitter.is_some()
            || liked
            || feared
            || signals.reward != 0.0
            || signals.punishment != 0.0
            || relief.iter().any(|&v| v != 0.0)
            || trace_teaches;
        let lessons = if learned_anything
            || self.looked != Some(self.summary_basis(data))
            || self.habits_pull_apart()
        {
            self.lessons(data)
        } else {
            Vec::new()
        };
        self.fade(physiology.forget_below);
        (lessons, moment)
    }

    /// What it knows of `topic`, of the values watching passes on (M2
    /// design §18.2). Of a thing, its worth for each need but the breeding
    /// drive's (§18.3), good and bad; of a sprite or the Cursor, its fear;
    /// and of either, the habit of the verb tried. Its badness and habit
    /// are what its decisions weigh, watched parts and all (§19.7).
    pub(crate) fn knowledge(&self, topic: Topic, data: &DataPack) -> Knowledge {
        self.knows(topic, data, false)
    }

    /// What it knows of `topic`, as `knowledge` says; at `first_hand`, its
    /// badness and habit only as it found them out itself and the Cursor
    /// taught it, without what it took on by watching (M2 design §19.7).
    fn knows(&self, topic: Topic, data: &DataPack, first_hand: bool) -> Knowledge {
        let experience = &self.experience;
        let mut knowledge = Knowledge::nothing(topic, data.need_places().len());
        let fear_of = |sprite: Option<EntityId>| {
            let known = match sprite {
                Some(sprite) => experience.individuals.get(&sprite),
                None => experience.cursor.as_ref(),
            };
            known.map_or(0.0, |known| known.fear)
        };
        match topic {
            Topic::Tried {
                subject,
                verb,
                sprite,
            } => {
                let known = experience.types.get(&subject);
                let i = column(verb);
                knowledge.habit = match known {
                    Some(known) if first_hand => known.first_hand_habit(i),
                    Some(known) => known.habit(i),
                    None => 0.0,
                };
                match (TriedAt::of(subject, sprite, data), known) {
                    (TriedAt::Feared(whom), _) => knowledge.fear = fear_of(whom),
                    (TriedAt::Thing, Some(known)) => {
                        knowledge.worth.clone_from(&known.worth);
                        if let Some(drive) = data.physiology().indices.drive {
                            knowledge.worth[drive] = 0.0;
                        }
                        knowledge.good = known.good;
                        knowledge.bad = if first_hand {
                            known.bad
                        } else {
                            known.badness()
                        };
                    }
                    (TriedAt::Thing, None) => {}
                }
            }
            Topic::Hitter(sprite) => knowledge.fear = fear_of(Some(sprite)),
            Topic::Cursor => knowledge.fear = fear_of(None),
            Topic::Place { .. } => {}
        }
        knowledge
    }

    /// What a sprite watching it make the try `topic` takes on (M2 design
    /// §18.2): what it knows of it, but of a thing's worth only that for
    /// the try's motive, the need that chose it (design §5.5), and of its
    /// badness and habit only what it found out itself or the Cursor taught
    /// it, so what's taken on by watching is never passed on (§19.7). A try
    /// no need chose, or that its brain didn't choose, as a scripted one,
    /// shows no worth.
    pub(crate) fn shown(&self, topic: Topic, data: &DataPack) -> Knowledge {
        let mut knowledge = self.knows(topic, data, true);
        if let Topic::Tried { subject, verb, .. } = topic {
            let motive = self.motive_for(verb, subject);
            for (need, worth) in knowledge.worth.iter_mut().enumerate() {
                if Some(need) != motive {
                    *worth = 0.0;
                }
            }
        }
        knowledge
    }

    /// The motive of a try with `verb` at `subject` (design §5.5), if that's
    /// what it chose at its latest step 5.
    fn motive_for(&self, verb: Verb, subject: Subject) -> Option<usize> {
        self.snapshot
            .as_ref()
            .filter(|s| s.verb == Some(verb) && s.subject == Some(subject))
            .and_then(|s| s.motive)
    }

    /// Its try at its latest step 6, from step 4 at `tick`.
    fn latest_try(&self, tick: u64) -> Option<Touch> {
        self.touched.filter(|touch| touch.tick + 1 == tick)
    }

    /// Whether a try it's making, about `topic`, shows `learner`, the
    /// sprite `id`, something it hasn't learned (M2 design §18.1): a value
    /// `learner` would take on by watching it that's `lesson_threshold` or
    /// more from 0 for this sprite, and not yet for `learner`, on the same
    /// side or at 0. Never a try on `learner` itself.
    pub(crate) fn shows(
        &self,
        topic: Topic,
        learner: &Brain,
        id: EntityId,
        data: &DataPack,
    ) -> bool {
        if topic.about(id) {
            return false;
        }
        let threshold = data.physiology().lesson_threshold;
        let theirs = self.shown(topic, data);
        let mine = learner.knowledge(topic, data);
        theirs.values().zip(mine.values()).any(|(theirs, mine)| {
            theirs.abs() >= threshold && mine.abs() < threshold && mine * theirs >= 0.0
        })
    }

    /// Takes on `share` of each of `shown`, what a sprite it watched showed
    /// it (M2 design §18.2), and returns the lessons that tipped. Of a
    /// thing, its worth, good and bad, and it knows the thing's
    /// object type; of a sprite or the Cursor, only fear. Its watched
    /// badness and the watched habit of the verb tried, kept apart, settle
    /// `share` of the way towards what was shown, either way (§19.7); and a
    /// place is remembered.
    pub(crate) fn watched(
        &mut self,
        shown: &[Knowledge],
        share: f32,
        data: &DataPack,
    ) -> Vec<(Learned, bool)> {
        let physiology = data.physiology();
        let needs = data.need_places().len();
        for theirs in shown {
            let experience = &mut self.experience;
            match theirs.topic {
                Topic::Tried {
                    subject,
                    verb,
                    sprite,
                } => {
                    match TriedAt::of(subject, sprite, data) {
                        TriedAt::Feared(whom) => {
                            experience.fear_more(whom, theirs.fear, share, needs)
                        }
                        // Having taken something on about it, it knows the
                        // thing's object type, so its own view counts rather
                        // than its category's summary.
                        TriedAt::Thing => {
                            let known = experience.types.get(&subject).cloned();
                            let mut known = known.unwrap_or_else(|| TypeMemory::new(needs));
                            let mut moved = false;
                            let mut take_on = |mine: &mut f32, theirs: f32| {
                                let to = toward(*mine, theirs, share);
                                moved |= to != *mine;
                                *mine = to;
                            };
                            for (mine, &their) in known.worth.iter_mut().zip(&theirs.worth) {
                                take_on(mine, their);
                            }
                            take_on(&mut known.good, theirs.good);
                            // Kept apart, settling either way (§19.7).
                            let to = settled(known.watched_bad, theirs.bad, share);
                            moved |= to != known.watched_bad;
                            known.watched_bad = to;
                            if moved {
                                known.touched = true;
                                experience.types.insert(subject, known);
                            }
                        }
                    }
                    let i = column(verb);
                    let mine = experience
                        .types
                        .get(&subject)
                        .map_or(0.0, |k| k.watched_habits[i]);
                    let to = settled(mine, theirs.habit, share);
                    if to != mine {
                        experience.learn_about(subject, needs).watched_habits[i] = to;
                    }
                }
                Topic::Hitter(sprite) => {
                    experience.fear_more(Some(sprite), theirs.fear, share, needs)
                }
                Topic::Cursor => experience.fear_more(None, theirs.fear, share, needs),
                Topic::Place {
                    subject,
                    target,
                    at,
                } => experience.remember_place(subject, target, at, physiology.remembered_places),
            }
        }
        self.lessons(data)
    }

    /// The learned values that are `lesson_threshold` or more from 0 for the
    /// first time, as `(learned, rose)`, in listed order. Each is marked by
    /// what it's about, so it's a lesson only once (design §5.6), however
    /// the sprites it remembers come and go (design v18 §5.6).
    fn lessons(&mut self, data: &DataPack) -> Vec<(Learned, bool)> {
        self.looked = Some(self.summary_basis(data));
        let threshold = data.physiology().lesson_threshold;
        let mut found = Vec::new();
        let mut over = Vec::new();
        self.each_learned(data, |learned, amount| {
            if amount.abs() >= threshold {
                over.push((learned, amount));
            }
        });
        for (learned, amount) in over {
            let learned = learned.named(data);
            if self.experience.taught.insert(learned.clone()) {
                found.push((learned, amount > 0.0));
            }
        }
        found
    }

    /// What its summaries are made of now (design v18, v19 §5.6).
    fn summary_basis(&self, data: &DataPack) -> SummaryBasis {
        let below = data.physiology().forget_below;
        let experience = &self.experience;
        SummaryBasis {
            sprites: experience
                .individuals
                .values()
                .filter(|known| known.taught(below))
                .count(),
            types: experience
                .types
                .values()
                .filter(|known| known.touched)
                .count(),
            generalise: self.params.get(BrainParam::Generalise).to_bits(),
            generalise_types: self.params.get(BrainParam::GeneraliseTypes).to_bits(),
        }
    }

    /// Whether, of a habit it learned, one the Cursor taught and one it
    /// took on by watching (#187, M2 design §19.7), two pull opposite ways:
    /// fading at their own rates, their sum can move away from 0. What it
    /// found out and took on of how bad a thing is can't, both being at
    /// most 0.
    fn habits_pull_apart(&self) -> bool {
        self.experience.types.values().any(|known| {
            let apart = |a: f32, b: f32| a * b < 0.0;
            (0..VERBS.len()).any(|i| {
                let (learned, taught, watched) = (
                    known.habits[i],
                    known.taught_habits[i],
                    known.watched_habits[i],
                );
                apart(learned, taught) || apart(learned, watched) || apart(taught, watched)
            })
        })
    }

    /// One tick of fading (design §5.6): good and bad, fear, familiarity,
    /// habits and taught habits (#187) each by their own rate. A remembered
    /// sprite faded until everything about it is nearer 0 than
    /// `forget_below` is forgotten (design v18 §5.6), and so is a remembered
    /// place (M2 design §7).
    fn fade(&mut self, forget_below: f32) {
        let keep = |param| 1.0 - self.params.get(param);
        let (good, bad, habit, bad_habit, taught) = (
            keep(BrainParam::WorthFadeGood),
            keep(BrainParam::WorthFadeBad),
            keep(BrainParam::HabitFade),
            keep(BrainParam::HabitFadeBad),
            keep(BrainParam::TaughtFade),
        );
        let experience = &mut self.experience;
        for known in experience.types.values_mut() {
            for value in &mut known.worth {
                *value *= good;
            }
            known.good *= good;
            known.bad *= bad;
            // Bad habits fade at their own rate (design v21 §5.6).
            for value in &mut known.habits {
                *value *= if *value < 0.0 { bad_habit } else { habit };
            }
            for value in &mut known.taught_habits {
                *value *= taught;
            }
            // What it took on by watching fades as what it was taught,
            // and its badness as what it found out (M2 design §19.7).
            for value in &mut known.watched_habits {
                *value *= taught;
            }
            known.watched_bad *= bad;
        }
        experience.new_things *= good;
        let (fear, familiar) = (keep(BrainParam::FearFade), keep(BrainParam::BefriendFade));
        let individuals = experience.individuals.values_mut();
        for individual in individuals.chain(&mut experience.cursor) {
            for value in &mut individual.worth {
                *value *= good;
            }
            individual.good *= good;
            individual.bad *= bad;
            individual.fear *= fear;
            // Familiarity too (M2 design §15.2).
            individual.familiarity *= familiar;
        }
        experience.forget_faded(forget_below);
        // Remembered places fade too, and are forgotten once faint (M2
        // design §7).
        let place = keep(BrainParam::PlaceFade);
        for remembered in &mut experience.places {
            remembered.recall *= place;
        }
        experience.places.retain(|p| p.recall >= forget_below);
    }

    /// How new a thing learned about as `subject` is to the sprite (design
    /// §5.6): 1 − familiarity.
    pub(crate) fn novelty(&self, subject: Subject) -> f32 {
        let known = self.experience.types.get(&subject);
        1.0 - known.map_or(0.0, |known| known.familiarity)
    }

    /// Each need's relief this tick (design §5.6): its fall since the last
    /// step 4, if at least `relief_deadband`, else 0. Remembers `needs` for
    /// the next.
    pub(crate) fn relief(&mut self, needs: &[f32], data: &DataPack) -> Vec<f32> {
        let deadband = data.physiology().relief_deadband;
        let relief = match &self.experience.needs_before {
            Some(before) => before
                .iter()
                .zip(needs)
                .map(|(&was, &now)| was - now)
                .map(|fall| if fall >= deadband { fall } else { 0.0 })
                .collect(),
            None => vec![0.0; needs.len()],
        };
        self.experience.needs_before = Some(needs.to_vec());
        relief
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

    /// What the sprite has learned, furthest from nothing first (design v40
    /// §5.9). Ties keep the order they're listed in. The Brain tab groups
    /// them, and leaves out one that rounds to nothing.
    pub(crate) fn memory(&self, data: &DataPack) -> Vec<Memory> {
        let mut memory = self.learned(data);
        memory.retain(|m| m.amount != 0.0);
        // A stable sort keeps a tie in listed order.
        memory.sort_by(|a, b| b.amount.abs().total_cmp(&a.amount.abs()));
        memory
    }

    /// The places it remembers, best remembered first, ties oldest first
    /// (M2 design §7).
    pub(crate) fn remembered_places(&self, data: &DataPack) -> Vec<RememberedPlace> {
        let mut places: Vec<RememberedPlace> = self
            .experience
            .places
            .iter()
            .map(|place| RememberedPlace {
                thing: subject_thing(place.subject, data),
                at: place.at,
                recall: place.recall,
            })
            .collect();
        // A stable sort keeps a tie oldest first.
        places.sort_by(|a, b| b.recall.total_cmp(&a.recall));
        places
    }

    /// Every value a brain learns, named, always in the same order: worth
    /// for each need then general good, bad, habits, and new things, each in
    /// category order, and within a category in object type order (design
    /// §5.6).
    fn learned(&self, data: &DataPack) -> Vec<Memory> {
        let mut memory = Vec::new();
        self.each_learned(data, |learned, amount| {
            memory.push(Memory {
                learned: learned.named(data),
                amount,
            });
        });
        memory
    }

    /// `learned`, each value given to `visit` by ID, before it's put in
    /// words: the words take text, and most values are looked at only for
    /// their amount, every tick.
    fn each_learned(&self, data: &DataPack, mut visit: impl FnMut(LearnedKey, f32)) {
        // Sprites in general are the summary of the ones it knows (design
        // v18 §5.6), never learned directly.
        let in_general = self.sprites_in_general(data);
        let categories = data.categories();
        let summaries: BTreeMap<CategoryId, Summary> = categories
            .iter()
            .map(|category| (category.id, self.category_summary(category.id, data)))
            .collect();
        // Each thing's worth for each need, general good and bad.
        let mut things: Vec<(ThingKey, &[f32], f32, f32)> = Vec::new();
        for category in categories {
            if category.id == data.sprite_category() {
                let thing = ThingKey::Category(category.id);
                things.push((thing, &in_general.worth, in_general.good, in_general.bad));
                continue;
            }
            // There's one Cursor, learned about as an individual (design v29
            // §5.6), so the category has no summary.
            if category.id == data.cursor_category() {
                if let Some(known) = &self.experience.cursor {
                    let thing = ThingKey::Cursor;
                    things.push((thing, &known.worth, known.good, known.bad));
                }
                continue;
            }
            let types = self.experience.types.iter();
            for (&subject, known) in types.filter(|(s, _)| s.category(data) == category.id) {
                let thing = ThingKey::Subject(subject);
                things.push((thing, &known.worth, known.good, known.badness()));
            }
            // Then the category's summary, never learned directly (design
            // v19 §5.6).
            let summary = &summaries[&category.id];
            let thing = ThingKey::Category(category.id);
            things.push((thing, &summary.worth, summary.good, summary.bad));
        }
        for (need, &place) in data.need_places().iter().enumerate() {
            for &(thing, worth, ..) in &things {
                let key = LearnedKey::Worth {
                    thing,
                    need: Some(place),
                };
                visit(key, worth[need]);
            }
        }
        for &(thing, _, good, _) in &things {
            visit(LearnedKey::Worth { thing, need: None }, good);
        }
        for &(thing, _, _, bad) in &things {
            visit(LearnedKey::Bad { thing }, bad);
        }
        // Habits are about object types, sprites' too (design v19 §5.6).
        let habits = categories.iter().flat_map(|category| {
            let types = self.experience.types.iter();
            types.filter(move |(s, _)| s.category(data) == category.id)
        });
        for (&subject, known) in habits {
            for (i, &verb) in VERBS.iter().enumerate() {
                let thing = ThingKey::Subject(subject);
                visit(LearnedKey::Habit { thing, verb }, known.habit(i));
            }
        }
        visit(LearnedKey::NewThings, self.experience.new_things);
        // The sprites it remembers (design v18 §5.9), in ID order: not
        // those it knows only by sight (M2 design §15.2).
        let below = data.physiology().forget_below;
        let taught = self.experience.individuals.iter();
        for (&id, individual) in taught.filter(|(_, known)| known.taught(below)) {
            let thing = ThingKey::Sprite(id);
            for (&value, &place) in individual.worth.iter().zip(data.need_places()) {
                visit(
                    LearnedKey::Worth {
                        thing,
                        need: Some(place),
                    },
                    value,
                );
            }
            visit(LearnedKey::Worth { thing, need: None }, individual.good);
            visit(LearnedKey::Bad { thing }, individual.bad);
            visit(LearnedKey::Fear { thing }, individual.fear);
        }
        // Fear of sprites in general comes of the sprites above, so it's
        // listed, and a lesson, after them.
        let sprites = ThingKey::Category(data.sprite_category());
        visit(LearnedKey::Fear { thing: sprites }, in_general.fear);
        if let Some(cursor) = &self.experience.cursor {
            let thing = ThingKey::Cursor;
            visit(LearnedKey::Fear { thing }, cursor.fear);
        }
    }

    /// Sprites in general (design v18 §5.6): each value's mean over the
    /// sprites it has learned something of, not those it knows only by
    /// sight (M2 design §15.2), at no strength while it knows one, and in
    /// full once it knows `generalise`. What a partner is worth for the
    /// breeding drive stays with that partner, so no sprite it doesn't
    /// know, young or grown, is worth anything for it (M2 design §5).
    pub(crate) fn sprites_in_general(&self, data: &DataPack) -> SpriteMemory {
        let below = data.physiology().forget_below;
        let known: Vec<&SpriteMemory> = self
            .experience
            .individuals
            .values()
            .filter(|known| known.taught(below))
            .collect();
        let share = summary_share(known.len(), self.params.get(BrainParam::Generalise));
        let mut summary = SpriteMemory::new(data.need_places().len());
        if share == 0.0 {
            return summary;
        }
        let drive = data.physiology().indices.drive;
        for individual in known {
            for (need, (summary, worth)) in
                summary.worth.iter_mut().zip(&individual.worth).enumerate()
            {
                if Some(need) != drive {
                    *summary += share * worth;
                }
            }
            summary.good += share * individual.good;
            summary.bad += share * individual.bad;
            summary.fear += share * individual.fear;
        }
        summary
    }

    /// `category`'s summary (design v19 §5.6): each value's mean over the
    /// object types in it the sprite knows, at no strength while it knows
    /// one, and in full once it knows `generalise_types`.
    pub(crate) fn category_summary(&self, category: CategoryId, data: &DataPack) -> Summary {
        let known = || {
            self.experience
                .types
                .iter()
                .filter(|(subject, memory)| memory.touched && subject.category(data) == category)
                .map(|(_, memory)| memory)
        };
        let share = summary_share(
            known().count(),
            self.params.get(BrainParam::GeneraliseTypes),
        );
        let mut summary = Summary {
            worth: vec![0.0; data.need_places().len()],
            good: 0.0,
            bad: 0.0,
        };
        if share == 0.0 {
            return summary;
        }
        for known in known() {
            for (summary, worth) in summary.worth.iter_mut().zip(&known.worth) {
                *summary += share * worth;
            }
            summary.good += share * known.good;
            summary.bad += share * known.badness();
        }
        summary
    }

    /// Gives a brain from a save from before Mate its decision instincts'
    /// column for it, at 0 (M2 design §5).
    pub(crate) fn widen_verbs(&mut self) {
        self.decision.widen(VERBS.len());
    }

    /// Whether a loaded brain fits `genome` and `data` at the tick `now`
    /// (design §2.8): its parameters, concepts and instincts are the ones
    /// the genome gives, at `stage` and with its genes' `health`; whatever it attends to, has learned or remembers
    /// doing is about categories, object types and needs the pack has; and
    /// what it remembers doing was done by `now`.
    pub(crate) fn fits(
        &self,
        genome: &Genome,
        data: &DataPack,
        stage: LifeStage,
        health: &[f32],
        now: u64,
    ) -> bool {
        let born = Brain::weakened(genome, data, stage, health);
        let needs = data.needs().count();
        let category = |id: CategoryId| data.category(id).is_some();
        let subject = |subject: Subject| subject.fits(data);
        let motive = |motive: Option<usize>| motive.is_none_or(|need| need < needs);
        let scored = |(&id, &thing): (&CategoryId, &Subject)| category(id) && subject(thing);
        let snapshot = |snapshot: &Snapshot| {
            snapshot.tick <= now
                && snapshot.inputs.len() == data.brain_inputs_in_order().len()
                && snapshot.activations.len() == self.concepts.len()
                && snapshot.attention.keys().all(|&id| category(id))
                && snapshot.scored.iter().all(scored)
                && snapshot.attended.is_none_or(category)
                && snapshot.subject.is_none_or(subject)
                && motive(snapshot.motive)
        };
        let traced = |entry: &TraceEntry| {
            entry.tick <= now && entry.subject.is_none_or(subject) && motive(entry.motive)
        };
        let touched = |touch: Touch| touch.tick <= now && subject(touch.subject);
        self.instincts_match(&born)
            && self.attended.is_none_or(category)
            && self.snapshot.as_ref().is_none_or(snapshot)
            && self.trace.iter().all(traced)
            && self.experience.fits(data)
            && self.touched.is_none_or(touched)
    }

    /// Checks the brain's state (design §5.6): instinct links within
    /// [−1, 1], what it has learned within its ranges, a felt value that's a
    /// number, and a trace within its cap.
    pub(crate) fn check(&self) -> Result<(), String> {
        self.decision.check()?;
        self.attention.check()?;
        if let Some(link) = self.whom.iter().find(|l| !(-1.0..=1.0).contains(&l.weight)) {
            return Err(format!("a link at {}, outside -1 to 1", link.weight));
        }
        self.experience.check()?;
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
    pub(crate) fn commit(&mut self, tick: u64, data: &DataPack) {
        if let Some(snapshot) = self.snapshot.as_ref().filter(|s| s.tick == tick) {
            let motive = snapshot.motive;
            if let Some(subject) = snapshot.subject {
                // Attending to an object type makes it familiar (design v19
                // §5.6).
                let rate = self.params.get(BrainParam::FamiliarityRate);
                let known = self
                    .experience
                    .learn_about(subject, data.need_places().len());
                known.familiarity = (known.familiarity + rate).min(1.0);
            }
            self.trace.push_back(TraceEntry {
                motive,
                tick,
                verb: snapshot.verb,
                subject: snapshot.subject,
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
    pub(crate) fn motive(&self, verb: Verb, activations: &[f32], data: &DataPack) -> Option<usize> {
        let v = column(verb);
        // The singletons come first, one per input, so a need's place in the
        // inputs is its singleton's place in the concepts.
        let mut pushes: Vec<(usize, usize, f32)> = data
            .need_places()
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
        // Each row is named by what was scored: its object type, or for
        // sprites the one scored (design v19 §5.9).
        let seen = |subject: Subject| thing_named(subject, snapshot.sprite_seen, data);
        let mut attention: Vec<(Thing, f32)> = snapshot
            .attention
            .iter()
            .map(|(category, &score)| (seen(snapshot.scored[category]), score))
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
                    part: Part::Concept(
                        signature
                            .iter()
                            .map(|&(i, negated)| (names[i].name.as_str(), negated))
                            .collect(),
                    ),
                    amount: activation * links[column(verb)],
                })
                .collect(),
            None => Vec::new(),
        };
        if let (Some((verb, _)), Some(subject)) = (decision, snapshot.subject) {
            let scoring = snapshot.scoring;
            let general = self.sprites_in_general(data);
            let parts = self.aimed_parts(subject, scoring, &snapshot.inputs, &general, data);
            let [worth, fear, habit] = parts[column(verb)];
            let thing = thing_named(subject, scoring.sprite, data);
            contributions.extend([
                Contribution {
                    part: Part::Worth(thing.clone()),
                    amount: worth,
                },
                Contribution {
                    part: Part::Fear(thing),
                    amount: fear,
                },
                Contribution {
                    part: Part::Habit(subject_thing(subject, data)),
                    amount: habit,
                },
            ]);
        }
        contributions.retain(|c| c.amount != 0.0);
        // A stable sort keeps a tie in listed order: concepts, worth, fear,
        // habit.
        contributions.sort_by(|a, b| b.amount.abs().total_cmp(&a.amount.abs()));
        Some(Explanation {
            attention,
            attended: snapshot.subject.map(seen),
            decision,
            contributions,
        })
    }

    /// Every input's value (design §5.2), in the pack's input order: State
    /// inputs from `body`, Target inputs from `aim`, and what the sprite it
    /// attends to shows, and what it knows of that sprite, from `looks` (M2
    /// design §10, §15), or 0 if it attends to none.
    pub(crate) fn inputs(
        &self,
        body: &Body,
        aim: Option<Aim>,
        looks: Option<&Looks>,
        data: &DataPack,
    ) -> Vec<f32> {
        let flag = |on: bool| if on { 1.0 } else { 0.0 };
        let mut shown = looks.map_or(&[][..], |looks| &looks.body).iter();
        data.brain_inputs_in_order()
            .iter()
            .map(|input| match input.source {
                Source::State(locus) => read(body, locus, data),
                Source::Shown(_) => shown.next().copied().unwrap_or(0.0),
                Source::Differs(locus) => shown
                    .next()
                    .map_or(0.0, |&theirs| (theirs - read(body, locus, data)).abs()),
                Source::Familiar => looks.map_or(0.0, |looks| looks.familiar),
                Source::Partner => flag(looks.is_some_and(|looks| looks.partner)),
                Source::Kin => flag(looks.is_some_and(|looks| looks.kin)),
                Source::Family => flag(aim.is_some_and(|a| a.relation.family)),
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

    /// `attention_scores_with`, working out sprites in general itself.
    #[cfg(test)]
    pub(crate) fn attention_scores(
        &self,
        inputs: &[f32],
        candidates: &BTreeMap<CategoryId, Seen>,
        curiosity_mod: f32,
        scoring: SpriteScoring,
        data: &DataPack,
    ) -> BTreeMap<CategoryId, f32> {
        let general = self.sprites_in_general(data);
        self.attention_scores_with(inputs, candidates, curiosity_mod, scoring, &general, data)
    }

    /// Each candidate category's attention score (design §5.3): the State
    /// inputs through the attention links, plus salience for nearness.
    /// `candidates` gives what each category's candidate is learned about as,
    /// its normalized distance, and how much of its worth counts: a
    /// remembered place's, as much as it's remembered (M2 design §7). Target
    /// inputs never count, whatever `inputs` holds for them. `general` is
    /// its sprites in general (`sprites_in_general`), worked out once a tick.
    pub(crate) fn attention_scores_with(
        &self,
        inputs: &[f32],
        candidates: &BTreeMap<CategoryId, Seen>,
        curiosity_mod: f32,
        scoring: SpriteScoring,
        general: &SpriteMemory,
        data: &DataPack,
    ) -> BTreeMap<CategoryId, f32> {
        let state = data.state_places();
        let salience = self.params.get(BrainParam::SalienceGain);
        let value_gain = self.params.get(BrainParam::ValueGain);
        let curiosity = self.params.get(BrainParam::Curiosity);
        let vigilance = self.params.get(BrainParam::Vigilance);
        let boldness = self.boldness(curiosity_mod);
        let drive = data.drive_places();
        candidates
            .iter()
            .map(|(&category, &seen)| {
                let Seen {
                    subject,
                    distance,
                    recall,
                    not_grown,
                    relation,
                } = seen;
                let c = data.category_index(category);
                // The breeding drive never draws the eye to a sprite not
                // grown up (M2 design §5).
                let instinct: f32 = state
                    .iter()
                    .filter(|i| !(not_grown && drive.contains(i)))
                    .map(|&i| inputs[i] * self.attention.get(i, c))
                    .sum::<f32>()
                    + self.whom_pull(c, relation, not_grown, inputs, data);
                let worth = recall
                    * value_gain
                    * self.worth_of(subject, scoring.sprite, inputs, general, data);
                let curious = curiosity * self.novelty(subject) * boldness;
                // Fear catches the eye (design v18 §5.3).
                let watchful = vigilance * self.fright(category, scoring, distance, general, data);
                (
                    category,
                    instinct + salience * (1.0 - distance) + worth + curious + watchful,
                )
            })
            .collect()
    }

    /// How much a thing `seen`, the particular `sprite` if it's one, draws
    /// the eye beyond the instincts about its whole category (design v19
    /// §3.6, §5.3): the instincts about whom it is to the sprite (M2 design
    /// §15.6), its nearness, what it's worth, as much as it's recalled (§7),
    /// how new it is, and, for a sprite, how frightening it is while near.
    /// `inputs` gives the State inputs, `curiosity_mod` the receptor target
    /// wariness lowers, and `general` its sprites in general
    /// (`sprites_in_general`), worked out once for all it sees.
    pub(crate) fn draw(
        &self,
        seen: Seen,
        sprite: Option<EntityId>,
        inputs: &[f32],
        curiosity_mod: f32,
        general: &SpriteMemory,
        data: &DataPack,
    ) -> f32 {
        let scoring = SpriteScoring {
            sprite,
            quiet: false,
        };
        let Seen {
            subject,
            distance,
            recall,
            not_grown,
            relation,
        } = seen;
        let boldness = self.boldness(curiosity_mod);
        let worth = self.worth_of(subject, sprite, inputs, general, data);
        let category = data.category_index(subject.category(data));
        self.whom_pull(category, relation, not_grown, inputs, data)
            + self.params.get(BrainParam::SalienceGain) * (1.0 - distance)
            + self.params.get(BrainParam::ValueGain) * recall * worth
            + self.params.get(BrainParam::Curiosity) * self.novelty(subject) * boldness
            + self.params.get(BrainParam::Vigilance)
                * self.fright(subject.category(data), scoring, distance, general, data)
    }

    /// How much the attention instincts about whom draw the eye to a thing of
    /// the category at index `category` that's `relation` to the sprite (M2
    /// design §15.6): the State inputs through the links about its family,
    /// if it's family, and about its kin, if it's kin. The breeding drive
    /// never draws the eye to one `not_grown` (§5).
    fn whom_pull(
        &self,
        category: usize,
        relation: Relation,
        not_grown: bool,
        inputs: &[f32],
        data: &DataPack,
    ) -> f32 {
        if self.whom.is_empty() {
            return 0.0;
        }
        let drive = data.drive_places();
        self.whom
            .iter()
            .filter(|link| link.category == category && relation.is(link.whom))
            .filter(|link| !(not_grown && drive.contains(&link.input)))
            .map(|link| inputs[link.input] * link.weight)
            .sum()
    }

    /// How boldly the unfamiliar draws the eye (design §5.3): the worth of
    /// new things, lifted by 1 and never below 0, times `curiosity_mod`, the
    /// receptor target wariness lowers.
    fn boldness(&self, curiosity_mod: f32) -> f32 {
        (1.0 + self.experience.new_things).max(0.0) * curiosity_mod
    }

    /// How frightening `category`'s thing is at normalized `distance` (design
    /// v18 §5.3), from 0 to 1: for sprites, the one in `scoring`, or the
    /// Cursor (design v29 §5.3), fading to nothing at `fear_reach`, and
    /// nothing while fear is quiet; nothing else is feared in M1.
    fn fright(
        &self,
        category: CategoryId,
        scoring: SpriteScoring,
        distance: f32,
        general: &SpriteMemory,
        data: &DataPack,
    ) -> f32 {
        if scoring.quiet {
            return 0.0;
        }
        let fear = if category == data.sprite_category() {
            self.memory_of(scoring.sprite, general, data).fear
        } else if category == data.cursor_category() {
            self.experience.cursor.as_ref().map_or(0.0, |c| c.fear)
        } else {
            return 0.0;
        };
        let reach = self.params.get(BrainParam::FearReach);
        -fear * (1.0 - distance / reach).max(0.0)
    }

    /// What the brain knows of `sprite` (design v18 §5.6): what it has
    /// learned of it, or `general`, its sprites in general, for one it
    /// hasn't, knowing it only by sight (M2 design §15.2) or not at all, or
    /// none.
    fn memory_of<'a>(
        &'a self,
        sprite: Option<EntityId>,
        general: &'a SpriteMemory,
        data: &DataPack,
    ) -> &'a SpriteMemory {
        let below = data.physiology().forget_below;
        sprite
            .and_then(|sprite| self.experience.individuals.get(&sprite))
            .filter(|known| known.taught(below))
            .unwrap_or(general)
    }

    /// Where attention goes (design §5.3). Choosing afresh, with no action
    /// running, it samples a category from softmax(score / τ_att), one draw.
    /// With an action running it draws nothing: it stays unless a rival
    /// beats it by `attention_margin`, and then goes to the best rival, ties
    /// to the lower category; with nothing attended, to the best candidate.
    pub(crate) fn attend(
        &mut self,
        scores: &BTreeMap<CategoryId, f32>,
        running: bool,
        exploration: f32,
        rng: &mut ChaCha8Rng,
    ) -> Option<CategoryId> {
        if scores.is_empty() {
            self.attended = None;
        } else if !running {
            let tau = self.params.get(BrainParam::TauAttBase) * exploration;
            let categories: Vec<CategoryId> = scores.keys().copied().collect();
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

    /// `scores_with`, working out sprites in general itself.
    #[cfg(test)]
    pub(crate) fn scores(
        &self,
        activations: &[f32],
        inputs: &[f32],
        aimed: Option<Subject>,
        scoring: SpriteScoring,
        data: &DataPack,
    ) -> [f32; VERBS.len()] {
        let general = self.sprites_in_general(data);
        self.scores_with(activations, inputs, aimed, scoring, &general, data)
    }

    /// Each verb's score (design §5.5), in `VERBS` order: the concepts'
    /// activations through the instinct links, and for the thing `aimed`
    /// at, learned about as that subject, its worth, which draws the sprite
    /// to it or makes it back away. `general` is its sprites in general
    /// (`sprites_in_general`), worked out once a tick.
    pub(crate) fn scores_with(
        &self,
        activations: &[f32],
        inputs: &[f32],
        aimed: Option<Subject>,
        scoring: SpriteScoring,
        general: &SpriteMemory,
        data: &DataPack,
    ) -> [f32; VERBS.len()] {
        let mut scores = [0.0; VERBS.len()];
        for (a, links) in activations.iter().zip(self.decision.rows()) {
            for (score, w) in scores.iter_mut().zip(links) {
                *score += a * w;
            }
        }
        if let Some(subject) = aimed {
            let parts = self.aimed_parts(subject, scoring, inputs, general, data);
            for (score, parts) in scores.iter_mut().zip(parts) {
                *score += parts.iter().sum::<f32>();
            }
        }
        scores
    }

    /// What a thing learned about as `subject`, aimed at, adds to each verb's
    /// score, in `VERBS` order (design v18 §5.5): its worth, how frightening
    /// it is, and the habit. Its worth and fear are the same for every verb,
    /// so they're worked out once.
    fn aimed_parts(
        &self,
        subject: Subject,
        scoring: SpriteScoring,
        inputs: &[f32],
        general: &SpriteMemory,
        data: &DataPack,
    ) -> [[f32; 3]; VERBS.len()] {
        let value_gain = self.params.get(BrainParam::ValueGain);
        let flight = self.params.get(BrainParam::Flight);
        let category = subject.category(data);
        let worth = value_gain * self.worth_of(subject, scoring.sprite, inputs, general, data);
        let distance = target_distance(inputs, data);
        let fear = self.fright(category, scoring, distance, general, data);
        let habits = self
            .experience
            .types
            .get(&subject)
            .map_or([0.0; VERBS.len()], |known| {
                std::array::from_fn(|i| known.habit(i))
            });
        // A pressing first-order need quiets good habits, never bad ones
        // (design v21 §5.6).
        let quiet = self.quiet(inputs, data);
        std::array::from_fn(|i| {
            let verb = VERBS[i];
            let habit = habits[i];
            [
                side(verb) * worth,
                fear_push(verb, flight, value_gain) * fear,
                if habit > 0.0 { quiet * habit } else { habit },
            ]
        })
    }

    /// What a thing learned about as `subject` is worth to the sprite now
    /// (design §5.6): each need's level, from `inputs`, times its worth for
    /// that need, plus its general good and its bad. For an object type it
    /// doesn't know, its category's summary (design v19 §5.6); for sprites,
    /// `sprite`'s, or `general`, sprites in general's (design v18 §5.6).
    fn worth_of(
        &self,
        subject: Subject,
        sprite: Option<EntityId>,
        inputs: &[f32],
        general: &SpriteMemory,
        data: &DataPack,
    ) -> f32 {
        let for_needs = |worth: &[f32]| -> f32 {
            data.need_places()
                .iter()
                .zip(worth)
                .map(|(&place, worth)| inputs[place] * worth)
                .sum()
        };
        let quiet = self.quiet(inputs, data);
        let now = |worth: &[f32], good: f32, bad: f32| for_needs(worth) + quiet * good + bad;
        if subject.category(data) == data.sprite_category() {
            let known = self.memory_of(sprite, general, data);
            return now(&known.worth, known.good, known.bad);
        }
        // The Cursor is one individual (design v29 §5.6).
        if subject.category(data) == data.cursor_category() {
            return self
                .experience
                .cursor
                .as_ref()
                .map_or(0.0, |known| now(&known.worth, known.good, known.bad));
        }
        match self.experience.types.get(&subject).filter(|t| t.touched) {
            Some(known) => now(&known.worth, known.good, known.badness()),
            None => {
                let summary = self.category_summary(subject.category(data), data);
                now(&summary.worth, summary.good, summary.bad)
            }
        }
    }

    /// How much of what the sprite merely likes counts now (design v21
    /// §5.6): 1 − quieting × urgency², urgency being the highest of its
    /// first-order needs in `inputs`. It never reaches 0 at a quieting
    /// below 1.
    fn quiet(&self, inputs: &[f32], data: &DataPack) -> f32 {
        let urgency = data
            .first_order_places()
            .iter()
            .map(|&place| inputs[place])
            .fold(0.0, f32::max);
        1.0 - self.params.get(BrainParam::Quieting) * urgency * urgency
    }

    /// A verb sampled from the `available` ones scoring at least
    /// `act_threshold`, by softmax(score / τ), τ being `tau_base` ×
    /// `exploration` (design §5.5): one draw. With none, it acts on nothing
    /// and stands where it is, drawing nothing (M2 design §19.2); at a
    /// threshold of 0, every available verb may start, as before.
    pub(crate) fn choose(
        &self,
        scores: &[f32; VERBS.len()],
        available: &[Verb],
        exploration: f32,
        rng: &mut ChaCha8Rng,
    ) -> Option<Verb> {
        let threshold = self.params.get(BrainParam::ActThreshold);
        let moving: Vec<Verb> = available
            .iter()
            .copied()
            .filter(|&verb| threshold <= 0.0 || scores[column(verb)] >= threshold)
            .collect();
        if moving.is_empty() {
            return None;
        }
        let tau = self.params.get(BrainParam::TauBase) * exploration;
        let values: Vec<f32> = moving.iter().map(|&v| scores[column(v)]).collect();
        Some(moving[sample(&values, tau, rng)])
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

/// What a body reads as a brain input at `locus`.
fn read(body: &Body, locus: LocusRef, data: &DataPack) -> f32 {
    match locus {
        LocusRef::Chem(id) => body.chems[data.chemical_index(id).expect("a checked input")],
        LocusRef::Locus(id) => body.loci[data.locus_index(id).expect("a checked input")],
    }
}

/// What a sprite sees of the hatched sprite it attends to (M2 design §10,
/// §15): its body language, and what it knows of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Looks {
    /// For each brain input that reads the attended sprite's body, in input
    /// order, the level it reads there: what its body shows.
    pub(crate) body: Vec<f32>,
    /// How well it knows it, from 0 to 1 (M2 design §15.2).
    pub(crate) familiar: f32,
    /// Whether it's its partner (M2 design §15.4).
    pub(crate) partner: bool,
    /// Whether it's its kin (M2 design §15.5).
    pub(crate) kin: bool,
}

/// What a hatched sprite's `body` shows a sprite attending to it (M2 design
/// §10): for each brain input that reads the attended sprite's body, in
/// input order, the level it reads there.
pub(crate) fn shown(body: &Body, data: &DataPack) -> Vec<f32> {
    data.brain_inputs_in_order()
        .iter()
        .filter_map(|input| match input.source {
            Source::Shown(locus) | Source::Differs(locus) => Some(read(body, locus, data)),
            _ => None,
        })
        .collect()
}

/// The chemical the breeding drive reads, by index, if it reads one (M2
/// design §5): what the front end must show as 0 in a sprite not grown up.
pub(crate) fn drive_chemical(data: &DataPack) -> Option<usize> {
    match data.drive()? {
        LocusRef::Chem(id) => data.chemical_index(id),
        _ => None,
    }
}

/// The breeding drive works only between adults (M2 design §5): in a
/// sprite not grown up, and in a choice about one, every input that reads
/// it, in the sprite or in the one it attends to, reads 0 in `inputs`,
/// whatever the genes.
pub(crate) fn without_drive(inputs: &mut [f32], data: &DataPack) {
    for &place in data.drive_places() {
        inputs[place] = 0.0;
    }
}

/// Where a sprite stands as it chooses, which decides the verbs it may
/// choose (design §5.2).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Situation {
    /// It has a target.
    pub(crate) target: bool,
    /// It's beside the target, on one of its goal tiles.
    pub(crate) beside: bool,
    /// The target is a place it remembers, out of sight (M2 design §7).
    pub(crate) remembered: bool,
    /// It and its target are both adults (M2 design §5).
    pub(crate) adults: bool,
    /// It's grown up.
    pub(crate) grown: bool,
    /// Its pack has calls and courtship (M2 design §15): a save from
    /// before them keeps its pack, which has neither.
    pub(crate) courts: bool,
}

/// The verbs a sprite may choose (design §5.2): Rest and Wander always;
/// with a target, every other verb, since a sprite may try anything on
/// anything (v16), but Approach only while it isn't already `beside` the
/// target, on one of its goal tiles, where Approach would do nothing, and
/// Retreat only from a thing in sight, not one `remembered` out of sight
/// (M2 design §7). Mate and Court are offered only when the sprite and its
/// target are both `adults`, and BreedAlone only when the sprite is
/// `grown`: the content-safety rule the engine enforces for every genome
/// and data pack (M2 design §5, §10, §15.3). Court and Call only in a pack
/// that `courts`.
pub(crate) fn available(situation: Situation) -> Vec<Verb> {
    let Situation {
        target,
        beside,
        remembered,
        adults,
        grown,
        courts,
    } = situation;
    VERBS
        .into_iter()
        .filter(|&verb| match verb {
            Verb::Rest | Verb::Wander => true,
            Verb::Approach => target && !beside,
            Verb::Retreat => target && !remembered,
            Verb::Mate => target && adults,
            Verb::BreedAlone => grown,
            Verb::Court => courts && target && adults,
            Verb::Call => courts,
            _ => target,
        })
        .collect()
}

/// What a thing learned about as `subject` is called, for sprites the
/// particular `sprite` if there is one (design v18, v19 §5.9).
fn thing_named(subject: Subject, sprite: Option<EntityId>, data: &DataPack) -> Thing {
    match sprite {
        Some(sprite) if subject.category(data) == data.sprite_category() => Thing::Sprite(sprite),
        _ => subject_thing(subject, data),
    }
}

/// Each one's share of a summary over `n` things known (design v18, v19
/// §5.6): its mean, at no strength while it knows one, and in full once it
/// knows `generalise`. 0 while the summary counts for nothing.
fn summary_share(n: usize, generalise: f32) -> f32 {
    let n = n as f32;
    let strength = ((n - 1.0) / (generalise - 1.0)).clamp(0.0, 1.0);
    if strength == 0.0 { 0.0 } else { strength / n }
}

/// What something learned about as `subject` is called (design v19 §6.1):
/// its object type, or its category if it has none; the Cursor by name
/// (design v29 §5.9).
fn subject_thing(subject: Subject, data: &DataPack) -> Thing {
    if subject.category(data) == data.cursor_category() {
        return Thing::Cursor;
    }
    match subject {
        Subject::ObjectType(id) => Thing::from(
            data.object_type(id)
                .expect("an object type in the pack")
                .name
                .as_str(),
        ),
        Subject::Category(category) => category_thing(category, data),
    }
}

/// The category `category`, named (design v19 §6.1).
fn category_thing(category: CategoryId, data: &DataPack) -> Thing {
    let category = data.category(category).expect("a category in the pack");
    Thing::Category(category.name.clone())
}

/// A thing something learned is about, by ID, before it's put in words (`Thing`).
#[derive(Debug, Clone, Copy)]
enum ThingKey {
    /// An object type, or water or sprites learned about as a category.
    Subject(Subject),
    /// A category's summary.
    Category(CategoryId),
    Sprite(EntityId),
    Cursor,
}

impl ThingKey {
    fn named(self, data: &DataPack) -> Thing {
        match self {
            ThingKey::Subject(subject) => subject_thing(subject, data),
            ThingKey::Category(category) => category_thing(category, data),
            ThingKey::Sprite(id) => Thing::Sprite(id),
            ThingKey::Cursor => Thing::Cursor,
        }
    }
}

/// Something a sprite has learned, by ID, before it's put in words
/// (`Learned`). A need is its place among the brain inputs.
#[derive(Debug, Clone, Copy)]
enum LearnedKey {
    Worth {
        thing: ThingKey,
        need: Option<usize>,
    },
    Bad {
        thing: ThingKey,
    },
    Fear {
        thing: ThingKey,
    },
    Habit {
        thing: ThingKey,
        verb: Verb,
    },
    NewThings,
}

impl LearnedKey {
    fn named(self, data: &DataPack) -> Learned {
        match self {
            LearnedKey::Worth { thing, need } => Learned::Worth {
                thing: thing.named(data),
                need: need.map(|place| data.brain_inputs_in_order()[place].name.clone()),
            },
            LearnedKey::Bad { thing } => Learned::Bad {
                thing: thing.named(data),
            },
            LearnedKey::Fear { thing } => Learned::Fear {
                thing: thing.named(data),
            },
            LearnedKey::Habit { thing, verb } => Learned::Habit {
                thing: thing.named(data),
                verb,
            },
            LearnedKey::NewThings => Learned::NewThings,
        }
    }
}

/// The `target_distance` input's value in `inputs` (design §5.2).
fn target_distance(inputs: &[f32], data: &DataPack) -> f32 {
    data.target_distance_place()
        .map_or(0.0, |place| inputs[place])
}

/// The item scoring more than `bar` that scores most, ties to the first.
pub(crate) fn best_above<T: Copy>(scored: impl Iterator<Item = (T, f32)>, bar: f32) -> Option<T> {
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
    use crate::test_util::{builtin, builtin_changing};

    /// The built-in pack's categories (design v20 §3.5.5, Appendix A).
    const BUSH: CategoryId = CategoryId(1);
    const FRUIT: CategoryId = CategoryId(2);
    const WATER: CategoryId = CategoryId(4);
    const TOY: CategoryId = CategoryId(5);

    /// The built-in pack's object types (design §3.5.3), as sprites learn
    /// about them.
    mod types {
        use crate::learning::Subject;

        pub(super) const BERRY_BUSH: Subject = Subject::ObjectType(1);
        pub(super) const BERRY: Subject = Subject::ObjectType(2);
        pub(super) const THORNBUSH: Subject = Subject::ObjectType(3);
        pub(super) const BALL: Subject = Subject::ObjectType(4);
        pub(super) const WATER: Subject = Subject::ObjectType(100);
        pub(super) const SPRITE: Subject = Subject::ObjectType(101);
    }

    /// Where `category` is in what a brain keeps per category.
    fn at(category: CategoryId) -> usize {
        builtin().category_index(category)
    }

    /// What `brain` has learned about `subject`, or nothing.
    fn about(brain: &Brain, subject: Subject) -> TypeMemory {
        let needs = builtin().need_places().len();
        brain
            .experience
            .types
            .get(&subject)
            .cloned()
            .unwrap_or_else(|| TypeMemory::new(needs))
    }

    /// What `brain` has learned about `subject`, having touched one, to set
    /// it.
    fn teach(brain: &mut Brain, subject: Subject) -> &mut TypeMemory {
        let needs = builtin().need_places().len();
        let known = brain.experience.learn_about(subject, needs);
        known.touched = true;
        known
    }

    fn brain(genes: &[&str]) -> Brain {
        let data = builtin();
        let text = format!("(format: 1, genes: [{}])", genes.join(", "));
        let genome = Genome::from_ron(&text, &data).expect("a valid genome");
        Brain::new(&genome, &data, LifeStage::Adult)
    }

    /// A brain of `genes` whose learned values never fade, so they read exactly.
    fn unfading(genes: &[&str]) -> Brain {
        let no_fade = [
            r#"BrainParam(param: "worth_fade_good", value: 0.0)"#,
            r#"BrainParam(param: "worth_fade_bad", value: 0.0)"#,
            r#"BrainParam(param: "habit_fade", value: 0.0)"#,
            r#"BrainParam(param: "habit_fade_bad", value: 0.0)"#,
            r#"BrainParam(param: "taught_fade", value: 0.0)"#,
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
        assert_eq!(
            n, 77,
            "60 State inputs, 4 of body language, 4 of whom it knows and 9 Target inputs"
        );
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
        let scores = brain.scores(
            &brain.activations(&x),
            &x,
            None,
            SpriteScoring::default(),
            &data,
        );
        assert_eq!(scores[column(Verb::Eat)], 0.6);
        assert_eq!(scores[column(Verb::Wander)], 0.3);
        assert_eq!(scores[column(Verb::Drink)], 0.0);
    }

    #[test]
    fn what_a_sprite_knows_of_the_one_it_attends_to_is_read_from_its_own_mind() {
        // M2 design §15.2, §15.4, §15.5: none of it with no sprite attended.
        let data = builtin();
        let brain = brain(&[]);
        let genome = Genome::from_ron("(format: 1, genes: [])", &data).expect("a genome");
        let program = crate::biochem::Program::new(&genome, &data, LifeStage::Adult);
        let body = Body::newborn(&program, &data);
        let names = ["target_familiar", "target_partner", "target_kin"];
        let read = |looks: Option<&Looks>| {
            let all = brain.inputs(&body, None, looks, &data);
            names.map(|name| all[input(name)])
        };
        let known = Looks {
            body: Vec::new(),
            familiar: 0.6,
            partner: true,
            kin: true,
        };
        assert_eq!(read(Some(&known)), [0.6, 1.0, 1.0]);
        let stranger = Looks {
            body: Vec::new(),
            familiar: 0.0,
            partner: false,
            kin: false,
        };
        assert_eq!(read(Some(&stranger)), [0.0; 3]);
        assert_eq!(read(None), [0.0; 3]);
    }

    #[test]
    fn target_inputs_never_feed_attention() {
        let data = builtin();
        let brain = brain(&[
            r#"AttentionInstinct(input: "hunger", category: "bush", weight: 0.8)"#,
            r#"BrainParam(param: "salience_gain", value: 0.5)"#,
        ]);
        let candidates = BTreeMap::from([
            (BUSH, Seen::in_sight(types::BERRY_BUSH, 0.4)),
            (WATER, Seen::in_sight(types::WATER, 0.0)),
        ]);
        let hungry = inputs(&[("hunger", 0.5)]);
        let also_aimed = inputs(&[
            ("hunger", 0.5),
            ("attended_water", 1.0),
            ("target_distance", 1.0),
            ("target_adjacent", 1.0),
        ]);
        let scores =
            brain.attention_scores(&hungry, &candidates, 0.0, SpriteScoring::default(), &data);
        // 0.5 × 0.8 learned, plus 0.5 × (1 − 0.4) salience.
        assert_eq!(scores[&BUSH], 0.4 + 0.3);
        assert_eq!(scores[&WATER], 0.5, "salience alone");
        assert_eq!(
            brain.attention_scores(
                &also_aimed,
                &candidates,
                0.0,
                SpriteScoring::default(),
                &data
            ),
            scores
        );
    }

    #[test]
    fn with_a_target_every_verb_can_be_tried_whatever_its_verb_table_says() {
        use Verb::*;
        assert_eq!(
            available(Situation::default()),
            [Rest, Wander],
            "no target: targetless only"
        );
        // Design v16 §5.2: a verb table says what a try does, not what's allowed.
        assert_eq!(
            available(Situation {
                target: true,
                ..Situation::default()
            }),
            [Approach, Eat, Drink, Hit, Play, Retreat, Rest, Wander]
        );
    }

    #[test]
    fn mate_is_offered_only_when_both_are_adults() {
        // M2 design §5: the engine's one content-safety rule.
        use Verb::*;
        assert!(
            !available(Situation {
                target: true,
                ..Situation::default()
            })
            .contains(&Mate)
        );
        assert_eq!(
            available(Situation {
                target: true,
                adults: true,
                ..Situation::default()
            }),
            [Approach, Eat, Drink, Hit, Play, Retreat, Rest, Wander, Mate]
        );
        assert_eq!(
            available(Situation {
                adults: true,
                ..Situation::default()
            }),
            [Rest, Wander]
        );
    }

    #[test]
    fn breeding_alone_is_offered_only_to_an_adult() {
        // M2 design §10: the same rule, for breeding alone.
        use Verb::*;
        assert!(
            !available(Situation {
                target: true,
                ..Situation::default()
            })
            .contains(&BreedAlone)
        );
        assert_eq!(
            available(Situation {
                grown: true,
                ..Situation::default()
            }),
            [Rest, Wander, BreedAlone]
        );
    }

    #[test]
    fn approach_is_not_offered_to_a_sprite_already_beside_its_target() {
        use Verb::*;
        assert_eq!(
            available(Situation {
                target: true,
                beside: true,
                ..Situation::default()
            }),
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
        brain.attended = Some(WATER);
        let mut scores = BTreeMap::from([(WATER, 0.3), (TOY, 0.45)]);
        assert_eq!(brain.attend(&scores, true, 1.0, &mut rng), Some(WATER));
        scores.insert(TOY, 0.55);
        scores.insert(FRUIT, 0.55);
        assert_eq!(
            brain.attend(&scores, true, 1.0, &mut rng),
            Some(FRUIT),
            "past the margin, to the best; a tie to the lower category"
        );
        scores.remove(&FRUIT);
        scores.remove(&WATER);
        brain.attended = Some(WATER);
        assert_eq!(
            brain.attend(&scores, true, 1.0, &mut rng),
            Some(TOY),
            "its category gone, the best of the rest"
        );
        assert_eq!(brain.attend(&BTreeMap::new(), true, 1.0, &mut rng), None);
    }

    #[test]
    fn both_temperatures_scale_with_exploration_mod() {
        // Design §5.3, §5.5: τ = base × exploration_mod. A brain of base 0.2
        // at exploration 2 draws as one of base 0.4 at exploration 1, and
        // not as itself at exploration 1.
        let draws = |base: f32, exploration: f32| {
            let mut brain = brain(&[
                &format!(r#"BrainParam(param: "tau_base", value: {base:?})"#),
                &format!(r#"BrainParam(param: "tau_att_base", value: {base:?})"#),
            ]);
            let mut rng = ChaCha8Rng::seed_from_u64(5);
            let scores = BTreeMap::from([(WATER, 0.3), (TOY, 0.6), (FRUIT, 0.1)]);
            let mut verbs = [0.0; VERBS.len()];
            verbs[column(Verb::Eat)] = 0.5;
            verbs[column(Verb::Rest)] = 0.2;
            let available = available(Situation {
                target: true,
                ..Situation::default()
            });
            (0..200)
                .map(|_| {
                    let attended = brain.attend(&scores, false, exploration, &mut rng);
                    let verb = brain.choose(&verbs, &available, exploration, &mut rng);
                    (attended, verb)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(draws(0.2, 2.0), draws(0.4, 1.0));
        assert_ne!(draws(0.2, 2.0), draws(0.2, 1.0));
    }

    #[test]
    fn the_brain_draws_from_the_rng_only_when_choosing_afresh() {
        let mut brain = brain(&[]);
        let scores = BTreeMap::from([(WATER, 0.3), (TOY, 0.5)]);
        let verbs = [0.1; VERBS.len()];
        let available = available(Situation {
            target: true,
            ..Situation::default()
        });
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let untouched = rng.clone();
        // An action running: attention and switching draw nothing.
        brain.attended = Some(WATER);
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
    fn a_sprite_starts_only_a_verb_pulling_at_least_the_act_threshold() {
        // M2 design §19.2: with `act_threshold` 0.25, only Eat (0.5) may
        // start; with nothing pulling that hard, it starts nothing and
        // draws nothing, and stands where it is.
        let brain = brain(&[r#"BrainParam(param: "act_threshold", value: 0.25)"#]);
        let available = available(Situation {
            target: true,
            ..Situation::default()
        });
        let mut verbs = [0.1; VERBS.len()];
        verbs[column(Verb::Eat)] = 0.5;
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        for _ in 0..50 {
            assert_eq!(
                brain.choose(&verbs, &available, 4.0, &mut rng),
                Some(Verb::Eat)
            );
        }
        verbs[column(Verb::Eat)] = 0.2;
        let untouched = rng.clone();
        assert_eq!(brain.choose(&verbs, &available, 1.0, &mut rng), None);
        assert_eq!(rng, untouched, "standing draws nothing");
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
    /// to a thing learned about as `attended`, with `activations`.
    fn snapshot(
        tick: u64,
        activations: Vec<f32>,
        verb: Verb,
        attended: Option<Subject>,
    ) -> Snapshot {
        Snapshot {
            tick,
            inputs: Vec::new(),
            activations,
            attention: BTreeMap::new(),
            scored: BTreeMap::new(),
            attended: attended.map(|s| s.category(&builtin())),
            target: None,
            subject: attended,
            scores: [0.0; VERBS.len()],
            verb: Some(verb),
            motive: None,
            sprite_seen: None,
            scoring: SpriteScoring::default(),
        }
    }

    /// Has `brain` decide and commit on each of `ticks`.
    fn decide_on(brain: &mut Brain, ticks: std::ops::Range<u64>) {
        for tick in ticks {
            brain.snapshot = Some(snapshot(tick, Vec::new(), Verb::Rest, None));
            brain.commit(tick, &builtin());
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

    /// Has `brain` decide `verb` at `tick`, attending to a thing learned
    /// about as `attended`, with the singletons `set` names active, and
    /// commit it.
    fn decide(
        brain: &mut Brain,
        tick: u64,
        verb: Verb,
        attended: Option<Subject>,
        set: &[(&str, f32)],
    ) {
        let a = activations(brain, set);
        let motive = brain.motive(verb, &a, &builtin());
        brain.snapshot = Some(Snapshot {
            motive,
            ..snapshot(tick, a, verb, attended)
        });
        brain.commit(tick, &builtin());
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_weakened_instinct_links_as_strongly_as_its_gene_s_health() {
        // M2 design §9.4: an instinct's weight, and an attention
        // instinct's, about whom or not, scale with its gene's health.
        let data = builtin();
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                Instinct(inputs: [("hunger", false)], verb: Eat, weight: 0.8),
                AttentionInstinct(input: "thirst", category: "water", weight: 0.6),
                AttentionInstinct(input: "always", category: "sprite", whom: Grown, weight: 0.4),
            ])"#,
            &data,
        )
        .expect("a valid genome");
        let brain = Brain::weakened(&genome, &data, LifeStage::Adult, &[0.5, 0.25, 0.5]);
        let eat = brain.decision.get(input("hunger"), column(Verb::Eat));
        assert!(close(eat, 0.4), "{eat}");
        let water = brain.attention.get(input("thirst"), at(WATER));
        assert!(close(water, 0.15), "{water}");
        let grown: Vec<f32> = brain.whom.iter().map(|link| link.weight).collect();
        assert_eq!(grown.len(), 1);
        assert!(close(grown[0], 0.2), "{grown:?}");
    }

    #[test]
    fn a_reward_or_a_punishment_leaves_the_instinct_links_as_they_were_born() {
        // Design v16 §5.1: instinct never changes in a sprite's life.
        let data = builtin();
        let mut brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 0.7)"#,
            r#"AttentionInstinct(input: "hunger", category: "fruit", weight: 0.4)"#,
        ]);
        let set = [("hunger", 1.0), ("always", 1.0)];
        decide(&mut brain, 9, Verb::Eat, Some(types::BERRY), &set);
        brain.touched = Some(Touch {
            tick: 9,
            verb: Some(Verb::Eat),
            subject: types::BERRY,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        });
        for (tick, hunger) in [(10, 1.0), (11, 0.0)] {
            let signals = hunger_at(&mut brain, &data, hunger);
            brain.learn(tick, &signals, 1.0, &data);
        }
        let eat = |brain: &Brain, name| brain.decision.get(input(name), column(Verb::Eat));
        assert_eq!(eat(&brain, "hunger"), 0.7);
        assert_eq!(eat(&brain, "always"), 0.0);
        let berry = brain.attention.get(input("hunger"), at(FRUIT));
        assert_eq!(berry, 0.4);
    }

    /// Step 4's signals for `brain` with hunger at `hunger`, every other
    /// need at 0, and the relief since its last step 4.
    fn hunger_at(brain: &mut Brain, data: &DataPack, hunger: f32) -> Signals {
        let needs = [vec![hunger], vec![0.0; data.need_places().len() - 1]].concat();
        Signals {
            relief: brain.relief(&needs, data),
            needs,
            ..Default::default()
        }
    }

    /// What `brain` has learned berry bushes are worth for hunger.
    fn bush_for_hunger(brain: &Brain) -> f32 {
        about(brain, types::BERRY_BUSH).worth[0]
    }

    #[test]
    fn a_need_s_fall_is_relief_only_at_or_past_the_deadband() {
        // physiology.ron: relief_deadband .02, worth_rate_good .5 by default.
        let data = builtin();
        let mut brain = unfading(&[]);
        let touch = Touch {
            tick: 1,
            verb: Some(Verb::Eat),
            subject: types::BERRY_BUSH,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        };
        brain.touched = Some(touch);
        let signals = hunger_at(&mut brain, &data, 0.5);
        brain.learn(1, &signals, 1.0, &data);
        let signals = hunger_at(&mut brain, &data, 0.49);
        brain.learn(2, &signals, 1.0, &data);
        assert_eq!(bush_for_hunger(&brain), 0.0, "a slow drift isn't relief");
        let signals = hunger_at(&mut brain, &data, 0.44);
        brain.learn(3, &signals, 1.0, &data);
        assert!(
            close(bush_for_hunger(&brain), 0.5 * 0.05),
            "{}",
            bush_for_hunger(&brain)
        );
    }

    #[test]
    fn learning_rate_mod_scales_what_is_learned_but_not_fading() {
        // Design §5.6: every learning rate is scaled by learning_rate_mod;
        // fading isn't (and familiarity grows in `commit`, which it never
        // reaches).
        let data = builtin();
        let learned_at = |rate_mod: f32| {
            let mut brain = unfading(&[]);
            brain.touched = Some(Touch {
                tick: 1,
                verb: Some(Verb::Eat),
                subject: types::BERRY_BUSH,
                sprite: None,
                novelty: 1.0,
                by_cursor: false,
                place: None,
            });
            let signals = hunger_at(&mut brain, &data, 0.5);
            brain.learn(1, &signals, rate_mod, &data);
            let signals = hunger_at(&mut brain, &data, 0.4);
            brain.learn(2, &signals, rate_mod, &data);
            bush_for_hunger(&brain)
        };
        assert!(close(learned_at(1.0), 0.5 * 0.1), "{}", learned_at(1.0));
        assert!(
            close(learned_at(2.0), 2.0 * 0.5 * 0.1),
            "{}",
            learned_at(2.0)
        );
        let faded_at = |rate_mod: f32| {
            let mut brain = brain(&[]);
            teach(&mut brain, types::BERRY_BUSH).worth[0] = 0.5;
            let signals = hunger_at(&mut brain, &data, 0.0);
            brain.learn(1, &signals, rate_mod, &data);
            bush_for_hunger(&brain)
        };
        assert!(faded_at(1.0) < 0.5, "it faded");
        assert_eq!(faded_at(2.0), faded_at(1.0));
        assert_eq!(faded_at(0.5), faded_at(1.0));
    }

    #[test]
    fn plasticity_scales_what_is_learned_as_learning_rate_mod_does() {
        // M2 design §9.5: the young learn faster.
        let data = builtin();
        let learned_with = |genes: &[&str]| {
            let mut brain = unfading(genes);
            brain.touched = Some(Touch {
                tick: 1,
                verb: Some(Verb::Eat),
                subject: types::BERRY_BUSH,
                sprite: None,
                novelty: 1.0,
                by_cursor: false,
                place: None,
            });
            let signals = hunger_at(&mut brain, &data, 0.5);
            brain.learn(1, &signals, 1.0, &data);
            let signals = hunger_at(&mut brain, &data, 0.4);
            brain.learn(2, &signals, 1.0, &data);
            bush_for_hunger(&brain)
        };
        let usual = learned_with(&[]);
        assert!(close(usual, 0.5 * 0.1), "{usual}");
        let plastic = learned_with(&[r#"BrainParam(param: "plasticity", value: 1.5)"#]);
        assert!(close(plastic, 1.5 * 0.5 * 0.1), "{plastic}");
    }

    #[test]
    fn of_two_needs_pushing_a_verb_equally_the_motive_is_the_lower_input_id() {
        // Design §5.5. The pack lists thirst before hunger, so the tie is
        // settled by input ID (hunger 1, thirst 2), not by the list.
        let data = builtin_changing(
            "brain_io.ron",
            &[(
                r#"needs: ["hunger", "thirst","#,
                r#"needs: ["thirst", "hunger","#,
            )],
        );
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                Instinct(inputs: [("hunger", false)], verb: Eat, weight: 0.5),
                Instinct(inputs: [("thirst", false)], verb: Eat, weight: 0.5),
            ])"#,
            &data,
        )
        .expect("a valid genome");
        let brain = Brain::new(&genome, &data, LifeStage::Adult);
        let place = |name: &str| {
            data.brain_inputs()
                .position(|(_, n)| n == name)
                .expect(name)
        };
        let (hunger, thirst) = (place("hunger"), place("thirst"));
        let need = |row: usize| data.need_places().iter().position(|&r| r == row);
        assert_eq!((need(thirst), need(hunger)), (Some(0), Some(1)));
        let mut a = vec![0.0; brain.concepts.len()];
        a[hunger] = 1.0;
        a[thirst] = 1.0;
        assert_eq!(brain.motive(Verb::Eat, &a, &data), need(hunger), "a tie");
        a[thirst] = 1.2;
        assert_eq!(
            brain.motive(Verb::Eat, &a, &data),
            need(thirst),
            "the bigger push"
        );
    }

    #[test]
    fn the_memory_lists_every_value_furthest_from_nothing_ties_in_listed_order() {
        // Design v40 §5.9: every value, not the furthest five. The two
        // bushes make their category a lesson too, at half strength.
        let data = builtin();
        let mut brain = unfading(&[]);
        teach(&mut brain, types::BALL).good = 0.9;
        teach(&mut brain, types::THORNBUSH).bad = -0.8;
        teach(&mut brain, types::BERRY).habits[column(Verb::Eat)] = 0.6;
        teach(&mut brain, types::BERRY_BUSH).worth[0] = 0.3;
        teach(&mut brain, types::BERRY).worth[0] = 0.3;
        teach(&mut brain, types::BALL).habits[column(Verb::Play)] = 0.3;
        let listed: Vec<Memory> = brain
            .learned(&data)
            .into_iter()
            .filter(|m| m.amount != 0.0)
            .collect();
        let memory = brain.memory(&data);
        assert_eq!(memory.len(), 8, "{listed:#?}");
        let amounts: Vec<f32> = memory.iter().map(|m| m.amount).collect();
        assert_eq!(amounts, [0.9, -0.8, 0.6, 0.3, 0.3, 0.3, -0.2, 0.075]);
        // All three 0.3s are kept, in the order they were listed.
        let ties: Vec<&Memory> = listed.iter().filter(|m| m.amount == 0.3).collect();
        assert_eq!(ties.len(), 3, "{listed:#?}");
        assert_eq!(
            [&memory[3], &memory[4], &memory[5]],
            [ties[0], ties[1], ties[2]]
        );
    }

    #[test]
    fn with_nothing_touched_relief_teaches_nothing() {
        let data = builtin();
        let mut brain = brain(&[]);
        let signals = hunger_at(&mut brain, &data, 1.0);
        brain.learn(1, &signals, 1.0, &data);
        let signals = hunger_at(&mut brain, &data, 0.5);
        brain.learn(2, &signals, 1.0, &data);
        assert_eq!(bush_for_hunger(&brain), 0.0);
        // A touch longer ago than the window (3 ticks) is forgotten too.
        brain.touched = Some(Touch {
            tick: 2,
            verb: Some(Verb::Eat),
            subject: types::BERRY_BUSH,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        });
        let signals = hunger_at(&mut brain, &data, 0.0);
        brain.learn(6, &signals, 1.0, &data);
        assert_eq!(bush_for_hunger(&brain), 0.0);
    }

    #[test]
    fn being_hit_teaches_nothing_about_what_sprites_are_worth() {
        // Design v16 §5.6: all sprites are one kind, so one attacker would
        // make a sprite shy of every sprite. The hit's punishment, with a
        // sprite attended and nothing touched, teaches no worth. Fear of the
        // one who hit it is learned apart (design v18 §5.6).
        let data = builtin();
        let mut brain = unfading(&[]);
        decide(&mut brain, 9, Verb::Wander, Some(types::SPRITE), &[]);
        let hit = Signals {
            needs: vec![0.0; data.need_places().len()],
            punishment: 1.0,
            ..Default::default()
        };
        brain.learn(10, &hit, 1.0, &data);
        assert_eq!(about(&brain, types::SPRITE).bad, 0.0);
        assert_eq!(brain.experience.new_things, 0.0);
    }

    /// Step 4's signals in a tick that teaches nothing.
    fn nothing_felt(data: &DataPack) -> Signals {
        Signals {
            needs: vec![0.0; data.need_places().len()],
            ..Default::default()
        }
    }

    #[test]
    fn forgetting_a_sprite_can_tip_a_lesson_about_sprites_in_general_in_a_tick_it_learns_nothing() {
        // Design v18 §5.6: sprites in general are the mean over the sprites
        // it remembers, in full once it knows generalise (3). Three bullies
        // feared at −.6 and a fourth sprite it barely fears make them
        // −.45, no lesson. The fourth fades past forget_below (.01) and is
        // forgotten, so the mean of the rest is −.6: a lesson, though the
        // tick teaches it nothing.
        let data = builtin();
        let mut brain = brain(&[]);
        let needs = data.need_places().len();
        for (id, fear) in [(7, -0.6), (8, -0.6), (9, -0.6), (10, -0.01)] {
            brain.experience.remember(EntityId(id), needs).fear = fear;
        }
        brain.learn(1, &nothing_felt(&data), 1.0, &data);
        assert_eq!(
            brain.experience.individuals.len(),
            3,
            "the fourth forgotten"
        );
        let in_general = Learned::Fear {
            thing: Thing::Category("sprite".into()),
        };
        assert_eq!(
            brain.learn(2, &nothing_felt(&data), 1.0, &data),
            [(in_general, false)]
        );
    }

    #[test]
    fn a_learned_and_a_taught_habit_pulling_apart_can_tip_a_lesson_in_a_tick_it_learns_nothing() {
        // #187: a habit is the one it learned plus the one the Cursor
        // taught, each fading at its own rate. Playing with balls learned
        // at .7 and taught at −.21 make .49, no lesson. The taught one fades
        // by taught_fade (.01) a tick and the learned one not at all, so
        // after five ticks of fading they pass .5, though no tick teaches it
        // anything.
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "habit_fade", value: 0.0)"#,
            r#"BrainParam(param: "taught_fade", value: 0.01)"#,
        ]);
        let ball = teach(&mut brain, types::BALL);
        ball.habits[column(Verb::Play)] = 0.7;
        ball.taught_habits[column(Verb::Play)] = -0.21;
        let learned: Vec<(u64, Vec<(Learned, bool)>)> = (1..=10)
            .map(|tick| (tick, brain.learn(tick, &nothing_felt(&data), 1.0, &data)))
            .filter(|(_, lessons)| !lessons.is_empty())
            .collect();
        let playing_with_balls = Learned::Habit {
            thing: "ball".into(),
            verb: Verb::Play,
        };
        assert_eq!(learned, [(6, vec![(playing_with_balls, true)])]);
    }

    #[test]
    fn touching_a_new_object_type_can_tip_a_category_lesson_in_a_tick_it_learns_nothing() {
        // Design v19 §5.6: a category's summary is the mean over the object
        // types in it the sprite has touched, at no strength while it knows
        // one, and in full once it knows generalise_types (2 here). Berry
        // bushes at −1 bad make bushes nothing while they're all it knows.
        // Touching a thornbush that teaches it nothing makes them −.5: a
        // lesson, though the tick teaches it nothing.
        let data = builtin();
        let mut brain = unfading(&[r#"BrainParam(param: "generalise_types", value: 2.0)"#]);
        teach(&mut brain, types::BERRY_BUSH).bad = -1.0;
        brain.learn(1, &nothing_felt(&data), 1.0, &data);
        teach(&mut brain, types::THORNBUSH);
        let bushes = Learned::Bad {
            thing: Thing::Category("bush".into()),
        };
        assert_eq!(
            brain.learn(2, &nothing_felt(&data), 1.0, &data),
            [(bushes, false)]
        );
    }

    #[test]
    fn generalising_more_can_tip_a_lesson_about_sprites_in_general_in_a_tick_it_learns_nothing() {
        // Design v18 §5.6: sprites in general count in full once it knows
        // generalise sprites. Two bullies feared at −.6 make them −.3 at
        // generalise 3, half strength. A new life stage can bring other
        // brain parameters (M2 design §9.2); at generalise 2 they're −.6:
        // a lesson, though the tick teaches it nothing.
        let data = builtin();
        let mut brain = brain(&[]);
        let needs = data.need_places().len();
        for id in [7, 8] {
            brain.experience.remember(EntityId(id), needs).fear = -0.6;
        }
        brain.learn(1, &nothing_felt(&data), 1.0, &data);
        brain.params = params(&[r#"BrainParam(param: "generalise", value: 2.0)"#]);
        let in_general = Learned::Fear {
            thing: Thing::Category("sprite".into()),
        };
        assert_eq!(
            brain.learn(2, &nothing_felt(&data), 1.0, &data),
            [(in_general, false)]
        );
    }

    #[test]
    fn a_thing_s_worth_for_a_need_counts_as_much_as_the_sprite_needs_it() {
        // Design v16 §5.3: value_gain (1) × Σ need × worth, plus good and bad.
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::BERRY).worth[0] = 0.5;
        teach(&mut brain, types::THORNBUSH).bad = -0.4;
        let candidates = BTreeMap::from([
            (FRUIT, Seen::in_sight(types::BERRY, 1.0)),
            (BUSH, Seen::in_sight(types::THORNBUSH, 1.0)),
        ]);
        let full = brain.attention_scores(
            &inputs(&[]),
            &candidates,
            0.0,
            SpriteScoring::default(),
            &data,
        );
        assert_eq!(full[&FRUIT], 0.0, "no hunger, no pull");
        assert_eq!(full[&BUSH], -0.4, "bad counts always");
        let hungry = brain.attention_scores(
            &inputs(&[("hunger", 0.8)]),
            &candidates,
            0.0,
            SpriteScoring::default(),
            &data,
        );
        assert!(close(hungry[&FRUIT], 0.4), "{hungry:?}");
    }

    #[test]
    fn a_remembered_place_draws_the_eye_by_its_worth_as_well_as_it_is_remembered() {
        // M2 design §7: worth × recall, and as at the edge of sight, no
        // salience for nearness.
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::WATER).worth[1] = 0.5;
        teach(&mut brain, types::WATER).familiarity = 1.0;
        let half_remembered = Seen {
            recall: 0.5,
            ..Seen::in_sight(types::WATER, 1.0)
        };
        let candidates = BTreeMap::from([(WATER, half_remembered)]);
        let none = SpriteScoring::default();
        let scores =
            brain.attention_scores(&inputs(&[("thirst", 0.8)]), &candidates, 0.0, none, &data);
        assert!(close(scores[&WATER], 0.8 * 0.5 * 0.5), "{scores:?}");
        let draw = brain.draw(
            half_remembered,
            None,
            &inputs(&[("thirst", 0.8)]),
            0.0,
            &brain.sprites_in_general(&data),
            &data,
        );
        assert!(close(draw, 0.8 * 0.5 * 0.5), "{draw}");
    }

    #[test]
    fn a_remembered_place_out_of_sight_is_never_backed_away_from() {
        // M2 design §7: there's nothing in sight to back away from.
        use Verb::*;
        assert_eq!(
            available(Situation {
                target: true,
                remembered: true,
                ..Situation::default()
            }),
            [Approach, Eat, Drink, Hit, Play, Rest, Wander]
        );
    }

    #[test]
    fn a_remembered_place_fades_and_is_forgotten_once_faint() {
        // M2 design §7: × (1 − place_fade) a tick, forgotten under
        // `forget_below` (.01).
        let data = builtin();
        let mut brain = brain(&[r#"BrainParam(param: "place_fade", value: 0.01)"#]);
        let at = Pos { x: 3, y: 4 };
        brain.experience.remember_place(
            types::WATER,
            Target::Water(at),
            at,
            crate::physiology::RememberedPlaces {
                held: 8,
                per_kind: 8,
                merge: 5,
            },
        );
        brain.fade(data.physiology().forget_below);
        assert!(close(brain.experience.places[0].recall, 0.99));
        // 0.99^458 is just over .01, and 0.99^459 just under.
        for _ in 1..458 {
            brain.fade(data.physiology().forget_below);
        }
        assert_eq!(brain.experience.places.len(), 1);
        brain.fade(data.physiology().forget_below);
        assert_eq!(brain.experience.places, []);
    }

    #[test]
    fn hunger_and_thirst_quiet_what_a_sprite_merely_likes_but_not_what_it_needs_or_dreads() {
        // Design v21 §5.6: general good counts × (1 − quieting (.8) ×
        // urgency²), urgency being the higher of hunger and thirst; worth for
        // a need, and bad, are never quieted.
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::BALL).good = 0.5;
        teach(&mut brain, types::THORNBUSH).bad = -0.4;
        teach(&mut brain, types::BERRY).worth[0] = 0.5;
        let candidates = BTreeMap::from([
            (TOY, Seen::in_sight(types::BALL, 1.0)),
            (BUSH, Seen::in_sight(types::THORNBUSH, 1.0)),
            (FRUIT, Seen::in_sight(types::BERRY, 1.0)),
        ]);
        let scores = |needs: &[(&str, f32)]| {
            let none = SpriteScoring::default();
            brain.attention_scores(&inputs(needs), &candidates, 0.0, none, &data)
        };
        assert_eq!(scores(&[])[&TOY], 0.5, "content, a like counts in full");
        let thirsty = scores(&[("thirst", 0.5)]);
        assert!(close(thirsty[&TOY], 0.5 * 0.8), "{thirsty:?}");
        let desperate = scores(&[("hunger", 1.0), ("thirst", 0.3)]);
        assert!(close(desperate[&TOY], 0.5 * 0.2), "{desperate:?}");
        assert_eq!(desperate[&BUSH], -0.4, "bad isn't quieted");
        assert!(
            close(desperate[&FRUIT], 0.5),
            "worth for hunger isn't quieted"
        );
    }

    #[test]
    fn hunger_and_thirst_quiet_good_habits_but_not_bad_ones() {
        // Design v21 §5.6, at quieting .8 and full thirst: a good habit keeps
        // a fifth, a bad one all.
        use Verb::*;
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::BALL).habits[column(Play)] = 0.5;
        teach(&mut brain, types::BALL).habits[column(Hit)] = -0.5;
        let scores = |needs: &[(&str, f32)]| {
            let x = inputs(needs);
            let none = SpriteScoring::default();
            brain.scores(&brain.activations(&x), &x, Some(types::BALL), none, &data)
        };
        let content = scores(&[]);
        assert_eq!((content[column(Play)], content[column(Hit)]), (0.5, -0.5));
        let desperate = scores(&[("thirst", 1.0)]);
        assert!(close(desperate[column(Play)], 0.1), "{desperate:?}");
        assert_eq!(desperate[column(Hit)], -0.5);
    }

    #[test]
    fn bad_habits_fade_at_their_own_rate() {
        // Design v21 §5.6: good habits by habit_fade, bad ones by
        // habit_fade_bad.
        use Verb::*;
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "habit_fade", value: 0.01)"#,
            r#"BrainParam(param: "habit_fade_bad", value: 0.001)"#,
        ]);
        teach(&mut brain, types::BALL).habits[column(Play)] = 0.5;
        teach(&mut brain, types::BALL).habits[column(Hit)] = -0.5;
        brain.fade(data.physiology().forget_below);
        let habits = about(&brain, types::BALL).habits;
        assert!(close(habits[column(Play)], 0.5 * 0.99), "{habits:?}");
        assert!(close(habits[column(Hit)], -0.5 * 0.999), "{habits:?}");
    }

    #[test]
    fn what_the_cursor_taught_fades_at_its_own_rate_good_or_bad() {
        // #187: by taught_fade, whatever the habit fades sprites learn
        // themselves.
        use Verb::*;
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "habit_fade_bad", value: 0.01)"#,
            r#"BrainParam(param: "taught_fade", value: 0.001)"#,
        ]);
        teach(&mut brain, types::BALL).taught_habits[column(Play)] = 0.5;
        teach(&mut brain, types::BALL).taught_habits[column(Hit)] = -0.5;
        brain.fade(data.physiology().forget_below);
        let taught = about(&brain, types::BALL).taught_habits;
        assert!(close(taught[column(Play)], 0.5 * 0.999), "{taught:?}");
        assert!(close(taught[column(Hit)], -0.5 * 0.999), "{taught:?}");
    }

    #[test]
    fn what_the_cursor_teaches_is_kept_apart_from_what_the_sprite_learns_itself() {
        // #187: a shock just after hitting a ball teaches the habit the
        // Cursor taught, at taught_rate (.9); what the sprite learns after,
        // along the trace, goes to its own, and the two count together.
        let data = builtin();
        let mut brain = unfading(&[]);
        decide(&mut brain, 9, Verb::Hit, Some(types::BALL), &[]);
        brain.touched = Some(Touch {
            tick: 9,
            verb: Some(Verb::Hit),
            subject: types::BALL,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        });
        let needs = vec![0.0; data.need_places().len()];
        let shocked = Signals {
            needs: needs.clone(),
            punishment: 1.0,
            corrected: true,
            ..Default::default()
        };
        brain.learn(10, &shocked, 1.0, &data);
        let hit = column(Verb::Hit);
        let ball = about(&brain, types::BALL);
        assert!(
            close(ball.taught_habits[hit], -0.9),
            "{:?}",
            ball.taught_habits
        );
        assert_eq!(ball.habits[hit], 0.0, "none of its own yet");
        // Hitting a ball again, it's rewarded with no Cursor near.
        decide(&mut brain, 20, Verb::Hit, Some(types::BALL), &[]);
        let rewarded = Signals {
            needs,
            reward: 1.0,
            ..Default::default()
        };
        brain.learn(21, &rewarded, 1.0, &data);
        let ball = about(&brain, types::BALL);
        assert!(ball.habits[hit] > 0.0, "{:?}", ball.habits);
        assert!(
            close(ball.taught_habits[hit], -0.9),
            "untouched: {:?}",
            ball.taught_habits
        );
        assert!(close(ball.habit(hit), ball.habits[hit] - 0.9));
    }

    #[test]
    fn a_habit_the_cursor_taught_counts_with_the_sprite_s_own_within_one() {
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::BALL).habits[column(Verb::Eat)] = -0.4;
        teach(&mut brain, types::BALL).taught_habits[column(Verb::Eat)] = -0.9;
        teach(&mut brain, types::BALL).taught_habits[column(Verb::Play)] = 0.3;
        let x = inputs(&[]);
        let scores = brain.scores(
            &brain.activations(&x),
            &x,
            Some(types::BALL),
            SpriteScoring::default(),
            &data,
        );
        assert_eq!(scores[column(Verb::Eat)], -1.0);
        assert!(close(scores[column(Verb::Play)], 0.3));
    }

    #[test]
    fn a_worthwhile_target_draws_the_sprite_to_it_and_a_bad_one_is_left_alone() {
        // Design v18 §5.5: + worth for Approach and the interactions, and
        // nothing for Retreat, Rest and Wander: badness no longer backs away.
        use Verb::*;
        let data = builtin();
        for worth in [0.3, -0.3] {
            let mut brain = brain(&[]);
            if worth > 0.0 {
                teach(&mut brain, types::BALL).good = worth;
            } else {
                teach(&mut brain, types::BALL).bad = worth;
            }
            let x = inputs(&[]);
            let none = SpriteScoring::default();
            let ball = Some(types::BALL);
            let scores = brain.scores(&brain.activations(&x), &x, ball, none, &data);
            for verb in [Approach, Eat, Drink, Hit, Play] {
                assert_eq!(scores[column(verb)], worth, "{verb:?}");
            }
            for verb in [Retreat, Rest, Wander] {
                assert_eq!(scores[column(verb)], 0.0, "{verb:?}");
            }
            let unaimed = brain.scores(&brain.activations(&x), &x, None, none, &data);
            assert_eq!(unaimed, [0.0; VERBS.len()], "nothing attended, no worth");
        }
    }

    #[test]
    fn the_breeding_drive_draws_no_eye_to_a_sprite_not_grown_up() {
        // M2 design §5: arousal's attention link counts for a grown sprite,
        // never for a young one or an egg.
        let data = builtin();
        let brain =
            brain(&[r#"AttentionInstinct(input: "arousal", category: "sprite", weight: 1.0)"#]);
        let aroused = inputs(&[("arousal", 1.0)]);
        let score = |not_grown: bool| {
            let seen = Seen {
                not_grown,
                ..Seen::in_sight(types::SPRITE, 1.0)
            };
            let candidates = BTreeMap::from([(data.sprite_category(), seen)]);
            let scores =
                brain.attention_scores(&aroused, &candidates, 1.0, SpriteScoring::default(), &data);
            scores[&data.sprite_category()]
        };
        assert!(
            close(score(false) - score(true), 1.0),
            "{} {}",
            score(false),
            score(true)
        );
    }

    #[test]
    fn what_partners_are_worth_for_the_breeding_drive_stays_with_them() {
        // M2 design §5: sprites in general are worth what the sprites known
        // are, but for the breeding drive nothing, so an aroused sprite is
        // drawn to no sprite it doesn't know, young or grown.
        let data = builtin();
        let mut brain = brain(&[r#"BrainParam(param: "generalise", value: 2.0)"#]);
        let needs: Vec<&str> = data.needs().collect();
        let need = |name: &str| needs.iter().position(|&n| n == name).expect("a need");
        let (arousal, loneliness) = (need("arousal"), need("loneliness"));
        for partner in [EntityId(7), EntityId(8)] {
            let mut known = SpriteMemory::new(needs.len());
            known.worth[arousal] = 0.8;
            known.worth[loneliness] = 0.4;
            brain.experience.individuals.insert(partner, known);
        }
        let general = brain.sprites_in_general(&data);
        assert!(close(general.worth[loneliness], 0.4), "{general:?}");
        assert_eq!(general.worth[arousal], 0.0);
        // To an aroused sprite, one it doesn't know is worth nothing, and a
        // partner what it learned.
        let aroused = inputs(&[("arousal", 1.0)]);
        let sprite = types::SPRITE;
        assert_eq!(brain.worth_of(sprite, None, &aroused, &general, &data), 0.0);
        let partner = brain.worth_of(sprite, Some(EntityId(7)), &aroused, &general, &data);
        assert!(close(partner, 0.8), "{partner}");
    }

    #[test]
    fn a_sprite_known_only_by_sight_is_judged_as_sprites_in_general_and_not_counted_in_them() {
        // M2 design §15.2: one it knows well by sight, and has learned less
        // than `forget_below` (0.01) of, isn't among the sprites its sprites
        // in general are the mean of, is worth what they are, and isn't
        // listed in its memory.
        let data = builtin();
        let mut brain = brain(&[r#"BrainParam(param: "generalise", value: 2.0)"#]);
        let needs: Vec<&str> = data.needs().collect();
        let hunger = needs.iter().position(|&n| n == "hunger").expect("a need");
        for taught in [EntityId(7), EntityId(8)] {
            brain.experience.remember(taught, needs.len()).worth[hunger] = 0.6;
        }
        let seen = EntityId(9);
        let below = data.physiology().forget_below;
        brain.experience.befriend(seen, 0.5, needs.len(), below);
        brain.experience.remember(seen, needs.len()).worth[hunger] = 0.005;
        let general = brain.sprites_in_general(&data);
        assert!(close(general.worth[hunger], 0.6), "{general:?}");
        let hungry = inputs(&[("hunger", 1.0)]);
        let worth = |sprite| brain.worth_of(types::SPRITE, sprite, &hungry, &general, &data);
        assert!(close(worth(Some(seen)), 0.6), "{}", worth(Some(seen)));
        assert_eq!(worth(Some(seen)), worth(None));
        let about_it = |m: &Memory| match &m.learned {
            Learned::Worth { thing, .. } | Learned::Bad { thing } | Learned::Fear { thing } => {
                *thing == Thing::Sprite(seen)
            }
            Learned::Habit { .. } | Learned::NewThings => false,
        };
        let memory = brain.memory(&data);
        assert!(!memory.iter().any(about_it), "{memory:?}");
        let taught = Learned::Worth {
            thing: Thing::Sprite(EntityId(7)),
            need: Some("hunger".to_string()),
        };
        assert!(memory.iter().any(|m| m.learned == taught), "{memory:?}");
    }

    #[test]
    fn the_starter_genes_lean_a_crowded_adult_to_hitting_a_stranger_but_not_its_family() {
        // M2 design §15.6: a family together is a crowd. Crowded and looking
        // at a sprite, the starter's adult leans to hitting it, but when the
        // sprite is its family, its partner or its young, Hit scores below
        // nothing. A lean, not a rule: Hit is only less likely.
        let data = builtin();
        let brain = Brain::new(data.starter(), &data, LifeStage::Adult);
        let hit = |family: f32| {
            let x = inputs(&[
                ("crowdedness", 1.0),
                ("attended_sprite", 1.0),
                ("target_family", family),
            ]);
            let sprite = Some(types::SPRITE);
            let scoring = SpriteScoring::default();
            brain.scores(&brain.activations(&x), &x, sprite, scoring, &data)[column(Verb::Hit)]
        };
        let (stranger, family) = (hit(0.0), hit(1.0));
        assert!(stranger > 0.0, "a stranger: {stranger}");
        assert!(family < 0.0, "its family: {family}");
    }

    #[test]
    fn a_frightening_sprite_near_is_backed_away_from_and_not_gone_near_but_never_hit() {
        // Design v18 §5.5: a fear of .5 adds flight (.8) × .5 to Retreat and
        // takes value_gain (1) × .5 from Approach, Eat, Drink and Play, but
        // not from Hit; it fades to nothing at fear_reach (.5), and is quiet
        // while a hit is felt or the sprite is cornered.
        use Verb::*;
        let data = builtin();
        let mut brain = brain(&[]);
        let bully = EntityId(7);
        let mut known = SpriteMemory::new(data.need_places().len());
        known.fear = -0.5;
        brain.experience.individuals.insert(bully, known);
        let scoring = SpriteScoring {
            sprite: Some(bully),
            quiet: false,
        };
        let scores = |distance: f32, scoring: SpriteScoring| {
            let x = inputs(&[("target_distance", distance)]);
            let sprite = Some(types::SPRITE);
            brain.scores(&brain.activations(&x), &x, sprite, scoring, &data)
        };
        let near = scores(0.0, scoring);
        assert!(close(near[column(Retreat)], 0.4), "{near:?}");
        for verb in [Approach, Eat, Drink, Play] {
            assert!(close(near[column(verb)], -0.5), "{verb:?}: {near:?}");
        }
        for verb in [Hit, Rest, Wander] {
            assert_eq!(near[column(verb)], 0.0, "{verb:?}");
        }
        let halfway = scores(0.25, scoring);
        assert!(close(halfway[column(Retreat)], 0.2), "{halfway:?}");
        assert_eq!(
            scores(0.5, scoring),
            [0.0; VERBS.len()],
            "out of fear's reach"
        );
        let quiet = SpriteScoring {
            quiet: true,
            ..scoring
        };
        assert_eq!(scores(0.0, quiet), [0.0; VERBS.len()], "quiet");
    }

    #[test]
    fn good_less_bad_teaches_habits_along_the_trace_the_most_recent_most() {
        // Design v16 §5.6: habit_rate (.3) × (reward − punishment) × λ^age,
        // for entries whose verb was aimed at the thing they attended.
        let data = builtin();
        let mut brain = unfading(&[r#"BrainParam(param: "trace_decay", value: 0.5)"#]);
        decide(&mut brain, 7, Verb::Rest, Some(types::BERRY), &[]);
        decide(&mut brain, 8, Verb::Approach, Some(types::BERRY), &[]);
        decide(&mut brain, 9, Verb::Eat, Some(types::BERRY), &[]);
        let hurt = Signals {
            needs: vec![0.0; data.need_places().len()],
            punishment: 1.0,
            ..Default::default()
        };
        brain.learn(10, &hurt, 1.0, &data);
        let habit = |verb| about(&brain, types::BERRY).habits[column(verb)];
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
        teach(&mut brain, types::BALL).habits[column(Verb::Eat)] = -0.4;
        let x = inputs(&[]);
        let scores = brain.scores(
            &brain.activations(&x),
            &x,
            Some(types::BALL),
            SpriteScoring::default(),
            &data,
        );
        assert_eq!(scores[column(Verb::Eat)], -0.4);
        assert_eq!(scores[column(Verb::Play)], 0.0);
        let at_a_berry = brain.scores(
            &brain.activations(&x),
            &x,
            Some(types::BERRY),
            SpriteScoring::default(),
            &data,
        );
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
        decide(&mut brain, 9, Verb::Eat, Some(types::BALL), &set);
        brain.touched = Some(Touch {
            tick: 9,
            verb: Some(Verb::Eat),
            subject: types::BALL,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        });
        let mut needs = vec![0.0; data.need_places().len()];
        needs[0] = 0.8;
        let fruitless = Signals {
            needs,
            fruitless: true,
            ..Default::default()
        };
        brain.learn(10, &fruitless, 1.0, &data);
        let habit = about(&brain, types::BALL).habits[column(Verb::Eat)];
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
            Some(types::BALL),
            &[("always", 1.0)],
        );
        brain.touched = Some(Touch {
            tick: 9,
            verb: Some(Verb::Play),
            subject: types::BALL,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        });
        let fruitless = Signals {
            needs: vec![1.0; data.need_places().len()],
            fruitless: true,
            ..Default::default()
        };
        brain.learn(10, &fruitless, 1.0, &data);
        assert_eq!(about(&brain, types::BALL).habits[column(Verb::Play)], 0.0);
    }

    #[test]
    fn learned_values_fade_each_tick_at_their_channel_s_rate() {
        // Design v16 §5.6: good by worth_fade_good, bad by worth_fade_bad,
        // good habits by habit_fade (bad ones: bad_habits_fade_at_their_own_rate).
        let data = builtin();
        let mut brain = brain(&[
            r#"BrainParam(param: "worth_fade_good", value: 0.01)"#,
            r#"BrainParam(param: "worth_fade_bad", value: 0.001)"#,
            r#"BrainParam(param: "habit_fade", value: 0.005)"#,
        ]);
        teach(&mut brain, types::BERRY).worth[0] = 0.5;
        teach(&mut brain, types::BERRY).good = 0.4;
        teach(&mut brain, types::THORNBUSH).bad = -0.5;
        teach(&mut brain, types::BERRY).habits[column(Verb::Eat)] = 0.2;
        let quiet = Signals {
            needs: vec![0.0; data.need_places().len()],
            ..Default::default()
        };
        brain.learn(1, &quiet, 1.0, &data);
        let (berries, thornbushes) = (about(&brain, types::BERRY), about(&brain, types::THORNBUSH));
        assert!(close(berries.worth[0], 0.5 * 0.99));
        assert!(close(berries.good, 0.4 * 0.99));
        assert!(close(thornbushes.bad, -0.5 * 0.999));
        assert!(close(berries.habits[column(Verb::Eat)], 0.2 * 0.995));
    }

    #[test]
    fn attending_to_an_object_type_makes_it_familiar() {
        // Design v16 §5.6: familiarity_rate (.002) each tick attended.
        let mut brain = brain(&[]);
        for tick in 0..10 {
            decide(&mut brain, tick, Verb::Rest, Some(types::BALL), &[]);
        }
        let familiar = about(&brain, types::BALL).familiarity;
        assert!(close(familiar, 0.02), "{familiar}");
        assert_eq!(about(&brain, types::BERRY).familiarity, 0.0);
    }

    #[test]
    fn attending_to_berry_bushes_leaves_thornbushes_new_though_both_are_bushes() {
        // Design v19 §5.6: newness is about the object type, so an apple is
        // new to a sprite that knows only berries. familiarity_rate (.002)
        // each tick attended. Both bushes are in the bush category.
        let data = builtin();
        let mut brain = brain(&[]);
        for tick in 0..10 {
            let attending = snapshot(tick, Vec::new(), Verb::Rest, Some(types::BERRY_BUSH));
            brain.snapshot = Some(attending);
            brain.commit(tick, &data);
        }
        let known = brain.novelty(types::BERRY_BUSH);
        assert!(close(known, 1.0 - 0.02), "{known}");
        assert_eq!(brain.novelty(types::THORNBUSH), 1.0);
    }

    #[test]
    fn unfamiliar_object_types_draw_the_eye_as_boldly_as_the_sprite_is_bold() {
        // Design v16 §5.3: curiosity (.3) × novelty × max(0, 1 + new things)
        // × curiosity_mod; salience is 0 at distance 1.
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::BALL).familiarity = 0.5;
        let candidates = BTreeMap::from([
            (FRUIT, Seen::in_sight(types::BERRY, 1.0)),
            (TOY, Seen::in_sight(types::BALL, 1.0)),
        ]);
        let x = inputs(&[]);
        let score = |brain: &Brain, mood: f32| {
            brain.attention_scores(&x, &candidates, mood, SpriteScoring::default(), &data)
        };
        let calm = score(&brain, 1.0);
        assert!(close(calm[&FRUIT], 0.3), "{calm:?}");
        assert!(close(calm[&TOY], 0.15), "half familiar, half the pull");
        let wary = score(&brain, 0.5);
        assert!(close(wary[&FRUIT], 0.15), "wariness halves it");
        brain.experience.new_things = -0.5;
        let shy = score(&brain, 1.0);
        assert!(close(shy[&FRUIT], 0.15), "a shy sprite, half");
    }

    #[test]
    fn a_hurt_from_something_new_teaches_that_new_things_are_bad() {
        // Design v16 §5.6: (good − worth_rate_bad (.8) × punishment) × how
        // new the thing touched was.
        let data = builtin();
        let mut brain = unfading(&[]);
        brain.touched = Some(Touch {
            tick: 1,
            verb: Some(Verb::Eat),
            subject: types::THORNBUSH,
            sprite: None,
            novelty: 0.5,
            by_cursor: false,
            place: None,
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
    fn a_choice_is_explained_by_its_instincts_the_thing_s_worth_and_the_habit() {
        // Design v16 §5.9: largest first whatever the sign.
        let data = builtin();
        let mut brain =
            brain(&[r#"Instinct(inputs: [("boredom", false)], verb: Play, weight: 1.0)"#]);
        teach(&mut brain, types::BALL).good = 0.3;
        teach(&mut brain, types::BALL).habits[column(Verb::Play)] = -0.2;
        let x = inputs(&[("boredom", 0.5)]);
        brain.snapshot = Some(Snapshot {
            inputs: x.clone(),
            ..snapshot(9, brain.activations(&x), Verb::Play, Some(types::BALL))
        });
        let explained = brain.explain(&data).expect("a decision");
        let parts: Vec<(Part, f32)> = explained
            .contributions
            .into_iter()
            .map(|c| (c.part, c.amount))
            .collect();
        assert_eq!(
            parts,
            [
                (Part::Concept(vec![("boredom", false)]), 0.5),
                (Part::Worth("ball".into()), 0.3),
                (Part::Habit("ball".into()), -0.2),
            ]
        );
    }

    #[test]
    fn a_hit_back_and_forth_teaches_nothing_about_what_sprites_are_worth() {
        // Design v16 §5.6, change 16: the sprite hit a sprite, the thing
        // touched, and a hit back within the touch window punishes it.
        let data = builtin();
        let mut brain = unfading(&[]);
        decide(&mut brain, 9, Verb::Hit, Some(types::SPRITE), &[]);
        brain.touched = Some(Touch {
            tick: 9,
            verb: Some(Verb::Hit),
            subject: types::SPRITE,
            sprite: None,
            novelty: 1.0,
            by_cursor: false,
            place: None,
        });
        let hit_back = Signals {
            needs: vec![0.0; data.need_places().len()],
            punishment: 1.0,
            hit: true,
            ..Default::default()
        };
        brain.learn(10, &hit_back, 1.0, &data);
        assert_eq!(about(&brain, types::SPRITE).bad, 0.0);
        assert_eq!(brain.experience.new_things, 0.0);
        let habit = about(&brain, types::SPRITE).habits[column(Verb::Hit)];
        assert!(habit < 0.0, "the habit still learns: {habit}");
    }

    #[test]
    fn an_instinct_past_one_is_born_at_one() {
        let brain = brain(&[
            r#"Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.5)"#,
            r#"AttentionInstinct(input: "hunger", category: "fruit", weight: -1.2)"#,
        ]);
        assert_eq!(brain.decision.get(input("hunger"), column(Verb::Eat)), 1.0);
        assert_eq!(brain.attention.get(input("hunger"), at(FRUIT)), -1.0);
    }

    #[test]
    fn a_brain_that_stops_deciding_still_drops_what_no_longer_counts() {
        // A sprite held by the Cursor, or on a scripted action, commits no
        // entries, but the ones it has still fade out: 0.9^43 is the last
        // that counts.
        let mut brain = brain(&[r#"BrainParam(param: "trace_decay", value: 0.9)"#]);
        decide_on(&mut brain, 0..3);
        brain.commit(40, &builtin());
        assert_eq!(brain.trace.len(), 3, "0.9^(41 − 0) still counts");
        brain.commit(43, &builtin());
        let ticks: Vec<u64> = brain.trace.iter().map(|e| e.tick).collect();
        assert_eq!(
            ticks,
            [1, 2],
            "at tick 44's step 4, tick 0's weight is 0.9^44"
        );
        brain.commit(100, &builtin());
        assert!(brain.trace.is_empty());
    }

    #[test]
    fn a_try_at_mating_or_courting_is_never_a_moment() {
        // M2 design §18.3: nothing about mating is learned by watching, so
        // a sprite watching never sees one; a try at playing is a moment.
        let data = builtin();
        let partner = EntityId(3);
        let moment = |verb| {
            let mut brain = brain(&[]);
            brain.touched = Some(Touch {
                tick: 9,
                verb: Some(verb),
                subject: types::SPRITE,
                sprite: Some(partner),
                novelty: 0.0,
                by_cursor: false,
                place: None,
            });
            let signals = hunger_at(&mut brain, &data, 0.0);
            brain.learn_with_moment(10, &signals, 1.0, &data).1
        };
        assert_eq!(moment(Verb::Mate), []);
        assert_eq!(moment(Verb::Court), []);
        assert_eq!(
            moment(Verb::Play),
            [Topic::Tried {
                subject: types::SPRITE,
                verb: Verb::Play,
                sprite: Some(partner),
            }]
        );
    }

    /// Has `brain` choose `verb` at a thing learned about as `subject` at
    /// its latest step 5, for the need `motive`.
    fn chose(brain: &mut Brain, verb: Verb, subject: Subject, motive: Option<usize>) {
        let mut chosen = snapshot(0, Vec::new(), verb, Some(subject));
        chosen.motive = motive;
        brain.snapshot = Some(chosen);
    }

    #[test]
    fn watching_moves_a_thing_s_worth_only_for_the_need_that_chose_the_try() {
        // M2 design §18.2: of a thing's worth, a sprite watching another
        // takes on only that for the try's motive (design §5.5). A try no
        // need chose, or that the other's brain didn't choose, shows no
        // worth; good and bad pass on whatever chose it.
        let data = builtin();
        let (hunger, thirst) = (0, 1);
        let play = Topic::Tried {
            subject: types::BALL,
            verb: Verb::Play,
            sprite: None,
        };
        let mut player = unfading(&[]);
        let known = teach(&mut player, types::BALL);
        known.worth.fill(0.8);
        known.good = 0.6;
        let watched = |player: &Brain| {
            let mut learner = unfading(&[]);
            learner.watched(&[player.shown(play, &data)], 0.5, &data);
            about(&learner, types::BALL)
        };
        chose(&mut player, Verb::Play, types::BALL, Some(thirst));
        let learned = watched(&player);
        for (need, &value) in learned.worth.iter().enumerate() {
            let expected = if need == thirst { 0.4 } else { 0.0 };
            assert!(close(value, expected), "need {need}: {value}");
        }
        assert!(close(learned.good, 0.3), "{}", learned.good);
        let no_worth = |learned: &TypeMemory| learned.worth.iter().all(|&w| w == 0.0);
        chose(&mut player, Verb::Play, types::BALL, None);
        let learned = watched(&player);
        assert!(no_worth(&learned), "no motive");
        assert!(close(learned.good, 0.3), "{}", learned.good);
        chose(&mut player, Verb::Eat, types::BALL, Some(hunger));
        assert!(no_worth(&watched(&player)), "another try");
        player.snapshot = None;
        assert!(no_worth(&watched(&player)), "none chosen");
    }

    #[test]
    fn watching_never_moves_worth_for_the_breeding_drive() {
        // M2 design §18.3: a sprite watching another takes on none of a
        // thing's worth for the breeding drive, whatever the other knows,
        // even when the drive chose the try.
        let data = builtin();
        let drive = data.physiology().indices.drive.expect("a breeding drive");
        let play = Topic::Tried {
            subject: types::BALL,
            verb: Verb::Play,
            sprite: None,
        };
        let mut player = unfading(&[]);
        teach(&mut player, types::BALL).worth.fill(0.8);
        chose(&mut player, Verb::Play, types::BALL, Some(drive));
        let mut learner = unfading(&[]);
        learner.watched(&[player.shown(play, &data)], 0.5, &data);
        let worth = about(&learner, types::BALL).worth;
        assert!(worth.iter().all(|&w| w == 0.0), "{worth:?}");
    }

    #[test]
    fn a_sprite_watched_using_a_thing_it_knows_nothing_of_teaches_nothing() {
        // M2 design §18.2: watching never moves a value back towards 0, so
        // a thing it judges by its category's summary stays judged so.
        let data = builtin();
        let eat = Topic::Tried {
            subject: types::BERRY_BUSH,
            verb: Verb::Eat,
            sprite: None,
        };
        let mut learner = unfading(&[]);
        let mut eater = unfading(&[]);
        chose(&mut eater, Verb::Eat, types::BERRY_BUSH, Some(0));
        learner.watched(&[eater.shown(eat, &data)], 0.5, &data);
        assert!(!about(&learner, types::BERRY_BUSH).touched);
    }

    /// A ball's player whose ball is worth .8 for each need, .6 good, and
    /// playing with one a habit of .9, that chose to play for its thirst;
    /// and a sprite that knew nothing of balls having watched it play with
    /// one, taking on half of what it showed.
    fn watched_playing() -> Brain {
        let data = builtin();
        let play = Topic::Tried {
            subject: types::BALL,
            verb: Verb::Play,
            sprite: None,
        };
        let mut player = unfading(&[]);
        let known = teach(&mut player, types::BALL);
        known.worth.fill(0.8);
        known.good = 0.6;
        known.habits[column(Verb::Play)] = 0.9;
        chose(&mut player, Verb::Play, types::BALL, Some(1));
        let mut learner = unfading(&[]);
        learner.watched(&[player.shown(play, &data)], 0.5, &data);
        learner
    }

    #[test]
    fn having_taken_on_a_thing_s_worth_by_watching_a_sprite_knows_its_object_type() {
        // M2 design §18.2: as if it had touched one, so its own view of
        // balls counts rather than the toys' summary.
        let learner = watched_playing();
        let ball = about(&learner, types::BALL);
        assert!(close(ball.good, 0.3), "{}", ball.good);
        assert!(ball.touched);
    }

    #[test]
    fn a_habit_taken_on_by_watching_is_kept_apart() {
        // M2 design §19.7: apart from what it learns itself and what the
        // Cursor teaches it, and counted with them.
        let learner = watched_playing();
        let ball = about(&learner, types::BALL);
        let play = column(Verb::Play);
        assert!(close(ball.watched_habits[play], 0.45), "{ball:?}");
        assert_eq!(ball.habits[play], 0.0);
        assert_eq!(ball.taught_habits[play], 0.0);
        assert!(close(ball.habit(play), 0.45), "{ball:?}");
    }

    /// A sprite that has watched `shown`'s player, knowing `mine` of balls
    /// already, its share being a half.
    fn watching_with(mine: impl Fn(&mut TypeMemory), shown: impl Fn(&mut TypeMemory)) -> Brain {
        let data = builtin();
        let play = Topic::Tried {
            subject: types::BALL,
            verb: Verb::Play,
            sprite: None,
        };
        let mut player = unfading(&[]);
        shown(teach(&mut player, types::BALL));
        chose(&mut player, Verb::Play, types::BALL, None);
        let mut learner = unfading(&[]);
        mine(teach(&mut learner, types::BALL));
        learner.watched(&[player.shown(play, &data)], 0.5, &data);
        learner
    }

    #[test]
    fn what_a_sprite_took_on_by_watching_settles_either_way_towards_what_it_sees() {
        // M2 design §19.7: half of the way, back towards 0 too, and
        // whatever it learned itself is left alone.
        let play = column(Verb::Play);
        let learner = watching_with(
            |mine| {
                mine.watched_habits[play] = 0.6;
                mine.watched_bad = -0.6;
                mine.habits[play] = 0.2;
                mine.bad = -0.2;
            },
            |theirs| theirs.habits[play] = 0.0,
        );
        let ball = about(&learner, types::BALL);
        assert!(close(ball.watched_habits[play], 0.3), "{ball:?}");
        assert!(close(ball.watched_bad, -0.3), "{ball:?}");
        assert_eq!((ball.habits[play], ball.bad), (0.2, -0.2));
        assert!(close(ball.habit(play), 0.5), "{ball:?}");
        assert!(close(ball.badness(), -0.5), "{ball:?}");
    }

    #[test]
    fn badness_taken_on_by_watching_is_kept_apart_from_what_a_sprite_found_out() {
        // M2 design §19.7: one it watches that finds balls very bad moves
        // what it took on, and its own view of balls stays as it found out.
        let learner = watching_with(|mine| mine.bad = -0.2, |theirs| theirs.bad = -0.8);
        let ball = about(&learner, types::BALL);
        assert_eq!(ball.bad, -0.2);
        assert!(close(ball.watched_bad, -0.4), "{ball:?}");
        assert!(close(ball.badness(), -0.6), "{ball:?}");
    }

    #[test]
    fn a_sprite_shows_what_it_found_out_and_was_taught_not_what_it_watched() {
        // M2 design §19.7: what it took on by watching is never passed on.
        let data = builtin();
        let play = column(Verb::Play);
        let mut sprite = unfading(&[]);
        let ball = teach(&mut sprite, types::BALL);
        ball.habits[play] = 0.1;
        ball.taught_habits[play] = 0.2;
        ball.watched_habits[play] = 0.4;
        ball.bad = -0.1;
        ball.watched_bad = -0.5;
        chose(&mut sprite, Verb::Play, types::BALL, None);
        let topic = Topic::Tried {
            subject: types::BALL,
            verb: Verb::Play,
            sprite: None,
        };
        let shown = sprite.shown(topic, &data);
        assert!(close(shown.habit, 0.3), "{shown:?}");
        assert_eq!(shown.bad, -0.1);
    }

    #[test]
    fn what_a_sprite_took_on_by_watching_fades_as_what_it_was_taught() {
        // M2 design §19.7: its habits by taught_fade, and its badness as
        // badness it found out, by worth_fade_bad.
        let data = builtin();
        let play = column(Verb::Play);
        let mut brain = brain(&[
            r#"BrainParam(param: "habit_fade", value: 0.01)"#,
            r#"BrainParam(param: "taught_fade", value: 0.001)"#,
            r#"BrainParam(param: "worth_fade_bad", value: 0.002)"#,
        ]);
        let ball = teach(&mut brain, types::BALL);
        ball.watched_habits[play] = 0.5;
        ball.watched_bad = -0.5;
        brain.fade(data.physiology().forget_below);
        let ball = about(&brain, types::BALL);
        assert!(close(ball.watched_habits[play], 0.5 * 0.999), "{ball:?}");
        assert!(close(ball.watched_bad, -0.5 * 0.998), "{ball:?}");
    }

    #[test]
    fn of_a_sprite_or_the_cursor_only_fear_passes_on_by_watching() {
        // M2 design §18.2 (Q2, Q4): whom to like a sprite finds out for
        // itself. The one it watches likes the sprite it plays with, and the
        // Cursor it goes to, and fears them both.
        let data = builtin();
        let needs = data.need_places().len();
        let (other, cursor) = (EntityId(9), Subject::Category(data.cursor_category()));
        let mut watched = unfading(&[]);
        let knows = |known: &mut SpriteMemory| {
            known.worth.fill(0.8);
            known.good = 0.6;
            known.fear = -0.6;
        };
        knows(watched.experience.remember(other, needs));
        knows(watched.experience.remember_the_cursor(needs));
        let tries = [
            (types::SPRITE, Verb::Play, Some(other)),
            (cursor, Verb::Approach, None),
        ];
        for (subject, verb, sprite) in tries {
            let topic = Topic::Tried {
                subject,
                verb,
                sprite,
            };
            chose(&mut watched, verb, subject, Some(1));
            let mut learner = unfading(&[]);
            learner.watched(&[watched.shown(topic, &data)], 0.5, &data);
            let experience = &learner.experience;
            let known = match topic {
                Topic::Tried {
                    sprite: Some(_), ..
                } => experience.individuals.get(&other),
                _ => experience.cursor.as_ref(),
            };
            let known = known.expect("it remembers whom it fears");
            assert!(close(known.fear, -0.3), "{topic:?}: {known:?}");
            assert!(
                known.worth.iter().all(|&w| w == 0.0),
                "{topic:?}: {known:?}"
            );
            assert_eq!(known.good, 0.0, "{topic:?}");
            let kind = about(&learner, subject);
            assert!(kind.worth.iter().all(|&w| w == 0.0) && kind.good == 0.0);
        }
    }

    #[test]
    fn a_try_shows_a_learner_only_a_lesson_it_could_take_on_and_hasn_t_learned() {
        // M2 design §18.1: a value the one trying shows (§18.2), past
        // lesson_threshold (.5) for it, below it for the learner, on the
        // same side or at 0.
        let data = builtin();
        let drive = data.physiology().indices.drive.expect("a breeding drive");
        let learner_id = EntityId(4);
        let tried = |verb, subject, sprite| Topic::Tried {
            subject,
            verb,
            sprite,
        };
        let shows =
            |brain: &Brain, topic, learner: &Brain| brain.shows(topic, learner, learner_id, &data);
        let drink = tried(Verb::Drink, types::WATER, None);
        let mut drinker = unfading(&[]);
        teach(&mut drinker, types::WATER).worth[1] = 0.6;
        let mut learner = unfading(&[]);
        assert!(
            !shows(&drinker, drink, &learner),
            "no thirst chose the drink"
        );
        chose(&mut drinker, Verb::Drink, types::WATER, Some(1));
        assert!(
            shows(&drinker, drink, &learner),
            "it knows nothing of water"
        );
        teach(&mut learner, types::WATER).worth[1] = 0.3;
        assert!(shows(&drinker, drink, &learner), "not yet past .5");
        teach(&mut learner, types::WATER).worth[1] = 0.5;
        assert!(!shows(&drinker, drink, &learner), "it has learned it");
        teach(&mut learner, types::WATER).worth[1] = -0.3;
        assert!(!shows(&drinker, drink, &learner), "it learned the opposite");
        // Never about mating (§18.3), nor about the learner itself.
        let mut player = unfading(&[]);
        teach(&mut player, types::BALL).worth[drive] = 0.9;
        chose(&mut player, Verb::Play, types::BALL, Some(drive));
        let play = tried(Verb::Play, types::BALL, None);
        assert!(!shows(&player, play, &unfading(&[])), "the drive's worth");
        let mut hitter = unfading(&[]);
        teach(&mut hitter, types::SPRITE).habits[column(Verb::Hit)] = 0.9;
        let hit_it = tried(Verb::Hit, types::SPRITE, Some(learner_id));
        assert!(
            !shows(&hitter, hit_it, &unfading(&[])),
            "a hit on the learner"
        );
        let hit_another = tried(Verb::Hit, types::SPRITE, Some(EntityId(5)));
        assert!(
            shows(&hitter, hit_another, &unfading(&[])),
            "a hit on another"
        );
    }
}
