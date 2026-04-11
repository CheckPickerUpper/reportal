//! Collects registered repo directories for clone placement choices.

use crate::reportal_config::ReportalConfig;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Collects unique parent directories from all registered repos.
pub fn collect_registered_parent_directories(loaded_config: &ReportalConfig) -> Vec<(&str, PathBuf)> {
    let all_repos = loaded_config.repos_matching_tag_filter(&crate::reportal_config::TagFilter::All);
    let mut seen_parents: BTreeSet<String> = BTreeSet::new();
    let mut labeled_directories: Vec<(&str, PathBuf)> = Vec::new();

    for (alias, repo) in &all_repos {
        let resolved = repo.resolved_path();
        let Some(parent_dir) = resolved.parent() else {
            continue;
        };
        let parent_string = parent_dir.display().to_string();
        if seen_parents.contains(&parent_string) {
            continue;
        }
        seen_parents.insert(parent_string);
        labeled_directories.push((alias, parent_dir.to_path_buf()));
    }

    labeled_directories
}

/// Collects registered repo directories that could be parents for a new clone.
pub fn collect_registered_repo_directories(loaded_config: &ReportalConfig) -> Vec<(&str, PathBuf)> {
    let all_repos = loaded_config.repos_matching_tag_filter(&crate::reportal_config::TagFilter::All);
    all_repos
        .iter()
        .map(|(alias, repo)| (alias.as_str(), repo.resolved_path()))
        .collect()
}
