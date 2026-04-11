//! Fuzzy-selects a repo and opens its remote URL in the default browser.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use crate::reportal_commands::repo_selection::{self, SelectedRepoParams};
use crate::reportal_commands::terminal_identity_emit::{
    self, TerminalIdentityEmitParams,
};
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
/// - `git@github.com:org/repo.git` → `https://github.com/org/repo`
/// - `ssh://git@github.com/org/repo.git` → `https://github.com/org/repo`
/// - `https://github.com/org/repo.git` → `https://github.com/org/repo`
///
/// Strips the trailing `.git` suffix for a cleaner browser URL.
fn remote_url_to_browser_url(remote_url: &str) -> String {
    let https_url = remote_url.strip_prefix("git@").map_or_else(
        || remote_url.strip_prefix("ssh://").map_or_else(
            || remote_url.to_owned(),
            |without_scheme| {
                let without_user = without_scheme.find('@')
                    .map_or(without_scheme, |at_position| &without_scheme[at_position + 1..]);
                format!("https://{without_user}")
            },
        ),
        |without_prefix| {
            let normalized = without_prefix.replacen(':', "/", 1);
            format!("https://{normalized}")
        },
    );

    if std::path::Path::new(&https_url)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("git"))
    {
        https_url[..https_url.len() - 4].to_owned()
    } else {
        https_url
    }
}

/// Attempts to read the git origin remote URL from a repo directory.
fn detect_git_remote(repo_path: &std::path::Path) -> RemoteResolution {
    let git_output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output();

    match git_output {
        Ok(output) => if output.status.success() {
            let trimmed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if trimmed.is_empty() { RemoteResolution::NotFound } else { RemoteResolution::Resolved(trimmed) }
        } else { RemoteResolution::NotFound },
        Err(_git_error) => RemoteResolution::NotFound,
    }
}

/// Opens a repo's remote URL in the default browser.
///
/// Resolves the remote URL from the config's `remote` field first,
/// falling back to `git remote get-url origin` in the repo directory.
/// Converts SSH remotes to HTTPS URLs for browser compatibility.
/// Emits tab title and color before opening the browser.
pub fn run_web(web_params: &WebCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let selection_params = SelectedRepoParams {
        loaded_config: &loaded_config,
        direct_alias: web_params.direct_alias,
        tag_filter: &web_params.tag_filter,
        prompt_label: "Open in browser",
    };
    let selected = repo_selection::select_repo(&selection_params)?;

    terminal_identity_emit::emit_repo_terminal_identity(&TerminalIdentityEmitParams {
        selected_alias: selected.repo_alias(),
        selected_repo: selected.repo_config(),
        title_override: "",
    });

    let raw_remote_url = if selected.repo_config().remote().is_empty() { match detect_git_remote(&selected.repo_config().resolved_path()) {
        RemoteResolution::Resolved(detected_url) => detected_url,
        RemoteResolution::NotFound => {
            return Err(ReportalError::NoRemoteUrl {
                alias: selected.repo_alias().to_owned(),
            });
        }
    } } else { selected.repo_config().remote().to_owned() };

    let browser_url = remote_url_to_browser_url(&raw_remote_url);

    open::that(&browser_url).map_err(|open_error| ReportalError::BrowserLaunchFailure {
        reason: open_error.to_string(),
    })?;

    terminal_style::print_success(&format!(
        "Opened {}",
        browser_url.style(terminal_style::PATH_STYLE),
    ));

    Ok(())
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
