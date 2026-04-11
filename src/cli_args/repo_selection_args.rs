/// Shared optional alias positional arg + `--tag` flag for repo selection.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::tag_filter_args::TagFilterArgs;

/// Optional repo alias (positional) combined with a `--tag` filter flag.
///
/// Used by commands that select a single repo — jump, open, edit, web,
/// run, and ai. The alias is optional: when empty, the command presents
/// a fuzzy finder. When non-empty, it jumps directly to that repo.
#[derive(Args)]
pub struct RepoSelectionArgs {
    /// Jump directly to this alias (skip fuzzy finder)
    #[arg(default_value = "", hide_default_value = true)]
    alias: String,
    #[command(flatten)]
    tag_filter: TagFilterArgs,
}

/// Consuming conversion that splits the args into their domain-layer parts.
impl RepoSelectionArgs {
    /// Returns the alias string and a `TagFilter`, consuming self.
    ///
    /// The alias may be empty (meaning no direct selection was provided).
    /// The tag filter is converted from the CLI flag value.
    pub fn into_parts(self) -> (String, TagFilter) {
        return (self.alias, self.tag_filter.into_tag_filter());
    }
}
