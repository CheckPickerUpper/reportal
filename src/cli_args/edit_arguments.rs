//! CLI args for `rep edit`.

use super::repository_selection_arguments::RepositorySelectionArguments;
use crate::reportal_config::TagFilter;
use clap::Args;

/// Arguments for the `rep edit` subcommand.
///
/// Accepts optional repo selection (alias + tag filter) to choose
/// which repo's metadata to edit interactively.
#[derive(Args)]
pub struct EditArguments {
    #[command(flatten)]
    selection: RepositorySelectionArguments,
}

/// Consuming conversion that splits into domain-layer parts.
impl EditArguments {
    /// Returns (alias, `tag_filter`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter) {
        self.selection.into_parts()
    }
}
