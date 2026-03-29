//! Classifies raw user input as a local path or git URL.

/// Whether the input is a local path or a git URL to clone.
pub enum AddSource {
    /// A local directory that already exists.
    LocalPath(String),
    /// A git URL that needs cloning first.
    GitUrl(String),
}

/// Determines if the input looks like a git URL or a local path.
pub fn classify_add_source(raw_input: &str) -> AddSource {
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
