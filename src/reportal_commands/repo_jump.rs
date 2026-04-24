//! Fuzzy-selects a repo and prints its path for shell `cd` integration.

use crate::error::ReportalError;
use crate::reportal_commands::direct_alias_router::{
    DirectAliasRouter, DirectAliasRouterOutcome,
};
use crate::reportal_commands::path_display::{self, SelectedPathDisplayParameters};
use crate::reportal_commands::repo_selection::{self, SelectedRepoParameters};
use crate::reportal_commands::target_selection::{
    self, SelectedTarget, SelectedTargetParameters,
};
use crate::reportal_commands::terminal_identity_emit::{
    self, TerminalIdentityEmitParameters,
};
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;

/// All parameters needed to run the jump command.
pub struct JumpCommandParameters<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and jump directly.
    pub direct_alias: &'a str,
    /// If non-empty, override the tab title for this session.
    pub title_override: &'a str,
}

/// Prints the selected repo's resolved path to stdout; the shell
/// wrapper function (`rj`) reads this and runs `cd`.
///
/// If a direct alias is given, repo resolution is tried first. When
/// that fails and the alias matches a registered workspace, falls
/// through to the workspace's `.code-workspace` file parent directory
/// so `rj venoble` for a workspace-only setup cd's to the common
/// ancestor folder instead of erroring out. Unknown aliases still
/// surface as `RepoNotFound` because that is the user-facing name
/// the shell wrapper promises.
///
/// The raw path always goes to stdout for the shell function; an
/// optional styled confirmation goes to stderr based on config. OSC
/// tab/color escape sequences are emitted only for repo jumps —
/// workspace jumps skip them because a workspace has no single
/// per-repo identity to apply.
///
/// # Errors
///
/// Returns [`ReportalError::RepoNotFound`] when the direct alias
/// resolves to neither a repo nor a workspace, or any error the
/// repo-selection or workspace-regenerator paths surface.
pub fn run_jump(jump_params: &JumpCommandParameters<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;

    let resolved_repo_alias: String = if jump_params.direct_alias.is_empty() {
        let target_params = SelectedTargetParameters {
            loaded_config: &loaded_config,
            tag_filter: &jump_params.tag_filter,
            prompt_label: "Jump to repo or workspace",
        };
        match target_selection::select_target(&target_params)? {
            SelectedTarget::Repo(chosen_repo_alias) => chosen_repo_alias,
            SelectedTarget::Workspace(canonical_workspace_name) => {
                let router = DirectAliasRouter::for_config(&loaded_config);
                return router.jump_to_workspace_parent(&canonical_workspace_name);
            }
        }
    } else {
        let router = DirectAliasRouter::for_config(&loaded_config);
        match router.classify(jump_params.direct_alias)? {
            DirectAliasRouterOutcome::RegisteredRepo => {
                jump_params.direct_alias.to_owned()
            }
            DirectAliasRouterOutcome::Workspace(canonical_workspace_name) => {
                return router.jump_to_workspace_parent(&canonical_workspace_name);
            }
            DirectAliasRouterOutcome::Unknown => {
                return Err(ReportalError::RepoNotFound {
                    alias: jump_params.direct_alias.to_owned(),
                });
            }
        }
    };

    let selection_params = SelectedRepoParameters {
        loaded_config: &loaded_config,
        direct_alias: &resolved_repo_alias,
        tag_filter: &jump_params.tag_filter,
        prompt_label: "Jump to repo",
    };
    let selected = repo_selection::select_repo(&selection_params)?;

    terminal_identity_emit::emit_repo_terminal_identity(&TerminalIdentityEmitParameters {
        selected_alias: selected.repository_alias(),
        selected_repo: selected.repo_config(),
        title_override: jump_params.title_override,
    });

    let resolved_path = selected.repo_config().resolved_path();
    let formatted_path = loaded_config.path_display_format().format_path(&resolved_path);

    terminal_style::write_stdout(&formatted_path.clone());

    path_display::print_selected_path_if_visible(&SelectedPathDisplayParameters {
        loaded_config: &loaded_config,
        resolved_path: &resolved_path,
    });

    Ok(())
}
