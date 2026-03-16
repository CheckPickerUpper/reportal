/// Interactively registers a new repo in the RePortal config.

use crate::error::ReportalError;
use crate::reportal_config::{RepoRegistrationBuilder, ReportalConfig};
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Input};
use owo_colors::OwoColorize;
use std::path::Path;
use std::process::Command;

/// Whether a git remote was found in the target directory.
enum GitRemoteDetection {
    /// A remote URL was successfully read from git.
    Found(String),
    /// The directory has no configured origin remote.
    NoOriginConfigured,
    /// Git command failed to execute (git not installed or not a repo).
    GitUnavailable,
}

/// Whether an alias could be inferred from the directory path.
enum AliasSuggestion {
    /// The folder name was extracted as a suggested alias.
    Inferred(String),
    /// The path had no usable folder name.
    NoSuggestion,
}

/// Detects the git remote URL by running `git remote get-url origin` in the directory.
fn detect_git_remote(directory_path: &str) -> GitRemoteDetection {
    let expanded_directory = shellexpand::tilde(directory_path);
    let detection_result = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(expanded_directory.as_ref())
        .output();

    match detection_result {
        Ok(command_output) => match command_output.status.success() {
            true => {
                let remote_url = String::from_utf8_lossy(&command_output.stdout).trim().to_string();
                match remote_url.is_empty() {
                    true => GitRemoteDetection::NoOriginConfigured,
                    false => GitRemoteDetection::Found(remote_url),
                }
            }
            false => GitRemoteDetection::NoOriginConfigured,
        },
        Err(git_spawn_error) => {
            eprintln!("  git not available: {}", git_spawn_error);
            GitRemoteDetection::GitUnavailable
        }
    }
}

/// Extracts a suggested alias from the last segment of a filesystem path.
fn suggest_alias_from_path(directory_path: &str) -> AliasSuggestion {
    match Path::new(directory_path).file_name() {
        Some(folder_name) => AliasSuggestion::Inferred(folder_name.to_string_lossy().to_string()),
        None => AliasSuggestion::NoSuggestion,
    }
}

/// Prompts the user for repo details, validates them, and saves to config.
///
/// Auto-detects git remote and suggests alias from folder name.
/// Shows a confirmation summary before saving.
pub fn run_add(filesystem_path: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    let prompt_theme = ColorfulTheme::default();

    let suggested_alias = match suggest_alias_from_path(filesystem_path) {
        AliasSuggestion::Inferred(folder_alias) => folder_alias,
        AliasSuggestion::NoSuggestion => String::new(),
    };

    let detected_remote = match detect_git_remote(filesystem_path) {
        GitRemoteDetection::Found(remote_url) => remote_url,
        GitRemoteDetection::NoOriginConfigured => String::new(),
        GitRemoteDetection::GitUnavailable => String::new(),
    };

    let repo_alias: String = Input::with_theme(&prompt_theme)
        .with_prompt("Alias")
        .default(suggested_alias)
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let repo_description: String = Input::with_theme(&prompt_theme)
        .with_prompt("Description")
        .default(String::new())
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let tags_input: String = Input::with_theme(&prompt_theme)
        .with_prompt("Tags (comma-separated)")
        .default(String::new())
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let parsed_tags: Vec<String> = tags_input
        .split(',')
        .map(|tag_segment| tag_segment.trim().to_string())
        .filter(|trimmed_tag| !trimmed_tag.is_empty())
        .collect();

    let repo_remote: String = Input::with_theme(&prompt_theme)
        .with_prompt("Remote URL")
        .default(detected_remote)
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    println!();
    println!("  {} {}", "Alias:".style(terminal_style::LABEL_STYLE), repo_alias.style(terminal_style::ALIAS_STYLE));
    println!("  {} {}", "Path:".style(terminal_style::LABEL_STYLE), filesystem_path.style(terminal_style::PATH_STYLE));
    if !repo_description.is_empty() {
        println!("  {} {}", "Desc:".style(terminal_style::LABEL_STYLE), repo_description);
    }
    if !parsed_tags.is_empty() {
        println!("  {} {}", "Tags:".style(terminal_style::LABEL_STYLE), parsed_tags.join(", ").style(terminal_style::TAG_STYLE));
    }
    if !repo_remote.is_empty() {
        println!("  {} {}", "Remote:".style(terminal_style::LABEL_STYLE), repo_remote.style(terminal_style::PATH_STYLE));
    }
    println!();

    let display_alias = repo_alias.as_str().to_string();

    let validated_registration = RepoRegistrationBuilder::start(repo_alias)
        .repo_path(filesystem_path.to_string())
        .repo_description(repo_description)
        .repo_tags(parsed_tags)
        .repo_remote(repo_remote)
        .build()?;

    loaded_config.add_repo(validated_registration)?;
    loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!("Registered '{}'", display_alias));
    Ok(())
}
