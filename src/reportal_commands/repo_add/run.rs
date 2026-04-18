//! Entry point for the `rep add` command.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::Input;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use super::add_source::{classify_add_source, AddSource};
use super::alias_suggestion::{repo_name_from_git_url, suggest_alias_from_path, AliasSuggestion};
use super::clone_destination::{prompt_clone_destination, CloneDestination};
use super::git_clone_operation::GitCloneOperation;
use super::git_remote_detection::{detect_git_remote, GitRemoteDetection};
use super::registration_context::RegistrationContext;

/// Prompts the user for a custom directory path to clone into.
fn prompt_custom_clone_path(prompt_theme: &dialoguer::theme::ColorfulTheme) -> Result<PathBuf, ReportalError> {
    let custom_path: String = Input::with_theme(prompt_theme)
        .with_prompt("Clone into directory")
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;
    let expanded = shellexpand::tilde(&custom_path);
    Ok(PathBuf::from(expanded.as_ref()))
}

/// Registers a local directory as a repo in the config.
fn add_local_repo(loaded_config: &mut ReportalConfig, local_path: &str) -> Result<(), ReportalError> {
    let suggested_alias = match suggest_alias_from_path(local_path) {
        AliasSuggestion::Inferred(folder_alias) => folder_alias,
        AliasSuggestion::NoSuggestion => String::new(),
    };
    let detected_remote = match detect_git_remote(local_path) {
        GitRemoteDetection::Found(remote_url) => remote_url,
        GitRemoteDetection::NoOriginConfigured | GitRemoteDetection::GitUnavailable => String::new(),
    };
    let registration = RegistrationContext {
        loaded_config,
        filesystem_path: local_path,
        suggested_alias,
        detected_remote,
    };
    registration.collect_metadata_and_register()
}

/// Clones a git URL to a chosen destination, then registers it.
fn add_remote_repo(loaded_config: &mut ReportalConfig, git_url: &str) -> Result<(), ReportalError> {
    let suggested_alias = match repo_name_from_git_url(git_url) {
        AliasSuggestion::Inferred(repo_name) => repo_name,
        AliasSuggestion::NoSuggestion => String::new(),
    };

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {} {}\n",
        "From:".style(terminal_style::LABEL_STYLE),
        git_url.style(terminal_style::PATH_STYLE),
    ));
    terminal_style::write_stdout("\n");

    let clone_destination = prompt_clone_destination(loaded_config)?;
    let prompt_theme = terminal_style::reportal_prompt_theme();

    let clone_parent = match clone_destination {
        CloneDestination::SiblingOf(parent_path) => parent_path,
        CloneDestination::ChildOf(repo_path) => repo_path,
        CloneDestination::CustomPath => prompt_custom_clone_path(&prompt_theme)?,
    };

    if !clone_parent.exists() {
        std::fs::create_dir_all(&clone_parent).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;
    }

    let clone_operation = GitCloneOperation {
        git_url,
        target_directory: &clone_parent,
    };
    clone_operation.run_git_clone()?;

    let cloned_repo_path = clone_parent.join(&suggested_alias);
    let final_repo_path = cloned_repo_path.display().to_string();

    let registration = RegistrationContext {
        loaded_config,
        filesystem_path: &final_repo_path,
        suggested_alias,
        detected_remote: git_url.to_owned(),
    };
    registration.collect_metadata_and_register()
}

/// Adds a repo to the config. Accepts local paths or git URLs.
///
/// For local paths: detects remote and alias, prompts for metadata, registers.
/// For git URLs: asks where to clone, clones, then prompts for metadata.
pub fn run_add(raw_input: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;

    match classify_add_source(raw_input) {
        AddSource::LocalPath(local_path) => add_local_repo(&mut loaded_config, &local_path),
        AddSource::GitUrl(git_url) => add_remote_repo(&mut loaded_config, &git_url),
    }
}
