//! Clone placement strategy — where to put a cloned repo relative to existing repos.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::FuzzySelect;
use std::path::PathBuf;

use super::clone_placement;

/// Where the user wants to clone a remote repo.
pub enum CloneDestination {
    /// Type a custom absolute path.
    CustomPath,
    /// Clone as a sibling of an existing registered repo.
    SiblingOf(PathBuf),
    /// Clone as a child inside an existing registered repo's directory.
    ChildOf(PathBuf),
}

/// Asks the user what kind of placement they want, then which repo to place relative to.
pub fn prompt_clone_destination(loaded_config: &ReportalConfig) -> Result<CloneDestination, ReportalError> {
    let prompt_theme = terminal_style::reportal_prompt_theme();

    let sibling_directories = clone_placement::collect_registered_parent_directories(loaded_config);
    let child_directories = clone_placement::collect_registered_repo_directories(loaded_config);

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
