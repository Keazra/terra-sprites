//! Categories (design v19 §3.5.5): what a sprite perceives a thing as, listed
//! in `categories.ron` with permanent IDs.

use serde::Deserialize;

use crate::data::{DataError, check_unique};

/// The categories file, relative to the pack root.
pub(crate) const CATEGORIES: &str = "categories.ron";

/// One category.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CategoryEntry {
    pub(crate) id: u16,
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
pub(crate) fn categories(mut entries: Vec<CategoryEntry>) -> Result<Vec<CategoryEntry>, DataError> {
    check_unique(CATEGORIES, entries.iter().map(|c| (c.id, c.name.as_str())))?;
    if let Some(c) = entries.iter().find(|c| !IDS.contains(&c.id)) {
        return Err(DataError::Invalid {
            file: CATEGORIES.into(),
            message: format!(
                "the id {} is outside {} to {}: each category's brain input needs an ID of its own",
                c.id,
                IDS.start(),
                IDS.end()
            ),
        });
    }
    entries.sort_by_key(|c| c.id);
    Ok(entries)
}
