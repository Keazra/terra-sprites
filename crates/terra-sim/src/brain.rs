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
    Experience, Links, Signals, SpriteMemory, Subject, TRACE_CAP, Touch, TraceEntry, still_counts,
    weight,
};
use crate::objects::EntityId;
use crate::perception::Target;
use crate::random::unit;
use crate::registry::{BrainParam, CategoryId, Verb};

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

/// Which way a thing's worth pushes a verb aimed at it (design v18 §5.5):
/// towards it for Approach and the interactions, and not at all for
/// Retreat, which is fear's (a bad thing is left alone, not backed away
/// from), or the targetless verbs.
fn side(verb: Verb) -> f32 {
    match verb {
        Verb::Retreat | Verb::Rest | Verb::Wander => 0.0,
        _ => 1.0,
    }
}

/// How fear pushes a verb aimed at a frightening thing (design v18 §5.5):
/// `flight` towards Retreat, `value_gain` away from Approach, Eat, Drink
/// and Play, and never towards or away from Hit, since fear that pushed
/// Hit started feuds (#62).
fn fear_push(verb: Verb, flight: f32, value_gain: f32) -> f32 {
    match verb {
        Verb::Retreat => flight,
        Verb::Approach | Verb::Eat | Verb::Drink | Verb::Play => -value_gain,
        _ => 0.0,
    }
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct BrainParams {
    /// In `BrainParam::ALL` order. A `Vec`, since serde serialises arrays
    /// only up to 32 long.
    values: Vec<f32>,
}

impl BrainParams {
    pub(crate) fn express(genome: &Genome, data: &DataPack) -> BrainParams {
        let ranges = &data.physiology().brain;
        let mut values: Vec<f32> = BrainParam::ALL
            .iter()
            .map(|&param| ranges.of(param).default)
            .collect();
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

/// Which sprite stands for sprites while the brain scores (design v18
/// §5.3, §5.5), and whether fear is quiet: a hit is still felt, or the
/// sprite is cornered, moments that are instinct's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub(crate) struct SpriteScoring {
    pub(crate) sprite: Option<EntityId>,
    pub(crate) quiet: bool,
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
}

/// A sprite's brain: its parameters, concepts and links (design §5.1).
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

/// How many learned things the memory lists (design §5.9).
const MEMORY_SIZE: usize = 5;

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
    pub(crate) fn new(genome: &Genome, data: &DataPack) -> Brain {
        let inputs = data.brain_inputs_in_order();
        let place = |id| {
            inputs
                .iter()
                .position(|input| input.id == id)
                .expect("a checked gene")
        };
        let mut concepts: Vec<Signature> = (0..inputs.len()).map(|i| vec![(i, false)]).collect();
        let mut decision = vec![vec![0.0; VERBS.len()]; inputs.len()];
        let mut attention = vec![vec![0.0; data.categories().len()]; inputs.len()];
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
                            decision.push(vec![0.0; VERBS.len()]);
                            concepts.len() - 1
                        }
                    };
                    decision[concept][column(verb)] = weight;
                }
                Gene::AttentionInstinct {
                    input,
                    category,
                    weight,
                } => attention[place(input)][data.category_index(category)] = weight,
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
            experience: Experience::default(),
            touched: None,
            reach_back: None,
            seen_cursor: false,
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
        let relief = &signals.relief;
        // A hurt done to it teaches fear of whoever did it (design v18 §5.6).
        if signals.hit
            && let Some(attacker) = signals.attacker
        {
            let rate = self.params.get(BrainParam::FearRate) * learning_rate_mod;
            let attacker = self.experience.remember(attacker, relief.len());
            attacker.fear = (attacker.fear - rate * signals.punishment).max(-1.0);
        }
        // A Cursor it can see is part of the lesson (design v29 §5.6): a pet
        // teaches it to like the Cursor, and a zap or a shock to fear it, as
        // well as what each teaches about what it touched.
        if signals.seen_cursor {
            let good = self.params.get(BrainParam::IndividualRateGood) * learning_rate_mod;
            let fear = self.params.get(BrainParam::FearRate) * learning_rate_mod;
            let cursor = self.experience.remember_the_cursor(relief.len());
            if signals.reach_back.is_some() {
                cursor.good = (cursor.good + good * signals.reward).min(1.0);
            }
            if signals.corrected {
                cursor.fear = (cursor.fear - fear * signals.punishment).max(-1.0);
            }
        }
        // A feared Cursor that stays near and does nothing wears the fear
        // off, fastest on the sprite's own tile: it comes to see it can't do
        // anything about it (design v29 §5.6).
        if !signals.corrected
            && let (Some(distance), Some(cursor)) =
                (signals.cursor_distance, self.experience.cursor.as_mut())
        {
            let reach = self.params.get(BrainParam::FearReach);
            let nearness = (1.0 - distance / reach).max(0.0);
            cursor.fear *= 1.0 - self.params.get(BrainParam::CursorCalming) * nearness;
        }
        // What the feelings are about: the latest try, while it's recent
        // (design §5.6). Relief and punishment look back the touch window, a
        // zap's or shock's too; reward, in a tick the Cursor rewarded the
        // sprite, looks back the Reward's reach back (design v21 §5.6).
        let within = |ticks: u64| self.touched.filter(|t| tick - t.tick <= ticks);
        let near = within(physiology.touch_window);
        let reached = signals.reach_back.map_or(near, within);
        // A crash after a shove by a Cursor it could see teaches fear of the
        // Cursor too, as a hit teaches fear of the hitter; the thing crashed
        // into is bad, as ever (design v29 §5.6).
        if let Some(Touch {
            by_cursor: true, ..
        }) = near
        {
            let fear = self.params.get(BrainParam::FearRate) * learning_rate_mod;
            let cursor = self.experience.remember_the_cursor(relief.len());
            cursor.fear = (cursor.fear - fear * signals.punishment).max(-1.0);
        }
        let needs = data.need_places().len();
        if let Some(Touch {
            subject,
            sprite,
            novelty,
            ..
        }) = reached.or(near)
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
            let experience = &mut self.experience;
            // How it went with something new teaches the worth of new things.
            let relieved: f32 = relief.iter().sum();
            let felt = good * (relieved + reward) - bad * punishment;
            experience.new_things = (experience.new_things + felt * novelty).clamp(-1.0, 1.0);
        }
        // Habits, along the trace: each aimed verb and the thing it attended.
        // A fruitless try disappoints the need that chose it. In a tick the
        // Cursor rewarded the sprite, the reward goes instead to the habit of
        // the latest try within the Reward's reach back, at full weight; in a
        // tick it corrected it, the punishment likewise to the latest try
        // within the touch window (design v21 §5.6).
        let rate = self.params.get(BrainParam::HabitRate) * learning_rate_mod;
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
                    let habit = &mut known.habits[column(verb)];
                    *habit = (*habit + rate * felt).clamp(-1.0, 1.0);
                }
                None => r += felt,
            }
        }
        // While a crash is the thing touched, the trace teaches nothing
        // either: it holds what the sprite chose before it was taken hold
        // of, which the crash's pain isn't about (design v25 §5.6).
        let crashed = near.is_some_and(|touch| touch.verb.is_none());
        let tried = self.touched.filter(|_| signals.fruitless).map(|t| t.tick);
        if !crashed && (r != 0.0 || tried.is_some()) {
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
        // A sprite touched that taught it nothing isn't remembered, so it
        // doesn't count towards sprites in general (design v18 §5.6).
        self.experience.forget_faded(physiology.forget_below);
        let lessons = self.lessons(data);
        self.fade(physiology.forget_below);
        lessons
    }

    /// The learned values that are `lesson_threshold` or more from 0 for the
    /// first time, as `(learned, rose)`, in listed order. Each is marked by
    /// what it's about, so it's a lesson only once (design §5.6), however
    /// the sprites it remembers come and go (design v18 §5.6).
    fn lessons(&mut self, data: &DataPack) -> Vec<(Learned, bool)> {
        let threshold = data.physiology().lesson_threshold;
        let mut found = Vec::new();
        for learned in self.learned(data) {
            if learned.amount.abs() >= threshold
                && self.experience.taught.insert(learned.learned.clone())
            {
                found.push((learned.learned, learned.amount > 0.0));
            }
        }
        found
    }

    /// One tick of fading (design §5.6): good and bad, fear and habits each
    /// by their own rate. A remembered sprite faded until everything about
    /// it is nearer 0 than `forget_below` is forgotten (design v18 §5.6).
    fn fade(&mut self, forget_below: f32) {
        let keep = |param| 1.0 - self.params.get(param);
        let (good, bad, habit, bad_habit) = (
            keep(BrainParam::WorthFadeGood),
            keep(BrainParam::WorthFadeBad),
            keep(BrainParam::HabitFade),
            keep(BrainParam::HabitFadeBad),
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
        }
        experience.new_things *= good;
        let fear = keep(BrainParam::FearFade);
        let individuals = experience.individuals.values_mut();
        for individual in individuals.chain(&mut experience.cursor) {
            for value in &mut individual.worth {
                *value *= good;
            }
            individual.good *= good;
            individual.bad *= bad;
            individual.fear *= fear;
        }
        experience.forget_faded(forget_below);
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

    /// What the sprite has learned, furthest from nothing first, up to
    /// five (design §5.9); ties keep the order they're listed in.
    pub(crate) fn memory(&self, data: &DataPack) -> Vec<Memory> {
        let mut memory = self.learned(data);
        memory.retain(|m| m.amount != 0.0);
        // A stable sort keeps a tie in listed order.
        memory.sort_by(|a, b| b.amount.abs().total_cmp(&a.amount.abs()));
        memory.truncate(MEMORY_SIZE);
        memory
    }

    /// Every value a brain learns, named, always in the same order: worth
    /// for each need then general good, bad, habits, and new things, each in
    /// category order, and within a category in object type order (design
    /// §5.6).
    fn learned(&self, data: &DataPack) -> Vec<Memory> {
        let inputs = data.brain_inputs_in_order();
        let mut memory: Vec<Memory> = Vec::new();
        // Sprites in general are the summary of the ones it knows (design
        // v18 §5.6), never learned directly.
        let in_general = self.sprites_in_general(data.need_places().len());
        let categories = data.categories();
        let summaries: BTreeMap<CategoryId, Summary> = categories
            .iter()
            .map(|category| (category.id, self.category_summary(category.id, data)))
            .collect();
        // Each thing's worth for each need, general good and bad.
        let mut things: Vec<(Thing, &[f32], f32, f32)> = Vec::new();
        for category in categories {
            if category.id == data.sprite_category() {
                let thing = category_thing(category.id, data);
                things.push((thing, &in_general.worth, in_general.good, in_general.bad));
                continue;
            }
            // There's one Cursor, learned about as an individual (design v29
            // §5.6), so the category has no summary.
            if category.id == data.cursor_category() {
                if let Some(known) = &self.experience.cursor {
                    things.push((Thing::Cursor, &known.worth, known.good, known.bad));
                }
                continue;
            }
            let types = self.experience.types.iter();
            for (&subject, known) in types.filter(|(s, _)| s.category(data) == category.id) {
                let thing = subject_thing(subject, data);
                things.push((thing, &known.worth, known.good, known.bad));
            }
            // Then the category's summary, never learned directly (design
            // v19 §5.6).
            let summary = &summaries[&category.id];
            let thing = category_thing(category.id, data);
            things.push((thing, &summary.worth, summary.good, summary.bad));
        }
        for (need, &place) in data.need_places().iter().enumerate() {
            for (thing, worth, ..) in &things {
                memory.push(Memory {
                    learned: Learned::Worth {
                        thing: thing.clone(),
                        need: Some(inputs[place].name.clone()),
                    },
                    amount: worth[need],
                });
            }
        }
        for &(ref thing, _, good, _) in &things {
            let learned = Learned::Worth {
                thing: thing.clone(),
                need: None,
            };
            memory.push(Memory {
                learned,
                amount: good,
            });
        }
        for &(ref thing, _, _, bad) in &things {
            let learned = Learned::Bad {
                thing: thing.clone(),
            };
            memory.push(Memory {
                learned,
                amount: bad,
            });
        }
        // Habits are about object types, sprites' too (design v19 §5.6).
        let habits = categories.iter().flat_map(|category| {
            let types = self.experience.types.iter();
            types.filter(move |(s, _)| s.category(data) == category.id)
        });
        for (&subject, known) in habits {
            for (&verb, &value) in VERBS.iter().zip(&known.habits) {
                let learned = Learned::Habit {
                    thing: subject_thing(subject, data),
                    verb,
                };
                memory.push(Memory {
                    learned,
                    amount: value,
                });
            }
        }
        memory.push(Memory {
            learned: Learned::NewThings,
            amount: self.experience.new_things,
        });
        // The sprites it remembers (design v18 §5.9), in ID order.
        for (&id, individual) in &self.experience.individuals {
            let thing = || Thing::Sprite(id);
            for (&value, &place) in individual.worth.iter().zip(data.need_places()) {
                let learned = Learned::Worth {
                    thing: thing(),
                    need: Some(inputs[place].name.clone()),
                };
                memory.push(Memory {
                    learned,
                    amount: value,
                });
            }
            let values = [
                (
                    Learned::Worth {
                        thing: thing(),
                        need: None,
                    },
                    individual.good,
                ),
                (Learned::Bad { thing: thing() }, individual.bad),
                (Learned::Fear { thing: thing() }, individual.fear),
            ];
            for (learned, amount) in values {
                memory.push(Memory { learned, amount });
            }
        }
        // Fear of sprites in general comes of the sprites above, so it's
        // listed, and a lesson, after them.
        memory.push(Memory {
            learned: Learned::Fear {
                thing: category_thing(data.sprite_category(), data),
            },
            amount: in_general.fear,
        });
        if let Some(cursor) = &self.experience.cursor {
            memory.push(Memory {
                learned: Learned::Fear {
                    thing: Thing::Cursor,
                },
                amount: cursor.fear,
            });
        }
        memory
    }

    /// Sprites in general (design v18 §5.6): each value's mean over the
    /// sprites it remembers, at no strength while it knows one, and in full
    /// once it knows `generalise`. For `needs` needs.
    pub(crate) fn sprites_in_general(&self, needs: usize) -> SpriteMemory {
        let known = &self.experience.individuals;
        let share = summary_share(known.len(), self.params.get(BrainParam::Generalise));
        let mut summary = SpriteMemory::new(needs);
        if share == 0.0 {
            return summary;
        }
        for individual in known.values() {
            for (summary, worth) in summary.worth.iter_mut().zip(&individual.worth) {
                *summary += share * worth;
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
        let known: Vec<_> = self
            .experience
            .types
            .iter()
            .filter(|(subject, memory)| memory.touched && subject.category(data) == category)
            .map(|(_, memory)| memory)
            .collect();
        let share = summary_share(known.len(), self.params.get(BrainParam::GeneraliseTypes));
        let mut summary = Summary {
            worth: vec![0.0; data.need_places().len()],
            good: 0.0,
            bad: 0.0,
        };
        if share == 0.0 {
            return summary;
        }
        for known in known {
            for (summary, worth) in summary.worth.iter_mut().zip(&known.worth) {
                *summary += share * worth;
            }
            summary.good += share * known.good;
            summary.bad += share * known.bad;
        }
        summary
    }

    /// Checks the brain's state (design §5.6): instinct links within
    /// [−1, 1], what it has learned within its ranges, a felt value that's a
    /// number, and a trace within its cap.
    pub(crate) fn check(&self) -> Result<(), String> {
        self.decision.check()?;
        self.attention.check()?;
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
            let parts = self.aimed_parts(subject, scoring, &snapshot.inputs, data);
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
    /// `candidates` gives what each category's candidate is learned about as
    /// and its normalized distance. Target inputs never count, whatever
    /// `inputs` holds for them.
    pub(crate) fn attention_scores(
        &self,
        inputs: &[f32],
        candidates: &BTreeMap<CategoryId, (Subject, f32)>,
        curiosity_mod: f32,
        scoring: SpriteScoring,
        data: &DataPack,
    ) -> BTreeMap<CategoryId, f32> {
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
        let vigilance = self.params.get(BrainParam::Vigilance);
        let boldness = self.boldness(curiosity_mod);
        candidates
            .iter()
            .map(|(&category, &(subject, distance))| {
                let c = data.category_index(category);
                let instinct: f32 = state
                    .iter()
                    .map(|&i| inputs[i] * self.attention.get(i, c))
                    .sum();
                let worth = value_gain * self.worth_of(subject, scoring.sprite, inputs, data);
                let curious = curiosity * self.novelty(subject) * boldness;
                // Fear catches the eye (design v18 §5.3).
                let watchful = vigilance * self.fright(category, scoring, distance, data);
                (
                    category,
                    instinct + salience * (1.0 - distance) + worth + curious + watchful,
                )
            })
            .collect()
    }

    /// How much a thing learned about as `subject`, the particular `sprite`
    /// if it's one, at normalized `distance`, draws the eye beyond the
    /// instincts about its category (design v19 §3.6, §5.3): its nearness,
    /// what it's worth, how new it is, and, for a sprite, how frightening it
    /// is while near. `inputs` gives the State inputs, and `curiosity_mod`
    /// the receptor target wariness lowers.
    pub(crate) fn draw(
        &self,
        subject: Subject,
        sprite: Option<EntityId>,
        distance: f32,
        inputs: &[f32],
        curiosity_mod: f32,
        data: &DataPack,
    ) -> f32 {
        let scoring = SpriteScoring {
            sprite,
            quiet: false,
        };
        let boldness = self.boldness(curiosity_mod);
        self.params.get(BrainParam::SalienceGain) * (1.0 - distance)
            + self.params.get(BrainParam::ValueGain) * self.worth_of(subject, sprite, inputs, data)
            + self.params.get(BrainParam::Curiosity) * self.novelty(subject) * boldness
            + self.params.get(BrainParam::Vigilance)
                * self.fright(subject.category(data), scoring, distance, data)
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
        data: &DataPack,
    ) -> f32 {
        if scoring.quiet {
            return 0.0;
        }
        let fear = if category == data.sprite_category() {
            let needs = data.need_places().len();
            self.memory_of(scoring.sprite, needs).fear
        } else if category == data.cursor_category() {
            self.experience.cursor.as_ref().map_or(0.0, |c| c.fear)
        } else {
            return 0.0;
        };
        let reach = self.params.get(BrainParam::FearReach);
        -fear * (1.0 - distance / reach).max(0.0)
    }

    /// What the brain knows of `sprite` (design v18 §5.6): what it remembers
    /// of it, or sprites in general for one it doesn't remember, or none.
    fn memory_of(&self, sprite: Option<EntityId>, needs: usize) -> SpriteMemory {
        sprite
            .and_then(|sprite| self.experience.individuals.get(&sprite))
            .cloned()
            .unwrap_or_else(|| self.sprites_in_general(needs))
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

    /// Each verb's score (design §5.5), in `VERBS` order: the concepts'
    /// activations through the instinct links, and for the thing `aimed`
    /// at, learned about as that subject, its worth, which draws the sprite
    /// to it or makes it back away.
    pub(crate) fn scores(
        &self,
        activations: &[f32],
        inputs: &[f32],
        aimed: Option<Subject>,
        scoring: SpriteScoring,
        data: &DataPack,
    ) -> [f32; VERBS.len()] {
        let mut scores = [0.0; VERBS.len()];
        for (a, links) in activations.iter().zip(self.decision.rows()) {
            for (score, w) in scores.iter_mut().zip(links) {
                *score += a * w;
            }
        }
        if let Some(subject) = aimed {
            let parts = self.aimed_parts(subject, scoring, inputs, data);
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
        data: &DataPack,
    ) -> [[f32; 3]; VERBS.len()] {
        let value_gain = self.params.get(BrainParam::ValueGain);
        let flight = self.params.get(BrainParam::Flight);
        let category = subject.category(data);
        let worth = value_gain * self.worth_of(subject, scoring.sprite, inputs, data);
        let fear = self.fright(category, scoring, target_distance(inputs, data), data);
        let habits = self
            .experience
            .types
            .get(&subject)
            .map_or([0.0; VERBS.len()], |known| known.habits);
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
    /// `sprite`'s, or sprites in general's (design v18 §5.6).
    fn worth_of(
        &self,
        subject: Subject,
        sprite: Option<EntityId>,
        inputs: &[f32],
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
            let known = self.memory_of(sprite, data.need_places().len());
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
            Some(known) => now(&known.worth, known.good, known.bad),
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

/// The `target_distance` input's value in `inputs` (design §5.2).
fn target_distance(inputs: &[f32], data: &DataPack) -> f32 {
    data.brain_inputs_in_order()
        .iter()
        .position(|input| matches!(input.source, Source::TargetDistance))
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
    use crate::learning::TypeMemory;

    fn builtin() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

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
        Brain::new(&genome, &data)
    }

    /// A brain of `genes` whose learned values never fade, so they read exactly.
    fn unfading(genes: &[&str]) -> Brain {
        let no_fade = [
            r#"BrainParam(param: "worth_fade_good", value: 0.0)"#,
            r#"BrainParam(param: "worth_fade_bad", value: 0.0)"#,
            r#"BrainParam(param: "habit_fade", value: 0.0)"#,
            r#"BrainParam(param: "habit_fade_bad", value: 0.0)"#,
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
    fn target_inputs_never_feed_attention() {
        let data = builtin();
        let brain = brain(&[
            r#"AttentionInstinct(input: "hunger", category: "bush", weight: 0.8)"#,
            r#"BrainParam(param: "salience_gain", value: 0.5)"#,
        ]);
        let candidates = BTreeMap::from([
            (BUSH, (types::BERRY_BUSH, 0.4)),
            (WATER, (types::WATER, 0.0)),
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
    fn the_brain_draws_from_the_rng_only_when_choosing_afresh() {
        let mut brain = brain(&[]);
        let scores = BTreeMap::from([(WATER, 0.3), (TOY, 0.5)]);
        let verbs = [0.1; VERBS.len()];
        let available = available(true, false);
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

    #[test]
    fn a_thing_s_worth_for_a_need_counts_as_much_as_the_sprite_needs_it() {
        // Design v16 §5.3: value_gain (1) × Σ need × worth, plus good and bad.
        let data = builtin();
        let mut brain = brain(&[]);
        teach(&mut brain, types::BERRY).worth[0] = 0.5;
        teach(&mut brain, types::THORNBUSH).bad = -0.4;
        let candidates = BTreeMap::from([
            (FRUIT, (types::BERRY, 1.0)),
            (BUSH, (types::THORNBUSH, 1.0)),
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
            (TOY, (types::BALL, 1.0)),
            (BUSH, (types::THORNBUSH, 1.0)),
            (FRUIT, (types::BERRY, 1.0)),
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
        let candidates = BTreeMap::from([(FRUIT, (types::BERRY, 1.0)), (TOY, (types::BALL, 1.0))]);
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
}
