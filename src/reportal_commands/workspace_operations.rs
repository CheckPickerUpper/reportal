//! Regeneration of `.code-workspace` files from the current config.
//!
//! `WorkspaceRegenerator` is the single chokepoint through which
//! every `rep workspace` subcommand writes a `.code-workspace` file,
//! so the resolution of member repo aliases to absolute paths and
//! the decision of where the file lives on disk cannot drift between
//! callers. The struct holds a borrowed reference to
//! `ReportalConfig` so each helper method carries the registry
//! context via `&self` and takes at most one additional argument,
//! which is the shape required by the project's param-count rules.

use crate::code_workspace::CodeWorkspaceFile;
use crate::error::ReportalError;
use crate::reportal_config::{
    ReportalConfig, WorkspaceEntry, WorkspaceMember,
};
use std::path::PathBuf;

/// Regenerates `.code-workspace` files from a borrowed config.
///
/// Constructed once per command invocation with
/// `WorkspaceRegenerator::for_config(&config)` and then asked to
/// regenerate one or more workspaces by name. Holding the config
/// reference as state means each helper method has exactly one
/// non-`self` parameter (the workspace name), which the project's
/// positional-argument rules require.
pub struct WorkspaceRegenerator<'config_lifetime> {
    /// The loaded config supplying repo paths and workspace entries.
    config_registry: &'config_lifetime ReportalConfig,
}

/// Regeneration methods that walk workspace membership and write
/// the resulting `.code-workspace` file to disk.
impl<'config_lifetime> WorkspaceRegenerator<'config_lifetime> {
    /// Builds a regenerator that reads workspace and repo data from
    /// the given config reference.
    #[must_use]
    pub fn for_config(config_registry: &'config_lifetime ReportalConfig) -> Self {
        Self { config_registry }
    }

    /// Regenerates the on-disk `.code-workspace` file for the named
    /// workspace and returns the resolved file path.
    ///
    /// Loads the existing file if present so the parse-merge-write
    /// path preserves user-authored top-level fields (settings,
    /// extensions, launch, tasks, comments) byte-for-byte, replaces
    /// only the `folders` array with entries built from the current
    /// absolute paths of the member repos in declared order, and
    /// writes the mutated document back. Creates the parent
    /// directory if the default location under
    /// `~/.reportal/workspaces/` does not yet exist.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
    /// name is not registered, [`ReportalError::RepoNotFound`] if any
    /// member alias does not resolve to a registered repo,
    /// [`ReportalError::ConfigIoFailure`] if the home directory
    /// cannot be resolved for the default path, or
    /// [`ReportalError::CodeWorkspaceIoFailure`] /
    /// [`ReportalError::CodeWorkspaceParseFailure`] if reading,
    /// parsing, or writing the file fails.
    pub fn regenerate_workspace_file(
        &self,
        workspace_name: &str,
    ) -> Result<PathBuf, ReportalError> {
        let target_workspace = self.config_registry.get_workspace(workspace_name)?;
        let member_absolute_paths = self.resolve_member_repo_paths(target_workspace)?;
        let workspace_file_path = self.resolve_workspace_file_path(workspace_name)?;
        let mut code_workspace_document =
            CodeWorkspaceFile::load_or_empty(&workspace_file_path)?;
        code_workspace_document.set_folder_paths(&member_absolute_paths);
        code_workspace_document.write_to_disk(&workspace_file_path)?;
        Ok(workspace_file_path)
    }

    /// Resolves every member repo alias in the given workspace to
    /// its absolute filesystem path, preserving the declared order.
    ///
    /// The ordering is load-bearing because it determines the
    /// folder order in the editor sidebar, so callers that later
    /// write these paths into the `folders` array get the same
    /// visual ordering the user declared in config.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::RepoNotFound`] at the first alias
    /// that does not resolve against the repo registry.
    fn resolve_member_repo_paths(
        &self,
        target_workspace: &WorkspaceEntry,
    ) -> Result<Vec<PathBuf>, ReportalError> {
        let declared_members = target_workspace.members();
        let mut resolved_paths = Vec::with_capacity(declared_members.len());
        for member in declared_members {
            match member {
                WorkspaceMember::RegisteredRepo(repo_alias) => {
                    let member_repo = self.config_registry.get_repo(repo_alias)?;
                    resolved_paths.push(member_repo.resolved_path());
                }
                WorkspaceMember::InlinePath { path } => {
                    let expanded = shellexpand::tilde(path);
                    resolved_paths.push(PathBuf::from(expanded.as_ref()));
                }
            }
        }
        Ok(resolved_paths)
    }

    /// Resolves the on-disk location of the `.code-workspace` file
    /// for the named workspace.
    ///
    /// Honors an explicit `path` field if the workspace entry sets
    /// one, expanding a leading `~` against the user home directory.
    /// Falls back to the default location
    /// `~/.reportal/workspaces/<name>.code-workspace` when the field
    /// is empty so workspaces created without a custom path land in
    /// a predictable directory reportal fully owns.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the name does
    /// not match a registered workspace, or
    /// [`ReportalError::ConfigIoFailure`] if the home directory is
    /// needed for the default location and cannot be resolved.
    fn resolve_workspace_file_path(
        &self,
        workspace_name: &str,
    ) -> Result<PathBuf, ReportalError> {
        let target_workspace = self.config_registry.get_workspace(workspace_name)?;
        let raw_path = target_workspace.raw_workspace_file_path();
        if raw_path.is_empty() {
            return default_workspace_file_location(workspace_name);
        }
        let expanded_path = shellexpand::tilde(raw_path);
        Ok(PathBuf::from(expanded_path.as_ref()))
    }
}

/// Computes the default on-disk location for a workspace file when
/// the workspace entry does not declare a custom path.
///
/// The path is `~/.reportal/workspaces/<name>.code-workspace`, which
/// places every default-located workspace in a single directory
/// under reportal's config root so discovery, cleanup, and manual
/// inspection are all straightforward.
///
/// # Errors
///
/// Returns [`ReportalError::ConfigIoFailure`] if the home directory
/// cannot be resolved, because without a home directory there is no
/// well-defined default location and silently falling back to a
/// relative path would produce an unpredictable file on disk.
fn default_workspace_file_location(workspace_name: &str) -> Result<PathBuf, ReportalError> {
    let home_directory =
        dirs::home_dir().ok_or_else(|| ReportalError::ConfigIoFailure {
            reason: "Could not determine home directory".to_owned(),
        })?;
    Ok(home_directory
        .join(".reportal")
        .join("workspaces")
        .join(format!("{workspace_name}.code-workspace")))
}
