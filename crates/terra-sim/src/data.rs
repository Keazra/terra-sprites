use serde::Deserialize;

/// A validated data pack: everything a world needs from `data/`.
#[derive(Debug, Clone)]
pub struct DataPack {
    manifest: Manifest,
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

/// The default pack's files, embedded at compile time from the repository's `data/`.
const BUILTIN: &[(&str, &str)] = &[(MANIFEST, include_str!("../../../data/pack.ron"))];

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

impl DataPack {
    /// The default data pack embedded in the binary.
    pub fn builtin() -> Result<DataPack, DataError> {
        DataPack::from_sources(BUILTIN)
    }

    /// Builds a pack from already-read files, given as `(path within the pack, RON text)`.
    pub fn from_sources(sources: &[(&str, &str)]) -> Result<DataPack, DataError> {
        let manifest = parse::<Manifest>(sources, MANIFEST)?;
        manifest.validate()?;
        Ok(DataPack { manifest })
    }

    /// The pack's name, from its manifest.
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// The pack's version, from its manifest.
    pub fn version(&self) -> &str {
        &self.manifest.version
    }
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
    ron::from_str(text).map_err(|e| DataError::Parse {
        file: file.into(),
        message: e.to_string(),
    })
}
