//! How the screen writes names and numbers.

use terra_sim::{DataPack, DeathCause, EntityId};

/// A name from the data, as shown on screen: `berry_bush` → `berry bush`.
pub(crate) fn display_name(name: &str) -> String {
    name.replace('_', " ")
}

/// How the screen names a sprite. Sprites have no names until the player
/// gives them one (design §6.5), so each shows by its ID.
pub(crate) fn sprite_label(id: EntityId) -> String {
    format!("Sprite #{}", id.0)
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

/// A level with its sign: `+.42`, `-.10`.
pub(crate) fn signed_level(value: f32) -> String {
    if value < 0.0 {
        level(value)
    } else {
        format!("+{}", level(value))
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
