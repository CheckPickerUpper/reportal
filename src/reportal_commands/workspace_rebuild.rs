//! Rebuilds a workspace's on-disk directory from the current config.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Drops and recreates the workspace directory's member symlinks /
/// junctions and the `.code-workspace` file inside it, from the
/// current config.
///
/// Idempotent: safe to run repeatedly. Useful after a member repo
/// is moved on disk (symlink targets get refreshed), after the
/// user renames a repo alias (link names get renamed), or when the
/// workspace directory has been deleted by hand and the user wants
/// it back. Does NOT move or modify the member repos themselves —
/// only the symlinks / junctions inside the workspace directory.
///
/// Auto-migrates pre-v0.15.2 workspaces (those whose stored `path`
/// still references a `.code-workspace` file rather than a
/// directory) by rewriting the config to the new directory-based
/// shape before materializing on disk. This is the explicit
/// migration path users run to formalize the layout change.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the name does
/// not match a registered workspace,
/// [`ReportalError::RepoNotFound`] if any member alias does not
/// resolve, or [`ReportalError::CodeWorkspaceIoFailure`] /
/// [`ReportalError::ValidationFailure`] for directory, link, or
/// file I/O failures.
pub fn run_workspace_rebuild(alias_or_canonical: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_name = loaded_config.resolve_workspace_canonical_name(alias_or_canonical)?;

    // Formalize a legacy file-path entry into a directory-path
    // entry before materializing, so subsequent loads stop
    // printing the migration notice.
    let was_legacy = {
        let target_workspace = loaded_config.get_workspace(&canonical_name)?;
        target_workspace.is_legacy_file_path()
    };
    if was_legacy {
        let new_directory_path = loaded_config
            .resolve_default_workspace_root()?
            .join(&canonical_name);
        let target_workspace = loaded_config.get_workspace_mut(&canonical_name)?;
        target_workspace
            .set_workspace_directory_path(new_directory_path.display().to_string());
        loaded_config.save_to_disk()?;
    }

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_file_path = regenerator.regenerate_workspace_file(&canonical_name)?;
    let workspace_directory = regenerator.resolve_workspace_directory(&canonical_name)?;

    terminal_style::print_success(&format!(
        "Rebuilt workspace {} at {}",
        canonical_name.style(terminal_style::ALIAS_STYLE),
        workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
    ));
    terminal_style::write_stdout(&format!(
        "   workspace file: {}\n",
        workspace_file_path.display().to_string().style(terminal_style::PATH_STYLE),
    ));
    Ok(())
}
