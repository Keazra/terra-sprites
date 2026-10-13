//! Inheritance (M2 design §12): what an egg gets from its parents. A pair's
//! genomes line up by what each gene does and mix in stretches, as
//! chromosomes cross over; each gene that can weaken has two copies, one
//! from each parent, either of which may carry a hidden flaw. A lone egg
//! gets its mother's genes, each copy doubled.

use rand_chacha::ChaCha8Rng;

use crate::ageing::Switch;
use crate::data::DataPack;
use crate::genome::{Gene, Genome};
use crate::mutation::longest;
use crate::random::chance;
use crate::variation::each_value;

/// One parent's genes as an egg inherits them (M2 design §12).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Parent<'a> {
    pub(crate) genome: &'a Genome,
    /// How many of its two copies of each gene are flawed, 0 to 2, in
    /// genome order: none where it has no entry.
    pub(crate) flaws: &'a [u8],
}

/// An egg's genes as its parents pass them on, before copying errors.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Inherited {
    pub(crate) genome: Genome,
    /// How many of its two copies of each gene are flawed, in genome order:
    /// empty when none are.
    pub(crate) flaws: Vec<u8>,
}

/// How many of the two copies of the gene at `index` are flawed: none where
/// `flaws` has no entry for it.
pub(crate) fn flaws_of(flaws: &[u8], index: usize) -> u8 {
    flaws.get(index).copied().unwrap_or(0)
}

/// `flaws`, or none at all when no copy is flawed, so a sprite without
/// flaws saves and hashes as one from before them.
pub(crate) fn tidied(flaws: Vec<u8>) -> Vec<u8> {
    if flaws.iter().all(|&flawed| flawed == 0) {
        Vec::new()
    } else {
        flaws
    }
}

/// What a gene does, whatever its values: what it refers to and how it
/// works, and the life stages it works in.
fn what_it_does(genome: &Genome, index: usize, data: &DataPack) -> (Gene, Switch) {
    let gene = each_value(&genome.genes[index], data, |_, _| 0.0);
    (gene, genome.switch(index))
}

/// Where a child's gene comes from.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// A gene both parents have: the mother's at the first index, lined up
    /// with the father's at the second.
    Both(usize, usize),
    /// A gene only the mother has.
    Mothers(usize),
    /// A gene only the father has.
    Fathers(usize),
}

/// The child's genes in order, each from where it comes (M2 design §12):
/// the nth gene of the father's that does something lines up with the nth
/// of the mother's that does it. The child has the mother's genes in her
/// order, and each gene only the father has after the one it follows in
/// his.
fn slots(mother: &Genome, father: &Genome, data: &DataPack) -> Vec<Slot> {
    let hers: Vec<_> = (0..mother.genes.len())
        .map(|index| what_it_does(mother, index, data))
        .collect();
    let mut partner = vec![None; hers.len()];
    // Each gene only the father has, with the mother's gene it follows.
    let mut his_own: Vec<(Option<usize>, usize)> = Vec::new();
    let mut follows = None;
    for index in 0..father.genes.len() {
        let does = what_it_does(father, index, data);
        let lined_up = (0..hers.len()).find(|&at| partner[at].is_none() && hers[at] == does);
        match lined_up {
            Some(at) => {
                partner[at] = Some(index);
                follows = Some(at);
            }
            None => his_own.push((follows, index)),
        }
    }
    let after = |at: Option<usize>| {
        his_own
            .iter()
            .filter(move |&&(follows, _)| follows == at)
            .map(|&(_, index)| Slot::Fathers(index))
    };
    let mut slots: Vec<Slot> = after(None).collect();
    for (at, partner) in partner.iter().enumerate() {
        slots.push(match *partner {
            Some(index) => Slot::Both(at, index),
            None => Slot::Mothers(at),
        });
        slots.extend(after(Some(at)));
    }
    slots
}

/// The copy of a gene a parent with `flawed` flawed copies passes on: a
/// flawed one or not, at random where it has one of each.
fn passed(flawed: u8, rng: &mut ChaCha8Rng) -> u8 {
    match flawed {
        0 => 0,
        1 => u8::from(chance(rng, 0.5)),
        _ => 1,
    }
}

/// A pair's egg's genes (M2 design §12). Its genome lines up by what each
/// gene does and comes from one parent in stretches: it starts with either,
/// and at each gene switches to the other with `breeding.crossover`'s
/// chance. A gene only one parent has goes to the child when its stretch is
/// from that parent, so to about half the young. Each gene gets one copy
/// from each parent, either flawed or not; a gene only one parent has gets
/// a sound copy from the other. A genome never grows past `longest` (M2
/// design §13): genes only one parent has go, the last first, until it fits.
pub(crate) fn of_pair(
    mother: Parent,
    father: Parent,
    data: &DataPack,
    rng: &mut ChaCha8Rng,
) -> Inherited {
    let crossover = data.physiology().breeding.crossover;
    let (mut genes, mut switches, mut flaws) = (Vec::new(), Vec::new(), Vec::new());
    // Whether each of the child's genes is one only one parent has.
    let mut one_has = Vec::new();
    let mut from_mother = chance(rng, 0.5);
    for (nth, slot) in slots(mother.genome, father.genome, data)
        .into_iter()
        .enumerate()
    {
        if nth > 0 && chance(rng, crossover) {
            from_mother = !from_mother;
        }
        let (hers, his) = match slot {
            Slot::Both(hers, his) => (Some(hers), Some(his)),
            Slot::Mothers(hers) => (Some(hers), None),
            Slot::Fathers(his) => (None, Some(his)),
        };
        let from = if from_mother {
            hers.map(|index| (mother.genome, index))
        } else {
            his.map(|index| (father.genome, index))
        };
        // A gene only one parent has goes to the child in that parent's
        // stretch.
        let Some((genome, index)) = from else {
            continue;
        };
        genes.push(genome.genes[index].clone());
        switches.push(genome.switch(index));
        let copy = |parent: Parent, at: Option<usize>, rng: &mut ChaCha8Rng| {
            at.map_or(0, |at| passed(flaws_of(parent.flaws, at), rng))
        };
        flaws.push(copy(mother, hers, rng) + copy(father, his, rng));
        one_has.push(hers.is_none() || his.is_none());
    }
    let mut over = genes.len().saturating_sub(longest(data));
    for index in (0..genes.len()).rev() {
        if over == 0 {
            break;
        }
        if one_has[index] {
            genes.remove(index);
            switches.remove(index);
            flaws.remove(index);
            over -= 1;
        }
    }
    Inherited {
        genome: Genome::switched(genes, switches),
        flaws: tidied(flaws),
    }
}

/// A lone egg's genes (M2 design §12): its mother's, with one of her two
/// copies of each gene, doubled, as a Komodo dragon's are. So each flaw she
/// carries hidden is broken in about half her lone young.
pub(crate) fn alone(mother: Parent, rng: &mut ChaCha8Rng) -> Inherited {
    let flaws = (0..mother.genome.genes.len())
        .map(|index| 2 * passed(flaws_of(mother.flaws, index), rng))
        .collect();
    Inherited {
        genome: mother.genome.clone(),
        flaws: tidied(flaws),
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use super::*;
    use crate::test_util::builtin;

    /// A genome of `genes`, written as a genome file's.
    fn genome(genes: &[String], data: &DataPack) -> Genome {
        let genes = genes.join(",\n");
        Genome::from_ron(&format!("(format: 2, genes: [{genes}])"), data).expect("a valid genome")
    }

    /// An instinct to `verb` with `weight`.
    fn instinct(verb: &str, weight: f32) -> String {
        format!(r#"Instinct(inputs: [("always", false)], verb: {verb}, weight: {weight:?})"#)
    }

    /// One instinct for each of `verbs`, each with `weight`.
    fn instincts(verbs: &[&str], weight: f32) -> Vec<String> {
        verbs.iter().map(|verb| instinct(verb, weight)).collect()
    }

    const VERBS: [&str; 8] = [
        "Eat", "Drink", "Play", "Hit", "Rest", "Wander", "Mate", "Approach",
    ];

    fn sound(genome: &Genome) -> Parent<'_> {
        Parent { genome, flaws: &[] }
    }

    fn weight(gene: &Gene) -> f32 {
        match *gene {
            Gene::Instinct { weight, .. } => weight,
            _ => panic!("not an instinct: {gene:?}"),
        }
    }

    #[test]
    fn a_child_takes_its_genes_from_one_parent_in_stretches() {
        // M2 design §12: at each gene, a child switches to the other parent
        // with the chance `breeding.crossover` (0.1), so neighbouring genes
        // tend to come from the same parent.
        let data = builtin();
        let mine = genome(&vec![instinct("Eat", 0.25); 60], &data);
        let theirs = genome(&vec![instinct("Eat", 0.75); 60], &data);
        let (mut switched, mut boundaries, mut hers, mut all) = (0, 0, 0, 0);
        for seed in 0..200 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let child = of_pair(sound(&mine), sound(&theirs), &data, &mut rng);
            let weights: Vec<f32> = child.genome.genes.iter().map(weight).collect();
            assert_eq!(weights.len(), 60);
            switched += weights.windows(2).filter(|pair| pair[0] != pair[1]).count();
            boundaries += weights.len() - 1;
            hers += weights.iter().filter(|&&weight| weight == 0.25).count();
            all += weights.len();
        }
        let switches = switched as f32 / boundaries as f32;
        assert!(
            (0.08..0.12).contains(&switches),
            "{switched} of {boundaries}"
        );
        let share = hers as f32 / all as f32;
        assert!((0.45..0.55).contains(&share), "{hers} of {all} were hers");
    }

    #[test]
    fn parents_line_up_by_what_each_gene_does_not_where_it_is() {
        // M2 design §12: the child has its mother's genes in her order, each
        // one hers or the father's that does the same.
        let data = builtin();
        let mine = data.starter().clone();
        let theirs = {
            let mut genes: Vec<Gene> = mine
                .genes
                .iter()
                .map(|gene| each_value(gene, &data, |value, _| value * 0.9))
                .collect();
            let mut switches: Vec<Switch> =
                (0..genes.len()).map(|index| mine.switch(index)).collect();
            genes.reverse();
            switches.reverse();
            Genome::switched(genes, switches)
        };
        let mut from_his = 0;
        for seed in 0..20 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let child = of_pair(sound(&mine), sound(&theirs), &data, &mut rng).genome;
            assert_eq!(child.genes.len(), mine.genes.len());
            for (index, gene) in child.genes.iter().enumerate() {
                let does = what_it_does(&child, index, &data);
                assert_eq!(does, what_it_does(&mine, index, &data), "gene {index}");
                let his = (0..theirs.genes.len()).any(|at| {
                    theirs.genes[at] == *gene && what_it_does(&theirs, at, &data) == does
                });
                assert!(
                    *gene == mine.genes[index] || his,
                    "gene {index} is neither's"
                );
                from_his += usize::from(*gene != mine.genes[index]);
            }
        }
        assert!(
            from_his > 0,
            "the father's genes lined up with none of hers"
        );
    }

    #[test]
    fn a_gene_only_one_parent_has_goes_to_about_half_the_young_where_it_was() {
        // M2 design §12: his extra gene follows the gene it followed in his
        // genome; hers stays where it was in hers.
        let data = builtin();
        let mine = genome(&instincts(&VERBS, 0.25), &data);
        let mut his = instincts(&VERBS, 0.75);
        his.insert(
            3,
            r#"AttentionInstinct(input: "thirst", category: "water", weight: 0.8)"#.into(),
        );
        // Hers alone: he has no instinct to approach.
        his.retain(|gene| !gene.contains("Approach"));
        let theirs = genome(&his, &data);
        let (mut his_extra, mut her_extra) = (0, 0);
        for seed in 0..1_000 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let child = of_pair(sound(&mine), sound(&theirs), &data, &mut rng).genome;
            if let Some(at) = child
                .genes
                .iter()
                .position(|gene| matches!(gene, Gene::AttentionInstinct { .. }))
            {
                his_extra += 1;
                assert_eq!(at, 3, "after the instinct to hit, as in his");
            }
            her_extra += usize::from(child.genes.iter().any(|gene| {
                matches!(gene, Gene::Instinct { verb, .. } if *verb == crate::registry::Verb::Approach)
            }));
        }
        assert!(
            (400..600).contains(&his_extra),
            "his gene in {his_extra} of 1,000"
        );
        assert!(
            (400..600).contains(&her_extra),
            "her gene in {her_extra} of 1,000"
        );
    }

    #[test]
    fn a_child_gets_one_copy_of_each_gene_from_each_parent_flawed_or_not() {
        // M2 design §12: a parent with one flawed copy passes it to about
        // half its young; one with two, to all.
        let data = builtin();
        let mine = genome(&instincts(&VERBS, 0.25), &data);
        let theirs = genome(&instincts(&VERBS, 0.75), &data);
        let pair = |her: u8, his: u8, seed: u64| {
            let (hers, theirs_flaws) = (vec![her; 8], vec![his; 8]);
            let mother = Parent {
                genome: &mine,
                flaws: &hers,
            };
            let father = Parent {
                genome: &theirs,
                flaws: &theirs_flaws,
            };
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let flaws = of_pair(mother, father, &data, &mut rng).flaws;
            (0..8)
                .map(|index| flaws_of(&flaws, index))
                .collect::<Vec<u8>>()
        };
        assert_eq!(pair(2, 2, 1), vec![2; 8], "both carry it doubled");
        assert_eq!(pair(2, 0, 1), vec![1; 8], "one carries it doubled");
        assert_eq!(pair(0, 0, 1), vec![0; 8]);
        let (mut carried, mut broken, mut all) = (0, 0, 0);
        for seed in 0..500 {
            for flawed in pair(1, 1, seed) {
                carried += usize::from(flawed == 1);
                broken += usize::from(flawed == 2);
                all += 1;
            }
        }
        // Two carriers' young: a quarter broken, half carriers.
        let (carried, broken) = (carried as f32 / all as f32, broken as f32 / all as f32);
        assert!((0.45..0.55).contains(&carried), "{carried} carriers");
        assert!((0.2..0.3).contains(&broken), "{broken} broken");
    }

    #[test]
    fn a_lone_egg_gets_one_of_its_mothers_copies_doubled() {
        // M2 design §12: so each flaw she carries is broken in about half
        // her lone young, and none is carried hidden.
        let data = builtin();
        let mine = genome(&instincts(&VERBS, 0.25), &data);
        let hers = [0, 1, 2, 1, 0, 1, 2, 1];
        let mut broken = 0;
        for seed in 0..500 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let mother = Parent {
                genome: &mine,
                flaws: &hers,
            };
            let child = alone(mother, &mut rng);
            assert_eq!(child.genome, mine);
            for (index, &her) in hers.iter().enumerate() {
                let flawed = flaws_of(&child.flaws, index);
                match her {
                    0 => assert_eq!(flawed, 0),
                    2 => assert_eq!(flawed, 2),
                    _ => {
                        assert_ne!(flawed, 1, "a lone egg carries nothing hidden");
                        broken += usize::from(flawed == 2);
                    }
                }
            }
        }
        // Four genes she carries, in 500 eggs.
        assert!((900..1_100).contains(&broken), "{broken} of 2,000 broken");
    }

    #[test]
    fn a_pair_s_child_never_has_more_genes_than_twice_the_starter_s() {
        // M2 design §13: genes only one parent has go, the last first.
        let data = builtin();
        let longest = 2 * data.starter().genes.len();
        let mine = genome(&vec![instinct("Eat", 0.25); longest], &data);
        let theirs = genome(&vec![instinct("Drink", 0.75); longest], &data);
        let mut trimmed = 0;
        for seed in 0..100 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let child = of_pair(sound(&mine), sound(&theirs), &data, &mut rng).genome;
            assert!(child.genes.len() <= longest, "{}", child.genes.len());
            trimmed += usize::from(child.genes.len() == longest);
        }
        assert!(trimmed > 0, "no child would have grown past it");
    }

    #[test]
    fn a_sound_childs_flaws_are_none_at_all() {
        // A sprite without flaws saves and hashes as one from before them.
        let data = builtin();
        let mine = genome(&instincts(&VERBS, 0.25), &data);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        assert!(alone(sound(&mine), &mut rng).flaws.is_empty());
        let child = of_pair(sound(&mine), sound(&mine), &data, &mut rng);
        assert!(child.flaws.is_empty());
    }
}
