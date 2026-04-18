//! Opens a workspace's `.code-workspace` file in the default editor.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_commands::workspace_selection;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::process::Command;

/// Regenerates the workspace's `.code-workspace` file and launches
/// the configured default editor against it.
///
/// Accepts either the canonical workspace key or any declared
/// alias; an empty string presents a workspace fuzzy finder. Every
/// path resolves to a canonical name first so the regenerator's
/// default file location uses the canonical name rather than the
/// user's short input.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if a non-empty
/// name or alias is unknown,
/// [`ReportalError::NoWorkspacesConfigured`] when the fuzzy
/// finder has nothing to show,
/// [`ReportalError::SelectionCancelled`] if the user escapes the
/// prompt, [`ReportalError::RepoNotFound`] if any member alias
/// does not resolve, [`ReportalError::EditorLaunchFailure`] if
/// the editor process cannot be spawned, or the config / file I/O
/// errors the regeneration path surfaces.
pub fn run_workspace_open(alias_or_canonical: &str) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_name = if alias_or_canonical.is_empty() {
        workspace_selection::select_workspace(&loaded_config)?
    } else {
        loaded_config.resolve_workspace_canonical_name(alias_or_canonical)?
    };

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_file_path = regenerator.regenerate_workspace_file(&canonical_name)?;

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
        canonical_name.style(terminal_style::ALIAS_STYLE),
        editor_command.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
