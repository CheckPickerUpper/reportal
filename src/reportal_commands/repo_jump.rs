/// Fuzzy-selects a repo and prints its path for shell `cd` integration.

use crate::error::ReportalError;
use crate::reportal_config::{PathVisibility, ReportalConfig, TagFilter};
use crate::terminal_style;
use crate::reportal_commands::repo_selection::{
    self, RepoSelectionParams, TerminalIdentityEmitParams,
};
use owo_colors::OwoColorize;

/// All parameters needed to run the jump command.
pub struct JumpCommandParams<'a> {
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
/// If a direct alias is given, skips the fuzzy finder entirely.
/// The raw path always goes to stdout for the shell function;
/// an optional styled confirmation goes to stderr based on config.
/// Also emits OSC escape sequences for tab title and background color.
pub fn run_jump(jump_params: JumpCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let selected = repo_selection::select_repo(RepoSelectionParams {
        loaded_config: &loaded_config,
        direct_alias: jump_params.direct_alias,
        tag_filter: &jump_params.tag_filter,
        prompt_label: "Jump to repo",
    })?;

    repo_selection::emit_repo_terminal_identity(TerminalIdentityEmitParams {
        selected_alias: selected.repo_alias(),
        selected_repo: selected.repo_config(),
        title_override: jump_params.title_override,
    });

    let resolved_path = selected.repo_config().resolved_path();
    let formatted_path = loaded_config.path_display_format().format_path(&resolved_path);

    print!("{formatted_path}");

    match loaded_config.path_on_select() {
        PathVisibility::Show => {
            eprintln!(
                "  {} {}",
                ">>".style(terminal_style::SUCCESS_STYLE),
                formatted_path.style(terminal_style::PATH_STYLE),
            );
        }
        PathVisibility::Hide => {}
    }

    return Ok(());
}
