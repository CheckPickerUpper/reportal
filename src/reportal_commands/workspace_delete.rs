//! Removes a workspace from config without touching the on-disk file.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Unregisters the named workspace from config.
///
/// Does NOT delete the on-disk `.code-workspace` file. Deletion
/// would break an open editor session holding the file and is a
/// destructive action the user should perform deliberately through
/// the filesystem, not as a side effect of a config mutation.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if no workspace
/// with that name is registered, or the config I/O errors that the
/// load and save paths surface.
pub fn run_workspace_delete(workspace_name: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    loaded_config.remove_workspace(workspace_name)?;
    loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!(
        "Removed workspace {} from config (the `.code-workspace` file on disk was not deleted)",
        workspace_name.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
