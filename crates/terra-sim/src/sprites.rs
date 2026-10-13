//! The world's sprites and where they stand (design §2.3, §3.4, §4).

use std::collections::{BTreeMap, VecDeque};

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::action::{Action, Did, ScriptedAction, Walk, chebyshev};
use crate::ageing::{LifeStage, Switch, frailty};
use crate::biochem::{Body, Program};
use crate::brain::Brain;
use crate::breeding::Sex;
use crate::data::DataPack;
use crate::genome::{Gene, Genome};
use crate::inheritance::{Parent, flaws_of};
use crate::map::{Dir, Map, Pos};
use crate::mutation::{Copied, CopyingError};
use crate::objects::{EntityId, Objects};
use crate::occupancy::Occupancy;
use crate::perception::{Flood, Target};
use crate::physiology::RememberedPlaces;
use crate::random::{chance, uniform};
use crate::registry::{Trait, Verb};
use crate::sliding::Slide;

/// One sprite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Sprite {
    pub(crate) pos: Pos,
    /// The tick it was born on: before the world began for a sprite that
    /// starts it grown (M2 design §9.1). For an egg, the tick it was laid
    /// on, until it hatches, which is its birth (M2 design §10).
    pub(crate) born: i64,
    /// The tick it hatches on, while it's an egg (M2 design §10).
    #[serde(default)]
    pub(crate) hatches: Option<u64>,
    /// Its parents (M2 design §10): the one that laid it, then its partner,
    /// if it had one. None for a sprite the world began with or the Cursor
    /// placed.
    #[serde(default)]
    pub(crate) parents: Vec<EntityId>,
    /// The tick it last had young on, if it has (M2 design §10).
    #[serde(default)]
    pub(crate) last_bred: Option<u64>,
    /// Its sex: male until it turns female (M2 design §10).
    #[serde(default)]
    pub(crate) sex: Sex,
    /// The name the player gave it, if any (design v28 §6.5).
    pub(crate) name: Option<String>,
    pub(crate) genome: Genome,
    /// The genome compiled for the chemistry step. It's derived from the
    /// genome, so it isn't hashed.
    #[serde(skip)]
    pub(crate) program: Program,
    /// The life stage its genes are expressed for (M2 design §9.1): the
    /// one it grew to at the latest step 3, or was made at. `None` only in
    /// a save from before life stages, until it's loaded.
    #[serde(default)]
    stage: Option<LifeStage>,
    /// Each gene's health, in genome order, from 1 (whole) to 0 (M2 design
    /// §9.4): it only ever falls, and scales the gene's effect. Empty only
    /// in a save from before ageing, until it's loaded.
    #[serde(default)]
    pub(crate) health: Vec<f32>,
    /// How many of its two copies of each gene are flawed, from 0 to 2, in
    /// genome order (M2 design §12): with both, the gene is broken. Empty
    /// when none are, as in a sprite from before flaws.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) flaws: Vec<u8>,
    /// The genes copying changed as it was laid (M2 design §12), with what
    /// each was as it inherited it. Empty for a sprite with no parents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) copying_errors: Vec<CopyingError>,
    pub(crate) body: Body,
    pub(crate) brain: Brain,
    /// What it's doing, or the action that last ended until the next starts.
    pub(crate) action: Option<Action>,
    /// Its way after the Cursor, while the Cursor leads it (design v23 §6.5).
    pub(crate) lead: Option<Walk>,
    /// Its slide, while a shove sends it (design v25 §3.5.4).
    pub(crate) slide: Option<Slide>,
    /// The actions a hand-made world starts it on, in order, until they start.
    pub(crate) scripted: VecDeque<ScriptedAction>,
    /// Its cached perception flood (design §3.6).
    pub(crate) flood: Option<Flood>,
    /// Move points banked towards its next step, in tenths (design §3.7).
    pub(crate) move_points: u32,
    /// What it did at the last step 6.
    pub(crate) did: Did,
    /// The direction of its latest step, if it has taken one: the way it
    /// pushes an item on its own tile (design §3.5.2).
    pub(crate) last_step: Option<Dir>,
}

impl Sprite {
    /// The sprite's age on the tick `tick`.
    pub(crate) fn age(&self, tick: u64) -> u64 {
        u64::try_from(tick as i64 - self.born).expect("a sprite is born by now")
    }

    /// Its life stage (M2 design §9.1): an egg's genes are expressed as a
    /// young sprite's.
    pub(crate) fn stage(&self) -> LifeStage {
        self.stage.expect("a loaded sprite has a life stage")
    }

    /// Whether it's still an egg (M2 design §10).
    pub(crate) fn is_egg(&self) -> bool {
        self.hatches.is_some()
    }

    /// How fast it walks on the tick `tick`, in move points (M2 design
    /// §9.5): its speed trait, less while it's young, rising steadily to all
    /// of it as it grows up. A led egg goes at a newborn's pace, its age
    /// counting from hatching (§10.4).
    pub(crate) fn speed(&self, tick: u64, data: &DataPack) -> f32 {
        let ageing = data.physiology().ageing;
        let traits = self.program.traits;
        let growing = if self.stage() <= LifeStage::Young {
            let age = if self.is_egg() { 0 } else { self.age(tick) };
            let grown = (age as f32 / traits.grows_up).min(1.0);
            ageing.young_speed + (1.0 - ageing.young_speed) * grown
        } else {
            1.0
        };
        traits.speed * growing
    }

    /// Its genes as an egg inherits them (M2 design §12).
    pub(crate) fn as_parent(&self) -> Parent<'_> {
        Parent {
            genome: &self.genome,
            flaws: &self.flaws,
        }
    }

    /// How many of its genes are broken (M2 design §12): both copies flawed.
    pub(crate) fn broken(&self) -> usize {
        self.flaws.iter().filter(|&&flawed| flawed >= 2).count()
    }

    /// The wear that brings its old age sooner and speeds the decay of its
    /// genes (M2 design §9.4): what a hard life left on its body, and
    /// `flaws.wear` for each broken gene (§12), at most 1.
    pub(crate) fn worn(&self, data: &DataPack) -> f32 {
        let broken = self.broken() as f32 * data.physiology().flaws.wear;
        (self.body.wear + broken).min(1.0)
    }

    /// The share of the age its genes say that a hard life and broken genes
    /// have brought its old age sooner by (M2 design §9.4, §12).
    pub(crate) fn sooner(&self, data: &DataPack) -> f32 {
        data.physiology().ageing.sooner * self.worn(data)
    }

    /// Expresses its genes afresh, for its life stage and as weakened as
    /// they are (M2 design §9.2, §9.4), leaving what it has learned as it
    /// was.
    fn express(&mut self, data: &DataPack) {
        self.express_body(data);
        self.brain
            .reach(&self.genome, data, self.stage(), &self.health);
    }

    /// The same for its body's genes only, for when only they changed.
    fn express_body(&mut self, data: &DataPack) {
        self.program = Program::weakened(&self.genome, data, self.stage(), &self.health);
    }

    /// Ages its genes for a tick (M2 design §9.4): while its injury is bad,
    /// a random gene may be scarred, at any age; and once it's old, damage
    /// its repair no longer keeps up with lands on random genes, faster the
    /// more a hard life has worn it, so that an unworn sprite's genes fall
    /// to where its body gives out in `decay.ticks`. Returns whether any
    /// gene weakened.
    pub(crate) fn age_genes(&mut self, data: &DataPack, rng: &mut ChaCha8Rng) -> bool {
        let physiology = data.physiology();
        let ageing = physiology.ageing;
        let (scar, decay) = (ageing.scar, ageing.decay);
        let injured = self.body.chems[physiology.indices.injury] > scar.above;
        let old = self.stage() == LifeStage::Old;
        // Most ticks nothing can happen, so skip counting the genes.
        if !injured && !old {
            return false;
        }
        let (n, frailty_sum) = self
            .genome
            .genes
            .iter()
            .map(frailty)
            .filter(|&f| f > 0.0)
            .fold((0_usize, 0.0_f32), |(n, sum), f| (n + 1, sum + f));
        if n == 0 {
            return false;
        }
        let scarred = (injured && chance(rng, scar.chance)).then_some(scar.hit);
        let decayed = if old {
            // A hit takes `hit` times the frailty of the gene it lands
            // on, so `hit` × the average frailty on average, which
            // brings the average health of `n` genes down by that over
            // `n`. Enough hits land, on average, to bring it to
            // `gives_out` in `ticks`: when that's more than one a tick,
            // the whole hits land, and the odds are of one more.
            let n = n as f32;
            let needed = (1.0 - decay.gives_out) * n * n / (decay.hit * frailty_sum);
            let pace = 1.0 + (ageing.wear_speedup - 1.0) * self.worn(data);
            let per_tick = pace * needed / decay.ticks as f32;
            let whole = libm::floorf(per_tick);
            whole as usize + usize::from(chance(rng, per_tick - whole))
        } else {
            0
        };
        // Hits enough to land on every gene, which only a data pack with
        // a tiny `hit` makes, land a round at a time, one on each, and the
        // rest at random: the same damage on average, in work that doesn't
        // grow with the hits.
        let (rounds, decayed) = (decayed / n, decayed % n);
        let mut weakened = false;
        let mut mind = false;
        if rounds > 0 {
            let hit = rounds as f32 * decay.hit;
            for (health, gene) in self.health.iter_mut().zip(&self.genome.genes) {
                if frailty(gene) > 0.0 {
                    *health = (*health - hit * frailty(gene)).max(0.0);
                    mind |= matches!(gene, Gene::Instinct { .. } | Gene::AttentionInstinct { .. });
                }
            }
            weakened = true;
        }
        let decayed = std::iter::repeat_n(decay.hit, decayed);
        for hit in scarred.into_iter().chain(decayed) {
            let nth = uniform(rng, n as u64) as usize;
            let (index, gene) = self
                .genome
                .genes
                .iter()
                .enumerate()
                .filter(|(_, gene)| frailty(gene) > 0.0)
                .nth(nth)
                .expect("the nth of n frail genes");
            let health = &mut self.health[index];
            *health = (*health - hit * frailty(gene)).max(0.0);
            weakened = true;
            mind |= matches!(gene, Gene::Instinct { .. } | Gene::AttentionInstinct { .. });
        }
        if mind {
            self.express(data);
        } else if weakened {
            // The brain reads only its instincts' health.
            self.express_body(data);
        }
        weakened
    }

    /// Sets its `grows_old` gene to `age`, so it grows old then, whatever
    /// its genes said (M2 design §9.3): for a lab scenario that watches it
    /// for longer than a life. Its genes are expressed afresh for its life
    /// stage on the tick `tick`. A genome with no such gene gets one.
    pub(crate) fn set_grows_old(&mut self, age: f32, tick: u64, data: &DataPack) {
        let gene = self.genome.genes.iter_mut().find_map(|gene| match gene {
            Gene::Trait {
                which: Trait::GrowsOld,
                value,
            } => Some(value),
            _ => None,
        });
        match gene {
            Some(value) => *value = age,
            None => {
                self.genome.genes.push(Gene::Trait {
                    which: Trait::GrowsOld,
                    value: age,
                });
                // An empty list of switches means none switch.
                if !self.genome.switches.is_empty() {
                    self.genome.switches.push(Switch::default());
                }
                self.health.push(1.0);
            }
        }
        self.express_body(data);
        self.stage = Some(
            self.program
                .traits
                .life_stage(self.age(tick), self.sooner(data)),
        );
        self.express(data);
    }

    /// Turns it female, if it's a male adult whose `feminiser` hormone is
    /// past the level physiology sets (M2 design §10), and says whether it
    /// did. A young sprite never turns.
    pub(crate) fn turn(&mut self, data: &DataPack) -> bool {
        let physiology = data.physiology();
        let Some(indices) = physiology.indices.breeding else {
            return false;
        };
        let level = self.body.chems[indices.feminiser];
        let turns = self.sex == Sex::Male
            && self.stage().is_grown()
            && level >= physiology.breeding.turns_female;
        if turns {
            self.sex = Sex::Female;
        }
        turns
    }

    /// Grows the sprite to its life stage on the tick `tick` (M2 design
    /// §9.2): if that's a new one, its genes are expressed afresh for it,
    /// leaving what it has learned as it was. An egg hatches on its tick,
    /// and is born then, owing nothing for a walk it was led on (§10, §10.4).
    /// Returns the stage it was in and the new one, if it has one.
    pub(crate) fn grow(&mut self, tick: u64, data: &DataPack) -> Option<(LifeStage, LifeStage)> {
        let was = self.stage();
        let stage = match self.hatches {
            Some(hatches) if tick < hatches => return None,
            Some(_) => {
                self.hatches = None;
                self.born = tick as i64;
                // The steps and points of a walk the Cursor led it on are
                // the egg's: the hatchling pays nothing for them (§10.4).
                self.did = Did::default();
                self.move_points = 0;
                self.program.traits.life_stage(0, self.sooner(data))
            }
            None => self
                .program
                .traits
                .life_stage(self.age(tick), self.sooner(data)),
        };
        if stage == was {
            return None;
        }
        self.stage = Some(stage);
        self.express(data);
        Some((was, stage))
    }

    /// A newborn with `genome` on `pos`, born on the tick `born` (design §4.7).
    pub(crate) fn newborn(genome: Genome, pos: Pos, born: u64, data: &DataPack) -> Sprite {
        Sprite::aged(genome, pos, born, 0, data)
    }

    /// An egg with `genes` on `pos`, laid on the tick `laid` by `parents`
    /// (M2 design §10): it hatches `breeding.hatches` ticks later, with a
    /// newborn's body (design §4.7). A broken gene is born with
    /// `flaws.health` (§12).
    pub(crate) fn egg(
        genes: Copied,
        pos: Pos,
        laid: u64,
        parents: Vec<EntityId>,
        data: &DataPack,
    ) -> Sprite {
        let Copied {
            genome,
            flaws,
            errors,
        } = genes;
        let mut egg = Sprite::newborn(genome, pos, laid, data);
        egg.stage = Some(LifeStage::Egg);
        egg.hatches = Some(laid.saturating_add(data.physiology().breeding.hatches));
        egg.parents = parents;
        egg.copying_errors = errors;
        egg.flaws = flaws;
        if egg.broken() > 0 {
            let born_with = data.physiology().flaws.health;
            for (index, health) in egg.health.iter_mut().enumerate() {
                if flaws_of(&egg.flaws, index) >= 2 {
                    *health = health.min(born_with);
                }
            }
            egg.express(data);
        }
        egg
    }

    /// A sprite with `genome` on `pos` that has just grown up on the tick
    /// `tick` (M2 design §9.1), with a newborn's body (design §4.7).
    pub(crate) fn grown_up(genome: Genome, pos: Pos, tick: u64, data: &DataPack) -> Sprite {
        let (grows_up, _) = Sprite::adulthood(&genome, data);
        Sprite::aged(genome, pos, tick, grows_up, data)
    }

    /// The ages, in whole ticks, at which a sprite with `genome` and an
    /// easy life grows up and grows old (M2 design §9.3).
    pub(crate) fn adulthood(genome: &Genome, data: &DataPack) -> (u64, u64) {
        let traits = Program::new(genome, data, LifeStage::Young).traits;
        let age = |trait_value: f32| libm::ceilf(trait_value) as u64;
        (age(traits.grows_up), age(traits.grows_old))
    }

    /// A sprite with `genome` on `pos` that is `age` ticks old on the tick
    /// `tick`, its genes expressed for its life stage then (M2 design §9.1).
    /// Its body is a newborn's (design §4.7), and it has learned nothing.
    pub(crate) fn aged(genome: Genome, pos: Pos, tick: u64, age: u64, data: &DataPack) -> Sprite {
        // Its traits, which set its stage, don't depend on it.
        let traits = Program::new(&genome, data, LifeStage::Young).traits;
        let stage = traits.life_stage(age, 0.0);
        let program = Program::new(&genome, data, stage);
        let body = Body::newborn(&program, data);
        let brain = Brain::new(&genome, data, stage);
        let genome_len = genome.genes.len();
        Sprite {
            pos,
            born: tick as i64 - age as i64,
            hatches: None,
            parents: Vec::new(),
            last_bred: None,
            sex: Sex::Male,
            name: None,
            genome,
            program,
            stage: Some(stage),
            health: vec![1.0; genome_len],
            flaws: Vec::new(),
            copying_errors: Vec::new(),
            body,
            brain,
            action: None,
            lead: None,
            slide: None,
            scripted: VecDeque::new(),
            flood: None,
            move_points: 0,
            did: Did::default(),
            last_step: None,
        }
    }
}

/// Every sprite in the world, and the tile each stands on. A tile holds at
/// most one sprite (design §3.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Sprites {
    /// In ascending ID order, the order every per-sprite pass takes (design §2.3).
    by_id: BTreeMap<EntityId, Sprite>,
    /// The sprite on each tile. It's derived from `by_id`, so it isn't hashed.
    #[serde(skip)]
    on_tile: Occupancy,
}

impl Sprites {
    /// No sprites, on `map`.
    pub(crate) fn new(map: &Map) -> Sprites {
        Sprites {
            by_id: BTreeMap::new(),
            on_tile: Occupancy::new(map),
        }
    }

    /// How many sprites there are, eggs included.
    pub(crate) fn len(&self) -> usize {
        self.by_id.len()
    }

    /// The sprite on the tile at `pos`, if any.
    pub(crate) fn at(&self, pos: Pos) -> Option<EntityId> {
        self.on_tile.at(pos)
    }

    /// Every sprite, in ascending ID order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (EntityId, &Sprite)> {
        self.by_id.iter().map(|(&id, sprite)| (id, sprite))
    }

    /// Every sprite, in ascending ID order, to change. Where each stands
    /// isn't theirs to change: `move_to` and `swap` keep the tile index.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Sprite)> {
        self.by_id.iter_mut().map(|(&id, sprite)| (id, sprite))
    }

    /// The sprite `id`, if it exists.
    pub(crate) fn get(&self, id: EntityId) -> Option<&Sprite> {
        self.by_id.get(&id)
    }

    /// Every sprite's body, in ascending ID order, to change.
    pub(crate) fn bodies_mut(&mut self) -> impl Iterator<Item = &mut Body> {
        self.by_id.values_mut().map(|sprite| &mut sprite.body)
    }

    /// Every sprite's ID, body and brain, in ascending ID order, to change.
    pub(crate) fn minds_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut Body, &mut Brain)> {
        self.by_id
            .iter_mut()
            .map(|(&id, sprite)| (id, &mut sprite.body, &mut sprite.brain))
    }

    /// The sprite `id`, if it exists, to change.
    pub(crate) fn get_mut(&mut self, id: EntityId) -> Option<&mut Sprite> {
        self.by_id.get_mut(&id)
    }

    /// Takes the sprite `id`, which must exist, out of the world.
    pub(crate) fn remove(&mut self, id: EntityId) -> Sprite {
        let sprite = self.by_id.remove(&id).expect("the sprite to remove");
        self.on_tile.clear(sprite.pos);
        sprite
    }

    /// Moves the sprite `id`, which must exist, onto the tile at `to`. The
    /// caller has checked the tile is free.
    pub(crate) fn move_to(&mut self, id: EntityId, to: Pos) {
        let sprite = self.by_id.get_mut(&id).expect("the sprite to move");
        self.on_tile.clear(sprite.pos);
        self.on_tile.put(to, id);
        sprite.last_step = Dir::towards(sprite.pos, to).or(sprite.last_step);
        sprite.pos = to;
    }

    /// Swaps the tiles of sprites `a` and `b`, which must exist.
    pub(crate) fn swap(&mut self, a: EntityId, b: EntityId) {
        let pa = self.by_id[&a].pos;
        let pb = self.by_id[&b].pos;
        self.on_tile.clear(pa);
        self.on_tile.clear(pb);
        self.on_tile.put(pa, b);
        self.on_tile.put(pb, a);
        for (id, from, to) in [(a, pa, pb), (b, pb, pa)] {
            let sprite = self.by_id.get_mut(&id).expect("a swapping sprite");
            sprite.last_step = Dir::towards(from, to).or(sprite.last_step);
            sprite.pos = to;
        }
    }

    /// Puts a new sprite on its tile. The caller has checked the tile is free.
    pub(crate) fn place(&mut self, id: EntityId, sprite: Sprite) {
        self.on_tile.put(sprite.pos, id);
        self.by_id.insert(id, sprite);
    }

    /// Rebuilds what a save leaves out (design §2.8): which sprite stands on
    /// each tile of `map`, each one's genome compiled for the chemistry
    /// step, and its levels before the tick, taken as its levels now; its
    /// remembered places are brought within the pack's limits. Says
    /// what's wrong if one is off the map or shares a tile, was born after
    /// the tick `now`, its genome, body or brain doesn't fit `data`, or its
    /// flood doesn't fit `map`.
    pub(crate) fn rebuild(&mut self, map: &Map, data: &DataPack, now: u64) -> Result<(), String> {
        self.on_tile = Occupancy::new(map);
        for (&id, sprite) in &mut self.by_id {
            if !map.contains(sprite.pos) || self.on_tile.at(sprite.pos).is_some() {
                return Err(format!("sprite {} has no tile of its own", id.0));
            }
            if sprite.born > now as i64 {
                return Err(format!("sprite {} was born after the save", id.0));
            }
            // The genome first: the brain is checked against it, once it
            // has any brain parameter added since it was saved.
            let misfit = || format!("sprite {} doesn't fit its data pack", id.0);
            if !sprite.genome.fits(data) {
                return Err(misfit());
            }
            // A save from before ageing has whole genes (M2 design §9.4).
            if sprite.health.is_empty() {
                sprite.health = vec![1.0; sprite.genome.genes.len()];
            }
            if sprite.health.len() != sprite.genome.genes.len() {
                return Err(misfit());
            }
            // A save from before life stages has none: its stage is the one
            // its age gives, from its traits, which don't depend on it.
            let stage = match sprite.stage {
                Some(stage) => stage,
                None => Program::new(&sprite.genome, data, LifeStage::Young)
                    .traits
                    .life_stage(sprite.age(now), sprite.sooner(data)),
            };
            sprite.stage = Some(stage);
            sprite.brain.params.catch_up(&sprite.genome, data, stage);
            sprite.brain.widen_verbs();
            // Places remembered under other limits, in a save from before
            // they were added, are brought within its pack's (M2 design §7.2).
            sprite
                .brain
                .experience
                .keep_within(data.physiology().remembered_places);
            let fits = |sprite: &Sprite| {
                sprite.body.fits(data)
                    && sprite
                        .brain
                        .fits(&sprite.genome, data, stage, &sprite.health, now)
            };
            if !fits(sprite) {
                return Err(misfit());
            }
            if sprite.flood.as_ref().is_some_and(|flood| !flood.fits(map)) {
                return Err(format!("sprite {}'s flood doesn't fit the map", id.0));
            }
            self.on_tile.put(sprite.pos, id);
            sprite.program = Program::weakened(&sprite.genome, data, stage, &sprite.health);
            sprite.body.chems_before_tick.clone_from(&sprite.body.chems);
        }
        Ok(())
    }

    /// Checks the sprites' invariants (design §7.1), or says which is broken:
    /// the tile index matches where the sprites are, so no two share a tile;
    /// none stands on a solid object; every chemical is within 0 to 1; a
    /// Mate action is only ever an adult's, aimed at an adult (M2 design
    /// §5); and each remembers no more places than it may, all on the map
    /// (M2 design §7).
    pub(crate) fn check(
        &self,
        map: &Map,
        objects: &Objects,
        data: &DataPack,
    ) -> Result<(), String> {
        let indexed = self.on_tile.count();
        if indexed != self.by_id.len() {
            return Err(format!(
                "the tile index holds {indexed} sprites, but there are {}",
                self.by_id.len()
            ));
        }
        for (&id, sprite) in &self.by_id {
            let pos = sprite.pos;
            if !map.contains(pos) || self.at(pos) != Some(id) {
                return Err(format!("the tile index doesn't have {id:?} on {pos:?}"));
            }
            if objects.is_solid_at(data, pos) {
                return Err(format!("{id:?} stands on a solid object at {pos:?}"));
            }
            if sprite.stage.is_none() {
                return Err(format!("{id:?} has no life stage"));
            }
            // An egg is laid, and hatches on its tick (M2 design §10).
            if sprite.is_egg() != (sprite.stage() == LifeStage::Egg) {
                return Err(format!(
                    "{id:?} is at the life stage {:?}, but hatches at {:?}",
                    sprite.stage(),
                    sprite.hatches
                ));
            }
            // It lies still until it hatches, unless the Cursor leads it
            // (M2 design §10.4).
            if sprite.is_egg()
                && (sprite.action.is_some()
                    || sprite.slide.is_some()
                    || !sprite.scripted.is_empty())
            {
                return Err(format!("{id:?} is an egg, but has something to do"));
            }
            if sprite.parents.len() > 2 || sprite.parents.contains(&id) {
                return Err(format!(
                    "{id:?} has the parents {:?}: at most two, never itself",
                    sprite.parents
                ));
            }
            let wear = sprite.body.wear;
            if !(0.0..=1.0).contains(&wear) {
                return Err(format!("{id:?} has a wear of {wear}, outside 0 to 1"));
            }
            if sprite.health.len() != sprite.genome.genes.len() {
                return Err(format!(
                    "{id:?} has {} gene healths, but {} genes",
                    sprite.health.len(),
                    sprite.genome.genes.len()
                ));
            }
            if let Some(health) = sprite.health.iter().find(|h| !(0.0..=1.0).contains(*h)) {
                return Err(format!(
                    "{id:?} has a gene of health {health}, outside 0 to 1"
                ));
            }
            // Two copies of each gene, either flawed (M2 design §12.3).
            if sprite.flaws.len() > sprite.genome.genes.len()
                || sprite.flaws.iter().any(|&flawed| flawed > 2)
            {
                return Err(format!(
                    "{id:?} has the flaws {:?}: at most two a gene, of {} genes",
                    sprite.flaws,
                    sprite.genome.genes.len()
                ));
            }
            if let Some(error) = sprite
                .copying_errors
                .iter()
                .find(|error| error.gene >= sprite.genome.genes.len())
            {
                return Err(format!(
                    "{id:?} has a copying error in gene {}, of {} genes",
                    error.gene,
                    sprite.genome.genes.len()
                ));
            }
            if sprite.body.chems_before_tick.len() != sprite.body.chems.len() {
                return Err(format!(
                    "{id:?} has {} levels from a tick ago, but {} chemicals",
                    sprite.body.chems_before_tick.len(),
                    sprite.body.chems.len()
                ));
            }
            if let Some(level) = sprite.body.chems.iter().find(|l| !(0.0..=1.0).contains(*l)) {
                return Err(format!("{id:?} has a chemical at {level}, outside 0 to 1"));
            }
            if let Some(value) = sprite.body.loci.iter().find(|v| !v.is_finite()) {
                return Err(format!(
                    "{id:?} has a locus at {value}, which isn't a number"
                ));
            }
            // Only an adult is female, and only an adult breeds alone (M2
            // design §10).
            if sprite.sex == Sex::Female && !sprite.stage().is_grown() {
                return Err(format!("{id:?} is female, but not grown up"));
            }
            if sprite
                .action
                .as_ref()
                .is_some_and(|a| a.verb == Verb::BreedAlone)
                && !sprite.stage().is_grown()
            {
                return Err(format!(
                    "{id:?} has a BreedAlone action, but isn't grown up"
                ));
            }
            // Mating only ever between adults, whatever its genes or the
            // data pack say (M2 design §5).
            if let Some(action) = sprite.action.as_ref().filter(|a| a.verb == Verb::Mate) {
                let partner = action.target.and_then(Target::sprite);
                let grown = |other: EntityId| self.get(other).is_none_or(|o| o.stage().is_grown());
                if !sprite.stage().is_grown() || partner.is_none_or(|other| !grown(other)) {
                    return Err(format!(
                        "{id:?} has a Mate action aimed at {:?}, but mating is only between adults",
                        action.target
                    ));
                }
            }
            sprite
                .brain
                .check()
                .map_err(|broken| format!("{id:?} {broken}"))?;
            // Its remembered places are few enough, no two of a kind within
            // the merge distance, and on the map (M2 design §7.2).
            let places = &sprite.brain.experience.places;
            let RememberedPlaces {
                held,
                per_kind,
                merge,
            } = data.physiology().remembered_places;
            if places.len() > usize::from(held) {
                return Err(format!(
                    "{id:?} remembers {} places, over {held}",
                    places.len()
                ));
            }
            for (i, place) in places.iter().enumerate() {
                let of_kind = places.iter().filter(|p| p.subject == place.subject).count();
                if of_kind > usize::from(per_kind) {
                    return Err(format!(
                        "{id:?} remembers {of_kind} places of one kind, over {per_kind}"
                    ));
                }
                let near = places[i + 1..]
                    .iter()
                    .find(|p| p.subject == place.subject && chebyshev(p.at, place.at) <= merge);
                if let Some(near) = near {
                    return Err(format!(
                        "{id:?} remembers two places of one kind within {merge} tiles, at {:?} and {:?}",
                        place.at, near.at
                    ));
                }
            }
            if let Some(place) = places.iter().find(|place| !map.contains(place.at)) {
                return Err(format!(
                    "{id:?} remembers a place off the map, at {:?}",
                    place.at
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::test_util::{at, builtin_changing};

    /// A sprite of speed 9 and sense radius 10 that grows up at 3,000 ticks
    /// and old at 6,000, `age` ticks old on tick 10,000.
    fn sprite(age: u64, data: &DataPack) -> Sprite {
        let genome = Genome::from_ron(
            r#"(format: 1, genes: [
                Trait(trait: "speed", value: 9.0),
                Trait(trait: "sense_radius", value: 10.0),
                Trait(trait: "grows_up", value: 3000.0),
                Trait(trait: "grows_old", value: 6000.0),
            ])"#,
            data,
        )
        .expect("a valid genome");
        Sprite::aged(genome, at(0, 0), 10_000, age, data)
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "{actual} should be {expected}"
        );
    }

    #[test]
    fn the_young_walk_slower_until_they_grow_up() {
        // M2 design §9.5: from 0.67 of its speed at birth, rising steadily.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        assert_close(sprite(0, &data).speed(10_000, &data), 9.0 * 0.67);
        assert_close(
            sprite(1_500, &data).speed(10_000, &data),
            9.0 * (0.67 + 0.33 * 0.5),
        );
        assert_close(sprite(3_000, &data).speed(10_000, &data), 9.0);
    }

    #[test]
    fn a_led_egg_goes_at_a_newborn_s_pace_however_long_ago_it_was_laid() {
        // M2 design §10.4: its age counts from hatching.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let genes = Copied {
            genome: sprite(0, &data).genome,
            flaws: Vec::new(),
            errors: Vec::new(),
        };
        let egg = Sprite::egg(genes, at(0, 0), 10_000, Vec::new(), &data);
        assert_close(egg.speed(10_000, &data), 9.0 * 0.67);
        assert_close(egg.speed(10_700, &data), 9.0 * 0.67);
    }

    /// The starter, `age` ticks old on tick 100,000: it grows old at
    /// 22,500 ticks.
    fn starter(age: u64, data: &DataPack) -> Sprite {
        Sprite::aged(data.starter().clone(), at(0, 0), 100_000, age, data)
    }

    /// Ages `sprite`'s genes until its body gives out, or for at most
    /// `limit` ticks. Returns how many ticks that took.
    fn until_it_gives_out(sprite: &mut Sprite, data: &DataPack, limit: u32) -> u32 {
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let gives_out = data.physiology().ageing.decay.gives_out;
        for tick in 1..=limit {
            sprite.age_genes(data, &mut rng);
            if sprite.program.vigour <= gives_out {
                return tick;
            }
        }
        limit
    }

    /// The average health of `sprite`'s genes that `frailty` picks.
    fn mean_health(sprite: &Sprite, pick: impl Fn(f32) -> bool) -> f32 {
        let healths: Vec<f32> = (0..sprite.genome.genes.len())
            .filter(|&i| pick(frailty(&sprite.genome.genes[i])))
            .map(|i| sprite.health[i])
            .collect();
        healths.iter().sum::<f32>() / healths.len() as f32
    }

    #[test]
    fn an_adult_s_genes_stay_whole_while_it_isn_t_hurt() {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let mut adult = starter(6_000, &data);
        assert_eq!(adult.stage(), LifeStage::Adult);
        until_it_gives_out(&mut adult, &data, 20_000);
        assert!(adult.health.iter().all(|&h| h == 1.0));
        assert_eq!(adult.program.vigour, 1.0);
    }

    #[test]
    fn an_old_sprite_s_genes_decay_until_its_body_gives_out_in_about_the_ticks_set() {
        // M2 design §9.4: 3,600 ticks for an unworn body, a third of that
        // for the most worn, when more than one gene weakens a tick.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let mut easy = starter(30_000, &data);
        assert_eq!(easy.stage(), LifeStage::Old);
        let genome = easy.genome.clone();
        let ticks = until_it_gives_out(&mut easy, &data, 100_000);
        assert!(
            (3_100..4_100).contains(&ticks),
            "an unworn body gave out after {ticks} ticks"
        );
        let mut hard = starter(30_000, &data);
        hard.body.wear = 1.0;
        let ticks = until_it_gives_out(&mut hard, &data, 100_000);
        assert!(
            (1_000..1_400).contains(&ticks),
            "the most worn body gave out after {ticks} ticks"
        );
        // Settings don't weaken, instincts weaken at half the pace of the
        // body's genes, and none of it is in the genome its eggs get.
        assert_eq!(mean_health(&easy, |f| f == 0.0), 1.0);
        let body = 1.0 - mean_health(&easy, |f| f == 1.0);
        let mind = 1.0 - mean_health(&easy, |f| f == 0.5);
        assert!(mind < body * 0.75, "the mind lost {mind}, the body {body}");
        assert_eq!(easy.genome, genome);
    }

    #[test]
    fn broken_genes_speed_the_decay_as_wear_does() {
        // M2 design §12.4: each broken gene counts as `flaws.wear` (0.05)
        // of wear, so twenty make the most worn body, a third of the ticks.
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let mut broken = starter(30_000, &data);
        let frail: Vec<usize> = (0..broken.genome.genes.len())
            .filter(|&i| frailty(&broken.genome.genes[i]) > 0.0)
            .collect();
        broken.flaws = vec![0; broken.genome.genes.len()];
        for &i in &frail[..20] {
            broken.flaws[i] = 2;
        }
        assert_eq!(broken.body.wear, 0.0);
        assert_close(broken.worn(&data), 1.0);
        let ticks = until_it_gives_out(&mut broken, &data, 100_000);
        assert!(
            (1_000..1_400).contains(&ticks),
            "a body with 20 broken genes gave out after {ticks} ticks"
        );
    }

    #[test]
    fn a_tiny_hit_lands_in_rounds_and_still_wears_the_body_out_in_the_ticks_set() {
        // A billion hits a tick would stall the world if each landed alone.
        let data = builtin_changing(
            "physiology.ron",
            &[(
                "decay: (ticks: 3600, hit: 0.01,",
                "decay: (ticks: 2, hit: 0.000000001,",
            )],
        );
        let mut old = starter(30_000, &data);
        assert_eq!(until_it_gives_out(&mut old, &data, 10), 2);
    }

    #[test]
    fn a_weakened_speed_gene_slows_the_sprite() {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let mut old = starter(30_000, &data);
        let speed = (0..old.genome.genes.len())
            .find(|&i| {
                matches!(
                    old.genome.genes[i],
                    Gene::Trait {
                        which: crate::registry::Trait::Speed,
                        ..
                    }
                )
            })
            .expect("the starter has a speed gene");
        old.health[speed] = 0.5;
        old.express(&data);
        // Halfway between the least speed, 4, and its gene's 7.
        assert_close(old.speed(100_000, &data), 5.5);
    }

    #[test]
    fn a_bad_injury_can_scar_a_gene_at_any_age() {
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let injury = data.physiology().indices.injury;
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let mut hurt = starter(500, &data);
        assert_eq!(hurt.stage(), LifeStage::Young);
        hurt.body.chems[injury] = 0.5;
        for _ in 0..5_000 {
            hurt.age_genes(&data, &mut rng);
        }
        assert_eq!(hurt.program.vigour, 1.0, "an injury of .5 isn't bad enough");
        hurt.body.chems[injury] = 0.6;
        for _ in 0..5_000 {
            hurt.age_genes(&data, &mut rng);
        }
        // About 10 scars, each 0.05 off a gene.
        let scarred = hurt.health.iter().filter(|&&h| h < 1.0).count();
        assert!((3..20).contains(&scarred), "{scarred} genes scarred");
        assert!(hurt.program.vigour < 1.0);
    }
}
