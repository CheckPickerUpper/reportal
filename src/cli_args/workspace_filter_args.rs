//! Shared `--workspace` flag used by commands that filter repos
//! by workspace membership.

use crate::reportal_config::WorkspaceFilter;
use clap::Args;

/// Shared `--workspace` flag that converts to a `WorkspaceFilter`
/// enum.
///
/// When the flag is absent (empty default), resolves to
/// `WorkspaceFilter::All`. When provided, resolves to
/// `WorkspaceFilter::ByName` with the given workspace name. The
/// empty-string sentinel is used instead of `Option<String>`
/// because the project's rules forbid `Option` for domain state
/// where both branches have meaning.
#[derive(Args)]
pub struct WorkspaceFilterArgs {
    /// Filter repos by workspace membership
    #[arg(long = "workspace", default_value = "", hide_default_value = true)]
    workspace_name: String,
}

/// Consuming conversion to the domain's `WorkspaceFilter` enum.
impl WorkspaceFilterArgs {
    /// Converts the CLI flag value into a `WorkspaceFilter`.
    ///
    /// Empty string (absent flag) becomes `All`; any non-empty
    /// value becomes `ByName` with that workspace name.
    pub fn into_workspace_filter(self) -> WorkspaceFilter {
        if self.workspace_name.is_empty() {
            WorkspaceFilter::All
        } else {
            WorkspaceFilter::ByName(self.workspace_name)
        }
    }
}
