//! Lists registered repos grouped by workspace membership.

use crate::cli_args::ListArgsFilterParts;
use crate::error::ReportalError;
use crate::reportal_commands::repo_listing_renderer::RepoListingRenderer;
use crate::reportal_commands::repo_tree_grouping::{RepoTreeGrouping, RepoTreeGroupingParams};
use crate::reportal_config::ReportalConfig;

/// Prints a workspace-grouped listing of registered repos.
///
/// Workspaces become the tree root because they answer "what am
/// I working on" (an explicit user grouping), while tags answer
/// "what kind of repo is this" (a classification). The two axes
/// are orthogonal and compose as an AND intersection when both
/// filters are specified.
///
/// Repos that belong to zero workspaces land in a synthetic
/// "(unassigned)" section at the bottom so no repo is hidden
/// from the listing. Multi-workspace repos render under every
/// containing workspace section because hiding multi-membership
/// would silently suppress user-declared relationships.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] when the
/// workspace filter names an unknown workspace (a silent empty
/// output would mask typos), or config I/O errors from the
/// underlying load path.
pub fn run_list(filter_parts: &ListArgsFilterParts) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let tree_grouping = RepoTreeGrouping::build(&RepoTreeGroupingParams {
        loaded_config: &loaded_config,
        tag_filter: filter_parts.tag_filter(),
        workspace_filter: filter_parts.workspace_filter(),
    })?;
    let listing_renderer = RepoListingRenderer::for_tree(&tree_grouping);
    listing_renderer.render(filter_parts)
}
