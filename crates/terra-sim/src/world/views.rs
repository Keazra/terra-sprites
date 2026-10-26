//! The read-only views the front end reads the world through (design §2.2):
//! a sprite, an object, the Cursor, an egg and a sprite that died.

use super::World;
use crate::action::{self, ActionView};
use crate::ageing::{LifeStage, Switch};
use crate::biochem::Traits;
use crate::brain::{self, Explanation, Memory, RememberedPlace};
use crate::breeding::{self, Fertility, Sex, Unhatched};
use crate::courtship;
use crate::ecology::holds_without_drawing;
use crate::events::DeathCause;
use crate::expression::{Expression, expressions};
use crate::genome::{GeneView, Genome};
use crate::inheritance;
use crate::lineage::{Origin, Record};
use crate::map::{Dir, Pos};
use crate::mutation::CopyChangeView;
use crate::objects::{EntityId, Object};
use crate::perception::Target;
use crate::registry::{BrainParam, ChemicalKind};
use crate::sprites::Sprite;

/// A chemical's level in one sprite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChemicalLevel<'a> {
    /// The chemical's name in the data pack.
    pub name: &'a str,
    /// Physical, drive, learning signal or hormone.
    pub kind: ChemicalKind,
    /// From 0 to 1.
    pub level: f32,
    /// The level now less the level one tick ago.
    pub change: f32,
}

/// A read-only view of one sprite.
pub struct SpriteView<'a> {
    pub(super) id: EntityId,
    pub(super) sprite: &'a Sprite,
    pub(super) world: &'a World,
}

impl<'a> SpriteView<'a> {
    /// The sprite's entity ID.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The tile the sprite stands on.
    pub fn pos(&self) -> Pos {
        self.sprite.pos
    }

    /// The name the player gave it, if any (design v28 §6.5).
    pub fn name(&self) -> Option<&'a str> {
        self.sprite.name.as_deref()
    }

    /// Its genome, as it was born with it: what `g` exports (design v28
    /// §6.5).
    pub fn genome(&self) -> &'a Genome {
        &self.sprite.genome
    }

    /// Ticks since the sprite was born.
    pub fn age(&self) -> u64 {
        self.sprite.age(self.world.state.tick)
    }

    /// What the sprite is doing, or the action that last ended until the
    /// next one starts; `None` before its first.
    pub fn action(&self) -> Option<ActionView> {
        action::view(self.sprite, &self.world.data)
    }

    /// While the Cursor leads it, the steps left on its way to where it's
    /// heading, as near the Cursor as it can get: 0 once it's there, or as
    /// close as it can get (design v23 §6.5). `None` while it isn't led.
    pub fn lead_steps_left(&self) -> Option<u32> {
        action::lead_steps_left(self.sprite)
    }

    /// While a shove sends it, the way it slides and the tiles it has left
    /// to go (design v25 §3.5.4). `None` while it isn't sliding.
    pub fn slide(&self) -> Option<(Dir, u16)> {
        self.sprite.slide.map(|slide| (slide.dir, slide.left))
    }

    /// What its brain did at the latest step 5, explained (design §5.9), or
    /// `None` before its first decision.
    pub fn explain(&self) -> Option<Explanation<'a>> {
        self.sprite.brain.explain(&self.world.data)
    }

    /// What it has learned, furthest from nothing first (design v40 §5.9).
    /// Ties keep the order they're listed in.
    pub fn memory(&self) -> Vec<Memory> {
        self.sprite.brain.memory(&self.world.data)
    }

    /// The places it remembers, best remembered first (M2 design §7).
    pub fn remembered_places(&self) -> Vec<RememberedPlace> {
        self.sprite.brain.remembered_places(&self.world.data)
    }

    /// The reward less the punishment it took in on its last tick, which
    /// learning used up (design §5.6): `last_r`.
    pub fn felt(&self) -> f32 {
        self.sprite.brain.felt
    }

    /// The tile of the one thing its attention was on at the latest step 5
    /// (design §5.3): an object's tile, a water tile, or where a sprite it
    /// attends to is now. `None` before its first decision, with nothing in
    /// reach, once that thing is gone, or while the Cursor leads it, when it
    /// attends to nothing (design v23 §6.5).
    pub fn attending_to(&self) -> Option<Pos> {
        if self.sprite.lead.is_some() {
            return None;
        }
        let target = self.sprite.brain.snapshot.as_ref()?.target?;
        let (pos, _) = self.world.state.whereabouts(&self.world.data, target)?;
        Some(pos)
    }

    /// What it attends to (design §5.3), while that's still in the world:
    /// the thing `attending_to` gives the tile of.
    pub fn attended(&self) -> Option<Target> {
        self.attending_to()?;
        self.sprite.brain.snapshot.as_ref()?.target
    }

    /// Whether it stands with nothing to do (M2 design §19.2), as its
    /// `standing` sensor reads it: no action running, and neither led by the
    /// Cursor nor sliding from a shove. An action that ended on the latest
    /// tick still shows how it ended, so the sprite isn't standing yet.
    pub fn is_standing(&self) -> bool {
        let latest = self.world.state.tick.saturating_sub(1);
        action::is_standing(self.sprite)
            && self
                .sprite
                .action
                .as_ref()
                .is_none_or(|action| action.ended_at.is_none_or(|t| t < latest))
    }

    /// The traits its body has: its genes', clamped to physiology's ranges.
    pub fn traits(&self) -> Traits {
        self.sprite.program.traits
    }

    /// Every gene in the sprite's genome, in order, with how it's expressed
    /// at its life stage.
    pub fn genes(&self) -> Vec<(GeneView<'a>, Expression)> {
        let data = &self.world.data;
        let genome = &self.sprite.genome;
        genome
            .genes
            .iter()
            .map(|gene| gene.view(data))
            .zip(expressions(genome, data, self.life_stage()))
            .collect()
    }

    /// When each gene in its genome works, in genome order (M2 design §9.2).
    pub fn switches(&self) -> Vec<Switch> {
        self.sprite.genome.switches()
    }

    /// Its life stage (M2 design §9.1), as of the latest tick.
    pub fn life_stage(&self) -> LifeStage {
        self.sprite.stage()
    }

    /// Its parents (M2 design §10): the one that laid it, then its partner,
    /// if it had one. None for a sprite the world began with or the Cursor
    /// made. A parent may have died since.
    pub fn parents(&self) -> &'a [EntityId] {
        &self.sprite.parents
    }

    /// Its generation (M2 design §17.1): the first for a sprite with no
    /// parents, else one after its newer parent's.
    pub fn generation(&self) -> u32 {
        self.sprite.generation
    }

    /// Where it came from (M2 design §17.1): the world began with it, the
    /// Cursor placed it, or it hatched.
    pub fn origin(&self) -> Origin {
        Origin::of(&self.sprite.parents, self.sprite.placed)
    }

    /// Its sex (M2 design §10).
    pub fn sex(&self) -> Sex {
        self.sprite.sex
    }

    /// Its partner, the sprite it last had an egg with, and whether the
    /// partner has died since (M2 design §15.4). A sprite whose partner died
    /// keeps its name until it pairs again.
    pub fn partner(&self) -> Option<(EntityId, bool)> {
        self.sprite
            .partner
            .map(|partner| (partner.id, partner.died))
    }

    /// How well it knows `other`, from 0 to 1 (M2 design §15.2): 0 for a
    /// sprite it doesn't remember.
    pub fn familiarity(&self, other: EntityId) -> f32 {
        let known = self.sprite.brain.experience.individuals.get(&other);
        known.map_or(0.0, |known| known.familiarity)
    }

    /// Each sprite it remembers, in ID order, with how well it knows it
    /// (M2 design §15.2).
    pub fn known_sprites(&self) -> Vec<(EntityId, f32)> {
        let known = &self.sprite.brain.experience.individuals;
        known
            .iter()
            .map(|(&id, known)| (id, known.familiarity))
            .collect()
    }

    /// The calls it remembers hearing, each sprite's latest: who called,
    /// and the tile it called from (M2 design §15.1).
    pub fn heard(&self) -> Vec<(EntityId, Pos)> {
        let heard = self.sprite.heard.iter();
        heard.map(|heard| (heard.caller, heard.from)).collect()
    }

    /// Whether it sees `pos`: within its sense radius, in any direction
    /// (M2 design §15).
    pub fn sees(&self, pos: Pos) -> bool {
        self.sprite.sees(pos)
    }

    /// Whether `other` is its close kin: a parent, a child, or a brother
    /// or sister (M2 design §15.5).
    pub fn is_kin(&self, other: EntityId) -> bool {
        courtship::kin(&self.world.state, self.id, other)
    }

    /// Whether it's fertile now (M2 design §10): grown up, fed, watered and
    /// rested since its last egg, in a world with room for another sprite,
    /// and with room nearby (§10.2).
    pub fn is_fertile(&self) -> bool {
        self.fertility() == Fertility::Fertile
    }

    /// Whether it can breed now, and if its body could but room stops it,
    /// which room (M2 design §10.2, §10.6).
    pub fn fertility(&self) -> Fertility {
        let state = &self.world.state;
        breeding::fertility(state, self.id, &self.world.data)
    }

    /// The level of its `feminiser` hormone over the level at which a male
    /// turns female, capped at 1 (M2 design §10): how near it is to turning.
    /// `None` for a female, or in a pack without breeding.
    pub fn turning(&self) -> Option<f32> {
        let physiology = self.world.data.physiology();
        let indices = physiology.indices.breeding?;
        let level = self.sprite.body.chems[indices.feminiser];
        let share = level / physiology.breeding.turns_female.max(f32::MIN_POSITIVE);
        (self.sprite.sex == Sex::Male).then_some(share.min(1.0))
    }

    /// How worn a hard life has left its body, from 0 to 1 (M2 design
    /// §9.4).
    pub fn wear(&self) -> f32 {
        self.sprite.body.wear
    }

    /// The age at which it grows old, in ticks (M2 design §9.3, §9.4): its
    /// `grows_old` trait, brought sooner by a hard life.
    pub fn grows_old_at(&self) -> f32 {
        let sooner = self.sprite.sooner(&self.world.data);
        self.sprite.program.traits.grows_old_at(sooner)
    }

    /// The average health of its genes that weaken, from 1 (all whole) to
    /// 0 (M2 design §9.4). Its body gives out at about half.
    pub fn vigour(&self) -> f32 {
        self.sprite.program.vigour
    }

    /// The health of each of its genes, in genome order, from 1 (whole) to
    /// 0 (M2 design §9.4).
    pub fn gene_health(&self) -> &'a [f32] {
        &self.sprite.health
    }

    /// The least pull that starts a verb while it isn't busy, as its brain
    /// takes it (M2 design §19.2): its expressed `act_threshold` gene, kept
    /// in range, or the pack's default without one.
    pub fn act_threshold(&self) -> f32 {
        self.sprite.brain.params.get(BrainParam::ActThreshold)
    }

    /// How many of its two copies of each gene carry a hidden flaw, from 0
    /// to 2, in genome order (M2 design §12). With one it carries the flaw
    /// unharmed; with both, the gene is broken.
    pub fn flaws(&self) -> Vec<u8> {
        (0..self.sprite.genome.genes.len())
            .map(|index| inheritance::flaws_of(&self.sprite.flaws, index))
            .collect()
    }

    /// The genes copying changed as it was laid (M2 design §12), in genome
    /// order: each one's place in its genome, and what it was as the egg
    /// inherited it. None for a sprite with no parents.
    pub fn copying_errors(&self) -> Vec<(usize, GeneView<'a>)> {
        let data = &self.world.data;
        self.sprite
            .copying_errors
            .iter()
            .map(|error| (error.gene, error.was.view(data)))
            .collect()
    }

    /// The whole genes copying doubled, lost or changed in what they do as
    /// it was laid (M2 design §13), in the order copying came to them. None
    /// for a sprite with no parents.
    pub fn copy_changes(&self) -> Vec<CopyChangeView<'a>> {
        let data = &self.world.data;
        self.sprite
            .copy_changes
            .iter()
            .map(|change| change.view(data))
            .collect()
    }

    /// Every chemical in the sprite, in the data pack's order. The breeding
    /// drive reads 0 in a sprite not grown up, whatever its genes, as it
    /// does to other sprites (M2 design §5).
    pub fn chemicals(&self) -> impl Iterator<Item = ChemicalLevel<'a>> + use<'a> {
        let body = &self.sprite.body;
        let hidden = self.hidden_drive();
        self.world
            .data
            .chemicals()
            .iter()
            .zip(body.chems.iter().zip(&body.chems_before_tick))
            .enumerate()
            .map(move |(index, (chemical, (&level, &before)))| {
                let (level, before) = if Some(index) == hidden {
                    (0.0, 0.0)
                } else {
                    (level, before)
                };
                ChemicalLevel {
                    name: &chemical.name,
                    kind: chemical.kind(),
                    level,
                    change: level - before,
                }
            })
    }

    /// The level of the chemical called `name`, or `None` if the pack has no
    /// such chemical. The breeding drive reads 0 in a sprite not grown up
    /// (M2 design §5).
    pub fn chemical(&self, name: &str) -> Option<f32> {
        self.chemicals()
            .find(|chemical| chemical.name == name)
            .map(|chemical| chemical.level)
    }

    /// The chemical the breeding drive reads, if this sprite isn't grown up:
    /// the one its views show as 0 (M2 design §5).
    fn hidden_drive(&self) -> Option<usize> {
        let grown = self.sprite.stage().is_grown();
        if grown {
            None
        } else {
            brain::drive_chemical(&self.world.data)
        }
    }
}

/// A read-only view of one object: on the map, or the item the Cursor holds
/// (`CursorView::holds`).
pub struct ObjectView<'a> {
    pub(super) id: EntityId,
    pub(super) object: &'a Object,
    pub(super) world: &'a World,
}

impl<'a> ObjectView<'a> {
    /// The object's entity ID.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The name of the object's type, as `objects.ron` gives it.
    pub fn type_name(&self) -> &'a str {
        &self.world.data.object_types()[self.object.kind].name
    }

    /// The tile the object stands on. The item the Cursor holds stands on
    /// none: for it, the tile it was picked up from.
    pub fn pos(&self) -> Pos {
        self.object.pos
    }

    /// Whether nothing can move through the object (design §3.3).
    pub fn is_solid(&self) -> bool {
        self.world.data.object_types()[self.object.kind].solid
    }

    /// Whether the Cursor may throw it, once it holds it (design §3.5.4):
    /// not an egg (M2 design §10.4).
    pub fn is_throwable(&self) -> bool {
        self.world.data.object_types()[self.object.kind].throwable
    }

    /// Whether it's an egg, which a sprite hatches from (M2 design §10.4):
    /// `World::egg` tells more.
    pub fn is_egg(&self) -> bool {
        self.object.unhatched.is_some()
    }

    /// The value of the object's counter called `name`, or `None` if its type has no such counter.
    pub fn counter(&self, name: &str) -> Option<u16> {
        let counters = &self.world.data.object_types()[self.object.kind].counters;
        let index = counters.iter().position(|c| c.name == name)?;
        Some(self.object.counters[index])
    }

    /// The name of the look a theme draws the object with: the state of the
    /// first of its type's visual rules whose conditions hold, or `"default"`.
    /// Held, it has no tile, so rules asking about one don't hold (design
    /// §3.5.2).
    pub fn visual_state(&self) -> &'a str {
        visual_state(self.world, self.object)
    }

    /// The name of the object's current stage, or `None` if its type has no stages.
    pub fn stage(&self) -> Option<&'a str> {
        let stages = &self.world.data.object_types()[self.object.kind].stages;
        self.object.stage.map(|stage| stages[stage].name.as_str())
    }
}

/// The name of the look a theme draws `object` with: the state of the first
/// of its type's visual rules whose conditions hold, or `"default"`.
fn visual_state<'a>(world: &'a World, object: &Object) -> &'a str {
    let object_type = &world.data.object_types()[object.kind];
    object_type
        .visual
        .iter()
        .find(|visual| {
            visual.conditions.iter().all(|condition| {
                holds_without_drawing(
                    &world.state.map,
                    &world.state.objects,
                    &world.data,
                    object,
                    object.pos,
                    condition,
                )
                .expect("visual rules never use Chance")
            })
        })
        .map_or("default", |visual| visual.state.as_str())
}

/// A read-only view of the Cursor, as far as it touches the world (design
/// v23 §6.5).
pub struct CursorView<'a> {
    pub(super) world: &'a World,
}

impl<'a> CursorView<'a> {
    /// The sprite the Cursor leads, if any.
    pub fn leads(&self) -> Option<EntityId> {
        self.world.state.cursor.leads()
    }

    /// The tile the world was last told the Cursor is on, which a led
    /// sprite heads for (design v23 §2.5).
    pub fn tile(&self) -> Option<Pos> {
        self.world.state.cursor.tile
    }

    /// Whether sprites can see the Cursor (design v29 §6.5).
    pub fn visible(&self) -> bool {
        self.world.state.cursor.visible
    }

    /// The item the Cursor holds, if any: an object off the map, never
    /// solid.
    pub fn holds(&self) -> Option<ObjectView<'a>> {
        let id = self.world.state.cursor.holds()?;
        let object = self.world.state.objects.get(id)?;
        Some(ObjectView {
            id,
            object,
            world: self.world,
        })
    }
}

/// A read-only view of an egg (M2 design §10.4): an item, on the map or in
/// the Cursor, holding the young sprite it hatches into.
pub struct EggView<'a> {
    pub(super) id: EntityId,
    pub(super) object: &'a Object,
    pub(super) unhatched: &'a Unhatched,
    pub(super) world: &'a World,
}

impl<'a> EggView<'a> {
    /// The egg's entity ID, which the sprite it hatches into keeps.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The name of its type, as `objects.ron` gives it.
    pub fn type_name(&self) -> &'a str {
        &self.world.data.object_types()[self.object.kind].name
    }

    /// The tile it lies on, or `None` while the Cursor holds it.
    pub fn pos(&self) -> Option<Pos> {
        (!self.object.held).then_some(self.object.pos)
    }

    /// Who laid it, then the partner, if there was one. A parent may have
    /// died since.
    pub fn parents(&self) -> &'a [EntityId] {
        &self.unhatched.parents
    }

    /// The tick it was laid on.
    pub fn laid(&self) -> u64 {
        self.unhatched.laid
    }

    /// The genome the sprite it holds hatches with, as copying left it (M2
    /// design §12).
    pub fn genome(&self) -> &'a Genome {
        &self.unhatched.genome
    }

    /// The soonest tick it hatches on: when its stages up to hatching end,
    /// each as short as it can be after the one it's in. Held, or with a
    /// sprite on it, it hatches once it can after that.
    pub fn hatches(&self) -> u64 {
        let object_type = &self.world.data.object_types()[self.object.kind];
        let stage = self
            .object
            .stage
            .expect("an egg has a stage that ends in hatching");
        self.object.stage_ends + object_type.least_ticks_after(stage)
    }

    /// The name the player gave it, which the sprite keeps, if any.
    pub fn name(&self) -> Option<&'a str> {
        self.unhatched.name.as_deref()
    }

    /// The generation the sprite it holds hatches into (M2 design §17.1).
    pub fn generation(&self) -> u32 {
        self.unhatched.generation
    }
}

/// A read-only view of a sprite that died, as the world remembers it for
/// the family tree (M2 design §17.1).
pub struct DeadView<'a> {
    pub(super) id: EntityId,
    pub(super) record: &'a Record,
}

impl<'a> DeadView<'a> {
    /// The ID it had.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The name the player gave it, if any.
    pub fn name(&self) -> Option<&'a str> {
        self.record.name.as_deref()
    }

    /// Its parents: the one that laid it, then its partner, if it had one.
    pub fn parents(&self) -> &'a [EntityId] {
        &self.record.parents
    }

    /// Its generation.
    pub fn generation(&self) -> u32 {
        self.record.generation
    }

    /// Where it came from.
    pub fn origin(&self) -> Origin {
        Origin::of(&self.record.parents, self.record.placed)
    }

    /// The tick it died on.
    pub fn died(&self) -> u64 {
        self.record.died
    }

    /// Its age when it died, in ticks.
    pub fn age(&self) -> u64 {
        self.record.age()
    }

    /// What it died of.
    pub fn cause(&self) -> DeathCause {
        self.record.cause
    }

    /// Its life stage when it died.
    pub fn life_stage(&self) -> LifeStage {
        self.record.stage
    }

    /// Its sex when it died.
    pub fn sex(&self) -> Sex {
        self.record.sex
    }
}
