//! Fast-forward (M2 design §16): a Lab world run with no screen, as fast
//! as the computer allows, for hours of play at 1×, then saved for the game
//! to open, with a population table and a genes table beside it. `main.rs`
//! runs it for `--fast-forward` and writes what it leaves behind; what it
//! runs, reports and tables is here, so it can be tested.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use terra_sim::{DeathCause, EntityId, EventKind, World, drift, gene_values, middle_values};

use crate::text::{capitalised, cause_name, thousands};

/// Ticks in an hour of play at 1×, 1.25 ticks a second (design §6.6).
pub const HOUR: u64 = 4_500;

/// How many rows a run reports: one each tenth of the way.
pub const ROWS: u64 = 10;

/// The most hours a run may be, so its ticks, counted in tenths for its
/// rows, still fit in a tick count: about 400 trillion.
pub const MOST_HOURS: u64 = u64::MAX / (HOUR * ROWS);

/// How many gene values the end of a run lists as moved most (M2 design
/// §16.4).
pub const MOVED_LISTED: usize = 5;

/// A world being fast-forwarded.
pub struct FastForward {
    world: World,
    /// The tick it started on, and the tick it runs to.
    start: u64,
    end: u64,
    /// How many rows it has reported.
    rows: u64,
    tally: Tally,
    genes: GeneTable,
}

impl FastForward {
    /// Fast-forwards `world` by `hours` of play at 1×.
    pub fn new(world: World, hours: u64) -> FastForward {
        let start = world.tick();
        let mut genes = GeneTable::new(&world);
        genes.take(&world);
        FastForward {
            start,
            end: start + hours * HOUR,
            rows: 0,
            tally: Tally::starting_from(&world),
            genes,
            world,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn into_world(self) -> World {
        self.world
    }

    /// The genes table so far: a row each hour it has run.
    pub fn genes(&self) -> &GeneTable {
        &self.genes
    }

    /// The deepest generation reached since it started.
    pub fn generation(&self) -> u32 {
        self.tally.deepest()
    }

    /// How many ticks it has run.
    pub fn ticks_run(&self) -> u64 {
        self.world.tick() - self.start
    }

    /// How many ticks it runs in all.
    pub fn ticks(&self) -> u64 {
        self.end - self.start
    }

    /// Whether it has run all its ticks.
    pub fn done(&self) -> bool {
        self.world.tick() >= self.end
    }

    /// The tick the next row is due on, or `None` once all have been: one
    /// each tenth of the way.
    pub fn next_row(&self) -> Option<u64> {
        (self.rows < ROWS).then(|| self.start + self.ticks() * (self.rows + 1) / ROWS)
    }

    /// Runs on by up to `ticks`, but no further than the next row, and
    /// takes that row once it's due. A run ends with its last row.
    pub fn advance(&mut self, ticks: u64) -> Option<Row> {
        let due = self.next_row()?;
        self.run_to(self.world.tick().saturating_add(ticks).min(due));
        (self.world.tick() >= due).then(|| self.take_row())
    }

    /// Runs on to `tick`, but no further than its end, taking a row of the
    /// genes table each hour.
    pub fn run_to(&mut self, tick: u64) {
        while self.world.tick() < tick.min(self.end) {
            for event in self.world.step() {
                self.tally.note(&event.kind);
            }
            if self.ticks_run().is_multiple_of(HOUR) {
                self.genes.take(&self.world);
            }
        }
    }

    /// Ends the genes table where the run stopped, if that was between
    /// hours.
    pub fn finish(&mut self) {
        if self.genes.last_tick() != Some(self.world.tick()) {
            self.genes.take(&self.world);
        }
    }

    /// Ends the run where it is (M2 design §16.3): a run `Ctrl+C` stopped
    /// before its end gives a last row, where it stopped, and the genes
    /// table ends there too.
    pub fn end(&mut self) -> Option<Row> {
        let stopped = (!self.done()).then(|| self.take_row());
        self.finish();
        stopped
    }

    /// How the world stands now, with the deaths since the row before, as
    /// the next row of the report.
    pub fn take_row(&mut self) -> Row {
        self.rows += 1;
        let count = self.world.population_now();
        Row {
            ticks_run: self.ticks_run(),
            young: count.young,
            adults: count.adults,
            old: count.old,
            eggs: count.eggs,
            generation: self.tally.deepest(),
            died: self.tally.take_deaths(),
        }
    }
}

/// What a run counts from the world's events: each sprite's generation,
/// and the deaths.
#[derive(Debug, Clone, Default)]
pub struct Tally {
    /// The generation of each sprite and egg in the world, counted from
    /// the sprites the run started with, which are the 0th.
    generations: BTreeMap<EntityId, u32>,
    /// The deepest generation reached.
    deepest: u32,
    /// Deaths since they were last taken, by cause.
    died: BTreeMap<DeathCause, u64>,
}

impl Tally {
    /// A tally from `world` as it is now: its sprites are the 0th
    /// generation, and its eggs the 1st.
    pub fn starting_from(world: &World) -> Tally {
        let sprites = world.sprites().map(|sprite| (sprite.id(), 0));
        let eggs = world.eggs().map(|egg| (egg.id(), 1));
        let generations: BTreeMap<EntityId, u32> = sprites.chain(eggs).collect();
        Tally {
            deepest: generations.values().copied().max().unwrap_or(0),
            generations,
            died: BTreeMap::new(),
        }
    }

    /// Takes in one of the world's events: an egg is a generation on from
    /// the deeper of its parents, a sprite the Cursor spawns is the 0th,
    /// and a death counts by its cause. The hatchling keeps its egg's ID,
    /// and so its generation.
    pub fn note(&mut self, kind: &EventKind) {
        match kind {
            EventKind::Laid { egg, by, with } => {
                let generation = [Some(by), with.as_ref()]
                    .into_iter()
                    .flatten()
                    .filter_map(|parent| self.generations.get(parent))
                    .max()
                    .map_or(1, |deepest| deepest + 1);
                self.generations.insert(*egg, generation);
                self.deepest = self.deepest.max(generation);
            }
            EventKind::Spawned { id, .. } => {
                self.generations.insert(*id, 0);
            }
            EventKind::Died { id, cause, .. } => {
                self.generations.remove(id);
                *self.died.entry(*cause).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    /// The deepest generation reached.
    pub fn deepest(&self) -> u32 {
        self.deepest
    }

    /// The deaths since they were last taken, by cause.
    pub fn take_deaths(&mut self) -> BTreeMap<DeathCause, u64> {
        std::mem::take(&mut self.died)
    }
}

/// How the world stood at a row of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The ticks run since the start.
    pub ticks_run: u64,
    pub young: u32,
    pub adults: u32,
    pub old: u32,
    pub eggs: u32,
    /// The deepest generation reached.
    pub generation: u32,
    /// Deaths since the row before, by cause.
    pub died: BTreeMap<DeathCause, u64>,
}

/// The report's column headings, over its rows.
pub const HEADER: &str =
    " Hours  Sprites  Young  Adults  Old  Eggs  Generation   Died  Old age  Hunger  Thirst  Hurt";

impl Row {
    /// The sprites, at every life stage: an egg isn't one until it
    /// hatches, as on the population graph (M2 design §14).
    pub fn sprites(&self) -> u32 {
        self.young + self.adults + self.old
    }

    /// The row as the report prints it, under `HEADER`.
    pub fn line(&self) -> String {
        let died = |cause| self.died.get(&cause).copied().unwrap_or(0);
        let hurt: u64 = self
            .died
            .iter()
            .filter(|(cause, _)| matches!(cause, DeathCause::HurtBy(_)))
            .map(|(_, n)| n)
            .sum();
        format!(
            "{:>6}  {:>7}  {:>5}  {:>6}  {:>3}  {:>4}  {:>10}  {:>5}  {:>7}  {:>6}  {:>6}  {:>4}",
            hours(self.ticks_run),
            thousands(self.sprites().into()),
            thousands(self.young.into()),
            thousands(self.adults.into()),
            thousands(self.old.into()),
            thousands(self.eggs.into()),
            self.generation,
            thousands(self.died.values().sum()),
            thousands(died(DeathCause::OldAge)),
            thousands(died(DeathCause::Starvation)),
            thousands(died(DeathCause::Dehydration)),
            thousands(hurt),
        )
    }
}

/// What the report says first: which world it runs, and for how long.
/// `loaded` is the save it came from, if any, and `preset` the preset a
/// new world was made from.
pub fn opening_line(world: &World, loaded: Option<&str>, preset: &str, hours: u64) -> String {
    let what = match loaded {
        Some(name) => format!("{name}, a Lab world at tick {}", thousands(world.tick())),
        None => format!("a new Lab world, seed {}, {preset} preset", world.seed()),
    };
    let unit = if hours == 1 { "hour" } else { "hours" };
    format!(
        "Fast-forwarding {what}, for {} {unit} ({} ticks).",
        thousands(hours),
        thousands(hours * HOUR)
    )
}

/// What the report says once a run ends: how far it got, how long that
/// took and how fast it ran. `stopped` is whether `Ctrl+C` stopped it
/// early.
pub fn closing_line(
    stopped: bool,
    ticks_run: u64,
    took: Duration,
    ticks_per_second: f64,
) -> String {
    let unit = if ticks_run == HOUR { "hour" } else { "hours" };
    let ran = hours(ticks_run);
    let took = spoken(took);
    let rate = thousands(ticks_per_second.round() as u64);
    if stopped {
        format!("Stopped at {ran} {unit}, after {took}, {rate} ticks a second.")
    } else {
        format!("Done: {ran} {unit} in {took}, {rate} ticks a second.")
    }
}

/// What the report says once the world is saved as `name` in the saves
/// folder, `folder` (M2 design §16.3).
pub fn saved_line(name: &str, folder: &Path) -> String {
    format!(
        "Saved as {name} in your saves folder ({}): the title screen's Continue opens it.",
        folder.display()
    )
}

/// What the report says of the tables written beside the save, by their
/// file names, or nothing if none could be (M2 design §16.3).
pub fn tables_line(written: &[&str]) -> Option<String> {
    (!written.is_empty()).then(|| format!("The tables are beside it: {}.", written.join(" and ")))
}

/// The name a run's save gets unless `--save` names it: what it's named
/// after, then the whole hours it ran, as `seed-7-200h`, `mira-50h`, or
/// `seed-7-143h` for a run stopped at 143.5 hours.
pub fn save_name(from: &str, ticks_run: u64) -> String {
    format!("{from}-{}h", ticks_run / HOUR)
}

/// The tables a run leaves beside its save, called `name` (M2 design
/// §16.4): each one's file name, and what's in it.
pub fn tables(name: &str, world: &World, genes: &GeneTable) -> [(String, String); 2] {
    [
        (format!("{name}-population.csv"), population_table(world)),
        (format!("{name}-genes.csv"), genes.table()),
    ]
}

/// `ticks` as hours of play at 1×, to the nearest tenth, with no ".0".
pub fn hours(ticks: u64) -> String {
    let tenths = (ticks * 10 + HOUR / 2) / HOUR;
    match tenths % 10 {
        0 => thousands(tenths / 10),
        tenth => format!("{}.{tenth}", thousands(tenths / 10)),
    }
}

/// The line that shows how a run is going, updated in place: how far it
/// has got, the generation, the sprites and the eggs, its speed, and about
/// how long it has left at that speed.
pub fn progress_line(run: &FastForward, ticks_per_second: f64) -> String {
    let count = run.world().population_now();
    let mut line = format!(
        "► {} hours · generation {} · {} sprites · {} eggs",
        hours(run.ticks_run()),
        run.generation(),
        thousands(count.sprites().into()),
        thousands(count.eggs.into()),
    );
    if ticks_per_second > 0.0 {
        let left = (run.ticks() - run.ticks_run()) as f64 / ticks_per_second;
        line += &format!(
            " · {} ticks a second · about {} left",
            thousands(ticks_per_second as u64),
            spoken(Duration::from_secs_f64(left)),
        );
    }
    line
}

/// `time` as the report says it: "45 seconds", "13 minutes", "2 hours 5
/// minutes".
pub fn spoken(time: Duration) -> String {
    let seconds = time.as_secs();
    let (hours, minutes) = (seconds / 3_600, seconds / 60 % 60);
    let plural = |n: u64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    match (hours, minutes) {
        (0, 0) => plural(seconds, "second"),
        (0, m) => plural(m, "minute"),
        (h, 0) => plural(h, "hour"),
        (h, m) => format!("{} {}", plural(h, "hour"), plural(m, "minute")),
    }
}

/// A table's hours: of play at 1×, to a hundredth, as a spreadsheet reads
/// a number.
fn table_hours(ticks: u64) -> String {
    let text = format!("{:.2}", ticks as f64 / HOUR as f64);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `fields` as a line of a table: commas between, and a field with a
/// comma or a quote in it quoted.
fn table_line(fields: impl IntoIterator<Item = String>) -> String {
    let fields: Vec<String> = fields
        .into_iter()
        .map(|field| {
            if field.contains([',', '"']) {
                format!("\"{}\"", field.replace('"', "\"\""))
            } else {
                field
            }
        })
        .collect();
    fields.join(",") + "\n"
}

/// What starts a table: the mark that tells a spreadsheet it's UTF-8, for
/// the arrows in the genes' names.
const UTF8: &str = "\u{feff}";

/// The population table (M2 design §16): a row for each count the world
/// keeps for its graph (M2 design §14), and one for the world as it is
/// now, with what happened since the row before: eggs laid and hatched,
/// sprites the Cursor spawned, and deaths, by cause. A cause has a column
/// once any sprite has died of it, and physiology's always do.
pub fn population_table(world: &World) -> String {
    let mut counts = world.population().to_vec();
    let now = world.population_now();
    if counts.last().is_none_or(|last| last.tick < now.tick) {
        counts.push(now);
    }
    let causes: BTreeSet<DeathCause> = DeathCause::PHYSIOLOGY
        .into_iter()
        .chain(
            counts
                .iter()
                .flat_map(|count| count.since.died.keys().copied()),
        )
        .collect();
    let headings = ["Hours", "Tick", "Eggs", "Young", "Adults", "Old"]
        .into_iter()
        .chain(["Laid", "Hatched", "Spawned", "Died"])
        .map(str::to_string)
        .chain(
            causes
                .iter()
                .map(|&cause| capitalised(&cause_name(cause, world.data()))),
        );
    let mut table = String::from(UTF8) + &table_line(headings);
    for count in &counts {
        let since = &count.since;
        let numbers = [count.eggs, count.young, count.adults, count.old]
            .into_iter()
            .chain([since.laid, since.hatched, since.spawned, since.deaths()])
            .chain(
                causes
                    .iter()
                    .map(|cause| since.died.get(cause).copied().unwrap_or(0)),
            )
            .map(|n| n.to_string());
        let row = [table_hours(count.tick), count.tick.to_string()]
            .into_iter()
            .chain(numbers);
        table += &table_line(row);
    }
    table
}

/// The genes table (M2 design §16.4): each of the starter genome's gene
/// values' middle among the living, a row an hour, with the starter's own
/// first.
#[derive(Debug, Clone)]
pub struct GeneTable {
    /// The starter's values, named, in its order: the table's columns.
    starter: Vec<(String, f32)>,
    rows: Vec<GeneRow>,
}

/// A row of the genes table.
#[derive(Debug, Clone)]
struct GeneRow {
    tick: u64,
    sprites: usize,
    /// Each value's middle, or none if no sprite was alive.
    middles: Option<Vec<Option<f32>>>,
}

impl GeneTable {
    /// A table with no rows yet, of the values of the starter genome in
    /// `world`'s data pack.
    pub fn new(world: &World) -> GeneTable {
        let data = world.data();
        GeneTable {
            starter: gene_values(&data.starter().genes(data), &data.starter().switches()),
            rows: Vec::new(),
        }
    }

    /// Takes a row from `world` as it is now.
    pub fn take(&mut self, world: &World) {
        self.rows.push(GeneRow {
            tick: world.tick(),
            sprites: world.sprites().count(),
            middles: middle_values(world, &self.starter),
        });
    }

    /// The tick of the last row, if there is one.
    pub fn last_tick(&self) -> Option<u64> {
        self.rows.last().map(|row| row.tick)
    }

    /// The table, for a spreadsheet: a column for each value, a row for
    /// the starter's, then a row for each time taken.
    pub fn table(&self) -> String {
        let headings = ["Hours", "Tick", "Sprites"]
            .into_iter()
            .map(str::to_string)
            .chain(self.starter.iter().map(|(name, _)| name.clone()));
        let mut table = String::from(UTF8) + &table_line(headings);
        let starter = self.starter.iter().map(|(_, value)| value.to_string());
        let blank = || String::new();
        table += &table_line(
            ["starter".to_string(), blank(), blank()]
                .into_iter()
                .chain(starter),
        );
        for row in &self.rows {
            let middles = self.starter.iter().enumerate().map(|(index, _)| {
                let middle = row.middles.as_ref().and_then(|middles| middles[index]);
                middle.map_or_else(blank, |value| value.to_string())
            });
            let first = [
                table_hours(row.tick),
                row.tick.to_string(),
                row.sprites.to_string(),
            ];
            table += &table_line(first.into_iter().chain(middles));
        }
        table
    }

    /// What the report ends with (M2 design §16.4): the `MOVED_LISTED`
    /// values whose middle moved furthest from the starter's by the last
    /// row, or why it lists none.
    pub fn ending(&self) -> Vec<String> {
        match self.rows.last().and_then(|row| row.middles.as_ref()) {
            None => vec!["No sprite is left alive to measure the genes of.".into()],
            Some(now) => moved_lines(&moved_most(&self.starter, now, MOVED_LISTED)),
        }
    }
}

/// The lines that list the values in `moved`, under a line that says what
/// they are, or the line that says none moved.
pub fn moved_lines(moved: &[Moved]) -> Vec<String> {
    if moved.is_empty() {
        return vec!["No gene value has moved a tenth from the starter's yet.".into()];
    }
    let width = moved
        .iter()
        .map(|m| m.name.chars().count())
        .max()
        .unwrap_or(0)
        .min(50);
    let heading = "Moved most from the starter's, as the middle value among the living:";
    std::iter::once(heading.to_string())
        .chain(moved.iter().map(|m| m.line(width)))
        .collect()
}

/// A gene value that moved from the starter's.
#[derive(Debug, Clone, PartialEq)]
pub struct Moved {
    pub name: String,
    pub starter: f32,
    pub now: f32,
}

impl Moved {
    /// How far it moved: halving as far as doubling, and crossing 0
    /// furthest of all.
    fn distance(&self) -> f32 {
        let ratio = self.now / self.starter;
        if ratio <= 0.0 {
            f32::INFINITY
        } else if ratio >= 1.0 {
            ratio - 1.0
        } else {
            1.0 / ratio - 1.0
        }
    }

    /// The line the report gives it, its name `width` wide: where it
    /// started, where it is, and by how much it went up or down.
    pub fn line(&self, width: usize) -> String {
        let change = (self.now - self.starter).abs() / self.starter.abs() * 100.0;
        let way = if self.now > self.starter {
            "up"
        } else {
            "down"
        };
        format!(
            "  {:<width$}  {:>8} → {:<8}  {way} {change:.0}%",
            self.name,
            short(self.starter),
            short(self.now)
        )
    }
}

/// Of the values in `starter`, the `most` whose value `now` moved furthest
/// from it, furthest first, of those that moved by a tenth or more, as G4
/// counts it (`drift`). A value the starter has at 0, or none alive has,
/// is left out.
pub fn moved_most(starter: &[(String, f32)], now: &[Option<f32>], most: usize) -> Vec<Moved> {
    let mut moved: Vec<Moved> = starter
        .iter()
        .zip(now)
        .filter_map(|((name, starter), now)| {
            let now = (*now)?;
            (*starter != 0.0 && drift(now / starter).is_some()).then(|| Moved {
                name: name.clone(),
                starter: *starter,
                now,
            })
        })
        .collect();
    moved.sort_by(|a, b| b.distance().total_cmp(&a.distance()));
    moved.truncate(most);
    moved
}

/// `value` to four figures, with no zeros on the end: `0.5`, `1235`.
fn short(value: f32) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let decimals = (3 - value.abs().log10().floor() as i32).max(0) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}
