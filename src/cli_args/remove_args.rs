//! CLI args for `rep remove`.

use clap::Args;

/// Arguments for the `rep remove` subcommand.
///
/// Takes a required alias identifying which repo to unregister
/// from the config (does not delete the repo's files on disk).
#[derive(Args)]
pub struct RemoveArgs {
    /// Alias of the repo to remove
    alias: String,
}

/// Accessor for the parsed alias.
impl RemoveArgs {
    /// The alias of the repo to unregister.
    pub fn alias(&self) -> &str {
        &self.alias
    }
}
