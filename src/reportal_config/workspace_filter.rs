//! Filtering repos by workspace membership for `rep list`.

/// Whether to filter repos by workspace membership or show every
/// workspace section.
///
/// Modeled as an enum rather than an `Option<String>` so the
/// "show all workspaces" branch is a named variant that callers
/// must acknowledge in pattern matches. This prevents the failure
/// mode where a missing filter is silently treated as a hidden
/// default and a branch is forgotten in future changes.
#[derive(Debug)]
pub enum WorkspaceFilter {
    /// Show every registered workspace (and the unassigned
    /// section for repos in zero workspaces).
    All,
    /// Show only the named workspace. The unassigned section is
    /// suppressed because the query is explicitly scoped to one
    /// workspace.
    ByName(String),
}
