//! CLI args for `rep list`.

use super::tag_filter_arguments::TagFilterArguments;
use super::workspace_filter_arguments::WorkspaceFilterArguments;
use crate::reportal_config::{TagFilter, WorkspaceFilter};
use clap::Args;

/// Arguments for the `rep list` subcommand.
///
/// Accepts `--tag` and `--workspace` as orthogonal filters.
/// Both are optional and compose as an AND intersection when
/// specified together — the rendered output includes only repos
/// that match both filters.
#[derive(Args)]
pub struct ListArguments {
    /// Optional `--tag` flag for tag-based filtering.
    #[command(flatten)]
    tag_filter: TagFilterArguments,
    /// Optional `--workspace` flag for workspace-membership
    /// filtering, orthogonal to `--tag`.
    #[command(flatten)]
    workspace_filter: WorkspaceFilterArguments,
}

/// Consuming accessors for `ListArguments`.
impl ListArguments {
    /// Extracts both filters as a named parts struct so the
    /// dispatcher receives exactly one argument, which the
    /// project's argument rules require for handler entry points.
    #[must_use]
    pub fn into_filter_parts(self) -> ListArgumentsFilterParts {
        ListArgumentsFilterParts {
            tag_filter: self.tag_filter.into_tag_filter(),
            workspace_filter: self.workspace_filter.into_workspace_filter(),
        }
    }
}

/// Owned named-field result of `ListArguments::into_filter_parts`.
///
/// Returned instead of a bare tuple so call sites never confuse
/// the two filter enums when dispatching to the listing handler.
pub struct ListArgumentsFilterParts {
    /// Resolved tag filter for the listing.
    pub(super) tag_filter: TagFilter,
    /// Resolved workspace-membership filter for the listing.
    pub(super) workspace_filter: WorkspaceFilter,
}

/// Accessors for the destructured filter parts.
impl ListArgumentsFilterParts {
    /// The resolved tag filter for this listing invocation.
    #[must_use]
    pub fn tag_filter(&self) -> &TagFilter {
        &self.tag_filter
    }

    /// The resolved workspace-membership filter for this
    /// listing invocation.
    #[must_use]
    pub fn workspace_filter(&self) -> &WorkspaceFilter {
        &self.workspace_filter
    }
}
