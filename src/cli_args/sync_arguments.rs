//! CLI args for `rep sync`.

use super::tag_filter_arguments::TagFilterArguments;
use crate::reportal_config::TagFilter;
use clap::Args;

/// Arguments for the `rep sync` subcommand.
///
/// Accepts an optional `--tag` flag to filter which repos are synced.
#[derive(Args)]
pub struct SyncArguments {
    #[command(flatten)]
    filter: TagFilterArguments,
}

/// Consuming conversion to the domain tag filter.
impl SyncArguments {
    /// Extracts the tag filter, consuming the parsed args.
    pub fn into_tag_filter(self) -> TagFilter {
        self.filter.into_tag_filter()
    }
}
