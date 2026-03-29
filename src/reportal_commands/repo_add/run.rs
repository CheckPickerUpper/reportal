//! Entry point for the `rep add` command.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Input};
use owo_colors::OwoColorize;
use std::path::PathBuf;

use super::add_source::{classify_add_source, AddSource};
use super::alias_suggestion::{repo_name_from_git_url, suggest_alias_from_path, AliasSuggestion};
use super::clone_destination::{prompt_clone_destination, CloneDestination};
use super::git_clone_operation::GitCloneOperation;
use super::git_remote_detection::{detect_git_remote, GitRemoteDetection};
use super::registration_context::RegistrationContext;

/// Adds a repo to the config. Accepts local paths or git URLs.
///
/// For local paths: detects remote and alias, prompts for metadata, registers.
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
            let registration = RegistrationContext {
                loaded_config: &mut loaded_config,
                filesystem_path: &local_path,
                suggested_alias,
                detected_remote,
            };
            registration.collect_metadata_and_register()
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

            let clone_operation = GitCloneOperation {
                git_url: &git_url,
                target_directory: &clone_parent,
            };
            clone_operation.run_git_clone()?;

            let cloned_repo_path = clone_parent.join(&suggested_alias);
            let final_repo_path = cloned_repo_path.display().to_string();

            let registration = RegistrationContext {
                loaded_config: &mut loaded_config,
                filesystem_path: &final_repo_path,
                suggested_alias,
                detected_remote: git_url,
            };
            registration.collect_metadata_and_register()
        }
    }
}
