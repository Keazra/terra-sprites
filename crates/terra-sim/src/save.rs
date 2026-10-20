//! Save files (design §2.8): the magic `TSPR`, then a header naming the
//! save's schema, the build that wrote it and the world's checksum, then
//! the world, each in MessagePack with named fields.
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

use std::collections::BTreeMap;

use xxhash_rust::xxh3::xxh3_64;

use crate::config::{Parts, WorldConfig};
use crate::data::{DataError, DataPack};
use crate::map::{MAX_SIDE, Pos};
use crate::world::WorldState;

/// What every save starts with.
const MAGIC: &[u8; 4] = b"TSPR";

/// The schema this build writes, and the newest it reads.
pub const SCHEMA_VERSION: u32 = 1;

/// The build that wrote a save, for the message when a newer one is
/// refused, and that recorded a replay, which plays only in the same build
/// (design §2.7).
pub(crate) const SIM_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            LoadError::Pack(DataError::MissingFile(file)) => {
                write!(f, "its data pack has no {file}")
            }
            LoadError::Pack(
                DataError::Parse { file, message } | DataError::Invalid { file, message },
            ) => write!(f, "its data pack's {file} doesn't load: {message}"),
        }
    }
}

/// What follows the magic. Its shape never changes, so any build can read
/// any save's header and say why it refuses it. A replay starts the same
/// way, after a magic of its own (design §2.7).
#[derive(Serialize, Deserialize)]
pub(crate) struct Header {
    pub(crate) schema_version: u32,
    pub(crate) sim_version: String,
    /// The body's bytes, hashed, so a file damaged on disk is refused
    /// before it's read, rather than read wrong.
    pub(crate) checksum: u64,
}

impl Header {
    /// Whether `body` is what the header's checksum was taken of.
    pub(crate) fn checks(&self, body: &[u8]) -> bool {
        xxh3_64(body) == self.checksum
    }
}

/// `magic`, then a header for this build, then `body`.
pub(crate) fn framed(magic: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let header = Header {
        schema_version: SCHEMA_VERSION,
        sim_version: SIM_VERSION.into(),
        checksum: xxh3_64(body),
    };
    let mut bytes = magic.to_vec();
    rmp_serde::encode::write_named(&mut bytes, &header).expect("a header always serializes");
    bytes.extend_from_slice(body);
    bytes
}

/// The header and the body after it, if `bytes` start with `magic` and a
/// header.
pub(crate) fn unframed<'a>(magic: &[u8; 4], bytes: &'a [u8]) -> Option<(Header, &'a [u8])> {
    let mut rest = bytes.strip_prefix(magic)?;
    let header = rmp_serde::from_read(&mut rest).ok()?;
    Some((header, rest))
}

/// The world as a save holds it, in the current schema.
#[derive(Serialize)]
pub(crate) struct Contents<'a> {
    pub(crate) seed: u64,
    pub(crate) config: Option<Preset>,
    /// The data pack's files, as `(path within the pack, RON text)`.
    pub(crate) pack: Vec<(String, String)>,
    pub(crate) state: &'a WorldState,
    /// The tile at the middle of the screen, if a screen saved it. Added
    /// within schema 1, so a save without it reads as having none.
    pub(crate) view: Option<Pos>,
}

/// The world as it's read back: `Contents`, owned.
#[derive(Deserialize)]
pub(crate) struct Loaded {
    pub(crate) seed: u64,
    pub(crate) config: Option<Preset>,
    pub(crate) pack: Vec<(String, String)>,
    pub(crate) state: WorldState,
    #[serde(default)]
    pub(crate) view: Option<Pos>,
}

/// A world's preset as a save or replay holds it: `WorldConfig`'s fields,
/// kept here so the public `WorldConfig` can only be made by checking a
/// preset, as `config` does again.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Preset {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) objects: BTreeMap<String, u32>,
    pub(crate) per_tiles: u32,
    pub(crate) sprites: u16,
    /// A save from before breeding had none, so it gets the default (M2
    /// design §10).
    #[serde(default = "crate::config::most_sprites")]
    pub(crate) most_sprites: u16,
    /// A save from before Slice 29 was made in its watery world, of small
    /// lakes, with the first sprites anywhere (M2 design §19.1).
    #[serde(default = "crate::config::water")]
    pub(crate) water: u8,
    #[serde(default = "crate::config::lake_size")]
    pub(crate) lake_size: u8,
    #[serde(default)]
    pub(crate) start_near_water: Option<u16>,
}

impl From<&WorldConfig> for Preset {
    fn from(config: &WorldConfig) -> Preset {
        let parts = config.parts();
        Preset {
            width: parts.width,
            height: parts.height,
            objects: parts.objects,
            per_tiles: parts.per_tiles,
            sprites: parts.sprites,
            most_sprites: parts.most_sprites,
            water: parts.water,
            lake_size: parts.lake_size,
            start_near_water: parts.start_near_water,
        }
    }
}

impl Preset {
    /// The config it holds, checked against `data` as a preset file is, or
    /// what's wrong with it, as a damaged file might have it.
    pub(crate) fn config(self, data: &DataPack) -> Result<WorldConfig, String> {
        let parts = Parts {
            width: self.width,
            height: self.height,
            objects: self.objects,
            per_tiles: self.per_tiles,
            sprites: self.sprites,
            most_sprites: self.most_sprites,
            water: self.water,
            lake_size: self.lake_size,
            start_near_water: self.start_near_water,
        };
        WorldConfig::from_parts(parts, data)
            .map_err(|err| format!("its preset breaks a rule: {err}"))
    }
}

/// The save's bytes.
pub(crate) fn write(contents: &Contents) -> Vec<u8> {
    let body = rmp_serde::to_vec_named(contents).expect("a world always serializes");
    framed(MAGIC, &body)
}

/// Reads a save, upgrading an older schema to the current one, or says why
/// it can't.
pub(crate) fn read(bytes: &[u8]) -> Result<Loaded, LoadError> {
    let (header, rest) = unframed(MAGIC, bytes).ok_or(LoadError::NotASave)?;
    match header.schema_version {
        0 => Err(LoadError::NotASave),
        schema if schema > SCHEMA_VERSION => Err(LoadError::Newer {
            schema,
            sim_version: header.sim_version,
        }),
        _ if !header.checks(rest) => Err(LoadError::Damaged(
            "its contents don't match their checksum".into(),
        )),
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
pub(crate) fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, LoadError> {
    rmp_serde::from_slice(body).map_err(|err| LoadError::Damaged(err.to_string()))
}

/// Reads bytes that were serialized as bytes, for state the hash keeps
/// compact, such as the map's tiles.
pub(crate) fn read_bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    deserializer.deserialize_byte_buf(BytesVisitor)
}

/// The most bytes `BytesVisitor` makes room for before they arrive: a
/// largest map's tiles, one byte each.
const MOST_RESERVED: usize = MAX_SIDE as usize * MAX_SIDE as usize;

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
        // The length is only the file's claim, and a damaged one could claim
        // gigabytes: room is made up front for no more than `MOST_RESERVED`,
        // and for the rest as the bytes arrive, so a false claim fails where
        // the file ends.
        let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(MOST_RESERVED));
        while let Some(byte) = seq.next_element()? {
            bytes.push(byte);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde::de::{DeserializeSeed, IntoDeserializer, value};

    /// Bytes as a save keeps them, for `read_bytes`.
    #[derive(Debug, Deserialize)]
    struct Kept(#[serde(deserialize_with = "read_bytes")] Vec<u8>);

    /// Bytes written as a list of numbers, the way a format with no bytes
    /// type hands them over, read back whole.
    #[test]
    fn bytes_written_as_a_list_of_numbers_read_back() {
        let list = rmp_serde::to_vec(&[3u8, 1, 4]).expect("it serializes");
        let kept: Kept = rmp_serde::from_slice(&list).expect("it reads");
        assert_eq!(kept.0, [3, 1, 4]);
    }

    /// Two bytes that claim to be as many as a list can hold.
    struct Claiming(std::vec::IntoIter<u8>);

    impl<'de> SeqAccess<'de> for Claiming {
        type Error = value::Error;

        fn next_element_seed<T: DeserializeSeed<'de>>(
            &mut self,
            seed: T,
        ) -> Result<Option<T::Value>, value::Error> {
            self.0
                .next()
                .map(|byte| seed.deserialize(byte.into_deserializer()))
                .transpose()
        }

        fn size_hint(&self) -> Option<usize> {
            Some(usize::MAX)
        }
    }

    /// A list of bytes makes room up front for no more than a largest
    /// map's tiles, whatever it claims: making room for all a damaged save
    /// or replay claims could stop the game for want of memory, rather than
    /// say the file is damaged (design §2.8).
    #[test]
    fn a_list_of_bytes_makes_room_for_a_map_s_worth_at_most() {
        let bytes = BytesVisitor
            .visit_seq(Claiming(vec![1, 2].into_iter()))
            .expect("two bytes");
        assert_eq!(bytes, [1, 2]);
        assert!(bytes.capacity() <= MOST_RESERVED);
    }

    /// The preset a save or replay from before Slice 29 kept, with no
    /// water settings.
    #[derive(Serialize)]
    struct PresetBeforeSlice29 {
        width: u16,
        height: u16,
        objects: BTreeMap<String, u32>,
        per_tiles: u32,
        sprites: u16,
        most_sprites: u16,
    }

    /// A save or replay from before Slice 29 was made in the watery world
    /// of small lakes, with its first sprites anywhere, so a replay
    /// generates its world again as it was (M2 design §19.1).
    #[test]
    fn a_preset_kept_before_slice_29_is_the_watery_world() {
        let data = crate::test_util::builtin();
        let old = PresetBeforeSlice29 {
            width: 64,
            height: 40,
            objects: BTreeMap::new(),
            per_tiles: 1,
            sprites: 20,
            most_sprites: 100,
        };
        let bytes = rmp_serde::to_vec_named(&old).expect("it serializes");
        let preset: Preset = rmp_serde::from_slice(&bytes).expect("it reads");
        let config = preset.config(&data).expect("it keeps the rules");
        assert_eq!(
            (
                config.water(),
                config.lake_size(),
                config.start_near_water()
            ),
            (32, 1, None)
        );
    }

    /// A list that claims to hold four billion bytes but holds two is
    /// refused where the file ends.
    #[test]
    fn a_list_of_bytes_claiming_more_than_it_holds_is_refused() {
        // MessagePack's `array 32` of u32::MAX items, then two of them.
        let claim = [0xdd, 0xff, 0xff, 0xff, 0xff, 0x01, 0x02];
        let read: Result<Kept, _> = rmp_serde::from_slice(&claim);
        assert!(read.is_err());
    }
}
