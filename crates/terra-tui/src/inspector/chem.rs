//! The Chem tab (design §6.1): each chemical's level and its change per
//! tick, and the hormones.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use terra_sim::{ChemicalKind, ChemicalLevel, SpriteView};

use crate::text::{change, display_name, level, signed_level};
use crate::theme::TextLook;

/// The Chem tab (design §6.1): each chemical on its own line with its level
/// and its change per tick, the physical chemicals, then the signal
/// chemicals, with what the sprite felt on the reward line; then the
/// hormones' levels, four to a line. A rise is in the theme's good colour
/// and a fall in its bad, and the hormones' heading in its heading colour
/// (design §6.2).
pub(super) fn chem_tab(sprite: &SpriteView, look: TextLook) -> Vec<Line<'static>> {
    let chemicals: Vec<ChemicalLevel> = sprite.chemicals().collect();
    let line = |c: &ChemicalLevel| {
        let name = display_name(c.name);
        let mut spans = vec![Span::raw(format!(" {name:<13}{:>4}", level(c.level)))];
        // Reward is used up every tick, so its line shows what learning
        // took in instead of a change: `last_r` (design §6.1).
        let (words, number) = match c.name {
            "reward" => ("felt ", Some(signed_level(sprite.felt()))),
            _ => ("", change(c.change)),
        };
        if let Some(number) = number {
            spans.push(Span::raw(format!("  {words}")));
            let style = look.signed(&number, Style::default());
            spans.push(Span::styled(number, style));
        }
        Line::from(spans)
    };
    let of_kinds = |kinds: &[ChemicalKind]| {
        chemicals
            .iter()
            .filter(|c| kinds.contains(&c.kind))
            .collect::<Vec<_>>()
    };
    let mut lines: Vec<Line<'static>> = of_kinds(&[ChemicalKind::Physical])
        .into_iter()
        .map(line)
        .collect();
    lines.push(Line::default());
    lines.extend(
        of_kinds(&[ChemicalKind::Drive])
            .into_iter()
            .chain(of_kinds(&[ChemicalKind::LearningSignal]))
            .map(line),
    );
    // The genes' own hormones, h0 to h15, four to a line; one with a
    // meaning of its own, as the feminiser (M2 design §10), has a line.
    lines.push(Line::styled(" HORMONES", look.heading));
    let (numbered, named): (Vec<&ChemicalLevel>, Vec<_>) = of_kinds(&[ChemicalKind::Hormone])
        .into_iter()
        .partition(|c| {
            let number = c.name.strip_prefix('h').unwrap_or_default();
            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
        });
    for four in numbered.chunks(4) {
        let cells: Vec<String> = four
            .iter()
            .map(|c| format!("{:<4}{:>4}", display_name(c.name), level(c.level)))
            .collect();
        lines.push(Line::from(format!(" {}", cells.join("   "))));
    }
    lines.extend(named.into_iter().map(line));
    lines
}
