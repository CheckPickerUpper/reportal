//! Removes a workspace from config without touching the on-disk file.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Unregisters the named workspace from config.
///
/// Accepts either the canonical workspace key or any declared
/// alias. Does NOT delete the on-disk `.code-workspace` file —
/// that is a destructive action the user should perform through
/// the filesystem, not as a side effect of a config mutation.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if no workspace
/// matches the name or alias, or the config I/O errors that the
/// load and save paths surface.
pub fn run_workspace_delete(alias_or_canonical: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    let canonical_name = loaded_config.resolve_workspace_canonical_name(alias_or_canonical)?;
    loaded_config.remove_workspace(&canonical_name)?;
    loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!(
        "Removed workspace {} from config (the `.code-workspace` file on disk was not deleted)",
        canonical_name.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
