//! The brain's I/O registry (design §5.2, Appendix A): every brain input, with
//! its stable ID. The State inputs come from `brain_io.ron`; the Target inputs
//! are one for each category (design v19 §3.5.5), then two fixed here, because
//! attention gives them their meaning.

use serde::{Deserialize, Serialize};

use crate::categories::{ATTENDED, Category, attended_input};
use crate::data::{DataError, check_unique};
use crate::genome::LocusRef;
use crate::registry::{CategoryId, Chemical, ChemicalKind, Locus, LocusKind};

/// The brain I/O registry, relative to the pack root.
pub(crate) const BRAIN_IO: &str = "brain_io.ron";

/// A brain input's stable ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct InputId(pub(crate) u16);

/// The IDs kept for Target inputs. State inputs take any other ID from 1.
pub(crate) const TARGET_IDS: std::ops::RangeInclusive<u16> = 36..=63;

/// What a brain input reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// A State input: a drive, a hormone, a body sensor or a pulse.
    State(LocusRef),
    /// 1 while attention is on this category, else 0.
    Attended(CategoryId),
    /// The attended target's path cost, over the flood's reach.
    TargetDistance,
    /// 1 while the sprite stands on one of its target's goal tiles, else 0.
    TargetAdjacent,
}

/// One brain input.
#[derive(Debug, Clone)]
pub(crate) struct BrainInput {
    pub(crate) id: InputId,
    pub(crate) name: String,
    pub(crate) source: Source,
}

/// `brain_io.ron`: the State inputs, and which drives are needs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BrainIoFile {
    inputs: Vec<InputEntry>,
    needs: Vec<String>,
}

/// One entry of `brain_io.ron`'s inputs.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputEntry {
    id: u16,
    name: String,
    reads: ReadsEntry,
}

/// What an entry reads, by name.
#[derive(Debug, Clone, Deserialize)]
enum ReadsEntry {
    Chem(String),
    Locus(String),
}

/// The Target inputs as `(id, name, source)`: one for each of the
/// `categories`, then the two fixed in code (design v19 §3.5.5).
fn target_inputs(categories: &[Category]) -> impl Iterator<Item = (u16, String, Source)> {
    categories
        .iter()
        .map(|c| {
            (
                attended_input(c.id),
                format!("{ATTENDED}{}", c.name),
                Source::Attended(c.id),
            )
        })
        .chain([
            (42, "target_distance".into(), Source::TargetDistance),
            (43, "target_adjacent".into(), Source::TargetAdjacent),
        ])
}

/// Every brain input, in ID order, and the needs, as places in that order
/// (design §5.2). A need that isn't a State input reading a drive, or is
/// named twice, is an error.
pub(crate) fn brain_io(
    file: BrainIoFile,
    categories: &[Category],
    chemicals: &[Chemical],
    loci: &[Locus],
) -> Result<(Vec<BrainInput>, Vec<usize>), DataError> {
    let inputs = brain_inputs(file.inputs, categories, chemicals, loci)?;
    let mut needs: Vec<usize> = Vec::new();
    for name in &file.needs {
        let invalid = |problem: &str| DataError::Invalid {
            file: BRAIN_IO.into(),
            message: format!("the need `{name}` {problem}"),
        };
        let place = inputs
            .iter()
            .position(|input| &input.name == name)
            .ok_or_else(|| invalid("isn't a brain input"))?;
        let reads_a_drive = match inputs[place].source {
            Source::State(LocusRef::Chem(id)) => chemicals
                .iter()
                .any(|c| c.id == id && c.kind() == ChemicalKind::Drive),
            _ => false,
        };
        if !reads_a_drive {
            return Err(invalid("doesn't read a drive"));
        }
        if needs.contains(&place) {
            return Err(invalid("is named twice"));
        }
        needs.push(place);
    }
    Ok((inputs, needs))
}

/// Every brain input, in ID order: the State inputs `entries` list, and the
/// Target inputs. An entry that reads anything but a drive, a hormone, a
/// body sensor or a pulse, takes an ID kept for Target inputs, or clashes
/// with another input, is an error.
fn brain_inputs(
    entries: Vec<InputEntry>,
    categories: &[Category],
    chemicals: &[Chemical],
    loci: &[Locus],
) -> Result<Vec<BrainInput>, DataError> {
    let invalid = |message: String| DataError::Invalid {
        file: BRAIN_IO.into(),
        message,
    };
    let mut inputs = Vec::new();
    for entry in entries {
        let source = match &entry.reads {
            ReadsEntry::Chem(name) => {
                let chemical = chemicals.iter().find(|c| &c.name == name).ok_or_else(|| {
                    invalid(format!(
                        "`{}` reads the unknown chemical `{name}`",
                        entry.name
                    ))
                })?;
                if !matches!(chemical.kind(), ChemicalKind::Drive | ChemicalKind::Hormone) {
                    return Err(invalid(format!(
                        "`{}` reads `{name}`, but the brain feels only drives and hormones among chemicals",
                        entry.name
                    )));
                }
                LocusRef::Chem(chemical.id)
            }
            ReadsEntry::Locus(name) => {
                let locus = loci.iter().find(|l| &l.name == name).ok_or_else(|| {
                    invalid(format!("`{}` reads the unknown locus `{name}`", entry.name))
                })?;
                if !matches!(locus.kind, LocusKind::BodySensor | LocusKind::Pulse) {
                    return Err(invalid(format!(
                        "`{}` reads `{name}`, but the brain feels only body sensors and pulses among loci",
                        entry.name
                    )));
                }
                LocusRef::Locus(locus.id)
            }
        };
        if entry.id == 0 || TARGET_IDS.contains(&entry.id) {
            return Err(invalid(format!(
                "`{}` has the id {}, but State inputs are numbered from 1, and {} to {} are kept for Target inputs",
                entry.name,
                entry.id,
                TARGET_IDS.start(),
                TARGET_IDS.end()
            )));
        }
        inputs.push(BrainInput {
            id: InputId(entry.id),
            name: entry.name,
            source: Source::State(source),
        });
    }
    inputs.extend(
        target_inputs(categories).map(|(id, name, source)| BrainInput {
            id: InputId(id),
            name,
            source,
        }),
    );
    inputs.sort_by_key(|input| input.id);
    check_unique(
        BRAIN_IO,
        inputs.iter().map(|input| (input.id.0, input.name.as_str())),
    )?;
    Ok(inputs)
}
