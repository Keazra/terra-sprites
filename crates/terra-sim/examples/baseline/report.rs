//! The baseline report (design §7.6): each criterion's numbers and verdict.

use terra_sim::{DataPack, LabRun, Window};

/// The slice that tunes the default world to meet A4 (design §7.4).
const A4_TUNING: &str = "#18";

/// How a criterion stands against its pass mark (design §7.6).
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Met,
    /// Not met; `until` names the slice that's meant to meet it.
    NotMet {
        until: Option<String>,
    },
    NoData,
}

/// A criterion's median across the seeds, and its verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct Criterion {
    pub median: Option<f64>,
    pub verdict: Verdict,
}

/// What the viability run measured (design §7.6).
#[derive(Debug, Clone, PartialEq)]
pub struct Viability {
    /// A4: the share of sprites alive at tick 10,000.
    pub survival: Criterion,
}

/// The viability run's report, from each seed's run or the reason it broke.
/// Every seed starts with `sprites` sprites.
pub fn viability(
    seeds: &[(u64, Result<LabRun, String>)],
    sprites: u64,
    _data: &DataPack,
) -> Viability {
    let finished: Vec<&LabRun> = seeds
        .iter()
        .filter_map(|(_, run)| run.as_ref().ok())
        .collect();
    let alive: Vec<f64> = finished
        .iter()
        .map(|run| {
            let died: u64 = window(run, 10_000).deaths.values().sum();
            sprites.saturating_sub(died) as f64 / sprites as f64
        })
        .collect();
    let median = median_of(&alive);
    let verdict = match median {
        None => Verdict::NoData,
        Some(share) if share >= 0.8 => Verdict::Met,
        Some(_) => Verdict::NotMet {
            until: Some(A4_TUNING.into()),
        },
    };
    Viability {
        survival: Criterion { median, verdict },
    }
}

/// The run's window over ticks 0 to `to`, as `scenarios/viability.ron` sets them.
fn window(run: &LabRun, to: u64) -> &Window {
    run.windows
        .iter()
        .find(|w| w.from == 0 && w.to == to)
        .unwrap_or_else(|| panic!("the viability run counts ticks 0 to {to}"))
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
    use terra_sim::DeathCause;

    fn data() -> DataPack {
        DataPack::builtin().expect("the built-in data pack is valid")
    }

    /// A window over ticks `from` to `to` that counted `deaths`.
    fn window(from: u64, to: u64, deaths: &[(DeathCause, u64)]) -> Window {
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
                window(0, 10_000, by_10k),
                window(0, 30_000, by_30k),
                window(0, 50_000, by_50k),
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
}
