//! CLI args for `rep color`.

use clap::Args;
use crate::reportal_commands::ColorCommandMode;

/// Arguments for the `rep color` subcommand.
///
/// Controls how terminal identity (tab title + background color) is
/// resolved and emitted. The `--mode` flag selects between prompt-hook
/// behavior (prints title to stdout, preserves state on no-match) and
/// explicit behavior (silent stdout, resets color on no-match).
#[derive(Args)]
pub struct ColorArgs {
    /// Look up this repo by alias instead of matching the current directory
    #[arg(long, default_value = "", hide_default_value = true)]
    repo: String,
    /// Invocation mode: prompt-hook (for shell integration) or explicit (default)
    #[arg(long, value_enum, default_value_t = ColorCommandMode::Explicit)]
    mode: ColorCommandMode,
}

/// Consuming conversion that splits into domain-layer parts.
impl ColorArgs {
    /// Returns (`repo_alias`, mode), consuming self.
    pub fn into_parts(self) -> (String, ColorCommandMode) {
        (self.repo, self.mode)
    }
}
