//! M1's acceptance scenarios (design §7.3, §7.4), run from the lab scenario
//! files they share with the lab runner.

mod common;
use common::builtin;

use terra_sim::{DataPack, LabScenario, Verb, Window, median};

/// Applied thornbush contacts in `window`: bites, plays and hits.
fn thornbush_contacts(window: &Window, data: &DataPack) -> u64 {
    [Verb::Eat, Verb::Play, Verb::Hit]
        .into_iter()
        .map(|verb| window.applied_on(verb, "thornbush", data))
        .sum()
}

/// Applied thornbush contacts over all of `windows`.
fn all_contacts(windows: &[Window], data: &DataPack) -> u64 {
    windows.iter().map(|w| thornbush_contacts(w, data)).sum()
}

#[test]
fn a1_a_sprite_learns_to_keep_away_from_thornbushes() {
    // Design v16 §7.3: a learner against the same seed without learning.
    let data = builtin();
    let text = include_str!("../../../scenarios/a1-thornbush.ron");
    let lab = LabScenario::from_ron(text, &data).expect("a valid scenario");
    let (mut learners, mut controls) = (Vec::new(), Vec::new());
    for seed in 1..=10 {
        let run = lab.run(data.clone(), seed);
        // A dead sprite touches no thornbushes, which would pass hollowly.
        let deaths: Vec<_> = run.windows.iter().flat_map(|w| w.deaths.keys()).collect();
        assert!(
            deaths.is_empty(),
            "seed {seed}: the learner died: {deaths:?}"
        );
        learners.push(all_contacts(&run.windows, &data));
        let control = run.control.expect("A1 asks for a control run");
        controls.push(all_contacts(&control, &data));
    }
    let (learner, control) = (median(&learners), median(&controls));
    assert!(
        control >= 20.0,
        "badly calibrated: the control's median is {control} contacts ({controls:?}), needing 20"
    );
    assert!(
        learner <= control / 2.0,
        "the learner's median is {learner} contacts ({learners:?}), against the control's {control} ({controls:?})"
    );
}

/// Each seed's count of applied `verb`s on `target` over ticks 10,000 to
/// 20,000 (the scenario's second window), trained and untrained.
fn after_training(
    lab: &LabScenario,
    data: &DataPack,
    verb: Verb,
    target: &str,
) -> (Vec<u64>, Vec<u64>) {
    let (mut trained, mut untrained) = (Vec::new(), Vec::new());
    for seed in 1..=10 {
        let run = lab.run(data.clone(), seed);
        trained.push(run.windows[1].applied_on(verb, target, data));
        let control = run.control.expect("a control run without the trainer");
        untrained.push(control[1].applied_on(verb, target, data));
    }
    (trained, untrained)
}

#[test]
fn a2_a_sprite_petted_for_playing_with_balls_plays_with_them_more() {
    // Design v21 §7.3: after training, its applied Plays on balls are at
    // least 1.5× the same seed's never trained, median of 10 seeds, with at
    // least 20 in the control.
    let data = builtin();
    let text = include_str!("../../../scenarios/a2-reward-training.ron");
    let lab = LabScenario::from_ron(text, &data).expect("a valid scenario");
    let (trained, untrained) = after_training(&lab, &data, Verb::Play, "ball");
    let (t, c) = (median(&trained), median(&untrained));
    assert!(
        c >= 20.0,
        "badly calibrated: the control's median is {c} ({untrained:?}), needing 20"
    );
    assert!(
        t >= 1.5 * c,
        "trained {t} ({trained:?}) against the control's {c} ({untrained:?})"
    );
}

#[test]
fn a3_sprites_shocked_for_hitting_each_other_hit_less() {
    // Design v21 §7.3: after training, their applied Hits on sprites, all
    // four combined, are at most half the same seed's never trained, median
    // of 10 seeds, with at least 20 in the control.
    let data = builtin();
    let text = include_str!("../../../scenarios/a3-correct-training.ron");
    let lab = LabScenario::from_ron(text, &data).expect("a valid scenario");
    let (trained, untrained) = after_training(&lab, &data, Verb::Hit, "sprite");
    let (t, c) = (median(&trained), median(&untrained));
    assert!(
        c >= 20.0,
        "badly calibrated: the control's median is {c} ({untrained:?}), needing 20"
    );
    assert!(
        t <= 0.5 * c,
        "trained {t} ({trained:?}) against the control's {c} ({untrained:?})"
    );
}
