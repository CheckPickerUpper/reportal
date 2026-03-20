/// Emits OSC terminal personalization sequences for the current or specified repo.
///
/// Used as a shell prompt hook so terminals that open directly into a repo
/// (e.g. VS Code integrated terminal) get the right tab title and background
/// color without going through `rj`.

use crate::error::ReportalError;
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle};
use crate::terminal_style::{self, BackgroundAction, TerminalIdentity, TerminalIdentityParams};

/// Parameters for the color subcommand.
pub struct ColorCommandParams<'a> {
    /// If non-empty, look up this alias directly instead of matching PWD.
    pub repo_alias: &'a str,
}

/// Emits OSC sequences for a repo's terminal identity.
///
/// If `repo_alias` is provided, looks up that repo directly.
/// Otherwise, matches the current working directory against all
/// registered repo paths using longest-prefix-match. If no repo
/// matches, emits a reset sequence to restore the terminal default.
pub fn run_color(color_params: ColorCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    match color_params.repo_alias.is_empty() {
        false => {
            let found_repo = loaded_config.get_repo(color_params.repo_alias)?;
            let resolved_title = match found_repo.tab_title() {
                TabTitle::Custom(custom_title) => custom_title.to_string(),
                TabTitle::UseAlias => color_params.repo_alias.to_string(),
            };
            let background_action = match found_repo.repo_color() {
                RepoColor::Themed(hex_color) => {
                    BackgroundAction::SetColor(hex_color.as_osc_background_sequence())
                }
                RepoColor::ResetToDefault => BackgroundAction::Reset,
            };
            let identity = TerminalIdentity::new(TerminalIdentityParams {
                resolved_title,
                background_action,
            });
            terminal_style::emit_terminal_identity_to_stdout(&identity);
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
                    let resolved_title = match matched_repo.tab_title() {
                        TabTitle::Custom(custom_title) => custom_title.to_string(),
                        TabTitle::UseAlias => best_alias.to_string(),
                    };
                    let background_action = match matched_repo.repo_color() {
                        RepoColor::Themed(hex_color) => {
                            BackgroundAction::SetColor(hex_color.as_osc_background_sequence())
                        }
                        RepoColor::ResetToDefault => BackgroundAction::Reset,
                    };
                    let identity = TerminalIdentity::new(TerminalIdentityParams {
                        resolved_title,
                        background_action,
                    });
                    terminal_style::emit_terminal_identity_to_stdout(&identity);
                }
                None => {
                    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                        print!("{}", terminal_style::osc_reset_background_sequence());
                    }
                }
            }
        }
    }

    return Ok(());
}
