//! Replays (design §2.7): a world's start, every command submitted to it
//! and a hash of it every 1,000 ticks, so a session can be played back
//! exactly, and playback can say where it first parted from the recording.
//!
//! A replay is framed as a save is (`save::framed`), after a magic of its
//! own, and plays only in the build that recorded it: the same sim version
//! and save format.

use std::fmt;

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::command::Command;
use crate::config::WorldConfig;
use crate::data::{DataError, DataPack};
use crate::events::Event;
use crate::save::{self, LoadError, Preset, SCHEMA_VERSION, SIM_VERSION};
use crate::world::World;

/// What every replay starts with.
const MAGIC: &[u8; 4] = b"TSRP";

/// How often a recording takes the world's hash, in ticks: at every tick
/// that's a multiple of it, as well as where it starts (design §2.7).
pub const CHECKPOINT_EVERY: u64 = 1_000;

/// Why a replay can't be played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// It isn't a Terra Sprites replay.
    NotAReplay,
    /// Another build recorded it: its sim version and save format.
    OtherBuild { sim_version: String, schema: u32 },
    /// It's damaged: what's wrong.
    Damaged(String),
    /// The data pack it embeds doesn't load in this build.
    Pack(DataError),
    /// Its world was made with another data pack than the one it names:
    /// the two packs, as `name version (files #hash)`.
    PackMismatch { named: String, found: String },
    /// Its start, a save, doesn't load.
    Start(LoadError),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayError::NotAReplay => f.write_str("it isn't a Terra Sprites replay"),
            ReplayError::OtherBuild {
                sim_version,
                schema,
            } => write!(
                f,
                "it was recorded by Terra Sprites {sim_version} (save format {schema}), and a \
                 replay plays only in the build that recorded it: this is {SIM_VERSION} (save \
                 format {SCHEMA_VERSION})"
            ),
            ReplayError::Damaged(what) => write!(f, "it's damaged: {what}"),
            // A replay's pack fails as a save's does.
            ReplayError::Pack(err) => LoadError::Pack(err.clone()).fmt(f),
            ReplayError::PackMismatch { named, found } => write!(
                f,
                "it names the data pack {named}, but its world was made with {found}"
            ),
            ReplayError::Start(err) => write!(f, "its start doesn't load: {err}"),
        }
    }
}

/// Where a replay starts (design §2.7).
#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum Start {
    /// A world generated from the preset and the seed, before anything
    /// happened to it.
    Fresh { seed: u64, config: Preset },
    /// A world's save, as of the tick the recording began.
    Snapshot(#[serde(serialize_with = "bytes", deserialize_with = "save::read_bytes")] Vec<u8>),
}

/// Writes bytes as bytes, rather than as a list of numbers.
fn bytes<S: serde::Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(bytes)
}

/// A data pack, as a replay names it: its name and version, and a hash of
/// its files, which tells apart two packs that share a name and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PackIdentity {
    name: String,
    version: String,
    hash: u64,
}

impl PackIdentity {
    /// `data`'s identity.
    fn of(data: &DataPack) -> PackIdentity {
        PackIdentity {
            name: data.name().into(),
            version: data.version().into(),
            hash: sources_hash(data.sources()),
        }
    }

    /// How a message names the pack: `core 1 (files #1a2b3c4d)`, the
    /// hash cut short.
    fn label(&self) -> String {
        format!(
            "{} {} (files #{:08x})",
            self.name,
            self.version,
            self.hash >> 32
        )
    }
}

/// A hash of a pack's files.
fn sources_hash(sources: &[(String, String)]) -> u64 {
    xxh3_64(&rmp_serde::to_vec(sources).expect("text always serializes"))
}

/// A world's session log, as it's being recorded (design §2.7). The world
/// keeps it, adding each command as it's submitted and each checkpoint as
/// its tick ends.
pub(crate) struct Recording {
    pub(crate) start: Start,
    /// Every command submitted, with the tick it's applied at, in order.
    pub(crate) commands: Vec<(u64, Command)>,
    /// The world's hash where the recording starts, and at each tick since
    /// that's a multiple of `CHECKPOINT_EVERY`, in order.
    pub(crate) checkpoints: Vec<(u64, u64)>,
}

/// A replay's body, as it's written.
#[derive(Serialize)]
struct Contents<'a> {
    pack: PackIdentity,
    /// The data pack's files, as `(path within the pack, RON text)`.
    sources: &'a [(String, String)],
    start: &'a Start,
    commands: &'a [(u64, Command)],
    checkpoints: &'a [(u64, u64)],
    /// The tick the world had reached when the replay was written.
    end: u64,
}

/// A replay's body, as it's read back: `Contents`, owned.
#[derive(Deserialize)]
struct ReadBack {
    pack: PackIdentity,
    sources: Vec<(String, String)>,
    start: Start,
    commands: Vec<(u64, Command)>,
    checkpoints: Vec<(u64, u64)>,
    end: u64,
}

/// The replay's bytes, for a recording of a world made with `data` that
/// has reached tick `end`.
pub(crate) fn write(recording: &Recording, data: &DataPack, end: u64) -> Vec<u8> {
    let body = rmp_serde::to_vec_named(&Contents {
        pack: PackIdentity::of(data),
        sources: data.sources(),
        start: &recording.start,
        commands: &recording.commands,
        checkpoints: &recording.checkpoints,
        end,
    })
    .expect("a replay always serializes");
    save::framed(MAGIC, &body)
}

/// Whether `ticks` never go back, from `start` on.
fn in_order(start: u64, mut ticks: impl Iterator<Item = u64>) -> bool {
    let mut last = start;
    ticks.all(|tick| {
        let ok = tick >= last;
        last = tick;
        ok
    })
}

/// Where playback first found the world different from the recording: the
/// first checkpoint whose hash didn't match, and the last one before it
/// that did, if any (design §2.7). The world parted from the recording at
/// some tick after the one and no later than the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence {
    /// The tick of the first checkpoint that didn't match.
    pub tick: u64,
    /// The tick of the last checkpoint that matched before it.
    pub matched: Option<u64>,
}

/// A replay being played back (design §2.7): the world it starts from,
/// stepped with the recorded commands at their ticks, checked at each
/// checkpoint.
pub struct Playback {
    world: World,
    commands: Vec<(u64, Command)>,
    /// The next of `commands` to submit.
    next: usize,
    checkpoints: Vec<(u64, u64)>,
    /// The next of `checkpoints` to check.
    next_checkpoint: usize,
    start: u64,
    end: u64,
    fresh: bool,
    matched: Option<u64>,
    divergence: Option<Divergence>,
}

impl Playback {
    /// Reads a replay and builds the world it starts from, or says why it
    /// can't be played. Its start is checked at once, so a world that
    /// starts out different is a divergence at the first checkpoint.
    pub fn new(bytes: &[u8]) -> Result<Playback, ReplayError> {
        let (header, body) = save::unframed(MAGIC, bytes).ok_or(ReplayError::NotAReplay)?;
        if header.schema_version != SCHEMA_VERSION || header.sim_version != SIM_VERSION {
            return Err(ReplayError::OtherBuild {
                sim_version: header.sim_version,
                schema: header.schema_version,
            });
        }
        if !header.checks(body) {
            return Err(ReplayError::Damaged(
                "its contents don't match their checksum".into(),
            ));
        }
        let read: ReadBack = save::decode(body).map_err(|err| match err {
            LoadError::Damaged(what) => ReplayError::Damaged(what),
            other => ReplayError::Damaged(other.to_string()),
        })?;
        if sources_hash(&read.sources) != read.pack.hash {
            return Err(ReplayError::Damaged(
                "its data pack doesn't match the one it names".into(),
            ));
        }
        let fresh = matches!(read.start, Start::Fresh { .. });
        let world = match read.start {
            Start::Fresh { seed, config } => {
                let sources: Vec<(&str, &str)> = read
                    .sources
                    .iter()
                    .map(|(path, text)| (path.as_str(), text.as_str()))
                    .collect();
                let data = DataPack::from_sources(&sources).map_err(ReplayError::Pack)?;
                World::new(WorldConfig::from(config), data, seed)
            }
            Start::Snapshot(save) => World::load(&save).map_err(ReplayError::Start)?,
        };
        // A save embeds its own pack, which must be the one the replay
        // names; and a fresh world's pack must be what its files say.
        let made_with = PackIdentity::of(world.data());
        if made_with != read.pack {
            return Err(ReplayError::PackMismatch {
                named: read.pack.label(),
                found: made_with.label(),
            });
        }
        let start = world.tick();
        let ticks_ok = in_order(start, read.commands.iter().map(|&(tick, _)| tick))
            && in_order(start, read.checkpoints.iter().map(|&(tick, _)| tick))
            && read.end >= start;
        if !ticks_ok {
            return Err(ReplayError::Damaged("its ticks are out of order".into()));
        }
        let mut playback = Playback {
            world,
            commands: read.commands,
            next: 0,
            checkpoints: read.checkpoints,
            next_checkpoint: 0,
            start,
            end: read.end,
            fresh,
            matched: None,
            divergence: None,
        };
        playback.check();
        Ok(playback)
    }

    /// The world as it plays back.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The tick the replay starts at.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// The tick the recording had reached when it was written: where the
    /// replay ends. Time can run on past it, with no more commands than
    /// those still waiting then, for the tick a panic may have cut short.
    pub fn end(&self) -> u64 {
        self.end
    }

    /// Whether the replay starts from a world generated afresh, rather
    /// than from a save.
    pub fn started_fresh(&self) -> bool {
        self.fresh
    }

    /// Where the world first parted from the recording, if it has.
    pub fn divergence(&self) -> Option<Divergence> {
        self.divergence
    }

    /// Runs one tick: submits the commands recorded for it, in the order
    /// they were submitted, steps the world, and checks it against the
    /// recording's checkpoint for the tick it has reached, if there's one.
    pub fn step(&mut self) -> Vec<Event> {
        let tick = self.world.tick();
        while let Some((at, command)) = self.commands.get(self.next)
            && *at <= tick
        {
            self.world.submit(command.clone());
            self.next += 1;
        }
        let events = self.world.step();
        self.check();
        events
    }

    /// For tests: submits a command the recording doesn't have, so the world
    /// parts from it.
    #[doc(hidden)]
    pub fn inject(&mut self, command: Command) {
        self.world.submit(command);
    }

    /// Checks the world against the recording's checkpoint for the tick it
    /// has reached, if there's one, noting the first that doesn't match.
    fn check(&mut self) {
        let tick = self.world.tick();
        while let Some(&(at, hash)) = self.checkpoints.get(self.next_checkpoint)
            && at <= tick
        {
            self.next_checkpoint += 1;
            if at < tick || self.divergence.is_some() {
                continue;
            }
            if self.world.state_hash() == hash {
                self.matched = Some(tick);
            } else {
                self.divergence = Some(Divergence {
                    tick,
                    matched: self.matched,
                });
            }
        }
    }
}
