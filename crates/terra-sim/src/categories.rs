//! Categories (design v19 §3.5.5): what a sprite perceives a thing as, listed
//! in `categories.ron` with permanent IDs.

use serde::Deserialize;

use crate::data::{DataError, check_unique};
use crate::registry::CategoryId;

/// The categories file, relative to the pack root.
pub(crate) const CATEGORIES: &str = "categories.ron";

/// The categories the engine perceives physics as: water tiles and sprites,
/// which aren't objects, so no object type can say what they are.
pub(crate) const WATER: &str = "water";
pub(crate) const SPRITE: &str = "sprite";

/// One category.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Category {
    pub(crate) id: CategoryId,
    pub(crate) name: String,
    /// How the screen says the category in general, such as "bushes"; none
    /// for one you don't count, such as water.
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "the screen words a category's summary by it from slice 9e (design v19 §6.1)"
    )]
    pub(crate) plural: Option<String>,
}

/// The IDs a category can have: each one's brain input needs an ID of its
/// own among those kept for Target inputs (design v19 §3.5.5).
const IDS: std::ops::RangeInclusive<u16> = 1..=26;

/// The categories `categories.ron` lists, in ascending ID order.
pub(crate) fn categories(mut entries: Vec<Category>) -> Result<Vec<Category>, DataError> {
    let invalid = |message: String| DataError::Invalid {
        file: CATEGORIES.into(),
        message,
    };
    check_unique(
        CATEGORIES,
        entries.iter().map(|c| (c.id.0, c.name.as_str())),
    )?;
    if let Some(c) = entries.iter().find(|c| !IDS.contains(&c.id.0)) {
        return Err(invalid(format!(
            "the id {} is outside {} to {}: each category's brain input needs an ID of its own",
            c.id.0,
            IDS.start(),
            IDS.end()
        )));
    }
    for physics in [WATER, SPRITE] {
        if !entries.iter().any(|c| c.name == physics) {
            return Err(invalid(format!(
                "`{physics}` is missing: the world always has it, so brains must perceive it"
            )));
        }
    }
    entries.sort_by_key(|c| c.id);
    Ok(entries)
}

/// How the name of a category's brain input starts: `attended_bush`.
pub(crate) const ATTENDED: &str = "attended_";

/// The ID of the brain input that's 1 while attention is on the category
/// `id` (design v19 §3.5.5): `35 + id` up to 6, then `37 + id`, past
/// `target_distance` and `target_adjacent`.
pub(crate) fn attended_input(id: CategoryId) -> u16 {
    if id.0 <= 6 { 35 + id.0 } else { 37 + id.0 }
}
