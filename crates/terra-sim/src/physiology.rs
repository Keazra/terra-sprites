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
    /// How a sprite grows up and grows old (M2 design §9).
    pub(crate) ageing: Ageing,
    /// How sprites have young (M2 design §10).
    pub(crate) breeding: Breeding,
    /// How calls carry and what they cost (M2 design §15.1).
    pub(crate) calls: Calls,
    /// The copying errors in an egg's genes (M2 design §12).
    pub(crate) mutation: Mutation,
    /// The hidden flaws genes carry, and what a broken gene does (M2 design
    /// §12).
    pub(crate) flaws: Flaws,
    /// The whole genes copying doubles, loses or changes in an egg (M2
    /// design §13).
    pub(crate) copy_and_change: CopyAndChange,
    /// What each cause-of-death tally is multiplied by every tick: the file's
    /// `cause_fade` is how many ticks it takes to halve.
    pub(crate) tally_fade: f32,
    pub(crate) traits: TraitRanges,
    pub(crate) brain: BrainRanges,
    /// Each receptor target's range, by locus ID.
    pub(crate) receptor_targets: BTreeMap<LocusId, (f32, f32)>,
    pub(crate) nearby_sprites: NearbySprites,
    pub(crate) spawn_variation: f32,
    /// The traits that vary by their own share at spawn rather than by
    /// `spawn_variation` (M2 design §9.3).
    pub(crate) trait_variation: BTreeMap<Trait, f32>,
    /// How far a learned value gets from 0 to be a lesson (design §5.6).
    pub(crate) lesson_threshold: f32,
    /// The least fall of a need in a tick that is relief (design §5.6).
    pub(crate) relief_deadband: f32,
    /// How many ticks after a try its target is still the thing touched.
    pub(crate) touch_window: u64,
    /// A remembered sprite is forgotten once everything learned about it is
    /// nearer 0 than this (design v18 §5.6).
    pub(crate) forget_below: f32,
    /// How many places a sprite remembers, and how near two of a kind are
    /// one place (M2 design §7).
    pub(crate) remembered_places: RememberedPlaces,
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
    /// A pack from before life stages takes the built-in ageing, without
    /// `thirst_childhood`, as it's from before Slice 29 too, so its young
    /// dry out at the grown rate (design §2.8: a save carries its own pack).
    #[serde(default)]
    ageing: Ageing,
    /// A pack from before breeding takes the built-in breeding (design
    /// §2.8).
    #[serde(default)]
    breeding: Breeding,
    /// A pack from before calls takes the built-in calls, which do nothing
    /// in it: it has no courtship's loci, so no sprite calls (M2 design
    /// §15).
    #[serde(default)]
    calls: Calls,
    /// A pack from before copying errors has none (design §2.8: a save
    /// carries its own pack, and keeps its rules).
    #[serde(default)]
    mutation: Mutation,
    /// A pack from before hidden flaws has none (design §2.8).
    #[serde(default)]
    flaws: Flaws,
    /// A pack from before copy and change has none (design §2.8).
    #[serde(default)]
    copy_and_change: CopyAndChange,
    cause_fade: u32,
    traits: TraitRanges,
    brain: BTreeMap<String, ParamRange>,
    receptor_targets: BTreeMap<String, (f32, f32)>,
    nearby_sprites: NearbySprites,
    spawn_variation: f32,
    /// A pack from before lives grew longer has none: every trait varies by
    /// `spawn_variation` (design §2.8).
    #[serde(default)]
    trait_variation: BTreeMap<String, f32>,
    lesson_threshold: f32,
    relief_deadband: f32,
    touch_window: u64,
    forget_below: f32,
    /// A pack from before remembered places takes the built-in ones (design
    /// §2.8: a save carries its own pack).
    #[serde(default)]
    remembered_places: RememberedPlaces,
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

/// What a sprite remembers of places (M2 design §7).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RememberedPlaces {
    /// The most places a sprite remembers: past it, the faintest goes.
    pub(crate) held: u16,
    /// The most places of one kind it remembers: past it, the faintest of
    /// that kind goes, so a big lake's shores can't crowd out food. A world
    /// saved before it had a limit gets the built-in one.
    #[serde(default = "RememberedPlaces::built_in_per_kind")]
    pub(crate) per_kind: u16,
    /// Places of one kind this many tiles apart or fewer, in any direction,
    /// are one place, so a pond is one place, not each of its tiles.
    pub(crate) merge: u16,
}

impl RememberedPlaces {
    fn built_in_per_kind() -> u16 {
        RememberedPlaces::default().per_kind
    }
}

/// What a saved world's pack from before them gets: the built-in pack's,
/// which a test checks they match.
impl Default for RememberedPlaces {
    fn default() -> RememberedPlaces {
        RememberedPlaces {
            held: 8,
            per_kind: 3,
            merge: 5,
        }
    }
}

/// The range and default of a brain parameter added after saves began
/// (design §2.8), which a saved world's pack from before it doesn't name: the
/// built-in pack's, which a test checks they match.
fn added_since_saves(param: BrainParam) -> Option<ParamRange> {
    match param {
        BrainParam::PlaceFade => Some(ParamRange {
            range: (0.0, 0.01),
            default: 0.0002,
        }),
        BrainParam::Plasticity => Some(ParamRange {
            range: (0.25, 4.0),
            default: 1.0,
        }),
        BrainParam::TaughtRate => Some(ParamRange {
            range: (0.0, 1.0),
            default: 0.9,
        }),
        BrainParam::TaughtFade => Some(ParamRange {
            range: (0.0, 0.01),
            default: 0.00000625,
        }),
        BrainParam::BefriendRate => Some(ParamRange {
            range: (0.0, 0.01),
            default: 0.0013,
        }),
        BrainParam::BefriendPlay => Some(ParamRange {
            range: (0.0, 1.0),
            default: 0.1,
        }),
        BrainParam::BefriendFade => Some(ParamRange {
            range: (0.0, 0.01),
            default: 0.000025,
        }),
        BrainParam::WatchRate => Some(ParamRange {
            range: (0.0, 1.0),
            default: 0.0,
        }),
        BrainParam::ActThreshold => Some(ParamRange {
            range: (0.0, 2.0),
            default: 0.0,
        }),
        _ => None,
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
    /// What the basal and sense costs are multiplied by while it rests (M2
    /// design §19.2): 1, the same as standing awake, for a pack from before
    /// Slice 29.
    #[serde(default = "same")]
    pub(crate) resting: f32,
}

/// A multiplier that changes nothing, for a pack from before it.
fn same() -> f32 {
    1.0
}

/// The gut, and how what's in it moves into the body (M2 design §19.4).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Digestion {
    /// How much the one gut holds, food and water together, at most 1: a
    /// bite or a drink that doesn't fit is Too full. `None`, for a pack
    /// from before Slice 29, is two guts with no limit.
    #[serde(default)]
    pub(crate) gut: Option<f32>,
    /// Food, into energy.
    pub(crate) food: GutContent,
    /// Water, into hydration.
    pub(crate) water: GutContent,
}

/// One thing in the gut: how much of it moves into the body a tick, and
/// whether it waits for the body to have room or passes through, the body
/// keeping what it can (M2 design §19.4).
#[derive(Debug, Clone, Copy)]
pub(crate) struct GutContent {
    pub(crate) pace: f32,
    pub(crate) waits: bool,
}

/// A thing in the gut as the file has it: `(pace: 0.001, waits: true)`,
/// or as a pack from before Slice 29 has it, only its pace, and then it
/// passes through.
impl<'de> Deserialize<'de> for GutContent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<GutContent, D::Error> {
        deserializer.deserialize_any(GutContentVisitor)
    }
}

struct GutContentVisitor;

impl<'de> serde::de::Visitor<'de> for GutContentVisitor {
    type Value = GutContent;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a pace, or `(pace: ..., waits: ...)`")
    }

    fn visit_f64<E: serde::de::Error>(self, pace: f64) -> Result<GutContent, E> {
        Ok(GutContent {
            pace: pace as f32,
            waits: false,
        })
    }

    fn visit_u64<E: serde::de::Error>(self, pace: u64) -> Result<GutContent, E> {
        self.visit_f64(pace as f64)
    }

    fn visit_i64<E: serde::de::Error>(self, pace: i64) -> Result<GutContent, E> {
        self.visit_f64(pace as f64)
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<GutContent, A::Error> {
        let (mut pace, mut waits) = (None, None);
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "pace" => pace = Some(map.next_value::<f32>()?),
                "waits" => waits = Some(map.next_value::<bool>()?),
                other => {
                    return Err(serde::de::Error::unknown_field(other, &["pace", "waits"]));
                }
            }
        }
        Ok(GutContent {
            pace: pace.ok_or_else(|| serde::de::Error::missing_field("pace"))?,
            waits: waits.ok_or_else(|| serde::de::Error::missing_field("waits"))?,
        })
    }
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

/// How a sprite grows up and grows old (M2 design §9).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ageing {
    /// The share of its speed a newborn walks at, rising steadily to all of
    /// it as it grows up (§9.5).
    pub(crate) young_speed: f32,
    /// The energy growing up takes, spread evenly over childhood (§9.3).
    pub(crate) growth: f32,
    /// Energy per tick, all its life, for every 10,000 ticks of `grows_old`
    /// (§9.3).
    pub(crate) upkeep: f32,
    /// What wears a body (§9.4).
    pub(crate) wear: Wear,
    /// How many times as fast the most worn old sprite's genes decay (§9.4).
    pub(crate) wear_speedup: f32,
    /// How much sooner than its genes say the most worn sprite grows old, as
    /// a share of that age (§9.4).
    pub(crate) sooner: f32,
    /// How an old sprite's genes decay (§9.4).
    pub(crate) decay: Decay,
    /// How a bad injury scars a gene, at any age (§9.4).
    pub(crate) scar: Scar,
    /// The childhood a young sprite's thirst is paced to, in ticks: its
    /// hydration loss, and its dying once it has dried out, are the grown
    /// rate times this over its `grows_up` (M2 design §19.5). `None`, for a
    /// pack from before Slice 29, is the grown rate.
    #[serde(default)]
    pub(crate) thirst_childhood: Option<f32>,
}

/// How an old sprite's genes decay (M2 design §9.4): damage its repair no
/// longer keeps up with lands on random genes, a hit at a time.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Decay {
    /// The ticks an unworn old sprite's genes take to fall to `gives_out`
    /// on average.
    pub(crate) ticks: u32,
    /// The health one hit takes off a gene; half that off an instinct, so
    /// the mind outlasts the body.
    pub(crate) hit: f32,
    /// The average health of its genes at which an old body gives out, and
    /// old age harms it.
    pub(crate) gives_out: f32,
}

/// How a bad injury scars a gene (M2 design §9.4).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scar {
    /// The injury level above which a sprite may be scarred.
    pub(crate) above: f32,
    /// The chance each tick above it that a random gene is scarred.
    pub(crate) chance: f32,
    /// The health a scar takes off the gene.
    pub(crate) hit: f32,
}

/// What wears a body, a little every tick, for good (M2 design §9.4).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wear {
    /// The level below which energy is starving, or hydration parched.
    pub(crate) low: f32,
    /// Wear per tick while starving.
    pub(crate) starving: f32,
    /// Wear per tick while parched.
    pub(crate) parched: f32,
    /// Wear per tick for each unit of injury.
    pub(crate) injured: f32,
}

/// What a saved world's pack from before life stages gets: the built-in
/// pack's, which a test checks they match, but for `thirst_childhood`.
impl Default for Ageing {
    fn default() -> Ageing {
        Ageing {
            young_speed: 0.67,
            growth: 0.3,
            upkeep: 0.0,
            wear: Wear {
                low: 0.25,
                starving: 0.0002,
                parched: 0.0002,
                injured: 0.001,
            },
            wear_speedup: 3.0,
            sooner: 0.33,
            decay: Decay {
                ticks: 3_600,
                hit: 0.01,
                gives_out: 0.5,
            },
            scar: Scar {
                above: 0.5,
                chance: 0.002,
                hit: 0.05,
            },
            // From before Slice 29 too, so its young dry out at the grown
            // rate (M2 design §19.5).
            thirst_childhood: None,
        }
    }
}

/// How sprites have young (M2 design §10).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Breeding {
    /// Only in a pack from before eggs were items (#196): how many ticks an
    /// egg took to hatch. Now an egg hatches as its type's stages say, in
    /// `objects.ron` (M2 design §10.4), and loading gives such a pack an
    /// egg type that takes as long.
    #[serde(default)]
    pub(crate) hatches: Option<u64>,
    /// The energy an egg costs, in all: its parents split it.
    pub(crate) cost: f32,
    /// The level an adult's energy and hydration must both be above for it
    /// to be fertile.
    pub(crate) fed: f32,
    /// How many ticks after its last egg an adult is fertile again.
    pub(crate) rest: u64,
    /// How few other grown sprites must be near a grown sprite for it to be
    /// fertile (M2 design §10.2, §10.9). A pack from before room nearby has
    /// none: only the world's limit holds breeding back.
    #[serde(default)]
    pub(crate) room_nearby: Option<RoomNearby>,
    /// The level of the hormone `feminiser` at which a male adult turns
    /// female, for good.
    pub(crate) turns_female: f32,
    /// Only in a pack from before Slice 32: the level of the hormone
    /// `longing` a female must have reached to breed alone. Now it's the
    /// body's rule, `alone_after` (M2 design §15.7).
    #[serde(default)]
    pub(crate) breeds_alone: Option<f32>,
    /// How many ticks a female must go with no male adult seen or heard
    /// before she can breed alone (M2 design §15.7). A pack from before
    /// Slice 32 has none, and gates it on `breeds_alone`.
    #[serde(default)]
    pub(crate) alone_after: Option<u64>,
    /// The level of the hormone `receptivity` a female must have for a pair
    /// to mate (M2 design §15.3). A pack from before courtship has none: 0.
    #[serde(default)]
    pub(crate) receptive: f32,
    /// How many ticks partners go with neither seeing nor hearing the other
    /// before they're freed (M2 design §15.4). A pack from before partners
    /// has none.
    #[serde(default)]
    pub(crate) apart: Option<u64>,
    /// The need that drives breeding, one of `brain_io.ron`'s: the engine
    /// has it work only between adults (M2 design §5).
    pub(crate) drive: String,
    /// The chance, at each gene, that a pair's egg switches to taking its
    /// genes from the other parent (M2 design §12), so that neighbouring
    /// genes come from one parent in stretches, as chromosomes cross over.
    /// A pack from before took each gene from either parent, as a coin
    /// toss: a chance of a half.
    #[serde(default = "gene_by_gene")]
    pub(crate) crossover: f32,
}

/// Room nearby (M2 design §10.2, §10.9): a grown sprite is fertile only
/// while fewer than `fewer_than` other grown sprites, adult or old, are
/// within `within` tiles of it in a straight line. Young sprites and eggs
/// don't count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoomNearby {
    pub(crate) within: u16,
    pub(crate) fewer_than: u16,
}

/// The `crossover` of a pack from before stretches: each gene from either
/// parent, as a coin toss.
fn gene_by_gene() -> f32 {
    0.5
}

/// What a saved world's pack from before breeding gets: the built-in
/// pack's, which a test checks they match, but for room nearby, which a
/// pack from before has none of (M2 design §10.9).
impl Default for Breeding {
    fn default() -> Breeding {
        Breeding {
            hatches: None,
            cost: 0.3,
            fed: 0.4,
            rest: 1_500,
            // From before room nearby too (#331).
            room_nearby: None,
            turns_female: 0.8,
            breeds_alone: None,
            alone_after: Some(4_500),
            receptive: 0.5,
            apart: Some(4_500),
            drive: "arousal".into(),
            crossover: 0.1,
        }
    }
}

/// How calls carry and what they cost (M2 design §15.1).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Calls {
    /// The energy a call costs.
    pub(crate) cost: f32,
    /// The most a call's way may cost, by each terrain's `sound`, for it to
    /// be heard.
    pub(crate) reach: u32,
    /// How many ticks a hearer goes on knowing who called and where from.
    pub(crate) heard_for: u64,
    /// How many ticks a caller stands listening after it calls, which is
    /// how long a call lasts.
    pub(crate) listen: u32,
}

/// What a saved world's pack from before calls gets: the built-in pack's,
/// which a test checks they match.
impl Default for Calls {
    fn default() -> Calls {
        Calls {
            cost: 0.002,
            reach: 300,
            heard_for: 150,
            listen: 10,
        }
    }
}

/// The copying errors in an egg's genes (M2 design §12): now and then a
/// gene value changes, mostly by a lot.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mutation {
    /// The chance that a gene value changes in an egg.
    pub(crate) chance: f32,
    /// How far a small change goes: the value is multiplied by a factor
    /// drawn evenly within this share of 1.
    pub(crate) small: f32,
    /// The changes that are large instead.
    pub(crate) large: Large,
}

/// Hidden flaws in genes (M2 design §12). A sprite has two copies of each
/// gene that can weaken, one from each parent, and either may carry a flaw.
/// One sound copy is enough; with both flawed, the gene is broken.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Flaws {
    /// The chance that a sound copy of a gene gets a flaw as an egg's genes
    /// are copied.
    pub(crate) arise: f32,
    /// The chance that a sprite made from the starter genome carries a flaw
    /// in one copy of a gene, as wild animals do.
    pub(crate) founders: f32,
    /// The health a broken gene is born with (M2 design §9.4).
    pub(crate) health: f32,
    /// The wear each broken gene counts as (M2 design §9.4), bringing old
    /// age sooner and speeding its decay.
    pub(crate) wear: f32,
}

/// What a pack from before hidden flaws gets: none.
impl Default for Flaws {
    fn default() -> Flaws {
        Flaws {
            arise: 0.0,
            founders: 0.0,
            health: 1.0,
            wear: 0.0,
        }
    }
}

/// Copy and change (M2 design §13): as an egg's genes are copied, now and
/// then a whole gene is doubled, lost, or changed in what it does.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyAndChange {
    /// The chance that a gene is doubled, the copy next to it.
    pub(crate) doubled: f32,
    /// The chance that a gene is lost.
    pub(crate) lost: f32,
    /// The chance that a gene is changed in what it does.
    pub(crate) changed: f32,
    /// The share of changes made on a fresh copy, so the original still
    /// works.
    pub(crate) on_copy: f32,
    /// The longest a genome may grow, as a multiple of the starter's length.
    pub(crate) longest: f32,
}

/// What a pack from before copy and change gets: none, and no limit on a
/// genome's length.
impl Default for CopyAndChange {
    fn default() -> CopyAndChange {
        CopyAndChange {
            doubled: 0.0,
            lost: 0.0,
            changed: 0.0,
            on_copy: 0.0,
            longest: f32::INFINITY,
        }
    }
}

/// The large copying errors (M2 design §12).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Large {
    /// The share of changes that are large.
    pub(crate) share: f32,
    /// How far a large change goes: the value is multiplied by a factor
    /// from 1 / `most` to `most`, as likely to halve as to double.
    pub(crate) most: f32,
}

/// What a pack from before copying errors gets: none, so its eggs are
/// copied exactly, as they were.
impl Default for Mutation {
    fn default() -> Mutation {
        Mutation {
            chance: 0.0,
            small: 0.0,
            large: Large {
                share: 0.0,
                most: 1.0,
            },
        }
    }
}

/// The range each trait is clamped to.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TraitRanges {
    pub(crate) speed: (f32, f32),
    pub(crate) sense_radius: (f32, f32),
    /// A pack from before M2 calls it `lifespan` (M2 design §9.3).
    #[serde(alias = "lifespan")]
    pub(crate) grows_old: (f32, f32),
    /// Added in M2 (M2 design §9.3); a pack saved before then has none.
    #[serde(default = "grows_up_range")]
    pub(crate) grows_up: (f32, f32),
}

/// The range of `grows_up` in a pack from before it was added.
fn grows_up_range() -> (f32, f32) {
    (1500.0, 13500.0)
}

impl TraitRanges {
    /// The range `which` is clamped to.
    pub(crate) fn range(&self, which: Trait) -> (f32, f32) {
        match which {
            Trait::Speed => self.speed,
            Trait::SenseRadius => self.sense_radius,
            Trait::GrowsOld => self.grows_old,
            Trait::GrowsUp => self.grows_up,
        }
    }
}

impl PhysiologyEntry {
    /// The need that drives breeding, as the file names it.
    pub(crate) fn drive(&self) -> &str {
        &self.breeding.drive
    }

    /// How many ticks an egg took to hatch, in a pack from before eggs were
    /// items (#196), which says.
    pub(crate) fn old_hatches(&self) -> Option<u64> {
        self.breeding.hatches
    }

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
            ("metabolism.resting", metabolism.resting),
            ("digestion.food.pace", digestion.food.pace),
            ("digestion.water.pace", digestion.water.pace),
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
        for (name, size) in [
            ("digestion.gut", digestion.gut),
            ("ageing.thirst_childhood", self.ageing.thirst_childhood),
        ] {
            if let Some(size) = size
                && !(size.is_finite() && size > 0.0)
            {
                return Err(format!("`{name}` is {size}, but must be a number above 0"));
            }
        }
        // Each thing in the gut stops at 1, so a bite that fitted a bigger
        // gut could still be lost (M2 design §19.4).
        if let Some(gut) = digestion.gut
            && gut > 1.0
        {
            return Err(format!(
                "`digestion.gut` is {gut}, but can't be above 1: each thing in the gut stops at 1"
            ));
        }
        let ageing = self.ageing;
        let (wear, decay, scar) = (ageing.wear, ageing.decay, ageing.scar);
        for (name, level) in [
            ("ageing.young_speed", ageing.young_speed),
            ("ageing.wear.low", wear.low),
            ("ageing.sooner", ageing.sooner),
            ("ageing.decay.hit", decay.hit),
            ("ageing.decay.gives_out", decay.gives_out),
            ("ageing.scar.above", scar.above),
            ("ageing.scar.chance", scar.chance),
            ("ageing.scar.hit", scar.hit),
        ] {
            fraction(name, level)?;
        }
        for (name, rate) in [
            ("ageing.growth", ageing.growth),
            ("ageing.upkeep", ageing.upkeep),
            ("ageing.wear.starving", wear.starving),
            ("ageing.wear.parched", wear.parched),
            ("ageing.wear.injured", wear.injured),
        ] {
            if !(rate.is_finite() && rate >= 0.0) {
                return Err(format!(
                    "`{name}` is {rate}, but must be a number, and can't be negative"
                ));
            }
        }
        if ageing.young_speed == 0.0 {
            return Err("`ageing.young_speed` must be above 0: a newborn has to move".into());
        }
        if decay.ticks == 0 {
            return Err("`ageing.decay.ticks` must be at least 1 tick".into());
        }
        if decay.hit == 0.0 || decay.gives_out == 1.0 {
            return Err(
                "`ageing.decay` needs a `hit` above 0 and `gives_out` below 1, or old age never ends"
                    .into(),
            );
        }
        if !(ageing.wear_speedup.is_finite() && ageing.wear_speedup >= 1.0) {
            return Err(format!(
                "`ageing.wear_speedup` is {}, but must be a number, at least 1",
                ageing.wear_speedup
            ));
        }
        let breeding = &self.breeding;
        fraction("breeding.cost", breeding.cost)?;
        fraction("breeding.fed", breeding.fed)?;
        fraction("breeding.turns_female", breeding.turns_female)?;
        if let Some(level) = breeding.breeds_alone {
            fraction("breeding.breeds_alone", level)?;
        }
        fraction("breeding.receptive", breeding.receptive)?;
        if breeding.apart == Some(0) {
            return Err("`breeding.apart` must be at least 1 tick".into());
        }
        let calls = self.calls;
        fraction("calls.cost", calls.cost)?;
        if calls.heard_for == 0 || calls.listen == 0 {
            return Err("`calls.heard_for` and `calls.listen` must be at least 1 tick".into());
        }
        if breeding.hatches == Some(0) {
            return Err("`breeding.hatches` must be at least 1 tick".into());
        }
        // With no rest, a female mated in a tick could lay again in the same
        // tick (M2 design §10).
        if breeding.rest == 0 {
            return Err("`breeding.rest` must be at least 1 tick".into());
        }
        // Fewer than none can't be near, so no sprite could breed; and
        // within no tiles none is near, so the rule would be off (M2 design
        // §10.9).
        if let Some(room) = breeding.room_nearby {
            if room.fewer_than == 0 {
                return Err("`breeding.room_nearby.fewer_than` must be at least 1".into());
            }
            if room.within == 0 {
                return Err("`breeding.room_nearby.within` must be at least 1 tile".into());
            }
        }
        fraction("breeding.crossover", breeding.crossover)?;
        let mutation = self.mutation;
        fraction("mutation.chance", mutation.chance)?;
        fraction("mutation.large.share", mutation.large.share)?;
        if !(0.0..1.0).contains(&mutation.small) {
            return Err(format!(
                "`mutation.small` is {}, but must be at least 0 and below 1",
                mutation.small
            ));
        }
        if !(mutation.large.most.is_finite() && mutation.large.most >= 1.0) {
            return Err(format!(
                "`mutation.large.most` is {}, but must be a number, at least 1",
                mutation.large.most
            ));
        }
        let flaws = self.flaws;
        fraction("flaws.arise", flaws.arise)?;
        fraction("flaws.founders", flaws.founders)?;
        fraction("flaws.health", flaws.health)?;
        if !(flaws.wear.is_finite() && flaws.wear >= 0.0) {
            return Err(format!(
                "`flaws.wear` is {}, but must be a number, and can't be negative",
                flaws.wear
            ));
        }
        let copy = self.copy_and_change;
        fraction("copy_and_change.doubled", copy.doubled)?;
        fraction("copy_and_change.lost", copy.lost)?;
        fraction("copy_and_change.changed", copy.changed)?;
        fraction("copy_and_change.on_copy", copy.on_copy)?;
        // One draw a gene decides which, if any, happens to it.
        if copy.doubled + copy.lost + copy.changed > 1.0 {
            return Err(
                "`copy_and_change`'s `doubled`, `lost` and `changed` add up to more than 1".into(),
            );
        }
        if copy.longest.is_nan() || copy.longest < 1.0 {
            return Err(format!(
                "`copy_and_change.longest` is {}, but must be a number, at least 1",
                copy.longest
            ));
        }
        if self.cause_fade == 0 {
            return Err("`cause_fade` must be at least 1 tick".into());
        }

        let traits = self.traits;
        for (name, bounds) in [
            ("traits.speed", traits.speed),
            ("traits.sense_radius", traits.sense_radius),
            ("traits.grows_old", traits.grows_old),
            ("traits.grows_up", traits.grows_up),
        ] {
            range(name, bounds)?;
            // A `grows_old` of 0 would make the age sensor 0 ÷ 0.
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
            let entry = self
                .brain
                .get(name)
                .copied()
                .or_else(|| added_since_saves(param))
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
        let mut trait_variation = BTreeMap::new();
        for (name, &spread) in &self.trait_variation {
            let which = Trait::named(name)
                .ok_or_else(|| format!("`trait_variation` names `{name}`, which isn't a trait"))?;
            if !(0.0..1.0).contains(&spread) {
                return Err(format!(
                    "`trait_variation.{name}` is {spread}, but must be at least 0 and below 1"
                ));
            }
            trait_variation.insert(which, spread);
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
        if self.remembered_places.held == 0 || self.remembered_places.per_kind == 0 {
            return Err("`remembered_places.held` and `per_kind` must be at least 1".into());
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
            ageing,
            breeding: self.breeding,
            calls: self.calls,
            mutation,
            flaws,
            copy_and_change: self.copy_and_change,
            tally_fade: halving_factor(self.cause_fade as f32),
            traits,
            brain: BrainRanges(brain),
            receptor_targets,
            nearby_sprites: self.nearby_sprites,
            spawn_variation: self.spawn_variation,
            trait_variation,
            lesson_threshold: self.lesson_threshold,
            relief_deadband: self.relief_deadband,
            touch_window: self.touch_window,
            forget_below: self.forget_below,
            remembered_places: self.remembered_places,
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
    /// 1 while it has no action (M2 design §19.2), if the pack has it: a
    /// pack from before Slice 29 doesn't.
    pub(crate) standing: Option<usize>,
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
    /// The pulse a bite or a drink with no room in the gut fires (M2 design
    /// §19.4), if the pack has it: a pack from before Slice 29 doesn't.
    pub(crate) too_full: Option<usize>,
    /// What breeding reads and writes (M2 design §10), if the pack has it
    /// all: a pack from before breeding doesn't.
    pub(crate) breeding: Option<BreedingIndices>,
    /// What calls and courtship read and write (M2 design §15), if the pack
    /// breeds and has them all: a pack from before them doesn't.
    pub(crate) courtship: Option<CourtshipIndices>,
    /// What families write (M2 design §15.6), if the pack has courtship and
    /// them all: a pack from before them doesn't.
    pub(crate) families: Option<FamilyIndices>,
    /// The need that drives breeding, `breeding.drive`, by its place among
    /// the needs, if it's one: it works only between adults, whether or not
    /// the pack breeds (M2 design §5).
    pub(crate) drive: Option<usize>,
}

/// The loci and chemicals breeding reads and writes (M2 design §10).
#[derive(Debug, Clone, Copy)]
pub(crate) struct BreedingIndices {
    /// The body sensors: 1 while it's fertile, 1 if it's female, 1 while
    /// it sees a female adult, and 1 while it sees a male adult.
    pub(crate) fertile: usize,
    pub(crate) female: usize,
    pub(crate) female_near: usize,
    pub(crate) male_near: usize,
    /// The pulse each parent fires as an egg is laid.
    pub(crate) bred: usize,
    /// The hormone that turns a male female, past `breeding.turns_female`.
    pub(crate) feminiser: usize,
    /// The hormone a female must have past `breeding.breeds_alone` to
    /// breed alone.
    pub(crate) longing: usize,
}

/// The loci and chemicals calls and courtship read and write (M2 design
/// §15).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CourtshipIndices {
    /// The pulses: it heard a call, another danced for it, it danced, and
    /// its partner died.
    pub(crate) heard: usize,
    pub(crate) courted: usize,
    pub(crate) danced: usize,
    pub(crate) lost_partner: usize,
    /// The body sensors: 1 while its partner is near, 1 while it has one,
    /// and how healthy it looks.
    pub(crate) partner_near: usize,
    pub(crate) paired: usize,
    pub(crate) condition: usize,
    /// The hormone a female must have past `breeding.receptive` to mate.
    pub(crate) receptivity: usize,
}

/// The body sensors families write (M2 design §15.6): 1 while an egg of
/// its own is out of sight, 1 while one of those was left alone, 1 while a
/// young one of its own is out of sight, 1 while it's young and sees no
/// living parent, and 1 while its partner is out of sight.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FamilyIndices {
    pub(crate) egg_away: usize,
    pub(crate) egg_alone: usize,
    pub(crate) young_away: usize,
    pub(crate) parent_away: usize,
    pub(crate) partner_away: usize,
    /// The pulse it feels on finding family it was missing.
    pub(crate) found: usize,
}

impl Indices {
    /// Finds each physical chemical and body sensor physiology needs, the
    /// receptor target the brain reads, and the pulses a cornered retreat
    /// and a fruitless try fire and an attacker is known by, or says which
    /// file lacks one. For breeding, it finds `drive` among the `needs`.
    pub(crate) fn find(
        chemicals: &[Chemical],
        loci: &[Locus],
        needs: &[&str],
        drive: &str,
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
        // The breeding drive works only between adults whenever it's one
        // of the needs, whether or not the pack breeds (M2 design §5). A
        // pack with breeding's loci and hormones has breeding, and its drive
        // must be one of the needs.
        let drive_place = needs.iter().position(|&need| need == drive);
        let locus = |name: &str, kind| loci.iter().position(|l| l.name == name && l.kind == kind);
        let breeding = match (
            locus("fertile", LocusKind::BodySensor),
            locus("female", LocusKind::BodySensor),
            locus("female_near", LocusKind::BodySensor),
            locus("male_near", LocusKind::BodySensor),
            locus("bred", LocusKind::Pulse),
            chemicals.iter().position(|c| c.name == "feminiser"),
            chemicals.iter().position(|c| c.name == "longing"),
        ) {
            (
                Some(fertile),
                Some(female),
                Some(female_near),
                Some(male_near),
                Some(bred),
                Some(feminiser),
                Some(longing),
            ) => {
                if drive_place.is_none() {
                    return Err((
                        "physiology.ron",
                        format!("`breeding.drive` is `{drive}`, but brain_io.ron has no such need"),
                    ));
                }
                Some(BreedingIndices {
                    fertile,
                    female,
                    female_near,
                    male_near,
                    bred,
                    feminiser,
                    longing,
                })
            }
            _ => None,
        };
        let courtship = match (
            breeding,
            locus("heard", LocusKind::Pulse),
            locus("courted", LocusKind::Pulse),
            locus("danced", LocusKind::Pulse),
            locus("lost_partner", LocusKind::Pulse),
            locus("partner_near", LocusKind::BodySensor),
            locus("paired", LocusKind::BodySensor),
            locus("condition", LocusKind::BodySensor),
            chemicals.iter().position(|c| c.name == "receptivity"),
        ) {
            (
                Some(_),
                Some(heard),
                Some(courted),
                Some(danced),
                Some(lost_partner),
                Some(partner_near),
                Some(paired),
                Some(condition),
                Some(receptivity),
            ) => Some(CourtshipIndices {
                heard,
                courted,
                danced,
                lost_partner,
                partner_near,
                paired,
                condition,
                receptivity,
            }),
            _ => None,
        };
        let sensor_if = |name: &str| locus(name, LocusKind::BodySensor);
        let families = match (
            courtship,
            sensor_if("egg_away"),
            sensor_if("egg_alone"),
            sensor_if("young_away"),
            sensor_if("parent_away"),
            sensor_if("partner_away"),
            locus("found_family", LocusKind::Pulse),
        ) {
            (
                Some(_),
                Some(egg_away),
                Some(egg_alone),
                Some(young_away),
                Some(parent_away),
                Some(partner_away),
                Some(found),
            ) => Some(FamilyIndices {
                egg_away,
                egg_alone,
                young_away,
                parent_away,
                partner_away,
                found,
            }),
            _ => None,
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
            standing: sensor_if("standing"),
            exploration_mod: target("exploration_mod")?,
            learning_rate_mod: target("learning_rate_mod")?,
            curiosity_mod: target("curiosity_mod")?,
            cornered: pulse("cornered")?,
            was_hit: pulse("was_hit")?,
            fruitless: pulse("fruitless")?,
            petted: pulse("petted")?,
            shocked: pulse("shocked")?,
            too_full: locus("too_full", LocusKind::Pulse),
            breeding,
            courtship,
            families,
            drive: drive_place,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{DataError, DataPack};
    use crate::test_util::builtin_changing;

    #[test]
    fn what_a_save_from_before_a_setting_gets_is_what_the_built_in_pack_says() {
        // A pack from before a setting gets the built-in pack's, written
        // twice: here and in `data/physiology.ron` (design §2.8).
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let physiology = data.physiology();
        for param in BrainParam::ALL {
            if let Some(added) = added_since_saves(param) {
                let built_in = physiology.brain.of(param);
                assert_eq!(
                    (added.range, added.default),
                    (built_in.range, built_in.default),
                    "{param:?}"
                );
            }
        }
        let (saved, built_in) = (RememberedPlaces::default(), physiology.remembered_places);
        assert_eq!(
            (saved.held, saved.per_kind, saved.merge),
            (built_in.held, built_in.per_kind, built_in.merge)
        );
        // A pack from before life stages is from before Slice 29 too, so
        // its young, such as an old save's or one the Cursor places, dry out
        // at the grown rate (M2 design §19.5).
        let ageing = Ageing {
            thirst_childhood: None,
            ..physiology.ageing
        };
        assert_eq!(Ageing::default(), ageing);
        // And from before room nearby (#331), so only the world's limit
        // holds its breeding back.
        let breeding = Breeding {
            room_nearby: None,
            ..physiology.breeding.clone()
        };
        assert_eq!(Breeding::default(), breeding);
        assert_eq!(Calls::default(), physiology.calls);
        assert_eq!(grows_up_range(), physiology.traits.grows_up);
    }

    #[test]
    fn the_gut_holds_at_most_1_as_each_thing_in_it_stops_at_1() {
        // M2 design §19.4: in a bigger gut, a bite could fit and still be
        // lost, as its food stopped at 1.
        let sources: Vec<(&str, String)> = DataPack::builtin_sources()
            .iter()
            .map(|&(path, text)| (path, text.replace("gut: 0.5,", "gut: 1.5,")))
            .collect();
        let sources: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
        match DataPack::from_sources(&sources).err() {
            Some(DataError::Invalid { file, message }) => {
                assert_eq!(file, "physiology.ron");
                assert!(message.contains("digestion.gut"), "{message}");
            }
            other => panic!("a gut of 1.5 should be refused, not {other:?}"),
        }
        let full = builtin_changing("physiology.ron", &[("gut: 0.5,", "gut: 1.0,")]);
        assert_eq!(full.physiology().digestion.gut, Some(1.0));
    }

    #[test]
    fn a_pack_from_before_genes_mixed_in_stretches_mixes_them_gene_by_gene() {
        // M2 design §12.2: its `breeding` has no `crossover`, and a switch
        // at every gene with even odds is a coin toss for each, as Slice 23
        // mixed them. And it has no copying errors or flaws.
        let data = builtin_changing(
            "physiology.ron",
            &[
                ("crossover: 0.1,", ""),
                (
                    "mutation: (chance: 0.01, small: 0.2, large: (share: 0.75, most: 2.0)),",
                    "",
                ),
                (
                    "flaws: (arise: 0.004, founders: 0.035, health: 0.5, wear: 0.05),",
                    "",
                ),
            ],
        );
        let physiology = data.physiology();
        assert_eq!(physiology.breeding.crossover, 0.5);
        assert_eq!(physiology.mutation.chance, 0.0);
        assert_eq!(physiology.flaws.arise, 0.0);
        assert_eq!(physiology.flaws.founders, 0.0);
    }
}
