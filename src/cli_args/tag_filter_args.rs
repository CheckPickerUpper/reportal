//! Shared `--tag` flag used by commands that filter repos.

use clap::Args;
use crate::reportal_config::TagFilter;

/// Shared `--tag` flag that converts to a `TagFilter` enum.
///
/// When the flag is absent (empty default), resolves to `TagFilter::All`.
/// When provided, resolves to `TagFilter::ByTag` with the given value.
#[derive(Args)]
pub struct TagFilterArgs {
    /// Filter repos by this tag
    #[arg(long, default_value = "", hide_default_value = true)]
    tag: String,
}

/// Consuming conversion to the domain's `TagFilter` enum.
impl TagFilterArgs {
    /// Converts the CLI flag value into a `TagFilter`.
    ///
    /// Empty string (absent flag) becomes `All`; any non-empty value
    /// becomes `ByTag` with that string.
    pub fn into_tag_filter(self) -> TagFilter {
        if self.tag.is_empty() { TagFilter::All } else { TagFilter::ByTag(self.tag) }
    }
}
