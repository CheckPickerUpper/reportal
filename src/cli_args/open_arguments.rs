//! CLI args for `rep open`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::repository_selection_arguments::RepositorySelectionArguments;

/// Arguments for the `rep open` subcommand.
///
/// Supports repo selection, an optional editor override, and an
/// optional tab title override.
#[derive(Args)]
pub struct OpenArguments {
    #[command(flatten)]
    selection: RepositorySelectionArguments,
    /// Override the default editor command
    #[arg(long, default_value = "", hide_default_value = true)]
    editor: String,
    /// Override the tab title for this session
    #[arg(long, default_value = "", hide_default_value = true)]
    title: String,
}

/// Consuming conversion that splits into domain-layer parts.
impl OpenArguments {
    /// Returns (alias, `tag_filter`, `editor_override`, `title_override`), consuming self.
    pub fn into_parts(self) -> (String, TagFilter, String, String) {
        let (alias, tag_filter) = self.selection.into_parts();
        (alias, tag_filter, self.editor, self.title)
    }
}
