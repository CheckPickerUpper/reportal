//! CLI args for `rep ai`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repository_selection_arguments::RepositorySelectionArguments;

/// Arguments for the `rep ai` subcommand.
///
/// Supports repo selection and an optional `--tool` override to
/// choose which AI coding CLI to launch.
#[derive(Args)]
pub struct AiArguments {
    #[command(flatten)]
    selection: RepositorySelectionArguments,
    /// Which AI tool to launch (overrides `default_ai_tool` setting)
    #[arg(long, default_value = "", hide_default_value = true)]
    tool: String,
}

/// Consuming conversion that splits into domain-layer parts.
impl AiArguments {
    /// Returns (alias, `tag_filter`, `tool_override`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter, String) {
        let (alias, tag_filter) = self.selection.into_parts();
        (alias, tag_filter, self.tool)
    }
}
