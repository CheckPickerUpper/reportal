//! Workspace entry: a named group of repos that open together as a
//! VSCode/Cursor `.code-workspace` file.

use serde::{Deserialize, Serialize};

/// A single registered VSCode/Cursor workspace definition.
///
/// Declares which repos open together as one editor window. Reportal
/// owns this definition as the single source of truth; the actual
/// `.code-workspace` file on disk is a derived artifact generated from
/// this entry and the member repos' current paths.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkspaceEntry {
    /// Ordered list of repo aliases that belong to this workspace.
    ///
    /// Each alias must resolve to a registered repo in `[repos.*]`.
    /// Validation runs on config load and rejects dangling references.
    pub(super) repos: Vec<String>,
    /// Human-readable description of what this workspace is for.
    #[serde(default)]
    pub(super) description: String,
    /// Filesystem path where the `.code-workspace` file is written.
    ///
    /// May contain `~` for home. When empty, the default location is
    /// `~/.reportal/workspaces/<name>.code-workspace`.
    #[serde(default)]
    pub(super) path: String,
}

/// Accessors for a workspace entry.
impl WorkspaceEntry {
    /// Ordered list of repo aliases that belong to this workspace.
    ///
    /// Order is preserved from the config so the generated
    /// `.code-workspace` file's `folders` array matches what the user
    /// declared, which controls the visual ordering in the editor's
    /// sidebar.
    #[must_use]
    pub fn repo_aliases(&self) -> &[String] {
        &self.repos
    }

    /// Human-readable description of this workspace's purpose.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The raw `.code-workspace` file path as stored in config, before
    /// tilde expansion. An empty string signals that the default
    /// location under `~/.reportal/workspaces/` should be used.
    #[must_use]
    pub fn raw_workspace_file_path(&self) -> &str {
        &self.path
    }

    /// Whether the given repo alias is a member of this workspace.
    ///
    /// Used by the reverse-index lookup that finds every workspace
    /// containing a repo — required so repo path changes can trigger
    /// regeneration of every affected `.code-workspace` file.
    #[must_use]
    pub fn contains_repo(&self, repo_alias: &str) -> bool {
        self.repos.iter().any(|alias| alias == repo_alias)
    }

    /// Replaces the ordered repo alias list with a new membership set.
    ///
    /// Callers must ensure the new aliases all resolve to registered
    /// repos; the config-level validation will reject dangling
    /// references on the next save/load cycle.
    pub fn set_repo_aliases(&mut self, new_repo_aliases: Vec<String>) {
        self.repos = new_repo_aliases;
    }

}
