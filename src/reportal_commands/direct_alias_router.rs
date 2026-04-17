//! Shared workspace/repo alias-dispatch helper for `rep jump` /
//! `rep open`.
//!
//! Both commands accept a name-or-alias argument that may resolve
//! to a registered repo OR a workspace. Keeping the routing in one
//! file prevents `jump` and `open` from drifting on edge cases like
//! "what if the alias matches neither" or "how should a workspace
//! member's identity propagate into tab-title emission".

use crate::error::ReportalError;
use crate::reportal_commands::path_display::{self, SelectedPathDisplayParams};
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;

/// Wrapper over a borrowed `ReportalConfig` that exposes the
/// direct-alias classification and workspace-jump behaviors needed
/// by `run_jump` and `run_open`.
///
/// Holds the config reference as state so every method has exactly
/// one non-`self` parameter, which satisfies the project's
/// positional-argument rules without forcing a params struct at
/// each call site.
pub struct DirectAliasRouter<'config> {
    loaded_config: &'config ReportalConfig,
}

/// Classified outcome of resolving a direct-alias argument.
///
/// Named variants instead of `Result<Option<...>, _>` because the
/// three outcomes have distinct command-level handling and nesting
/// `Option` inside `Result` would force every caller to flatten the
/// shape by hand.
pub enum DirectAliasRouterOutcome {
    /// The alias matches a registered repo; the caller proceeds
    /// with the normal repo flow.
    RegisteredRepo,
    /// The alias matches a workspace; the payload is the canonical
    /// workspace name the caller passes to workspace resolution.
    Workspace(String),
    /// The alias matches neither a repo nor a workspace.
    Unknown,
}

/// Classification + workspace-jump methods for the router.
impl<'config> DirectAliasRouter<'config> {
    /// Builds a router backed by the given loaded config.
    #[must_use]
    pub fn for_config(loaded_config: &'config ReportalConfig) -> Self {
        Self { loaded_config }
    }

    /// Classifies the direct-alias input into a repo, a workspace,
    /// or neither so `run_jump` and `run_open` dispatch uniformly.
    ///
    /// Real I/O failures from either registry (anything that is not
    /// a plain "not found" error) propagate up; only the specific
    /// `RepoNotFound` / `WorkspaceNotFound` variants are folded into
    /// routing outcomes. This keeps an actual config-read failure
    /// from being silently rendered as `Unknown`.
    ///
    /// # Errors
    ///
    /// Returns any non-NotFound error surfaced by the underlying
    /// `get_repo` or `resolve_workspace_canonical_name` calls.
    pub fn classify(
        &self,
        alias: &str,
    ) -> Result<DirectAliasRouterOutcome, ReportalError> {
        match self.loaded_config.get_repo(alias) {
            Ok(_repo_found_for_classification) => {
                Ok(DirectAliasRouterOutcome::RegisteredRepo)
            }
            Err(ReportalError::RepoNotFound { .. }) => {
                self.classify_workspace_branch(alias)
            }
            Err(other_repo_error) => Err(other_repo_error),
        }
    }

    /// Second half of `classify` — only reached when the repo
    /// lookup returned `RepoNotFound`. Extracted so the outer
    /// method stays within the project's depth budget and the
    /// workspace-vs-unknown distinction is named explicitly rather
    /// than folded into a catch-all arm.
    ///
    /// # Errors
    ///
    /// Returns any non-NotFound error surfaced by
    /// `resolve_workspace_canonical_name`.
    fn classify_workspace_branch(
        &self,
        alias: &str,
    ) -> Result<DirectAliasRouterOutcome, ReportalError> {
        match self.loaded_config.resolve_workspace_canonical_name(alias) {
            Ok(canonical_workspace_name) => {
                Ok(DirectAliasRouterOutcome::Workspace(canonical_workspace_name))
            }
            Err(ReportalError::WorkspaceNotFound { .. }) => {
                Ok(DirectAliasRouterOutcome::Unknown)
            }
            Err(other_workspace_error) => Err(other_workspace_error),
        }
    }

    /// Prints the workspace's `.code-workspace` file parent
    /// directory to stdout so the `rj` shell wrapper cd's there.
    ///
    /// The parent directory is the common ancestor the user
    /// declared when they chose the workspace file location, which
    /// is the predictable cd target for a multi-folder workspace —
    /// no single member directory is correct when the workspace
    /// contains three sibling repos.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
    /// disappears between alias resolution and file-path resolution,
    /// or [`ReportalError::ConfigIoFailure`] if the default
    /// `~/.reportal/workspaces/` location needs the home directory
    /// and it cannot be resolved.
    pub fn jump_to_workspace_parent(
        &self,
        canonical_workspace_name: &str,
    ) -> Result<(), ReportalError> {
        let regenerator = WorkspaceRegenerator::for_config(self.loaded_config);
        let workspace_file_path =
            regenerator.resolve_workspace_file_path(canonical_workspace_name)?;
        let target_directory = match workspace_file_path.parent() {
            Some(parent_directory) => parent_directory.to_path_buf(),
            None => std::path::PathBuf::from("."),
        };
        let formatted_path = self
            .loaded_config
            .path_display_format()
            .format_path(&target_directory);
        terminal_style::write_stdout(&formatted_path);
        path_display::print_selected_path_if_visible(&SelectedPathDisplayParams {
            loaded_config: self.loaded_config,
            resolved_path: &target_directory,
        });
        Ok(())
    }
}
