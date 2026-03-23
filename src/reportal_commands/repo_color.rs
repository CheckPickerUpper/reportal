/// Emits OSC terminal personalization sequences for the current or specified repo.
///
/// Used as a shell prompt hook so terminals that open directly into a repo
/// (e.g. VS Code integrated terminal) get the right tab title and background
/// color without going through `rj`.

use crate::error::ReportalError;
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};

/// Whether to print the resolved title to stdout for shell integration.
pub enum TitleOutput {
    /// Print the resolved title text to stdout.
    PrintToStdout,
    /// Do not print the title to stdout (OSC-only).
    Silent,
}

/// Parameters for the color subcommand.
pub struct ColorCommandParams<'a> {
    /// If non-empty, look up this alias directly instead of matching PWD.
    pub repo_alias: &'a str,
    /// Whether to print the resolved tab title to stdout.
    pub title_output: TitleOutput,
}

/// Resolved repo identity for a single color invocation.
struct ResolvedIdentity {
    title: String,
    tab_color_action: TabColorAction,
}

/// Parameters for resolving a repo's terminal identity.
struct ResolveIdentityParams<'a> {
    /// The repo entry to resolve identity for.
    repo: &'a crate::reportal_config::RepoEntry,
    /// The alias to use as fallback when no custom title is configured.
    fallback_alias: &'a str,
}

/// Resolves the tab title and color action for a repo entry.
fn resolve_identity(identity_params: ResolveIdentityParams<'_>) -> ResolvedIdentity {
    let title = match identity_params.repo.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_string(),
        TabTitle::UseAlias => identity_params.fallback_alias.to_string(),
    };
    let tab_color_action = match identity_params.repo.repo_color() {
        RepoColor::Themed(hex_color) => {
            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
        }
        RepoColor::ResetToDefault => TabColorAction::Reset,
    };
    return ResolvedIdentity { title, tab_color_action };
}

/// Emits OSC sequences for a repo's terminal identity.
///
/// If `repo_alias` is provided, looks up that repo directly.
/// Otherwise, matches the current working directory against all
/// registered repo paths using longest-prefix-match. If no repo
/// matches, emits a reset sequence to restore the terminal default.
///
/// When `title_output` is `PrintToStdout`, also writes the resolved
/// title text to stdout so shell integrations can set it via the
/// native API (e.g. `$Host.UI.RawUI.WindowTitle` in PowerShell).
pub fn run_color(color_params: ColorCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let resolved = match color_params.repo_alias.is_empty() {
        false => {
            let found_repo = loaded_config.get_repo(color_params.repo_alias)?;
            Some(resolve_identity(ResolveIdentityParams {
                repo: found_repo,
                fallback_alias: color_params.repo_alias,
            }))
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
                Some(matched_repo) => Some(resolve_identity(ResolveIdentityParams {
                    repo: matched_repo,
                    fallback_alias: best_alias,
                })),
                None => None,
            }
        }
    };

    match resolved {
        Some(identity) => {
            let terminal_identity = TerminalIdentity::new(TerminalIdentityParams {
                resolved_title: identity.title.as_str().to_string(),
                tab_color_action: identity.tab_color_action,
            });
            terminal_style::emit_terminal_identity_to_console(&terminal_identity);

            match color_params.title_output {
                TitleOutput::PrintToStdout => print!("{}", identity.title),
                TitleOutput::Silent => {}
            }
        }
        None => {
            terminal_style::write_to_console(terminal_style::osc_reset_tab_color_sequence());
        }
    }

    return Ok(());
}
