//! The Genome tab (design §6.1): what's new in a sprite with parents (M2
//! design §12), then its genes, grouped under headings, as plain lines.

use ratatui::style::Style;
use ratatui::text::Line;
use terra_sim::{
    CopyChangeView, EmitterMode, Expression, GeneView, LifeStage, SpriteView, Switch, Trait, Whom,
};

use super::actions::verb_name;
use super::body::{trait_name, trait_value};
use super::brain::concept_name;
use super::{TabLine, listed, wrapped};
use crate::app::App;
use crate::listed::listed_enum;
use crate::spots::Spot;
use crate::text::{BOUND, display_name, level, signed, significant, thousands, unbroken};

listed_enum! {
    /// A group of genes on the Genome tab.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum GeneGroup {
        Traits => "TRAITS",
        HalfLives => "HALF-LIVES",
        Reactions => "REACTIONS",
        Emitters => "EMITTERS",
        Receptors => "RECEPTORS",
        StartingLevels => "STARTING LEVELS",
        BrainSettings => "BRAIN SETTINGS",
        Instincts => "INSTINCTS",
        Attention => "ATTENTION INSTINCTS",
        Unknown => "UNKNOWN GENES",
    }

    /// Every group, in the tab's order. Traits come first, since they share
    /// one line.
    const ALL;

    /// The group's heading.
    fn heading(self) -> &'static str;
}

impl GeneGroup {
    /// The group `gene` goes in.
    fn of(gene: &GeneView) -> GeneGroup {
        match gene {
            GeneView::Trait { .. } => GeneGroup::Traits,
            GeneView::HalfLife { .. } => GeneGroup::HalfLives,
            GeneView::Reaction { .. } => GeneGroup::Reactions,
            GeneView::Emitter { .. } => GeneGroup::Emitters,
            GeneView::Receptor { .. } => GeneGroup::Receptors,
            GeneView::InitialConcentration { .. } => GeneGroup::StartingLevels,
            GeneView::BrainParam { .. } => GeneGroup::BrainSettings,
            GeneView::Instinct { .. } => GeneGroup::Instincts,
            GeneView::AttentionInstinct { .. } => GeneGroup::Attention,
            GeneView::Unknown { .. } => GeneGroup::Unknown,
        }
    }
}

/// The Genome tab (design §6.1): what's new in a sprite with parents (M2
/// design §12), then the genes grouped under headings, each group in genome
/// order, as plain lines. The expressed traits share one line. A gene with
/// no effect is dimmed, with the reason below it. The pointer on a gene's
/// lines, or a trait's part of the line they share, says what it does
/// (#304).
pub(super) fn genome_tab(sprite: &SpriteView, app: &App) -> Vec<TabLine> {
    let genes: Vec<(usize, GeneView, Expression, Switch, f32)> = sprite
        .genes()
        .into_iter()
        .zip(sprite.switches())
        .zip(sprite.gene_health().iter().copied())
        .enumerate()
        .map(|(index, (((gene, how), switch), health))| (index, gene, how, switch, health))
        .collect();
    let look = app.theme.text();
    let plain = Style::default();
    let mut lines: Vec<TabLine> = whats_new(sprite, app)
        .into_iter()
        .map(TabLine::from)
        .collect();
    for group in GeneGroup::ALL {
        let members: Vec<&(usize, GeneView, Expression, Switch, f32)> = genes
            .iter()
            .filter(|(_, gene, ..)| GeneGroup::of(gene) == group)
            .collect();
        if members.is_empty() {
            continue;
        }
        lines.push(Line::styled(format!(" {}", group.heading()), look.heading).into());
        let (together, apart): (Vec<_>, Vec<_>) =
            members.into_iter().partition(|(_, gene, how, ..)| {
                matches!(gene, GeneView::Trait { .. }) && *how == Expression::Expressed
            });
        if !together.is_empty() {
            let traits: Vec<(usize, String)> = together
                .iter()
                .map(|(index, gene, _, _, health)| {
                    let text = format!("{}{}", gene_text(gene), health_text(*health));
                    (*index, text)
                })
                .collect();
            let texts: Vec<String> = traits.iter().map(|(_, text)| text.clone()).collect();
            lines.extend(traits_spotted(
                wrapped(&listed(&texts), 1, plain, look.muted),
                &traits,
            ));
        }
        for &(index, ref gene, ref how, switch, health) in apart {
            let text = format!(
                "{}{}{}",
                gene_text(gene),
                switch_text(switch),
                health_text(health)
            );
            let spotted = |lines: Vec<Line<'static>>| {
                lines
                    .into_iter()
                    .map(move |line| TabLine::spotted(line, Spot::Gene(index)))
            };
            let reason = match how {
                Expression::Expressed => {
                    lines.extend(spotted(wrapped(&text, 1, plain, look.muted)));
                    continue;
                }
                Expression::Flagged(reason) => format!("flagged: {reason}"),
                Expression::Unexpressed => "unexpressed: an earlier gene sets this".into(),
                Expression::Unknown => "unknown: this version can't read it".into(),
                Expression::Unmatched => {
                    "unmatched: names a category this world doesn't have".into()
                }
                Expression::Dormant => "dormant: switched off at this life stage".into(),
            };
            // A gene with no effect is minor, and says why (design §6.2).
            lines.extend(spotted(wrapped(&text, 1, look.muted, look.muted)));
            lines.extend(spotted(wrapped(&reason, 3, look.muted, look.muted)));
        }
    }
    lines
}

/// The traits' shared `lines`, each trait's spot over its part of them:
/// `traits` are each gene's place in the genome and its text, in the order
/// the lines list them.
fn traits_spotted(lines: Vec<Line<'static>>, traits: &[(usize, String)]) -> Vec<TabLine> {
    let mut traits = traits.iter().peekable();
    lines
        .into_iter()
        .map(|line| {
            let text = line.to_string();
            let mut row = TabLine::from(line);
            let mut from = 0;
            while let Some((index, trait_text)) = traits.peek() {
                // As `wrapped` draws it, its spaces kept together as plain ones.
                let trait_text = trait_text.replace(BOUND, " ");
                let Some(at) = text[from..].find(&trait_text) else {
                    break;
                };
                let start = text[..from + at].chars().count() as u16;
                let width = trait_text.chars().count() as u16;
                row.spots.push((start..start + width, Spot::Gene(*index)));
                from += at + trait_text.len();
                traits.next();
            }
            row
        })
        .collect()
}

/// What's new in a sprite with parents (M2 design §12, §13), at the top of
/// its Genome tab: who it's from; each gene copying changed in what it
/// does, with what it was, or what it's a copy of; the genes copying
/// doubled and lost; each gene copying changed a number of, with what it
/// was; its broken genes and the flaws it carries hidden; then a blank
/// line. Nothing for a sprite with no parents.
fn whats_new(sprite: &SpriteView, app: &App) -> Vec<Line<'static>> {
    let parents: Vec<String> = sprite
        .parents()
        .iter()
        .map(|&id| unbroken(&app.names().label(id).to_uppercase()))
        .collect();
    let from = match parents.as_slice() {
        [] => return Vec::new(),
        [mother] => format!("FROM {mother} ALONE"),
        parents => format!("FROM {}", parents.join(" AND ")),
    };
    let look = app.theme.text();
    let plain = Style::default();
    let mut lines = wrapped(&from, 1, look.heading, look.muted);
    let genes = sprite.genes();
    let switches = sprite.switches();
    let (mut doubled, mut lost) = (Vec::new(), Vec::new());
    for change in sprite.copy_changes() {
        match change {
            CopyChangeView::Changed {
                gene,
                was,
                switch,
                on_copy,
            } => {
                // A copy's original still works; a gene changed in its
                // place doesn't.
                let how = if on_copy { "from" } else { "was" };
                let now = named(&genes[gene].0, switches[gene]);
                let text = format!("new: {now}, {how} {}", named(&was, switch));
                lines.extend(wrapped(&text, 1, plain, look.muted));
            }
            CopyChangeView::Doubled { gene } => {
                doubled.push(named(&genes[gene].0, switches[gene]));
            }
            CopyChangeView::Lost { was, switch } => lost.push(named(&was, switch)),
        }
    }
    for (label, names) in [("doubled", doubled), ("lost", lost)] {
        if !names.is_empty() {
            let text = format!("{label}: {}", listed(&names));
            lines.extend(wrapped(&text, 1, plain, look.muted));
        }
    }
    for (index, was) in sprite.copying_errors() {
        let now = &genes[index].0;
        let text = format!("{}, was {}", gene_text(now), was_text(now, &was));
        lines.extend(wrapped(&text, 1, plain, look.muted));
    }
    let flaws = sprite.flaws();
    for (flawed, label) in [(2, "broken"), (1, "carries flaws")] {
        let names: Vec<String> = genes
            .iter()
            .zip(&flaws)
            .filter(|&(_, &its)| its == flawed)
            .map(|((gene, _), _)| gene_name(gene))
            .collect();
        if !names.is_empty() {
            let text = format!("{label}: {}", listed(&names));
            lines.extend(wrapped(&text, 1, plain, look.muted));
        }
    }
    lines.push(Line::from(""));
    lines
}

/// The genes copying changed in what they do in `sprite` (M2 design §13),
/// as its hatching line in the log names them: `thirst → attends to
/// sprite`. None if it has none.
pub(crate) fn something_new(sprite: &SpriteView) -> Option<String> {
    let (genes, switches) = (sprite.genes(), sprite.switches());
    let new: Vec<String> = sprite
        .copy_changes()
        .into_iter()
        .filter_map(|change| match change {
            CopyChangeView::Changed { gene, .. } => Some(named(&genes[gene].0, switches[gene])),
            _ => None,
        })
        .collect();
    (!new.is_empty()).then(|| new.join(", "))
}

/// A gene's name and when it works, as the Genome tab's list at the top and
/// the log name a gene copying changed: `hunger → eat from Adult`.
fn named(gene: &GeneView, switch: Switch) -> String {
    format!("{}{}", gene_name(gene), switch_text(switch))
}

/// The numbers `was` had that `now` doesn't, as the Genome tab writes
/// them: `+.00428`, `past .5`. All of them, if none reads differently.
fn was_text(now: &GeneView, was: &GeneView) -> String {
    let (now, was) = (numbers(now), numbers(was));
    let differ: Vec<&str> = was
        .iter()
        .zip(&now)
        .filter(|(was, now)| was != now)
        .map(|(was, _)| was.as_str())
        .collect();
    let shown = if differ.is_empty() {
        was.iter().map(String::as_str).collect()
    } else {
        differ
    };
    let shown: Vec<&str> = shown.into_iter().filter(|n| !n.is_empty()).collect();
    shown.join(" ")
}

/// A gene's numbers, as the Genome tab writes them, in the order it does:
/// `+.00428`, `past .5`. A threshold of 0 isn't written, so it's empty.
/// `gene_text` writes a gene's numbers from these, so the two agree.
fn numbers(gene: &GeneView) -> Vec<String> {
    let past = |threshold: f32| {
        if threshold == 0.0 {
            String::new()
        } else {
            format!("past{BOUND}{}", significant(threshold))
        }
    };
    match *gene {
        GeneView::Trait { which, value } => vec![trait_value(which, value)],
        GeneView::HalfLife { ticks: 1, .. } => vec![format!("every{BOUND}tick")],
        GeneView::HalfLife { ticks, .. } => vec![format!(
            "every{BOUND}{}{BOUND}ticks",
            thousands(u64::from(ticks))
        )],
        GeneView::Reaction { rate, .. } => vec![format!("rate{BOUND}{}", significant(rate))],
        GeneView::Emitter {
            threshold, gain, ..
        } => vec![signed(gain), past(threshold)],
        GeneView::Receptor {
            threshold, gain, ..
        } => vec![past(threshold), signed(gain)],
        GeneView::BrainParam { value, .. } | GeneView::InitialConcentration { value, .. } => {
            vec![significant(value)]
        }
        GeneView::Instinct { weight, .. } | GeneView::AttentionInstinct { weight, .. } => {
            vec![signed(weight)]
        }
        GeneView::Unknown { .. } => Vec::new(),
    }
}

/// What a gene does, without its numbers: `low energy → hunger`, `thirst →
/// drink`, `speed`.
fn gene_name(gene: &GeneView) -> String {
    match *gene {
        GeneView::Trait { which, .. } => trait_name(which).into(),
        GeneView::HalfLife { chem, .. } => format!("{} halves", display_name(chem)),
        GeneView::Reaction {
            ref reactants,
            ref products,
            ..
        } => reaction_sides(reactants, products),
        GeneView::Emitter {
            locus,
            mode,
            invert,
            chem,
            ..
        } => format!(
            "{} → {}",
            emitter_source(locus, mode, invert),
            display_name(chem)
        ),
        GeneView::Receptor { chem, target, .. } => {
            format!("{} → {}", display_name(chem), display_name(target))
        }
        GeneView::BrainParam { param, .. } => display_name(param),
        GeneView::Instinct {
            ref inputs, verb, ..
        } => format!(
            "{} → {}",
            concept_name(inputs),
            verb_name(verb).to_lowercase()
        ),
        GeneView::AttentionInstinct {
            input,
            category,
            whom,
            ..
        } => format!(
            "{} → attends to {}",
            display_name(input),
            attended(category, whom)
        ),
        GeneView::InitialConcentration { chem, .. } => format!("{}'s start", display_name(chem)),
        GeneView::Unknown { .. } => gene_text(gene),
    }
}

/// How weakened a gene is (M2 design §9.4), after the gene: ` · health .62`.
/// Nothing for a whole gene.
fn health_text(health: f32) -> String {
    if health < 1.0 {
        format!(" · health{BOUND}{}", level(health))
    } else {
        String::new()
    }
}

/// What an attention instinct draws the eye to: `sprite`, or, about whom
/// it is (M2 design §15.6), `its young (sprite)`, and about grown-ups,
/// whoever's they are, `grown-ups (sprite)` (§18.4).
fn attended(category: &str, whom: Whom) -> String {
    match (whom, whom.name()) {
        (Whom::Grown, Some(whom)) => format!("{whom} ({})", display_name(category)),
        (_, Some(whom)) => format!("its {whom} ({})", display_name(category)),
        (_, None) => display_name(category),
    }
}

/// When a gene that switches with life stage works (M2 design §9.2), after
/// the gene: ` · from adult`, ` · until adult`, ` · from adult until old`. Nothing for
/// a gene that works all its life.
fn switch_text(switch: Switch) -> String {
    match (switch.from, switch.until) {
        (LifeStage::Young, None) => String::new(),
        (from, None) => format!(" · from{BOUND}{}", from.name()),
        (LifeStage::Young, Some(until)) => format!(" · until{BOUND}{}", until.name()),
        (from, Some(until)) => {
            format!(" · from{BOUND}{} until{BOUND}{}", from.name(), until.name())
        }
    }
}

/// A gene as a plain line: `low energy → hunger +.00428 past .5`.
fn gene_text(gene: &GeneView) -> String {
    let numbers = numbers(gene);
    let n = |at: usize| numbers[at].as_str();
    // A number that may be empty, with a space before it if it isn't.
    let then = |at: usize| {
        if numbers[at].is_empty() {
            String::new()
        } else {
            format!(" {}", numbers[at])
        }
    };
    match *gene {
        GeneView::Trait { which, .. } => format!("{} {}", trait_name(which), n(0)),
        GeneView::HalfLife { chem, .. } => format!("{} halves {}", display_name(chem), n(0)),
        GeneView::Reaction {
            ref reactants,
            ref products,
            ..
        } => format!("{}, {}", reaction_sides(reactants, products), n(0)),
        GeneView::Emitter {
            locus,
            mode,
            invert,
            chem,
            ..
        } => format!(
            "{} → {} {}{}",
            emitter_source(locus, mode, invert),
            display_name(chem),
            n(0),
            then(1)
        ),
        GeneView::Receptor { chem, target, .. } => format!(
            "{}{} → {} {}",
            display_name(chem),
            then(0),
            display_name(target),
            n(1)
        ),
        GeneView::BrainParam { param, .. } => format!("{} {}", display_name(param), n(0)),
        GeneView::Instinct {
            ref inputs, verb, ..
        } => format!(
            "{} → {} {}",
            concept_name(inputs),
            verb_name(verb).to_lowercase(),
            n(0)
        ),
        GeneView::AttentionInstinct {
            input,
            category,
            whom,
            ..
        } => format!(
            "{} → attends to {} {}",
            display_name(input),
            attended(category, whom),
            n(0)
        ),
        GeneView::InitialConcentration { chem, .. } => {
            format!("{} starts at{BOUND}{}", display_name(chem), n(0))
        }
        GeneView::Unknown {
            type_id,
            version,
            bytes: 1,
        } => format!("type {type_id}, version {version}, 1 byte"),
        GeneView::Unknown {
            type_id,
            version,
            bytes,
        } => format!("type {type_id}, version {version}, {bytes} bytes"),
    }
}

/// A reaction's two sides: `2 h0 → h1 + h2`.
fn reaction_sides(reactants: &[(&str, u8)], products: &[(&str, u8)]) -> String {
    let side = |terms: &[(&str, u8)]| {
        if terms.is_empty() {
            return "nothing".to_string();
        }
        let terms: Vec<String> = terms
            .iter()
            .map(|&(chem, n)| match n {
                1 => display_name(chem),
                n => format!("{n} {}", display_name(chem)),
            })
            .collect();
        terms.join(" + ")
    };
    format!("{} → {}", side(reactants), side(products))
}

/// What an emitter reads, as the Genome tab says it: `low energy`,
/// `injury rises`.
fn emitter_source(locus: &str, mode: EmitterMode, invert: bool) -> String {
    let locus = display_name(locus);
    match mode {
        EmitterMode::Level if invert => format!("low {locus}"),
        EmitterMode::Level => locus,
        EmitterMode::Rise => format!("{locus} rises"),
        EmitterMode::Fall => format!("{locus} falls"),
    }
}

/// What `gene` does, in a sentence, as the status line says it while the
/// pointer is on it (#304). What each kind of gene does is the engine's,
/// so the sentence's shape is the screen's; the names in it are the data
/// pack's: `An emitter: hunger rises while energy is below .7`.
pub(crate) fn gene_about(gene: &GeneView) -> String {
    let raises = |by: f32| if by < 0.0 { "lowers" } else { "raises" };
    let past = |threshold: f32| {
        if threshold == 0.0 {
            String::new()
        } else {
            format!(", once past {}", significant(threshold))
        }
    };
    match *gene {
        GeneView::Trait { which, .. } => format!(
            "A trait: {} is {}",
            trait_name(which),
            match which {
                Trait::Speed => "how fast it walks, and the faster the more energy a step costs",
                Trait::SenseRadius => "how far it senses, and the further the more energy it costs",
                Trait::GrowsUp => "the age it grows up at",
                Trait::GrowsOld => "the age it grows old at",
            }
        ),
        GeneView::HalfLife { chem, .. } => format!(
            "A half-life: {} fades by itself, by half {}",
            display_name(chem),
            numbers(gene)[0].replace(BOUND, " ")
        ),
        GeneView::Reaction {
            ref reactants,
            ref products,
            ..
        } => {
            let sides = reaction_sides(reactants, products);
            let (from, to) = sides.split_once(" → ").unwrap_or((&sides, ""));
            format!("A reaction: each tick, {from} turns into {to}")
        }
        GeneView::Emitter {
            locus,
            mode,
            invert,
            threshold,
            gain,
            chem,
        } => {
            let rises = if gain < 0.0 { "falls" } else { "rises" };
            let (chem, locus) = (display_name(chem), display_name(locus));
            match mode {
                EmitterMode::Level if invert => format!(
                    "An emitter: {chem} {rises} while {locus} is below {}",
                    significant(1.0 - threshold)
                ),
                EmitterMode::Level if threshold > 0.0 => format!(
                    "An emitter: {chem} {rises} while {locus} is above {}",
                    significant(threshold)
                ),
                EmitterMode::Level => format!(
                    "An emitter: {locus} {} {chem}{}",
                    raises(gain),
                    past(threshold)
                ),
                EmitterMode::Rise => {
                    format!(
                        "An emitter: {chem} {rises} as {locus} rises{}",
                        past(threshold)
                    )
                }
                EmitterMode::Fall => {
                    format!(
                        "An emitter: {chem} {rises} as {locus} falls{}",
                        past(threshold)
                    )
                }
            }
        }
        GeneView::Receptor {
            chem,
            threshold,
            gain,
            target,
        } => format!(
            "A receptor: {} {} its {}{}",
            display_name(chem),
            raises(gain),
            display_name(target),
            past(threshold)
        ),
        GeneView::InitialConcentration { chem, value } => format!(
            "A starting level: {} starts at {} when it hatches",
            display_name(chem),
            significant(value)
        ),
        GeneView::BrainParam { param, value } => format!(
            "A brain setting: its {} is {}, one of the numbers its brain works by",
            display_name(param),
            significant(value)
        ),
        GeneView::Instinct {
            ref inputs,
            verb,
            weight,
        } => format!(
            "An instinct: {} leans it {} {}",
            concept_name(inputs).replace(BOUND, " "),
            if weight < 0.0 { "away from" } else { "towards" },
            verb_name(verb).to_lowercase()
        ),
        GeneView::AttentionInstinct {
            input,
            category,
            whom,
            weight,
        } => format!(
            "An attention instinct: {} draws its eye {} {}",
            display_name(input),
            if weight < 0.0 { "away from" } else { "to" },
            attended(category, whom)
        ),
        GeneView::Unknown { .. } => "A gene this version of the game can't read".into(),
    }
}
