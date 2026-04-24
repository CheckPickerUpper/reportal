//! Workspace entry: a named group of repos that open together as a
//! VSCode/Cursor `.code-workspace` file.

use crate::reportal_config::has_aliases::HasAliases;
use crate::reportal_config::repository_color::RepoColor;
use crate::reportal_config::tab_title::TabTitle;
use crate::reportal_config::workspace_member::{
    WorkspaceMember, WorkspaceMemberAliasLookup,
};
use serde::{Deserialize, Serialize};

/// A single registered VSCode/Cursor workspace definition.
///
/// Declares which folders open together as one editor window.
/// Reportal owns this definition as the single source of truth; the
/// actual `.code-workspace` file on disk is a derived artifact
/// generated from each member's resolved path (repo registry lookup
/// for alias members, direct tilde-expansion for inline-path
/// members).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkspaceEntry {
    /// Ordered list of members — each is either a registered-repo
    /// reference or an inline filesystem path.
    ///
    /// The TOML field is still called `repos` for backwards
    /// compatibility with configs written before inline paths were
    /// supported. Bare strings in the array continue to parse as
    /// `WorkspaceMember::RegisteredRepo` via the untagged serde
    /// representation on `WorkspaceMember`, so v0.14.1 configs load
    /// unchanged.
    pub(super) repos: Vec<WorkspaceMember>,
    /// Human-readable description of what this workspace is for.
    #[serde(default)]
    pub(super) description: String,
    /// Filesystem path of the workspace directory on disk.
    ///
    /// Semantic change in v0.15.2: this field now stores the
    /// directory that contains the workspace's symlinks and the
    /// `<name>.code-workspace` file, not the workspace file itself.
    /// May contain `~` for home. When empty, the default location
    /// is `<default_workspace_root>/<name>/` as resolved from the
    /// `[settings]` table.
    ///
    /// Pre-v0.15.2 configs stored the `.code-workspace` file path
    /// here (detected by a trailing `.code-workspace` component).
    /// Such entries are auto-migrated on the first `rep workspace
    /// jump` / `open` / `rebuild` that touches them.
    #[serde(default)]
    pub(super) path: String,
    /// Alternative short names that resolve to this workspace's
    /// canonical key in commands that target a workspace by name.
    ///
    /// Each alias must be unique across every workspace's canonical
    /// name and every other workspace's alias list, and must not
    /// collide with any repo's canonical key or repo-level alias.
    /// Validation runs on config load so an ambiguous alias is
    /// rejected before any command resolves it.
    #[serde(default)]
    pub(super) aliases: Vec<String>,
    /// Short label shown in the shell prompt badge and editor
    /// window title when the user is inside this workspace. When
    /// `UseAlias`, the workspace's canonical name is used.
    #[serde(default)]
    pub(super) title: TabTitle,
    /// Accent color for the shell prompt badge, the editor title
    /// bar (`workbench.colorCustomizations`), and the terminal tab
    /// strip when the user is inside this workspace. When
    /// `ResetToDefault`, no accent is applied and the badge falls
    /// back to the terminal's default foreground.
    #[serde(default)]
    pub(super) color: RepoColor,
}

/// Accessors and mutators for a workspace entry.
impl WorkspaceEntry {
    /// Ordered list of every member in this workspace.
    ///
    /// Callers that need to resolve each member to an absolute path
    /// pattern-match on [`WorkspaceMember`] variants; callers that
    /// only care about registered-repo references use
    /// [`Self::repo_aliases`].
    #[must_use]
    pub fn members(&self) -> &[WorkspaceMember] {
        &self.repos
    }

    /// The alias strings of every member that is a registered-repo
    /// reference, in declared order.
    ///
    /// Inline-path members are skipped because they do not
    /// participate in the repo-rename reverse-index or the
    /// `validate_workspace_references` dangling-member check.
    #[must_use]
    pub fn repo_aliases(&self) -> Vec<&str> {
        self.repos
            .iter()
            .filter_map(|member| match member.registered_repo_alias() {
                WorkspaceMemberAliasLookup::Matches(alias) => Some(alias),
                WorkspaceMemberAliasLookup::NotARepoReference => None,
            })
            .collect()
    }

    /// Human-readable description of this workspace's purpose.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The raw workspace directory path as stored in config, before
    /// tilde expansion.
    ///
    /// Since v0.15.2 this is the directory containing the symlinks
    /// and the `.code-workspace` file, not the `.code-workspace`
    /// file path itself. An empty string signals that the default
    /// location `<default_workspace_root>/<name>/` should be used.
    /// Pre-v0.15.2 configs may still store a path ending in
    /// `.code-workspace`; use [`Self::is_legacy_file_path`] to
    /// detect and migrate such entries.
    #[must_use]
    pub fn raw_workspace_file_path(&self) -> &str {
        &self.path
    }

    /// Whether the stored `path` looks like a pre-v0.15.2
    /// `.code-workspace` file path (rather than a workspace
    /// directory).
    ///
    /// True when the stored path ends with the `.code-workspace`
    /// extension. Callers use this to trigger auto-migration on
    /// the first command that touches the entry after upgrade.
    #[must_use]
    pub fn is_legacy_file_path(&self) -> bool {
        let trimmed = self.path.trim();
        !trimmed.is_empty()
            && std::path::Path::new(trimmed)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("code-workspace"))
    }

    /// Replaces the workspace directory path stored in config.
    ///
    /// Used by the auto-migration path to formalize the move from
    /// a legacy `.code-workspace`-file-path entry to the new
    /// directory-based layout, and by any future `rep workspace
    /// move` subcommand.
    pub fn set_workspace_directory_path(&mut self, new_directory_path: String) {
        self.path = new_directory_path;
    }

    /// Whether the given repo alias is a registered-repo member of
    /// this workspace.
    ///
    /// Used by the reverse-index lookup `workspaces_containing_repo`
    /// and the `rep remove` guard. Inline-path members never match
    /// because they carry no repo alias by construction.
    #[must_use]
    pub fn contains_repo(&self, repository_alias: &str) -> bool {
        self.repos.iter().any(|member| {
            matches!(
                member.registered_repo_alias(),
                WorkspaceMemberAliasLookup::Matches(alias) if alias == repository_alias
            )
        })
    }

    /// @why Exposes the workspace's prompt-badge / window-title
    /// label so `rep prompt` and `rep workspace rebuild` can render
    /// the same identity without re-reading the raw field.
    #[must_use]
    pub fn tab_title(&self) -> &TabTitle {
        &self.title
    }

    /// @why Exposes the workspace's accent color so the prompt
    /// badge, the terminal tab strip, and the editor title bar all
    /// draw from one source of truth per workspace.
    #[must_use]
    pub fn workspace_color(&self) -> &RepoColor {
        &self.color
    }

    /// Replaces the ordered member list with a new one.
    ///
    /// Callers that want to mutate registered-repo membership
    /// without touching inline-path members should compose a new
    /// list from [`Self::members`] and pass it here. The
    /// `rep workspace add-repo` / `remove-repo` commands do exactly
    /// that so inline-path members are preserved across alias-only
    /// edits.
    pub fn set_members(&mut self, new_members: Vec<WorkspaceMember>) {
        self.repos = new_members;
    }
}

impl HasAliases for WorkspaceEntry {
    fn aliases(&self) -> &[String] {
        &self.aliases
    }
}
