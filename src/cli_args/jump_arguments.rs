//! CLI args for `rep jump`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repository_selection_arguments::RepositorySelectionArguments;

/// Arguments for the `rep jump` subcommand.
///
/// Supports optional repo selection (alias + tag filter) and an
/// optional `--title` override for the terminal tab title.
#[derive(Args)]
pub struct JumpArguments {
    #[command(flatten)]
    selection: RepositorySelectionArguments,
    /// Override the tab title for this session
    #[arg(long, default_value = "", hide_default_value = true)]
    title: String,
}

/// Consuming conversion that splits into domain-layer parts.
impl JumpArguments {
    /// Returns (alias, `tag_filter`, `title_override`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter, String) {
        let (alias, tag_filter) = self.selection.into_parts();
        (alias, tag_filter, self.title)
    }
}
