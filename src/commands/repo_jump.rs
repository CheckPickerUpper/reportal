/// Fuzzy-selects a repo and prints its path for shell `cd` integration.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use dialoguer::FuzzySelect;

/// Presents an interactive fuzzy finder of all repos, then prints
/// the selected repo's resolved path to stdout.
///
/// The shell wrapper function (`rj`) reads this output and runs `cd`.
/// Only the path is printed so the wrapper can consume it cleanly.
pub fn run_jump(tag_filter: TagFilter) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;
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
        .with_prompt("Jump to repo")
        .items(&display_labels)
        .interact_opt()
        .map_err(|select_error| ReportalError::ConfigIoFailure {
            reason: select_error.to_string(),
        })?;

    match selected_index {
        Some(chosen_index) => {
            let (_, chosen_repo) = &matching_repos[chosen_index];
            print!("{}", chosen_repo.resolved_path().display());
            Ok(())
        }
        None => Err(ReportalError::SelectionCancelled),
    }
}
