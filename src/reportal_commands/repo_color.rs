/// Emits OSC terminal personalization sequences for the current or specified repo.
///
/// Used as a shell prompt hook so terminals that open directly into a repo
/// (e.g. VS Code integrated terminal) get the right tab title and background
/// color without going through `rj`.

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
/// (e.g. `$Host.UI.RawUI.WindowTitle` in PowerShell).
pub fn run_color(color_params: ColorCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let resolved = match color_params.repo_alias.is_empty() {
        false => {
            let found_repo = loaded_config.get_repo(color_params.repo_alias)?;
            let title = match found_repo.tab_title() {
                TabTitle::Custom(custom_title) => custom_title.to_string(),
                TabTitle::UseAlias => color_params.repo_alias.to_string(),
            };
            let tab_color_action = match found_repo.repo_color() {
                RepoColor::Themed(hex_color) => {
                    TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
                }
                RepoColor::ResetToDefault => TabColorAction::Reset,
            };
            Some((title, tab_color_action))
        }
        true => {
            let current_directory = std::env::current_dir().map_err(|io_error| {
                ReportalError::ConfigIoFailure {
                    reason: io_error.to_string(),
                }
            })?;

            let all_repos = loaded_config.repos_with_aliases();
            let mut best_match_length: usize = 0;
            let mut best_alias: &str = "";
            let mut best_repo: std::option::Option<&crate::reportal_config::RepoEntry> = None;

            for (alias, repo) in &all_repos {
                let repo_path = repo.resolved_path();
                match current_directory.starts_with(&repo_path) {
                    true => {
                        let path_length = repo_path.as_os_str().len();
                        match path_length > best_match_length {
                            true => {
                                best_match_length = path_length;
                                best_alias = alias.as_str();
                                best_repo = Some(repo);
                            }
                            false => {}
                        }
                    }
                    false => {}
                }
            }

            match best_repo {
                Some(matched_repo) => {
                    let title = match matched_repo.tab_title() {
                        TabTitle::Custom(custom_title) => custom_title.to_string(),
                        TabTitle::UseAlias => best_alias.to_string(),
                    };
                    let tab_color_action = match matched_repo.repo_color() {
                        RepoColor::Themed(hex_color) => {
                            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
                        }
                        RepoColor::ResetToDefault => TabColorAction::Reset,
                    };
                    Some((title, tab_color_action))
                }
                None => None,
            }
        }
    };

    match resolved {
        Some((title, tab_color_action)) => {
            match color_params.mode {
                ColorCommandMode::PromptHook => print!("{}", &title),
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

    return Ok(());
}
