//! CLI args for `rep ai`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repo_selection_args::RepoSelectionArgs;

/// Arguments for the `rep ai` subcommand.
///
/// Supports repo selection and an optional `--tool` override to
/// choose which AI coding CLI to launch.
#[derive(Args)]
pub struct AiArgs {
    #[command(flatten)]
    selection: RepoSelectionArgs,
    /// Which AI tool to launch (overrides `default_ai_tool` setting)
    #[arg(long, default_value = "", hide_default_value = true)]
    tool: String,
}

/// Consuming conversion that splits into domain-layer parts.
impl AiArgs {
    /// Returns (alias, `tag_filter`, `tool_override`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter, String) {
        let (alias, tag_filter) = self.selection.into_parts();
        (alias, tag_filter, self.tool)
    }
}
