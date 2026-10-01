//! Baseline runs (design §7.6): measures on a commit of `main` what's too
//! slow for CI, and reports what moved since the last commit measured.

mod report;

use std::path::{Path, PathBuf};

use report::Report;

fn main() {}

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
    use report::{Lesson, Verdict, Viability};

    fn report() -> Report {
        let none = || Lesson {
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
