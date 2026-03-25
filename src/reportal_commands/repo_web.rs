/// Fuzzy-selects a repo and opens its remote URL in the default browser.

use crate::error::ReportalError;
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle, TagFilter};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use owo_colors::OwoColorize;
use std::process::Command;

/// All parameters needed to run the web command.
pub struct WebCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and open this alias directly.
    pub direct_alias: &'a str,
}

/// The result of resolving a git remote URL to a browser-friendly HTTPS URL.
enum RemoteResolution {
    /// A browsable HTTPS URL was resolved.
    Resolved(String),
    /// No remote URL could be found from any source.
    NotFound,
}

/// Converts a git remote URL (SSH or HTTPS) into a browser-friendly HTTPS URL.
///
/// Handles these formats:
/// - `git@github.com:org/repo.git` → `https://github.com/org/repo.git`
/// - `ssh://git@github.com/org/repo.git` → `https://github.com/org/repo.git`
/// - `https://github.com/org/repo.git` → unchanged
///
/// Strips the trailing `.git` suffix for a cleaner browser URL.
fn remote_url_to_browser_url(remote_url: &str) -> String {
    let https_url = match remote_url.starts_with("git@") {
        true => {
            let without_prefix = &remote_url["git@".len()..];
            let normalized = without_prefix.replacen(':', "/", 1);
            format!("https://{normalized}")
        }
        false => match remote_url.starts_with("ssh://") {
            true => {
                let without_scheme = &remote_url["ssh://".len()..];
                let without_user = match without_scheme.find('@') {
                    Some(at_position) => &without_scheme[at_position + 1..],
                    None => without_scheme,
                };
                format!("https://{without_user}")
            }
            false => remote_url.to_string(),
        },
    };

    match https_url.strip_suffix(".git") {
        Some(without_git_suffix) => without_git_suffix.to_string(),
        None => https_url,
    }
}

/// Attempts to read the git origin remote URL from a repo directory.
fn detect_git_remote(repo_path: &std::path::Path) -> RemoteResolution {
    let git_output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output();

    match git_output {
        Ok(output) => match output.status.success() {
            true => {
                let trimmed = String::from_utf8_lossy(&output.stdout).trim().to_string();
                match trimmed.is_empty() {
                    true => RemoteResolution::NotFound,
                    false => RemoteResolution::Resolved(trimmed),
                }
            }
            false => RemoteResolution::NotFound,
        },
        Err(_git_error) => RemoteResolution::NotFound,
    }
}

/// Opens a repo's remote URL in the default browser.
///
/// Resolves the remote URL from the config's `remote` field first,
/// falling back to `git remote get-url origin` in the repo directory.
/// Converts SSH remotes to HTTPS URLs for browser compatibility.
pub fn run_web(web_params: WebCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let (selected_alias, selected_repo): (&str, &crate::reportal_config::RepoEntry) =
        match web_params.direct_alias.is_empty() {
            false => {
                let found_repo = loaded_config.get_repo(web_params.direct_alias)?;
                (web_params.direct_alias, found_repo)
            }
            true => {
                let matching_repos =
                    loaded_config.repos_matching_tag_filter(&web_params.tag_filter);
                if matching_repos.is_empty() {
                    return Err(ReportalError::NoReposMatchFilter);
                }

                let display_labels: Vec<String> = matching_repos
                    .iter()
                    .map(|(alias, repo)| {
                        let mut label = alias.to_string();

                        match repo.aliases().is_empty() {
                            true => {}
                            false => {
                                let aliases_joined = repo.aliases().join(", ");
                                label.push_str(&format!(" ({aliases_joined})"));
                            }
                        }

                        match repo.description().is_empty() {
                            true => {}
                            false => {
                                label.push_str(&format!(" — {}", repo.description()));
                            }
                        }

                        return label;
                    })
                    .collect();

                let selected_index = FuzzySelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Open in browser")
                    .items(&display_labels)
                    .interact_opt()
                    .map_err(|select_error| ReportalError::ConfigIoFailure {
                        reason: select_error.to_string(),
                    })?;

                match selected_index {
                    Some(chosen_index) => match matching_repos.get(chosen_index) {
                        Some((chosen_alias, chosen_repo)) => (chosen_alias.as_str(), *chosen_repo),
                        None => return Err(ReportalError::SelectionCancelled),
                    },
                    None => return Err(ReportalError::SelectionCancelled),
                }
            }
        };

    let resolved_title = match selected_repo.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_string(),
        TabTitle::UseAlias => selected_alias.to_string(),
    };

    let tab_color_action = match selected_repo.repo_color() {
        RepoColor::Themed(hex_color) => {
            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
        }
        RepoColor::ResetToDefault => TabColorAction::Reset,
    };

    let identity = TerminalIdentity::new(TerminalIdentityParams {
        resolved_title,
        tab_color_action,
    });
    terminal_style::emit_terminal_identity_to_console(&identity);

    let raw_remote_url = match selected_repo.remote().is_empty() {
        false => selected_repo.remote().to_string(),
        true => match detect_git_remote(&selected_repo.resolved_path()) {
            RemoteResolution::Resolved(detected_url) => detected_url,
            RemoteResolution::NotFound => {
                return Err(ReportalError::NoRemoteUrl {
                    alias: selected_alias.to_string(),
                });
            }
        },
    };

    let browser_url = remote_url_to_browser_url(&raw_remote_url);

    open::that(&browser_url).map_err(|open_error| ReportalError::BrowserLaunchFailure {
        reason: open_error.to_string(),
    })?;

    terminal_style::print_success(&format!(
        "Opened {}",
        browser_url.style(terminal_style::PATH_STYLE),
    ));

    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_ssh_remote_to_https() {
        assert_eq!(
            remote_url_to_browser_url("git@github.com:org/repo.git"),
            "https://github.com/org/repo"
        );
    }

    #[test]
    fn converts_ssh_scheme_remote_to_https() {
        assert_eq!(
            remote_url_to_browser_url("ssh://git@github.com/org/repo.git"),
            "https://github.com/org/repo"
        );
    }

    #[test]
    fn passes_through_https_remote() {
        assert_eq!(
            remote_url_to_browser_url("https://github.com/org/repo.git"),
            "https://github.com/org/repo"
        );
    }

    #[test]
    fn handles_https_without_git_suffix() {
        assert_eq!(
            remote_url_to_browser_url("https://github.com/org/repo"),
            "https://github.com/org/repo"
        );
    }
}
