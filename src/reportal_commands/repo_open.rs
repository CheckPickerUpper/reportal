//! Fuzzy-selects a repo and opens it in the configured editor.

use crate::error::ReportalError;
use crate::reportal_commands::direct_alias_router::{
    DirectAliasRouter, DirectAliasRouterOutcome,
};
use crate::reportal_commands::repo_selection::{self, SelectedRepoParams};
use crate::reportal_commands::run_workspace_open;
use crate::reportal_commands::target_selection::{
    self, SelectedTarget, SelectedTargetParams,
};
use crate::reportal_commands::terminal_identity_emit::{
    self, TerminalIdentityEmitParams,
};
use crate::reportal_config::{PathVisibility, ReportalConfig, TagFilter};
use crate::terminal_style;
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

/// Opens a repo — or a workspace — in the configured editor.
///
/// If `direct_alias` is provided, repo resolution is tried first;
/// when no repo matches and the alias resolves to a registered
/// workspace, the workspace-open flow runs instead and launches
/// the editor against the `.code-workspace` file. Unknown aliases
/// surface as `RepoNotFound` so the error names the user-facing
/// concept the shell wrapper advertises.
///
/// Without a direct alias, the fuzzy finder stays repo-only — use
/// `rep workspace list` / `row` for workspace selection.
///
/// # Errors
///
/// Returns [`ReportalError::RepoNotFound`] when the direct alias
/// resolves to neither a repo nor a workspace,
/// [`ReportalError::EditorLaunchFailure`] if the editor process
/// cannot be spawned, or any config / file I/O errors the
/// underlying paths surface.
pub fn run_open(open_params: &OpenCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let resolved_repo_alias: String = if open_params.direct_alias.is_empty() {
        let target_params = SelectedTargetParams {
            loaded_config: &loaded_config,
            tag_filter: &open_params.tag_filter,
            prompt_label: "Open repo or workspace",
        };
        match target_selection::select_target(&target_params)? {
            SelectedTarget::Repo(chosen_repo_alias) => chosen_repo_alias,
            SelectedTarget::Workspace(canonical_workspace_name) => {
                return run_workspace_open(&canonical_workspace_name);
            }
        }
    } else {
        let router = DirectAliasRouter::for_config(&loaded_config);
        match router.classify(open_params.direct_alias)? {
            DirectAliasRouterOutcome::RegisteredRepo => {
                open_params.direct_alias.to_owned()
            }
            DirectAliasRouterOutcome::Workspace(canonical_workspace_name) => {
                return run_workspace_open(&canonical_workspace_name);
            }
            DirectAliasRouterOutcome::Unknown => {
                return Err(ReportalError::RepoNotFound {
                    alias: open_params.direct_alias.to_owned(),
                });
            }
        }
    };

    let selection_params = SelectedRepoParams {
        loaded_config: &loaded_config,
        direct_alias: &resolved_repo_alias,
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
