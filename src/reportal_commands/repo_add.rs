/// Interactively registers a new repo in the RePortal config.
/// Supports both local paths and git URLs (clones first, then registers).

use crate::error::ReportalError;
use crate::reportal_commands::prompts::{
    self, ColorPromptResult, TextPromptParams,
};
use crate::reportal_config::{RepoRegistrationBuilder, ReportalConfig};
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use owo_colors::OwoColorize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
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

/// Whether an alias could be inferred from a path or URL.
enum AliasSuggestion {
    /// A name was extracted as a suggested alias.
    Inferred(String),
    /// No usable name could be extracted.
    NoSuggestion,
}

/// Whether the input is a local path or a git URL to clone.
enum AddSource {
    /// A local directory that already exists.
    LocalPath(String),
    /// A git URL that needs cloning first.
    GitUrl(String),
}

/// Where the user wants to clone a remote repo.
enum CloneDestination {
    /// Type a custom absolute path.
    CustomPath,
    /// Clone as a sibling of an existing registered repo.
    SiblingOf(PathBuf),
    /// Clone as a child inside an existing registered repo's directory.
    ChildOf(PathBuf),
}

/// Parameters for cloning a git repo into a directory.
struct GitCloneOperation<'a> {
    /// The git URL to clone from.
    git_url: &'a str,
    /// The directory to clone into.
    target_directory: &'a Path,
}

/// All data needed to run the interactive metadata collection and registration.
struct RegistrationContext<'a> {
    /// Mutable reference to the loaded config for adding the repo.
    loaded_config: &'a mut ReportalConfig,
    /// The filesystem path where the repo lives.
    filesystem_path: &'a str,
    /// A suggested alias to pre-fill the prompt.
    suggested_alias: String,
    /// A detected or known remote URL to pre-fill the prompt.
    detected_remote: String,
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

/// Extracts the repo name from a git URL for use as default alias.
fn repo_name_from_git_url(git_url: &str) -> AliasSuggestion {
    let trimmed = git_url.trim_end_matches(".git");
    match trimmed.rsplit('/').next() {
        Some(repo_name) => match repo_name.is_empty() {
            true => match trimmed.rsplit(':').next() {
                Some(ssh_path) => match ssh_path.rsplit('/').next() {
                    Some(ssh_repo_name) => match ssh_repo_name.is_empty() {
                        true => AliasSuggestion::NoSuggestion,
                        false => AliasSuggestion::Inferred(ssh_repo_name.to_string()),
                    },
                    None => AliasSuggestion::NoSuggestion,
                },
                None => AliasSuggestion::NoSuggestion,
            },
            false => AliasSuggestion::Inferred(repo_name.to_string()),
        },
        None => AliasSuggestion::NoSuggestion,
    }
}

/// Determines if the input looks like a git URL or a local path.
fn classify_add_source(raw_input: &str) -> AddSource {
    if raw_input.starts_with("https://")
        || raw_input.starts_with("git@")
        || raw_input.starts_with("ssh://")
        || raw_input.ends_with(".git")
    {
        AddSource::GitUrl(raw_input.to_string())
    } else {
        AddSource::LocalPath(raw_input.to_string())
    }
}

/// Collects unique parent directories from all registered repos.
fn collect_registered_parent_directories(loaded_config: &ReportalConfig) -> Vec<(String, PathBuf)> {
    let all_repos = loaded_config.repos_matching_tag_filter(&crate::reportal_config::TagFilter::All);
    let mut seen_parents: BTreeSet<String> = BTreeSet::new();
    let mut labeled_directories: Vec<(String, PathBuf)> = Vec::new();

    for (alias, repo) in &all_repos {
        let resolved = repo.resolved_path();
        match resolved.parent() {
            Some(parent_dir) => {
                let parent_string = parent_dir.display().to_string();
                if !seen_parents.contains(&parent_string) {
                    seen_parents.insert(parent_string);
                    labeled_directories.push((alias.to_string(), parent_dir.to_path_buf()));
                }
            }
            None => {}
        }
    }

    labeled_directories
}

/// Collects registered repo directories that could be parents for a new clone.
fn collect_registered_repo_directories(loaded_config: &ReportalConfig) -> Vec<(String, PathBuf)> {
    let all_repos = loaded_config.repos_matching_tag_filter(&crate::reportal_config::TagFilter::All);
    all_repos
        .iter()
        .map(|(alias, repo)| (alias.to_string(), repo.resolved_path()))
        .collect()
}

/// Step 1: Ask the user what kind of placement they want.
/// Step 2: If sibling or child, show them which repo to place relative to.
fn prompt_clone_destination(loaded_config: &ReportalConfig) -> Result<CloneDestination, ReportalError> {
    let prompt_theme = ColorfulTheme::default();

    let sibling_directories = collect_registered_parent_directories(loaded_config);
    let child_directories = collect_registered_repo_directories(loaded_config);

    let mut placement_labels: Vec<String> = vec!["Custom path".to_string()];
    let mut placement_has_sibling = false;
    let mut placement_has_child = false;

    if !sibling_directories.is_empty() {
        placement_labels.push("Sibling of existing repo".to_string());
        placement_has_sibling = true;
    }
    if !child_directories.is_empty() {
        placement_labels.push("Child of existing repo".to_string());
        placement_has_child = true;
    }

    let placement_index = FuzzySelect::with_theme(&prompt_theme)
        .with_prompt("How to place this repo?")
        .items(&placement_labels)
        .interact_opt()
        .map_err(|select_error| ReportalError::ConfigIoFailure {
            reason: select_error.to_string(),
        })?;

    let chosen_placement = match placement_index {
        Some(chosen_index) => match placement_labels.get(chosen_index) {
            Some(chosen_label) => chosen_label.as_str(),
            None => return Err(ReportalError::SelectionCancelled),
        },
        None => return Err(ReportalError::SelectionCancelled),
    };

    match chosen_placement {
        "Custom path" => Ok(CloneDestination::CustomPath),
        "Sibling of existing repo" if placement_has_sibling => {
            let sibling_labels: Vec<String> = sibling_directories
                .iter()
                .map(|(near_alias, parent_path)| {
                    format!("{} ({})", near_alias, parent_path.display())
                })
                .collect();

            let sibling_index = FuzzySelect::with_theme(&prompt_theme)
                .with_prompt("Sibling of which repo?")
                .items(&sibling_labels)
                .interact_opt()
                .map_err(|select_error| ReportalError::ConfigIoFailure {
                    reason: select_error.to_string(),
                })?;

            match sibling_index {
                Some(chosen_index) => match sibling_directories.get(chosen_index) {
                    Some((_, parent_path)) => Ok(CloneDestination::SiblingOf(parent_path.to_path_buf())),
                    None => Err(ReportalError::SelectionCancelled),
                },
                None => Err(ReportalError::SelectionCancelled),
            }
        }
        "Child of existing repo" if placement_has_child => {
            let child_labels: Vec<String> = child_directories
                .iter()
                .map(|(inside_alias, repo_path)| {
                    format!("{} ({})", inside_alias, repo_path.display())
                })
                .collect();

            let child_index = FuzzySelect::with_theme(&prompt_theme)
                .with_prompt("Child of which repo?")
                .items(&child_labels)
                .interact_opt()
                .map_err(|select_error| ReportalError::ConfigIoFailure {
                    reason: select_error.to_string(),
                })?;

            match child_index {
                Some(chosen_index) => match child_directories.get(chosen_index) {
                    Some((_, repo_path)) => Ok(CloneDestination::ChildOf(repo_path.to_path_buf())),
                    None => Err(ReportalError::SelectionCancelled),
                },
                None => Err(ReportalError::SelectionCancelled),
            }
        }
        _ => Ok(CloneDestination::CustomPath),
    }
}

/// Clones a git repo into the target directory.
fn clone_repo(clone_operation: GitCloneOperation<'_>) -> Result<(), ReportalError> {
    println!(
        "  {} {}",
        "Cloning:".style(terminal_style::LABEL_STYLE),
        clone_operation.git_url.style(terminal_style::PATH_STYLE),
    );

    let clone_result = Command::new("git")
        .args(["clone", clone_operation.git_url])
        .current_dir(clone_operation.target_directory)
        .status();

    match clone_result {
        Ok(exit_status) => match exit_status.success() {
            true => Ok(()),
            false => Err(ReportalError::ConfigIoFailure {
                reason: format!("git clone exited with status {}", exit_status),
            }),
        },
        Err(git_spawn_error) => Err(ReportalError::ConfigIoFailure {
            reason: format!("Failed to run git: {}", git_spawn_error),
        }),
    }
}

/// Shared interactive flow for collecting repo metadata and saving to config.
fn collect_metadata_and_register(registration_context: RegistrationContext<'_>) -> Result<(), ReportalError> {
    let prompt_theme = ColorfulTheme::default();

    let repo_alias = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Alias",
        default_value: registration_context.suggested_alias,
    })?;

    let repo_description = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Description",
        default_value: String::new(),
    })?;

    let tags_input = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Tags (comma-separated)",
        default_value: String::new(),
    })?;

    let parsed_tags = prompts::parse_comma_separated_tags(&tags_input);

    let repo_remote = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Remote URL",
        default_value: registration_context.detected_remote,
    })?;

    let tab_title = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Tab title (empty = use alias)",
        default_value: String::new(),
    })?;

    let repo_color = prompts::prompt_for_color(&prompt_theme)?;

    println!();
    println!("  {} {}", "Alias:".style(terminal_style::LABEL_STYLE), repo_alias.style(terminal_style::ALIAS_STYLE));
    println!("  {} {}", "Path:".style(terminal_style::LABEL_STYLE), registration_context.filesystem_path.style(terminal_style::PATH_STYLE));
    if !repo_description.is_empty() {
        println!("  {} {}", "Desc:".style(terminal_style::LABEL_STYLE), repo_description);
    }
    if !parsed_tags.is_empty() {
        println!("  {} {}", "Tags:".style(terminal_style::LABEL_STYLE), parsed_tags.join(", ").style(terminal_style::TAG_STYLE));
    }
    if !repo_remote.is_empty() {
        println!("  {} {}", "Remote:".style(terminal_style::LABEL_STYLE), repo_remote.style(terminal_style::PATH_STYLE));
    }
    if !tab_title.is_empty() {
        println!("  {} {}", "Title:".style(terminal_style::LABEL_STYLE), tab_title.style(terminal_style::ALIAS_STYLE));
    }
    match &repo_color {
        ColorPromptResult::Provided(hex_color) => {
            println!("  {} {}", "Color:".style(terminal_style::LABEL_STYLE), hex_color.raw_value());
        }
        ColorPromptResult::Skipped => {}
    }
    println!();

    let display_alias = repo_alias.as_str().to_string();

    let mut builder = RepoRegistrationBuilder::start(repo_alias)
        .repo_path(registration_context.filesystem_path.to_string())
        .repo_description(repo_description)
        .repo_tags(parsed_tags)
        .repo_remote(repo_remote);

    if !tab_title.is_empty() {
        builder = builder.repo_title(tab_title);
    }
    match repo_color {
        ColorPromptResult::Provided(hex_color) => {
            builder = builder.repo_color(hex_color);
        }
        ColorPromptResult::Skipped => {}
    }

    let validated_registration = builder.build()?;

    registration_context.loaded_config.add_repo(validated_registration)?;
    registration_context.loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!("Registered '{}'", display_alias));
    Ok(())
}

/// Adds a repo to the config. Accepts local paths or git URLs.
///
/// For local paths: prompts for metadata and registers.
/// For git URLs: asks where to clone, clones, then prompts for metadata.
pub fn run_add(raw_input: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;

    match classify_add_source(raw_input) {
        AddSource::LocalPath(local_path) => {
            let suggested_alias = match suggest_alias_from_path(&local_path) {
                AliasSuggestion::Inferred(folder_alias) => folder_alias,
                AliasSuggestion::NoSuggestion => String::new(),
            };
            let detected_remote = match detect_git_remote(&local_path) {
                GitRemoteDetection::Found(remote_url) => remote_url,
                GitRemoteDetection::NoOriginConfigured => String::new(),
                GitRemoteDetection::GitUnavailable => String::new(),
            };
            collect_metadata_and_register(RegistrationContext {
                loaded_config: &mut loaded_config,
                filesystem_path: &local_path,
                suggested_alias,
                detected_remote,
            })
        }
        AddSource::GitUrl(git_url) => {
            let suggested_alias = match repo_name_from_git_url(&git_url) {
                AliasSuggestion::Inferred(repo_name) => repo_name,
                AliasSuggestion::NoSuggestion => String::new(),
            };

            println!();
            println!(
                "  {} {}",
                "From:".style(terminal_style::LABEL_STYLE),
                git_url.style(terminal_style::PATH_STYLE),
            );
            println!();

            let clone_destination = prompt_clone_destination(&loaded_config)?;
            let prompt_theme = ColorfulTheme::default();

            let clone_parent = match clone_destination {
                CloneDestination::CustomPath => {
                    let custom_path: String = Input::with_theme(&prompt_theme)
                        .with_prompt("Clone into directory")
                        .interact_text()
                        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                            reason: prompt_error.to_string(),
                        })?;
                    let expanded = shellexpand::tilde(&custom_path);
                    PathBuf::from(expanded.as_ref())
                }
                CloneDestination::SiblingOf(parent_path) => parent_path,
                CloneDestination::ChildOf(repo_path) => repo_path,
            };

            if !clone_parent.exists() {
                std::fs::create_dir_all(&clone_parent).map_err(|io_error| {
                    ReportalError::ConfigIoFailure {
                        reason: io_error.to_string(),
                    }
                })?;
            }

            clone_repo(GitCloneOperation {
                git_url: &git_url,
                target_directory: &clone_parent,
            })?;

            let cloned_repo_path = clone_parent.join(&suggested_alias);
            let final_path = cloned_repo_path.display().to_string();

            collect_metadata_and_register(RegistrationContext {
                loaded_config: &mut loaded_config,
                filesystem_path: &final_path,
                suggested_alias,
                detected_remote: git_url,
            })
        }
    }
}
