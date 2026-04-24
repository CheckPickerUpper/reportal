//! CLI args for `rep status`.

use clap::Args;
use crate::reportal_config::TagFilter;
use super::tag_filter_arguments::TagFilterArguments;

/// Arguments for the `rep status` subcommand.
///
/// Accepts an optional `--tag` flag to filter which repos are checked.
#[derive(Args)]
pub struct StatusArguments {
    #[command(flatten)]
    filter: TagFilterArguments,
}

/// Consuming conversion to the domain tag filter.
impl StatusArguments {
    /// Extracts the tag filter, consuming the parsed args.
    pub fn into_tag_filter(self) -> TagFilter {
        self.filter.into_tag_filter()
    }
}
