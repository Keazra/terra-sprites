//! The baseline report (design §7.6): each criterion's numbers and verdict.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use terra_sim::{DataPack, DeathCause, LabRun, Verb, Window};

/// The slice that tunes the default world to meet A4 (design §7.4).
const A4_TUNING: &str = "#18";

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

/// A criterion's median across the seeds, and its verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Criterion {
    pub median: Option<f64>,
    pub verdict: Verdict,
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
    pub viability: Viability,
    pub a1: Behaviour,
    pub a2: Behaviour,
    pub a3: Behaviour,
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
        page.push_str("## A4: viability (design §7.4)\n\n");
        if (v.finished as u64) < self.seeds {
            page.push_str(&format!(
                "Over the {} of {} seeds that finished.\n\n",
                v.finished, self.seeds
            ));
        }
        page.push_str("| | Median | Pass mark | Verdict |\n|---|---|---|---|\n");
        page.push_str(&format!(
            "| Alive at tick 10,000 | {} | at least 80% | {} |\n",
            percent(v.survival.median),
            v.survival.verdict
        ));
        let share = match v.hunger_and_thirst.median {
            None => "no data: no sprite died".to_string(),
            Some(_) => format!(
                "{}, over the {} of {} seeds with deaths",
                percent(v.hunger_and_thirst.median),
                v.seeds_with_deaths,
                v.finished
            ),
        };
        page.push_str(&format!(
            "| Hunger and thirst's share of deaths, ticks 0–50,000 | {share} | under 25% | {} |\n\n",
            v.hunger_and_thirst.verdict
        ));

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
        let v = &self.viability;
        let mut numbers = vec![
            (
                "A4: alive at tick 10,000, median".to_string(),
                percent(v.survival.median),
            ),
            (
                "A4: hunger and thirst's share of deaths, median".into(),
                percent(v.hunger_and_thirst.median),
            ),
            (
                "A4: seeds with deaths".into(),
                v.seeds_with_deaths.to_string(),
            ),
            (
                "Thorn trap: deaths by thornbush in ticks 0–30,000, median".into(),
                number(v.thorn_trap.median),
            ),
        ];
        for (what, control, _, behaviour) in self.behaviours() {
            numbers.push((format!("{what}, median"), number(behaviour.median)));
            numbers.push((
                format!("{what} {control}, median"),
                number(behaviour.control_median),
            ));
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

/// What the viability run measured (design §7.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viability {
    /// How many seeds finished; the medians are theirs.
    pub finished: usize,
    /// A4: the share of sprites alive at tick 10,000.
    pub survival: Criterion,
    /// A4: starvation and dehydration's share of deaths over 50,000 ticks,
    /// over the seeds where any sprite died.
    pub hunger_and_thirst: Criterion,
    /// How many seeds had a death in their 50,000 ticks.
    pub seeds_with_deaths: usize,
    /// The thorn trap (design §7.3): deaths by thornbush over ticks 0–30,000.
    pub thorn_trap: Count,
    /// A row for each seed that finished: who survived, what killed the rest, and the verbs.
    pub seeds: Vec<SeedRow>,
}

/// One seed's 50,000 ticks: who survived, what killed the rest, and how
/// often each verb was applied, each by name.
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
    let alive: Vec<f64> = finished
        .iter()
        .map(|(_, run)| {
            let died: u64 = window(run, 10_000).deaths.values().sum();
            sprites.saturating_sub(died) as f64 / sprites as f64
        })
        .collect();
    // A seed where no one died has no share of deaths (design §7.6).
    let hunger_and_thirst: Vec<f64> = finished
        .iter()
        .map(|(_, run)| &window(run, 50_000).deaths)
        .filter_map(|deaths| {
            let all: u64 = deaths.values().sum();
            let hungry_or_thirsty: u64 = deaths
                .iter()
                .filter(|(cause, _)| {
                    matches!(cause, DeathCause::Starvation | DeathCause::Dehydration)
                })
                .map(|(_, n)| n)
                .sum();
            (all > 0).then(|| hungry_or_thirsty as f64 / all as f64)
        })
        .collect();
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
                alive: sprites.saturating_sub(all.deaths.values().sum()),
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
        seeds: rows,
        survival: a4(median_of(&alive), |share| share >= 0.8),
        hunger_and_thirst: a4(median_of(&hunger_and_thirst), |share| share < 0.25),
        seeds_with_deaths: hunger_and_thirst.len(),
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

/// One of A4's halves: its median, met when it `passes`, and otherwise
/// not met until slice 17 is done.
fn a4(median: Option<f64>, passes: impl Fn(f64) -> bool) -> Criterion {
    let verdict = match median {
        None => Verdict::NoData,
        Some(share) if passes(share) => Verdict::Met,
        Some(_) => Verdict::NotMet {
            until: Some(A4_TUNING.into()),
        },
    };
    Criterion { median, verdict }
}

/// The run's window over ticks 0 to `to`, as `scenarios/viability.ron` sets them.
fn window(run: &LabRun, to: u64) -> &Window {
    run.windows
        .iter()
        .find(|w| w.from == 0 && w.to == to)
        .unwrap_or_else(|| panic!("the viability run counts ticks 0 to {to}"))
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
    use terra_sim::{DeathCause, Verb};

    fn data() -> DataPack {
        DataPack::builtin().expect("the built-in data pack is valid")
    }

    /// A window over ticks `from` to `to` that counted `deaths`.
    fn deaths_window(from: u64, to: u64, deaths: &[(DeathCause, u64)]) -> Window {
        Window {
            from,
            to,
            applied: BTreeMap::new(),
            deaths: deaths.iter().copied().collect(),
            touches: BTreeMap::new(),
            lessons: BTreeMap::new(),
        }
    }

    /// A viability run's seed, with the deaths in its first 10,000, 30,000
    /// and 50,000 ticks.
    fn viability_run(
        by_10k: &[(DeathCause, u64)],
        by_30k: &[(DeathCause, u64)],
        by_50k: &[(DeathCause, u64)],
    ) -> LabRun {
        LabRun {
            windows: vec![
                deaths_window(0, 10_000, by_10k),
                deaths_window(0, 30_000, by_30k),
                deaths_window(0, 50_000, by_50k),
            ],
            control: None,
            without: None,
        }
    }

    #[test]
    fn a4_survival_is_met_when_the_median_seed_keeps_80_percent_to_tick_10000() {
        // Design §7.4, A4: ≥80% of sprites survive the first 10,000 ticks.
        let six = [(DeathCause::Starvation, 6)]; // 24 of 30 alive: 80%
        let seeds: Vec<_> = (1..=10)
            .map(|seed| (seed, Ok(viability_run(&six, &six, &six))))
            .collect();
        let survival = viability(&seeds, 30, &data()).survival;
        assert_eq!(survival.median, Some(0.8));
        assert_eq!(survival.verdict, Verdict::Met);
    }

    #[test]
    fn a4_survival_under_80_percent_is_not_met_yet_until_slice_17() {
        let six = [(DeathCause::Starvation, 6)]; // 24 of 30 alive
        let seven = [(DeathCause::Starvation, 7)]; // 23 of 30 alive
        let seeds: Vec<_> = (1..=10)
            .map(|seed| {
                let deaths = if seed <= 5 { &seven } else { &six };
                (seed, Ok(viability_run(deaths, deaths, deaths)))
            })
            .collect();
        let survival = viability(&seeds, 30, &data()).survival;
        // The median is halfway between 23 and 24 of 30.
        let median = survival.median.expect("ten seeds finished");
        assert!((median - 47.0 / 60.0).abs() < 1e-12, "median {median}");
        assert_eq!(
            survival.verdict,
            Verdict::NotMet {
                until: Some("#18".into())
            }
        );
    }

    /// Ten seeds whose first 50,000 ticks saw these deaths, by seed.
    fn seeds_dying(deaths: [&[(DeathCause, u64)]; 10]) -> Vec<(u64, Result<LabRun, String>)> {
        (1..=10)
            .zip(deaths)
            .map(|(seed, deaths)| (seed, Ok(viability_run(&[], &[], deaths))))
            .collect()
    }

    #[test]
    fn a4_hunger_and_thirst_share_is_the_median_over_the_seeds_where_sprites_died() {
        // Design §7.4, A4: starvation plus dehydration cause <25% of deaths
        // over the first 50,000 ticks. A seed where no one died has no share
        // (design §7.6), so five seeds give the median.
        use DeathCause::{Dehydration, OldAge, Starvation};
        let viability = viability(
            &seeds_dying([
                &[],
                &[],
                &[],
                &[],
                &[],
                &[(Dehydration, 1)],              // 100%
                &[(Dehydration, 2)],              // 100%
                &[(Starvation, 1), (OldAge, 3)],  // 25%
                &[(OldAge, 4)],                   // 0%
                &[(Dehydration, 1), (OldAge, 1)], // 50%
            ]),
            30,
            &data(),
        );
        assert_eq!(viability.hunger_and_thirst.median, Some(0.5));
        assert_eq!(viability.seeds_with_deaths, 5);
        assert_eq!(
            viability.hunger_and_thirst.verdict,
            Verdict::NotMet {
                until: Some("#18".into())
            }
        );
    }

    #[test]
    fn a4_hunger_and_thirst_share_is_met_only_under_a_quarter() {
        use DeathCause::{OldAge, Starvation};
        let quarter: &[(DeathCause, u64)] = &[(Starvation, 1), (OldAge, 3)];
        let at_a_quarter = viability(&seeds_dying([quarter; 10]), 30, &data());
        assert_eq!(
            at_a_quarter.hunger_and_thirst.verdict,
            Verdict::NotMet {
                until: Some("#18".into())
            }
        );
        let under: &[(DeathCause, u64)] = &[(Starvation, 1), (OldAge, 4)];
        let under_a_quarter = viability(&seeds_dying([under; 10]), 30, &data());
        assert_eq!(under_a_quarter.hunger_and_thirst.verdict, Verdict::Met);
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
                    (thornbush, thornbush_deaths + 5),
                    (DeathCause::Dehydration, 2),
                ];
                (seed, Ok(viability_run(&[], &by_30k, &by_50k)))
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
            (1, Ok(viability_run(&[], &[], &[]))),
            (2, Err(message.clone())),
            (3, Ok(viability_run(&[], &[], &[]))),
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
        // Design §7.6: per seed, who survived the 50,000 ticks, what killed
        // the rest, and how often each verb was applied, whatever its target.
        let data = data();
        assert_eq!(data.object_type_name(2), Some("berry"), "objects.ron");
        assert_eq!(data.object_type_name(3), Some("thornbush"), "objects.ron");
        let mut run = viability_run(
            &[],
            &[],
            &[(DeathCause::Dehydration, 2), (DeathCause::HurtBy(3), 1)],
        );
        run.windows[2].applied = BTreeMap::from([
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
                alive: 27,
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
    fn a4_hunger_and_thirst_share_has_no_data_when_no_sprite_died() {
        let viability = viability(&seeds_dying([&[]; 10]), 30, &data());
        assert_eq!(viability.hunger_and_thirst.median, None);
        assert_eq!(viability.hunger_and_thirst.verdict, Verdict::NoData);
        assert_eq!(viability.seeds_with_deaths, 0);
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

    /// A report of ten seeds, one of which lost a sprite to thirst.
    fn sample() -> Report {
        let data = data();
        let mut deaths: [&[(DeathCause, u64)]; 10] = [&[]; 10];
        deaths[0] = &[(DeathCause::Dehydration, 1)];
        Report {
            commit: "abc1234".into(),
            compared_with: None,
            measured: "2026-10-01 09:00".into(),
            seconds: 900,
            seeds: 10,
            sprites: 30,
            viability: viability(&seeds_dying(deaths), 30, &data),
            a1: a1(&[], &data),
            a2: a2(&[], &data),
            a3: a3(&[], &data),
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
            page.contains("| Alive at tick 10,000 | 100% | at least 80% | met |"),
            "{page}"
        );
        assert!(
            page.contains(
                "| Hunger and thirst's share of deaths, ticks 0–50,000 \
                 | 100%, over the 1 of 10 seeds with deaths | under 25% | not met yet (#18) |"
            ),
            "{page}"
        );
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
                "## A4: viability (design §7.4)\n\nOver the 9 of 10 seeds that finished.\n"
            ),
            "{page}"
        );
        assert!(
            page.contains("| A1: thornbush contacts | 10, over 9 of 10 seeds | 20 |"),
            "{page}"
        );
    }

    #[test]
    fn the_hunger_and_thirst_share_counts_seeds_with_deaths_among_those_that_finished() {
        // Gemini's review on #95: "2 of 10 seeds with deaths" would suggest
        // that the seeds which broke had no deaths.
        let mut deaths: [&[(DeathCause, u64)]; 10] = [&[]; 10];
        deaths[0] = &[(DeathCause::Dehydration, 1)];
        deaths[1] = &[(DeathCause::Starvation, 1)];
        let mut seeds = seeds_dying(deaths);
        for broken in &mut seeds[5..] {
            broken.1 = Err("tick 3: IDs only go up".into());
        }
        let mut report = sample();
        report.viability = viability(&seeds, 30, &data());
        let page = report.markdown(None);
        assert!(
            page.contains("| 100%, over the 2 of 5 seeds with deaths |"),
            "{page}"
        );
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
