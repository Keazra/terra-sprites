//! How the screen writes names and numbers.

use std::collections::BTreeMap;
use std::ops::Deref;

use terra_sim::{DataPack, DeathCause, EntityId, Terrain};

/// A name from the data, as shown on screen: `berry_bush` → `berry bush`.
pub(crate) fn display_name(name: &str) -> String {
    name.replace('_', " ")
}

/// Why a fixture can't be grabbed (design v23 §6.1).
pub(crate) const ROOTED: &str = "it's rooted to the ground";

/// A terrain's name on screen: `deep water`.
pub(crate) fn terrain_name(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Grass => "grass",
        Terrain::Dirt => "dirt",
        Terrain::Sand => "sand",
        Terrain::ShallowWater => "shallow water",
        Terrain::DeepWater => "deep water",
        Terrain::Rock => "rock",
    }
}

/// How the screen names sprite `id`, called `name` or not yet named:
/// "Mira #12", or "Sprite #12" (design v26 §6.5).
pub(crate) fn label(id: EntityId, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("{name} #{}", id.0),
        None => format!("Sprite #{}", id.0),
    }
}

/// The names the player has given sprites (design §6.5), as the screen
/// last saw them: a sprite that has died keeps its name in the log.
#[derive(Debug, Clone, Default)]
pub struct Names(BTreeMap<EntityId, String>);

impl Names {
    /// How the screen names sprite `id`: "Mira #12" once the player has
    /// named it, and until then "Sprite #12" (design §6.5).
    pub fn label(&self, id: EntityId) -> String {
        label(id, self.get(id))
    }

    /// The name sprite `id` was given, if any.
    pub fn get(&self, id: EntityId) -> Option<&str> {
        self.0.get(&id).map(String::as_str)
    }

    /// Notes that sprite `id` is called `name`.
    pub(crate) fn note(&mut self, id: EntityId, name: &str) {
        if self.get(id) != Some(name) {
            self.0.insert(id, name.to_string());
        }
    }
}

/// What the screen's sentences are made from: the data pack's names for
/// things, and the player's for sprites. It reads as the data pack.
#[derive(Clone, Copy)]
pub(crate) struct Words<'a> {
    pub(crate) data: &'a DataPack,
    pub(crate) names: &'a Names,
}

impl Words<'_> {
    /// How the screen names sprite `id` (design §6.5).
    pub(crate) fn label(&self, id: EntityId) -> String {
        self.names.label(id)
    }
}

impl Deref for Words<'_> {
    type Target = DataPack;

    fn deref(&self) -> &DataPack {
        self.data
    }
}

/// A cause of death, as the screen names it: "hurt by thornbush" names the
/// object type from the data pack.
pub(crate) fn cause_name(cause: DeathCause, data: &DataPack) -> String {
    match cause {
        DeathCause::Starvation => "starvation".into(),
        DeathCause::Dehydration => "dehydration".into(),
        DeathCause::OldAge => "old age".into(),
        DeathCause::HurtBy(id) => {
            let name = data.object_type_name(id).unwrap_or("something");
            format!("hurt by {}", display_name(name))
        }
    }
}

/// `1234567` → `"1,234,567"`.
pub(crate) fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `value` rounded to a whole number, with its sign, grouped in thousands:
/// `-1,235`.
pub(crate) fn whole(value: f64) -> String {
    let sign = if value.round() < 0.0 { "-" } else { "" };
    format!("{sign}{}", group_thousands(value.abs().round() as u64))
}

/// A level: two decimals, with no leading zero: `.42`, `1.00`.
pub(crate) fn level(level: f32) -> String {
    without_leading_zero(&format!("{level:.2}"))
}

/// A change per tick: 4 decimals, with its sign and no leading zero
/// (`-.0002`), or `None` when it rounds to 0.
pub(crate) fn change(change: f32) -> Option<String> {
    let digits = format!("{:.4}", change.abs());
    if digits.chars().all(|c| c == '0' || c == '.') {
        return None;
    }
    let sign = if change > 0.0 { "+" } else { "-" };
    Some(format!("{sign}{}", without_leading_zero(&digits)))
}

/// `value` to 3 significant figures, with no trailing zeros and no leading
/// zero, and whole numbers grouped in thousands: `7.25`, `9.5`, `.00428`,
/// `1,230`.
pub(crate) fn significant(value: f32) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let magnitude = value.abs().log10().floor() as i32;
    if magnitude >= 2 {
        let unit = 10f64.powi(magnitude - 2);
        return whole((f64::from(value) / unit).round() * unit);
    }
    let decimals = (2 - magnitude) as usize;
    let text = format!("{value:.decimals$}");
    without_leading_zero(text.trim_end_matches('0').trim_end_matches('.'))
}

/// A gene value with its sign: `+.004`, `-.5`.
pub(crate) fn signed(value: f32) -> String {
    if value < 0.0 {
        significant(value)
    } else {
        format!("+{}", significant(value))
    }
}

/// A level with its sign: `+.42`, `-.10`. One that rounds to zero reads
/// `+.00`, never `-.00`.
pub(crate) fn signed_level(value: f32) -> String {
    let magnitude = level(value.abs());
    if value < 0.0 && magnitude != ".00" {
        format!("-{magnitude}")
    } else {
        format!("+{magnitude}")
    }
}

/// `0.42` → `.42`, `-0.5` → `-.5`.
fn without_leading_zero(number: &str) -> String {
    if let Some(rest) = number.strip_prefix("0.") {
        format!(".{rest}")
    } else if let Some(rest) = number.strip_prefix("-0.") {
        format!("-.{rest}")
    } else {
        number.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signed_level_that_rounds_to_zero_reads_plus_zero() {
        assert_eq!(signed_level(0.42), "+.42");
        assert_eq!(signed_level(-0.1), "-.10");
        assert_eq!(signed_level(-0.002), "+.00", "never -.00");
        assert_eq!(signed_level(0.0), "+.00");
    }
}
