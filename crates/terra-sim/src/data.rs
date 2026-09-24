use std::collections::{BTreeMap, BTreeSet};

use ron::extensions::Extensions;
use serde::Deserialize;

use crate::object_types::{OBJECTS, ObjectType, TypeEntry, object_types};
use crate::registry::{Chemical, Locus};
use crate::terrain::{Terrain, TerrainProps};

/// A validated data pack: everything a world needs from `data/`.
#[derive(Debug, Clone)]
pub struct DataPack {
    manifest: Manifest,
    /// Indexed by `Terrain as usize`.
    terrain: Vec<TerrainProps>,
    #[expect(dead_code, reason = "biochemistry reads the registries from slice 4")]
    chemicals: Vec<Chemical>,
    #[expect(dead_code, reason = "biochemistry reads the registries from slice 4")]
    loci: Vec<Locus>,
    /// In ascending ID order.
    object_types: Vec<ObjectType>,
}

/// Why a data pack could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// A required file is absent from the pack.
    MissingFile(String),
    /// A file is not valid RON for its format.
    Parse { file: String, message: String },
    /// A file parses but breaks a rule of its format.
    Invalid { file: String, message: String },
}

/// The name of the pack manifest, relative to the pack root.
const MANIFEST: &str = "pack.ron";
/// The terrain properties file, relative to the pack root.
const TERRAIN: &str = "terrain.ron";
/// The chemicals registry, relative to the pack root.
const CHEMICALS: &str = "chemicals.ron";
/// The loci registry, relative to the pack root.
const LOCI: &str = "loci.ron";

/// The default pack's files, embedded at compile time from the repository's `data/`.
const BUILTIN: &[(&str, &str)] = &[
    (MANIFEST, include_str!("../../../data/pack.ron")),
    (TERRAIN, include_str!("../../../data/terrain.ron")),
    (CHEMICALS, include_str!("../../../data/chemicals.ron")),
    (LOCI, include_str!("../../../data/loci.ron")),
    (OBJECTS, include_str!("../../../data/objects.ron")),
];

/// `pack.ron`: identifies the pack.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    name: String,
    version: String,
}

impl Manifest {
    fn validate(&self) -> Result<(), DataError> {
        for (field, value) in [("name", &self.name), ("version", &self.version)] {
            if value.trim().is_empty() {
                return Err(DataError::Invalid {
                    file: MANIFEST.into(),
                    message: format!("`{field}` must not be empty"),
                });
            }
        }
        Ok(())
    }
}

/// One entry of `terrain.ron`, before validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TerrainEntry {
    walkable: bool,
    #[serde(default)]
    step_cost: Option<u16>,
    #[serde(default)]
    fertility: Option<f32>,
    #[serde(default)]
    drinkable: Option<bool>,
    #[serde(default)]
    allows_fixtures: Option<bool>,
}

/// Checks `terrain.ron` and returns each terrain's properties, indexed by `Terrain as usize`.
fn terrain_table(
    mut entries: BTreeMap<Terrain, TerrainEntry>,
) -> Result<Vec<TerrainProps>, DataError> {
    Terrain::ALL
        .iter()
        .map(|&terrain| {
            let name = terrain_name(terrain);
            let invalid = |problem: &str| DataError::Invalid {
                file: TERRAIN.into(),
                message: format!("`{name}` {problem}"),
            };
            let entry = entries
                .remove(&terrain)
                .ok_or_else(|| invalid("is missing"))?;
            entry.into_props(terrain).map_err(invalid)
        })
        .collect()
}

impl TerrainEntry {
    /// The validated properties, or what is wrong with the entry.
    fn into_props(self, terrain: Terrain) -> Result<TerrainProps, &'static str> {
        if !self.walkable {
            // Carving turns unwalkable tiles into these two, so they must be walkable
            // for a generated map to be connected (design §3.2).
            if matches!(terrain, Terrain::Dirt | Terrain::ShallowWater) {
                return Err("must be walkable, because carving creates it");
            }
            if self.step_cost.is_some()
                || self.fertility.is_some()
                || self.drinkable.is_some()
                || self.allows_fixtures.is_some()
            {
                return Err("is unwalkable, so it takes nothing but `walkable: false`");
            }
            return Ok(TerrainProps {
                step_cost: None,
                fertility: 0.0,
                drinkable: false,
                allows_fixtures: false,
            });
        }
        let step_cost = match self.step_cost {
            Some(cost) if cost > 0 => cost,
            _ => return Err("is walkable, so it needs a `step_cost` above 0"),
        };
        let fertility = match self.fertility {
            Some(fertility) if (0.0..=1.0).contains(&fertility) => fertility,
            _ => return Err("is walkable, so it needs a `fertility` from 0 to 1"),
        };
        let Some(drinkable) = self.drinkable else {
            return Err("is walkable, so it must say whether it is `drinkable`");
        };
        let Some(allows_fixtures) = self.allows_fixtures else {
            return Err("is walkable, so it must say whether it `allows_fixtures`");
        };
        Ok(TerrainProps {
            step_cost: Some(step_cost),
            fertility,
            drinkable,
            allows_fixtures,
        })
    }
}

/// A terrain's name as written in `terrain.ron`.
fn terrain_name(terrain: Terrain) -> String {
    ron::to_string(&terrain).expect("a unit variant always serializes")
}

impl DataPack {
    /// The default data pack embedded in the binary.
    pub fn builtin() -> Result<DataPack, DataError> {
        DataPack::from_sources(BUILTIN)
    }

    /// Builds a pack from already-read files, given as `(path within the pack, RON text)`.
    pub fn from_sources(sources: &[(&str, &str)]) -> Result<DataPack, DataError> {
        let manifest = parse::<Manifest>(sources, MANIFEST)?;
        manifest.validate()?;
        let terrain = terrain_table(parse(sources, TERRAIN)?)?;
        let chemicals: Vec<Chemical> = parse(sources, CHEMICALS)?;
        check_unique(CHEMICALS, chemicals.iter().map(|c| (c.id, c.name.as_str())))?;
        let loci: Vec<Locus> = parse(sources, LOCI)?;
        check_unique(LOCI, loci.iter().map(|l| (l.id, l.name.as_str())))?;
        let object_types = object_types(
            parse::<Vec<TypeEntry>>(sources, OBJECTS)?,
            &chemicals,
            &loci,
        )?;
        Ok(DataPack {
            manifest,
            terrain,
            chemicals,
            loci,
            object_types,
        })
    }

    /// The pack's name, from its manifest.
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// The pack's version, from its manifest.
    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    /// A terrain's properties, from `terrain.ron`.
    pub fn terrain(&self, terrain: Terrain) -> &TerrainProps {
        &self.terrain[terrain as usize]
    }

    /// The names of the object types that can have objects (not pseudo types), in ID order.
    pub fn object_type_names(&self) -> impl Iterator<Item = &str> {
        self.object_types
            .iter()
            .filter(|t| !t.pseudo)
            .map(|t| t.name.as_str())
    }

    /// The names of an object type's stages, in order. Empty for a type with
    /// no stages, or no such type.
    pub fn stage_names(&self, object_type: &str) -> Vec<&str> {
        self.named(object_type)
            .map(|t| t.stages.iter().map(|s| s.name.as_str()).collect())
            .unwrap_or_default()
    }

    /// The names of an object type's counters.
    pub fn counter_names(&self, object_type: &str) -> Vec<&str> {
        self.named(object_type)
            .map(|t| t.counters.iter().map(|c| c.name.as_str()).collect())
            .unwrap_or_default()
    }

    /// Every visual state an object type can be in: those its visual rules name,
    /// in order, then `"default"`.
    pub fn visual_states(&self, object_type: &str) -> Vec<&str> {
        let Some(named) = self.named(object_type) else {
            return Vec::new();
        };
        let mut states: Vec<&str> = Vec::new();
        for state in named
            .visual
            .iter()
            .map(|v| v.state.as_str())
            .chain(["default"])
        {
            if !states.contains(&state) {
                states.push(state);
            }
        }
        states
    }

    fn named(&self, object_type: &str) -> Option<&ObjectType> {
        self.object_types.iter().find(|t| t.name == object_type)
    }

    /// Every object type, in ascending ID order. Rules refer to types by their index here.
    pub(crate) fn object_types(&self) -> &[ObjectType] {
        &self.object_types
    }

    /// The index of the object type called `name`.
    pub(crate) fn object_type_named(&self, name: &str) -> Option<usize> {
        self.object_types.iter().position(|t| t.name == name)
    }
}

/// Checks that no two registry entries in `file` share an ID or a name.
pub(crate) fn check_unique<'a>(
    file: &str,
    entries: impl Iterator<Item = (u16, &'a str)>,
) -> Result<(), DataError> {
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for (id, name) in entries {
        let duplicate = if !ids.insert(id) {
            format!("the id {id}")
        } else if !names.insert(name) {
            format!("the name `{name}`")
        } else {
            continue;
        };
        return Err(DataError::Invalid {
            file: file.into(),
            message: format!("{duplicate} is used more than once"),
        });
    }
    Ok(())
}

/// Finds `file` among `sources` and parses it as `T`.
fn parse<T: for<'de> Deserialize<'de>>(
    sources: &[(&str, &str)],
    file: &str,
) -> Result<T, DataError> {
    let (_, text) = sources
        .iter()
        .find(|(path, _)| *path == file)
        .ok_or_else(|| DataError::MissingFile(file.into()))?;
    // `implicit_some` lets optional fields be written as plain values: `step_cost: 10`.
    ron::Options::default()
        .with_default_extension(Extensions::IMPLICIT_SOME)
        .from_str(text)
        .map_err(|e| DataError::Parse {
            file: file.into(),
            message: e.to_string(),
        })
}
