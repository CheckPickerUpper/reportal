//! CLI args for `rep jump`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repo_selection_args::RepoSelectionArgs;

/// Arguments for the `rep jump` subcommand.
///
/// Supports optional repo selection (alias + tag filter) and an
/// optional `--title` override for the terminal tab title.
#[derive(Args)]
pub struct JumpArgs {
    #[command(flatten)]
    selection: RepoSelectionArgs,
    /// Override the tab title for this session
    #[arg(long, default_value = "", hide_default_value = true)]
    title: String,
}

/// Consuming conversion that splits into domain-layer parts.
impl JumpArgs {
    /// Returns (alias, `tag_filter`, `title_override`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter, String) {
        let (alias, tag_filter) = self.selection.into_parts();
        (alias, tag_filter, self.title)
    }
}
