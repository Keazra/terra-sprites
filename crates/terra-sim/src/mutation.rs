//! Copying (M2 design §12, §13): as an egg's genes are copied, now and
//! then a gene value changes, never what a gene refers to or how it works,
//! and now and then a copy of a gene gets a hidden flaw. Now and then too a
//! whole gene is doubled, lost, or changed in what it does. And the flaws
//! the sprites the world begins with carry.

use std::iter::once;

use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::ageing::{LifeStage, Switch, frailty};
use crate::data::DataPack;
use crate::expression::{Expression, breaks_restrictions, breaks_switching, expressions};
use crate::genome::{Gene, GeneView, Genome, Whom};
use crate::inheritance::{Built, Inherited, flaws_of, longest, tidied, what_it_does};
use crate::random::{chance, uniform, unit};
use crate::registry::Verb;
use crate::variation::{each_value, varied};

/// A gene that copying changed in an egg (M2 design §12): where it is in
/// the egg's genome, and what it was as the egg inherited it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CopyingError {
    pub(crate) gene: usize,
    pub(crate) was: Gene,
}

/// A whole gene that copying doubled, lost or changed in what it does in an
/// egg (M2 design §13).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum CopyChange {
    /// The gene at `gene` in the egg's genome is a copy of the one before it.
    Doubled { gene: usize },
    /// A gene the egg inherited and lost: what it was, and when it worked.
    Lost { was: Gene, switch: Switch },
    /// The gene at `gene` in the egg's genome was changed in what it does:
    /// what it was as the egg inherited it, and when it worked. Changed `on_copy`,
    /// the original is the gene before it.
    Changed {
        gene: usize,
        was: Gene,
        switch: Switch,
        on_copy: bool,
    },
}

impl CopyChange {
    /// The change with what genes refer to named from `data`.
    pub(crate) fn view<'a>(&'a self, data: &'a DataPack) -> CopyChangeView<'a> {
        match *self {
            CopyChange::Doubled { gene } => CopyChangeView::Doubled { gene },
            CopyChange::Lost { ref was, switch } => CopyChangeView::Lost {
                was: was.view(data),
                switch,
            },
            CopyChange::Changed {
                gene,
                ref was,
                switch,
                on_copy,
            } => CopyChangeView::Changed {
                gene,
                was: was.view(data),
                switch,
                on_copy,
            },
        }
    }
}

/// A whole gene that copying doubled, lost or changed in what it does as a
/// sprite was laid (M2 design §13), with what genes refer to by the data
/// pack's names.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyChangeView<'a> {
    /// The gene at `gene` in its genome is a copy of the one before it.
    Doubled { gene: usize },
    /// A gene it inherited and lost: what it was, and when it worked.
    Lost { was: GeneView<'a>, switch: Switch },
    /// The gene at `gene` in its genome was changed in what it does: what it
    /// was as it inherited it, and when it worked. Changed `on_copy`, the
    /// original is the gene before it, and still works.
    Changed {
        gene: usize,
        was: GeneView<'a>,
        switch: Switch,
        on_copy: bool,
    },
}

/// An egg's genes once copied (M2 design §12).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Copied {
    pub(crate) genome: Genome,
    /// How many of its two copies of each gene are flawed, in genome order:
    /// empty when none are.
    pub(crate) flaws: Vec<u8>,
    /// The genes copying changed, in genome order.
    pub(crate) errors: Vec<CopyingError>,
    /// The whole genes copying doubled, lost or changed, in the order it
    /// came to them (M2 design §13).
    pub(crate) changes: Vec<CopyChange>,
}

/// The genes an egg `inherited`, as copied into it (M2 design §12, §13).
/// First whole genes are now and then doubled, lost or changed in what they
/// do, as `whole_genes_copied` says. Then each gene value has physiology's
/// `mutation.chance` of changing, by a large factor or, now and then, a
/// small one, then is held within its range as spawn variation's are;
/// values are drawn in genome order. Only values change, so a flagged gene
/// stays flagged and no copying error can make a gene change the body
/// directly. Then each sound copy of each gene that can weaken gets a flaw
/// with `flaws.arise`'s chance. A pack without copy and change, copying
/// errors or flaws copies genes exactly, drawing nothing.
pub(crate) fn copied(inherited: Inherited, data: &DataPack, rng: &mut ChaCha8Rng) -> Copied {
    let (Inherited { genome, flaws }, changes) = whole_genes_copied(inherited, data, rng);
    let (genome, errors) = values_copied(genome, data, rng);
    let flaws = flawed(&genome, flaws, data, rng);
    Copied {
        genome,
        flaws,
        errors,
        changes,
    }
}

/// `inherited`'s genes with whole ones doubled, lost or changed in what
/// they do (M2 design §13), and the changes. One draw for each gene decides
/// which, if any, happens to it, with `copy_and_change`'s chances, so at
/// most one does. A doubled gene's copy goes next to it, with its switch
/// and its flaws, so the two tend to be inherited together. A changed gene
/// is changed on a fresh copy next to it, with the chance `on_copy`, so the
/// original still works, or else in its place. A copy is made only while
/// the genome stays within `longest`, and a genome never loses the last
/// gene it can read. A gene this build can't read, or one that names a
/// category the world lacks, is kept exactly as it is (M1 design §2.8, v19
/// §5.7), drawing nothing. A pack without copy and change draws nothing.
fn whole_genes_copied(
    inherited: Inherited,
    data: &DataPack,
    rng: &mut ChaCha8Rng,
) -> (Inherited, Vec<CopyChange>) {
    let rules = data.physiology().copy_and_change;
    if rules.doubled + rules.lost + rules.changed == 0.0 {
        return (inherited, Vec::new());
    }
    // Where each kind's share of a draw ends.
    let lost_below = rules.lost;
    let doubled_below = lost_below + rules.doubled;
    let changed_below = doubled_below + rules.changed;
    let longest = longest(data);
    let Inherited { genome, flaws } = inherited;
    let count = genome.genes.len();
    let readable = |gene: &Gene| !matches!(gene, Gene::Unknown { .. } | Gene::Unmatched(_));
    let last_readable = genome.genes.iter().rposition(readable);
    let (mut built, mut changes) = (Built::default(), Vec::new());
    let mut kept_readable = false;
    for (index, gene) in genome.genes.iter().enumerate() {
        let (switch, flawed) = (genome.switch(index), flaws_of(&flaws, index));
        if !readable(gene) {
            built.push(gene.clone(), switch, flawed);
            continue;
        }
        let roll = unit(rng);
        let last = !kept_readable && Some(index) == last_readable;
        if roll < lost_below && !last {
            changes.push(CopyChange::Lost {
                was: gene.clone(),
                switch,
            });
            continue;
        }
        kept_readable = true;
        built.push(gene.clone(), switch, flawed);
        // Room for a copy, with each gene still to come kept.
        let room = built.len() + 1 + (count - index - 1) <= longest;
        if (lost_below..doubled_below).contains(&roll) && room {
            built.push(gene.clone(), switch, flawed);
            changes.push(CopyChange::Doubled {
                gene: built.len() - 1,
            });
        } else if (doubled_below..changed_below).contains(&roll)
            && let Some((new, new_switch)) = changed(gene, switch, data, rng)
        {
            let on_copy = room && chance(rng, rules.on_copy);
            if !on_copy {
                built.pop();
            }
            built.push(new, new_switch, flawed);
            changes.push(CopyChange::Changed {
                gene: built.len() - 1,
                was: gene.clone(),
                switch,
                on_copy,
            });
        }
    }
    (built.inherited(), changes)
}

/// One thing a gene does that copying can change (M2 design §13).
#[derive(Debug, Clone, Copy)]
enum Aspect {
    /// One of an instinct's inputs.
    Input,
    /// An instinct's verb.
    Verb,
    /// The input that draws an attention instinct's eye.
    AttentionInput,
    /// What an attention instinct draws the eye to.
    Category,
    /// Whom in it an attention instinct is about (§15.6).
    Whom,
    /// The life stages it works in.
    Switch,
}

/// `gene`, working as `switch`, changed in one thing it does (M2 design
/// §13), with when it then works: one of an instinct's inputs or its verb,
/// what draws an attention instinct's eye or what to, or whom in it
/// (§15.6), or the life stages a gene works in, each as likely as the
/// others. A gene that may not switch, such as a trait (§9.2), has no life
/// stages to change. Each choice is one the pack has, other than the one
/// the gene had, and none makes a gene change the body directly (M1 design
/// §4.3). None for a gene with nothing to change, or a change the pack
/// can't have, which isn't made.
fn changed(
    gene: &Gene,
    switch: Switch,
    data: &DataPack,
    rng: &mut ChaCha8Rng,
) -> Option<(Gene, Switch)> {
    let mut aspects = match gene {
        Gene::Instinct { .. } => vec![Aspect::Input, Aspect::Verb],
        Gene::AttentionInstinct { .. } => {
            vec![Aspect::AttentionInput, Aspect::Category, Aspect::Whom]
        }
        _ => Vec::new(),
    };
    let switched = Switch {
        from: LifeStage::Adult,
        until: None,
    };
    if breaks_switching(gene, switched).is_none() {
        aspects.push(Aspect::Switch);
    }
    let aspect = *pick(&aspects, rng)?;
    let inputs = data.brain_inputs_in_order();
    let (mut gene, mut switch) = (gene.clone(), switch);
    match (aspect, &mut gene) {
        (Aspect::Input, Gene::Instinct { inputs: its, .. }) => {
            // One of its inputs, its negation kept.
            let which = uniform(rng, its.len() as u64) as usize;
            let others: Vec<_> = inputs
                .iter()
                .map(|input| input.id)
                .filter(|&id| its.iter().all(|&(has, _)| has != id))
                .collect();
            its[which].0 = *pick(&others, rng)?;
        }
        (Aspect::Verb, Gene::Instinct { verb, .. }) => {
            let others: Vec<Verb> = Verb::ALL
                .into_iter()
                .filter(|other| !other.is_reserved() && other != verb)
                .collect();
            *verb = *pick(&others, rng)?;
        }
        (
            Aspect::AttentionInput,
            Gene::AttentionInstinct {
                input,
                category,
                whom,
                weight,
            },
        ) => {
            // Only inputs it may read (M1 design §4.3).
            let others: Vec<_> = inputs
                .iter()
                .map(|other| other.id)
                .filter(|&other| {
                    let reading = Gene::AttentionInstinct {
                        input: other,
                        category: *category,
                        whom: *whom,
                        weight: *weight,
                    };
                    other != *input && breaks_restrictions(&reading, data).is_none()
                })
                .collect();
            *input = *pick(&others, rng)?;
        }
        (Aspect::Category, Gene::AttentionInstinct { category, .. }) => {
            let others: Vec<_> = data
                .categories()
                .iter()
                .map(|other| other.id)
                .filter(|other| other != category)
                .collect();
            *category = *pick(&others, rng)?;
        }
        (Aspect::Whom, Gene::AttentionInstinct { whom, .. }) => {
            let others: Vec<Whom> = Whom::ALL
                .into_iter()
                .filter(|other| other != whom)
                .collect();
            *whom = *pick(&others, rng)?;
        }
        (Aspect::Switch, _) => {
            let stages = [LifeStage::Young, LifeStage::Adult, LifeStage::Old];
            let others: Vec<Switch> = stages
                .into_iter()
                .flat_map(|from| {
                    once(None)
                        .chain(stages.map(Some))
                        .map(move |until| Switch { from, until })
                })
                .filter(|other| other.ever_on() && *other != switch)
                .collect();
            switch = *pick(&others, rng)?;
        }
        _ => unreachable!("each gene's aspects are its own"),
    }
    gene.check(data).ok()?;
    Some((gene, switch))
}

/// One of `options`, at random: none if there are none.
fn pick<'a, T>(options: &'a [T], rng: &mut ChaCha8Rng) -> Option<&'a T> {
    if options.is_empty() {
        return None;
    }
    Some(&options[uniform(rng, options.len() as u64) as usize])
}

/// Whether `changes` fit `genome` (M2 design §13), or what's wrong: what
/// each lost or changed gene was refers only to what the pack has; each
/// doubled gene does what the gene before it does, in the same life
/// stages; and each changed gene differs from what it was in what it does
/// or when, and one changed on a copy follows a gene that does what it was.
pub(crate) fn check_changes(
    genome: &Genome,
    changes: &[CopyChange],
    data: &DataPack,
) -> Result<(), String> {
    let count = genome.genes.len();
    let its = |index: usize| what_it_does(&genome.genes[index], genome.switch(index), data);
    let fits = |was: &Gene| {
        was.check(data)
            .map_err(|problem| format!("has a copy change whose gene was one that {problem}"))
    };
    for change in changes {
        match *change {
            CopyChange::Doubled { gene } => {
                if !(1..count).contains(&gene) {
                    return Err(format!("has gene {gene} doubled, of {count} genes"));
                }
                if its(gene) != its(gene - 1) {
                    return Err(format!(
                        "has gene {gene} doubled, but it does other than the gene before it"
                    ));
                }
            }
            CopyChange::Lost { ref was, .. } => fits(was)?,
            CopyChange::Changed {
                gene,
                ref was,
                switch,
                on_copy,
            } => {
                let first = usize::from(on_copy);
                if !(first..count).contains(&gene) {
                    return Err(format!("has gene {gene} changed, of {count} genes"));
                }
                fits(was)?;
                let was = what_it_does(was, switch, data);
                if its(gene) == was {
                    return Err(format!("has gene {gene} changed, but it does what it did"));
                }
                if on_copy && its(gene - 1) != was {
                    return Err(format!(
                        "has gene {gene} changed on a copy, but the gene before it isn't the original"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// `genome` with its values copied, and the genes copying changed.
fn values_copied(
    genome: Genome,
    data: &DataPack,
    rng: &mut ChaCha8Rng,
) -> (Genome, Vec<CopyingError>) {
    let mutation = data.physiology().mutation;
    if mutation.chance == 0.0 {
        return (genome, Vec::new());
    }
    let (small, large) = (mutation.small, mutation.large);
    let mut errors = Vec::new();
    let genes = genome
        .genes
        .into_iter()
        .enumerate()
        .map(|(index, was)| {
            let gene = each_value(&was, data, |value, _| {
                if !chance(rng, mutation.chance) {
                    return value;
                }
                let factor = if chance(rng, large.share) {
                    // As likely to halve as to double.
                    libm::powf(large.most, 2.0 * unit(rng) - 1.0)
                } else {
                    1.0 - small + 2.0 * small * unit(rng)
                };
                value * factor
            });
            if gene != was {
                errors.push(CopyingError { gene: index, was });
            }
            gene
        })
        .collect();
    // When each gene works is structural (M2 design §9.2).
    let genome = Genome {
        genes,
        switches: genome.switches,
    };
    (genome, errors)
}

/// Whether each of `genome`'s genes can carry a flaw (M2 design §12.3):
/// one that can weaken, unless it's flagged, since a flagged gene does
/// nothing whatever its copies are.
fn can_be_flawed(genome: &Genome, data: &DataPack) -> Vec<bool> {
    // Whether a gene is flagged doesn't depend on the life stage.
    expressions(genome, data, LifeStage::Adult)
        .iter()
        .zip(&genome.genes)
        .map(|(expression, gene)| {
            frailty(gene) > 0.0 && !matches!(expression, Expression::Flagged(_))
        })
        .collect()
}

/// `flaws` with the new ones copying left (M2 design §12): each sound copy
/// of each gene of `genome` that can be flawed gets a flaw with
/// `flaws.arise`'s chance.
fn flawed(genome: &Genome, flaws: Vec<u8>, data: &DataPack, rng: &mut ChaCha8Rng) -> Vec<u8> {
    let arise = data.physiology().flaws.arise;
    if arise == 0.0 {
        return flaws;
    }
    let flaws = can_be_flawed(genome, data)
        .into_iter()
        .enumerate()
        .map(|(index, can)| {
            let flawed = flaws_of(&flaws, index);
            if !can {
                return flawed;
            }
            // One draw for each sound copy.
            let new = (flawed..2).filter(|_| chance(rng, arise)).count();
            flawed + new as u8
        })
        .collect();
    tidied(flaws)
}

/// A new sprite's genes from the starter genome, varied (design §4.9),
/// with the hidden flaws it carries, as wild animals do (M2 design §12.3).
pub(crate) fn wild(data: &DataPack, rng: &mut ChaCha8Rng) -> (Genome, Vec<u8>) {
    let genome = varied(data.starter(), data, rng);
    let flaws = founders(&genome, data, rng);
    (genome, flaws)
}

/// The flaws a sprite made from the starter genome carries (M2 design
/// §12), as wild animals do: each gene of `genome` that can be flawed has a
/// flaw in one copy with `flaws.founders`' chance. One sound copy is
/// enough, so they're hidden: only its young may suffer them.
fn founders(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Vec<u8> {
    let founders = data.physiology().flaws.founders;
    if founders == 0.0 {
        return Vec::new();
    }
    let flaws = can_be_flawed(genome, data)
        .into_iter()
        .map(|can| u8::from(can && chance(rng, founders)))
        .collect();
    tidied(flaws)
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use proptest::prelude::*;

    use super::*;
    use crate::ageing::LifeStage;
    use crate::brain_io::Source;
    use crate::expression::expressions;
    use crate::genome::Gene;
    use crate::inheritance::{Parent, of_pair};
    use crate::test_util::{builtin, builtin_changing, every_kind};

    /// The built-in pack's copying errors, flaws, and copy and change, as
    /// `physiology.ron` writes them.
    const MUTATION: &str =
        "    mutation: (chance: 0.01, small: 0.2, large: (share: 0.75, most: 2.0)),\n";
    const FLAWS: &str = "    flaws: (arise: 0.004, founders: 0.035, health: 0.5, wear: 0.05),\n";
    const COPY_AND_CHANGE: &str = "    copy_and_change: (doubled: 0.00029, lost: 0.00029, changed: 0.00029, on_copy: 0.75, longest: 2.0),\n";

    /// The built-in pack without copy and change, so copying keeps each gene
    /// where it is and changes only values.
    fn values_only() -> DataPack {
        builtin_changing("physiology.ron", &[(COPY_AND_CHANGE, "")])
    }

    /// The built-in pack with whole genes doubled, lost and changed with
    /// these chances, `on_copy` of changes on a copy, and neither copying
    /// errors nor flaws, so only whole genes change.
    fn whole_genes_only(doubled: f32, lost: f32, changed: f32, on_copy: f32) -> DataPack {
        let rules = format!(
            "    copy_and_change: (doubled: {doubled:?}, lost: {lost:?}, changed: {changed:?}, on_copy: {on_copy:?}, longest: 2.0),\n"
        );
        builtin_changing(
            "physiology.ron",
            &[
                (COPY_AND_CHANGE, rules.as_str()),
                (MUTATION, ""),
                (FLAWS, ""),
            ],
        )
    }

    /// `genome`, inherited without flaws, as copied into an egg.
    fn copy(genome: &Genome, data: &DataPack, rng: &mut ChaCha8Rng) -> Copied {
        let inherited = Inherited {
            genome: genome.clone(),
            flaws: Vec::new(),
        };
        copied(inherited, data, rng)
    }

    /// Each value of `gene`, in the order of its fields.
    fn values(gene: &Gene) -> Vec<f32> {
        match *gene {
            Gene::HalfLife { ticks, .. } => vec![ticks as f32],
            Gene::Reaction { rate, .. } => vec![rate],
            Gene::Emitter {
                threshold, gain, ..
            }
            | Gene::Receptor {
                threshold, gain, ..
            } => vec![threshold, gain],
            Gene::InitialConcentration { value, .. }
            | Gene::Trait { value, .. }
            | Gene::BrainParam { value, .. } => vec![value],
            Gene::Instinct { weight, .. } | Gene::AttentionInstinct { weight, .. } => vec![weight],
            Gene::Unknown { .. } | Gene::Unmatched(_) => vec![],
        }
    }

    #[test]
    fn about_one_gene_value_in_100_changes_by_at_most_half_or_double() {
        // M2 design §12: the built-in pack's chance is 0.01, and the
        // largest change halves or doubles a value.
        let data = values_only();
        let original = every_kind(&data);
        let (mut changed, mut all) = (0, 0);
        for seed in 0..4_000 {
            let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed)).genome;
            assert_eq!(copy.switches, original.switches);
            for (before, after) in original.genes.iter().zip(&copy.genes) {
                let (old, new) = (values(before), values(after));
                assert_eq!(old.len(), new.len());
                for (&old, &new) in old.iter().zip(&new) {
                    all += 1;
                    if old != new {
                        changed += 1;
                        let factor = new / old;
                        assert!((0.5..=2.0).contains(&factor), "{old} became {new}");
                    }
                }
            }
        }
        let share = changed as f32 / all as f32;
        assert!(
            (0.008..0.012).contains(&share),
            "{changed} of {all} changed"
        );
    }

    #[test]
    fn most_changes_are_large_and_about_one_in_four_small() {
        // M2 design §12: three changes in four are large, from half to
        // double, as likely one as the other, so most of those fall outside
        // a fifth either way; the rest are small, within a fifth.
        let data = values_only();
        let original = every_kind(&data);
        let (mut changes, mut outside, mut up) = (0, 0, 0);
        for seed in 0..20_000 {
            let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed)).genome;
            for (before, after) in original.genes.iter().zip(&copy.genes) {
                for (&old, &new) in values(before).iter().zip(&values(after)) {
                    if old == new {
                        continue;
                    }
                    changes += 1;
                    let factor = new / old;
                    outside += usize::from(!(0.8..=1.2).contains(&factor));
                    up += usize::from(factor > 1.2);
                }
            }
        }
        // A large change lands outside a fifth 71% of the time, so about
        // 53% of all changes do.
        let share = outside as f32 / changes as f32;
        assert!((0.49..0.57).contains(&share), "{outside} of {changes}");
        let up = up as f32 / outside as f32;
        assert!((0.42..0.62).contains(&up), "{up} of the large ones went up");
    }

    #[test]
    fn a_pack_without_copying_errors_or_flaws_copies_exactly_and_draws_nothing() {
        // A save from before copying errors keeps its pack's rules (M2
        // design §12): its eggs' genes are what they inherited.
        let data = builtin_changing(
            "physiology.ron",
            &[(MUTATION, ""), (FLAWS, ""), (COPY_AND_CHANGE, "")],
        );
        let inherited = Inherited {
            genome: every_kind(&data),
            flaws: vec![0, 1, 2, 1],
        };
        let mut rng = ChaCha8Rng::seed_from_u64(5);
        let before = rng.clone();
        let copy = copied(inherited.clone(), &data, &mut rng);
        assert_eq!(
            (copy.genome, copy.flaws),
            (inherited.genome, inherited.flaws)
        );
        assert!(copy.errors.is_empty());
        assert!(copy.changes.is_empty());
        assert_eq!(rng, before);
        assert!(founders(data.starter(), &data, &mut rng).is_empty());
        assert_eq!(rng, before);
    }

    #[test]
    fn an_egg_keeps_each_gene_copying_changed_as_it_inherited_it() {
        // M2 design §12: so the Genome tab can say what each was.
        let data = values_only();
        let original = every_kind(&data);
        let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(9));
        let changed: Vec<usize> = (0..original.genes.len())
            .filter(|&index| copy.genome.genes[index] != original.genes[index])
            .collect();
        let errors: Vec<usize> = copy.errors.iter().map(|error| error.gene).collect();
        assert_eq!(errors, changed);
        for error in &copy.errors {
            assert_eq!(error.was, original.genes[error.gene]);
        }
        let changes: usize = (0..2_000)
            .map(|seed| copy_errors(&original, &data, seed))
            .sum();
        assert!(changes > 0, "copying never changed a gene");
    }

    /// How many genes copying changed in an egg of `genome`, with `seed`.
    fn copy_errors(genome: &Genome, data: &DataPack, seed: u64) -> usize {
        copy(genome, data, &mut ChaCha8Rng::seed_from_u64(seed))
            .errors
            .len()
    }

    #[test]
    fn about_one_egg_in_two_gets_a_new_flaw_only_in_genes_that_can_weaken() {
        // M2 design §12: each sound copy of a gene that can weaken gets a
        // flaw with the chance `flaws.arise` (0.004), so about one egg of
        // the starter's in two has a new one.
        let data = values_only();
        let starter = data.starter();
        let mut flawed_eggs = 0;
        for seed in 0..2_000 {
            let flaws = copy(starter, &data, &mut ChaCha8Rng::seed_from_u64(seed)).flaws;
            flawed_eggs += usize::from(!flaws.is_empty());
            for (index, &flawed) in flaws.iter().enumerate() {
                assert!(flawed <= 2);
                if flawed > 0 {
                    assert!(frailty(&starter.genes[index]) > 0.0, "gene {index}");
                }
            }
        }
        assert!(
            (800..1_200).contains(&flawed_eggs),
            "{flawed_eggs} of 2,000"
        );
    }

    #[test]
    fn a_founder_carries_about_three_flaws_all_hidden() {
        // M2 design §12: each gene of the starter's that can weaken has a
        // flaw in one copy with the chance `flaws.founders` (0.035).
        let data = builtin();
        let starter = data.starter();
        let can_weaken = starter
            .genes
            .iter()
            .filter(|gene| frailty(gene) > 0.0)
            .count();
        let mut carried = 0;
        for seed in 0..1_000 {
            let flaws = founders(starter, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            for (index, &flawed) in flaws.iter().enumerate() {
                assert!(flawed <= 1, "a founder's flaw is hidden");
                if flawed > 0 {
                    assert!(frailty(&starter.genes[index]) > 0.0, "gene {index}");
                }
                carried += usize::from(flawed);
            }
        }
        let each = carried as f32 / 1_000.0;
        let expected = 0.035 * can_weaken as f32;
        assert!(
            (2.0..4.0).contains(&expected),
            "{can_weaken} genes can weaken"
        );
        assert!((each - expected).abs() < 0.3, "{each} each, not {expected}");
    }

    /// A genome with a gene flagged for each way a gene may not change the
    /// body (M1 design §4.3), among genes that work.
    const FLAGGED: &str = r#"(format: 2, genes: [
        HalfLife(chem: "energy", ticks: 300),
        Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food"),
        Reaction(reactants: [("food", 1)], products: [("energy", 1)], rate: 0.5),
        InitialConcentration(chem: "hydration", value: 0.5),
        Trait(trait: "speed", value: 7.0, from: Adult),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "hunger"),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0, from: Adult),
    ])"#;

    /// A random gene of any type, as a genome file writes it, with random
    /// values and a random switch: some change the body directly, or switch
    /// when they can't, so they're flagged.
    fn random_gene() -> impl Strategy<Value = String> {
        let chem = || {
            prop::sample::select(vec![
                "hunger",
                "pain",
                "boredom",
                "reward",
                "energy",
                "food",
                "hydration",
            ])
        };
        let input = || prop::sample::select(vec!["always", "hunger", "thirst", "pain", "age"]);
        let gene = prop_oneof![
            (chem(), 1u32..5_000).prop_map(|(c, t)| format!(r#"HalfLife(chem: "{c}", ticks: {t}"#)),
            (chem(), chem(), 0.0f32..1.0).prop_map(|(a, b, rate)| format!(
                r#"Reaction(reactants: [("{a}", 1)], products: [("{b}", 1)], rate: {rate:?}"#
            )),
            (chem(), 0.0f32..1.0, -1.0f32..1.0, chem()).prop_map(|(from, threshold, gain, to)| {
                format!(
                    r#"Emitter(locus: Chem("{from}"), mode: Level, threshold: {threshold:?}, gain: {gain:?}, chem: "{to}""#
                )
            }),
            (chem(), 0.0f32..1.0, -1.0f32..1.0).prop_map(|(c, threshold, gain)| format!(
                r#"Receptor(chem: "{c}", threshold: {threshold:?}, gain: {gain:?}, target: "learning_rate_mod""#
            )),
            (chem(), 0.0f32..1.0)
                .prop_map(|(c, value)| format!(r#"InitialConcentration(chem: "{c}", value: {value:?}"#)),
            (2.0f32..12.0).prop_map(|value| format!(r#"Trait(trait: "speed", value: {value:?}"#)),
            (6_000.0f32..60_000.0)
                .prop_map(|value| format!(r#"Trait(trait: "grows_old", value: {value:?}"#)),
            (0.0f32..1.0)
                .prop_map(|value| format!(r#"BrainParam(param: "tau_base", value: {value:?}"#)),
            (input(), -1.0f32..1.0).prop_map(|(i, weight)| format!(
                r#"Instinct(inputs: [("{i}", false)], verb: Eat, weight: {weight:?}"#
            )),
            (input(), -1.0f32..1.0).prop_map(|(i, weight)| format!(
                r#"AttentionInstinct(input: "{i}", category: "water", weight: {weight:?}"#
            )),
        ];
        let switch = prop::sample::select(vec![
            "",
            ", from: Adult",
            ", until: Adult",
            ", from: Old, until: Adult",
        ]);
        (gene, switch).prop_map(|(gene, switch)| format!("{gene}{switch})"))
    }

    /// The built-in pack, with every gene value copied wrongly and every
    /// copy that can be flawed flawed, and whole genes changed as `rules`
    /// says.
    fn every_copy_wrong(rules: &str) -> DataPack {
        builtin_changing(
            "physiology.ron",
            &[
                ("mutation: (chance: 0.01,", "mutation: (chance: 1.0,"),
                (
                    "flaws: (arise: 0.004, founders: 0.035,",
                    "flaws: (arise: 1.0, founders: 1.0,",
                ),
                (COPY_AND_CHANGE, rules),
            ],
        )
    }

    /// Whether `gene`, working as `switch`, is flagged (M1 design §4.3).
    fn flagged(gene: &Gene, switch: Switch, data: &DataPack) -> bool {
        let alone = Genome::switched(vec![gene.clone()], vec![switch]);
        matches!(
            expressions(&alone, data, LifeStage::Adult)[0],
            Expression::Flagged(_)
        )
    }

    /// Checks `copy`, what copying made of `inherited` (M2 design §13):
    /// each change is one the pack can have, none flags a gene that
    /// worked, a flagged gene carries no flaws, and it never grows past
    /// `longest` unless what it inherited already had.
    fn check_copy(
        inherited: &Inherited,
        copy: &Copied,
        data: &DataPack,
    ) -> Result<(), TestCaseError> {
        prop_assert!(copy.genome.fits(data));
        prop_assert_eq!(check_changes(&copy.genome, &copy.changes, data), Ok(()));
        let most = inherited.genome.genes.len().max(longest(data));
        prop_assert!(copy.genome.genes.len() <= most);
        for change in &copy.changes {
            if let CopyChange::Changed {
                gene,
                ref was,
                switch,
                ..
            } = *change
            {
                let now = (&copy.genome.genes[gene], copy.genome.switch(gene));
                prop_assert!(
                    flagged(was, switch, data) || !flagged(now.0, now.1, data),
                    "{:?} became {:?}",
                    was,
                    now
                );
            }
        }
        let expressed = expressions(&copy.genome, data, LifeStage::Adult);
        for (index, how) in expressed.iter().enumerate() {
            if matches!(how, Expression::Flagged(_)) {
                prop_assert_eq!(flaws_of(&copy.flaws, index), 0);
            }
        }
        Ok(())
    }

    /// The built-in pack with every gene value copied wrongly, every copy
    /// that can be flawed flawed, and whole genes often doubled, lost and
    /// changed, in a genome no longer than the starter's.
    fn every_change() -> DataPack {
        every_copy_wrong(
            "    copy_and_change: (doubled: 0.3, lost: 0.3, changed: 0.4, on_copy: 0.5, longest: 1.0),\n",
        )
    }

    /// A genome of `genes`, as an egg inherits it from a founder with one
    /// copy of each gene that can be flawed flawed.
    fn founded(genes: &[String], data: &DataPack, rng: &mut ChaCha8Rng) -> Inherited {
        let file = format!("(format: 2, genes: [{}])", genes.join(",\n"));
        let genome = Genome::from_ron(&file, data).expect("a valid genome");
        let flaws = founders(&genome, data, rng);
        Inherited { genome, flaws }
    }

    proptest! {
        #[test]
        fn copying_errors_never_change_how_a_gene_is_expressed(seed: u64) {
            // M2 design §12: a flagged gene stays flagged, and no copying
            // error makes a gene change the body directly, at any stage.
            let data = values_only();
            let flagged = Genome::from_ron(FLAGGED, &data).expect("a valid genome");
            for genome in [data.starter().clone(), every_kind(&data), flagged] {
                let copy = copy(&genome, &data, &mut ChaCha8Rng::seed_from_u64(seed)).genome;
                for stage in [LifeStage::Young, LifeStage::Adult, LifeStage::Old] {
                    prop_assert_eq!(
                        expressions(&copy, &data, stage),
                        expressions(&genome, &data, stage)
                    );
                }
            }
        }

        #[test]
        fn whatever_the_genome_copying_keeps_how_each_gene_works_and_flaws_no_flagged_one(
            genes in prop::collection::vec(random_gene(), 1..12),
            seed: u64,
        ) {
            // M2 design §12.1, §12.3, with every value changed and every
            // copy flawed: a flagged gene stays flagged and does nothing,
            // whatever its copies, and no gene comes to change the body.
            let data = every_copy_wrong("");
            let file = format!("(format: 2, genes: [{}])", genes.join(",\n"));
            let genome = Genome::from_ron(&file, &data).expect("a valid genome");
            let rng = &mut ChaCha8Rng::seed_from_u64(seed);
            let copy = copy(&genome, &data, rng);
            for stage in [LifeStage::Young, LifeStage::Adult, LifeStage::Old] {
                prop_assert_eq!(
                    expressions(&copy.genome, &data, stage),
                    expressions(&genome, &data, stage)
                );
            }
            let founders = founders(&genome, &data, rng);
            let expressed = expressions(&genome, &data, LifeStage::Adult);
            for (index, gene) in genome.genes.iter().enumerate() {
                let flagged = matches!(expressed[index], Expression::Flagged(_));
                let can = frailty(gene) > 0.0 && !flagged;
                prop_assert_eq!(flaws_of(&copy.flaws, index), if can { 2 } else { 0 });
                prop_assert_eq!(flaws_of(&founders, index), u8::from(can));
            }
        }

        #[test]
        fn whatever_the_genome_no_whole_gene_change_makes_a_working_gene_change_the_body(
            genes in prop::collection::vec(random_gene(), 1..12),
            seed: u64,
        ) {
            // M2 design §13, with whole genes often doubled, lost and
            // changed, every value changed and every copy flawed: each
            // change is one the pack can have, none flags a gene that
            // worked, a flagged gene carries no flaws, and the genome
            // never grows past `longest`.
            let data = every_change();
            let rng = &mut ChaCha8Rng::seed_from_u64(seed);
            let mut inherited = founded(&genes, &data, rng);
            // Over generations, as changes on copies grow it to the limit.
            for _ in 0..30 {
                let copy = copied(inherited.clone(), &data, rng);
                check_copy(&inherited, &copy, &data)?;
                inherited = Inherited {
                    genome: copy.genome,
                    flaws: copy.flaws,
                };
            }
        }

        #[test]
        fn whatever_the_genomes_no_whole_gene_change_makes_a_working_gene_in_a_pair_s_child_change_the_body(
            hers in prop::collection::vec(random_gene(), 1..12),
            his in prop::collection::vec(random_gene(), 1..12),
            seed: u64,
        ) {
            // M2 design §13.3: the same for a pair's child, whose parents'
            // genes line up and mix (§12.2) before they're copied, and
            // which never grows past `longest` either, generation after
            // generation.
            let data = every_change();
            let rng = &mut ChaCha8Rng::seed_from_u64(seed);
            let (mut mother, mut father) = (founded(&hers, &data, rng), founded(&his, &data, rng));
            fn parent(inherited: &Inherited) -> Parent<'_> {
                Parent {
                    genome: &inherited.genome,
                    flaws: &inherited.flaws,
                }
            }
            for _ in 0..30 {
                let child = of_pair(parent(&mother), parent(&father), &data, rng);
                prop_assert!(child.genome.genes.len() <= longest(&data));
                let copy = copied(child.clone(), &data, rng);
                check_copy(&child, &copy, &data)?;
                prop_assert!(copy.flaws.len() <= copy.genome.genes.len());
                prop_assert!(copy.flaws.iter().all(|&flawed| flawed <= 2));
                mother = father;
                father = Inherited {
                    genome: copy.genome,
                    flaws: copy.flaws,
                };
            }
        }
    }

    #[test]
    fn about_one_hatchling_of_the_starter_s_in_ten_has_a_whole_gene_doubled_lost_or_changed() {
        // M2 design §13: each with the chance 0.00029 a gene, so about one
        // hatchling in ten, each kind as likely as the others, but that a
        // trait has nothing to change.
        let data = builtin();
        let starter = data.starter();
        let (mut eggs, mut kinds) = (0, [0; 3]);
        for seed in 0..10_000 {
            let copy = copy(starter, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            assert_eq!(check_changes(&copy.genome, &copy.changes, &data), Ok(()));
            eggs += usize::from(!copy.changes.is_empty());
            for change in &copy.changes {
                kinds[match change {
                    CopyChange::Doubled { .. } => 0,
                    CopyChange::Lost { .. } => 1,
                    CopyChange::Changed { .. } => 2,
                }] += 1;
            }
        }
        // About 975 expected.
        assert!((880..1_070).contains(&eggs), "{eggs} of 10,000");
        let all: usize = kinds.iter().sum();
        for (kind, n) in ["doubled", "lost", "changed"].iter().zip(kinds) {
            let share = n as f32 / all as f32;
            assert!((0.27..0.39).contains(&share), "{n} of {all} {kind}");
        }
    }

    /// Every gene of `every_kind` but the unknown one, the instinct working
    /// from adulthood on.
    fn readable(data: &DataPack) -> Genome {
        let mut genes = every_kind(data).genes;
        genes.pop();
        let mut switches = vec![Switch::default(); genes.len()];
        switches[7].from = LifeStage::Adult;
        Genome::switched(genes, switches)
    }

    #[test]
    fn a_gene_this_build_cant_read_is_kept_exactly_as_it_is() {
        // M1 design §2.8, v19 §5.7: an unknown gene, or one naming a
        // category the world lacks, is never doubled, lost or changed.
        let genome = |data: &DataPack| {
            let text = r#"(format: 1, genes: [
                Gene(type: 900, version: 1, payload: "c0ffee"),
                AttentionInstinct(input: "hunger", category: "tree", weight: 1.0),
                Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
                Instinct(inputs: [("thirst", false)], verb: Drink, weight: 1.0),
            ])"#;
            Genome::from_ron(text, data).expect("a valid genome")
        };
        // Each of the instincts changed, but one lost: a genome keeps the
        // last gene it can read.
        for (rules, changes) in [
            ((1.0, 0.0, 0.0), 2),
            ((0.0, 1.0, 0.0), 1),
            ((0.0, 0.0, 1.0), 2),
        ] {
            let data = whole_genes_only(rules.0, rules.1, rules.2, 1.0);
            let original = genome(&data);
            let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(1));
            assert_eq!(copy.genome.genes[..2], original.genes[..2], "{rules:?}");
            assert_eq!(copy.changes.len(), changes, "only the instincts changed");
        }
    }

    #[test]
    fn a_gene_this_build_cant_read_spends_no_roll_of_the_dice() {
        // M2 design §13.1: so the genes it can read are copied as they would
        // be without it, and the world's dice roll on as they would have.
        let data = whole_genes_only(0.1, 0.1, 0.1, 0.75);
        let unreadable = Genome::from_ron(
            r#"(format: 1, genes: [
                Gene(type: 900, version: 1, payload: "c0ffee"),
                AttentionInstinct(input: "hunger", category: "tree", weight: 1.0),
                Gene(type: 901, version: 1, payload: "decade"),
            ])"#,
            &data,
        )
        .expect("a valid genome")
        .genes;
        let starter = data.starter();
        let (mut genes, mut switches) = (Vec::new(), Vec::new());
        for (index, gene) in starter.genes.iter().enumerate() {
            let before = match index {
                0 => Some(0),
                50 => Some(1),
                _ => None,
            };
            if let Some(at) = before {
                genes.push(unreadable[at].clone());
                switches.push(Switch::default());
            }
            genes.push(gene.clone());
            switches.push(starter.switch(index));
        }
        genes.push(unreadable[2].clone());
        switches.push(Switch::default());
        let with_them = Genome::switched(genes, switches);
        let readable = |gene: &&Gene| !matches!(gene, Gene::Unknown { .. } | Gene::Unmatched(_));
        assert_eq!(
            with_them.genes.iter().filter(readable).count(),
            starter.genes.len()
        );
        let mut changed = 0;
        for seed in 0..20 {
            let (mut dice, mut same_dice) = (
                ChaCha8Rng::seed_from_u64(seed),
                ChaCha8Rng::seed_from_u64(seed),
            );
            let without = copy(starter, &data, &mut dice).genome;
            let with = copy(&with_them, &data, &mut same_dice).genome;
            let read: Vec<&Gene> = with.genes.iter().filter(readable).collect();
            assert!(
                read.iter().copied().eq(without.genes.iter()),
                "seed {seed}: the genes it can read were copied differently"
            );
            assert_eq!(
                unit(&mut dice),
                unit(&mut same_dice),
                "seed {seed}: the next roll"
            );
            changed += usize::from(without != *starter);
        }
        assert!(changed > 0, "nothing was copied and changed");
    }

    #[test]
    fn a_doubled_gene_sits_next_to_its_original_with_its_switch_and_flaws() {
        // M2 design §13: so the two tend to be inherited together.
        let data = whole_genes_only(1.0, 0.0, 0.0, 0.0);
        let original = readable(&data);
        let flaws = vec![0, 1, 2, 0, 0, 0, 0, 1];
        let inherited = Inherited {
            genome: original.clone(),
            flaws: flaws.clone(),
        };
        let copy = copied(inherited, &data, &mut ChaCha8Rng::seed_from_u64(1));
        assert_eq!(copy.genome.genes.len(), 2 * original.genes.len());
        for (index, gene) in original.genes.iter().enumerate() {
            for at in [2 * index, 2 * index + 1] {
                assert_eq!(copy.genome.genes[at], *gene, "gene {index}");
                assert_eq!(copy.genome.switch(at), original.switch(index));
                assert_eq!(flaws_of(&copy.flaws, at), flaws_of(&flaws, index));
            }
        }
        let doubled: Vec<CopyChange> = (0..original.genes.len())
            .map(|index| CopyChange::Doubled {
                gene: 2 * index + 1,
            })
            .collect();
        assert_eq!(copy.changes, doubled);
        assert_eq!(check_changes(&copy.genome, &copy.changes, &data), Ok(()));
    }

    #[test]
    fn a_genome_never_grows_past_twice_the_starter_s_length() {
        // M2 design §13: `longest` (2) times as many genes as the starter
        // has.
        let data = whole_genes_only(1.0, 0.0, 0.0, 0.0);
        let longest = 2 * data.starter().genes.len();
        assert_eq!(super::longest(&data), longest);
        let rng = &mut ChaCha8Rng::seed_from_u64(1);
        let once = copy(data.starter(), &data, rng).genome;
        assert_eq!(once.genes.len(), longest);
        let twice = copy(&once, &data, rng);
        assert_eq!(twice.genome, once, "no room to double another");
        assert!(twice.changes.is_empty());
        // One already longer, as a genome file's may be, doesn't grow.
        let longer = Genome::switched(
            [once.genes.clone(), once.genes.clone()].concat(),
            Vec::new(),
        );
        assert_eq!(copy(&longer, &data, rng).genome, longer);
        // A genome copied often, with genes doubled and changed on copies
        // as much as lost, stays within it too.
        let data = whole_genes_only(0.3, 0.3, 0.3, 1.0);
        let mut genome = data.starter().clone();
        for _ in 0..50 {
            genome = copy(&genome, &data, rng).genome;
            assert!(genome.genes.len() <= longest);
        }
    }

    #[test]
    fn a_lost_gene_is_remembered_but_the_last_is_never_lost() {
        // M2 design §13: any gene can be lost, but a genome keeps one.
        let data = whole_genes_only(0.0, 1.0, 0.0, 0.0);
        let original = readable(&data);
        let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(1));
        let last = original.genes.len() - 1;
        assert_eq!(copy.genome.genes, vec![original.genes[last].clone()]);
        let lost: Vec<CopyChange> = (0..last)
            .map(|index| CopyChange::Lost {
                was: original.genes[index].clone(),
                switch: original.switch(index),
            })
            .collect();
        assert_eq!(copy.changes, lost);
        assert_eq!(check_changes(&copy.genome, &copy.changes, &data), Ok(()));
        // The last it can read, though a gene it can't comes after.
        let text = r#"(format: 1, genes: [
            Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
            Gene(type: 900, version: 1, payload: "c0ffee"),
        ])"#;
        let original = Genome::from_ron(text, &data).expect("a valid genome");
        let kept = super::tests::copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(1));
        assert_eq!(kept.genome, original);
    }

    /// What differs between a gene and its switch as they were and as they
    /// are: one of an instinct's inputs, its verb, an attention instinct's
    /// input or category, the switch, or anything else.
    fn differences(was: (&Gene, Switch), now: (&Gene, Switch)) -> Vec<&'static str> {
        let mut differ = Vec::new();
        if was.1 != now.1 {
            differ.push("switch");
        }
        match (was.0, now.0) {
            (
                Gene::Instinct {
                    inputs: before,
                    verb: did,
                    weight: was,
                },
                Gene::Instinct {
                    inputs: after,
                    verb: does,
                    weight: is,
                },
            ) => {
                if before.len() != after.len() || was != is {
                    differ.push("other");
                }
                for (&(old, negated), &(new, negates)) in before.iter().zip(after) {
                    if negated != negates {
                        differ.push("other");
                    }
                    if old != new {
                        differ.push("input");
                    }
                }
                if did != does {
                    differ.push("verb");
                }
            }
            (
                Gene::AttentionInstinct {
                    input: old,
                    category: was_on,
                    whom: was_about,
                    weight: was,
                },
                Gene::AttentionInstinct {
                    input: new,
                    category: is_on,
                    whom: is_about,
                    weight: is,
                },
            ) => {
                if was != is {
                    differ.push("other");
                }
                if old != new {
                    differ.push("attention input");
                }
                if was_on != is_on {
                    differ.push("category");
                }
                if was_about != is_about {
                    differ.push("whom");
                }
            }
            (was, now) => {
                if was != now {
                    differ.push("other");
                }
            }
        }
        differ
    }

    #[test]
    fn a_changed_gene_changes_one_input_action_category_whom_or_when_it_works() {
        // M2 design §13: what an instinct reacts to or its action, what
        // draws the eye to what and whom (§15.6), or the life stages a gene
        // works in, each a
        // choice the pack has; a trait and a starting level have none to
        // change.
        let data = whole_genes_only(0.0, 0.0, 1.0, 0.0);
        let original = Genome::from_ron(
            r#"(format: 2, genes: [
                Instinct(inputs: [("hunger", false), ("pain", true)], verb: Eat, weight: 1.0),
                AttentionInstinct(input: "thirst", category: "water", weight: 0.8),
                Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "hunger"),
                Instinct(inputs: [("always", false)], verb: Wander, weight: 0.2, from: Adult),
                Trait(trait: "speed", value: 7.0),
                InitialConcentration(chem: "boredom", value: 0.2),
                Gene(type: 900, version: 1, payload: "c0ffee"),
            ])"#,
            &data,
        )
        .expect("a valid genome");
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..500 {
            let copy = copy(&original, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            assert_eq!(copy.genome.genes.len(), original.genes.len());
            assert_eq!(copy.changes.len(), 4, "all but the last three");
            assert_eq!(check_changes(&copy.genome, &copy.changes, &data), Ok(()));
            for index in 4..7 {
                assert_eq!(copy.genome.genes[index], original.genes[index]);
            }
            for change in &copy.changes {
                let CopyChange::Changed {
                    gene,
                    ref was,
                    switch,
                    on_copy,
                } = *change
                else {
                    panic!("only changes: {change:?}");
                };
                assert!(!on_copy);
                assert_eq!(
                    (was, switch),
                    (&original.genes[gene], original.switch(gene))
                );
                let now = (&copy.genome.genes[gene], copy.genome.switch(gene));
                let differ = differences((was, switch), now);
                assert_eq!(differ.len(), 1, "{was:?} became {now:?}");
                assert!(now.1.ever_on(), "{now:?} never works");
                if let Gene::Instinct { verb, .. } = now.0 {
                    assert!(!verb.is_reserved());
                }
                if let Gene::AttentionInstinct { input, .. } = *now.0 {
                    let input = data.brain_input(input).expect("in the pack");
                    assert!(matches!(input.source, Source::State(_)), "{}", input.name);
                }
                seen.insert((gene, differ[0]));
            }
        }
        let every: std::collections::BTreeSet<_> = [
            (0, "input"),
            (0, "verb"),
            (0, "switch"),
            (1, "attention input"),
            (1, "category"),
            (1, "whom"),
            (1, "switch"),
            (2, "switch"),
            (3, "input"),
            (3, "verb"),
            (3, "switch"),
        ]
        .into_iter()
        .collect();
        assert_eq!(seen, every);
    }

    #[test]
    fn three_changes_in_four_are_made_on_a_fresh_copy_so_the_original_still_works() {
        // M2 design §13: `on_copy` (0.75) of changes, while there's room.
        let data = whole_genes_only(0.0, 0.0, 1.0, 0.75);
        let starter = data.starter();
        let (mut on_copies, mut changes) = (0, 0);
        for seed in 0..100 {
            let copy = copy(starter, &data, &mut ChaCha8Rng::seed_from_u64(seed));
            assert_eq!(check_changes(&copy.genome, &copy.changes, &data), Ok(()));
            for change in &copy.changes {
                let CopyChange::Changed {
                    gene,
                    ref was,
                    switch,
                    on_copy,
                } = *change
                else {
                    panic!("only changes: {change:?}");
                };
                changes += 1;
                if on_copy {
                    on_copies += 1;
                    assert_eq!(copy.genome.genes[gene - 1], *was);
                    assert_eq!(copy.genome.switch(gene - 1), switch);
                }
            }
        }
        let share = on_copies as f32 / changes as f32;
        assert!((0.72..0.78).contains(&share), "{on_copies} of {changes}");
    }
}
