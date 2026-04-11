/// CLI args for `rep list`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::tag_filter_args::TagFilterArgs;

/// Arguments for the `rep list` subcommand.
///
/// Accepts an optional `--tag` flag to filter the displayed repos.
#[derive(Args)]
pub struct ListArgs {
    #[command(flatten)]
    tag_filter: TagFilterArgs,
}

/// Consuming conversion to the domain tag filter.
impl ListArgs {
    /// Extracts the tag filter, consuming the parsed args.
    pub fn into_tag_filter(self) -> TagFilter {
        return self.tag_filter.into_tag_filter();
    }
}
