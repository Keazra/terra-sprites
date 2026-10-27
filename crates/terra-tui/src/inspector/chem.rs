//! The Chem tab (design §6.1): each chemical's level and its change per
//! tick, and the hormones.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::{ChemicalKind, ChemicalLevel, SpriteView};

use super::TabLine;
use crate::spots::Spot;
use crate::text::{change, display_name, level, signed_level};
use crate::theme::TextLook;

/// The Chem tab (design §6.1): each chemical on its own line with its level
/// and its change per tick, the physical chemicals, then the signal
/// chemicals, with what the sprite felt on the reward line; then the
/// hormones' levels, four to a line. What was felt is in the theme's good
/// colour when it's a reward and its bad when it's a punishment, and the
/// hormones' heading in its heading colour (design §6.2). A change stays
/// plain: a rise is good for food and bad for hunger. The pointer on a
/// chemical's name, on its line or in the grid, says what it is (design
/// §6.1).
pub(super) fn chem_tab(sprite: &SpriteView, look: TextLook) -> Vec<TabLine> {
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    let line = |c: &ChemicalLevel| {
        let name = display_name(c.name);
        let mut spans = vec![Span::raw(format!(" {name:<13}{:>4}", level(c.level)))];
        // Reward is used up every tick, so its line shows what learning
        // took in instead of a change: `last_r` (design §6.1).
        let (words, number, felt) = match c.name {
            "reward" => ("felt ", Some(signed_level(sprite.felt())), true),
            _ => ("", change(c.change), false),
        };
        if let Some(number) = number {
            spans.push(Span::raw(format!("  {words}")));
            let style = if felt {
                look.signed(&number, Style::default())
            } else {
                Style::default()
            };
            spans.push(Span::styled(number, style));
        }
        TabLine::spotted_at(Line::from(spans), TabLine::name_columns(&name), spot(c))
    };
    let of_kinds = |kinds: &[ChemicalKind]| {
        chemicals
            .iter()
            .filter(|c| kinds.contains(&c.kind))
            .collect::<Vec<_>>()
    };
    let mut lines: Vec<TabLine> = of_kinds(&[ChemicalKind::Physical])
        .into_iter()
        .map(line)
        .collect();
    lines.push(Line::default().into());
    lines.extend(
        of_kinds(&[ChemicalKind::Drive])
            .into_iter()
            .chain(of_kinds(&[ChemicalKind::LearningSignal]))
            .map(line),
    );
    // The genes' own hormones, h0 to h15, four to a line; one with a
    // meaning of its own, as the feminiser (M2 design §10), has a line.
    lines.push(Line::styled(" HORMONES", look.heading).into());
    let (numbered, named): (Vec<&ChemicalLevel>, Vec<_>) = of_kinds(&[ChemicalKind::Hormone])
        .into_iter()
        .partition(|c| {
            let number = c.name.strip_prefix('h').unwrap_or_default();
            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
        });
    for four in numbered.chunks(4) {
        // Each cell's name is its spot, the cells 3 apart, after a space.
        let (mut text, mut spots) = (String::from(" "), Vec::new());
        for (i, c) in four.iter().enumerate() {
            if i > 0 {
                text.push_str("   ");
            }
            let (start, name) = (text.chars().count() as u16, display_name(c.name));
            text.push_str(&format!("{name:<4}{:>4}", level(c.level)));
            spots.push((start..start + name.chars().count() as u16, spot(c)));
        }
        lines.push(TabLine {
            line: Line::from(text),
            spots,
        });
    }
    lines.extend(named.into_iter().map(line));
    lines
}

/// The spot of chemical `c`'s name (design §6.1).
fn spot(c: &ChemicalLevel) -> Spot {
    Spot::chemical(c.name, c.kind)
}
