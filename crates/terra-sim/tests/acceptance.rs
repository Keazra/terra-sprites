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

#[test]
fn a1_a_sprite_learns_to_keep_away_from_thornbushes() {
    let data = builtin();
    let text = include_str!("../../../scenarios/a1-thornbush.ron");
    let lab = LabScenario::from_ron(text, &data).expect("a valid scenario");
    let (mut first, mut last) = (Vec::new(), Vec::new());
    for seed in 1..=10 {
        let run = lab.run(data.clone(), seed);
        let windows = &run.windows;
        // A dead sprite touches no thornbushes, which would pass hollowly.
        let deaths: Vec<_> = windows.iter().flat_map(|w| w.deaths.keys()).collect();
        assert!(
            deaths.is_empty(),
            "seed {seed}: the sprite died: {deaths:?}"
        );
        first.push(thornbush_contacts(&windows[0], &data));
        last.push(thornbush_contacts(&windows[windows.len() - 1], &data));
    }
    let (early, late) = (median(&first), median(&last));
    assert!(
        early >= 20.0,
        "badly calibrated: a median of {early} contacts in ticks 0–5,000 ({first:?}), needing 20"
    );
    assert!(
        late <= early / 2.0,
        "ticks 15,000–20,000 had a median of {late} contacts ({last:?}), against {early} ({first:?})"
    );
}
