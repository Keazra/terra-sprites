//! Inheritance (M2 design §12): a child's genes are its parents', copied
//! with now and then an error in a gene value, never in what a gene does;
//! and copying errors are world state, saved and replayed.

mod common;
use common::{ALONE, FEMALE, Laying, at, builtin_changing, family_in, laid};

use terra_sim::{DataPack, EventKind, GeneView, Playback, World};

/// The built-in pack, with every gene value copied wrongly.
fn always_wrong() -> DataPack {
    builtin_changing(
        "physiology.ron",
        &[("mutation: (chance: 0.04,", "mutation: (chance: 1.0,")],
    )
}

/// What `gene` does, without its values: its kind and what it refers to.
fn what_it_does(gene: &GeneView) -> String {
    format!("{gene:?}")
        .chars()
        .filter(|c| !(c.is_ascii_digit() || *c == '.' || *c == '-'))
        .collect()
}

#[test]
fn a_lone_egg_s_genes_are_its_mother_s_with_copying_errors_in_their_values() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(always_wrong(), &[(at(2, 1), &female)], &[], 1);
    let Laying { egg, by, .. } = laid(&mut world, 50).expect("she laid an egg");
    let (egg, mother) = (world.sprite(egg).unwrap(), world.sprite(by).unwrap());
    assert_ne!(
        egg.genome(),
        mother.genome(),
        "her genes were copied wrongly"
    );
    let (child, hers) = (egg.genes(), mother.genes());
    assert_eq!(child.len(), hers.len());
    for ((gene, how), (theirs, how_theirs)) in child.iter().zip(&hers) {
        assert_eq!(what_it_does(gene), what_it_does(theirs));
        assert_eq!(how, how_theirs);
    }
}

#[test]
fn copying_errors_load_and_replay_the_same() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(always_wrong(), &[(at(2, 1), &female)], &[], 3);
    world.start_recording();
    let mut loaded = World::load(&world.save()).expect("the save loads");
    let mut eggs = 0;
    for _ in 0..1_000 {
        let events = world.step();
        assert_eq!(events, loaded.step());
        eggs += events
            .iter()
            .filter(|event| matches!(event.kind, EventKind::Laid { .. }))
            .count();
    }
    assert!(eggs >= 1, "an egg was laid");
    assert_eq!(loaded.state_hash(), world.state_hash());
    let mut playback = Playback::new(&world.replay().unwrap()).expect("the replay plays");
    while playback.world().tick() < playback.end() {
        playback.step();
    }
    assert_eq!(playback.divergence(), None);
    assert_eq!(playback.world().state_hash(), world.state_hash());
}
