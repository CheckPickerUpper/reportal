//! Step-by-step builder for constructing a validated workspace entry.

use crate::error::ReportalError;
use crate::reportal_config::workspace_entry::WorkspaceEntry;

/// Chainable builder that assembles a `WorkspaceEntry` from separate
/// field inputs and validates name/membership invariants on `build()`.
///
/// Follows the same shape as `RepoRegistrationBuilder` so the two
/// registration flows stay symmetric. The builder deliberately does
/// NOT verify that the collected repo aliases resolve to registered
/// repos: that check requires the full repo registry and belongs at
/// the config level, performed on save/load so dangling references
/// are caught even when an entry is hand-edited into the TOML.
pub struct WorkspaceRegistrationBuilder {
    /// Config key (workspace name) for the entry under construction.
    workspace_name: String,
    /// Alias list collected so far.
    repo_aliases: Vec<String>,
    /// Description collected so far.
    workspace_description: String,
    /// Explicit `.code-workspace` file path, or empty for default.
    workspace_file_path: String,
}

/// Chainable builder methods for assembling a workspace registration.
impl WorkspaceRegistrationBuilder {
    /// Begins a registration for a workspace with the given name.
    ///
    /// The name becomes the config key under `[workspaces.<name>]`
    /// and the default `.code-workspace` filename if no explicit
    /// path is provided later via `workspace_file_path()`.
    #[must_use]
    pub fn start(workspace_name: String) -> Self {
        Self {
            workspace_name,
            repo_aliases: Vec::new(),
            workspace_description: String::new(),
            workspace_file_path: String::new(),
        }
    }

    /// Sets the ordered list of repo aliases for this workspace.
    ///
    /// Order matters: the generated `.code-workspace` file renders
    /// folders in this order in the editor sidebar, so the list
    /// passed here is the user-visible ordering.
    #[must_use]
    pub fn repo_aliases(mut self, aliases: Vec<String>) -> Self {
        self.repo_aliases = aliases;
        self
    }

    /// Sets the human-readable description of this workspace.
    #[must_use]
    pub fn workspace_description(mut self, description_text: String) -> Self {
        self.workspace_description = description_text;
        self
    }

    /// Sets an explicit filesystem path for the `.code-workspace` file.
    ///
    /// Leave unset (or pass an empty string) to use the default
    /// location `~/.reportal/workspaces/<name>.code-workspace`.
    #[must_use]
    pub fn workspace_file_path(mut self, file_path: String) -> Self {
        self.workspace_file_path = file_path;
        self
    }

    /// Validates all fields and produces a workspace name + entry pair.
    ///
    /// Rejects an empty workspace name because it would produce an
    /// unreachable config entry, and rejects an empty alias list
    /// because a workspace with zero repos has no meaning — there
    /// would be nothing to open. Does not verify that each alias
    /// resolves to a registered repo; that check belongs at the
    /// config level where the full repo registry is available.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::ValidationFailure`] when the
    /// workspace name is empty or the repo alias list is empty.
    pub fn build(self) -> Result<(String, WorkspaceEntry), ReportalError> {
        if self.workspace_name.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "workspace name".to_owned(),
                reason: "must not be empty".to_owned(),
            });
        }
        if self.repo_aliases.is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "workspace repos".to_owned(),
                reason: "at least one repo alias is required".to_owned(),
            });
        }
        let validated_entry = WorkspaceEntry {
            repos: self.repo_aliases,
            description: self.workspace_description,
            path: self.workspace_file_path,
        };
        Ok((self.workspace_name, validated_entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_name_is_rejected() {
        let workspace_builder = WorkspaceRegistrationBuilder::start(String::new())
            .repo_aliases(vec!["alpha".to_owned()]);
        let build_outcome = workspace_builder.build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace name"),
            "expected ValidationFailure for workspace name, got {build_outcome:?}",
        );
    }

    #[test]
    fn whitespace_only_name_is_rejected() {
        let workspace_builder = WorkspaceRegistrationBuilder::start("   ".to_owned())
            .repo_aliases(vec!["alpha".to_owned()]);
        let build_outcome = workspace_builder.build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace name"),
            "expected ValidationFailure for workspace name on whitespace input, got {build_outcome:?}",
        );
    }

    #[test]
    fn empty_repo_list_is_rejected() {
        let workspace_builder = WorkspaceRegistrationBuilder::start("backend".to_owned());
        let build_outcome = workspace_builder.build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace repos"),
            "expected ValidationFailure for workspace repos, got {build_outcome:?}",
        );
    }

    #[test]
    fn valid_builder_produces_entry_with_ordered_members() {
        let workspace_builder = WorkspaceRegistrationBuilder::start("backend".to_owned())
            .repo_aliases(vec!["alpha".to_owned(), "bravo".to_owned(), "charlie".to_owned()])
            .workspace_description("Backend services".to_owned())
            .workspace_file_path("~/work.code-workspace".to_owned());
        let (workspace_name, entry) = workspace_builder
            .build()
            .expect("valid builder must succeed");
        assert_eq!(workspace_name, "backend");
        assert_eq!(entry.repo_aliases(), &["alpha", "bravo", "charlie"]);
        assert_eq!(entry.description(), "Backend services");
        assert_eq!(entry.raw_workspace_file_path(), "~/work.code-workspace");
    }

    #[test]
    fn member_order_is_preserved_verbatim() {
        let declared_order = vec!["zebra".to_owned(), "apple".to_owned(), "mango".to_owned()];
        let (_, entry) = WorkspaceRegistrationBuilder::start("ordered".to_owned())
            .repo_aliases(declared_order.clone())
            .build()
            .expect("valid builder must succeed");
        assert_eq!(
            entry.repo_aliases(),
            declared_order.as_slice(),
            "sidebar ordering is load-bearing and must match the declared input verbatim",
        );
    }
}
