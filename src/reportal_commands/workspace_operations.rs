//! Regeneration of workspace directories + `.code-workspace` files
//! from the current config.
//!
//! `WorkspaceRegenerator` is the single chokepoint through which
//! every `rep workspace` subcommand materializes a workspace on
//! disk, so the resolution of member repo aliases to absolute
//! paths, the decision of where the workspace directory lives on
//! disk, and the symlink / junction creation all stay in one place
//! and cannot drift between callers.
//!
//! Since v0.15.2 the workspace is a real directory on disk that
//! contains the `.code-workspace` file plus one symlink / junction
//! per member repo, not a loose `.code-workspace` file under
//! `~/.reportal/workspaces/`. The regenerator creates and maintains
//! that directory structure via the `workspace_layout` module.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_layout::{
    materialize_workspace_layout, workspace_file_path_inside_dir, WorkspaceLayoutParams,
    WorkspaceLinkSpec,
};
use crate::reportal_config::{ReportalConfig, WorkspaceEntry, WorkspaceMember};
use std::path::PathBuf;

/// Regenerates workspace directories and their `.code-workspace`
/// files from a borrowed config.
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

/// Regeneration methods that walk workspace membership and produce
/// the materialized workspace directory on disk.
impl<'config_lifetime> WorkspaceRegenerator<'config_lifetime> {
    /// Builds a regenerator that reads workspace and repo data from
    /// the given config reference.
    #[must_use]
    pub fn for_config(config_registry: &'config_lifetime ReportalConfig) -> Self {
        Self { config_registry }
    }

    /// Rebuilds the workspace directory, its member symlinks /
    /// junctions, and its `.code-workspace` file, and returns the
    /// path of the workspace file.
    ///
    /// Idempotent: running against an existing workspace directory
    /// updates links whose targets have moved, leaves correct links
    /// alone, and preserves user-authored fields inside the
    /// `.code-workspace` file (only the `folders` array is replaced).
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
    /// name is not registered, [`ReportalError::RepoNotFound`] if
    /// any member alias does not resolve, or
    /// [`ReportalError::CodeWorkspaceIoFailure`] /
    /// [`ReportalError::ValidationFailure`] for directory, link, or
    /// `.code-workspace` I/O failures surfaced by
    /// [`materialize_workspace_layout`].
    pub fn regenerate_workspace_file(
        &self,
        workspace_name: &str,
    ) -> Result<PathBuf, ReportalError> {
        let target_workspace = self.config_registry.get_workspace(workspace_name)?;
        let member_links = self.build_member_link_specs(target_workspace)?;
        let workspace_directory = self.resolve_workspace_directory(workspace_name)?;
        materialize_workspace_layout(&WorkspaceLayoutParams {
            workspace_directory: &workspace_directory,
            workspace_name,
            member_links: &member_links,
        })
    }

    /// Resolves the absolute path of the workspace's on-disk
    /// directory (the one that contains the `.code-workspace` file
    /// and member symlinks / junctions).
    ///
    /// Honors an explicit `path` field on the entry if set. Pre-v0.15.2
    /// entries that still store a `.code-workspace` file path in
    /// that field are interpreted as legacy and mapped to the
    /// default layout under `<default_workspace_root>/<name>/`, so
    /// `rjw` on an un-migrated workspace still lands in the right
    /// place. Empty `path` falls back to that same default.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] if the name is
    /// unknown, or [`ReportalError::ConfigIoFailure`] if the home
    /// directory is needed for the default and cannot be resolved.
    pub fn resolve_workspace_directory(
        &self,
        workspace_name: &str,
    ) -> Result<PathBuf, ReportalError> {
        let target_workspace = self.config_registry.get_workspace(workspace_name)?;
        let raw_path = target_workspace.raw_workspace_file_path().trim();
        if raw_path.is_empty() || target_workspace.is_legacy_file_path() {
            let default_root = self.config_registry.resolve_default_workspace_root()?;
            return Ok(default_root.join(workspace_name));
        }
        let expanded_path = shellexpand::tilde(raw_path);
        Ok(PathBuf::from(expanded_path.as_ref()))
    }

    /// Resolves the absolute path of the `.code-workspace` file
    /// the `rep workspace open` / `row` commands should launch.
    ///
    /// The file lives inside the workspace directory as
    /// `<workspace-dir>/<name>.code-workspace`.
    ///
    /// # Errors
    ///
    /// Returns the same error set as
    /// [`Self::resolve_workspace_directory`].
    pub fn resolve_workspace_file_path(
        &self,
        workspace_name: &str,
    ) -> Result<PathBuf, ReportalError> {
        let directory = self.resolve_workspace_directory(workspace_name)?;
        Ok(workspace_file_path_inside_dir(&directory, workspace_name))
    }

    /// Resolves every workspace member to the link-spec shape the
    /// layout materializer consumes: a short link name (the repo
    /// alias, or the inline path's file-stem) and the absolute
    /// target path.
    ///
    /// Inline-path members fall back to the path's final component
    /// for the link name, preserving the pre-v0.15.2 behavior where
    /// inline members had no stable identifier beyond their path.
    /// The declared order is preserved so the `.code-workspace`
    /// `folders[]` order and the on-disk directory listing match
    /// what the user wrote.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::RepoNotFound`] at the first alias
    /// that does not resolve against the repo registry.
    fn build_member_link_specs(
        &self,
        target_workspace: &WorkspaceEntry,
    ) -> Result<Vec<WorkspaceLinkSpec>, ReportalError> {
        let declared_members = target_workspace.members();
        let mut resolved_specs = Vec::with_capacity(declared_members.len());
        for member in declared_members {
            match member {
                WorkspaceMember::RegisteredRepo(repo_alias) => {
                    let member_repo = self.config_registry.get_repo(repo_alias)?;
                    resolved_specs.push(WorkspaceLinkSpec {
                        link_name: repo_alias.clone(),
                        target_absolute_path: member_repo.resolved_path(),
                    });
                }
                WorkspaceMember::InlinePath { path } => {
                    let expanded = shellexpand::tilde(path);
                    let target_path = PathBuf::from(expanded.as_ref());
                    let link_name = inline_path_link_name(&target_path);
                    resolved_specs.push(WorkspaceLinkSpec {
                        link_name,
                        target_absolute_path: target_path,
                    });
                }
            }
        }
        Ok(resolved_specs)
    }
}

/// Derives a short, stable link name for an inline-path workspace
/// member from the path's final component.
///
/// Falls back to `"inline"` if the path has no usable final
/// component (e.g. a bare `/`), which is unlikely in practice but
/// keeps the materializer from ever emitting an empty link name.
fn inline_path_link_name(target_path: &std::path::Path) -> String {
    target_path
        .file_name()
        .map_or_else(
            || "inline".to_owned(),
            |os_str| os_str.to_string_lossy().into_owned(),
        )
}
