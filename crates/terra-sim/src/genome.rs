//! Genomes (design §2.8, §4.3): ordered genes with stable type IDs and
//! versioned payloads, and the genome file format. A gene this build can't
//! read is kept exactly as it is.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::data::DataPack;
use crate::registry::Trait;

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
    HalfLife { chem: u16, ticks: u32 },
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
        chem: u16,
    },
    /// Type 4: a chemical's level past `threshold` moves a receptor target by `gain` of it.
    Receptor {
        chem: u16,
        threshold: f32,
        gain: f32,
        target: u16,
    },
    /// Type 5: a chemical's level at birth.
    InitialConcentration { chem: u16, value: f32 },
    /// Type 6: a body trait.
    Trait { which: Trait, value: f32 },
    /// A gene this build can't read: an unknown type, or a payload version
    /// newer than it knows. Kept exactly as it is.
    Unknown {
        type_id: u16,
        version: u8,
        payload: Vec<u8>,
    },
}

/// A chemical and its coefficient in a reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Term {
    pub(crate) chem: u16,
    pub(crate) coefficient: u8,
}

/// What an emitter reads: a chemical's level, or another locus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum LocusRef {
    Chem(u16),
    Locus(u16),
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
    pub(crate) fn view<'a>(&self, data: &'a DataPack) -> GeneView<'a> {
        let chem = |id: u16| data.chemical(id).expect("a checked gene").name.as_str();
        let locus = |id: u16| data.locus(id).expect("a checked gene").name.as_str();
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
            Gene::Unknown {
                type_id,
                version,
                ref payload,
            } => GeneView::Unknown {
                type_id,
                version,
                bytes: payload.len(),
            },
        }
    }

    /// Checks the gene's references and values, or says what's wrong.
    fn check(&self, data: &DataPack) -> Result<(), String> {
        let chemical = |id: u16| {
            data.chemical(id)
                .map(|_| ())
                .ok_or_else(|| format!("refers to chemical {id}, which isn't in the pack"))
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
            Gene::Trait { value, .. } => finite("value", value)?,
            Gene::Unknown { .. } => {}
        }
        Ok(())
    }

    /// The gene as it's written in a genome file.
    fn to_ron(&self, data: &DataPack) -> String {
        let chem = |id: u16| &data.chemical(id).expect("a checked gene").name;
        let locus = |id: u16| &data.locus(id).expect("a checked gene").name;
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
            Gene::Unknown {
                type_id,
                version,
                ref payload,
            } => {
                let hex: String = payload.iter().map(|byte| format!("{byte:02x}")).collect();
                format!("Gene(type: {type_id}, version: {version}, payload: {hex:?})")
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
            GeneEntry::Gene {
                type_id,
                version,
                payload,
            } => decode(type_id, version, hex(&payload)?)?,
        })
    }
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
    let terms = |terms: Vec<(u16, u8)>| {
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
                u16,
            ) = read(&payload)?;
            let locus = match kind {
                0 => LocusRef::Chem(id),
                1 => LocusRef::Locus(id),
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
        _ => Gene::Unknown {
            type_id,
            version,
            payload,
        },
    })
}
