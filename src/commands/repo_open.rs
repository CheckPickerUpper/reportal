/// Fuzzy-selects a repo and opens it in the configured editor.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use dialoguer::FuzzySelect;
use std::process::Command;

/// Opens a repo in the configured editor (default: cursor).
///
/// If `direct_alias` is provided, opens that repo directly without
/// prompting. Otherwise, presents a fuzzy finder for interactive selection.
/// The editor is launched by `cd`-ing into the repo directory first,
/// then running `<editor> .` so the editor opens the folder correctly.
pub fn run_open(tag_filter: TagFilter, direct_alias: &str, editor_override: &str) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let resolved_repo_path = match direct_alias.is_empty() {
        false => {
            let found_repo = loaded_config.get_repo(direct_alias)?;
            found_repo.resolved_path()
        }
        true => {
            let matching_repos = loaded_config.repos_matching_tag_filter(&tag_filter);
            if matching_repos.is_empty() {
                return Err(ReportalError::NoReposMatchFilter);
            }

            let display_labels: Vec<String> = matching_repos
                .iter()
                .map(|(alias, repo)| {
                    let description_suffix = match repo.description().is_empty() {
                        true => String::new(),
                        false => format!(" - {}", repo.description()),
                    };
                    format!("{}{}", alias, description_suffix)
                })
                .collect();

            let selected_index = FuzzySelect::new()
                .with_prompt("Open in editor")
                .items(&display_labels)
                .interact_opt()
                .map_err(|select_error| ReportalError::ConfigIoFailure {
                    reason: select_error.to_string(),
                })?;

            match selected_index {
                Some(chosen_index) => {
                    let (_, chosen_repo) = &matching_repos[chosen_index];
                    chosen_repo.resolved_path()
                }
                None => return Err(ReportalError::SelectionCancelled),
            }
        }
    };

    let editor_command = match editor_override.is_empty() {
        true => loaded_config.default_editor(),
        false => editor_override,
    };

    Command::new(editor_command)
        .arg(".")
        .current_dir(&resolved_repo_path)
        .spawn()
        .map_err(|spawn_error| ReportalError::EditorLaunchFailure {
            reason: spawn_error.to_string(),
        })?;

    println!("Opened {} in {}", resolved_repo_path.display(), editor_command);
    Ok(())
}
