//! Fuzzy-select helper that returns the canonical name of a
//! registered workspace.
//!
//! Used by `rep workspace jump` and `rep workspace open` when the
//! user invokes them without a direct workspace name, mirroring
//! the no-arg fuzzy behavior of `rep jump` and `rep open` on the
//! repo side.

use crate::error::ReportalError;
use crate::reportal_config::{HasAliases, ReportalConfig};
use crate::terminal_style;
use dialoguer::FuzzySelect;

/// Presents a fuzzy finder over every registered workspace and
/// returns the canonical name of the one the user chose.
///
/// Refuses to open the finder when zero workspaces are registered
/// so the user gets an explicit `NoWorkspacesConfigured` error
/// instead of an empty prompt that only offers cancellation.
///
/// # Errors
///
/// Returns [`ReportalError::NoWorkspacesConfigured`] when the
/// workspace registry is empty, or
/// [`ReportalError::SelectionCancelled`] if the user escapes out
/// of the prompt. Fuzzy I/O failures surface as
/// [`ReportalError::ConfigIoFailure`].
pub fn select_workspace(loaded_config: &ReportalConfig) -> Result<String, ReportalError> {
    let registered_workspaces = loaded_config.workspaces_with_names();
    if registered_workspaces.is_empty() {
        return Err(ReportalError::NoWorkspacesConfigured);
    }

    let display_labels: Vec<String> = registered_workspaces
        .iter()
        .map(|(workspace_name, workspace_entry)| {
            let uppercase_name = workspace_name.to_uppercase();
            let mut label = format!("▣▣ {uppercase_name}");
            let declared_aliases = workspace_entry.aliases();
            if !declared_aliases.is_empty() {
                let aliases_joined = declared_aliases.join(", ");
                label = format!("{label} ({aliases_joined})");
            }
            if !workspace_entry.description().is_empty() {
                label = format!("{label} — {}", workspace_entry.description());
            }
            format!("{label} [workspace]")
        })
        .collect();

    let prompt_theme = terminal_style::reportal_prompt_theme();
    let selected_index = FuzzySelect::with_theme(&prompt_theme)
        .with_prompt("Jump to workspace")
        .items(&display_labels)
        .highlight_matches(false)
        .interact_opt()
        .map_err(|select_error| ReportalError::ConfigIoFailure {
            reason: select_error.to_string(),
        })?;

    let chosen_index = match selected_index {
        Some(index) => index,
        None => return Err(ReportalError::SelectionCancelled),
    };
    match registered_workspaces.get(chosen_index) {
        Some((chosen_workspace_name, _chosen_entry)) => {
            Ok(chosen_workspace_name.to_string())
        }
        None => Err(ReportalError::SelectionCancelled),
    }
}
