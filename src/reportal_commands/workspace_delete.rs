//! Removes a workspace from config, optionally purging its on-disk
//! directory.

use crate::cli_args::WorkspaceArgsDeleteParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_layout::purge_workspace_directory;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Unregisters the named workspace from config, optionally purging
/// its on-disk directory.
///
/// Accepts either the canonical workspace key or any declared
/// alias. Default behavior is to remove the config entry and leave
/// the workspace directory in place — that keeps the `--purge`
/// step explicit so an accidental `delete` cannot destroy an open
/// editor session. Member repo directories are never touched, even
/// with `--purge`; only the symlinks inside the workspace directory
/// are removed.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if no workspace
/// matches the name or alias, or the config / filesystem I/O
/// errors that the load, save, and purge paths surface.
pub fn run_workspace_delete(
    delete_parts: &WorkspaceArgsDeleteParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_name =
        loaded_config.resolve_workspace_canonical_name(delete_parts.workspace_name())?;

    // Resolve the directory BEFORE removing the entry — the
    // resolver reads the workspace entry, so we need to capture
    // the path while the entry is still in the registry. If the
    // user did not pass --purge we still load the path so the
    // deletion message can point at exactly what would be removed.
    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_directory = regenerator.resolve_workspace_directory(&canonical_name)?;

    loaded_config.remove_workspace(&canonical_name)?;
    loaded_config.save_to_disk()?;

    if delete_parts.purge() {
        terminal_style::write_stdout(&format!(
            "   Purging workspace directory {} (member repos are not touched)\n",
            workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
        ));
        purge_workspace_directory(&workspace_directory)?;
        terminal_style::print_success(&format!(
            "Removed workspace {} from config and purged {}",
            canonical_name.style(terminal_style::ALIAS_STYLE),
            workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
        ));
    } else {
        terminal_style::print_success(&format!(
            "Removed workspace {} from config (directory at {} was not deleted — pass --purge to remove it)",
            canonical_name.style(terminal_style::ALIAS_STYLE),
            workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
        ));
    }
    Ok(())
}
