//! Categories (design v19 §3.5.5): what a sprite perceives a thing as, listed
//! in `categories.ron` with permanent IDs.

use serde::Deserialize;

use crate::brain_io::MOST_CATEGORIES;
use crate::data::{DataError, check_unique};
use crate::registry::CategoryId;

/// The categories file, relative to the pack root.
pub(crate) const CATEGORIES: &str = "categories.ron";

/// The categories water tiles, sprites and the Cursor are perceived as.
/// They aren't objects, so no object type can say what they are, and every
/// world has them, so the list must too (design v29 §3.5.5).
pub(crate) const WATER: &str = "water";
pub(crate) const SPRITE: &str = "sprite";
pub(crate) const CURSOR: &str = "cursor";

/// One category.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Category {
    pub(crate) id: CategoryId,
    pub(crate) name: String,
    /// How the screen says the category in general, such as "bushes"; none
    /// for one you don't count, such as water.
    #[serde(default)]
    pub(crate) plural: Option<String>,
}

/// The categories `categories.ron` lists, in ascending ID order, each
/// plural without spaces at either end.
pub(crate) fn categories(mut entries: Vec<Category>) -> Result<Vec<Category>, DataError> {
    let invalid = |message: String| DataError::Invalid {
        file: CATEGORIES.into(),
        message,
    };
    check_unique(
        CATEGORIES,
        entries.iter().map(|c| (c.id.0, c.name.as_str())),
    )?;
    // Each category's brain input needs an ID of its own (design v19 §3.5.5).
    if let Some(c) = entries
        .iter()
        .find(|c| !(1..=MOST_CATEGORIES).contains(&c.id.0))
    {
        return Err(invalid(format!(
            "the id {} is outside 1 to {MOST_CATEGORIES}: each category's brain input needs an ID of its own",
            c.id.0
        )));
    }
    for name in [WATER, SPRITE, CURSOR] {
        if !entries.iter().any(|c| c.name == name) {
            return Err(invalid(format!(
                "`{name}` is missing: every world has it, so brains must perceive it"
            )));
        }
    }
    for c in &mut entries {
        // Spaces at either end would put stray gaps in the screen's sentences.
        if let Some(plural) = &mut c.plural {
            *plural = plural.trim().to_string();
            if plural.is_empty() {
                return Err(invalid(format!(
                    "`{}` has an empty plural: leave it out for one you don't count",
                    c.name
                )));
            }
        }
    }
    entries.sort_by_key(|c| c.id);
    Ok(entries)
}
