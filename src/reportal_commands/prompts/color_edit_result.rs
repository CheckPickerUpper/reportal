//! Outcome type for the color edit prompt.

use crate::reportal_config::HexColor;

/// Whether the user changed, cleared, or kept the color during repo editing.
pub enum ColorEditResult {
    /// The user entered a new valid hex color.
    Provided(HexColor),
    /// The user cleared the color (entered empty).
    Cleared,
    /// The user kept the existing color unchanged.
    Unchanged(HexColor),
}
