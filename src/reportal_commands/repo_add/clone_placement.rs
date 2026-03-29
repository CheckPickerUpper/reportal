//! Collects registered repo directories for clone placement choices.

use crate::reportal_config::ReportalConfig;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Collects unique parent directories from all registered repos.
pub fn collect_registered_parent_directories(loaded_config: &ReportalConfig) -> Vec<(String, PathBuf)> {
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
pub fn collect_registered_repo_directories(loaded_config: &ReportalConfig) -> Vec<(String, PathBuf)> {
    let all_repos = loaded_config.repos_matching_tag_filter(&crate::reportal_config::TagFilter::All);
    all_repos
        .iter()
        .map(|(alias, repo)| (alias.to_string(), repo.resolved_path()))
        .collect()
}
