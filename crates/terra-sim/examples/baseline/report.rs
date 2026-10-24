//! The baseline report (design §7.6): each criterion's numbers and verdict.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use terra_sim::{DataPack, DeathCause, LabRun, LifeStage, SoakRun, Verb, Window};

/// How many grown-up deaths, over all the seeds, A4 needs before it judges
/// what share hunger and thirst caused (design v37 §7.4).
const A4_FEWEST_DEATHS: u64 = 10;

/// The share of unattended newborns A4 asks to grow up, in the middle seed:
/// from the first to the second (M2 design §19.6).
const A4_UNATTENDED: (f64, f64) = (0.1, 0.3);

/// The least share of the young hatched in the viability run A4 asks to
/// grow up, over all the seeds (M2 design §19.6).
const A4_GUIDED: f64 = 0.6;

/// The most share of grown-up deaths A4 allows adults' hunger and thirst,
/// over all the seeds: under 1 in 20 (M2 design §19.6).
const A4_GROWN: f64 = 0.05;

/// A scenario's seeds, each with its run, or what it said when it panicked.
pub type SeedRuns = [(u64, Result<LabRun, String>)];

/// How a criterion stands against its pass mark (design §7.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Verdict {
    Met,
    /// Not met; `until` names the slice that's meant to meet it.
    NotMet {
        until: Option<String>,
    },
    NoData,
}

/// A baseline report (design §7.6): what one commit of `main` measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// The commit measured.
    pub commit: String,
    /// The commit of the report it's compared with, if there was one.
    pub compared_with: Option<String>,
    /// When it was measured, in local time, as the launcher gives it.
    pub measured: String,
    /// How long the run took.
    pub seconds: u64,
    /// How many seeds it ran, from 1.
    pub seeds: u64,
    /// How many sprites each seed of the viability run starts with.
    pub sprites: u64,
    /// A4's unattended run (M2 design §19.6). Reports from before Slice 29
    /// have none.
    #[serde(default)]
    pub unattended: Unattended,
    pub viability: Viability,
    pub a1: Behaviour,
    pub a2: Behaviour,
    pub a3: Behaviour,
    /// The soak (A7). Reports from before slice 17 have none.
    #[serde(default)]
    pub soak: Option<Soak>,
    pub broken: Vec<Broken>,
}

/// A number that moved since the previous report: what it is, and how each
/// report shows it, or none where a report doesn't have it.
#[derive(Debug, Clone, PartialEq)]
pub struct Moved {
    pub name: String,
    pub was: Option<String>,
    pub now: Option<String>,
}

impl Report {
    /// The report as RON, the file the next baseline run compares with.
    pub fn to_ron(&self) -> String {
        let pretty = ron::ser::PrettyConfig::default();
        ron::ser::to_string_pretty(self, pretty).expect("a report is plain data")
    }

    /// A report from the RON that `to_ron` wrote.
    pub fn from_ron(text: &str) -> Result<Report, String> {
        ron::from_str(text).map_err(|e| e.to_string())
    }

    /// The report as a page (design §7.6), given what `moved` since the
    /// report it's compared with, or none if there isn't one.
    pub fn markdown(&self, moved: Option<&[Moved]>) -> String {
        let mut page = format!("# Baseline report: `main` at {}\n\n", self.commit);
        page.push_str(&format!(
            "Measured {}, in {}, on seeds 1–{}.",
            self.measured,
            duration(self.seconds),
            self.seeds
        ));
        if let Some(previous) = &self.compared_with {
            page.push_str(&format!(" Compared with {previous}."));
        }
        page.push_str("\n\n");
        match moved {
            None => page.push_str(
                "This is the first baseline report: there's nothing to compare it with.\n\n",
            ),
            Some([]) => page.push_str(&format!(
                "Nothing moved since {}.\n\n",
                self.compared_with
                    .as_deref()
                    .unwrap_or("the previous report")
            )),
            Some(moved) => {
                page.push_str("## What moved\n\n");
                for m in moved {
                    let (was, now) = (m.was.as_deref(), m.now.as_deref());
                    page.push_str(&format!(
                        "- {}: {} → {}\n",
                        m.name,
                        was.unwrap_or("none"),
                        now.unwrap_or("none")
                    ));
                }
                page.push('\n');
            }
        }
        if !self.broken.is_empty() {
            page.push_str("## Broken\n\n");
            for b in &self.broken {
                page.push_str(&format!(
                    "- {}, seed {}: {}. Replay: `{}`\n",
                    b.scenario, b.seed, b.message, b.replay
                ));
            }
            page.push('\n');
        }

        let v = &self.viability;
        page.push_str(&self.a4_markdown());
        page.push_str("## The thorn trap (design §7.3)\n\n");
        page.push_str(&format!(
            "Deaths by thornbush in ticks 0–30,000: median {}. It has no pass mark.\n\n",
            number(v.thorn_trap.median)
        ));
        if !v.thorn_trap.per_seed.is_empty() {
            let (mut seeds, mut rule, mut deaths) = (
                "| Seed |".to_string(),
                "|---|".to_string(),
                "| Deaths |".to_string(),
            );
            for (seed, n) in &v.thorn_trap.per_seed {
                seeds.push_str(&format!(" {seed} |"));
                rule.push_str("---|");
                deaths.push_str(&format!(" {n} |"));
            }
            page.push_str(&format!("{seeds}\n{rule}\n{deaths}\n\n"));
        }

        page.push_str("## A1–A3 (design §7.3; CI is their judge)\n\n");
        page.push_str("| | Median | Control's median | Pass mark | Verdict |\n");
        page.push_str("|---|---|---|---|---|\n");
        for (what, _, mark, behaviour) in self.behaviours() {
            let mut median = number(behaviour.median);
            if behaviour.finished > 0 && (behaviour.finished as u64) < self.seeds {
                median.push_str(&format!(
                    ", over {} of {} seeds",
                    behaviour.finished, self.seeds
                ));
            }
            page.push_str(&format!(
                "| {what} | {median} | {} | {mark} | {} |\n",
                number(behaviour.control_median),
                behaviour.verdict
            ));
        }
        page.push('\n');

        if let Some(soak) = &self.soak {
            page.push_str(&soak.markdown());
        }

        page.push_str("## Each seed (ticks 0–50,000)\n\n");
        let verbs: BTreeSet<&String> = v.seeds.iter().flat_map(|row| row.verbs.keys()).collect();
        page.push_str("| Seed | Alive | Deaths |");
        for verb in &verbs {
            page.push_str(&format!(" {verb} |"));
        }
        page.push_str(&format!("\n|---|---|---|{}\n", "---|".repeat(verbs.len())));
        for row in &v.seeds {
            let deaths: Vec<String> = row
                .deaths
                .iter()
                .map(|(cause, n)| format!("{cause} {n}"))
                .collect();
            let deaths = if deaths.is_empty() {
                "none".to_string()
            } else {
                deaths.join(", ")
            };
            page.push_str(&format!(
                "| {} | {} of {} | {deaths} |",
                row.seed, row.alive, self.sprites
            ));
            for verb in &verbs {
                let n = row.verbs.get(*verb).copied().unwrap_or(0);
                page.push_str(&format!(" {n} |"));
            }
            page.push('\n');
        }
        page
    }

    /// A4's section of the page (design §7.4), which is G5 too (M2 design
    /// §6, §19.6): the unattended run, the young hatched in the viability
    /// run, and the grown-up deaths.
    fn a4_markdown(&self) -> String {
        let (v, u) = (&self.viability, &self.unattended);
        let mut page = String::from("## A4 and G5: viability (design §7.4, M2 design §6)\n\n");
        for (name, finished) in [("unattended", u.finished), ("viability", v.finished)] {
            if (finished as u64) < self.seeds {
                page.push_str(&format!(
                    "The {name} run: over the {finished} of {} seeds that finished.\n",
                    self.seeds
                ));
            }
        }
        if (u.finished as u64) < self.seeds || (v.finished as u64) < self.seeds {
            page.push('\n');
        }
        page.push_str("| | Measured | Pass mark | Verdict |\n|---|---|---|---|\n");
        let (low, high) = A4_UNATTENDED;
        page.push_str(&format!(
            "| Unattended newborns that grow up, middle seed (the unattended run) | {} \
             | {} to {} | {} |\n",
            percent(u.median),
            percent(Some(low)),
            percent(Some(high)),
            u.verdict
        ));
        let guided = &v.guided;
        page.push_str(&format!(
            "| Young hatched in the viability run that grow up, all seeds | {} | at least {} \
             | {} |\n",
            share_of(guided.grew_up, guided.grew_up + guided.died_young),
            percent(Some(A4_GUIDED)),
            guided.verdict
        ));
        let g = &v.grown_ups;
        let share = match g.deaths {
            0 => "none died".to_string(),
            n if n < A4_FEWEST_DEATHS => {
                format!("{} of {n}: too few deaths to judge", g.hunger_and_thirst)
            }
            n => share_of(g.hunger_and_thirst, n),
        };
        page.push_str(&format!(
            "| Adults' deaths of hunger or thirst, of grown-up deaths, all seeds | {share} \
             | under 1 in 20, once {A4_FEWEST_DEATHS} have died | {} |\n\n",
            g.verdict
        ));
        page.push_str(&format!(
            "Old sprites' deaths of hunger or thirst, reported apart: {} of the {} grown-up \
             deaths.",
            g.old_hunger_and_thirst, g.deaths
        ));
        if !u.died_of.is_empty() {
            let died_of: Vec<String> = u
                .died_of
                .iter()
                .map(|(cause, n)| format!("{cause} {n}"))
                .collect();
            page.push_str(&format!(
                " What unattended newborns died of, all seeds: {}.",
                died_of.join(", ")
            ));
        }
        page.push_str("\n\n");
        if !u.per_seed.is_empty() {
            let (mut seeds, mut rule, mut grew) = (
                "| Unattended run | ".to_string(),
                "|---|".to_string(),
                "| Grew up |".to_string(),
            );
            seeds.truncate(seeds.len() - 1);
            for &(seed, grew_up, died_young) in &u.per_seed {
                seeds.push_str(&format!(" Seed {seed} |"));
                rule.push_str("---|");
                grew.push_str(&format!(" {grew_up} of {} |", grew_up + died_young));
            }
            page.push_str(&format!("{seeds}\n{rule}\n{grew}\n\n"));
        }
        page
    }

    /// A1–A3: what each counts, what its control lacks, its pass mark, and
    /// what it measured.
    fn behaviours(&self) -> [(&'static str, &'static str, &'static str, &Behaviour); 3] {
        [
            (
                "A1: thornbush contacts",
                "without learning",
                "at most half the control's, which needs at least 20",
                &self.a1,
            ),
            (
                "A2: plays with a ball after training",
                "without the trainer",
                "at least 1.5× the control's, which needs at least 20",
                &self.a2,
            ),
            (
                "A3: hits on sprites after training",
                "without the trainer",
                "at most half the control's, which needs at least 20",
                &self.a3,
            ),
        ]
    }

    /// Every number in the report, by name and as it's shown, in the
    /// report's order.
    fn numbers(&self) -> Vec<(String, String)> {
        let (v, u) = (&self.viability, &self.unattended);
        let (g, guided) = (&v.grown_ups, &v.guided);
        let mut numbers = vec![(
            "A4: unattended newborns that grow up, middle seed".to_string(),
            percent(u.median),
        )];
        // A report from before Slice 29 counted something else under these
        // names, so it has none of them (M2 design §19.6).
        if v.by_stage {
            numbers.extend([
                (
                    "A4: young hatched in the viability run that grew up".into(),
                    format!(
                        "{} of {}",
                        guided.grew_up,
                        guided.grew_up + guided.died_young
                    ),
                ),
                (
                    "A4: grown-up deaths (adult or old)".into(),
                    g.deaths.to_string(),
                ),
                (
                    "A4: adults' deaths of hunger or thirst".into(),
                    g.hunger_and_thirst.to_string(),
                ),
                (
                    "A4: old sprites' deaths of hunger or thirst".into(),
                    g.old_hunger_and_thirst.to_string(),
                ),
            ]);
        }
        numbers.push((
            "Thorn trap: deaths by thornbush in ticks 0–30,000, median".into(),
            number(v.thorn_trap.median),
        ));
        for &(seed, grew_up, died_young) in &u.per_seed {
            numbers.push((
                format!("Unattended: seed {seed} grew up"),
                format!("{grew_up} of {}", grew_up + died_young),
            ));
        }
        for (what, control, _, behaviour) in self.behaviours() {
            numbers.push((format!("{what}, median"), number(behaviour.median)));
            numbers.push((
                format!("{what} {control}, median"),
                number(behaviour.control_median),
            ));
        }
        // Only the soak's verdict: it runs on a new seed each time, so its
        // counts always move.
        if let Some(soak) = &self.soak {
            numbers.push(("A7: the soak".into(), soak.verdict.to_string()));
        }
        for (seed, n) in &v.thorn_trap.per_seed {
            numbers.push((format!("Thorn trap: seed {seed}"), n.to_string()));
        }
        for row in &v.seeds {
            let seed = row.seed;
            numbers.push((
                format!("Seed {seed}: alive at tick 50,000"),
                row.alive.to_string(),
            ));
            for (cause, n) in &row.deaths {
                numbers.push((format!("Seed {seed}: died of {cause}"), n.to_string()));
            }
            for (verb, n) in &row.verbs {
                numbers.push((format!("Seed {seed}: {verb}"), n.to_string()));
            }
        }
        for broken in &self.broken {
            let name = format!("Broken: {} seed {}", broken.scenario, broken.seed);
            numbers.push((name, broken.message.clone()));
        }
        numbers
    }
}

/// Every number in `current` that `previous` showed differently or didn't
/// have, in `current`'s order, then any that `current` no longer has.
pub fn moved(previous: &Report, current: &Report) -> Vec<Moved> {
    let was: BTreeMap<String, String> = previous.numbers().into_iter().collect();
    let now = current.numbers();
    let mut moved: Vec<Moved> = now
        .iter()
        .filter(|(name, shown)| was.get(name) != Some(shown))
        .map(|(name, shown)| Moved {
            name: name.clone(),
            was: was.get(name).cloned(),
            now: Some(shown.clone()),
        })
        .collect();
    let named: BTreeSet<&String> = now.iter().map(|(name, _)| name).collect();
    for (name, shown) in previous.numbers() {
        if !named.contains(&name) {
            moved.push(Moved {
                name,
                was: Some(shown),
                now: None,
            });
        }
    }
    moved
}

/// What the viability run measured (design §7.6). A report from before
/// Slice 29 also has the share alive at tick 10,000, which A4 no longer
/// asks (M2 design §19.6), and reads without it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viability {
    /// How many seeds finished; the medians are theirs.
    pub finished: usize,
    /// Whether A4's counts are by life stage, as from Slice 29 (M2 design
    /// §19.6). A report from before has none: its grown-up counts were
    /// every death after tick 10,000, at any life stage, so what moved
    /// doesn't set them against these.
    #[serde(default)]
    pub by_stage: bool,
    /// A4: what became of the young hatched over the run, all the seeds
    /// together (M2 design §19.6). Reports from before Slice 29 have none.
    #[serde(default)]
    pub guided: Guided,
    /// A4: the grown-up deaths, over all the seeds (v37; by life stage from
    /// Slice 29). Reports from before v37 have none.
    #[serde(default)]
    pub grown_ups: GrownUps,
    /// The thorn trap (design §7.3): deaths by thornbush over ticks 0–30,000.
    pub thorn_trap: Count,
    /// A row for each seed that finished: who survived, what killed the rest, and the verbs.
    pub seeds: Vec<SeedRow>,
}

/// A4's grown-up deaths (M2 design §19.6): the deaths of sprites that had
/// grown up, adult or old, over the whole run and all the seeds that
/// finished. The world's first sprites start grown up, so theirs count from
/// the start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrownUps {
    pub deaths: u64,
    /// Adults' deaths of starvation or dehydration.
    pub hunger_and_thirst: u64,
    /// Old sprites' deaths of starvation or dehydration, reported apart.
    /// Reports from before Slice 29 have none.
    #[serde(default)]
    pub old_hunger_and_thirst: u64,
    /// Met while fewer than `A4_FEWEST_DEATHS` died, or when adults' hunger
    /// and thirst caused under 1 in 20 of them.
    pub verdict: Verdict,
}

impl Default for GrownUps {
    fn default() -> GrownUps {
        GrownUps {
            deaths: 0,
            hunger_and_thirst: 0,
            old_hunger_and_thirst: 0,
            verdict: Verdict::NoData,
        }
    }
}

/// A4's guided young (M2 design §19.6): of the young hatched over the
/// viability run, all the seeds that finished together, how many grew up and
/// how many died young. Those still growing at the end are in neither.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guided {
    pub grew_up: u64,
    pub died_young: u64,
    /// Met when at least `A4_GUIDED` of them grew up.
    pub verdict: Verdict,
}

impl Default for Guided {
    fn default() -> Guided {
        Guided {
            grew_up: 0,
            died_young: 0,
            verdict: Verdict::NoData,
        }
    }
}

/// A4's unattended run (M2 design §19.6): how many of each seed's newborns,
/// with no one to learn from, grew up and how many died young, and the
/// middle seed's share that grew up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unattended {
    /// How many seeds finished; the median is theirs.
    pub finished: usize,
    /// Each seed, how many grew up, and how many died young.
    pub per_seed: Vec<(u64, u64, u64)>,
    pub median: Option<f64>,
    /// Met when the middle seed's share is within `A4_UNATTENDED`.
    pub verdict: Verdict,
    /// What the newborns died of, over all the seeds, by name.
    pub died_of: BTreeMap<String, u64>,
}

impl Default for Unattended {
    fn default() -> Unattended {
        Unattended {
            finished: 0,
            per_seed: Vec::new(),
            median: None,
            verdict: Verdict::NoData,
            died_of: BTreeMap::new(),
        }
    }
}

/// The unattended run's report, from each seed's run or the reason it
/// broke (M2 design §19.6).
pub fn unattended(seeds: &SeedRuns, data: &DataPack) -> Unattended {
    let finished: Vec<(u64, &LabRun)> = seeds
        .iter()
        .filter_map(|(seed, run)| Some((*seed, run.as_ref().ok()?)))
        .collect();
    let mut died_of = BTreeMap::new();
    let mut per_seed = Vec::new();
    let mut shares = Vec::new();
    for (seed, run) in &finished {
        let whole = window(run, 6_000);
        per_seed.push((*seed, whole.first.grew_up, whole.first.died_young));
        shares.extend(whole.first.share());
        for (&(stage, cause), &n) in &whole.stage_deaths {
            if stage == LifeStage::Young {
                *died_of.entry(cause_name(cause, data)).or_insert(0) += n;
            }
        }
    }
    let median = median_of(&shares);
    let (low, high) = A4_UNATTENDED;
    let verdict = match median {
        None => Verdict::NoData,
        Some(share) if (low..=high).contains(&share) => Verdict::Met,
        Some(_) => Verdict::NotMet { until: None },
    };
    Unattended {
        finished: finished.len(),
        per_seed,
        median,
        verdict,
        died_of,
    }
}

/// One seed's 50,000 ticks: how many of its first sprites survived, what
/// killed whom, and how often each verb was applied, each by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeedRow {
    pub seed: u64,
    pub alive: u64,
    pub deaths: BTreeMap<String, u64>,
    pub verbs: BTreeMap<String, u64>,
}

/// A number counted on each seed, and its median. It has no pass mark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Count {
    /// Each seed and its count.
    pub per_seed: Vec<(u64, u64)>,
    pub median: Option<f64>,
}

/// The viability run's report, from each seed's run or the reason it broke.
/// Every seed starts with `sprites` sprites.
pub fn viability(seeds: &SeedRuns, sprites: u64, data: &DataPack) -> Viability {
    let finished: Vec<(u64, &LabRun)> = seeds
        .iter()
        .filter_map(|(seed, run)| Some((*seed, run.as_ref().ok()?)))
        .collect();
    // The young hatched, and the grown-up deaths, by life stage (M2 design
    // §19.6).
    let need =
        |cause: DeathCause| matches!(cause, DeathCause::Starvation | DeathCause::Dehydration);
    let (mut grew_up, mut died_young) = (0, 0);
    let (mut deaths, mut hunger_and_thirst, mut old_hunger_and_thirst) = (0, 0, 0);
    for (_, run) in &finished {
        let whole = window(run, 50_000);
        grew_up += whole.later.grew_up;
        died_young += whole.later.died_young;
        for (&(stage, cause), &n) in &whole.stage_deaths {
            match stage {
                LifeStage::Young => {}
                LifeStage::Adult => {
                    deaths += n;
                    if need(cause) {
                        hunger_and_thirst += n;
                    }
                }
                LifeStage::Old => {
                    deaths += n;
                    if need(cause) {
                        old_hunger_and_thirst += n;
                    }
                }
            }
        }
    }
    let verdict = match deaths {
        _ if finished.is_empty() => Verdict::NoData,
        n if n < A4_FEWEST_DEATHS => Verdict::Met,
        n if (hunger_and_thirst as f64) < A4_GROWN * n as f64 => Verdict::Met,
        _ => Verdict::NotMet { until: None },
    };
    let guided = match grew_up + died_young {
        0 => Verdict::NoData,
        n if grew_up as f64 >= A4_GUIDED * n as f64 => Verdict::Met,
        _ => Verdict::NotMet { until: None },
    };
    let thornbush_deaths: Vec<(u64, u64)> = finished
        .iter()
        .map(|(seed, run)| {
            let deaths = &window(run, 30_000).deaths;
            let by_thornbush = deaths
                .iter()
                .filter(|(cause, _)| match cause {
                    DeathCause::HurtBy(id) => data.object_type_name(*id) == Some("thornbush"),
                    _ => false,
                })
                .map(|(_, n)| n)
                .sum();
            (*seed, by_thornbush)
        })
        .collect();
    let counts: Vec<f64> = thornbush_deaths.iter().map(|&(_, n)| n as f64).collect();
    let rows = finished
        .iter()
        .map(|(seed, run)| {
            let all = window(run, 50_000);
            let mut verbs = BTreeMap::new();
            for (&(verb, _), n) in &all.applied {
                *verbs.entry(verb_name(verb)).or_insert(0) += n;
            }
            SeedRow {
                seed: *seed,
                alive: sprites.saturating_sub(all.first_died),
                deaths: all
                    .deaths
                    .iter()
                    .map(|(&cause, &n)| (cause_name(cause, data), n))
                    .collect(),
                verbs,
            }
        })
        .collect();
    Viability {
        finished: finished.len(),
        by_stage: true,
        seeds: rows,
        guided: Guided {
            grew_up,
            died_young,
            verdict: guided,
        },
        grown_ups: GrownUps {
            deaths,
            hunger_and_thirst,
            old_hunger_and_thirst,
            verdict,
        },
        thorn_trap: Count {
            median: median_of(&counts),
            per_seed: thornbush_deaths,
        },
    }
}

/// One of the behaviour scenarios A1–A3 (design §7.3): its runs' median,
/// its controls', and its verdict, by the same pass marks and on the same
/// seeds as CI's acceptance tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Behaviour {
    /// How many seeds finished; the medians are theirs.
    pub finished: usize,
    pub median: Option<f64>,
    pub control_median: Option<f64>,
    pub verdict: Verdict,
}

/// A1: thornbush contacts (Eat, Play and Hit) over the whole run, at most
/// half the control's.
pub fn a1(seeds: &SeedRuns, data: &DataPack) -> Behaviour {
    let contacts = |windows: &[Window]| {
        let verbs = [Verb::Eat, Verb::Play, Verb::Hit];
        windows
            .iter()
            .flat_map(|w| verbs.map(|verb| w.applied_on(verb, "thornbush", data)))
            .sum()
    };
    let mut a1 = behaviour(seeds, contacts, |run, control| run <= control / 2.0);
    // A dead learner touches no thornbushes, which would pass hollowly, so
    // CI fails A1 when one dies (§7.3).
    let died = seeds
        .iter()
        .filter_map(|(_, run)| run.as_ref().ok())
        .any(|run| run.windows.iter().any(|w| !w.deaths.is_empty()));
    if died {
        a1.verdict = Verdict::NotMet { until: None };
    }
    a1
}

/// A2: applied Plays on a ball after training, at least 1.5× the control's.
pub fn a2(seeds: &SeedRuns, data: &DataPack) -> Behaviour {
    let plays = |windows: &[Window]| after_training(windows).applied_on(Verb::Play, "ball", data);
    behaviour(seeds, plays, |run, control| run >= 1.5 * control)
}

/// A3: applied Hits on a sprite after training, at most half the control's.
pub fn a3(seeds: &SeedRuns, data: &DataPack) -> Behaviour {
    let hits = |windows: &[Window]| after_training(windows).applied_on(Verb::Hit, "sprite", data);
    behaviour(seeds, hits, |run, control| run <= 0.5 * control)
}

/// A behaviour's medians of what `count` counts in each seed's windows, and
/// whether they pass: `passes` given the run's and the control's, and a
/// control of at least 20, or the scenario is badly calibrated (§7.3).
fn behaviour(
    seeds: &SeedRuns,
    count: impl Fn(&[Window]) -> u64,
    passes: impl Fn(f64, f64) -> bool,
) -> Behaviour {
    let finished: Vec<&LabRun> = seeds
        .iter()
        .filter_map(|(_, run)| run.as_ref().ok())
        .collect();
    let runs: Vec<f64> = finished
        .iter()
        .map(|run| count(&run.windows) as f64)
        .collect();
    let controls: Vec<f64> = finished
        .iter()
        .filter_map(|run| run.control.as_deref())
        .map(|windows| count(windows) as f64)
        .collect();
    let (median, control_median) = (median_of(&runs), median_of(&controls));
    // CI's tests panic on a broken seed, so a scenario with one isn't met.
    let all_finished = finished.len() == seeds.len();
    let verdict = match (median, control_median) {
        _ if !all_finished => Verdict::NotMet { until: None },
        (Some(run), Some(control)) if control >= 20.0 && passes(run, control) => Verdict::Met,
        (Some(_), Some(_)) => Verdict::NotMet { until: None },
        _ => Verdict::NoData,
    };
    Behaviour {
        finished: finished.len(),
        median,
        control_median,
        verdict,
    }
}

/// The window A2 and A3 measure in: ticks 10,000–20,000, after training
/// and its washout (design §7.3).
fn after_training(windows: &[Window]) -> &Window {
    windows
        .iter()
        .find(|w| w.from == 10_000 && w.to == 20_000)
        .expect("A2 and A3 count ticks 10,000 to 20,000")
}

/// A seed that panicked or broke an invariant (design §7.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Broken {
    /// The lab scenario, by the name of its file in `scenarios/`.
    pub scenario: String,
    pub seed: u64,
    /// What it said, which names the tick for a broken invariant (§7.1).
    pub message: String,
    /// The command that replays the seed, with the self-check on.
    pub replay: String,
}

/// The seeds of `scenario` that broke.
pub fn broken(scenario: &str, seeds: &SeedRuns) -> Vec<Broken> {
    seeds
        .iter()
        .filter_map(|(seed, run)| {
            let message = run.as_ref().err()?;
            Some(Broken {
                scenario: scenario.into(),
                seed: *seed,
                message: message.clone(),
                replay: format!(
                    "cargo run --profile baseline -p terra-sim --example lab -- \
                     scenarios/{scenario}.ron --seed {seed}"
                ),
            })
        })
        .collect()
}

/// The soak (design §7.4 A7, §7.6): a million ticks of the default world
/// with a random script of the Cursor's commands, on a new seed each run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Soak {
    pub seed: u64,
    /// How many ticks it was to run.
    pub ticks: u64,
    /// Met when it finished with no panic and no broken invariant.
    pub verdict: Verdict,
    /// What it did, if it finished.
    pub finished: Option<SoakCounts>,
}

/// What a soak that finished did, each by name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoakCounts {
    /// The commands its script gave.
    pub given: BTreeMap<String, u64>,
    /// The commands the world refused.
    pub refused: BTreeMap<String, u64>,
    pub deaths: BTreeMap<String, u64>,
    /// Sprites alive at the end.
    pub sprites: usize,
    /// The most sprites alive at once.
    pub most_sprites: usize,
    /// Objects in the world at the end.
    pub objects: usize,
}

/// The soak's report, from its run or what it said when it broke.
pub fn soak(seed: u64, ticks: u64, run: &Result<SoakRun, String>, data: &DataPack) -> Soak {
    let named = |counts: &BTreeMap<&'static str, u64>| -> BTreeMap<String, u64> {
        counts.iter().map(|(&name, &n)| (name.into(), n)).collect()
    };
    let finished = run.as_ref().ok().map(|run| SoakCounts {
        given: named(&run.given),
        refused: named(&run.refused),
        deaths: run
            .deaths
            .iter()
            .map(|(&cause, &n)| (cause_name(cause, data), n))
            .collect(),
        sprites: run.sprites,
        most_sprites: run.most_sprites,
        objects: run.objects,
    });
    let verdict = match finished {
        Some(_) => Verdict::Met,
        None => Verdict::NotMet { until: None },
    };
    Soak {
        seed,
        ticks,
        verdict,
        finished,
    }
}

impl Soak {
    /// The soak's section of the page.
    fn markdown(&self) -> String {
        let mut page = String::from("## A7: the soak (design §7.4)\n\n");
        page.push_str(&format!(
            "The default world for {} ticks on seed {}, with a random script of the Cursor's \
             commands.\n\n",
            thousands(self.ticks),
            self.seed
        ));
        page.push_str("| | Pass mark | Verdict |\n|---|---|---|\n");
        page.push_str(&format!(
            "| Panics and broken invariants | none | {} |\n\n",
            self.verdict
        ));
        let Some(counts) = &self.finished else {
            page.push_str("It broke: see Broken, above.\n\n");
            return page;
        };
        let deaths: Vec<String> = counts
            .deaths
            .iter()
            .map(|(cause, n)| format!("{cause} {n}"))
            .collect();
        let deaths = if deaths.is_empty() {
            "none".to_string()
        } else {
            deaths.join(", ")
        };
        page.push_str(&format!(
            "It ended with {} sprites alive, and at most {} at once, and {} objects. \
             Deaths: {deaths}.\n\n",
            counts.sprites, counts.most_sprites, counts.objects
        ));
        page.push_str("| Command | Given | Refused |\n|---|---|---|\n");
        for (name, given) in &counts.given {
            let refused = counts.refused.get(name).copied().unwrap_or(0);
            page.push_str(&format!("| {name} | {given} | {refused} |\n"));
        }
        page.push('\n');
        page
    }
}

/// The soak, if it broke, with the command that runs it again.
pub fn broken_soak(soak: &Soak, run: &Result<SoakRun, String>) -> Option<Broken> {
    let message = run.as_ref().err()?;
    Some(Broken {
        scenario: "soak".into(),
        seed: soak.seed,
        message: message.clone(),
        replay: format!(
            "cargo run --profile baseline -p terra-sim --example soak -- --seed {} --ticks {}",
            soak.seed, soak.ticks
        ),
    })
}

/// A whole number with commas between its thousands: "1,000,000".
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The run's window over ticks 0 to `to`, as `scenarios/viability.ron` and
/// `scenarios/unattended.ron` set them.
fn window(run: &LabRun, to: u64) -> &Window {
    run.windows
        .iter()
        .find(|w| w.from == 0 && w.to == to)
        .unwrap_or_else(|| panic!("the run counts ticks 0 to {to}"))
}

/// A verb by name, as the lab runner writes it: "eat".
fn verb_name(verb: Verb) -> String {
    format!("{verb:?}").to_lowercase()
}

/// A cause of death by name, as the lab runner writes it: "hurt by
/// thornbush" names the object type.
fn cause_name(cause: DeathCause, data: &DataPack) -> String {
    match cause {
        DeathCause::Starvation => "starvation".into(),
        DeathCause::Dehydration => "dehydration".into(),
        DeathCause::OldAge => "old age".into(),
        DeathCause::HurtBy(id) => format!("hurt by {}", data.object_type_name(id).unwrap_or("?")),
    }
}

impl std::fmt::Display for Verdict {
    /// "met", "not met yet (#18)", "not met" or "no data".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Met => f.write_str("met"),
            Verdict::NotMet { until: Some(slice) } => write!(f, "not met yet ({slice})"),
            Verdict::NotMet { until: None } => f.write_str("not met"),
            Verdict::NoData => f.write_str("no data"),
        }
    }
}

/// How long a run took: "15 min 0 s", or "42 s".
fn duration(seconds: u64) -> String {
    match seconds {
        0..60 => format!("{seconds} s"),
        _ => format!("{} min {} s", seconds / 60, seconds % 60),
    }
}

/// `part` of `whole` as a report shows it: "3 of 40: 7.5%", or "none" of
/// none.
fn share_of(part: u64, whole: u64) -> String {
    match whole {
        0 => "none".into(),
        _ => format!(
            "{part} of {whole}: {}",
            percent(Some(part as f64 / whole as f64))
        ),
    }
}

/// A share as a report shows it: "80%", "78.3%", or "no data".
fn percent(share: Option<f64>) -> String {
    match share {
        None => "no data".into(),
        Some(share) => {
            // Rounded first, so 28.000000000000004 is "28%", not "28.0%".
            let percent = format!("{:.1}", share * 100.0);
            format!("{}%", percent.strip_suffix(".0").unwrap_or(&percent))
        }
    }
}

/// A median as a report shows it: "2", "1.5", or "no data".
fn number(median: Option<f64>) -> String {
    match median {
        None => "no data".into(),
        Some(n) if n.fract() == 0.0 => format!("{n:.0}"),
        Some(n) => format!("{n:.1}"),
    }
}

/// The median of `values`, or none if there are none.
fn median_of(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(sorted[n / 2]),
        _ => Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use terra_sim::{Childhood, DeathCause, LifeStage, Verb};

    fn data() -> DataPack {
        DataPack::builtin().expect("the built-in data pack is valid")
    }

    /// A window over ticks `from` to `to` that counted `deaths`.
    fn deaths_window(from: u64, to: u64, deaths: &[(DeathCause, u64)]) -> Window {
        Window {
            from,
            to,
            deaths: deaths.iter().copied().collect(),
            ..Default::default()
        }
    }

    /// Deaths of sprites at a life stage, of a cause, and how many.
    type Dying = [(LifeStage, DeathCause, u64)];

    /// A window over ticks `from` to `to` that counted `deaths`, each at
    /// the life stage it gives.
    fn dying_window(from: u64, to: u64, deaths: &Dying) -> Window {
        let mut window = deaths_window(from, to, &[]);
        for &(stage, cause, n) in deaths {
            *window.deaths.entry(cause).or_insert(0) += n;
            *window.stage_deaths.entry((stage, cause)).or_insert(0) += n;
        }
        window
    }

    /// A viability run's seed, with the deaths in its first 30,000 ticks and
    /// those, by life stage, in its first 50,000.
    fn viability_run(by_30k: &[(DeathCause, u64)], by_50k: &Dying) -> LabRun {
        LabRun {
            windows: vec![
                deaths_window(0, 30_000, by_30k),
                dying_window(0, 50_000, by_50k),
            ],
            ..Default::default()
        }
    }

    /// Ten seeds whose first 50,000 ticks saw these deaths, by seed.
    fn seeds_dying(deaths: [&Dying; 10]) -> Vec<(u64, Result<LabRun, String>)> {
        (1..=10)
            .zip(deaths)
            .map(|(seed, deaths)| (seed, Ok(viability_run(&[], deaths))))
            .collect()
    }

    /// An unattended run's seed, in which `grew_up` of its first sprites
    /// grew up and the rest of 30 died young of thirst.
    fn unattended_run(grew_up: u64) -> LabRun {
        let died_young = 30 - grew_up;
        let young = [(LifeStage::Young, DeathCause::Dehydration, died_young)];
        let mut window = dying_window(0, 6_000, &young);
        window.first = Childhood {
            grew_up,
            died_young,
        };
        LabRun {
            windows: vec![window],
            ..Default::default()
        }
    }

    /// Ten seeds of the unattended run, with how many grew up in each.
    fn unattended_seeds(grew_up: [u64; 10]) -> Vec<(u64, Result<LabRun, String>)> {
        (1..=10)
            .zip(grew_up)
            .map(|(seed, grew_up)| (seed, Ok(unattended_run(grew_up))))
            .collect()
    }

    #[test]
    fn a4_unattended_is_met_when_the_middle_seed_raises_10_to_30_percent() {
        // M2 design §19.6: of newborns no one looks after, 10% to 30% grow
        // up, in the middle seed of ten.
        let data = data();
        // Sorted, 3 3 4 5 6 6 8 9 10 12 of 30: the middle two are 6 and 6.
        let met = unattended(&unattended_seeds([6, 3, 10, 4, 6, 12, 5, 3, 8, 9]), &data);
        assert_eq!(met.median, Some(0.2));
        assert_eq!(met.verdict, Verdict::Met);
        assert_eq!(met.per_seed[0], (1, 6, 24));
        assert_eq!(met.died_of, BTreeMap::from([("dehydration".into(), 234)]));
        let edges = unattended(&unattended_seeds([3; 10]), &data);
        assert_eq!(edges.verdict, Verdict::Met, "10% is in the band");
        let too_few = unattended(&unattended_seeds([2; 10]), &data);
        assert_eq!(too_few.verdict, Verdict::NotMet { until: None });
        let too_many = unattended(&unattended_seeds([10; 10]), &data);
        assert_eq!(too_many.verdict, Verdict::NotMet { until: None });
        assert_eq!(unattended(&[], &data).verdict, Verdict::NoData);
    }

    #[test]
    fn a4_guided_young_are_met_when_at_least_60_percent_of_the_hatched_grow_up() {
        // M2 design §19.6: the young hatched in the viability run, all the
        // seeds together; those still growing at the end aren't counted.
        let hatched = |grew_up, died_young| -> Vec<(u64, Result<LabRun, String>)> {
            (1..=10)
                .map(|seed| {
                    let mut run = viability_run(&[], &[]);
                    run.windows[1].later = Childhood {
                        grew_up,
                        died_young,
                    };
                    (seed, Ok(run))
                })
                .collect()
        };
        let met = viability(&hatched(6, 4), 30, &data()).guided;
        assert_eq!((met.grew_up, met.died_young), (60, 40));
        assert_eq!(met.verdict, Verdict::Met);
        let short = viability(&hatched(5, 4), 30, &data()).guided;
        assert_eq!(short.verdict, Verdict::NotMet { until: None });
        let none = viability(&hatched(0, 0), 30, &data()).guided;
        assert_eq!(none.verdict, Verdict::NoData);
    }

    #[test]
    fn a4_grown_up_deaths_are_adults_and_old_sprites_pooled_over_the_seeds() {
        // M2 design §19.6, A4: adults' hunger and thirst cause under 1 in 20
        // of grown-up deaths, judged by life stage, counted over all the
        // seeds together. Young deaths don't count, and old sprites' hunger
        // and thirst is reported apart.
        use DeathCause::{Dehydration, OldAge, Starvation};
        use LifeStage::{Adult, Old, Young};
        let deaths: &Dying = &[
            (Young, Dehydration, 5),
            (Adult, Dehydration, 1),
            (Old, Starvation, 2),
            (Old, OldAge, 7),
        ];
        let grown_ups = viability(&seeds_dying([deaths; 10]), 30, &data()).grown_ups;
        assert_eq!(
            (
                grown_ups.deaths,
                grown_ups.hunger_and_thirst,
                grown_ups.old_hunger_and_thirst
            ),
            (100, 10, 20)
        );
        assert_eq!(grown_ups.verdict, Verdict::NotMet { until: None });
    }

    #[test]
    fn a4_grown_ups_are_met_under_1_in_20_once_ten_have_died() {
        use DeathCause::{OldAge, Starvation};
        use LifeStage::{Adult, Old};
        let one_in_20: &Dying = &[(Adult, Starvation, 1), (Old, OldAge, 19)];
        let at = viability(&seeds_dying([one_in_20; 10]), 30, &data());
        assert_eq!(at.grown_ups.deaths, 200);
        assert_eq!(at.grown_ups.verdict, Verdict::NotMet { until: None });
        let under: &Dying = &[(Adult, Starvation, 1), (Old, OldAge, 20)];
        let under_1_in_20 = viability(&seeds_dying([under; 10]), 30, &data());
        assert_eq!(under_1_in_20.grown_ups.verdict, Verdict::Met);
    }

    #[test]
    fn a4_grown_ups_are_met_with_fewer_than_ten_deaths_whatever_killed_them() {
        // Design v37 §7.4: a share of a handful of deaths means little, and
        // hardly anyone dying is what viability asks.
        use LifeStage::Adult;
        let mut deaths: [&Dying; 10] = [&[]; 10];
        deaths[0] = &[(Adult, DeathCause::Dehydration, 5)];
        deaths[1] = &[(Adult, DeathCause::Starvation, 4)];
        let nine = viability(&seeds_dying(deaths), 30, &data()).grown_ups;
        assert_eq!((nine.deaths, nine.hunger_and_thirst), (9, 9));
        assert_eq!(nine.verdict, Verdict::Met);
        deaths[2] = &[(Adult, DeathCause::Dehydration, 1)];
        let ten = viability(&seeds_dying(deaths), 30, &data()).grown_ups;
        assert_eq!(ten.verdict, Verdict::NotMet { until: None });
    }

    #[test]
    fn the_thorn_trap_counts_thornbush_deaths_in_the_first_30000_ticks() {
        // Design §7.3: the thorn trap counts deaths by thornbush over ticks
        // 0–30,000; later ones, and other causes, don't count.
        let data = data();
        assert_eq!(data.object_type_name(3), Some("thornbush"), "objects.ron");
        let thornbush = DeathCause::HurtBy(3);
        let seeds: Vec<_> = (1..=10)
            .zip([1, 2, 0, 3, 1, 2, 4, 0, 1, 2])
            .map(|(seed, thornbush_deaths)| {
                let by_30k = [(thornbush, thornbush_deaths), (DeathCause::Dehydration, 2)];
                let by_50k = [
                    (LifeStage::Adult, thornbush, thornbush_deaths + 5),
                    (LifeStage::Adult, DeathCause::Dehydration, 2),
                ];
                (seed, Ok(viability_run(&by_30k, &by_50k)))
            })
            .collect();
        let thorn_trap = viability(&seeds, 30, &data).thorn_trap;
        assert_eq!(
            thorn_trap.per_seed,
            vec![
                (1, 1),
                (2, 2),
                (3, 0),
                (4, 3),
                (5, 1),
                (6, 2),
                (7, 4),
                (8, 0),
                (9, 1),
                (10, 2)
            ]
        );
        // Sorted, 0 0 1 1 1 2 2 2 3 4: halfway between the middle two.
        assert_eq!(thorn_trap.median, Some(1.5));
    }

    #[test]
    fn a_seed_that_panicked_is_broken_with_how_to_replay_it_and_the_rest_are_still_reported() {
        // Design §7.6: broken means a panic or a broken invariant, given with
        // the seed, what it said (naming the tick, for a broken invariant)
        // and the command that replays it.
        let message = "tick 4,312: no tile holds more than one object".to_string();
        let seeds = vec![
            (1, Ok(viability_run(&[], &[]))),
            (2, Err(message.clone())),
            (3, Ok(viability_run(&[], &[]))),
        ];
        assert_eq!(
            broken("viability", &seeds),
            vec![Broken {
                scenario: "viability".into(),
                seed: 2,
                message,
                replay: "cargo run --profile baseline -p terra-sim --example lab -- \
                         scenarios/viability.ron --seed 2"
                    .into(),
            }]
        );
        let rows = viability(&seeds, 30, &data()).seeds;
        let reported: Vec<u64> = rows.iter().map(|row| row.seed).collect();
        assert_eq!(reported, vec![1, 3]);
    }

    #[test]
    fn each_seed_has_a_row_of_who_survived_what_killed_the_rest_and_the_verbs() {
        // Design §7.6: per seed, how many of the first sprites survived the
        // 50,000 ticks, what killed whom, and how often each verb was
        // applied, whatever its target. Of the three deaths, the young
        // sprite was hatched in the world, so 28 of the 30 first sprites
        // are left.
        let data = data();
        assert_eq!(data.object_type_name(2), Some("berry"), "objects.ron");
        assert_eq!(data.object_type_name(3), Some("thornbush"), "objects.ron");
        let mut run = viability_run(
            &[],
            &[
                (LifeStage::Adult, DeathCause::Dehydration, 2),
                (LifeStage::Young, DeathCause::HurtBy(3), 1),
            ],
        );
        run.windows[1].first_died = 2;
        run.windows[1].applied = BTreeMap::from([
            ((Verb::Eat, Some(2)), 10),
            ((Verb::Eat, Some(3)), 2),
            ((Verb::Rest, None), 50),
            ((Verb::Wander, None), 100),
        ]);
        let seeds = vec![(7, Ok(run))];
        let rows = viability(&seeds, 30, &data).seeds;
        assert_eq!(
            rows,
            vec![SeedRow {
                seed: 7,
                alive: 28,
                deaths: BTreeMap::from([
                    ("dehydration".into(), 2),
                    ("hurt by thornbush".into(), 1)
                ]),
                verbs: BTreeMap::from([
                    ("eat".into(), 12),
                    ("rest".into(), 50),
                    ("wander".into(), 100)
                ]),
            }]
        );
    }

    #[test]
    fn a4_grown_ups_are_met_when_no_sprite_died_and_have_no_data_when_no_seed_finished() {
        let none_died = viability(&seeds_dying([&[]; 10]), 30, &data()).grown_ups;
        assert_eq!(none_died.deaths, 0);
        assert_eq!(none_died.verdict, Verdict::Met);
        assert_eq!(
            viability(&[], 30, &data()).grown_ups.verdict,
            Verdict::NoData
        );
    }

    #[test]
    fn a_report_from_before_the_grown_up_rule_still_reads() {
        // Older reports have a median share and a count of seeds with deaths
        // in place of the grown-ups' counts.
        let text = sample().to_ron();
        let start = text.find("grown_ups:").expect("the grown-ups' field");
        let end = start + text[start..].find("),").expect("its end") + 2;
        let text = format!(
            "{}hunger_and_thirst: (median: None, verdict: NoData), seeds_with_deaths: 0,{}",
            &text[..start],
            &text[end..]
        );
        assert!(!text.contains("grown_ups"), "{text}");
        let report = Report::from_ron(&text).expect("an older report reads");
        assert_eq!(report.viability.grown_ups.verdict, Verdict::NoData);
    }

    /// `text` without the field `name` and its value, a bracketed one.
    fn without_field(text: &str, name: &str) -> String {
        let start = text.find(&format!("{name}:")).expect("the field");
        let mut depth = 0;
        let mut end = start;
        for (at, c) in text[start..].char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + at + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end + text[end..].find(',').expect("a comma after it") + 1;
        format!("{}{}", &text[..start], &text[end..])
    }

    #[test]
    fn a_report_from_before_slice_29_still_reads() {
        // Older reports have A4's share alive at tick 10,000 in place of the
        // unattended run and the guided young, no old sprites' count, and
        // grown-up counts not by life stage.
        let text = without_field(&sample().to_ron(), "unattended");
        let text = without_field(&text, "guided");
        let mut text = text.replacen(
            "viability: (",
            "viability: (survival: (median: Some(0.9), verdict: Met), ",
            1,
        );
        for scalar in ["old_hunger_and_thirst:", "by_stage:"] {
            let start = text.find(scalar).expect("the field");
            let end = start + text[start..].find(',').expect("its end") + 1;
            text = format!("{}{}", &text[..start], &text[end..]);
        }
        for gone in ["unattended", "guided", "old_hunger_and_thirst", "by_stage"] {
            assert!(!text.contains(gone), "{text}");
        }
        let report = Report::from_ron(&text).expect("an older report reads");
        assert_eq!(report.unattended.verdict, Verdict::NoData);
        assert_eq!(report.viability.guided.verdict, Verdict::NoData);
        assert_eq!(report.viability.grown_ups.old_hunger_and_thirst, 0);
        // Its grown-up counts were every death after tick 10,000, at any
        // life stage, so what moved shows today's by-stage counts as new
        // rather than set against them.
        let moved = moved(&report, &sample());
        for name in [
            "A4: young hatched in the viability run that grew up",
            "A4: grown-up deaths (adult or old)",
            "A4: adults' deaths of hunger or thirst",
            "A4: old sprites' deaths of hunger or thirst",
        ] {
            let line = moved.iter().find(|m| m.name == name);
            assert_eq!(line.map(|m| m.was.as_deref()), Some(None), "{name}");
        }
    }

    /// A window over ticks `from` to `to` that counted these applied actions.
    fn applied(from: u64, to: u64, applied: &[((Verb, Option<u16>), u64)]) -> Window {
        Window {
            applied: applied.iter().copied().collect(),
            ..deaths_window(from, to, &[])
        }
    }

    /// Ten seeds of a behaviour, each with the same run and control windows.
    fn behaviour_seeds(
        run: impl Fn() -> Vec<Window>,
        control: impl Fn() -> Vec<Window>,
    ) -> Vec<(u64, Result<LabRun, String>)> {
        (1..=10)
            .map(|seed| {
                let run = LabRun {
                    windows: run(),
                    control: Some(control()),
                    without: None,
                };
                (seed, Ok(run))
            })
            .collect()
    }

    #[test]
    fn a1_is_met_when_the_learner_has_at_most_half_its_controls_thornbush_contacts() {
        // Design §7.3, A1: thornbush contacts (Eat, Play and Hit) over the
        // whole run ≤ 50% of the control's, which needs ≥20.
        let data = data();
        let thornbush = Some(3);
        let contacts = |eats, plays, hits| {
            move || {
                vec![
                    applied(
                        0,
                        10_000,
                        &[((Verb::Eat, thornbush), eats), ((Verb::Rest, None), 99)],
                    ),
                    applied(
                        10_000,
                        20_000,
                        &[
                            ((Verb::Play, thornbush), plays),
                            ((Verb::Hit, thornbush), hits),
                        ],
                    ),
                ]
            }
        };
        let half = a1(
            &behaviour_seeds(contacts(10, 5, 5), contacts(20, 10, 10)),
            &data,
        );
        assert_eq!((half.median, half.control_median), (Some(20.0), Some(40.0)));
        assert_eq!(half.verdict, Verdict::Met);
        let more = a1(
            &behaviour_seeds(contacts(10, 5, 6), contacts(20, 10, 10)),
            &data,
        );
        assert_eq!(more.verdict, Verdict::NotMet { until: None });
        // A control under 20 is badly calibrated, however few the learner's.
        let thin = a1(
            &behaviour_seeds(contacts(0, 0, 0), contacts(10, 5, 4)),
            &data,
        );
        assert_eq!(thin.verdict, Verdict::NotMet { until: None });
    }

    #[test]
    fn a2_is_met_when_trained_sprites_play_with_balls_half_as_often_again_after_training() {
        // Design §7.3, A2: applied Plays on a ball over ticks 10,000–20,000
        // ≥ 1.5× the control's, which needs ≥20. Training's plays don't count.
        let data = data();
        assert_eq!(data.object_type_name(4), Some("ball"), "objects.ron");
        let plays = |during, after| {
            move || {
                vec![
                    applied(0, 9_900, &[((Verb::Play, Some(4)), during)]),
                    applied(10_000, 20_000, &[((Verb::Play, Some(4)), after)]),
                ]
            }
        };
        let met = a2(&behaviour_seeds(plays(0, 30), plays(500, 20)), &data);
        assert_eq!((met.median, met.control_median), (Some(30.0), Some(20.0)));
        assert_eq!(met.verdict, Verdict::Met);
        let short = a2(&behaviour_seeds(plays(500, 29), plays(0, 20)), &data);
        assert_eq!(short.verdict, Verdict::NotMet { until: None });
    }

    #[test]
    fn a3_is_met_when_shocked_sprites_hit_each_other_half_as_often_after_training() {
        // Design §7.3, A3: applied Hits on a sprite over ticks 10,000–20,000
        // ≤ 0.5× the control's, which needs ≥20.
        let data = data();
        assert_eq!(data.object_type_name(101), Some("sprite"), "objects.ron");
        let hits = |during, after| {
            move || {
                vec![
                    applied(0, 9_900, &[((Verb::Hit, Some(101)), during)]),
                    applied(10_000, 20_000, &[((Verb::Hit, Some(101)), after)]),
                ]
            }
        };
        let met = a3(&behaviour_seeds(hits(500, 10), hits(0, 20)), &data);
        assert_eq!((met.median, met.control_median), (Some(10.0), Some(20.0)));
        assert_eq!(met.verdict, Verdict::Met);
        let more = a3(&behaviour_seeds(hits(0, 11), hits(0, 20)), &data);
        assert_eq!(more.verdict, Verdict::NotMet { until: None });
    }

    /// A report of ten seeds, one of which lost an adult to thirst, whose
    /// unattended newborns grew up 6 in 30 on each seed.
    fn sample() -> Report {
        let data = data();
        let mut deaths: [&Dying; 10] = [&[]; 10];
        deaths[0] = &[(LifeStage::Adult, DeathCause::Dehydration, 1)];
        Report {
            commit: "abc1234".into(),
            compared_with: None,
            measured: "2026-10-01 09:00".into(),
            seconds: 900,
            seeds: 10,
            sprites: 30,
            unattended: unattended(&unattended_seeds([6; 10]), &data),
            viability: viability(&seeds_dying(deaths), 30, &data),
            a1: a1(&[], &data),
            a2: a2(&[], &data),
            a3: a3(&[], &data),
            soak: None,
            broken: Vec::new(),
        }
    }

    #[test]
    fn what_moved_lists_each_number_that_changed_since_the_previous_report() {
        // Design §7.6: the report opens with every number that moved.
        let previous = sample();
        let mut current = sample();
        current.viability.thorn_trap.median = Some(2.0);
        current.viability.seeds[0]
            .deaths
            .insert("starvation".into(), 1);
        assert_eq!(
            moved(&previous, &current),
            vec![
                Moved {
                    name: "Thorn trap: deaths by thornbush in ticks 0–30,000, median".into(),
                    was: Some("0".into()),
                    now: Some("2".into()),
                },
                Moved {
                    name: "Seed 1: died of starvation".into(),
                    was: None,
                    now: Some("1".into()),
                },
            ]
        );
    }

    #[test]
    fn nothing_moved_between_two_reports_of_the_same_numbers() {
        let mut current = sample();
        current.commit = "def5678".into();
        current.seconds = 950;
        assert_eq!(moved(&sample(), &current), Vec::new());
    }

    #[test]
    fn the_page_opens_with_the_commit_when_it_was_measured_and_what_it_is_compared_with() {
        // Design §7.6: the report names the commit it measured and the one
        // it's compared with.
        let mut report = sample();
        report.compared_with = Some("def5678".into());
        let page = report.markdown(Some(&[]));
        assert!(
            page.starts_with(
                "# Baseline report: `main` at abc1234\n\n\
                 Measured 2026-10-01 09:00, in 15 min 0 s, on seeds 1–10. \
                 Compared with def5678.\n"
            ),
            "{page}"
        );
    }

    #[test]
    fn the_page_says_what_moved_nothing_moved_or_that_there_is_nothing_to_compare() {
        let mut report = sample();
        assert!(
            report
                .markdown(None)
                .contains("This is the first baseline report: there's nothing to compare it with.")
        );
        report.compared_with = Some("def5678".into());
        assert!(
            report
                .markdown(Some(&[]))
                .contains("Nothing moved since def5678.")
        );
        let moved = [Moved {
            name: "Seed 1: died of starvation".into(),
            was: None,
            now: Some("1".into()),
        }];
        assert!(
            report
                .markdown(Some(&moved))
                .contains("## What moved\n\n- Seed 1: died of starvation: none → 1\n")
        );
    }

    #[test]
    fn the_page_gives_each_criterion_its_pass_mark_and_verdict() {
        // Design §7.6: met, not met yet or no data, against the design's
        // pass marks exactly.
        let page = sample().markdown(None);
        assert!(
            page.contains(
                "| Unattended newborns that grow up, middle seed (the unattended run) | 20% \
                 | 10% to 30% | met |"
            ),
            "{page}"
        );
        assert!(
            page.contains(
                "| Young hatched in the viability run that grow up, all seeds | none \
                 | at least 60% | no data |"
            ),
            "{page}"
        );
        assert!(
            page.contains(
                "| Adults' deaths of hunger or thirst, of grown-up deaths, all seeds \
                 | 1 of 1: too few deaths to judge | under 1 in 20, once 10 have died | met |"
            ),
            "{page}"
        );
        assert!(
            page.contains(
                "Old sprites' deaths of hunger or thirst, reported apart: 0 of the 1 grown-up \
                 deaths. What unattended newborns died of, all seeds: dehydration 240.\n"
            ),
            "{page}"
        );
        assert!(page.contains("| Unattended run | Seed 1 |"), "{page}");
        assert!(
            page.contains("| A1: thornbush contacts | no data | no data |"),
            "{page}"
        );
    }

    #[test]
    fn the_page_lists_broken_seeds_with_their_replay_commands() {
        let mut report = sample();
        report.broken = broken(
            "viability",
            &[(
                4,
                Err("tick 4,312: no tile holds more than one object".into()),
            )],
        );
        let page = report.markdown(None);
        assert!(
            page.contains(
                "## Broken\n\n- viability, seed 4: tick 4,312: no tile holds more than one \
                 object. Replay: `cargo run --profile baseline -p terra-sim --example lab -- \
                 scenarios/viability.ron --seed 4`\n"
            ),
            "{page}"
        );
    }

    #[test]
    fn a_report_written_as_ron_reads_back_the_same() {
        // The next baseline run compares itself with this one's file.
        let mut report = sample();
        report.compared_with = Some("def5678".into());
        report.broken = broken("viability", &[(4, Err("tick 9: \"quoted\"".into()))]);
        assert_eq!(Report::from_ron(&report.to_ron()), Ok(report));
    }

    /// A soak that finished: two pets given, one refused, and a sprite dead
    /// of thirst.
    fn soaked() -> Result<SoakRun, String> {
        Ok(SoakRun {
            ticks: 1_000_000,
            given: [("pet", 2), ("throw", 3)].into_iter().collect(),
            refused: [("pet", 1)].into_iter().collect(),
            deaths: [(DeathCause::Dehydration, 1)].into_iter().collect(),
            sprites: 41,
            most_sprites: 60,
            objects: 3_210,
        })
    }

    #[test]
    fn a_soak_that_finished_meets_a7_and_the_page_shows_what_its_script_did() {
        // Design §7.4, A7: no panics and no invariant violations.
        let mut report = sample();
        report.soak = Some(soak(1717, 1_000_000, &soaked(), &data()));
        assert_eq!(
            report.soak.as_ref().map(|s| &s.verdict),
            Some(&Verdict::Met)
        );
        let page = report.markdown(None);
        assert!(
            page.contains(
                "## A7: the soak (design §7.4)\n\n\
                 The default world for 1,000,000 ticks on seed 1717, with a random script of \
                 the Cursor's commands.\n\n\
                 | | Pass mark | Verdict |\n|---|---|---|\n\
                 | Panics and broken invariants | none | met |\n\n\
                 It ended with 41 sprites alive, and at most 60 at once, and 3210 objects. \
                 Deaths: dehydration 1.\n\n\
                 | Command | Given | Refused |\n|---|---|---|\n\
                 | pet | 2 | 1 |\n| throw | 3 | 0 |\n"
            ),
            "{page}"
        );
    }

    #[test]
    fn a_soak_that_broke_is_not_met_and_listed_as_broken_with_how_to_run_it_again() {
        let run =
            Err("a broken invariant at the end of tick 523,114: two sprites share a tile".into());
        let soak = soak(1717, 1_000_000, &run, &data());
        assert_eq!(soak.verdict, Verdict::NotMet { until: None });
        assert_eq!(soak.finished, None);
        let broken = broken_soak(&soak, &run).expect("broken");
        assert_eq!(broken.scenario, "soak");
        assert_eq!(broken.seed, 1717);
        assert_eq!(
            broken.replay,
            "cargo run --profile baseline -p terra-sim --example soak -- --seed 1717 --ticks 1000000"
        );
        assert_eq!(broken_soak(&soak, &soaked()), None);
        let mut report = sample();
        report.soak = Some(soak);
        assert!(report.markdown(None).contains(
            "| Panics and broken invariants | none | not met |\n\nIt broke: see Broken, above.\n"
        ));
    }

    #[test]
    fn only_the_soaks_verdict_can_move_as_it_runs_on_a_new_seed_each_time() {
        let (mut previous, mut current) = (sample(), sample());
        previous.soak = Some(soak(1, 1_000_000, &soaked(), &data()));
        let mut other = soaked();
        if let Ok(run) = &mut other {
            run.sprites = 12;
        }
        current.soak = Some(soak(2, 1_000_000, &other, &data()));
        assert_eq!(moved(&previous, &current), Vec::new());
        current.soak = Some(soak(2, 1_000_000, &Err("it panicked".into()), &data()));
        assert_eq!(
            moved(&previous, &current),
            vec![Moved {
                name: "A7: the soak".into(),
                was: Some("met".into()),
                now: Some("not met".into()),
            }]
        );
    }

    #[test]
    fn a_report_with_a_soak_reads_back_and_one_from_before_the_soak_has_none() {
        let mut report = sample();
        report.soak = Some(soak(1717, 1_000_000, &soaked(), &data()));
        let text = report.to_ron();
        assert_eq!(Report::from_ron(&text), Ok(report.clone()));
        // A report written before slice 17 has no soak field.
        let before = report_without_soak(&report);
        assert!(!before.contains("soak"), "{before}");
        assert_eq!(Report::from_ron(&before).map(|r| r.soak), Ok(None));
    }

    /// `report` as RON written before it had a soak field.
    fn report_without_soak(report: &Report) -> String {
        let mut report = report.clone();
        report.soak = None;
        let text = report.to_ron();
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("soak:"))
            .collect();
        lines.join("\n")
    }

    /// Ten seeds of A1 that would meet it: half the control's contacts.
    fn a1_met() -> Vec<(u64, Result<LabRun, String>)> {
        let contacts = |eats| move || vec![applied(0, 20_000, &[((Verb::Eat, Some(3)), eats)])];
        behaviour_seeds(contacts(10), contacts(20))
    }

    #[test]
    fn a1_is_not_met_when_a_learner_died_as_ci_fails_it() {
        // acceptance.rs: a dead sprite touches no thornbushes, which would
        // pass hollowly, so CI fails A1 when the learner dies.
        let data = data();
        assert_eq!(a1(&a1_met(), &data).verdict, Verdict::Met);
        let mut seeds = a1_met();
        if let (_, Ok(run)) = &mut seeds[3] {
            run.windows[0].deaths.insert(DeathCause::HurtBy(3), 1);
        }
        assert_eq!(a1(&seeds, &data).verdict, Verdict::NotMet { until: None });
    }

    #[test]
    fn a_behaviour_scenario_with_a_broken_seed_is_not_met_as_ci_fails_it() {
        // CI's acceptance tests panic on a broken seed, so the report can't
        // call the scenario met on the seeds that finished.
        let mut seeds = a1_met();
        seeds[9].1 = Err("tick 3: IDs only go up".into());
        let a1 = a1(&seeds, &data());
        assert_eq!(a1.verdict, Verdict::NotMet { until: None });
        assert_eq!(a1.finished, 9);
    }

    #[test]
    fn the_page_says_how_many_seeds_its_medians_cover_when_some_broke() {
        let data = data();
        let mut seeds = seeds_dying([&[]; 10]);
        seeds[9].1 = Err("tick 3: IDs only go up".into());
        let mut report = sample();
        report.viability = viability(&seeds, 30, &data);
        let mut a1_seeds = a1_met();
        a1_seeds[9].1 = Err("tick 3: IDs only go up".into());
        report.a1 = a1(&a1_seeds, &data);
        let page = report.markdown(None);
        assert!(
            page.contains(
                "## A4 and G5: viability (design §7.4, M2 design §6)\n\n\
                 The viability run: over the 9 of 10 seeds that finished.\n"
            ),
            "{page}"
        );
        assert!(
            page.contains("| A1: thornbush contacts | 10, over 9 of 10 seeds | 20 |"),
            "{page}"
        );
    }

    #[test]
    fn the_grown_up_rule_counts_only_the_seeds_that_finished() {
        let mut deaths: [&Dying; 10] = [&[(LifeStage::Old, DeathCause::OldAge, 3)]; 10];
        deaths[0] = &[(LifeStage::Adult, DeathCause::Dehydration, 1)];
        let mut seeds = seeds_dying(deaths);
        for broken in &mut seeds[5..] {
            broken.1 = Err("tick 3: IDs only go up".into());
        }
        let mut report = sample();
        report.viability = viability(&seeds, 30, &data());
        let page = report.markdown(None);
        // Seed 1's thirst death and three of old age on each of seeds 2–5.
        assert!(page.contains("| 1 of 13: 7.7% |"), "{page}");
    }

    #[test]
    fn a_behaviour_scenario_whose_every_seed_broke_is_not_met() {
        // Gemini's review on #95: CI fails it, so it's no different from one
        // broken seed; "no data" is for a scenario that didn't run.
        let seeds: Vec<_> = (1..=10)
            .map(|seed| (seed, Err("tick 3: IDs only go up".into())))
            .collect();
        assert_eq!(a1(&seeds, &data()).verdict, Verdict::NotMet { until: None });
        assert_eq!(a1(&[], &data()).verdict, Verdict::NoData);
    }

    #[test]
    fn a_whole_percentage_shows_no_decimals_however_the_float_rounds() {
        // Gemini's review on #95: 7 / 25 × 100 is 28.000000000000004.
        assert_eq!(percent(Some(7.0 / 25.0)), "28%");
        assert_eq!(percent(Some(47.0 / 60.0)), "78.3%");
        assert_eq!(percent(Some(1.0)), "100%");
    }
}
