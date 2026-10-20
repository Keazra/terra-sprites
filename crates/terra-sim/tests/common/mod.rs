//! Shared helpers for terra-sim integration tests.
//!
//! Each test file is its own crate, so it `mod common;` and uses only the
//! pieces it needs. Helpers a given file never calls are allowed here so the
//! shared API stays one place (#155).
#![allow(dead_code)]

use terra_sim::{
    Command, DataPack, EntityId, Event, EventKind, Genome, Learned, Map, Outcome, Pos, Scenario,
    ScenarioError, ScriptedAction, Verb, World,
};

/// The built-in data pack.
pub fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

/// A position at `(x, y)`.
pub fn at(x: u16, y: u16) -> Pos {
    Pos::new(x, y)
}

/// True when two levels are the same, give or take a little.
pub fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A genome with only traits: speed 10, sense radius **10** — so a script
/// decides what the sprite does. For a different sense radius (e.g. movement
/// tests at 14), call [`traits_only`] instead.
pub fn walker(data: &DataPack) -> Genome {
    traits_only(10.0, 10.0, data)
}

/// A genome with only `speed` and `sense_radius`.
pub fn traits_only(speed: f32, sense_radius: f32, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 1, genes: [
            Trait(trait: "speed", value: {speed:?}),
            Trait(trait: "sense_radius", value: {sense_radius:?}),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// A genome of `genes`, which may switch with life stage, plus speed and
/// sense 10, with learning that does not fade, so what is learned reads
/// exactly.
pub fn genome(genes: &str, data: &DataPack) -> Genome {
    let text = format!(
        r#"(format: 2, genes: [
            Trait(trait: "speed", value: 10.0),
            Trait(trait: "sense_radius", value: 10.0),
            {genes}
            // No fading, so what is learned reads exactly.
            BrainParam(param: "worth_fade_good", value: 0.0),
            BrainParam(param: "worth_fade_bad", value: 0.0),
            BrainParam(param: "habit_fade", value: 0.0),
            BrainParam(param: "habit_fade_bad", value: 0.0),
            BrainParam(param: "fear_fade", value: 0.0),
            BrainParam(param: "taught_fade", value: 0.0),
        ])"#
    );
    Genome::from_ron(&text, data).expect("a valid genome")
}

/// Every action that ended in `events`, as `(verb, outcome)`.
pub fn endings(events: &[Event]) -> Vec<(Verb, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::ActionEnded { verb, outcome, .. } => Some((verb, outcome)),
            _ => None,
        })
        .collect()
}

/// The change to `physiology.ron` that puts breeding out of reach: no
/// sprite is ever fertile (M2 design §10).
pub const BARREN: (&str, &str) = ("fed: 0.4,", "fed: 1.0,");

/// The built-in data pack with breeding out of reach, for a test of
/// something else that watches grown sprites long enough to breed.
pub fn barren() -> DataPack {
    builtin_changing("physiology.ron", &[BARREN])
}

/// The built-in data pack, with every sprite's body starting with `energy`
/// and `hydration`, as a newborn's does.
pub fn starting_with(energy: f32, hydration: f32) -> DataPack {
    let newborn = format!("newborn: (energy: {energy:?}, hydration: {hydration:?}, stamina: 1.0),");
    let builtin = "newborn: (energy: 1.0, hydration: 1.0, stamina: 1.0),";
    builtin_changing("physiology.ron", &[(builtin, newborn.as_str())])
}

/// The built-in data pack, with `changes` made to `file`'s text.
pub fn builtin_changing(file: &str, changes: &[(&str, &str)]) -> DataPack {
    builtin_changing_files(&[(file, changes)])
}

/// The built-in data pack, with `file`'s text replaced by `text`.
pub fn builtin_with(file: &str, text: &str) -> DataPack {
    let sources = DataPack::builtin_sources();
    assert!(
        sources.iter().any(|&(path, _)| path == file),
        "{file} is in the pack"
    );
    let replaced: Vec<(&str, &str)> = sources
        .iter()
        .map(|&(path, builtin)| (path, if path == file { text } else { builtin }))
        .collect();
    DataPack::from_sources(&replaced).expect("the changed pack is valid")
}

/// The built-in data pack, with each file's `changes` made to its text.
pub fn builtin_changing_files(files: &[(&str, &[(&str, &str)])]) -> DataPack {
    let sources: Vec<(&str, String)> = DataPack::builtin_sources()
        .iter()
        .map(|&(path, text)| {
            let mut text = text.to_string();
            for &(file, changes) in files.iter().filter(|(file, _)| *file == path) {
                for &(from, to) in changes {
                    assert!(text.contains(from), "{from:?} is in {file}");
                    text = text.replace(from, to);
                }
            }
            (path, text)
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = sources.iter().map(|(p, t)| (*p, t.as_str())).collect();
    DataPack::from_sources(&borrowed).expect("the changed pack is valid")
}

/// Makes a sprite's genome from the data pack.
pub type MakeGenome = fn(&DataPack) -> Genome;

/// Builds a hand-drawn test world: rows, objects, sprites (of genes, of a
/// genome, or of the starter genome), scripted actions, and a seed.
///
/// Sprite and script methods append; [`Self::objects`] replaces.
pub struct WorldBuilder<'a> {
    rows: &'a [&'a str],
    objects: &'a [(Pos, &'a str)],
    sprites: Vec<(Pos, SpriteGenome<'a>)>,
    scripted: Vec<(Pos, ScriptedAction)>,
    newborns: Vec<Pos>,
    data: Option<DataPack>,
    seed: u64,
}

/// Where the genome of a [`WorldBuilder`] sprite comes from. Those read
/// with the data pack are read once the world is built, so the pack can be
/// set before or after.
enum SpriteGenome<'a> {
    /// The starter genome, with spawn variation.
    Starter,
    /// A genome already made.
    Made(Genome),
    /// [`genome`]`(genes)`, read with the world's data pack.
    Genes(&'a str),
    /// What this makes from the world's data pack.
    MadeBy(MakeGenome),
}

impl<'a> WorldBuilder<'a> {
    /// Starts from an ascii drawing of the map.
    pub fn new(rows: &'a [&'a str]) -> Self {
        Self {
            rows,
            objects: &[],
            sprites: Vec::new(),
            scripted: Vec::new(),
            newborns: Vec::new(),
            data: None,
            seed: 1,
        }
    }

    /// Objects as `(tile, object type name)` (replaces any previous list).
    pub fn objects(mut self, objects: &'a [(Pos, &'a str)]) -> Self {
        self.objects = objects;
        self
    }

    /// Uses this data pack instead of the built-in one.
    pub fn data(mut self, data: DataPack) -> Self {
        self.data = Some(data);
        self
    }

    /// The world seed (default 1).
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// A sprite of [`genome`]`(genes)` on each tile, as `(tile, genes)`
    /// (appends).
    pub fn sprites(mut self, sprites: &[(Pos, &'a str)]) -> Self {
        let genomes = sprites
            .iter()
            .map(|&(pos, genes)| (pos, SpriteGenome::Genes(genes)));
        self.sprites.extend(genomes);
        self
    }

    /// A sprite on each tile with the genome its function makes, as
    /// `(tile, function)` (appends).
    pub fn sprites_made(mut self, sprites: &[(Pos, MakeGenome)]) -> Self {
        let genomes = sprites
            .iter()
            .map(|&(pos, make)| (pos, SpriteGenome::MadeBy(make)));
        self.sprites.extend(genomes);
        self
    }

    /// Sprites already built as `(pos, optional genome)`, `None` for the
    /// starter genome (appends).
    pub fn sprites_with(mut self, sprites: Vec<(Pos, Option<Genome>)>) -> Self {
        let genomes = sprites.into_iter().map(|(pos, genome)| {
            let genome = genome.map_or(SpriteGenome::Starter, SpriteGenome::Made);
            (pos, genome)
        });
        self.sprites.extend(genomes);
        self
    }

    /// A sprite of the starter genome, with spawn variation, on each of
    /// `tiles` (appends).
    pub fn starters(mut self, tiles: &[Pos]) -> Self {
        let genomes = tiles.iter().map(|&pos| (pos, SpriteGenome::Starter));
        self.sprites.extend(genomes);
        self
    }

    /// Scripted actions by starting tile (appends).
    pub fn scripts(mut self, scripted: &[(Pos, ScriptedAction)]) -> Self {
        self.scripted.extend_from_slice(scripted);
        self
    }

    /// The sprite starting on `tile` does `script`, in order (appends).
    pub fn script(mut self, tile: Pos, script: &[ScriptedAction]) -> Self {
        self.scripted
            .extend(script.iter().map(|&action| (tile, action)));
        self
    }

    /// The tiles of sprites that start as newborns, not grown up (M2 design
    /// §9.1) (appends).
    pub fn newborns(mut self, newborns: &[Pos]) -> Self {
        self.newborns.extend_from_slice(newborns);
        self
    }

    /// Builds the world.
    pub fn build(self) -> World {
        self.try_build().expect("a valid scenario")
    }

    /// Builds the world, or says why the scenario can't be one, for a test
    /// of what `World::from_scenario` turns away. The drawing must be valid.
    pub fn try_build(self) -> Result<World, ScenarioError> {
        let data = self.data.unwrap_or_else(builtin);
        let map = Map::from_ascii(self.rows, &data).expect("valid drawing");
        let sprites: Vec<(Pos, Option<Genome>)> = self
            .sprites
            .into_iter()
            .map(|(pos, source)| match source {
                SpriteGenome::Starter => (pos, None),
                SpriteGenome::Made(genome) => (pos, Some(genome)),
                SpriteGenome::Genes(genes) => (pos, Some(genome(genes, &data))),
                SpriteGenome::MadeBy(make) => (pos, Some(make(&data))),
            })
            .collect();
        let scenario = Scenario {
            map,
            objects: self.objects,
            sprites: &sprites,
            scripted: &self.scripted,
            newborns: &self.newborns,
        };
        World::from_scenario(scenario, data, self.seed)
    }
}

/// An instinct to mate with whatever sprite it sees, and nothing else (M2
/// design §10), and a body receptive from the start, as if courted
/// (§15.3).
pub const KEEN: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
    InitialConcentration(chem: "receptivity", value: 1.0),
"#;

/// A body receptive from the start, as if courted (M2 design §15.3).
pub const RECEPTIVE: &str = r#"InitialConcentration(chem: "receptivity", value: 1.0),"#;

/// Genes that copy the pulse `pulse` into `h1`, so a test can read it: 1 on
/// the tick it goes live.
pub fn pulse_in_h1(pulse: &str) -> String {
    format!(
        r#"
    Emitter(locus: Locus("{pulse}"), mode: Level, gain: 4.0, chem: "h1"),
    HalfLife(chem: "h1", ticks: 1),
"#
    )
}

/// A body that turns female at once, as an adult (M2 design §10).
pub const FEMALE: &str = r#"InitialConcentration(chem: "feminiser", value: 1.0),"#;

/// An instinct to breed alone whenever it can, and the longing it takes.
pub const ALONE: &str = r#"
    Instinct(inputs: [("always", false)], verb: BreedAlone, weight: 1.0),
    InitialConcentration(chem: "longing", value: 1.0),
"#;

/// The change to `physiology.ron` that lets a female breed alone at once,
/// with a male in sight or not (M2 design §15.7), for tests of what a lone
/// egg does.
pub const LONE: (&str, &str) = ("alone_after: 4500,", "alone_after: 0,");

/// The built-in data pack in which a female may breed alone at once.
pub fn lone() -> DataPack {
    builtin_changing("physiology.ron", &[LONE])
}

/// The changes to `physiology.ron` that let a sprite grow up and grow old
/// as soon as 1 tick old (M2 design §9.3), so a test of a young or old
/// sprite needn't wait the thousands of ticks the built-in ranges take.
pub const AGES_AT_ONCE: [(&str, &str); 2] = [
    ("grows_old: (6000.0,", "grows_old: (1.0,"),
    ("grows_up: (1500.0,", "grows_up: (1.0,"),
];

/// Genes that have a sprite grow up and grow old at 1 tick old, in a pack
/// changed by `AGES_AT_ONCE`: a scenario's sprite, which starts just grown
/// up, starts old (M2 design §9.1).
pub const OLD: &str = r#"Trait(trait: "grows_up", value: 1.0),
    Trait(trait: "grows_old", value: 1.0),"#;

/// A world drawn from `rows`, with a sprite of its own genes on each tile,
/// all grown up, made with `data`.
pub fn world_in(data: DataPack, rows: &[&str], sprites: &[(Pos, &str)]) -> World {
    WorldBuilder::new(rows).data(data).sprites(sprites).build()
}

/// A world drawn from `rows`, with `objects`, and a [`walker`] on each tile
/// of `sprites` doing its script, made with `data`.
pub fn walkers_in(
    data: DataPack,
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[(Pos, &[ScriptedAction])],
) -> World {
    let mut builder = WorldBuilder::new(rows).data(data).objects(objects);
    for &(pos, script) in sprites {
        builder = builder.sprites_made(&[(pos, walker)]).script(pos, script);
    }
    builder.build()
}

/// The same, with the built-in pack.
pub fn walkers(
    rows: &[&str],
    objects: &[(Pos, &str)],
    sprites: &[(Pos, &[ScriptedAction])],
) -> World {
    walkers_in(builtin(), rows, objects, sprites)
}

/// The ID of the sprite standing on `pos`.
pub fn id_at(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
}

/// What sprite `id` has learned `learned` is worth, or 0.
pub fn value_of(world: &World, id: EntityId, learned: &Learned) -> f32 {
    let sprite = world.sprite(id).expect("the sprite");
    sprite
        .memory()
        .into_iter()
        .find(|m| &m.learned == learned)
        .map_or(0.0, |m| m.amount)
}

/// A small field of grass with a sprite of its own genes on each tile, the
/// ones on `young` as newborns and the rest just grown up.
pub fn family(sprites: &[(Pos, &str)], young: &[Pos], seed: u64) -> World {
    family_in(builtin(), sprites, young, seed)
}

/// The same, with `data`.
pub fn family_in(data: DataPack, sprites: &[(Pos, &str)], young: &[Pos], seed: u64) -> World {
    WorldBuilder::new(&["........", "........", "........", "........"])
        .data(data)
        .sprites(sprites)
        .newborns(young)
        .seed(seed)
        .build()
}

/// Steps `world` until `found` picks something out of a tick's events, or
/// `ticks` ticks have passed.
pub fn until<T>(
    world: &mut World,
    ticks: u64,
    mut found: impl FnMut(&Event) -> Option<T>,
) -> Option<T> {
    for _ in 0..ticks {
        if let Some(found) = world.step().iter().find_map(&mut found) {
            return Some(found);
        }
    }
    None
}

/// An egg laid: the egg, who laid it, with whom, and on which tick.
pub struct Laying {
    pub egg: EntityId,
    pub by: EntityId,
    pub with: Option<EntityId>,
    pub tick: u64,
}

/// Steps `world` until an egg is laid.
pub fn laid(world: &mut World, ticks: u64) -> Option<Laying> {
    until(world, ticks, |event| match event.kind {
        EventKind::Laid { egg, by, with } => Some(Laying {
            egg,
            by,
            with,
            tick: event.tick,
        }),
        _ => None,
    })
}

/// Steps `world` until the egg `egg` hatches (M2 design §10.4), so the
/// young sprite it held, with its ID, can be looked at.
pub fn hatch(world: &mut World, egg: EntityId) {
    let hatched = until(world, 1_000, |event| match event.kind {
        EventKind::Hatched { id, .. } if id == egg => Some(()),
        _ => None,
    });
    hatched.expect("the egg hatched");
}

/// The changes to `physiology.ron` that turn copying errors off, so an
/// egg's genes are its parents' exactly, with no new flaws (M2 design §12).
pub const NO_COPYING_ERRORS: [(&str, &str); 2] = [
    ("mutation: (chance: 0.01,", "mutation: (chance: 0.0,"),
    ("flaws: (arise: 0.004,", "flaws: (arise: 0.0,"),
];

/// Steps `world` `ticks` times.
pub fn run_for(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
}

/// The Cursor's pet for `sprite` (design v21 §4.6), its feeling looking
/// `reach_back` ticks back for the sprite's latest attempt (§5.6).
pub fn pet(sprite: EntityId, reach_back: u64) -> Command {
    Command::Reward {
        sprite,
        amplified: false,
        reach_back,
    }
}

/// The Cursor's hug for `sprite`: an amplified pet (design v21 §4.6).
pub fn hug(sprite: EntityId, reach_back: u64) -> Command {
    Command::Reward {
        sprite,
        amplified: true,
        reach_back,
    }
}

/// The Cursor's zap for `sprite` (design v21 §4.6). Its feeling looks back
/// only the touch window (§5.6).
pub fn zap(sprite: EntityId) -> Command {
    Command::Correct {
        sprite,
        amplified: false,
    }
}

/// The Cursor's shock for `sprite`: an amplified zap (design v21 §4.6).
pub fn shock(sprite: EntityId) -> Command {
    Command::Correct {
        sprite,
        amplified: true,
    }
}
