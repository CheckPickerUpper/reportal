/// CLI args for `rep status`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::tag_filter_args::TagFilterArgs;

/// Arguments for the `rep status` subcommand.
///
/// Accepts an optional `--tag` flag to filter which repos are checked.
#[derive(Args)]
pub struct StatusArgs {
    #[command(flatten)]
    filter: TagFilterArgs,
}

/// Consuming conversion to the domain tag filter.
impl StatusArgs {
    /// Extracts the tag filter, consuming the parsed args.
    pub fn into_tag_filter(self) -> TagFilter {
        return self.filter.into_tag_filter();
    }
}
