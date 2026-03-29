//! Alias inference from filesystem paths and git URLs.

use std::path::Path;

/// Whether an alias could be inferred from a path or URL.
pub enum AliasSuggestion {
    /// A name was extracted as a suggested alias.
    Inferred(String),
    /// No usable name could be extracted.
    NoSuggestion,
}

/// Extracts a suggested alias from the last segment of a filesystem path.
pub fn suggest_alias_from_path(directory_path: &str) -> AliasSuggestion {
    match Path::new(directory_path).file_name() {
        Some(folder_name) => AliasSuggestion::Inferred(folder_name.to_string_lossy().to_string()),
        None => AliasSuggestion::NoSuggestion,
    }
}

/// Extracts the repo name from a git URL for use as default alias.
pub fn repo_name_from_git_url(git_url: &str) -> AliasSuggestion {
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
