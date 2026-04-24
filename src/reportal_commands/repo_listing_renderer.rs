//! Rendering surface for the workspace-grouped `rep list` output.
//!
//! Grouping logic lives in `repo_tree_grouping.rs`; this file
//! owns the terminal output for an already-built tree. Splitting
//! the two responsibilities keeps the grouping unit-testable
//! without a terminal, and keeps the rendering free to change
//! its output format without touching the tree-building rules.

use crate::cli_args::ListArgumentsFilterParts;
use crate::error::ReportalError;
use crate::reportal_commands::repo_tree_grouping::RepoTreeGrouping;
use crate::reportal_commands::repo_tree_workspace_section::WorkspaceSection;
use crate::reportal_config::{RepoEntry, TagFilter, WorkspaceFilter};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Renders a built `RepoTreeGrouping` to stdout in the
/// workspace-grouped layout.
///
/// Holds a borrowed reference to the tree so rendering and tree
/// building are separable: tests can build trees without a
/// terminal, and the render format can evolve without altering
/// the invariants encoded in the tree-building pass.
pub struct RepoListingRenderer<'tree> {
    /// The grouping to render; borrowed so ownership stays with
    /// the caller and the renderer can be dropped as soon as the
    /// render pass completes.
    tree_grouping: &'tree RepoTreeGrouping<'tree>,
}

/// Construction and rendering methods for `RepoListingRenderer`.
impl<'tree> RepoListingRenderer<'tree> {
    /// Builds a renderer for the given tree. The tree must be
    /// fully constructed before this call because the renderer
    /// only reads from it.
    #[must_use]
    pub fn for_tree(tree_grouping: &'tree RepoTreeGrouping<'tree>) -> Self {
        Self { tree_grouping }
    }

    /// Renders the tree to stdout, including the empty-state
    /// message when the tree has no sections and no unassigned
    /// rows. The filter parts are consulted only for the
    /// empty-state message so the user sees which filters are
    /// active when the output is empty.
    ///
    /// # Errors
    ///
    /// Returns any error surfaced by the per-repo color swatch
    /// resolution path.
    pub fn render(&self, filter_parts: &ListArgumentsFilterParts) -> Result<(), ReportalError> {
        if self.tree_grouping.is_empty() {
            Self::render_empty_state(filter_parts);
            return Ok(());
        }

        terminal_style::write_stdout("\n");
        terminal_style::write_stdout(&format!(
            "  {}\n\n",
            "RePortal".style(terminal_style::EMPHASIS_STYLE),
        ));

        for workspace_section in self.tree_grouping.workspace_sections() {
            Self::render_workspace_section(workspace_section)?;
        }

        if !self.tree_grouping.unassigned_repos().is_empty() {
            terminal_style::write_stdout(&format!(
                "  {}\n\n",
                "(unassigned)".style(terminal_style::EMPHASIS_STYLE),
            ));
            for repo_pair in self.tree_grouping.unassigned_repos() {
                Self::render_repo_leaf(repo_pair)?;
            }
        }

        terminal_style::write_stdout(&format!(
            "  {} repos total\n\n",
            self.tree_grouping
                .distinct_repo_count()
                .style(terminal_style::EMPHASIS_STYLE),
        ));
        Ok(())
    }

    /// Renders an empty-state message describing which filters
    /// are active so the user can tell whether the config is
    /// empty or the current filter combination rejects every
    /// repo. Takes the filter parts directly (no `self`) because
    /// the message content is derived entirely from the filters,
    /// not from the tree the renderer wraps.
    fn render_empty_state(filter_parts: &ListArgumentsFilterParts) {
        let empty_state_message = match (
            filter_parts.tag_filter(),
            filter_parts.workspace_filter(),
        ) {
            (TagFilter::All, WorkspaceFilter::All) => {
                "No repos registered. Use 'reportal add <path>' to add one.".to_owned()
            }
            (TagFilter::ByTag(target_tag), WorkspaceFilter::All) => {
                format!("No repos found with tag '{target_tag}'")
            }
            (TagFilter::All, WorkspaceFilter::ByName(target_workspace)) => {
                format!("No repos found in workspace '{target_workspace}'")
            }
            (TagFilter::ByTag(target_tag), WorkspaceFilter::ByName(target_workspace)) => {
                format!(
                    "No repos found in workspace '{target_workspace}' with tag '{target_tag}'"
                )
            }
        };
        terminal_style::write_stdout(&format!("{empty_state_message}\n"));
    }

    /// Renders one workspace section header, description, and
    /// the indented member rows. Takes the section as its only
    /// parameter because the render is a pure transformation of
    /// the section's own data.
    fn render_workspace_section(
        workspace_section: &WorkspaceSection<'_>,
    ) -> Result<(), ReportalError> {
        let uppercase_name = workspace_section.workspace_name().to_uppercase();
        terminal_style::write_stdout(&format!(
            "  {}\n",
            uppercase_name.style(terminal_style::EMPHASIS_STYLE),
        ));
        let section_description = workspace_section.workspace_entry().description();
        if !section_description.is_empty() {
            terminal_style::write_stdout(&format!(
                "  {}\n",
                section_description.style(terminal_style::TAG_STYLE),
            ));
        }
        terminal_style::write_stdout("\n");
        for repo_pair in workspace_section.member_repos() {
            Self::render_repo_leaf(repo_pair)?;
        }
        Ok(())
    }

    /// Renders one repo row at leaf indentation level with its
    /// alias, color swatch, path, description, tags, and
    /// directory-exists marker. Takes the full
    /// `(alias, entry)` tuple as one parameter because the
    /// project's argument rules cap non-`self` parameters at one
    /// per method.
    fn render_repo_leaf(repo_pair: &(&str, &RepoEntry)) -> Result<(), ReportalError> {
        let (repository_alias, repo_entry) = *repo_pair;
        let directory_exists = repo_entry.resolved_path().exists();
        let swatch_style = terminal_style::swatch_style_for_repo_color(repo_entry.repo_color())?;
        let uppercase_alias = repository_alias.to_uppercase();
        terminal_style::write_stdout(&format!(
            "     {} {}\n",
            "██".style(swatch_style),
            uppercase_alias.style(terminal_style::ALIAS_STYLE),
        ));
        terminal_style::write_stdout(&format!(
            "        {} {}\n",
            "Path:".style(terminal_style::LABEL_STYLE),
            repo_entry.raw_path().style(terminal_style::PATH_STYLE),
        ));
        if !repo_entry.description().is_empty() {
            terminal_style::write_stdout(&format!(
                "        {} {}\n",
                "Desc:".style(terminal_style::LABEL_STYLE),
                repo_entry.description(),
            ));
        }
        if !repo_entry.tags().is_empty() {
            let formatted_tags = repo_entry.tags().join(", ");
            terminal_style::write_stdout(&format!(
                "        {} {}\n",
                "Tags:".style(terminal_style::LABEL_STYLE),
                formatted_tags.style(terminal_style::TAG_STYLE),
            ));
        }
        let found_label = if directory_exists {
            "yes".style(terminal_style::SUCCESS_STYLE).to_string()
        } else {
            "no".style(terminal_style::FAILURE_STYLE).to_string()
        };
        terminal_style::write_stdout(&format!(
            "        {} {}\n",
            "Found:".style(terminal_style::LABEL_STYLE),
            found_label,
        ));
        terminal_style::write_stdout("\n");
        Ok(())
    }
}
