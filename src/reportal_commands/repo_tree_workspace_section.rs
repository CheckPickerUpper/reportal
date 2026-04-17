//! One workspace section of the `rep list` tree-grouped render.

use crate::reportal_config::{RepoEntry, WorkspaceEntry};

/// One workspace section of the `rep list` tree-grouped render.
///
/// Holds borrowed references to the backing config so no cloning
/// happens during tree construction — the grouping is built,
/// rendered, and dropped within a single handler invocation, so
/// the borrow is always shorter than the config load. Member
/// order mirrors the workspace's `repos` field order because
/// that is the sidebar order the user explicitly declared and
/// the listing output must match it.
#[derive(Debug)]
pub struct WorkspaceSection<'config> {
    /// The workspace this section represents.
    pub(super) workspace_name: &'config String,
    /// The full workspace entry, used for description display.
    pub(super) workspace_entry: &'config WorkspaceEntry,
    /// Ordered list of member repos that survived filtering, in
    /// the same order their aliases appear in
    /// `WorkspaceEntry::repo_aliases`.
    pub(super) member_repos: Vec<(&'config str, &'config RepoEntry)>,
}

/// Accessors for a workspace section.
impl<'config> WorkspaceSection<'config> {
    /// The workspace name this section groups under.
    #[must_use]
    pub fn workspace_name(&self) -> &'config str {
        self.workspace_name
    }

    /// The workspace entry, used to read the description for
    /// display purposes.
    #[must_use]
    pub fn workspace_entry(&self) -> &'config WorkspaceEntry {
        self.workspace_entry
    }

    /// The ordered list of member repos that survived the active
    /// filters for this listing.
    #[must_use]
    pub fn member_repos(&self) -> &[(&'config str, &'config RepoEntry)] {
        &self.member_repos
    }
}
