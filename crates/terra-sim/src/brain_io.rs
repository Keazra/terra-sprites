//! The brain's I/O registry (design §5.2, Appendix A): every brain input, with
//! its stable ID. The State inputs come from `brain_io.ron`; the Target inputs
//! are fixed here, after them, because attention gives them their meaning.

use serde::{Deserialize, Serialize};

use crate::data::{DataError, check_unique};
use crate::genome::LocusRef;
use crate::registry::{Category, Chemical, ChemicalKind, Locus, LocusKind};

/// The brain I/O registry, relative to the pack root.
pub(crate) const BRAIN_IO: &str = "brain_io.ron";

/// A brain input's stable ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct InputId(pub(crate) u16);

/// The first Target input's ID. State inputs are numbered below it.
const FIRST_TARGET: u16 = 36;

/// What a brain input reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// A State input: a drive, a hormone, a body sensor or a pulse.
    State(LocusRef),
    /// 1 while attention is on this category, else 0.
    Attended(Category),
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

/// One entry of `brain_io.ron`.
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

/// The Target inputs, fixed in code, in ID order from `FIRST_TARGET`.
fn target_inputs() -> impl Iterator<Item = (String, Source)> {
    Category::ALL
        .into_iter()
        .map(|c| (format!("attended_{}", c.name()), Source::Attended(c)))
        .chain([
            ("target_distance".into(), Source::TargetDistance),
            ("target_adjacent".into(), Source::TargetAdjacent),
        ])
}

/// Every brain input, in ID order: the State inputs `entries` list, then
/// the Target inputs. An entry that reads anything but a drive, a hormone, a
/// body sensor or a pulse, or clashes with another input, is an error.
pub(crate) fn brain_inputs(
    entries: Vec<InputEntry>,
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
        if entry.id == 0 || entry.id >= FIRST_TARGET {
            return Err(invalid(format!(
                "`{}` has the id {}, but State inputs are numbered 1 to {}",
                entry.name,
                entry.id,
                FIRST_TARGET - 1
            )));
        }
        inputs.push(BrainInput {
            id: InputId(entry.id),
            name: entry.name,
            source: Source::State(source),
        });
    }
    inputs.sort_by_key(|input| input.id);
    inputs.extend(
        target_inputs()
            .zip(FIRST_TARGET..)
            .map(|((name, source), id)| BrainInput {
                id: InputId(id),
                name,
                source,
            }),
    );
    check_unique(
        BRAIN_IO,
        inputs.iter().map(|input| (input.id.0, input.name.as_str())),
    )?;
    Ok(inputs)
}
