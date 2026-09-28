//! M1's acceptance scenarios (design §7.3, §7.4), run from the lab scenario
//! files they share with the lab runner.

use terra_sim::{DataPack, LabScenario, Verb, Window, median};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

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
