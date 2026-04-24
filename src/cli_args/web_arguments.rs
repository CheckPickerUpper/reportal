//! CLI args for `rep web`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repository_selection_arguments::RepositorySelectionArguments;

/// Arguments for the `rep web` subcommand.
///
/// Supports optional repo selection (alias + tag filter) to choose
/// which repo's remote URL to open in the browser.
#[derive(Args)]
pub struct WebArguments {
    #[command(flatten)]
    selection: RepositorySelectionArguments,
}

/// Consuming conversion that splits into domain-layer parts.
impl WebArguments {
    /// Returns (alias, `tag_filter`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter) {
        self.selection.into_parts()
    }
}
