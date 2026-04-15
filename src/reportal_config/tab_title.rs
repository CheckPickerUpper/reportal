//! Terminal tab title preference for a registered repo.

use serde::{Deserialize, Serialize};

/// Whether a repo has a custom tab title or falls back to its alias.
///
/// Modeled as an enum rather than an `Option<String>` so the
/// "use the alias" case is a named variant instead of `None`, which
/// forces every caller to acknowledge the branch in pattern matches
/// and prevents the failure mode where a missing title silently
/// renders as an empty string in the terminal.
#[derive(Debug, Serialize, Clone, Default)]
#[serde(untagged)]
pub enum TabTitle {
    /// No custom title configured; the repo alias is used instead.
    #[default]
    UseAlias,
    /// A custom title the user chose for this repo's terminal tab.
    Custom(String),
}

/// Deserializes an empty string as `UseAlias`, non-empty as `Custom`.
impl<'de> Deserialize<'de> for TabTitle {
    fn deserialize<D: serde::Deserializer<'de>>(
        tab_title_deserializer: D,
    ) -> Result<Self, D::Error> {
        let raw: String = String::deserialize(tab_title_deserializer)?;
        if raw.is_empty() {
            Ok(TabTitle::UseAlias)
        } else {
            Ok(TabTitle::Custom(raw))
        }
    }
}
