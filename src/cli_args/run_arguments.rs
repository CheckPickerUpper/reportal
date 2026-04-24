//! CLI args for `rep run`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repository_selection_arguments::RepositorySelectionArguments;

/// Arguments for the `rep run` subcommand.
///
/// Supports repo selection and an optional `--cmd` to skip the
/// command fuzzy finder and run a specific configured command directly.
#[derive(Args)]
pub struct RunArguments {
    #[command(flatten)]
    selection: RepositorySelectionArguments,
    /// Run this command directly (skip command fuzzy finder)
    #[arg(long, default_value = "", hide_default_value = true)]
    cmd: String,
}

/// Consuming conversion that splits into domain-layer parts.
impl RunArguments {
    /// Returns (alias, `tag_filter`, `direct_command`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter, String) {
        let (alias, tag_filter) = self.selection.into_parts();
        (alias, tag_filter, self.cmd)
    }
}
