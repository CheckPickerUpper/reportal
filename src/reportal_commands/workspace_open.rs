//! Opens a workspace's `.code-workspace` file in the default editor.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::process::Command;

/// Regenerates the workspace's `.code-workspace` file and launches
/// the configured default editor against it.
///
/// The regeneration step runs unconditionally so the file the
/// editor loads reflects the current member repo paths, which is
/// required for the invariant that opening a workspace always
/// produces a window grouping the live paths rather than whatever
/// was written the last time `create` or a membership edit ran.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the name is
/// unknown, [`ReportalError::RepoNotFound`] if any member alias
/// does not resolve, [`ReportalError::EditorLaunchFailure`] if the
/// editor process cannot be spawned, or the config / file I/O
/// errors the regeneration path surfaces.
pub fn run_workspace_open(workspace_name: &str) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;
    loaded_config.get_workspace(workspace_name)?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_file_path = regenerator.regenerate_workspace_file(workspace_name)?;

    let editor_command = loaded_config.default_editor();

    #[cfg(target_os = "windows")]
    let spawn_result = Command::new("cmd")
        .args(["/c", editor_command])
        .arg(&workspace_file_path)
        .spawn();

    #[cfg(not(target_os = "windows"))]
    let spawn_result = Command::new(editor_command)
        .arg(&workspace_file_path)
        .spawn();

    spawn_result.map_err(|spawn_error| ReportalError::EditorLaunchFailure {
        reason: spawn_error.to_string(),
    })?;

    terminal_style::print_success(&format!(
        "Opened workspace {} in {}",
        workspace_name.style(terminal_style::ALIAS_STYLE),
        editor_command.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
