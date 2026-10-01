//! Baseline runs (design §7.6): measures on a commit of `main` what's too
//! slow for CI, and reports what moved since the last commit measured.
//!
//! `cargo run --profile baseline -p terra-sim --example baseline -- --commit
//! abc1234 --measured "2026-10-01 09:00" --out docs/reports [--previous
//! <report.ron>] [--seeds N]`
//!
//! Build it with the `baseline` profile, which keeps the self-check after
//! every tick (§7.1), so a broken invariant stops its seed naming the tick.
//! It exits with 2 when a seed broke, and 1 when it couldn't run or save.

mod report;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::time::Instant;

use report::Report;
use terra_sim::{DataPack, LabRun, LabScenario, WorldConfig};

/// The lab scenarios a baseline run measures, by the names of their files
/// in `scenarios/`.
const SCENARIOS: [(&str, &str); 4] = [
    (
        "viability",
        include_str!("../../../../scenarios/viability.ron"),
    ),
    (
        "a1-thornbush",
        include_str!("../../../../scenarios/a1-thornbush.ron"),
    ),
    (
        "a2-reward-training",
        include_str!("../../../../scenarios/a2-reward-training.ron"),
    ),
    (
        "a3-correct-training",
        include_str!("../../../../scenarios/a3-correct-training.ron"),
    ),
];

/// What the launcher passes in.
struct Options {
    commit: String,
    measured: String,
    out: PathBuf,
    previous: Option<PathBuf>,
    seeds: u64,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!(
                "{message}\nusage: baseline --commit C --measured \"YYYY-MM-DD HH:MM\" \
                 --out DIR [--previous REPORT.ron] [--seeds N]"
            );
            return ExitCode::FAILURE;
        }
    };
    // Read the previous report first, so a bad one is found before the run.
    let previous = match options.previous.as_deref().map(read_report).transpose() {
        Ok(previous) => previous,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let started = Instant::now();
    let results = run_all(&data, options.seeds);
    let seeds_of = |name: &str| -> &report::SeedRuns {
        &results[SCENARIOS
            .iter()
            .position(|(n, _)| *n == name)
            .expect("a scenario")]
    };
    let sprites = WorldConfig::builtin(&data).sprites() as u64;
    let report = Report {
        commit: options.commit,
        compared_with: previous.as_ref().map(|p| p.commit.clone()),
        measured: options.measured,
        seconds: started.elapsed().as_secs(),
        seeds: options.seeds,
        sprites,
        viability: report::viability(seeds_of("viability"), sprites, &data),
        a1: report::a1(seeds_of("a1-thornbush"), &data),
        a2: report::a2(seeds_of("a2-reward-training"), &data),
        a3: report::a3(seeds_of("a3-correct-training"), &data),
        broken: SCENARIOS
            .iter()
            .flat_map(|(name, _)| report::broken(name, seeds_of(name)))
            .collect(),
    };
    let moved = previous.as_ref().map(|p| report::moved(p, &report));
    let page = report.markdown(moved.as_deref());
    match save(&options.out, &report, &page) {
        Ok((md, _)) => println!("{page}\nSaved as {}", md.display()),
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    }
    if report.broken.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

/// The options, from the command line.
fn parse(args: &[String]) -> Result<Options, String> {
    let (mut commit, mut measured, mut out, mut previous, mut seeds) = (None, None, None, None, 10);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--commit" => commit = Some(value()?),
            "--measured" => measured = Some(value()?),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--previous" => previous = Some(PathBuf::from(value()?)),
            "--seeds" => {
                let n = value()?;
                seeds = n
                    .parse()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or(format!("--seeds takes a whole number above 0, not {n}"))?;
            }
            _ => return Err(format!("unexpected argument {arg}")),
        }
    }
    Ok(Options {
        commit: commit.ok_or("which commit? (--commit)")?,
        measured: measured.ok_or("when? (--measured)")?,
        out: out.ok_or("where to? (--out)")?,
        previous,
        seeds,
    })
}

/// A report from its RON file.
fn read_report(path: &Path) -> Result<Report, String> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the baseline program reads its previous report; the sim itself never does"
    )]
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("can't read the previous report {}: {e}", path.display()))?;
    Report::from_ron(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs seeds 1 to `seeds` of every scenario, as many at once as the machine
/// has threads, longest first. Each seed runs on a thread of its own, so a
/// panic, or a broken invariant with the self-check on, ends only that seed:
/// its result is what it said. The results are in `SCENARIOS`' order, then
/// by seed.
fn run_all(data: &DataPack, seeds: u64) -> Vec<Vec<(u64, Result<LabRun, String>)>> {
    let labs: Vec<LabScenario> = SCENARIOS
        .iter()
        .map(|(name, text)| {
            LabScenario::from_ron(text, data)
                .unwrap_or_else(|e| panic!("scenarios/{name}.ron: {e}"))
        })
        .collect();
    // Jobs are taken from the end, so the viability run's, the longest, go
    // first.
    let jobs: Vec<(usize, u64)> = (0..labs.len())
        .rev()
        .flat_map(|lab| (1..=seeds).rev().map(move |seed| (lab, seed)))
        .collect();
    let jobs = Mutex::new(jobs);
    let results = Mutex::new((0..labs.len()).map(|_| Vec::new()).collect::<Vec<_>>());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    // Taken on a line of its own, so the lock isn't held
                    // while the seed runs.
                    let job = jobs.lock().expect("no worker panics").pop();
                    let Some((lab, seed)) = job else { break };
                    let run = std::thread::scope(|inner| {
                        inner
                            .spawn(|| labs[lab].run(data.clone(), seed))
                            .join()
                            .map_err(|panic| panic_message(&*panic))
                    });
                    results.lock().expect("no worker panics")[lab].push((seed, run));
                }
            });
        }
    });
    let mut results = results.into_inner().expect("no worker panics");
    for runs in &mut results {
        runs.sort_by_key(|(seed, _)| *seed);
    }
    results
}

/// What a panic said.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "it panicked".into())
}

/// What a report's files are named after: the day it was measured and its
/// commit, such as `2026-10-01-abc1234`.
fn stem(report: &Report) -> String {
    let day = report.measured.get(..10).unwrap_or(&report.measured);
    format!("{day}-{}", report.commit)
}

/// Writes the report into `dir` as a page and as RON, returning their paths.
fn save(dir: &Path, report: &Report, page: &str) -> Result<(PathBuf, PathBuf), String> {
    let stem = stem(report);
    let md = dir.join(format!("{stem}-report.md"));
    let ron = dir.join(format!("{stem}-report.ron"));
    #[expect(
        clippy::disallowed_methods,
        reason = "the baseline program writes its report; the sim itself never does"
    )]
    for (path, text) in [(&md, page.to_string()), (&ron, report.to_ron())] {
        std::fs::write(path, text).map_err(|e| format!("can't write {}: {e}", path.display()))?;
    }
    Ok((md, ron))
}

#[cfg(test)]
mod tests {
    use super::*;
    use report::{Behaviour, Verdict, Viability};

    fn report() -> Report {
        let none = || Behaviour {
            median: None,
            control_median: None,
            verdict: Verdict::NoData,
        };
        let data = terra_sim::DataPack::builtin().expect("the built-in data pack is valid");
        let viability: Viability = report::viability(&[], 30, &data);
        Report {
            commit: "abc1234".into(),
            compared_with: None,
            measured: "2026-10-01 09:00".into(),
            seconds: 900,
            seeds: 10,
            sprites: 30,
            viability,
            a1: none(),
            a2: none(),
            a3: none(),
            broken: Vec::new(),
        }
    }

    #[test]
    fn a_report_is_saved_as_a_page_and_as_ron_named_by_day_and_commit() {
        let dir = std::env::temp_dir().join(format!("baseline-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        let (md, ron) = save(&dir, &report(), "the page").expect("written");
        assert_eq!(md, dir.join("2026-10-01-abc1234-report.md"));
        assert_eq!(ron, dir.join("2026-10-01-abc1234-report.ron"));
        #[expect(
            clippy::disallowed_methods,
            reason = "reading back what the test wrote"
        )]
        let (page, text) = (
            std::fs::read_to_string(&md).expect("the page"),
            std::fs::read_to_string(&ron).expect("the RON"),
        );
        assert_eq!(page, "the page");
        assert_eq!(Report::from_ron(&text), Ok(report()));
        std::fs::remove_dir_all(&dir).expect("cleaned up");
    }

    #[test]
    fn a_report_that_cant_be_written_says_where() {
        let missing = std::env::temp_dir().join("baseline-test-no-such-folder/inside");
        let error = save(&missing, &report(), "the page").expect_err("no folder to write in");
        assert!(error.contains("2026-10-01-abc1234-report.md"), "{error}");
    }
}
