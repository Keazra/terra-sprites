//! Inheritance (M2 design §12, §13): a child's genes are its parents',
//! copied with now and then an error in a gene value, never in what a gene
//! does, and now and then a hidden flaw; a gene flawed in both copies is
//! broken. Now and then too a whole gene is doubled, lost or changed in
//! what it does. Copying errors, flaws and copy changes are world state,
//! saved and replayed.

mod common;
use common::{ALONE, FEMALE, LONE, Laying, at, builtin, builtin_changing, family_in, hatch, laid};

use terra_sim::{
    Command, CopyChangeView, DataPack, EventKind, GeneView, Playback, World, WorldConfig,
};

/// The change to the built-in physiology that copies every gene value
/// wrongly.
const EVERY_VALUE_WRONG: (&str, &str) = ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,");

/// The built-in physiology's copy and change, as its file begins it.
const COPY_AND_CHANGE: &str =
    "copy_and_change: (doubled: 0.00039, lost: 0.00039, changed: 0.00039,";

/// The change to the built-in physiology that copies whole genes as
/// `rules` say.
fn whole_genes(rules: &str) -> (&'static str, String) {
    (COPY_AND_CHANGE, format!("copy_and_change: ({rules},"))
}

/// The built-in pack, with every gene value copied wrongly, and every
/// whole gene kept where it is. In each of these packs a female may breed
/// alone at once (M2 design §15.7).
fn always_wrong() -> DataPack {
    let (from, to) = whole_genes("doubled: 0.0, lost: 0.0, changed: 0.0");
    builtin_changing("physiology.ron", &[EVERY_VALUE_WRONG, (from, &to), LONE])
}

/// The built-in pack, with `changes` to its hidden flaws' line, and every
/// whole gene kept where it is.
fn with_flaws(changes: &str) -> DataPack {
    let (from, to) = whole_genes("doubled: 0.0, lost: 0.0, changed: 0.0");
    builtin_changing(
        "physiology.ron",
        &[
            (
                "flaws: (arise: 0.005, founders: 0.043,",
                &format!("flaws: ({changes}"),
            ),
            (from, &to),
            LONE,
        ],
    )
}

/// What `gene` does, without its values: its kind and what it refers to.
/// Its numbers are dropped, but not digits in a quoted name, such as `h3`.
fn what_it_does(gene: &GeneView) -> String {
    let mut quoted = false;
    format!("{gene:?}")
        .chars()
        .filter(|&c| {
            quoted ^= c == '"';
            quoted || !(c.is_ascii_digit() || c == '.' || c == '-')
        })
        .collect()
}

#[test]
fn a_lone_egg_s_genes_are_its_mother_s_with_copying_errors_in_their_values() {
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(always_wrong(), &[(at(2, 1), &female)], &[], 1);
    let Laying { egg, by, .. } = laid(&mut world, 50).expect("she laid an egg");
    hatch(&mut world, egg);
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
fn an_egg_knows_which_genes_copying_changed_and_what_each_was() {
    // M2 design §12: for the Genome tab's list of what's new.
    let female = format!("{ALONE}{FEMALE}");
    let mut world = family_in(always_wrong(), &[(at(2, 1), &female)], &[], 1);
    let Laying { egg, by, .. } = laid(&mut world, 50).expect("she laid an egg");
    hatch(&mut world, egg);
    let (egg, mother) = (world.sprite(egg).unwrap(), world.sprite(by).unwrap());
    let (genes, hers) = (egg.genes(), mother.genes());
    let errors = egg.copying_errors();
    assert!(!errors.is_empty());
    for (index, was) in &errors {
        assert_eq!(*was, hers[*index].0, "a lone egg inherits her genes");
        assert_ne!(genes[*index].0, *was, "gene {index} was changed");
    }
    let changed = (0..genes.len()).filter(|&index| genes[index].0 != hers[index].0);
    assert_eq!(changed.count(), errors.len());
    assert!(
        mother.copying_errors().is_empty(),
        "the world began with her"
    );
}

#[test]
fn a_gene_flawed_in_both_copies_is_born_broken_at_half_health_and_brings_old_age_sooner() {
    // M2 design §12: with every copy flawed as it's copied, each of the
    // egg's genes that can weaken is broken.
    let female = format!("{ALONE}{FEMALE}");
    let data = with_flaws("arise: 1.0, founders: 0.043,");
    let mut world = family_in(data, &[(at(2, 1), &female)], &[], 1);
    let Laying { egg, by, .. } = laid(&mut world, 50).expect("she laid an egg");
    hatch(&mut world, egg);
    let (egg, mother) = (world.sprite(egg).unwrap(), world.sprite(by).unwrap());
    let (flaws, health) = (egg.flaws(), egg.gene_health());
    let broken = flaws.iter().filter(|&&flawed| flawed == 2).count();
    assert_eq!(broken, 3, "speed, sense and breeding alone: {flaws:?}");
    for (index, &flawed) in flaws.iter().enumerate() {
        assert!(flawed == 0 || flawed == 2, "gene {index} is half flawed");
        let born_with = if flawed == 2 { 0.5 } else { 1.0 };
        assert_eq!(health[index], born_with, "gene {index}");
    }
    assert!(egg.vigour() < 1.0);
    // Each broken gene counts as 0.05 of wear, and the most wear brings
    // old age a third sooner.
    let sooner = 1.0 - egg.grows_old_at() / egg.traits().grows_old;
    let expected = 0.33 * (0.05 * broken as f32).min(1.0);
    assert!(
        (sooner - expected).abs() < 1e-4,
        "{sooner} sooner, not {expected}"
    );
    assert!(mother.flaws().iter().all(|&flawed| flawed == 0));
    assert_eq!(mother.grows_old_at(), mother.traits().grows_old);
}

#[test]
fn the_first_sprites_carry_a_few_hidden_flaws_and_none_broken() {
    // M2 design §12: as wild animals do, about three each.
    let data = builtin();
    let config = WorldConfig::builtin(&data);
    let world = World::new(config, data, 7);
    let mut carried = 0;
    let mut sprites = 0;
    for sprite in world.sprites() {
        sprites += 1;
        for (index, &flawed) in sprite.flaws().iter().enumerate() {
            assert!(flawed <= 1, "a first sprite's gene {index} is broken");
            carried += usize::from(flawed);
        }
        assert!(sprite.gene_health().iter().all(|&health| health == 1.0));
    }
    let each = carried as f32 / sprites as f32;
    assert!((1.5..5.0).contains(&each), "{each} flaws each");
}

#[test]
fn a_sprite_placed_from_the_starter_carries_hidden_flaws_and_one_from_a_file_none() {
    // M2 design §12: one from a genome file has the genes it was given.
    let data = with_flaws("arise: 0.005, founders: 1.0,");
    let mut world = family_in(data, &[(at(0, 0), "")], &[], 1);
    let file = world.sprite_at(at(0, 0)).unwrap().genome().clone();
    for (tile, genome) in [(at(3, 2), None), (at(5, 2), Some(file))] {
        world.submit(Command::SpawnSprite { tile, genome });
    }
    world.step();
    let wild = world.sprite_at(at(3, 2)).expect("the starter's sprite");
    let flaws = wild.flaws();
    assert!(flaws.iter().all(|&flawed| flawed <= 1));
    assert!(flaws.iter().filter(|&&flawed| flawed == 1).count() > 20);
    let filed = world.sprite_at(at(5, 2)).expect("the file's sprite");
    assert!(filed.flaws().iter().all(|&flawed| flawed == 0));
}

#[test]
fn a_lone_egg_s_genes_are_now_and_then_doubled_lost_or_changed() {
    // M2 design §13: a doubled gene or a changed copy sits next to its
    // original, and the egg remembers what it lost and what a changed gene
    // was.
    let female = format!("{ALONE}{FEMALE}");
    let (from, to) = whole_genes("doubled: 0.1, lost: 0.1, changed: 0.1");
    let data = builtin_changing("physiology.ron", &[(from, &to), LONE]);
    let mut world = family_in(data, &[(at(2, 1), &female)], &[], 1);
    let Laying { egg, by, .. } = laid(&mut world, 50).expect("she laid an egg");
    hatch(&mut world, egg);
    let (egg, mother) = (world.sprite(egg).unwrap(), world.sprite(by).unwrap());
    let (child, hers) = (egg.genes(), mother.genes());
    let changes = egg.copy_changes();
    let count = |kind: fn(&CopyChangeView) -> bool| changes.iter().filter(|c| kind(c)).count();
    let doubled = count(|c| matches!(c, CopyChangeView::Doubled { .. }));
    let lost = count(|c| matches!(c, CopyChangeView::Lost { .. }));
    let copies = count(|c| matches!(c, CopyChangeView::Changed { on_copy: true, .. }));
    assert!(doubled > 0 && lost > 0 && copies > 0, "{changes:?}");
    assert_eq!(child.len(), hers.len() + doubled + copies - lost);
    for change in &changes {
        match *change {
            CopyChangeView::Doubled { gene } => {
                assert_eq!(
                    what_it_does(&child[gene].0),
                    what_it_does(&child[gene - 1].0)
                );
            }
            CopyChangeView::Lost { ref was, .. } => {
                assert!(
                    hers.iter()
                        .any(|(gene, _)| what_it_does(gene) == what_it_does(was))
                );
            }
            CopyChangeView::Changed {
                gene,
                ref was,
                on_copy,
                ..
            } => {
                assert!(
                    hers.iter()
                        .any(|(gene, _)| what_it_does(gene) == what_it_does(was))
                );
                if on_copy {
                    assert_eq!(what_it_does(&child[gene - 1].0), what_it_does(was));
                }
            }
        }
    }
    assert!(mother.copy_changes().is_empty(), "she has no parents");
}

#[test]
fn copying_errors_flaws_and_copy_changes_load_and_replay_the_same() {
    let female = format!("{ALONE}{FEMALE}");
    let (from, to) = whole_genes("doubled: 0.1, lost: 0.1, changed: 0.1");
    let data = builtin_changing(
        "physiology.ron",
        &[
            EVERY_VALUE_WRONG,
            ("flaws: (arise: 0.005,", "flaws: (arise: 0.5,"),
            (from, &to),
            LONE,
        ],
    );
    let mut world = family_in(data, &[(at(2, 1), &female)], &[], 3);
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
