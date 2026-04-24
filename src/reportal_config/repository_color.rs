//! Terminal background color preference for a registered repo.

use crate::reportal_config::hex_color::HexColor;
use serde::{Deserialize, Serialize};

/// Whether a repo has a terminal background color configured.
///
/// Modeled as an enum rather than an `Option<HexColor>` so the
/// "reset to default" case is a named variant instead of `None`,
/// which forces every caller to acknowledge the reset branch and
/// prevents the failure mode where a missing color silently leaves
/// the terminal on whatever color was last set by another repo.
#[derive(Debug, Serialize, Clone, Default)]
#[serde(untagged)]
pub enum RepoColor {
    /// No color set; the terminal resets to its default background on jump.
    #[default]
    ResetToDefault,
    /// A specific background color applied via OSC 11 on jump.
    Themed(HexColor),
}

/// Unwrapping of the inner `HexColor` without exposing callers
/// to the enum's variant vocabulary.
impl RepoColor {
    /// @why Returns the themed hex color when one is set so
    /// callers that only care about "is there a color" can skip
    /// pattern-matching on the two-variant enum and keep their
    /// own files free of `Themed` / `ResetToDefault` references.
    #[must_use]
    pub fn themed_hex_color(&self) -> Option<&HexColor> {
        if let Self::Themed(hex_color) = self {
            Some(hex_color)
        } else {
            None
        }
    }
}

/// Deserializes an empty string as `ResetToDefault`, valid hex as `Themed`.
impl<'de> Deserialize<'de> for RepoColor {
    fn deserialize<D: serde::Deserializer<'de>>(
        repo_color_deserializer: D,
    ) -> Result<Self, D::Error> {
        let raw: String = String::deserialize(repo_color_deserializer)?;
        if raw.is_empty() {
            Ok(RepoColor::ResetToDefault)
        } else {
            let parsed = HexColor::parse(&raw).map_err(serde::de::Error::custom)?;
            Ok(RepoColor::Themed(parsed))
        }
    }
}
