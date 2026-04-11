//! Shared path display helper for commands that print after repo selection.

use crate::reportal_config::{PathVisibility, ReportalConfig};
use crate::terminal_style;
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// Parameters for conditionally printing the selected repo's path.
pub struct SelectedPathDisplayParams<'a> {
    /// The loaded config (provides `path_on_select` and `path_display_format`).
    pub loaded_config: &'a ReportalConfig,
    /// The resolved filesystem path of the selected repo.
    pub resolved_path: &'a PathBuf,
}

/// Prints the selected repo's path to stderr if the config says to show it.
///
/// Used by jump and open after repo selection to give the user visual
/// feedback about which path was resolved. Respects the `path_on_select`
/// and `path_display_format` settings.
pub fn print_selected_path_if_visible(display_params: &SelectedPathDisplayParams<'_>) {
    match display_params.loaded_config.path_on_select() {
        PathVisibility::Show => {
            let formatted_path = display_params
                .loaded_config
                .path_display_format()
                .format_path(display_params.resolved_path);
            terminal_style::write_stderr(&format!(
                "  {} {}\n",
                ">>".style(terminal_style::SUCCESS_STYLE),
                formatted_path.style(terminal_style::PATH_STYLE),
            ));
        }
        PathVisibility::Hide => {}
    }
}
