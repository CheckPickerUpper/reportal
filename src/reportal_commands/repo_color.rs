//! Emits OSC terminal personalization sequences for the current or specified repo.
//!
//! Used as a shell prompt hook so terminals that open directly into a repo
//! (e.g. VS Code integrated terminal) get the right tab title and background
//! color without going through `rj`.

use crate::error::ReportalError;
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};

use super::color_command_mode::ColorCommandMode;

/// Parameters for the color subcommand.
pub struct ColorCommandParams<'a> {
    /// If non-empty, look up this alias directly instead of matching PWD.
    pub repo_alias: &'a str,
    /// How the command was invoked, controlling stdout and no-match behavior.
    pub mode: ColorCommandMode,
}

/// Resolves the tab title from a repo's config, falling back to the alias.
fn resolve_tab_title(tab_title: &TabTitle, fallback_alias: &str) -> String {
    match tab_title {
        TabTitle::Custom(custom_title) => custom_title.to_owned(),
        TabTitle::UseAlias => fallback_alias.to_owned(),
    }
}

/// Converts a repo's color config into the appropriate tab color action.
fn resolve_tab_color(repo_color: &RepoColor) -> TabColorAction {
    match repo_color {
        RepoColor::Themed(hex_color) => TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence()),
        RepoColor::ResetToDefault => TabColorAction::Reset,
    }
}

/// Finds the best-matching repo for the current working directory using
/// longest-prefix-match against all registered repo paths.
fn resolve_from_working_directory(loaded_config: &ReportalConfig) -> Result<Option<(String, TabColorAction)>, ReportalError> {
    let current_directory = std::env::current_dir().map_err(|io_error| {
        ReportalError::ConfigIoFailure {
            reason: io_error.to_string(),
        }
    })?;

    let all_repos = loaded_config.repos_with_aliases();
    let mut best_match_length: usize = 0;
    let mut best_alias: &str = "";
    let mut best_repo: Option<&crate::reportal_config::RepoEntry> = None;

    for (alias, repo) in &all_repos {
        let repo_path = repo.resolved_path();
        if !current_directory.starts_with(&repo_path) {
            continue;
        }
        let path_length = repo_path.as_os_str().len();
        if path_length > best_match_length {
            best_match_length = path_length;
            best_alias = alias.as_str();
            best_repo = Some(repo);
        }
    }

    Ok(best_repo.map(|matched_repo| {
        let title = resolve_tab_title(matched_repo.tab_title(), best_alias);
        let tab_color_action = resolve_tab_color(matched_repo.repo_color());
        (title, tab_color_action)
    }))
}

/// Emits OSC sequences for a repo's terminal identity.
///
/// If `repo_alias` is provided, looks up that repo directly.
/// Otherwise, matches the current working directory against all
/// registered repo paths using longest-prefix-match.
///
/// When no repo matches, behavior depends on `mode`:
/// - `Explicit` (direct call): resets the tab color to terminal default.
/// - `PromptHook` (shell prompt): does nothing, preserving the
///   color and title set by the last `rj` or `ro` invocation.
///
/// When `mode` is `PromptHook`, also writes the resolved title text
/// to stdout so shell integrations can set it via the native API
/// (e.g. `$Host.UI.RawUI.WindowTitle` in `PowerShell`).
pub fn run_color(color_params: &ColorCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;

    let resolved = if color_params.repo_alias.is_empty() {
        resolve_from_working_directory(&loaded_config)?
    } else {
        let found_repo = loaded_config.get_repo(color_params.repo_alias)?;
        let title = resolve_tab_title(found_repo.tab_title(), color_params.repo_alias);
        let tab_color_action = resolve_tab_color(found_repo.repo_color());
        Some((title, tab_color_action))
    };

    match resolved {
        Some((title, tab_color_action)) => {
            match color_params.mode {
                ColorCommandMode::PromptHook => terminal_style::write_stdout(&title.clone()),
                ColorCommandMode::Explicit => {}
            }

            let terminal_identity = TerminalIdentity::new(TerminalIdentityParams {
                resolved_title: title,
                tab_color_action,
            });
            terminal_style::emit_terminal_identity_to_console(&terminal_identity);
        }
        None => match color_params.mode {
            ColorCommandMode::Explicit => {
                terminal_style::write_to_console(terminal_style::osc_reset_tab_color_sequence());
            }
            ColorCommandMode::PromptHook => {}
        },
    }

    Ok(())
}
