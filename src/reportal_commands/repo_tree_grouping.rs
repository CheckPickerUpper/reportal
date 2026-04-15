//! Builds the workspace-rooted tree structure rendered by `rep list`.
//!
//! The tree groups registered repos by workspace membership:
//! each workspace becomes a section whose members are the repos
//! listed in its `repos` field, preserving declared order. Repos
//! that belong to no workspace land in a synthetic "unassigned"
//! section at the bottom so no repo is ever hidden from the
//! listing output.

use crate::error::ReportalError;
use crate::reportal_commands::repo_tree_workspace_section::WorkspaceSection;
use crate::reportal_config::{RepoEntry, ReportalConfig, TagFilter, WorkspaceFilter};
use std::collections::BTreeSet;

/// Workspace-rooted grouping of registered repos, filtered by
/// the active `--tag` and `--workspace` selections.
///
/// Built once per `rep list` invocation, rendered to stdout by
/// `repo_listing::run_list`, and dropped when the handler
/// returns. Holding borrowed references to the backing config
/// avoids cloning repo entries into the tree.
///
/// Multi-workspace repos (repos listed as members of more than
/// one workspace) render under every containing workspace rather
/// than being deduped into a single section. Deduping would
/// silently hide user-declared multi-membership, which is a
/// legitimate relationship the user must see. The
/// `distinct_repo_count` field reports unique repos so the
/// summary line at the bottom of the output does not exaggerate
/// the total when a repo appears in multiple sections.
#[derive(Debug)]
pub struct RepoTreeGrouping<'config> {
    /// Workspace sections that have at least one member surviving
    /// filtering, in the iteration order of `workspaces_with_names`
    /// (which is `BTreeMap` order — deterministic across runs).
    workspace_sections: Vec<WorkspaceSection<'config>>,
    /// Repos that belong to zero workspaces in the full config,
    /// populated only when the workspace filter is `All`. When a
    /// specific workspace is requested this section is empty
    /// because the query is scoped to one workspace and
    /// non-members are off-topic.
    unassigned_repos: Vec<(&'config String, &'config RepoEntry)>,
    /// Count of distinct repo aliases visible anywhere in the
    /// tree, used by the summary line at the bottom of `rep list`.
    distinct_repo_count: usize,
}

/// Accessors for a built `RepoTreeGrouping`.
impl<'config> RepoTreeGrouping<'config> {
    /// Builds the grouping from a loaded config and the active
    /// filters by delegating to methods on the parameter struct.
    /// Splitting the collection logic across per-section methods
    /// keeps each one within the project's nesting budget — a
    /// single inlined `build` body exceeded the limit because
    /// `match` inside `for` inside `if` pushed the depth over.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] when the
    /// workspace filter is `ByName(name)` and no workspace with
    /// that name is registered, or
    /// [`ReportalError::RepoNotFound`] if a workspace's member
    /// alias fails post-load resolution (which should not happen
    /// if `validate_workspace_references` succeeded).
    pub fn build(
        build_params: &RepoTreeGroupingParams<'config>,
    ) -> Result<Self, ReportalError> {
        build_params.validate_workspace_target()?;
        let workspace_sections = build_params.collect_workspace_sections()?;
        let unassigned_repos = build_params.collect_unassigned_repos();
        let mut distinct_aliases: BTreeSet<&str> = BTreeSet::new();
        for section in &workspace_sections {
            for (member_alias, _member_entry) in section.member_repos() {
                distinct_aliases.insert(member_alias.as_str());
            }
        }
        for (orphan_alias, _orphan_entry) in &unassigned_repos {
            distinct_aliases.insert(orphan_alias.as_str());
        }
        let distinct_repo_count = distinct_aliases.len();
        Ok(Self {
            workspace_sections,
            unassigned_repos,
            distinct_repo_count,
        })
    }

    /// The workspace sections in iteration order, each with its
    /// ordered member list.
    #[must_use]
    pub fn workspace_sections(&self) -> &[WorkspaceSection<'config>] {
        &self.workspace_sections
    }

    /// The repos that belong to zero workspaces. Empty when the
    /// workspace filter is scoped to a specific workspace.
    #[must_use]
    pub fn unassigned_repos(&self) -> &[(&'config String, &'config RepoEntry)] {
        &self.unassigned_repos
    }

    /// Count of distinct repos visible anywhere in the tree.
    #[must_use]
    pub fn distinct_repo_count(&self) -> usize {
        self.distinct_repo_count
    }

    /// Whether the tree has zero visible sections and zero
    /// unassigned repos, used by the caller to decide whether to
    /// render an empty-state message instead of the tree.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.workspace_sections.is_empty() && self.unassigned_repos.is_empty()
    }
}

/// Borrowed inputs required to build a [`RepoTreeGrouping`].
///
/// Bundling the config and filter references into a single
/// parameter struct satisfies the project rule that constructors
/// take at most one non-`self` argument, and the named fields
/// prevent call sites from swapping the two filter refs (which
/// share the same enum taxonomy at a glance).
pub struct RepoTreeGroupingParams<'config> {
    /// The loaded config whose workspaces and repos are grouped.
    pub(super) loaded_config: &'config ReportalConfig,
    /// The tag filter applied to every member before inclusion.
    pub(super) tag_filter: &'config TagFilter,
    /// The workspace filter selecting which sections render.
    pub(super) workspace_filter: &'config WorkspaceFilter,
}

/// Collection methods for `RepoTreeGroupingParams`, each written
/// as a separate method so nesting starts fresh inside each body
/// and no single method exceeds the project's depth budget.
impl<'config> RepoTreeGroupingParams<'config> {
    /// Ensures the workspace filter, when set to `ByName`,
    /// references a workspace that exists. A silent empty tree
    /// for a typo would mask the mistake, so the early check
    /// fails loudly with [`ReportalError::WorkspaceNotFound`].
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::WorkspaceNotFound`] when the
    /// filter is `ByName(name)` and no workspace with that name
    /// is registered.
    fn validate_workspace_target(&self) -> Result<(), ReportalError> {
        match *self.workspace_filter {
            WorkspaceFilter::All => Ok(()),
            WorkspaceFilter::ByName(ref requested_name) => {
                self.loaded_config.get_workspace(requested_name)?;
                Ok(())
            }
        }
    }

    /// Walks every registered workspace in deterministic order,
    /// filters by the workspace-filter selection, resolves each
    /// member alias to its backing [`RepoEntry`], and keeps only
    /// the members that survive the tag filter. Workspaces that
    /// end up with zero surviving members are omitted because
    /// rendering an empty section adds no information to the
    /// listing.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::RepoNotFound`] when a workspace
    /// member alias does not resolve, which should not happen
    /// after a successful `validate_workspace_references` pass.
    fn collect_workspace_sections(
        &self,
    ) -> Result<Vec<WorkspaceSection<'config>>, ReportalError> {
        let workspace_restriction_target = self.workspace_restriction_target();
        let workspace_restriction_active = !workspace_restriction_target.is_empty();
        let mut workspace_sections: Vec<WorkspaceSection<'config>> = Vec::new();
        for (workspace_name, workspace_entry) in self.loaded_config.workspaces_with_names() {
            if workspace_restriction_active && workspace_name != workspace_restriction_target {
                continue;
            }
            let surviving_members = self.resolve_surviving_members(workspace_entry)?;
            if surviving_members.is_empty() {
                continue;
            }
            workspace_sections.push(WorkspaceSection {
                workspace_name,
                workspace_entry,
                member_repos: surviving_members,
            });
        }
        Ok(workspace_sections)
    }

    /// Resolves every member alias in the given workspace to its
    /// backing [`RepoEntry`] and keeps only the members that
    /// survive the tag filter. Extracted as its own method so
    /// the inner iteration starts its nesting count fresh,
    /// which keeps `collect_workspace_sections` within the
    /// project's depth budget.
    ///
    /// # Errors
    ///
    /// Returns [`ReportalError::RepoNotFound`] when a member
    /// alias does not resolve.
    fn resolve_surviving_members(
        &self,
        workspace_entry: &'config crate::reportal_config::WorkspaceEntry,
    ) -> Result<Vec<(&'config String, &'config RepoEntry)>, ReportalError> {
        let tag_restriction_target = self.tag_restriction_target();
        let tag_restriction_active = !tag_restriction_target.is_empty();
        let mut surviving_members: Vec<(&'config String, &'config RepoEntry)> = Vec::new();
        for member_alias in workspace_entry.repo_aliases() {
            let member_repo = self.loaded_config.get_repo(member_alias)?;
            if tag_restriction_active
                && !member_repo
                    .tags()
                    .iter()
                    .any(|existing_tag| existing_tag == tag_restriction_target)
            {
                continue;
            }
            surviving_members.push((member_alias, member_repo));
        }
        Ok(surviving_members)
    }

    /// Walks every registered repo and collects the ones that
    /// belong to zero workspaces AND survive the tag filter.
    /// Returns an empty vector when the workspace filter is
    /// scoped to a specific workspace because the unassigned
    /// section is suppressed in that case — the query is about
    /// one workspace, so non-members are off-topic.
    fn collect_unassigned_repos(&self) -> Vec<(&'config String, &'config RepoEntry)> {
        let workspace_restriction_target = self.workspace_restriction_target();
        let workspace_restriction_active = !workspace_restriction_target.is_empty();
        let tag_restriction_target = self.tag_restriction_target();
        let tag_restriction_active = !tag_restriction_target.is_empty();
        let mut unassigned_repos: Vec<(&'config String, &'config RepoEntry)> = Vec::new();
        if workspace_restriction_active {
            return unassigned_repos;
        }
        for (repo_alias, repo_entry) in self.loaded_config.repos_with_aliases() {
            let repo_survives_tag = !tag_restriction_active
                || repo_entry
                    .tags()
                    .iter()
                    .any(|existing_tag| existing_tag == tag_restriction_target);
            if !repo_survives_tag {
                continue;
            }
            let containing_workspaces =
                self.loaded_config.workspaces_containing_repo(repo_alias);
            if containing_workspaces.is_empty() {
                unassigned_repos.push((repo_alias, repo_entry));
            }
        }
        unassigned_repos
    }

    /// Returns the target workspace name as a borrowed string
    /// slice, or an empty string when the filter is `All`. The
    /// empty-string sentinel is safe because
    /// `WorkspaceFilterArgs::into_workspace_filter` rejects
    /// empty input as the `All` variant, so `ByName` never
    /// carries an empty string in production.
    fn workspace_restriction_target(&self) -> &'config str {
        match *self.workspace_filter {
            WorkspaceFilter::All => "",
            WorkspaceFilter::ByName(ref requested_name) => requested_name.as_str(),
        }
    }

    /// Returns the target tag as a borrowed string slice, or an
    /// empty string when the filter is `All`. The empty-string
    /// sentinel is safe because `TagFilterArgs::into_tag_filter`
    /// rejects empty input as the `All` variant.
    fn tag_restriction_target(&self) -> &'config str {
        match *self.tag_filter {
            TagFilter::All => "",
            TagFilter::ByTag(ref target_tag) => target_tag.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_test_config(toml_text: &str) -> ReportalConfig {
        toml::from_str(toml_text).expect("test fixture must parse as valid config")
    }

    fn build_tree<'config>(
        test_config: &'config ReportalConfig,
        tag_filter: &'config TagFilter,
        workspace_filter: &'config WorkspaceFilter,
    ) -> Result<RepoTreeGrouping<'config>, ReportalError> {
        RepoTreeGrouping::build(&RepoTreeGroupingParams {
            loaded_config: test_config,
            tag_filter,
            workspace_filter,
        })
    }

    #[test]
    fn unassigned_repo_lands_in_unassigned_section() {
        let test_config = parse_test_config(
            r#"
            [repos.lonely]
            path = "/tmp/lonely"
            "#,
        );
        let tag_filter = TagFilter::All;
        let workspace_filter = WorkspaceFilter::All;
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        assert_eq!(tree_grouping.unassigned_repos().len(), 1);
        assert_eq!(tree_grouping.unassigned_repos()[0].0, "lonely");
        assert!(tree_grouping.workspace_sections().is_empty());
        assert_eq!(tree_grouping.distinct_repo_count(), 1);
    }

    #[test]
    fn multi_workspace_repo_renders_in_every_section() {
        let test_config = parse_test_config(
            r#"
            [repos.shared]
            path = "/tmp/shared"

            [workspaces.frontend]
            repos = ["shared"]

            [workspaces.backend]
            repos = ["shared"]
            "#,
        );
        let tag_filter = TagFilter::All;
        let workspace_filter = WorkspaceFilter::All;
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        assert_eq!(
            tree_grouping.workspace_sections().len(),
            2,
            "repo should render under BOTH containing workspaces",
        );
        assert_eq!(
            tree_grouping.distinct_repo_count(),
            1,
            "distinct count must dedupe the multi-workspace repo",
        );
        assert!(
            tree_grouping.unassigned_repos().is_empty(),
            "a repo that IS in workspaces must not appear as unassigned",
        );
    }

    #[test]
    fn workspace_filter_by_name_scopes_to_one_section() {
        let test_config = parse_test_config(
            r#"
            [repos.alpha]
            path = "/tmp/alpha"
            [repos.bravo]
            path = "/tmp/bravo"

            [workspaces.frontend]
            repos = ["alpha"]

            [workspaces.backend]
            repos = ["bravo"]
            "#,
        );
        let tag_filter = TagFilter::All;
        let workspace_filter = WorkspaceFilter::ByName("frontend".to_owned());
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        assert_eq!(tree_grouping.workspace_sections().len(), 1);
        assert_eq!(
            tree_grouping.workspace_sections()[0].workspace_name(),
            "frontend",
        );
        assert!(
            tree_grouping.unassigned_repos().is_empty(),
            "unassigned section must be suppressed when workspace filter is scoped",
        );
    }

    #[test]
    fn workspace_filter_unknown_name_errors() {
        let test_config = parse_test_config(
            r#"
            [repos.alpha]
            path = "/tmp/alpha"
            "#,
        );
        let tag_filter = TagFilter::All;
        let workspace_filter = WorkspaceFilter::ByName("nonexistent".to_owned());
        let build_outcome = build_tree(&test_config, &tag_filter, &workspace_filter);
        assert!(
            matches!(build_outcome, Err(ReportalError::WorkspaceNotFound { .. })),
            "unknown workspace name must error loudly, not yield empty output: {build_outcome:?}",
        );
    }

    #[test]
    fn tag_filter_restricts_members_within_workspaces() {
        let test_config = parse_test_config(
            r#"
            [repos.api]
            path = "/tmp/api"
            tags = ["work"]

            [repos.hobby]
            path = "/tmp/hobby"
            tags = ["personal"]

            [workspaces.combined]
            repos = ["api", "hobby"]
            "#,
        );
        let tag_filter = TagFilter::ByTag("work".to_owned());
        let workspace_filter = WorkspaceFilter::All;
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        assert_eq!(tree_grouping.workspace_sections().len(), 1);
        assert_eq!(tree_grouping.workspace_sections()[0].member_repos().len(), 1);
        assert_eq!(
            tree_grouping.workspace_sections()[0].member_repos()[0].0,
            "api",
        );
    }

    #[test]
    fn workspace_with_no_surviving_members_is_omitted() {
        let test_config = parse_test_config(
            r#"
            [repos.personal_only]
            path = "/tmp/personal"
            tags = ["personal"]

            [workspaces.personal_ws]
            repos = ["personal_only"]
            "#,
        );
        let tag_filter = TagFilter::ByTag("work".to_owned());
        let workspace_filter = WorkspaceFilter::All;
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        assert!(
            tree_grouping.workspace_sections().is_empty(),
            "workspaces with zero surviving members must be hidden",
        );
        assert!(
            tree_grouping.unassigned_repos().is_empty(),
            "the only repo was filtered out by tag",
        );
        assert!(tree_grouping.is_empty());
    }

    #[test]
    fn member_order_matches_declared_order() {
        let test_config = parse_test_config(
            r#"
            [repos.alpha]
            path = "/tmp/alpha"
            [repos.bravo]
            path = "/tmp/bravo"
            [repos.charlie]
            path = "/tmp/charlie"

            [workspaces.ordered]
            repos = ["charlie", "alpha", "bravo"]
            "#,
        );
        let tag_filter = TagFilter::All;
        let workspace_filter = WorkspaceFilter::All;
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        let observed_member_order: Vec<&str> = tree_grouping.workspace_sections()[0]
            .member_repos()
            .iter()
            .map(|(repo_alias, _repo_entry)| repo_alias.as_str())
            .collect();
        assert_eq!(
            observed_member_order,
            vec!["charlie", "alpha", "bravo"],
            "sidebar ordering from declared `repos` field must round-trip into the tree",
        );
    }

    #[test]
    fn tag_and_workspace_filters_intersect() {
        let test_config = parse_test_config(
            r#"
            [repos.api]
            path = "/tmp/api"
            tags = ["work"]
            [repos.tool]
            path = "/tmp/tool"
            tags = ["work"]
            [repos.game]
            path = "/tmp/game"
            tags = ["fun"]

            [workspaces.backend]
            repos = ["api", "tool"]

            [workspaces.play]
            repos = ["game"]
            "#,
        );
        let tag_filter = TagFilter::ByTag("work".to_owned());
        let workspace_filter = WorkspaceFilter::ByName("backend".to_owned());
        let tree_grouping = build_tree(&test_config, &tag_filter, &workspace_filter)
            .expect("build must succeed");
        assert_eq!(tree_grouping.workspace_sections().len(), 1);
        assert_eq!(
            tree_grouping.workspace_sections()[0].member_repos().len(),
            2,
            "both backend members carry the work tag",
        );
        assert_eq!(tree_grouping.distinct_repo_count(), 2);
    }
}

