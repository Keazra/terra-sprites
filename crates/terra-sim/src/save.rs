//! Save files (design §2.8): the magic `TSPR`, then a header naming the
//! save's schema and the build that wrote it, then the world, each in
//! MessagePack with named fields.
//!
//! **Schema versions.** A change that only adds fields, each with a serde
//! default, keeps the schema. A breaking change bumps `SCHEMA_VERSION` and
//! adds a step to `upgrade`, so loading runs the chain one version at a time.
//! The step works on frozen copies of the old types: before changing a type
//! the save holds, copy it as it is into a module for the old schema (`v1`),
//! have the old schema's `Contents` hold the copies, and write
//! `migrate_v1_to_v2` from them to the live types. The live types then
//! change freely, and an old save still reads as it was written. Every
//! released schema keeps a golden save in the tests, which must still load.

use std::fmt;

use serde::de::{self, DeserializeOwned, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::config::WorldConfig;
use crate::data::DataError;
use crate::world::WorldState;

/// What every save starts with.
const MAGIC: &[u8; 4] = b"TSPR";

/// The schema this build writes, and the newest it reads.
pub const SCHEMA_VERSION: u32 = 1;

/// The build that wrote a save, for the message when a newer one is
/// refused.
const SIM_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Why a save couldn't be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// It isn't a Terra Sprites save.
    NotASave,
    /// A newer build wrote it, in a schema this one can't read.
    Newer {
        /// The save's schema.
        schema: u32,
        /// The version of the build that wrote it.
        sim_version: String,
    },
    /// It's damaged: what's wrong.
    Damaged(String),
    /// The data pack it embeds doesn't load in this build.
    Pack(DataError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::NotASave => f.write_str("it isn't a Terra Sprites save"),
            LoadError::Newer {
                schema,
                sim_version,
            } => write!(
                f,
                "a newer Terra Sprites ({sim_version}) saved it, in save format {schema}; \
                 this one reads formats up to {SCHEMA_VERSION}"
            ),
            LoadError::Damaged(what) => write!(f, "it's damaged: {what}"),
            LoadError::Pack(err) => write!(f, "its data pack doesn't load: {err:?}"),
        }
    }
}

/// What follows the magic. Its shape never changes, so any build can read
/// any save's header and say why it refuses it.
#[derive(Serialize, Deserialize)]
struct Header {
    schema_version: u32,
    sim_version: String,
}

/// The world as a save holds it, in the current schema.
#[derive(Serialize)]
pub(crate) struct Contents<'a> {
    pub(crate) seed: u64,
    pub(crate) config: Option<WorldConfig>,
    /// The data pack's files, as `(path within the pack, RON text)`.
    pub(crate) pack: Vec<(String, String)>,
    pub(crate) state: &'a WorldState,
}

/// The world as it's read back: `Contents`, owned.
#[derive(Deserialize)]
pub(crate) struct Loaded {
    pub(crate) seed: u64,
    pub(crate) config: Option<WorldConfig>,
    pub(crate) pack: Vec<(String, String)>,
    pub(crate) state: WorldState,
}

/// The save's bytes.
pub(crate) fn write(contents: &Contents) -> Vec<u8> {
    let header = Header {
        schema_version: SCHEMA_VERSION,
        sim_version: SIM_VERSION.into(),
    };
    let mut bytes = MAGIC.to_vec();
    rmp_serde::encode::write_named(&mut bytes, &header).expect("a header always serializes");
    rmp_serde::encode::write_named(&mut bytes, contents).expect("a world always serializes");
    bytes
}

/// Reads a save, upgrading an older schema to the current one, or says why
/// it can't.
pub(crate) fn read(bytes: &[u8]) -> Result<Loaded, LoadError> {
    let mut rest = bytes.strip_prefix(MAGIC).ok_or(LoadError::NotASave)?;
    let header: Header = rmp_serde::from_read(&mut rest).map_err(|_| LoadError::NotASave)?;
    match header.schema_version {
        0 => Err(LoadError::NotASave),
        schema if schema > SCHEMA_VERSION => Err(LoadError::Newer {
            schema,
            sim_version: header.sim_version,
        }),
        schema => upgrade(schema, rest),
    }
}

/// Reads a world saved in `schema`, which this build knows, and upgrades it
/// one schema at a time to the current one.
fn upgrade(schema: u32, body: &[u8]) -> Result<Loaded, LoadError> {
    match schema {
        // When schema 2 comes: `1 => decode::<v1::Contents>(body).map(migrate_v1_to_v2)`,
        // with `2 => decode(body)` the current schema.
        1 => decode(body),
        _ => unreachable!("schema {schema} is between 1 and {SCHEMA_VERSION}"),
    }
}

/// Decodes a save's body as `T`.
fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, LoadError> {
    rmp_serde::from_slice(body).map_err(|err| LoadError::Damaged(err.to_string()))
}

/// Reads bytes that were serialized as bytes, for state the hash keeps
/// compact, such as the map's tiles.
pub(crate) fn read_bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    deserializer.deserialize_byte_buf(BytesVisitor)
}

/// Takes bytes however the format hands them over.
struct BytesVisitor;

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bytes")
    }

    fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Vec<u8>, E> {
        Ok(bytes.to_vec())
    }

    fn visit_byte_buf<E: de::Error>(self, bytes: Vec<u8>) -> Result<Vec<u8>, E> {
        Ok(bytes)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
        let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(byte) = seq.next_element()? {
            bytes.push(byte);
        }
        Ok(bytes)
    }
}
