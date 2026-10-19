//! Step 4 (design §5.6): each sprite reads its needs' relief and uses up its
//! reward and punishment, and learns what things are worth and its habits.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::action::chebyshev;
use crate::brain::{Learned, VERBS};
use crate::courtship::Company;
use crate::data::DataPack;
use crate::events::{Event, EventKind};
use crate::map::Pos;
use crate::objects::EntityId;
use crate::perception::Target;
use crate::physiology::RememberedPlaces;
use crate::registry::{BrainParam, CategoryId, Verb};
use crate::world::WorldState;

/// Instinct links (design §5.1, §5.4): a row per concept or input, a column
/// per verb or category, each at the weight the genome gave it. They never
/// change in a sprite's life; learning is worth and habits (§5.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Links {
    w: Vec<Vec<f32>>,
}

impl Links {
    /// Links at the weights `birth` gives, each clamped to [−1, 1]: spawn
    /// variation can take a 1.0 instinct past 1.
    pub(crate) fn new(mut birth: Vec<Vec<f32>>) -> Links {
        for w in birth.iter_mut().flatten() {
            *w = w.clamp(-1.0, 1.0);
        }
        Links { w: birth }
    }

    /// Every row's weights.
    pub(crate) fn rows(&self) -> &[Vec<f32>] {
        &self.w
    }

    /// The weight of the link in `row` and `column`.
    pub(crate) fn get(&self, row: usize, column: usize) -> f32 {
        self.w[row][column]
    }

    /// Checks every weight is a number within [−1, 1], or says which isn't.
    pub(crate) fn check(&self) -> Result<(), String> {
        match self
            .w
            .iter()
            .flatten()
            .copied()
            .find(|w| !(-1.0..=1.0).contains(w))
        {
            Some(w) => Err(format!("a link at {w}, outside -1 to 1")),
            None => Ok(()),
        }
    }

    /// Widens every row to `columns` with links at 0: a save's decision
    /// instincts from before a verb was added (M2 design §5).
    pub(crate) fn widen(&mut self, columns: usize) {
        for row in &mut self.w {
            if row.len() < columns {
                row.resize(columns, 0.0);
            }
        }
    }

    /// Sets the weight in `row` and `column` to `w`, clamped to [−1, 1] as
    /// `new` clamps it.
    pub(crate) fn set_clamped(&mut self, row: usize, column: usize, w: f32) {
        self.w[row][column] = w.clamp(-1.0, 1.0);
    }

    /// Sets the weight in `row` and `column`, unchecked.
    #[cfg(test)]
    pub(crate) fn set(&mut self, row: usize, column: usize, w: f32) {
        self.w[row][column] = w;
    }
}

/// Reads a value for each verb, in `VERBS` order. A save from before Mate
/// and BreedAlone, or Court and Call, has fewer, and theirs are 0 (M2
/// design §5, §10, §15).
pub(crate) fn per_verb<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[f32; VERBS.len()], D::Error> {
    let values = Vec::<f32>::deserialize(deserializer)?;
    let mut row = [0.0; VERBS.len()];
    if values.len() > VERBS.len() {
        return Err(serde::de::Error::invalid_length(
            values.len(),
            &"a value for each verb",
        ));
    }
    row[..values.len()].copy_from_slice(&values);
    Ok(row)
}

/// What a sprite learns about a thing as (design v19 §5.6): its object type,
/// by its stable ID, or its category, for water or sprites in a pack with no
/// object type for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum Subject {
    ObjectType(u16),
    Category(CategoryId),
}

impl Subject {
    /// What a thing of `category`, and of the object type with the stable ID
    /// `object_type` if it has one, is learned about as.
    pub(crate) fn of(category: CategoryId, object_type: Option<u16>) -> Subject {
        object_type.map_or(Subject::Category(category), Subject::ObjectType)
    }

    /// The category it's in.
    pub(crate) fn category(self, data: &DataPack) -> CategoryId {
        match self {
            Subject::ObjectType(id) => {
                data.object_type(id)
                    .expect("an object type in the pack")
                    .category
            }
            Subject::Category(category) => category,
        }
    }

    /// Whether `data` has it: its object type, or its category.
    pub(crate) fn fits(self, data: &DataPack) -> bool {
        match self {
            Subject::ObjectType(id) => data.object_type(id).is_some(),
            Subject::Category(category) => data.category(category).is_some(),
        }
    }
}

/// What a sprite has learned about one object type (design v19 §5.6): its
/// worth for each need and in general, how bad it is, its habits and its
/// taught habits (design §5.6, #187), what it took on by watching of how
/// bad it is and of its habits (M2 design §19.7), how familiar it is, and
/// whether it knows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TypeMemory {
    /// Worth for each need, in the pack's needs order (0 to 1).
    pub(crate) worth: Vec<f32>,
    /// General good, from `reward` (0 to 1).
    pub(crate) good: f32,
    /// Bad, from `punishment`, as it found out itself (−1 to 0).
    pub(crate) bad: f32,
    /// Habits: doing each verb to it, in `VERBS` order.
    #[serde(deserialize_with = "per_verb")]
    pub(crate) habits: [f32; VERBS.len()],
    /// Taught habits: what the Cursor taught about doing each verb to it,
    /// kept apart from the habits it learns itself (design §5.6, #187), in
    /// `VERBS` order. A save from before them has none, and one from before
    /// Court and Call has fewer (M2 design §15).
    #[serde(default, deserialize_with = "per_verb")]
    pub(crate) taught_habits: [f32; VERBS.len()],
    /// How bad it is, as it took on by watching, kept apart from what it
    /// found out itself (−1 to 0; M2 design §19.7). A save from before
    /// Slice 29 has none.
    #[serde(default)]
    pub(crate) watched_bad: f32,
    /// Watched habits: what it took on by watching about doing each verb to
    /// it, kept apart from its own and its taught habits (M2 design §19.7),
    /// in `VERBS` order. A save from before Slice 29 has none.
    #[serde(default, deserialize_with = "per_verb")]
    pub(crate) watched_habits: [f32; VERBS.len()],
    /// How familiar it is, from attending to it (0 to 1).
    pub(crate) familiarity: f32,
    /// Whether it has touched one, and so knows it: until then, it's
    /// judged by its category's summary.
    pub(crate) touched: bool,
}

impl TypeMemory {
    /// An object type newly learned about, for `needs` needs: nothing yet.
    pub(crate) fn new(needs: usize) -> TypeMemory {
        TypeMemory {
            worth: vec![0.0; needs],
            good: 0.0,
            bad: 0.0,
            habits: [0.0; VERBS.len()],
            taught_habits: [0.0; VERBS.len()],
            watched_bad: 0.0,
            watched_habits: [0.0; VERBS.len()],
            familiarity: 0.0,
            touched: false,
        }
    }

    /// Its habit of doing `VERBS[i]` to it, as a decision weighs it: the
    /// one it learned, the one the Cursor taught (design §5.5, #187) and
    /// the one it took on by watching (M2 design §19.7), together.
    pub(crate) fn habit(&self, i: usize) -> f32 {
        (self.habits[i] + self.taught_habits[i] + self.watched_habits[i]).clamp(-1.0, 1.0)
    }

    /// Its habit of doing `VERBS[i]` to it at first hand: the one it
    /// learned and the one the Cursor taught, which is what watching it
    /// shows another (M2 design §19.7).
    pub(crate) fn first_hand_habit(&self, i: usize) -> f32 {
        (self.habits[i] + self.taught_habits[i]).clamp(-1.0, 1.0)
    }

    /// How bad it is, as a decision weighs it: what it found out itself
    /// and what it took on by watching, together (M2 design §19.7).
    pub(crate) fn badness(&self) -> f32 {
        (self.bad + self.watched_bad).clamp(-1.0, 0.0)
    }
}

/// What a brain has learned (design §5.6), all starting at 0: what each
/// object type is worth for each need and in general, how bad it is, its
/// habits and how familiar it is; the sprites it remembers; and the worth of
/// new things.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Experience {
    /// What it has learned about each object type (design v19 §5.6). Object
    /// types are never forgotten.
    pub(crate) types: BTreeMap<Subject, TypeMemory>,
    /// The worth of new things.
    pub(crate) new_things: f32,
    /// The sprites it remembers (design v18 §5.6), by ID.
    pub(crate) individuals: BTreeMap<EntityId, SpriteMemory>,
    /// What it remembers of the Cursor, learned about as one individual, as
    /// a sprite is (design v29 §5.6), if anything.
    pub(crate) cursor: Option<SpriteMemory>,
    /// Which learned values have been lessons (design §5.6).
    pub(crate) taught: BTreeSet<Learned>,
    /// Each need's level at the last step 4, to read its relief from.
    pub(crate) needs_before: Option<Vec<f32>>,
    /// The places it remembers (M2 design §7), oldest first.
    #[serde(default)]
    pub(crate) places: Vec<PlaceMemory>,
}

/// A place a sprite remembers (M2 design §7): where something that stays
/// put eased one of its needs, and how well it remembers it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct PlaceMemory {
    /// What it found there is learned about as.
    pub(crate) subject: Subject,
    /// The thing it found there: water, or a fixed object.
    pub(crate) target: Target,
    /// The thing's tile.
    pub(crate) at: Pos,
    /// Its recall: how well it remembers it, from 1 when it was last eased
    /// there, fading towards 0. Saves from before it had this name call it
    /// `strength`.
    #[serde(alias = "strength")]
    pub(crate) recall: f32,
}

impl Experience {
    /// Whether a loaded experience fits `data` (design §2.8): each thing
    /// learned about is in the pack, and each worth, and the needs' levels
    /// it keeps, cover the pack's needs.
    pub(crate) fn fits(&self, data: &DataPack) -> bool {
        let needs = data.needs().count();
        let mut remembered = self.individuals.values().chain(&self.cursor);
        let levels = self.needs_before.as_ref();
        self.places.iter().all(|place| place.subject.fits(data))
            && self
                .types
                .iter()
                .all(|(subject, known)| subject.fits(data) && known.worth.len() == needs)
            && remembered.all(|known| known.worth.len() == needs)
            && levels.is_none_or(|levels| levels.len() == needs)
    }

    /// Checks every learned value is a number within its range (design
    /// §5.6): worth, good and familiarity 0 to 1, bad −1 to 0, habits and
    /// the worth of new things −1 to 1. Says which isn't.
    pub(crate) fn check(&self) -> Result<(), String> {
        for known in self.types.values() {
            within(&known.worth, (0.0, 1.0), "a worth")?;
            within([&known.good], (0.0, 1.0), "a good")?;
            within([&known.bad], (-1.0, 0.0), "a bad")?;
            within(&known.habits, (-1.0, 1.0), "a habit")?;
            within(&known.taught_habits, (-1.0, 1.0), "a taught habit")?;
            within([&known.watched_bad], (-1.0, 0.0), "a watched bad")?;
            within(&known.watched_habits, (-1.0, 1.0), "a watched habit")?;
            within([&known.familiarity], (0.0, 1.0), "a familiarity")?;
        }
        within([&self.new_things], (-1.0, 1.0), "the worth of new things")?;
        for individual in self.individuals.values().chain(&self.cursor) {
            within(&individual.worth, (0.0, 1.0), "a sprite's worth")?;
            within([&individual.good], (0.0, 1.0), "a sprite's good")?;
            within([&individual.bad], (-1.0, 0.0), "a sprite's bad")?;
            within([&individual.fear], (-1.0, 0.0), "a sprite's fear")?;
            within(
                [&individual.familiarity],
                (0.0, 1.0),
                "a sprite's familiarity",
            )?;
        }
        for place in &self.places {
            within([&place.recall], (0.0, 1.0), "a place's recall")?;
        }
        Ok(())
    }
}

impl Experience {
    /// What it has learned about `subject`, starting afresh, for `needs`
    /// needs, if it has learned nothing yet (design v19 §5.6).
    pub(crate) fn learn_about(&mut self, subject: Subject, needs: usize) -> &mut TypeMemory {
        self.types
            .entry(subject)
            .or_insert_with(|| TypeMemory::new(needs))
    }

    /// What it remembers of the Cursor, remembering it afresh, for `needs`
    /// needs, if it doesn't yet (design v29 §5.6).
    pub(crate) fn remember_the_cursor(&mut self, needs: usize) -> &mut SpriteMemory {
        self.cursor.get_or_insert_with(|| SpriteMemory::new(needs))
    }

    /// What it remembers of `sprite`, remembering it afresh, for `needs`
    /// needs, if it doesn't yet (design v18 §5.6).
    pub(crate) fn remember(&mut self, sprite: EntityId, needs: usize) -> &mut SpriteMemory {
        self.individuals
            .entry(sprite)
            .or_insert_with(|| SpriteMemory::new(needs))
    }

    /// Its fear of `sprite`, or of the Cursor for none, moved `share` of the
    /// way towards `theirs`, as watching moves it (M2 design §18.2), for
    /// `needs` needs. It's remembered for it only if it moves.
    pub(crate) fn fear_more(
        &mut self,
        sprite: Option<EntityId>,
        theirs: f32,
        share: f32,
        needs: usize,
    ) {
        let known = match sprite {
            Some(sprite) => self.individuals.get(&sprite),
            None => self.cursor.as_ref(),
        };
        let mine = known.map_or(0.0, |known| known.fear);
        let moved = toward(mine, theirs, share);
        if moved != mine {
            let known = match sprite {
                Some(sprite) => self.remember(sprite, needs),
                None => self.remember_the_cursor(needs),
            };
            known.fear = moved;
        }
    }

    /// Gets to know `sprite` better by `gain` (M2 design §15.2), for `needs`
    /// needs: one it sees, or plays with, is remembered, from `below`, the
    /// level it's forgotten under, so a glimpse is forgotten once it fades
    /// by more than `gain`.
    pub(crate) fn befriend(&mut self, sprite: EntityId, gain: f32, needs: usize, below: f32) {
        let known = self.remember(sprite, needs);
        known.familiarity = (known.familiarity.max(below) + gain).min(1.0);
    }

    /// Forgets every remembered sprite, and the Cursor, that everything
    /// learned about is nearer 0 than `below` (design v18, v29 §5.6).
    pub(crate) fn forget_faded(&mut self, below: f32) {
        self.individuals.retain(|_, memory| !memory.faded(below));
        self.cursor = self.cursor.take().filter(|memory| !memory.faded(below));
    }

    /// Remembers that `subject`'s `target`, on `at`, eased a need just now
    /// (M2 design §7.2): fully, as one place on `at` with every place of its
    /// kind remembered within `merge` tiles, so no two places of a kind are
    /// that near. A new place past `per_kind` pushes out the faintest of its
    /// kind, and one past `held` the faintest of all: the oldest of the
    /// faintest.
    pub(crate) fn remember_place(
        &mut self,
        subject: Subject,
        target: Target,
        at: Pos,
        limits: RememberedPlaces,
    ) {
        let place = PlaceMemory {
            subject,
            target,
            at,
            recall: 1.0,
        };
        let near = |p: &PlaceMemory| p.subject == subject && chebyshev(p.at, at) <= limits.merge;
        let merged = self.places.iter().any(near);
        self.places.retain(|p| !near(p));
        let of_kind = self.places.iter().filter(|p| p.subject == subject).count();
        if !merged {
            if of_kind >= usize::from(limits.per_kind) {
                self.forget_faintest(|p| p.subject == subject);
            } else if self.places.len() >= usize::from(limits.held) {
                self.forget_faintest(|_| true);
            }
        }
        self.places.push(place);
    }

    /// Brings places remembered under other `limits`, such as in a save from
    /// before `per_kind`, within these (M2 design §7.2): of two of a kind
    /// within `merge` tiles the older is forgotten, then the faintest of a
    /// kind past `per_kind`, then the faintest past `held`.
    pub(crate) fn keep_within(&mut self, limits: RememberedPlaces) {
        let mut kept: Vec<PlaceMemory> = Vec::with_capacity(self.places.len());
        for place in self.places.drain(..).rev() {
            let near = kept
                .iter()
                .any(|k| k.subject == place.subject && chebyshev(k.at, place.at) <= limits.merge);
            if !near {
                kept.push(place);
            }
        }
        kept.reverse();
        self.places = kept;
        while let Some(subject) = self.places.iter().map(|p| p.subject).find(|&subject| {
            let of_kind = self.places.iter().filter(|p| p.subject == subject).count();
            of_kind > usize::from(limits.per_kind)
        }) {
            self.forget_faintest(|p| p.subject == subject);
        }
        while self.places.len() > usize::from(limits.held) {
            self.forget_faintest(|_| true);
        }
    }

    /// Forgets the faintest of the places `which` picks, the oldest of the
    /// faintest, if it picks any.
    fn forget_faintest(&mut self, which: impl Fn(&PlaceMemory) -> bool) {
        let faintest = self
            .places
            .iter()
            .enumerate()
            .filter(|(_, p)| which(p))
            .min_by(|(_, a), (_, b)| a.recall.total_cmp(&b.recall))
            .map(|(i, _)| i);
        if let Some(faintest) = faintest {
            self.places.remove(faintest);
        }
    }

    /// How well it remembers a place of `subject` within `merge` tiles of
    /// `at` (M2 design §7.3), the best if more than one: 0 if it doesn't. A
    /// trip keeps its pull when its place merges into one near it.
    pub(crate) fn recall(&self, subject: Subject, at: Pos, merge: u16) -> f32 {
        self.places
            .iter()
            .filter(|place| place.subject == subject && chebyshev(place.at, at) <= merge)
            .map(|place| place.recall)
            .fold(0.0, f32::max)
    }

    /// Forgets the place of `target` (M2 design §7): it's gone, or there's
    /// no way there.
    pub(crate) fn forget_place(&mut self, target: Target) {
        self.places.retain(|place| place.target != target);
    }

    /// Forgets every remembered sprite not in `alive`: a sprite that has
    /// died is forgotten (design v18 §5.6).
    pub(crate) fn forget_dead(&mut self, alive: &BTreeSet<EntityId>) {
        self.individuals.retain(|sprite, _| alive.contains(sprite));
    }
}

/// What a sprite has learned about one other sprite (design v18 §5.6), or
/// the Cursor (design v29 §5.6): its worth for each need and in general, how
/// bad it is, and how frightening.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SpriteMemory {
    /// Worth for each need, in the pack's needs order (0 to 1).
    pub(crate) worth: Vec<f32>,
    /// General good, from `reward` (0 to 1).
    pub(crate) good: f32,
    /// Bad, from hurting itself on it (−1 to 0).
    pub(crate) bad: f32,
    /// Fear, from its hurting the sprite (−1 to 0).
    pub(crate) fear: f32,
    /// How well it knows the sprite, from seeing it and playing with it (0
    /// to 1; M2 design §15.2). Always 0 for the Cursor. A save from before
    /// it has none.
    #[serde(default)]
    pub(crate) familiarity: f32,
}

impl SpriteMemory {
    /// Whether everything learned about it, familiarity included, is
    /// nearer 0 than `below`.
    pub(crate) fn faded(&self, below: f32) -> bool {
        self.familiarity < below && !self.taught(below)
    }

    /// Whether it has learned what the sprite is worth, or to fear it: a
    /// value at least `below` from 0. One it only knows by sight hasn't,
    /// and is judged as sprites in general are (M2 design §15.2).
    pub(crate) fn taught(&self, below: f32) -> bool {
        let learned = |v: f32| v.abs() >= below;
        learned(self.good)
            || learned(self.bad)
            || learned(self.fear)
            || self.worth.iter().any(|&v| learned(v))
    }

    /// A sprite newly remembered, for `needs` needs: nothing learned yet.
    pub(crate) fn new(needs: usize) -> SpriteMemory {
        SpriteMemory {
            worth: vec![0.0; needs],
            good: 0.0,
            bad: 0.0,
            fear: 0.0,
            familiarity: 0.0,
        }
    }
}

/// Checks each of `values` is a number from `low` to `high`, or says which
/// of `what` isn't.
fn within<'a>(
    values: impl IntoIterator<Item = &'a f32>,
    (low, high): (f32, f32),
    what: &str,
) -> Result<(), String> {
    match values.into_iter().find(|v| !(low..=high).contains(*v)) {
        Some(v) => Err(format!("{what} at {v}, outside {low} to {high}")),
        None => Ok(()),
    }
}

/// What a sprite tried a verb on, and when (design §5.6): the thing a
/// feeling is about, for `touch_window` ticks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct Touch {
    pub(crate) tick: u64,
    /// The verb it tried (design v21 §5.6), or none for a crash, a touch it
    /// didn't choose (design v23 §5.6).
    pub(crate) verb: Option<Verb>,
    /// What it's learned about as (design v19 §5.6).
    pub(crate) subject: Subject,
    /// Which sprite, if it was one (design v18 §5.6).
    pub(crate) sprite: Option<EntityId>,
    /// How new its object type was to the sprite then (design v19 §5.6).
    pub(crate) novelty: f32,
    /// Whether it was a crash after a shove by a Cursor the sprite could
    /// see (design v29 §5.6).
    pub(crate) by_cursor: bool,
    /// The thing and its tile, if it stays put, so a need it eases makes it
    /// a remembered place (M2 design §7).
    #[serde(default)]
    pub(crate) place: Option<(Target, Pos)>,
}

/// What a sprite's moment at step 4 was about, which a sprite watching it
/// can take on (M2 design §18.2). Never a try at mating or courting (§18.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Topic {
    /// The thing it tried `verb` on at its latest step 6, or the try the
    /// Cursor rewarded or corrected this tick: learned about as `subject`,
    /// and the particular sprite, if it was one.
    Tried {
        subject: Subject,
        verb: Verb,
        sprite: Option<EntityId>,
    },
    /// The sprite that hit it, which it learned to fear.
    Hitter(EntityId),
    /// The Cursor, which it learned to fear.
    Cursor,
    /// The place where something it touched eased it (M2 design §7).
    Place {
        subject: Subject,
        target: Target,
        at: Pos,
    },
}

impl Topic {
    /// A try at `verb` on `subject`, the particular `sprite` if it's one, as
    /// a moment (M2 design §18.2): none for a try at mating or courting,
    /// which is never a moment (§18.3).
    pub(crate) fn tried(verb: Verb, subject: Subject, sprite: Option<EntityId>) -> Option<Topic> {
        (!verb.is_mating()).then_some(Topic::Tried {
            subject,
            verb,
            sprite,
        })
    }

    /// The try `touch`, as a moment: none for a crash, which has no verb.
    pub(crate) fn touched(touch: Touch) -> Option<Topic> {
        Topic::tried(touch.verb?, touch.subject, touch.sprite)
    }

    /// Whether it's about the sprite `id`: it hit `id`, or `id` hit it. A
    /// sprite learns nothing by watching about itself (M2 design §18.2).
    pub(crate) fn about(self, id: EntityId) -> bool {
        match self {
            Topic::Tried { sprite, .. } => sprite == Some(id),
            Topic::Hitter(hitter) => hitter == id,
            Topic::Cursor | Topic::Place { .. } => false,
        }
    }

    /// Whether a sprite whose eye was on `target`, learned about as
    /// `subject`, had its eye on what it's about: the same sprite, the
    /// Cursor, or the same object type (M2 design §18.1).
    pub(crate) fn caught(self, target: Option<Target>, subject: Option<Subject>) -> bool {
        match self {
            Topic::Tried {
                sprite: Some(sprite),
                ..
            }
            | Topic::Hitter(sprite) => target == Some(Target::Sprite(sprite)),
            Topic::Tried { subject: about, .. } | Topic::Place { subject: about, .. } => {
                subject == Some(about)
            }
            Topic::Cursor => target == Some(Target::Cursor),
        }
    }
}

/// What a try was at, as watching takes it on (M2 design §18.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TriedAt {
    /// A thing, learned about as its object type: its worth, good and bad
    /// pass on.
    Thing,
    /// The sprite, or for none the Cursor: only fear passes on, since whom
    /// to like a sprite finds out for itself.
    Feared(Option<EntityId>),
}

impl TriedAt {
    /// What a try at `subject`, the particular `sprite` if it was one, was
    /// at.
    pub(crate) fn of(subject: Subject, sprite: Option<EntityId>, data: &DataPack) -> TriedAt {
        match sprite {
            Some(_) => TriedAt::Feared(sprite),
            None if subject.category(data) == data.cursor_category() => TriedAt::Feared(None),
            None => TriedAt::Thing,
        }
    }
}

/// What a sprite knows of what its moment was about: the values a sprite
/// watching it takes on (M2 design §18.2). Of a thing, its worth for each
/// need but the breeding drive's (§18.3), good and bad; of a sprite or the
/// Cursor, its fear; and of either, the habit of the verb tried, its own
/// and taught together. The rest are 0, which watching moves nothing
/// towards.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Knowledge {
    pub(crate) topic: Topic,
    pub(crate) worth: Vec<f32>,
    pub(crate) good: f32,
    pub(crate) bad: f32,
    pub(crate) fear: f32,
    pub(crate) habit: f32,
}

impl Knowledge {
    /// Nothing known of `topic`, for `needs` needs.
    pub(crate) fn nothing(topic: Topic, needs: usize) -> Knowledge {
        Knowledge {
            topic,
            worth: vec![0.0; needs],
            good: 0.0,
            bad: 0.0,
            fear: 0.0,
            habit: 0.0,
        }
    }

    /// Every value, in the same order for any two sprites' knowledge of a
    /// topic.
    pub(crate) fn values(&self) -> impl Iterator<Item = f32> + '_ {
        let rest = [self.good, self.bad, self.fear, self.habit];
        self.worth.iter().copied().chain(rest)
    }
}

/// `mine` moved `share` of the way towards `theirs`, when theirs is further
/// from 0 the same way (M2 design §18.2): never past it, never back towards
/// 0, and never across it, so watching never undoes what a sprite found out
/// for itself. Worth, good and fear move so; badness and habits are kept
/// apart and settle either way (M2 design §19.7).
pub(crate) fn toward(mine: f32, theirs: f32, share: f32) -> f32 {
    let further = if theirs > 0.0 {
        mine >= 0.0 && theirs > mine
    } else {
        mine <= 0.0 && theirs < mine
    };
    if further {
        mine + share * (theirs - mine)
    } else {
        mine
    }
}

/// `mine` moved `share` of the way towards `theirs`, either way: what a
/// sprite learns by watching, kept apart from what it found out for itself,
/// settles on what it sees (M2 design §19.7).
pub(crate) fn settled(mine: f32, theirs: f32, share: f32) -> f32 {
    mine + share * (theirs - mine)
}

/// What step 4 reads for one sprite (design §5.6).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Signals {
    /// Each need's level now, in the pack's needs order.
    pub(crate) needs: Vec<f32>,
    /// Each need's relief this tick, in the same order.
    pub(crate) relief: Vec<f32>,
    /// The general good and bad channels: `reward` and `punishment`.
    pub(crate) reward: f32,
    pub(crate) punishment: f32,
    /// Whether a `fruitless` pulse is live: its latest try did nothing.
    pub(crate) fruitless: bool,
    /// Whether a `was_hit` pulse is live: its punishment teaches fear of
    /// the attacker and habits, not what anything is worth (design v18 §5.6).
    pub(crate) hit: bool,
    /// Who hit it, the `was_hit` pulse's source, if one is live.
    pub(crate) attacker: Option<EntityId>,
    /// How far back the Cursor's Reward looks, if it rewarded the sprite
    /// this tick (design v21 §5.6).
    pub(crate) reach_back: Option<u64>,
    /// Whether the Cursor corrected the sprite this tick: a `shocked` pulse
    /// is live (design v21 §5.6).
    pub(crate) corrected: bool,
    /// Whether the sprite could see the Cursor that touched it this tick
    /// (design v29 §5.6).
    pub(crate) seen_cursor: bool,
    /// How far the Cursor is, while the sprite knows it, can see it and its
    /// flood reaches it: its Chebyshev distance over the reach of the
    /// sprite's flood, as `target_distance` is on grass (design v29 §5.6).
    pub(crate) cursor_distance: Option<f32>,
}

/// The most entries a trace keeps (design §5.6).
pub(crate) const TRACE_CAP: usize = 512;

/// The least weight an entry keeps its place in the trace with (design §5.6).
const TRACE_FLOOR: f32 = 0.01;

/// The weight step 4 at tick `now` gives the trace entry from tick `then`
/// (design §5.6): λ^(now − then).
pub(crate) fn weight(trace_decay: f32, now: u64, then: u64) -> f32 {
    libm::powf(trace_decay, (now - then) as f32)
}

/// Whether the entry from tick `then` still counts at tick `now`'s step 4.
pub(crate) fn still_counts(trace_decay: f32, now: u64, then: u64) -> bool {
    weight(trace_decay, now, then) >= TRACE_FLOOR
}

/// One tick of a brain's trace (design §5.6): what it felt and chose at
/// that tick's step 5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TraceEntry {
    pub(crate) tick: u64,
    /// The verb it chose or kept doing, if any.
    pub(crate) verb: Option<Verb>,
    /// What the thing its decision was about, if any, is learned about as
    /// (design v19 §5.6): the one attention was on, or a running action's
    /// target it carried on with, its eye off it (M2 design §19.2).
    pub(crate) subject: Option<Subject>,
    /// The verb's motive (design §5.5): the need, by its place in the pack's
    /// needs, whose instinct did most to choose it.
    pub(crate) motive: Option<usize>,
}

/// Step 4 for every sprite not `dying` (design §2.4): reads each need's
/// relief, reward and punishment, resets the last two to 0, keeps what the
/// sprite felt, and learns from them, reporting each lesson. Each gets to
/// know the sprites step 3 found it sees, in its `company` (M2 design
/// §15.2).
pub(crate) fn run(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    company: &BTreeMap<EntityId, Company>,
    events: &mut Vec<Event>,
) {
    let indices = &data.physiology().indices;
    let courts = indices.courtship.is_some();
    let (forget_below, needs_count) = (data.physiology().forget_below, data.need_places().len());
    let tick = state.tick;
    let learning = state.learning;
    let window = data.physiology().touch_window;
    let drive = indices.drive;
    // A sprite that has died is forgotten (design v18 §5.6).
    let alive: BTreeSet<EntityId> = state
        .sprites
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !dying.contains(id))
        .collect();
    // How far each sprite that knows the Cursor sees it, by its flood's
    // reach, if its flood reaches it (design v29 §3.6, §5.6): only those can
    // get used to it.
    let cursor_distances: BTreeMap<EntityId, f32> = match state.cursor.seen_at() {
        Some(tile) => state
            .sprites
            .iter()
            .filter(|(_, sprite)| sprite.brain.experience.cursor.is_some())
            .filter_map(|(id, sprite)| {
                let flood = sprite.flood.as_ref()?;
                flood.nearest_goal(&state.map, tile, true)?;
                let reach = f32::from(flood.reach().max(1));
                Some((id, f32::from(chebyshev(tile, sprite.pos)) / reach))
            })
            .collect(),
        None => BTreeMap::new(),
    };
    // What each sprite's moment was about, for those watching it (M2
    // design §18.2).
    let mut moments: BTreeMap<EntityId, Vec<Topic>> = BTreeMap::new();
    let living = state
        .sprites
        .minds_mut()
        .filter(|(id, ..)| !dying.contains(id));
    for (id, body, brain) in living {
        let cursor_distance = cursor_distances.get(&id).copied();
        let (reward, punishment) = (body.chems[indices.reward], body.chems[indices.punishment]);
        body.chems[indices.reward] = 0.0;
        body.chems[indices.punishment] = 0.0;
        let reach_back = brain.reach_back.take();
        let seen_cursor = std::mem::take(&mut brain.seen_cursor);
        brain.experience.forget_dead(&alive);
        let needs = brain.need_levels(body, data);
        let mut relief = brain.relief(&needs, data);
        // The breeding drive eased teaches only about a partner: it's relief
        // only when the sprite has just mated, which only adults do, so no
        // genome can have a sprite learn that a young one, or anything else,
        // is good for it (M2 design §5).
        let mated = brain
            .touched
            .is_some_and(|t| t.verb == Some(Verb::Mate) && tick - t.tick <= window);
        if let Some(need) = drive.filter(|_| !mated) {
            relief[need] = 0.0;
        }
        brain.felt = relief.iter().sum::<f32>() + reward - punishment;
        if !learning {
            continue;
        }
        let signals = Signals {
            needs,
            relief,
            reward,
            punishment,
            fruitless: body.loci[indices.fruitless] > 0.0,
            hit: body.loci[indices.was_hit] > 0.0,
            attacker: body.sources.get(&indices.was_hit).copied(),
            reach_back,
            corrected: body.loci[indices.shocked] > 0.0,
            seen_cursor,
            cursor_distance,
        };
        let rate = body.loci[indices.learning_rate_mod];
        let (lessons, moment) = brain.learn_with_moment(tick, &signals, rate, data);
        for (learned, good) in lessons {
            events.push(Event {
                tick,
                kind: EventKind::LearnedMilestone {
                    id,
                    learned,
                    good,
                    watching: None,
                },
            });
        }
        if !moment.is_empty() {
            moments.insert(id, moment);
        }
        // Each sprite in sight grows more familiar after the tick's fading,
        // so one in sight is never forgotten (M2 design §15.2).
        if courts {
            let gain = brain.params.get(BrainParam::BefriendRate);
            let seen = company.get(&id).map_or(&[][..], |company| &company.seen);
            for &other in seen.iter().filter(|other| alive.contains(other)) {
                brain
                    .experience
                    .befriend(other, gain, needs_count, forget_below);
            }
        }
    }
    if !moments.is_empty() {
        watch(state, data, dying, company, &moments, events);
    }
}

/// The end of step 4, once every sprite has learned from what happened to
/// itself (M2 design §18.1): each sprite that learns by watching, not
/// `dying`, takes on what each sprite in its `company` that it watched shows
/// of what its moment was about, one of `moments`, and each lesson that tips
/// is reported with whom it watched. What each shows is read before anyone
/// takes anything on, so nothing passes through two sprites in a tick.
fn watch(
    state: &mut WorldState,
    data: &DataPack,
    dying: &[EntityId],
    company: &BTreeMap<EntityId, Company>,
    moments: &BTreeMap<EntityId, Vec<Topic>>,
    events: &mut Vec<Event>,
) {
    let tick = state.tick;
    let learning_rate_mod = data.physiology().indices.learning_rate_mod;
    let mut watched = Vec::new();
    for (id, sprite) in state.sprites.iter() {
        let brain = &sprite.brain;
        let rate = brain.params.get(BrainParam::WatchRate);
        let share = (rate * sprite.body.loci[learning_rate_mod]).clamp(0.0, 1.0);
        if share <= 0.0 || dying.contains(&id) {
            continue;
        }
        // Where its eye was at its latest step 5, the tick before.
        let Some(eye) = brain.snapshot.as_ref().filter(|eye| eye.tick + 1 == tick) else {
            continue;
        };
        let seen = company.get(&id).map_or(&[][..], |company| &company.seen);
        for &other in seen {
            let (Some(moment), Some(them)) = (moments.get(&other), state.sprites.get(other)) else {
                continue;
            };
            let on_them = eye.target == Some(Target::Sprite(other));
            let shown: Vec<Knowledge> = moment
                .iter()
                .filter(|topic| !topic.about(id))
                .filter(|topic| on_them || topic.caught(eye.target, eye.eye_subject()))
                .map(|&topic| them.brain.shown(topic, data))
                .collect();
            if !shown.is_empty() {
                watched.push((id, other, share, shown));
            }
        }
    }
    for (id, other, share, shown) in watched {
        let brain = &mut state
            .sprites
            .get_mut(id)
            .expect("a sprite that watched")
            .brain;
        for (learned, good) in brain.watched(&shown, share, data) {
            events.push(Event {
                tick,
                kind: EventKind::LearnedMilestone {
                    id,
                    learned,
                    good,
                    watching: Some(other),
                },
            });
        }
    }
}

/// The end of step 6 for every sprite: each brain that decided this tick
/// commits its trace entry.
pub(crate) fn commit(state: &mut WorldState, data: &DataPack) {
    let tick = state.tick;
    for (_, _, brain) in state.sprites.minds_mut() {
        brain.commit(tick, data);
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::at;

    use super::*;

    const WATER: Subject = Subject::ObjectType(100);
    const BUSH: Subject = Subject::ObjectType(1);

    /// Remembers water on `pos`, in a memory of 3 that takes places of a
    /// kind 5 tiles apart as one.
    fn water(experience: &mut Experience, pos: Pos) {
        experience.remember_place(
            WATER,
            Target::Water(pos),
            pos,
            RememberedPlaces {
                held: 3,
                per_kind: 3,
                merge: 5,
            },
        );
    }

    fn tiles(experience: &Experience) -> Vec<Pos> {
        experience.places.iter().map(|place| place.at).collect()
    }

    #[test]
    fn places_of_a_kind_within_the_merge_distance_are_one_moved_to_the_newest() {
        // M2 design §7: a lake is one place, not each of its tiles.
        let mut experience = Experience::default();
        water(&mut experience, at(0, 0));
        water(&mut experience, at(5, 3));
        assert_eq!(tiles(&experience), [at(5, 3)]);
        water(&mut experience, at(11, 3));
        assert_eq!(tiles(&experience), [at(5, 3), at(11, 3)], "6 apart: two");
        // A bush beside the water is a place of its own.
        let bush = Target::Object(EntityId(7));
        experience.remember_place(
            BUSH,
            bush,
            at(5, 4),
            RememberedPlaces {
                held: 3,
                per_kind: 3,
                merge: 5,
            },
        );
        assert_eq!(experience.places.len(), 3);
    }

    #[test]
    fn past_the_most_of_one_kind_a_new_place_pushes_out_the_faintest_of_its_kind() {
        // M2 design §7.2: a big lake's shores can't crowd out food, even
        // when the food is remembered more faintly.
        let limits = RememberedPlaces {
            held: 8,
            per_kind: 2,
            merge: 5,
        };
        let mut experience = Experience::default();
        let bush = Target::Object(EntityId(7));
        experience.remember_place(BUSH, bush, at(50, 0), limits);
        for x in [0, 10] {
            experience.remember_place(WATER, Target::Water(at(x, 0)), at(x, 0), limits);
        }
        experience.places[0].recall = 0.1;
        experience.places[1].recall = 0.5;
        experience.places[2].recall = 0.9;
        experience.remember_place(WATER, Target::Water(at(20, 0)), at(20, 0), limits);
        assert_eq!(tiles(&experience), [at(50, 0), at(10, 0), at(20, 0)]);
    }

    #[test]
    fn with_its_memory_full_a_new_kind_of_place_pushes_out_the_faintest_of_any_kind() {
        // M2 design §7.2: holding `held` places, and fewer than `per_kind`
        // of the new place's kind, the faintest of all goes, here water.
        let limits = RememberedPlaces {
            held: 3,
            per_kind: 3,
            merge: 5,
        };
        let mut experience = Experience::default();
        for x in [0, 10] {
            experience.remember_place(WATER, Target::Water(at(x, 0)), at(x, 0), limits);
        }
        let bush = Target::Object(EntityId(7));
        experience.remember_place(BUSH, bush, at(50, 0), limits);
        experience.places[0].recall = 0.9;
        experience.places[1].recall = 0.1;
        experience.places[2].recall = 0.5;
        let (third_kind, thing) = (Subject::ObjectType(2), Target::Object(EntityId(8)));
        experience.remember_place(third_kind, thing, at(30, 0), limits);
        assert_eq!(tiles(&experience), [at(0, 0), at(50, 0), at(30, 0)]);
    }

    #[test]
    fn a_place_eased_again_is_remembered_fully_again() {
        let mut experience = Experience::default();
        water(&mut experience, at(0, 0));
        experience.places[0].recall = 0.3;
        water(&mut experience, at(1, 0));
        assert_eq!(experience.places[0].recall, 1.0);
    }

    #[test]
    fn past_the_most_it_holds_a_new_place_pushes_out_the_faintest_the_oldest_of_them() {
        let mut experience = Experience::default();
        for x in [0, 10, 20] {
            water(&mut experience, at(x, 0));
        }
        experience.places[0].recall = 0.5;
        experience.places[1].recall = 0.2;
        experience.places[2].recall = 0.2;
        water(&mut experience, at(30, 0));
        assert_eq!(tiles(&experience), [at(0, 0), at(20, 0), at(30, 0)]);
    }

    #[test]
    fn a_place_is_forgotten_by_its_thing_and_recalled_by_its_kind_and_tile() {
        let mut experience = Experience::default();
        water(&mut experience, at(0, 0));
        water(&mut experience, at(10, 0));
        experience.places[1].recall = 0.4;
        assert_eq!(experience.recall(WATER, at(10, 0), 5), 0.4);
        // A trip to a tile of the lake keeps its pull once the place moves
        // within the lake (M2 design §7.3).
        assert_eq!(experience.recall(WATER, at(14, 2), 5), 0.4);
        assert_eq!(experience.recall(WATER, at(16, 0), 5), 0.0);
        assert_eq!(experience.recall(BUSH, at(10, 0), 5), 0.0);
        experience.forget_place(Target::Water(at(0, 0)));
        assert_eq!(tiles(&experience), [at(10, 0)]);
    }

    #[test]
    fn a_place_merges_with_every_place_of_its_kind_within_the_merge_distance() {
        // M2 design §7.2: between two places of a lake 8 apart, the new one
        // takes in both, so no two of a kind are within 5.
        let mut experience = Experience::default();
        water(&mut experience, at(0, 0));
        water(&mut experience, at(8, 0));
        water(&mut experience, at(4, 0));
        assert_eq!(tiles(&experience), [at(4, 0)]);
    }

    #[test]
    fn places_remembered_under_other_limits_are_brought_within_these() {
        // M2 design §7.2: a save from before `per_kind` may hold 8 places of
        // water, some near each other.
        let loose = RememberedPlaces {
            held: 8,
            per_kind: 8,
            merge: 0,
        };
        let mut experience = Experience::default();
        for (x, recall) in [(0, 0.9), (2, 0.8), (10, 0.2), (20, 0.7), (30, 0.6)] {
            experience.remember_place(WATER, Target::Water(at(x, 0)), at(x, 0), loose);
            experience
                .places
                .last_mut()
                .expect("just remembered")
                .recall = recall;
        }
        experience.remember_place(BUSH, Target::Object(EntityId(7)), at(40, 0), loose);
        experience.keep_within(RememberedPlaces {
            held: 2,
            per_kind: 2,
            merge: 5,
        });
        // The older of the two near the start goes, then the faintest water
        // past 2 of it, then the faintest of all past 2.
        assert_eq!(tiles(&experience), [at(2, 0), at(40, 0)]);
    }

    #[test]
    fn watching_moves_a_value_only_towards_a_view_further_from_0() {
        // M2 design §18.2: never past the other's view, never back towards
        // 0, never across it.
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(close(toward(0.2, 0.6, 0.5), 0.4));
        assert!(close(toward(0.0, -0.4, 0.5), -0.2));
        assert!(close(toward(0.2, 0.6, 1.0), 0.6), "never past it");
        assert_eq!(toward(0.6, 0.2, 0.5), 0.6, "never back towards 0");
        assert_eq!(toward(-0.2, -0.1, 0.5), -0.2, "never back towards 0");
        assert_eq!(toward(-0.2, 0.6, 0.5), -0.2, "never across it");
        assert_eq!(toward(0.3, -0.6, 0.5), 0.3, "never across it");
        assert_eq!(toward(0.0, 0.0, 0.5), 0.0);
    }

    #[test]
    fn a_moment_is_about_a_sprite_when_it_hit_that_one_or_was_hit_by_it() {
        // M2 design §18.2: a sprite learns nothing by watching about itself.
        let me = EntityId(4);
        let hit_it = Topic::Tried {
            subject: Subject::ObjectType(101),
            verb: Verb::Hit,
            sprite: Some(me),
        };
        assert!(hit_it.about(me));
        assert!(Topic::Hitter(me).about(me));
        assert!(!Topic::Hitter(EntityId(5)).about(me));
        let drank = Topic::Tried {
            subject: WATER,
            verb: Verb::Drink,
            sprite: None,
        };
        let place = Topic::Place {
            subject: WATER,
            target: Target::Water(at(0, 0)),
            at: at(0, 0),
        };
        for topic in [drank, Topic::Cursor, place] {
            assert!(!topic.about(me), "{topic:?}");
        }
    }

    #[test]
    fn a_sprite_watching_catches_a_moment_about_what_its_eye_is_on() {
        // M2 design §18.1: the same sprite, the Cursor, or the same object
        // type, not just the same one of it.
        let drank = Topic::Tried {
            subject: WATER,
            verb: Verb::Drink,
            sprite: None,
        };
        let pool = Some(Target::Water(at(9, 9)));
        assert!(drank.caught(pool, Some(WATER)), "other water");
        assert!(!drank.caught(Some(Target::Object(EntityId(7))), Some(BUSH)));
        let bully = Topic::Hitter(EntityId(5));
        assert!(bully.caught(Some(Target::Sprite(EntityId(5))), None));
        assert!(!bully.caught(Some(Target::Sprite(EntityId(6))), None));
        assert!(Topic::Cursor.caught(Some(Target::Cursor), None));
        assert!(!Topic::Cursor.caught(None, None));
    }
}
