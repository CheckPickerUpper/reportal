//! Fuzzy-selects a repo and opens it in the configured editor.

use crate::error::ReportalError;
use crate::reportal_config::{PathVisibility, ReportalConfig, TagFilter};
use crate::terminal_style;
use crate::reportal_commands::repo_selection::{self, SelectedRepoParams};
use crate::reportal_commands::terminal_identity_emit::{
    self, TerminalIdentityEmitParams,
};
use owo_colors::OwoColorize;
use std::process::Command;

/// All parameters needed to run the open command.
pub struct OpenCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and open this alias directly.
    pub direct_alias: &'a str,
    /// If non-empty, use this editor instead of the configured default.
    pub editor_override: &'a str,
    /// If non-empty, override the tab title for this session.
    pub title_override: &'a str,
}

/// Opens a repo in the configured editor (default: cursor).
///
/// If `direct_alias` is provided, opens that repo directly without
/// prompting. Otherwise, presents a fuzzy finder for interactive selection.
/// The editor is launched by `cd`-ing into the repo directory first,
/// then running `<editor> .` so the editor opens the folder correctly.
/// Also emits OSC escape sequences for tab title and background color.
pub fn run_open(open_params: &OpenCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let selection_params = SelectedRepoParams {
        loaded_config: &loaded_config,
        direct_alias: open_params.direct_alias,
        tag_filter: &open_params.tag_filter,
        prompt_label: "Open in editor",
    };
    let selected = repo_selection::select_repo(&selection_params)?;

    terminal_identity_emit::emit_repo_terminal_identity(&TerminalIdentityEmitParams {
        selected_alias: selected.repo_alias(),
        selected_repo: selected.repo_config(),
        title_override: open_params.title_override,
    });

    let resolved_repo_path = selected.repo_config().resolved_path();

    let editor_command = if open_params.editor_override.is_empty() { loaded_config.default_editor() } else { open_params.editor_override };

    #[cfg(target_os = "windows")]
    let spawn_result = Command::new("cmd")
        .args(["/c", editor_command, "."])
        .current_dir(&resolved_repo_path)
        .spawn();

    #[cfg(not(target_os = "windows"))]
    let spawn_result = Command::new(editor_command)
        .arg(".")
        .current_dir(&resolved_repo_path)
        .spawn();

    spawn_result.map_err(|spawn_error| ReportalError::EditorLaunchFailure {
        reason: spawn_error.to_string(),
    })?;

    match loaded_config.path_on_select() {
        PathVisibility::Show => {
            let formatted_path =
                loaded_config.path_display_format().format_path(&resolved_repo_path);
            terminal_style::print_success(&format!(
                "Opened {} in {}",
                formatted_path.style(terminal_style::PATH_STYLE),
                editor_command.style(terminal_style::ALIAS_STYLE)
            ));
        }
        PathVisibility::Hide => {}
    }

    Ok(())
}
