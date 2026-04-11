//! Filtering repos by tag for list, status, sync, and fuzzy-select commands.

/// Whether to filter repos by a specific tag or show all repos.
#[derive(Debug)]
pub enum TagFilter {
    /// Show every registered repo regardless of tags.
    All,
    /// Show only repos that carry this exact tag string.
    ByTag(String),
}
