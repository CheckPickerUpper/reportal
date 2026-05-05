//! Step-by-step builder for constructing a validated workspace entry.

use crate::error::ReportalError;
use crate::reportal_config::shell_alias_export::ShellAliasExport;
use crate::reportal_config::workspace_entry::WorkspaceEntry;
use crate::reportal_config::workspace_member::WorkspaceMember;
use crate::system_executable_lookup::SystemExecutableLookupOutcome;

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
    /// Member repo alias list collected so far.
    repo_aliases: Vec<String>,
    /// Description collected so far.
    workspace_description: String,
    /// Explicit `.code-workspace` file path, or empty for default.
    workspace_file_path: String,
    /// Short-name aliases that resolve to this workspace in
    /// commands taking a workspace name argument.
    workspace_aliases: Vec<String>,
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
            workspace_aliases: Vec::new(),
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

    /// Sets the short-name aliases that resolve to this workspace.
    ///
    /// Each alias is checked for emptiness and intra-list duplicates
    /// at `build()` time. Cross-workspace and cross-namespace (vs.
    /// repo) collision detection happens one layer up at the config
    /// level, because the builder does not have the full registry in
    /// scope and trying to validate there would either require
    /// plumbing the registry in (defeating the builder's isolation)
    /// or silently let collisions through.
    #[must_use]
    pub fn workspace_aliases(mut self, declared_aliases: Vec<String>) -> Self {
        self.workspace_aliases = declared_aliases;
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
    /// workspace name is empty, the repo alias list is empty, or
    /// any declared workspace alias is empty, duplicated within the
    /// list, or equal to the owning workspace's canonical name.
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
        validate_alias_list_shape(&self.workspace_aliases, &self.workspace_name)?;
        reject_alias_that_shadows_system_command(&self.workspace_name, "workspace name")?;
        for declared_alias in &self.workspace_aliases {
            reject_alias_that_shadows_system_command(declared_alias, "workspace alias")?;
        }
        let validated_entry = WorkspaceEntry {
            repos: self
                .repo_aliases
                .into_iter()
                .map(WorkspaceMember::RegisteredRepo)
                .collect(),
            description: self.workspace_description,
            path: self.workspace_file_path,
            aliases: self.workspace_aliases,
            title: crate::reportal_config::TabTitle::default(),
            color: crate::reportal_config::RepoColor::default(),
            shell_alias: ShellAliasExport::Disabled,
        };
        Ok((self.workspace_name, validated_entry))
    }
}

/// Validates the shape of a workspace's declared alias list,
/// independent of other workspaces and repos.
///
/// Only checks intra-list invariants: no empty/whitespace entries
/// (they would resolve nothing), no duplicates within the list
/// (they waste config bytes and imply ordering semantics the
/// resolver does not honor), and no alias equal to the workspace's
/// own canonical name (declaring `vn` on workspace `vn` is a no-op
/// that signals user confusion).
///
/// Cross-entity collisions (alias of workspace A clashes with
/// canonical name or alias of workspace B, or any repo) require
/// the full config registry and belong on the config-level
/// validation pass that runs on load and `add_workspace`.
///
/// # Errors
///
/// Returns [`ReportalError::ValidationFailure`] with `field =
/// "workspace alias"` for each kind of shape violation.
fn validate_alias_list_shape(
    declared_aliases: &[String],
    owning_workspace_name: &str,
) -> Result<(), ReportalError> {
    for declared in declared_aliases {
        if declared.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "workspace alias".to_owned(),
                reason: format!(
                    "workspace '{owning_workspace_name}' declares an empty alias"
                ),
            });
        }
        if declared == owning_workspace_name {
            return Err(ReportalError::ValidationFailure {
                field: "workspace alias".to_owned(),
                reason: format!(
                    "workspace '{owning_workspace_name}' declares '{declared}' as an alias but that is already its canonical name"
                ),
            });
        }
    }
    for earlier_index in 0..declared_aliases.len() {
        for later_index in (earlier_index + 1)..declared_aliases.len() {
            if declared_aliases[earlier_index] == declared_aliases[later_index] {
                return Err(ReportalError::ValidationFailure {
                    field: "workspace alias".to_owned(),
                    reason: format!(
                        "workspace '{owning_workspace_name}' declares duplicate alias '{}'",
                        declared_aliases[earlier_index],
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Rejects a workspace name or alias that resolves to an existing
/// executable on the user's `PATH`, so opting the workspace into
/// shell-alias export later cannot silently shadow a real
/// system command (`mc`, `train`, `home`, etc.).
fn reject_alias_that_shadows_system_command(
    candidate_name: &str,
    field_label_for_error: &str,
) -> Result<(), ReportalError> {
    match SystemExecutableLookupOutcome::for_candidate_name(candidate_name.trim()) {
        SystemExecutableLookupOutcome::NotFound => Ok(()),
        SystemExecutableLookupOutcome::ShadowsExisting { existing_executable } => {
            Err(ReportalError::ValidationFailure {
                field: field_label_for_error.to_owned(),
                reason: format!(
                    "'{candidate}' would shadow the existing system command at {existing}; pick a different name",
                    candidate = candidate_name.trim(),
                    existing = existing_executable.display(),
                ),
            })
        }
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

    #[test]
    fn workspace_aliases_populate_into_entry() {
        let declared_aliases = vec!["vn".to_owned(), "noble".to_owned()];
        let (_, entry) = WorkspaceRegistrationBuilder::start("venoble".to_owned())
            .repo_aliases(vec!["app".to_owned()])
            .workspace_aliases(declared_aliases.clone())
            .build()
            .expect("valid builder must succeed");
        use crate::reportal_config::has_aliases::HasAliases;
        assert_eq!(entry.aliases(), declared_aliases.as_slice());
    }

    #[test]
    fn empty_alias_is_rejected() {
        let build_outcome = WorkspaceRegistrationBuilder::start("venoble".to_owned())
            .repo_aliases(vec!["app".to_owned()])
            .workspace_aliases(vec!["vn".to_owned(), String::new()])
            .build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace alias"),
            "expected ValidationFailure for workspace alias on empty entry, got {build_outcome:?}",
        );
    }

    #[test]
    fn whitespace_only_alias_is_rejected() {
        let build_outcome = WorkspaceRegistrationBuilder::start("venoble".to_owned())
            .repo_aliases(vec!["app".to_owned()])
            .workspace_aliases(vec!["   ".to_owned()])
            .build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace alias"),
            "expected ValidationFailure on whitespace-only alias, got {build_outcome:?}",
        );
    }

    #[test]
    fn duplicate_alias_in_same_workspace_is_rejected() {
        let build_outcome = WorkspaceRegistrationBuilder::start("venoble".to_owned())
            .repo_aliases(vec!["app".to_owned()])
            .workspace_aliases(vec!["vn".to_owned(), "vn".to_owned()])
            .build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace alias"),
            "expected ValidationFailure on duplicate alias, got {build_outcome:?}",
        );
    }

    #[test]
    fn alias_equal_to_own_canonical_name_is_rejected() {
        let build_outcome = WorkspaceRegistrationBuilder::start("venoble".to_owned())
            .repo_aliases(vec!["app".to_owned()])
            .workspace_aliases(vec!["venoble".to_owned()])
            .build();
        assert!(
            matches!(build_outcome, Err(ReportalError::ValidationFailure { ref field, .. }) if field == "workspace alias"),
            "expected ValidationFailure when alias equals canonical name, got {build_outcome:?}",
        );
    }
}
