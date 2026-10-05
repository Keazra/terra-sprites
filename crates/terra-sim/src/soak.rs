//! The soak (design §7.4 A7, §7.6): the default world run for a million
//! ticks with a random script of the Cursor's commands, looking only for
//! panics and broken invariants.
//!
//! The script plays the player. It sends the commands the screen sends, at
//! random, and some the screen never would: touches and grips on sprites
//! that have died, tiles off the map, object types the pack doesn't have and
//! names a sprite can't carry. The world must refuse those, never break. Its
//! randomness is its own, from its seed, so the world's RNG draws nothing
//! for it, and the same seed gives the same run.

use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{Rng, SeedableRng};

use crate::command::{Command, CursorTouch};
use crate::config::WorldConfig;
use crate::cursor::Grip;
use crate::data::DataPack;
use crate::events::{DeathCause, EventKind};
use crate::map::{Dir, Pos};
use crate::objects::EntityId;
use crate::random::{chance, uniform};
use crate::world::World;

/// How likely the script is to act on any one tick: about once a second at
/// normal speed, a busy player's pace.
const ACT_CHANCE: f32 = 0.1;

/// How likely the Cursor is to move on a tick while it leads a sprite, as a
/// drag moves it.
const LEAD_MOVE_CHANCE: f32 = 0.5;

/// How likely a target is to be one the screen would never send: a sprite
/// or item that isn't there, or a tile off the map.
const STRAY_CHANCE: f32 = 0.05;

/// What the script can do, each with how often it's picked against the
/// others.
const ACTS: [(Act, u64); 15] = [
    (Act::Touch(CursorTouch::Pet), 2),
    (Act::Touch(CursorTouch::Hug), 1),
    (Act::Touch(CursorTouch::Zap), 2),
    (Act::Touch(CursorTouch::Shock), 1),
    (Act::TakeHold, 2),
    (Act::LetGo, 1),
    (Act::Shove, 2),
    (Act::PickUp, 2),
    (Act::PutDown, 1),
    (Act::Throw, 2),
    (Act::MoveCursor, 1),
    (Act::ShowCursor, 1),
    (Act::Place, 1),
    (Act::SpawnSprite, 1),
    (Act::Rename, 1),
];

/// One of the script's acts.
#[derive(Debug, Clone, Copy)]
enum Act {
    Touch(CursorTouch),
    TakeHold,
    LetGo,
    Shove,
    PickUp,
    PutDown,
    Throw,
    MoveCursor,
    ShowCursor,
    Place,
    SpawnSprite,
    Rename,
}

/// The soak's random script of the Cursor's commands (design §7.6).
#[derive(Debug, Clone)]
pub struct Soak {
    rng: ChaCha8Rng,
    /// It spawns sprites only while fewer than this many live.
    most_sprites: usize,
}

/// What a soak did, for its report. A soak that broke panicked instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoakRun {
    pub ticks: u64,
    /// The commands the script gave, by name.
    pub given: BTreeMap<&'static str, u64>,
    /// The commands the world refused, by name.
    pub refused: BTreeMap<&'static str, u64>,
    /// Deaths, by cause.
    pub deaths: BTreeMap<DeathCause, u64>,
    /// Sprites alive at the end.
    pub sprites: usize,
    /// The most sprites alive at once.
    pub most_sprites: usize,
    /// Objects in the world at the end.
    pub objects: usize,
}

impl Soak {
    /// A script for `world`, as it is now, from `seed`. It spawns sprites
    /// only while there are fewer than twice as many as now, so a long soak
    /// doesn't slow to a crawl under its own crowd.
    pub fn new(world: &World, seed: u64) -> Soak {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        // A stream of its own, so it never draws what a world from the same
        // seed draws.
        rng.set_stream(1);
        Soak {
            rng,
            most_sprites: 2 * world.sprites().count().max(1),
        }
    }

    /// Runs the default world from `seed` for `ticks` ticks with the
    /// script from the same seed, submitting its commands before each tick.
    /// With debug assertions on, a broken invariant panics naming the tick
    /// (design §7.1).
    pub fn run(data: DataPack, seed: u64, ticks: u64) -> SoakRun {
        let mut world = World::new(WorldConfig::builtin(&data), data, seed);
        let mut script = Soak::new(&world, seed);
        let mut run = SoakRun {
            ticks: 0,
            given: BTreeMap::new(),
            refused: BTreeMap::new(),
            deaths: BTreeMap::new(),
            sprites: 0,
            most_sprites: world.sprites().count(),
            objects: 0,
        };
        for _ in 0..ticks {
            for command in script.commands(&world) {
                *run.given.entry(name(&command)).or_insert(0) += 1;
                world.submit(command);
            }
            for event in world.step() {
                if let EventKind::CommandRejected { command, .. } = &event.kind {
                    *run.refused.entry(name(command)).or_insert(0) += 1;
                }
            }
            run.ticks += 1;
            run.most_sprites = run.most_sprites.max(world.sprites().count());
        }
        run.deaths = world.deaths_by_cause().collect();
        run.sprites = world.sprites().count();
        run.objects = world.objects().count();
        run
    }

    /// The commands the script gives before `world`'s next tick, if any.
    pub fn commands(&mut self, world: &World) -> Vec<Command> {
        let mut commands = Vec::new();
        // While it leads a sprite, the Cursor wanders about it, as a drag does.
        let led = world.cursor().leads().and_then(|id| world.sprite(id));
        if let Some(led) = led
            && chance(&mut self.rng, LEAD_MOVE_CHANCE)
        {
            let tile = self.near(world, led.pos());
            commands.push(Command::MoveCursor { tile });
        }
        if chance(&mut self.rng, ACT_CHANCE) {
            commands.extend(self.act(world));
        }
        commands
    }

    /// One act, picked by its weight, as a command, or none when the act
    /// has nothing to work with, such as a spawn in a crowded world.
    fn act(&mut self, world: &World) -> Option<Command> {
        let total: u64 = ACTS.iter().map(|(_, weight)| weight).sum();
        let mut pick = uniform(&mut self.rng, total);
        let act = ACTS
            .iter()
            .find(|(_, weight)| {
                let found = pick < *weight;
                pick = pick.saturating_sub(*weight);
                found
            })
            .map(|(act, _)| *act)
            .expect("a pick below the total weight");
        let command = match act {
            Act::Touch(touch) => {
                let reach_back = uniform(&mut self.rng, 200);
                touch.command(self.sprite(world), reach_back)
            }
            Act::TakeHold => Command::TakeHold {
                sprite: self.sprite(world),
            },
            Act::LetGo => Command::LetGo,
            Act::Shove => {
                let toward = self.dir();
                let furthest = world
                    .cursor()
                    .leads()
                    .map(|id| world.furthest(Grip::Leads(id)));
                let tiles = self.tiles(furthest);
                Command::Shove { toward, tiles }
            }
            Act::PickUp => Command::PickUp {
                item: self.object(world),
            },
            Act::PutDown => Command::PutDown {
                tile: self.tile(world),
            },
            Act::Throw => {
                let from = match world.cursor().tile() {
                    Some(tile) => self.near(world, tile),
                    None => self.tile(world),
                };
                let toward = self.dir();
                let held = world.cursor().holds().map(|held| held.id());
                let tiles = self.tiles(held.map(|id| world.furthest(Grip::Holds(id))));
                Command::Throw {
                    from,
                    toward,
                    tiles,
                }
            }
            Act::MoveCursor => Command::MoveCursor {
                tile: self.tile(world),
            },
            Act::ShowCursor => Command::ShowCursor {
                visible: !world.cursor().visible(),
            },
            Act::Place => {
                let tile = self.tile(world);
                let types: Vec<&str> = world.data().object_type_names().collect();
                let object_type = if chance(&mut self.rng, STRAY_CHANCE) {
                    u16::MAX
                } else {
                    let name = types[uniform(&mut self.rng, types.len() as u64) as usize];
                    world
                        .data()
                        .object_type_id(name)
                        .expect("a type the pack has")
                };
                Command::Place { tile, object_type }
            }
            Act::SpawnSprite => {
                if world.sprites().count() >= self.most_sprites {
                    return None;
                }
                let tile = self.tile(world);
                // Now and then a copy of a living sprite's genome, as a
                // genome file would bring.
                let genome = if chance(&mut self.rng, 0.3) {
                    let id = self.sprite(world);
                    world.sprite(id).map(|sprite| sprite.genome().clone())
                } else {
                    None
                };
                Command::SpawnSprite { tile, genome }
            }
            Act::Rename => {
                let sprite = self.sprite(world);
                let name = if chance(&mut self.rng, 0.2) {
                    const BAD: [&str; 4] = ["", "   ", "a name far too long to carry", "名前"];
                    BAD[uniform(&mut self.rng, BAD.len() as u64) as usize].to_string()
                } else {
                    world.data().random_name(self.rng.next_u64())
                };
                Command::Rename { sprite, name }
            }
        };
        Some(command)
    }

    /// A living sprite, or now and then one that isn't there.
    fn sprite(&mut self, world: &World) -> EntityId {
        let ids: Vec<EntityId> = world.sprites().map(|sprite| sprite.id()).collect();
        self.pick(&ids)
    }

    /// An item, a fixture now and then, or one that isn't there.
    fn object(&mut self, world: &World) -> EntityId {
        let fixture = chance(&mut self.rng, 0.1);
        let ids: Vec<EntityId> = world
            .objects()
            .filter(|object| object.is_solid() == fixture)
            .map(|object| object.id())
            .collect();
        self.pick(&ids)
    }

    /// One of `ids`, or now and then, or if there are none, an ID nothing
    /// in the world has.
    fn pick(&mut self, ids: &[EntityId]) -> EntityId {
        if ids.is_empty() || chance(&mut self.rng, STRAY_CHANCE) {
            return EntityId(u64::MAX - uniform(&mut self.rng, 1_000));
        }
        ids[uniform(&mut self.rng, ids.len() as u64) as usize]
    }

    /// Any tile on the map, or now and then one just off it.
    fn tile(&mut self, world: &World) -> Pos {
        let map = world.map();
        let (width, height) = (u64::from(map.width()), u64::from(map.height()));
        let stray = if chance(&mut self.rng, STRAY_CHANCE) {
            5
        } else {
            0
        };
        Pos {
            x: uniform(&mut self.rng, width + stray) as u16,
            y: uniform(&mut self.rng, height + stray) as u16,
        }
    }

    /// A tile on the map within three of `pos`, either way.
    fn near(&mut self, world: &World, pos: Pos) -> Pos {
        let map = world.map();
        let mut step = |at: u16, size: u16| {
            let moved = i64::from(at) + uniform(&mut self.rng, 7) as i64 - 3;
            moved.clamp(0, i64::from(size) - 1) as u16
        };
        Pos {
            x: step(pos.x, map.width()),
            y: step(pos.y, map.height()),
        }
    }

    /// One of the eight directions.
    fn dir(&mut self) -> Dir {
        Dir::ALL[uniform(&mut self.rng, Dir::ALL.len() as u64) as usize]
    }

    /// How far to throw or shove: up to two tiles past the `furthest` the
    /// Cursor can, which the world cuts back, and now and then none at all.
    fn tiles(&mut self, furthest: Option<u16>) -> u16 {
        let furthest = u64::from(furthest.unwrap_or(4));
        uniform(&mut self.rng, furthest + 3) as u16
    }
}

/// A command's name, as a soak's report counts it.
fn name(command: &Command) -> &'static str {
    if let Some(touch) = CursorTouch::of(command) {
        return touch.name();
    }
    match command {
        Command::TakeHold { .. } => "take hold",
        Command::PickUp { .. } => "pick up",
        Command::PutDown { .. } => "put down",
        Command::LetGo => "let go",
        Command::MoveCursor { .. } => "move the Cursor",
        Command::ShowCursor { .. } => "show or hide the Cursor",
        Command::Throw { .. } => "throw",
        Command::Shove { .. } => "shove",
        Command::Place { .. } => "place",
        Command::SpawnSprite { .. } => "spawn",
        Command::Rename { .. } => "rename",
        Command::Reward { .. } | Command::Correct { .. } => unreachable!("a touch"),
    }
}
