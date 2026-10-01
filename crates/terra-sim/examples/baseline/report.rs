//! The baseline report (design §7.6): each criterion's numbers and verdict.

use std::collections::BTreeMap;

use terra_sim::{DataPack, DeathCause, LabRun, Verb, Window};

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
    /// A4: starvation and dehydration's share of deaths over 50,000 ticks,
    /// over the seeds where any sprite died.
    pub hunger_and_thirst: Criterion,
    /// How many seeds had a death in their 50,000 ticks.
    pub seeds_with_deaths: usize,
    /// The thorn trap (design §7.3): deaths by thornbush over ticks 0–30,000.
    pub thorn_trap: Count,
    /// The health table: a row for each seed that finished.
    pub seeds: Vec<SeedRow>,
}

/// One seed's 50,000 ticks: who survived, what killed the rest, and how
/// often each verb was applied, each by name.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedRow {
    pub seed: u64,
    pub alive: u64,
    pub deaths: BTreeMap<String, u64>,
    pub verbs: BTreeMap<String, u64>,
}

/// A number counted on each seed, and its median. It has no pass mark.
#[derive(Debug, Clone, PartialEq)]
pub struct Count {
    /// Each seed and its count.
    pub per_seed: Vec<(u64, u64)>,
    pub median: Option<f64>,
}

/// The viability run's report, from each seed's run or the reason it broke.
/// Every seed starts with `sprites` sprites.
pub fn viability(
    seeds: &[(u64, Result<LabRun, String>)],
    sprites: u64,
    data: &DataPack,
) -> Viability {
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
            let starved: u64 = deaths
                .iter()
                .filter(|(cause, _)| {
                    matches!(cause, DeathCause::Starvation | DeathCause::Dehydration)
                })
                .map(|(_, n)| n)
                .sum();
            (all > 0).then(|| starved as f64 / all as f64)
        })
        .collect();
    let pricked: Vec<(u64, u64)> = finished
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
    let counts: Vec<f64> = pricked.iter().map(|&(_, n)| n as f64).collect();
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
        seeds: rows,
        survival: a4(median_of(&alive), |share| share >= 0.8),
        hunger_and_thirst: a4(median_of(&hunger_and_thirst), |share| share < 0.25),
        seeds_with_deaths: hunger_and_thirst.len(),
        thorn_trap: Count {
            median: median_of(&counts),
            per_seed: pricked,
        },
    }
}

/// One of A1–A3 (design §7.3): its runs' median, its controls', and the
/// verdict CI's acceptance tests give on the same seeds.
#[derive(Debug, Clone, PartialEq)]
pub struct Lesson {
    pub median: Option<f64>,
    pub control_median: Option<f64>,
    pub verdict: Verdict,
}

/// A1: thornbush contacts (Eat, Play and Hit) over the whole run, at most
/// half the control's.
pub fn a1(seeds: &[(u64, Result<LabRun, String>)], data: &DataPack) -> Lesson {
    let contacts = |windows: &[Window]| {
        let verbs = [Verb::Eat, Verb::Play, Verb::Hit];
        windows
            .iter()
            .flat_map(|w| verbs.map(|verb| w.applied_on(verb, "thornbush", data)))
            .sum()
    };
    lesson(seeds, contacts, |run, control| run <= control / 2.0)
}

/// A2: applied Plays on a ball after training, at least 1.5× the control's.
pub fn a2(seeds: &[(u64, Result<LabRun, String>)], data: &DataPack) -> Lesson {
    let plays = |windows: &[Window]| after_training(windows).applied_on(Verb::Play, "ball", data);
    lesson(seeds, plays, |run, control| run >= 1.5 * control)
}

/// A3: applied Hits on a sprite after training, at most half the control's.
pub fn a3(seeds: &[(u64, Result<LabRun, String>)], data: &DataPack) -> Lesson {
    let hits = |windows: &[Window]| after_training(windows).applied_on(Verb::Hit, "sprite", data);
    lesson(seeds, hits, |run, control| run <= 0.5 * control)
}

/// A lesson's medians of what `count` counts in each seed's windows, and
/// whether they pass: `passes` given the run's and the control's, and a
/// control of at least 20, or the scenario is badly calibrated (§7.3).
fn lesson(
    seeds: &[(u64, Result<LabRun, String>)],
    count: impl Fn(&[Window]) -> u64,
    passes: impl Fn(f64, f64) -> bool,
) -> Lesson {
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
    let verdict = match (median, control_median) {
        (Some(run), Some(control)) if control >= 20.0 && passes(run, control) => Verdict::Met,
        (Some(_), Some(_)) => Verdict::NotMet { until: None },
        _ => Verdict::NoData,
    };
    Lesson {
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

/// A seed that crashed or broke an invariant (design §7.6).
#[derive(Debug, Clone, PartialEq)]
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
pub fn broken(scenario: &str, seeds: &[(u64, Result<LabRun, String>)]) -> Vec<Broken> {
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
            .map(|(seed, pricked)| {
                let by_30k = [(thornbush, pricked), (DeathCause::Dehydration, 2)];
                let by_50k = [(thornbush, pricked + 5), (DeathCause::Dehydration, 2)];
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
    fn a_seed_that_crashed_is_broken_with_how_to_replay_it_and_the_rest_are_still_reported() {
        // Design §7.6: broken means a crash or a broken invariant, given with
        // the seed, the tick (in the message) and the command that replays it.
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
    fn the_health_table_has_a_row_per_seed_of_who_survived_what_killed_the_rest_and_the_verbs() {
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
            ..window(from, to, &[])
        }
    }

    /// Ten seeds of a lesson, each with the same run and control windows.
    fn lesson_seeds(
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
    fn a1_is_met_when_the_learner_touches_thornbushes_half_as_often_as_its_control() {
        // Design §7.3, A1: thornbush contacts (Eat, Play and Hit) over the
        // whole run ≤ 50% of the control's, which needs ≥20.
        let data = data();
        let thornbush = Some(3);
        let touches = |eats, plays, hits| {
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
        let half = a1(&lesson_seeds(touches(10, 5, 5), touches(20, 10, 10)), &data);
        assert_eq!((half.median, half.control_median), (Some(20.0), Some(40.0)));
        assert_eq!(half.verdict, Verdict::Met);
        let more = a1(&lesson_seeds(touches(10, 5, 6), touches(20, 10, 10)), &data);
        assert_eq!(more.verdict, Verdict::NotMet { until: None });
        // A control under 20 is badly calibrated, however few the learner's.
        let thin = a1(&lesson_seeds(touches(0, 0, 0), touches(10, 5, 4)), &data);
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
        let met = a2(&lesson_seeds(plays(0, 30), plays(500, 20)), &data);
        assert_eq!((met.median, met.control_median), (Some(30.0), Some(20.0)));
        assert_eq!(met.verdict, Verdict::Met);
        let short = a2(&lesson_seeds(plays(500, 29), plays(0, 20)), &data);
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
        let met = a3(&lesson_seeds(hits(500, 10), hits(0, 20)), &data);
        assert_eq!((met.median, met.control_median), (Some(10.0), Some(20.0)));
        assert_eq!(met.verdict, Verdict::Met);
        let more = a3(&lesson_seeds(hits(0, 11), hits(0, 20)), &data);
        assert_eq!(more.verdict, Verdict::NotMet { until: None });
    }
}
