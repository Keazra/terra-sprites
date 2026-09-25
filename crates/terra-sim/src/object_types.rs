//! Object types (design §3.5): objects defined as data, with a closed rule
//! vocabulary. `objects.ron` is parsed into entries, then every name in it is
//! resolved and checked, so a loaded pack can't refer to anything that isn't there.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::data::{DataError, check_unique};
use crate::registry::{Category, Chemical, ChemicalClass, Locus, LocusKind, Verb};

/// The object types file, relative to the pack root.
pub(crate) const OBJECTS: &str = "objects.ron";

/// A validated object type. Names it referred to are resolved: object types to
/// their index in the pack's list, stages and counters to their index in this type.
#[derive(Debug, Clone)]
pub(crate) struct ObjectType {
    #[expect(
        dead_code,
        reason = "saves record object types by stable ID from slice 12"
    )]
    pub(crate) id: u16,
    pub(crate) name: String,
    pub(crate) category: Category,
    /// Nothing can move through it. In M1 every solid object is also a fixture,
    /// and every other object is an item (design §3.5.1).
    pub(crate) solid: bool,
    /// A verb table only, with no instances or lifecycle: water and sprites.
    pub(crate) pseudo: bool,
    pub(crate) counters: Vec<CounterDef>,
    pub(crate) stages: Vec<Stage>,
    pub(crate) rules: Vec<Rule>,
    #[expect(dead_code, reason = "sprites apply verbs from slice 6")]
    pub(crate) verbs: BTreeMap<Verb, Vec<Effect>>,
    pub(crate) visual: Vec<Visual>,
}

#[derive(Debug, Clone)]
pub(crate) struct CounterDef {
    pub(crate) name: String,
    pub(crate) max: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Stage {
    pub(crate) name: String,
    /// The shortest and longest the stage lasts, in ticks.
    pub(crate) ticks: (u32, u32),
    /// The stage that follows, or `None` if the object expires.
    pub(crate) next: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    pub(crate) trigger: Trigger,
    pub(crate) conditions: Vec<Condition>,
    pub(crate) effects: Vec<Effect>,
}

#[derive(Debug, Clone)]
pub(crate) struct Visual {
    pub(crate) conditions: Vec<Condition>,
    pub(crate) state: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Trigger {
    Every(u32),
    OnStageEnter(usize),
    OnExpire,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Condition {
    InStage(usize),
    Counter(usize, Cmp, u16),
    Chance(f32),
    Fertility(Cmp, f32),
    DensityBelow(usize, u16, u16),
    KeepsPathsOpen,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Effect {
    AddCounter(usize, i32),
    RequireCounter(usize, u16),
    SpawnNearby(usize, u16),
    SpreadTo(usize, u16, Vec<Condition>),
    ReplaceWith(usize),
    DestroySelf,
    /// A physical chemical's ID.
    Inject(Party, u16, f32),
    /// A pulse locus's ID.
    Signal(Party, u16),
    Push(u16),
}

/// A comparison in a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum Cmp {
    Lt,
    Le,
    Eq,
    Ne,
    Ge,
    Gt,
}

impl Cmp {
    /// Whether `left` compares with `right` this way.
    pub(crate) fn holds<T: PartialOrd>(self, left: T, right: T) -> bool {
        match self {
            Cmp::Lt => left < right,
            Cmp::Le => left <= right,
            Cmp::Eq => left == right,
            Cmp::Ne => left != right,
            Cmp::Ge => left >= right,
            Cmp::Gt => left > right,
        }
    }
}

/// Who a verb's effect acts on: the sprite doing the verb, or the sprite it's done to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum Party {
    Actor,
    Target,
}

/// Parses and validates `objects.ron`, returning the types in ascending ID order.
pub(crate) fn object_types(
    text_entries: Vec<TypeEntry>,
    chemicals: &[Chemical],
    loci: &[Locus],
) -> Result<Vec<ObjectType>, DataError> {
    check_unique(
        OBJECTS,
        text_entries.iter().map(|t| (t.id, t.name.as_str())),
    )?;
    let mut entries = text_entries;
    entries.sort_by_key(|t| t.id);
    let names = Names {
        types: entries
            .iter()
            .enumerate()
            .map(|(index, t)| (t.name.clone(), (index, t.pseudo)))
            .collect(),
        chemicals,
        loci,
    };
    entries
        .into_iter()
        .map(|entry| {
            let name = entry.name.clone();
            entry.resolve(&names).map_err(|problem| DataError::Invalid {
                file: OBJECTS.into(),
                message: format!("`{name}`: {problem}"),
            })
        })
        .collect()
}

/// Everything a name in `objects.ron` can refer to outside its own type.
struct Names<'a> {
    /// Object type name → (index in the sorted list, whether it's a pseudo type).
    types: BTreeMap<String, (usize, bool)>,
    chemicals: &'a [Chemical],
    loci: &'a [Locus],
}

impl Names<'_> {
    /// The index of the object type called `name`, which must be a real (not pseudo) type.
    fn real_type(&self, name: &str) -> Result<usize, String> {
        match self.types.get(name) {
            None => Err(format!("names the unknown object type `{name}`")),
            Some(&(_, true)) => Err(format!(
                "names `{name}`, a pseudo type, which has no objects"
            )),
            Some(&(index, false)) => Ok(index),
        }
    }
}

/// Where in a type a condition or effect appears, which decides what's allowed there.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Rule,
    Verb,
    Visual,
}

/// One entry of `objects.ron`, before validation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TypeEntry {
    id: u16,
    name: String,
    category: Category,
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(default)]
    pseudo: bool,
    #[serde(default)]
    counters: BTreeMap<String, u16>,
    #[serde(default)]
    stages: Vec<StageEntry>,
    #[serde(default)]
    rules: Vec<RuleEntry>,
    #[serde(default)]
    verbs: BTreeMap<Verb, Vec<EffectEntry>>,
    #[serde(default)]
    visual: Vec<VisualEntry>,
}

/// The closed set of tags an object type can have (design §3.5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum Tag {
    Solid,
    Fixture,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StageEntry {
    name: String,
    ticks: (u32, u32),
    next: NextEntry,
}

#[derive(Debug, Deserialize)]
enum NextEntry {
    Stage(String),
    Expire,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleEntry {
    trigger: TriggerEntry,
    #[serde(rename = "if", default)]
    conditions: Vec<ConditionEntry>,
    #[serde(rename = "do")]
    effects: Vec<EffectEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisualEntry {
    #[serde(rename = "if", default)]
    conditions: Vec<ConditionEntry>,
    state: String,
}

#[derive(Debug, Deserialize)]
enum TriggerEntry {
    Every(u32),
    OnStageEnter(String),
    OnExpire,
}

#[derive(Debug, Deserialize)]
enum ConditionEntry {
    InStage(String),
    Counter(String, Cmp, u16),
    Chance(f32),
    Fertility(Cmp, f32),
    DensityBelow(String, u16, u16),
    KeepsPathsOpen,
}

#[derive(Debug, Deserialize)]
enum EffectEntry {
    AddCounter(String, i32),
    RequireCounter(String, u16),
    SpawnNearby(String, u16),
    SpreadTo(String, u16, Vec<ConditionEntry>),
    ReplaceWith(String),
    DestroySelf,
    Inject(Party, String, f32),
    Signal(Party, String),
    Push(u16),
}

impl TypeEntry {
    /// The validated type, or what's wrong with the entry.
    fn resolve(self, names: &Names) -> Result<ObjectType, String> {
        let solid = self.tags.contains(&Tag::Solid);
        let fixture = self.tags.contains(&Tag::Fixture);
        match (solid, fixture) {
            (true, false) => {
                return Err(
                    "is solid but not a fixture: pushable solids aren't supported yet".into(),
                );
            }
            (false, true) => {
                return Err(
                    "is a fixture but not solid: walk-over fixtures aren't supported yet".into(),
                );
            }
            _ => {}
        }
        if self.pseudo {
            let lifecycle = [
                ("tags", !self.tags.is_empty()),
                ("counters", !self.counters.is_empty()),
                ("stages", !self.stages.is_empty()),
                ("rules", !self.rules.is_empty()),
                ("visual rules", !self.visual.is_empty()),
            ];
            if let Some((what, _)) = lifecycle.iter().find(|(_, present)| *present) {
                return Err(format!(
                    "is a pseudo type, a verb table only, so it can't have {what}"
                ));
            }
        }

        let counters: Vec<CounterDef> = self
            .counters
            .into_iter()
            .map(|(name, max)| CounterDef { name, max })
            .collect();
        if let Some(counter) = counters.iter().find(|c| c.max == 0) {
            return Err(format!(
                "the counter `{}` needs a maximum of at least 1",
                counter.name
            ));
        }
        let stage_names: Vec<&str> = self.stages.iter().map(|s| s.name.as_str()).collect();
        for (index, name) in stage_names.iter().enumerate() {
            if stage_names[..index].contains(name) {
                return Err(format!("the stage `{name}` is listed more than once"));
            }
        }
        let scope = Scope {
            names,
            counters: &counters,
            stages: &stage_names,
        };

        let stages = self
            .stages
            .iter()
            .map(|stage| scope.stage(stage))
            .collect::<Result<_, _>>()?;
        let rules = self
            .rules
            .iter()
            .enumerate()
            .map(|(index, rule)| {
                scope
                    .rule(rule)
                    .map_err(|e| format!("rule {}: {e}", index + 1))
            })
            .collect::<Result<_, _>>()?;
        let verbs = self
            .verbs
            .iter()
            .map(|(&verb, effects)| {
                if !verb.is_interaction() {
                    return Err(format!(
                        "has a verb table for {verb:?}, but only Eat, Drink, Hit and Play act on a target"
                    ));
                }
                let effects = effects
                    .iter()
                    .map(|effect| scope.effect(effect, Section::Verb))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("the {verb:?} verb: {e}"))?;
                Ok((verb, effects))
            })
            .collect::<Result<_, String>>()?;
        let visual = self
            .visual
            .iter()
            .enumerate()
            .map(|(index, visual)| {
                let conditions = scope
                    .conditions(&visual.conditions, Section::Visual)
                    .map_err(|e| format!("visual rule {}: {e}", index + 1))?;
                Ok(Visual {
                    conditions,
                    state: visual.state.clone(),
                })
            })
            .collect::<Result<_, String>>()?;

        Ok(ObjectType {
            id: self.id,
            name: self.name,
            category: self.category,
            solid,
            pseudo: self.pseudo,
            counters,
            stages,
            rules,
            verbs,
            visual,
        })
    }
}

/// The names visible inside one object type: the pack's, plus its own counters and stages.
struct Scope<'a> {
    names: &'a Names<'a>,
    counters: &'a [CounterDef],
    stages: &'a [&'a str],
}

impl Scope<'_> {
    fn counter(&self, name: &str) -> Result<usize, String> {
        self.counters
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| format!("names the unknown counter `{name}`"))
    }

    fn stage_index(&self, name: &str) -> Result<usize, String> {
        self.stages
            .iter()
            .position(|&s| s == name)
            .ok_or_else(|| format!("names the unknown stage `{name}`"))
    }

    fn stage(&self, stage: &StageEntry) -> Result<Stage, String> {
        let (min, max) = stage.ticks;
        if min < 1 || min > max {
            return Err(format!(
                "the stage `{}` lasts ({min}, {max}) ticks, but needs 1 ≤ min ≤ max",
                stage.name
            ));
        }
        let next = match &stage.next {
            NextEntry::Stage(name) => Some(self.stage_index(name)?),
            NextEntry::Expire => None,
        };
        Ok(Stage {
            name: stage.name.clone(),
            ticks: stage.ticks,
            next,
        })
    }

    fn rule(&self, rule: &RuleEntry) -> Result<Rule, String> {
        let trigger = match &rule.trigger {
            TriggerEntry::Every(0) => return Err("`Every(0)` never fires; use 1 or more".into()),
            &TriggerEntry::Every(n) => Trigger::Every(n),
            TriggerEntry::OnStageEnter(name) => Trigger::OnStageEnter(self.stage_index(name)?),
            TriggerEntry::OnExpire => Trigger::OnExpire,
        };
        Ok(Rule {
            trigger,
            conditions: self.conditions(&rule.conditions, Section::Rule)?,
            effects: rule
                .effects
                .iter()
                .map(|effect| self.effect(effect, Section::Rule))
                .collect::<Result<_, _>>()?,
        })
    }

    fn conditions(
        &self,
        conditions: &[ConditionEntry],
        section: Section,
    ) -> Result<Vec<Condition>, String> {
        conditions
            .iter()
            .map(|condition| self.condition(condition, section))
            .collect()
    }

    fn condition(&self, condition: &ConditionEntry, section: Section) -> Result<Condition, String> {
        Ok(match condition {
            ConditionEntry::InStage(name) => Condition::InStage(self.stage_index(name)?),
            &ConditionEntry::Counter(ref name, cmp, value) => {
                Condition::Counter(self.counter(name)?, cmp, value)
            }
            &ConditionEntry::Chance(p) => {
                if section == Section::Visual {
                    return Err("`Chance` isn't allowed, so drawing never uses the RNG".into());
                }
                if !(0.0..=1.0).contains(&p) {
                    return Err(format!("`Chance({p})` needs a probability from 0 to 1"));
                }
                Condition::Chance(p)
            }
            &ConditionEntry::Fertility(cmp, value) => Condition::Fertility(cmp, value),
            &ConditionEntry::DensityBelow(ref name, radius, max) => {
                Condition::DensityBelow(self.names.real_type(name)?, radius, max)
            }
            ConditionEntry::KeepsPathsOpen => Condition::KeepsPathsOpen,
        })
    }

    fn effect(&self, effect: &EffectEntry, section: Section) -> Result<Effect, String> {
        let verb_only = matches!(
            effect,
            EffectEntry::RequireCounter(..)
                | EffectEntry::Inject(..)
                | EffectEntry::Signal(..)
                | EffectEntry::Push(..)
        );
        if verb_only && section != Section::Verb {
            return Err(format!(
                "`{}` needs a sprite doing a verb, so it's only allowed in a verb table",
                effect_name(effect)
            ));
        }
        Ok(match effect {
            &EffectEntry::AddCounter(ref name, delta) => {
                Effect::AddCounter(self.counter(name)?, delta)
            }
            &EffectEntry::RequireCounter(ref name, n) => {
                Effect::RequireCounter(self.counter(name)?, n)
            }
            &EffectEntry::SpawnNearby(ref name, radius) => {
                Effect::SpawnNearby(self.names.real_type(name)?, radius)
            }
            &EffectEntry::SpreadTo(ref name, radius, ref conditions) => Effect::SpreadTo(
                self.names.real_type(name)?,
                radius,
                self.conditions(conditions, section)?,
            ),
            EffectEntry::ReplaceWith(name) => Effect::ReplaceWith(self.names.real_type(name)?),
            EffectEntry::DestroySelf => Effect::DestroySelf,
            &EffectEntry::Inject(party, ref name, amount) => {
                let chemical = self
                    .names
                    .chemicals
                    .iter()
                    .find(|c| &c.name == name)
                    .ok_or_else(|| format!("names the unknown chemical `{name}`"))?;
                if chemical.class != ChemicalClass::Physical {
                    return Err(format!(
                        "`Inject` can't target `{name}`: only physical chemicals are changed by objects"
                    ));
                }
                Effect::Inject(party, chemical.id, amount)
            }
            &EffectEntry::Signal(party, ref name) => {
                let locus = self
                    .names
                    .loci
                    .iter()
                    .find(|l| &l.name == name)
                    .ok_or_else(|| format!("names the unknown locus `{name}`"))?;
                if locus.kind != LocusKind::Pulse {
                    return Err(format!("`Signal` can't target `{name}`: it isn't a pulse"));
                }
                Effect::Signal(party, locus.id)
            }
            &EffectEntry::Push(tiles) => Effect::Push(tiles),
        })
    }
}

/// An effect's name as written in `objects.ron`.
fn effect_name(effect: &EffectEntry) -> &'static str {
    match effect {
        EffectEntry::AddCounter(..) => "AddCounter",
        EffectEntry::RequireCounter(..) => "RequireCounter",
        EffectEntry::SpawnNearby(..) => "SpawnNearby",
        EffectEntry::SpreadTo(..) => "SpreadTo",
        EffectEntry::ReplaceWith(..) => "ReplaceWith",
        EffectEntry::DestroySelf => "DestroySelf",
        EffectEntry::Inject(..) => "Inject",
        EffectEntry::Signal(..) => "Signal",
        EffectEntry::Push(..) => "Push",
    }
}
