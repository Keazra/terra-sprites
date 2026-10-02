//! Genomes (design §2.8, §4.3): ordered genes with stable type IDs and
//! versioned payloads, and the genome file format. A gene this build can't
//! read is kept exactly as it is.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::brain_io::{ATTENDED, InputId, TARGET_IDS};
use crate::data::DataPack;
use crate::registry::{BrainParam, CategoryId, ChemId, LocusId, Trait, Verb};

/// The genome file format this build writes, and the newest it reads.
const FORMAT: u32 = 1;
/// The payload version of every gene type this build knows.
const VERSION: u8 = 1;

/// A sprite's genes, in order. A genome is checked against the data pack it
/// was read with, and must be used with that pack.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Genome {
    pub(crate) genes: Vec<Gene>,
}

/// Why a genome file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenomeError {
    /// The text is not valid RON for a genome file.
    Parse(String),
    /// The file parses but breaks a rule of the format.
    Invalid(String),
}

impl std::fmt::Display for GenomeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenomeError::Parse(message) | GenomeError::Invalid(message) => f.write_str(message),
        }
    }
}

/// One gene. Everything it refers to is a stable registry ID.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) enum Gene {
    /// Type 1: the chemical decays by half every `ticks` ticks.
    HalfLife { chem: ChemId, ticks: u32 },
    /// Type 2: reactants turn into products, at `rate` of the most the reactants allow.
    Reaction {
        reactants: Vec<Term>,
        products: Vec<Term>,
        rate: f32,
    },
    /// Type 3: a locus's level, rise or fall, past `threshold`, adds `gain` of it to `chem`.
    Emitter {
        locus: LocusRef,
        mode: EmitterMode,
        invert: bool,
        threshold: f32,
        gain: f32,
        chem: ChemId,
    },
    /// Type 4: a chemical's level past `threshold` moves a receptor target by `gain` of it.
    Receptor {
        chem: ChemId,
        threshold: f32,
        gain: f32,
        target: LocusId,
    },
    /// Type 5: a chemical's level at birth.
    InitialConcentration { chem: ChemId, value: f32 },
    /// Type 6: a body trait.
    Trait { which: Trait, value: f32 },
    /// Type 7: a setting of how the brain works.
    BrainParam { param: BrainParam, value: f32 },
    /// Type 8: the concept of these inputs, each maybe negated, starts with
    /// `weight` towards `verb`.
    Instinct {
        inputs: Vec<(InputId, bool)>,
        verb: Verb,
        weight: f32,
    },
    /// Type 9: a State input starts with `weight` towards attending to `category`.
    AttentionInstinct {
        input: InputId,
        category: CategoryId,
        weight: f32,
    },
    /// A gene this build can't read: an unknown type, or a payload version
    /// newer than it knows. Kept exactly as it is.
    Unknown {
        type_id: u16,
        version: u8,
        payload: Vec<u8>,
    },
    /// A gene that names a category this world doesn't have (design v19
    /// §5.7), kept exactly as it was written. It has no effect.
    Unmatched(AsWritten),
}

/// How an unmatched gene was written, so it's written back the same way.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) enum AsWritten {
    /// An attention instinct by name, with its names as written.
    AttentionInstinct {
        input: String,
        category: String,
        weight: f32,
    },
    /// An instinct by name whose inputs include a missing category's
    /// `attended_<category>`, with its names as written.
    Instinct {
        inputs: Vec<(String, bool)>,
        verb: Verb,
        weight: f32,
    },
    /// Any gene by number, as an unknown gene is.
    ByNumber {
        type_id: u16,
        version: u8,
        payload: Vec<u8>,
    },
}

impl AsWritten {
    /// The missing category it names, if it was written by name: an
    /// attention instinct's category, or the category of an instinct's
    /// attended input (`tree` for `attended_tree`).
    pub(crate) fn missing<'a>(&'a self, data: &DataPack) -> Option<&'a str> {
        match self {
            AsWritten::AttentionInstinct { category, .. } => Some(category),
            AsWritten::Instinct { inputs, .. } => inputs
                .iter()
                .map(|(name, _)| name.as_str())
                .find(|name| missing_named(name, data))
                .and_then(|name| name.strip_prefix(ATTENDED)),
            AsWritten::ByNumber { .. } => None,
        }
    }
}

/// A chemical and its coefficient in a reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Term {
    pub(crate) chem: ChemId,
    pub(crate) coefficient: u8,
}

/// What an emitter reads: a chemical's level, or another locus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum LocusRef {
    Chem(ChemId),
    Locus(LocusId),
}

/// One of a sprite's genes, with what it refers to by the data pack's names.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneView<'a> {
    /// Type 1: the chemical decays by half every `ticks` ticks.
    HalfLife { chem: &'a str, ticks: u32 },
    /// Type 2: reactants turn into products, as `(chemical, coefficient)`.
    Reaction {
        reactants: Vec<(&'a str, u8)>,
        products: Vec<(&'a str, u8)>,
        rate: f32,
    },
    /// Type 3: a locus's level, rise or fall, past `threshold`, adds `gain`
    /// of it to `chem`. The locus may be a chemical.
    Emitter {
        locus: &'a str,
        mode: EmitterMode,
        invert: bool,
        threshold: f32,
        gain: f32,
        chem: &'a str,
    },
    /// Type 4: a chemical's level past `threshold` moves a receptor target by `gain` of it.
    Receptor {
        chem: &'a str,
        threshold: f32,
        gain: f32,
        target: &'a str,
    },
    /// Type 5: a chemical's level at birth.
    InitialConcentration { chem: &'a str, value: f32 },
    /// Type 6: a body trait.
    Trait { which: Trait, value: f32 },
    /// Type 7: a setting of how the brain works.
    BrainParam { param: &'static str, value: f32 },
    /// Type 8: the concept of these inputs, as `(input, negated)`, starts
    /// with `weight` towards `verb`.
    Instinct {
        inputs: Vec<(&'a str, bool)>,
        verb: Verb,
        weight: f32,
    },
    /// Type 9: an input starts with `weight` towards attending to a category.
    AttentionInstinct {
        input: &'a str,
        category: &'a str,
        weight: f32,
    },
    /// A gene this build can't read, with the length of its payload.
    Unknown {
        type_id: u16,
        version: u8,
        bytes: usize,
    },
}

/// What an emitter responds to (design §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitterMode {
    /// The locus's value.
    Level,
    /// How much it went up since the previous tick.
    Rise,
    /// How much it went down since the previous tick.
    Fall,
}

impl Genome {
    /// Reads a genome file, resolving every name in it against `data`.
    pub fn from_ron(text: &str, data: &DataPack) -> Result<Genome, GenomeError> {
        let file: GenomeFile =
            ron::from_str(text).map_err(|e| GenomeError::Parse(e.to_string()))?;
        if !(1..=FORMAT).contains(&file.format) {
            return Err(GenomeError::Invalid(format!(
                "the genome's format is {}, but this build reads formats 1 to {FORMAT}",
                file.format
            )));
        }
        let genes = file
            .genes
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                let name = entry.type_name();
                entry
                    .resolve(data)
                    .and_then(|gene| gene.check(data).map(|()| gene))
                    .map_err(|problem| {
                        GenomeError::Invalid(format!("gene {} ({name}) {problem}", index + 1))
                    })
            })
            .collect::<Result<Vec<Gene>, GenomeError>>()?;
        Ok(Genome { genes })
    }

    /// Whether every gene fits `data`: what each refers to is in the pack,
    /// and its values are in range. A genome read with the pack always does;
    /// one a spawn carries is checked, since a replay may bring it from
    /// another pack (design v26 §2.5).
    pub(crate) fn fits(&self, data: &DataPack) -> bool {
        self.genes.iter().all(|gene| gene.check(data).is_ok())
    }

    /// The genome as a genome file: genes this build knows by name, the rest by number.
    pub fn to_ron(&self, data: &DataPack) -> String {
        let mut text = format!("(\n    format: {FORMAT},\n    genes: [\n");
        for gene in &self.genes {
            let _ = writeln!(text, "        {},", gene.to_ron(data));
        }
        text + "    ],\n)\n"
    }
}

impl Gene {
    /// The gene with what it refers to named from `data`.
    pub(crate) fn view<'a>(&'a self, data: &'a DataPack) -> GeneView<'a> {
        let chem = |id: ChemId| data.chemical(id).expect("a checked gene").name.as_str();
        let locus = |id: LocusId| data.locus(id).expect("a checked gene").name.as_str();
        let input = |id: InputId| data.brain_input(id).expect("a checked gene").name.as_str();
        let terms = |terms: &[Term]| {
            terms
                .iter()
                .map(|t| (chem(t.chem), t.coefficient))
                .collect()
        };
        match *self {
            Gene::HalfLife { chem: id, ticks } => GeneView::HalfLife {
                chem: chem(id),
                ticks,
            },
            Gene::Reaction {
                ref reactants,
                ref products,
                rate,
            } => GeneView::Reaction {
                reactants: terms(reactants),
                products: terms(products),
                rate,
            },
            Gene::Emitter {
                locus: read,
                mode,
                invert,
                threshold,
                gain,
                chem: id,
            } => GeneView::Emitter {
                locus: match read {
                    LocusRef::Chem(id) => chem(id),
                    LocusRef::Locus(id) => locus(id),
                },
                mode,
                invert,
                threshold,
                gain,
                chem: chem(id),
            },
            Gene::Receptor {
                chem: id,
                threshold,
                gain,
                target,
            } => GeneView::Receptor {
                chem: chem(id),
                threshold,
                gain,
                target: locus(target),
            },
            Gene::InitialConcentration { chem: id, value } => GeneView::InitialConcentration {
                chem: chem(id),
                value,
            },
            Gene::Trait { which, value } => GeneView::Trait { which, value },
            Gene::BrainParam { param, value } => GeneView::BrainParam {
                param: param.name(),
                value,
            },
            Gene::Instinct {
                ref inputs,
                verb,
                weight,
            } => GeneView::Instinct {
                inputs: inputs
                    .iter()
                    .map(|&(id, negated)| (input(id), negated))
                    .collect(),
                verb,
                weight,
            },
            Gene::AttentionInstinct {
                input: id,
                category,
                weight,
            } => GeneView::AttentionInstinct {
                input: input(id),
                category: &data.category(category).expect("a checked gene").name,
                weight,
            },
            Gene::Unknown {
                type_id,
                version,
                ref payload,
            }
            | Gene::Unmatched(AsWritten::ByNumber {
                type_id,
                version,
                ref payload,
            }) => GeneView::Unknown {
                type_id,
                version,
                bytes: payload.len(),
            },
            Gene::Unmatched(AsWritten::AttentionInstinct {
                ref input,
                ref category,
                weight,
            }) => GeneView::AttentionInstinct {
                input,
                category,
                weight,
            },
            Gene::Unmatched(AsWritten::Instinct {
                ref inputs,
                verb,
                weight,
            }) => GeneView::Instinct {
                inputs: inputs
                    .iter()
                    .map(|(name, negated)| (name.as_str(), *negated))
                    .collect(),
                verb,
                weight,
            },
        }
    }

    /// Checks the gene's references and values, or says what's wrong.
    fn check(&self, data: &DataPack) -> Result<(), String> {
        let chemical = |id: ChemId| {
            data.chemical(id)
                .map(|_| ())
                .ok_or_else(|| format!("refers to chemical {id}, which isn't in the pack"))
        };
        let input = |id: InputId| {
            data.brain_input(id)
                .map(|input| input.name.as_str())
                .ok_or_else(|| format!("refers to brain input {}, which isn't in the pack", id.0))
        };
        let level = |field: &str, value: f32| {
            if (0.0..=1.0).contains(&value) {
                Ok(())
            } else {
                Err(format!("`{field}` is {value}, but must be from 0 to 1"))
            }
        };
        let at_least_0 = |field: &str, value: f32| {
            if value >= 0.0 && value.is_finite() {
                Ok(())
            } else {
                Err(format!(
                    "`{field}` is {value}, but must be a number, at least 0"
                ))
            }
        };
        let finite = |field: &str, value: f32| {
            if value.is_finite() {
                Ok(())
            } else {
                Err(format!("`{field}` is {value}, but must be a number"))
            }
        };
        match *self {
            Gene::HalfLife { chem, ticks } => {
                chemical(chem)?;
                if ticks == 0 {
                    return Err("`ticks` is 0, but must be at least 1".into());
                }
            }
            Gene::Reaction {
                ref reactants,
                ref products,
                rate,
            } => {
                if !(1..=2).contains(&reactants.len()) {
                    return Err(format!(
                        "has {} reactants, but needs one or two",
                        reactants.len()
                    ));
                }
                if products.len() > 2 {
                    return Err(format!(
                        "has {} products, but may have at most two",
                        products.len()
                    ));
                }
                for term in reactants.iter().chain(products) {
                    chemical(term.chem)?;
                    if term.coefficient == 0 {
                        return Err("has a coefficient of 0, but each must be at least 1".into());
                    }
                }
                level("rate", rate)?;
            }
            Gene::Emitter {
                locus,
                mode,
                invert,
                threshold,
                gain,
                chem,
            } => {
                if invert && mode != EmitterMode::Level {
                    return Err(
                        "has `invert: true`, but only a Level emitter can be inverted".into(),
                    );
                }
                match locus {
                    LocusRef::Chem(id) => chemical(id)?,
                    LocusRef::Locus(id) => {
                        data.locus(id).ok_or_else(|| {
                            format!("refers to locus {id}, which isn't in the pack")
                        })?;
                    }
                }
                at_least_0("threshold", threshold)?;
                finite("gain", gain)?;
                chemical(chem)?;
            }
            Gene::Receptor {
                chem,
                threshold,
                gain,
                target,
            } => {
                chemical(chem)?;
                at_least_0("threshold", threshold)?;
                finite("gain", gain)?;
                data.locus(target)
                    .ok_or_else(|| format!("refers to locus {target}, which isn't in the pack"))?;
            }
            Gene::InitialConcentration { chem, value } => {
                chemical(chem)?;
                level("value", value)?;
            }
            Gene::Trait { value, .. } | Gene::BrainParam { value, .. } => {
                finite("value", value)?;
            }
            Gene::Instinct {
                ref inputs,
                verb,
                weight,
            } => {
                if !(1..=3).contains(&inputs.len()) {
                    return Err(format!(
                        "combines {} inputs, but an instinct combines one to three",
                        inputs.len()
                    ));
                }
                for (index, &(id, _)) in inputs.iter().enumerate() {
                    let named = input(id)?;
                    if inputs[..index].iter().any(|&(earlier, _)| earlier == id) {
                        return Err(format!("names `{named}` twice, but its inputs must differ"));
                    }
                }
                if verb.is_reserved() {
                    return Err(format!(
                        "leads to {verb:?}, a verb reserved for a later milestone"
                    ));
                }
                finite("weight", weight)?;
            }
            Gene::AttentionInstinct {
                input: id,
                category,
                weight,
            } => {
                input(id)?;
                data.category(category).ok_or_else(|| {
                    format!("refers to category {}, which isn't in the pack", category.0)
                })?;
                finite("weight", weight)?;
            }
            Gene::Unknown { .. } | Gene::Unmatched(_) => {}
        }
        Ok(())
    }

    /// The gene as it's written in a genome file.
    fn to_ron(&self, data: &DataPack) -> String {
        let chem = |id: ChemId| &data.chemical(id).expect("a checked gene").name;
        let locus = |id: LocusId| &data.locus(id).expect("a checked gene").name;
        let input = |id: InputId| &data.brain_input(id).expect("a checked gene").name;
        let terms = |terms: &[Term]| {
            let written: Vec<String> = terms
                .iter()
                .map(|t| format!("({:?}, {})", chem(t.chem), t.coefficient))
                .collect();
            format!("[{}]", written.join(", "))
        };
        match *self {
            Gene::HalfLife { chem: id, ticks } => {
                format!("HalfLife(chem: {:?}, ticks: {ticks})", chem(id))
            }
            Gene::Reaction {
                ref reactants,
                ref products,
                rate,
            } => format!(
                "Reaction(reactants: {}, products: {}, rate: {rate:?})",
                terms(reactants),
                terms(products)
            ),
            Gene::Emitter {
                locus: read,
                mode,
                invert,
                threshold,
                gain,
                chem: id,
            } => {
                let read = match read {
                    LocusRef::Chem(id) => format!("Chem({:?})", chem(id)),
                    LocusRef::Locus(id) => format!("Locus({:?})", locus(id)),
                };
                let mut text = format!("Emitter(locus: {read}, mode: {mode:?}, ");
                if invert {
                    text += "invert: true, ";
                }
                if threshold != 0.0 {
                    let _ = write!(text, "threshold: {threshold:?}, ");
                }
                text + &format!("gain: {gain:?}, chem: {:?})", chem(id))
            }
            Gene::Receptor {
                chem: id,
                threshold,
                gain,
                target,
            } => {
                let mut text = format!("Receptor(chem: {:?}, ", chem(id));
                if threshold != 0.0 {
                    let _ = write!(text, "threshold: {threshold:?}, ");
                }
                text + &format!("gain: {gain:?}, target: {:?})", locus(target))
            }
            Gene::InitialConcentration { chem: id, value } => {
                format!(
                    "InitialConcentration(chem: {:?}, value: {value:?})",
                    chem(id)
                )
            }
            Gene::Trait { which, value } => {
                format!("Trait(trait: {:?}, value: {value:?})", which.name())
            }
            Gene::BrainParam { param, value } => {
                format!("BrainParam(param: {:?}, value: {value:?})", param.name())
            }
            Gene::Instinct {
                ref inputs,
                verb,
                weight,
            } => {
                let written: Vec<String> = inputs
                    .iter()
                    .map(|&(id, negated)| format!("({:?}, {negated})", input(id)))
                    .collect();
                format!(
                    "Instinct(inputs: [{}], verb: {verb:?}, weight: {weight:?})",
                    written.join(", ")
                )
            }
            Gene::AttentionInstinct {
                input: id,
                category,
                weight,
            } => format!(
                "AttentionInstinct(input: {:?}, category: {:?}, weight: {weight:?})",
                input(id),
                data.category(category).expect("a checked gene").name
            ),
            Gene::Unknown {
                type_id,
                version,
                ref payload,
            }
            | Gene::Unmatched(AsWritten::ByNumber {
                type_id,
                version,
                ref payload,
            }) => {
                let hex: String = payload.iter().map(|byte| format!("{byte:02x}")).collect();
                format!("Gene(type: {type_id}, version: {version}, payload: {hex:?})")
            }
            Gene::Unmatched(AsWritten::AttentionInstinct {
                ref input,
                ref category,
                weight,
            }) => format!(
                "AttentionInstinct(input: {input:?}, category: {category:?}, weight: {weight:?})"
            ),
            Gene::Unmatched(AsWritten::Instinct {
                ref inputs,
                verb,
                weight,
            }) => {
                let written: Vec<String> = inputs
                    .iter()
                    .map(|(name, negated)| format!("({name:?}, {negated})"))
                    .collect();
                format!(
                    "Instinct(inputs: [{}], verb: {verb:?}, weight: {weight:?})",
                    written.join(", ")
                )
            }
        }
    }
}

/// A genome file, before its names are resolved.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GenomeFile {
    format: u32,
    genes: Vec<GeneEntry>,
}

/// One gene as a genome file writes it, before its names are resolved.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum GeneEntry {
    HalfLife {
        chem: String,
        ticks: u32,
    },
    Reaction {
        reactants: Vec<(String, u8)>,
        products: Vec<(String, u8)>,
        rate: f32,
    },
    Emitter {
        locus: LocusEntry,
        mode: EmitterMode,
        #[serde(default)]
        invert: bool,
        #[serde(default)]
        threshold: f32,
        gain: f32,
        chem: String,
    },
    Receptor {
        chem: String,
        #[serde(default)]
        threshold: f32,
        gain: f32,
        target: String,
    },
    InitialConcentration {
        chem: String,
        value: f32,
    },
    Trait {
        #[serde(rename = "trait")]
        which: String,
        value: f32,
    },
    BrainParam {
        param: String,
        value: f32,
    },
    Instinct {
        inputs: Vec<(String, bool)>,
        verb: Verb,
        weight: f32,
    },
    AttentionInstinct {
        input: String,
        category: String,
        weight: f32,
    },
    /// Any gene, by number: its type ID, payload version and payload bytes in hex.
    Gene {
        #[serde(rename = "type")]
        type_id: u16,
        version: u8,
        payload: String,
    },
}

#[derive(Deserialize)]
enum LocusEntry {
    Chem(String),
    Locus(String),
}

impl GeneEntry {
    /// The gene type's name, for messages.
    fn type_name(&self) -> &'static str {
        match self {
            GeneEntry::HalfLife { .. } => "HalfLife",
            GeneEntry::Reaction { .. } => "Reaction",
            GeneEntry::Emitter { .. } => "Emitter",
            GeneEntry::Receptor { .. } => "Receptor",
            GeneEntry::InitialConcentration { .. } => "InitialConcentration",
            GeneEntry::Trait { .. } => "Trait",
            GeneEntry::BrainParam { .. } => "BrainParam",
            GeneEntry::Instinct { .. } => "Instinct",
            GeneEntry::AttentionInstinct { .. } => "AttentionInstinct",
            GeneEntry::Gene { .. } => "Gene",
        }
    }

    /// The gene with its names resolved to IDs, or what's wrong with it.
    fn resolve(self, data: &DataPack) -> Result<Gene, String> {
        let chem = |name: &str| {
            data.chemical_named(name)
                .map(|c| c.id)
                .ok_or_else(|| format!("names the unknown chemical `{name}`"))
        };
        let locus = |name: &str| {
            data.locus_named(name)
                .map(|l| l.id)
                .ok_or_else(|| format!("names the unknown locus `{name}`"))
        };
        let input = |name: &str| {
            data.brain_input_named(name)
                .map(|input| input.id)
                .ok_or_else(|| format!("names the unknown brain input `{name}`"))
        };
        let terms = |terms: Vec<(String, u8)>| {
            terms
                .into_iter()
                .map(|(name, coefficient)| {
                    Ok(Term {
                        chem: chem(&name)?,
                        coefficient,
                    })
                })
                .collect::<Result<Vec<Term>, String>>()
        };
        Ok(match self {
            GeneEntry::HalfLife { chem: name, ticks } => Gene::HalfLife {
                chem: chem(&name)?,
                ticks,
            },
            GeneEntry::Reaction {
                reactants,
                products,
                rate,
            } => Gene::Reaction {
                reactants: terms(reactants)?,
                products: terms(products)?,
                rate,
            },
            GeneEntry::Emitter {
                locus: read,
                mode,
                invert,
                threshold,
                gain,
                chem: name,
            } => Gene::Emitter {
                locus: match read {
                    LocusEntry::Chem(name) => LocusRef::Chem(chem(&name)?),
                    LocusEntry::Locus(name) => LocusRef::Locus(locus(&name)?),
                },
                mode,
                invert,
                threshold,
                gain,
                chem: chem(&name)?,
            },
            GeneEntry::Receptor {
                chem: name,
                threshold,
                gain,
                target,
            } => Gene::Receptor {
                chem: chem(&name)?,
                threshold,
                gain,
                target: locus(&target)?,
            },
            GeneEntry::InitialConcentration { chem: name, value } => Gene::InitialConcentration {
                chem: chem(&name)?,
                value,
            },
            GeneEntry::Trait { which, value } => Gene::Trait {
                which: Trait::named(&which)
                    .ok_or_else(|| format!("names the unknown trait `{which}`"))?,
                value,
            },
            GeneEntry::BrainParam { param, value } => Gene::BrainParam {
                param: BrainParam::named(&param)
                    .ok_or_else(|| format!("names the unknown brain parameter `{param}`"))?,
                value,
            },
            GeneEntry::Instinct {
                inputs,
                verb,
                weight,
            } => {
                // Any unknown input but a missing category's is an error.
                let ids = inputs
                    .iter()
                    .filter(|(name, _)| !missing_named(name, data))
                    .map(|(name, negated)| Ok((input(name)?, *negated)))
                    .collect::<Result<_, String>>()?;
                if inputs.iter().any(|(name, _)| missing_named(name, data)) {
                    Gene::Unmatched(AsWritten::Instinct {
                        inputs,
                        verb,
                        weight,
                    })
                } else {
                    Gene::Instinct {
                        inputs: ids,
                        verb,
                        weight,
                    }
                }
            }
            GeneEntry::AttentionInstinct {
                input: name,
                category,
                weight,
            } => {
                let id = input(&name)?;
                match data.category_named(&category) {
                    Some(category) => Gene::AttentionInstinct {
                        input: id,
                        category,
                        weight,
                    },
                    None => Gene::Unmatched(AsWritten::AttentionInstinct {
                        input: name,
                        category,
                        weight,
                    }),
                }
            }
            GeneEntry::Gene {
                type_id,
                version,
                payload,
            } => {
                let payload = hex(&payload)?;
                let gene = decode(type_id, version, payload.clone())?;
                if names_a_missing_category(&gene, data) {
                    Gene::Unmatched(AsWritten::ByNumber {
                        type_id,
                        version,
                        payload,
                    })
                } else {
                    gene
                }
            }
        })
    }
}

/// Whether `gene`, read by number, names a category the pack doesn't have
/// (design v19 §5.7): an attention instinct's category, or an instinct's
/// attended input.
fn names_a_missing_category(gene: &Gene, data: &DataPack) -> bool {
    match gene {
        Gene::AttentionInstinct { category, .. } => data.category(*category).is_none(),
        Gene::Instinct { inputs, .. } => inputs.iter().any(|&(id, _)| {
            // A Target input the pack lacks is a missing category's.
            data.brain_input(id).is_none() && TARGET_IDS.contains(&id.0)
        }),
        _ => false,
    }
}

/// Whether the brain input called `name` is a missing category's: an
/// attended input the pack lacks (design v19 §5.7).
fn missing_named(name: &str, data: &DataPack) -> bool {
    data.brain_input_named(name).is_none() && name.starts_with(ATTENDED)
}

/// The bytes a payload's hex digits spell.
fn hex(digits: &str) -> Result<Vec<u8>, String> {
    let bad = || format!("has the payload {digits:?}, which isn't pairs of hex digits");
    // Not `from_str_radix` alone: it takes a leading `+`.
    if !digits.len().is_multiple_of(2) || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad());
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&digits[i..i + 2], 16).map_err(|_| bad()))
        .collect()
}

/// A gene from its type ID, payload version and payload. A type this build
/// doesn't know, or a version newer than it knows, is kept as it is.
///
/// A payload is MessagePack: the gene's fields in order, as an array, with
/// every reference as a stable ID, an emitter's locus as `[0, chemical ID]` or
/// `[1, locus ID]`, and its mode as 0 (Level), 1 (Rise) or 2 (Fall).
fn decode(type_id: u16, version: u8, payload: Vec<u8>) -> Result<Gene, String> {
    if version == 0 {
        return Err("has version 0, but versions start at 1".into());
    }
    if version > VERSION {
        return Ok(Gene::Unknown {
            type_id,
            version,
            payload,
        });
    }
    fn read<T: for<'de> Deserialize<'de>>(payload: &[u8]) -> Result<T, String> {
        rmp_serde::from_slice(payload)
            .map_err(|e| format!("has a payload that can't be read for its type: {e}"))
    }
    let terms = |terms: Vec<(ChemId, u8)>| {
        terms
            .into_iter()
            .map(|(chem, coefficient)| Term { chem, coefficient })
            .collect()
    };
    Ok(match type_id {
        1 => {
            let (chem, ticks) = read(&payload)?;
            Gene::HalfLife { chem, ticks }
        }
        2 => {
            let (reactants, products, rate) = read(&payload)?;
            Gene::Reaction {
                reactants: terms(reactants),
                products: terms(products),
                rate,
            }
        }
        3 => {
            let ((kind, id), mode, invert, threshold, gain, chem): (
                (u8, u16),
                u8,
                bool,
                f32,
                f32,
                ChemId,
            ) = read(&payload)?;
            let locus = match kind {
                0 => LocusRef::Chem(ChemId(id)),
                1 => LocusRef::Locus(LocusId(id)),
                _ => {
                    return Err(format!(
                        "has a payload with the locus kind {kind}, which isn't 0 or 1"
                    ));
                }
            };
            let mode = match mode {
                0 => EmitterMode::Level,
                1 => EmitterMode::Rise,
                2 => EmitterMode::Fall,
                _ => {
                    return Err(format!(
                        "has a payload with the mode {mode}, which isn't 0, 1 or 2"
                    ));
                }
            };
            Gene::Emitter {
                locus,
                mode,
                invert,
                threshold,
                gain,
                chem,
            }
        }
        4 => {
            let (chem, threshold, gain, target) = read(&payload)?;
            Gene::Receptor {
                chem,
                threshold,
                gain,
                target,
            }
        }
        5 => {
            let (chem, value) = read(&payload)?;
            Gene::InitialConcentration { chem, value }
        }
        6 => {
            let (id, value): (u16, f32) = read(&payload)?;
            let which = Trait::ALL
                .into_iter()
                .find(|&t| t as u16 == id)
                .ok_or_else(|| format!("refers to trait {id}, which doesn't exist"))?;
            Gene::Trait { which, value }
        }
        7 => {
            let (id, value): (u16, f32) = read(&payload)?;
            let param = BrainParam::ALL
                .into_iter()
                .find(|&p| p as u16 == id)
                .ok_or_else(|| format!("refers to brain parameter {id}, which doesn't exist"))?;
            Gene::BrainParam { param, value }
        }
        8 => {
            let (inputs, verb, weight): (Vec<(InputId, bool)>, u16, f32) = read(&payload)?;
            let verb = Verb::ALL
                .into_iter()
                .find(|&v| v as u16 == verb)
                .ok_or_else(|| format!("refers to verb {verb}, which doesn't exist"))?;
            Gene::Instinct {
                inputs,
                verb,
                weight,
            }
        }
        9 => {
            let (input, category, weight): (InputId, u16, f32) = read(&payload)?;
            let category = CategoryId(category);
            Gene::AttentionInstinct {
                input,
                category,
                weight,
            }
        }
        _ => Gene::Unknown {
            type_id,
            version,
            payload,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checking_an_attention_instinct_checks_its_category_is_in_the_pack() {
        // As every other reference is checked. A gene read from a genome file
        // never fails this: a missing category makes it unmatched instead
        // (design v19 §5.7).
        let data = DataPack::builtin().expect("built-in data pack is valid");
        let gene = |category| Gene::AttentionInstinct {
            input: InputId(1),
            category: CategoryId(category),
            weight: 0.5,
        };
        assert_eq!(gene(6).check(&data), Ok(()));
        let error = gene(26).check(&data).expect_err("no category 26");
        assert!(error.contains("category 26"), "{error}");
    }
}
