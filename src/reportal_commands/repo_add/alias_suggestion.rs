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
    Path::new(directory_path).file_name().map_or(
        AliasSuggestion::NoSuggestion,
        |folder_name| AliasSuggestion::Inferred(folder_name.to_string_lossy().into_owned()),
    )
}

/// Extracts the repo name from a git URL for use as default alias.
pub fn repo_name_from_git_url(git_url: &str) -> AliasSuggestion {
    let trimmed = git_url.trim_end_matches(".git");
    let repo_name = trimmed.rsplit('/').next().unwrap_or("");
    if !repo_name.is_empty() {
        return AliasSuggestion::Inferred(repo_name.to_owned());
    }
    let ssh_repo_name = trimmed.rsplit(':').next()
        .and_then(|ssh_path| ssh_path.rsplit('/').next())
        .unwrap_or("");
    if ssh_repo_name.is_empty() {
        AliasSuggestion::NoSuggestion
    } else {
        AliasSuggestion::Inferred(ssh_repo_name.to_owned())
    }
}
