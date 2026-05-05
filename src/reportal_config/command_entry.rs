//! User-defined command entries stored in the `[commands.*]`
//! sections of `config.toml`.

use crate::reportal_config::shell_alias_export::ShellAliasExport;
use serde::{Deserialize, Serialize};

/// A registered command with its shell command string and description.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandEntry {
    /// The shell command to execute (e.g. "npm run dev", "cargo test").
    command: String,
    /// Human-readable description shown in the fuzzy picker.
    #[serde(default)]
    description: String,
    /// When `Enabled`, `rep init <shell>` emits a top-level shell
    /// function named after the command's config key that runs
    /// `rep run --cmd <key>` so the user can invoke the command
    /// from any shell prompt without typing `rep run` first.
    /// Opt-in because the command's name shadows whatever else
    /// the user's shell PATH already resolves to under that name.
    #[serde(default)]
    shell_alias: ShellAliasExport,
}

/// Construction and accessors for command configuration.
impl CommandEntry {
    /// @why Exposes the raw shell string `rep run` will pass to
    /// `sh -c` so callers render previews and spawn subprocesses
    /// from one source of truth without re-parsing config TOML.
    pub fn shell_command(&self) -> &str {
        &self.command
    }

    /// @why Exposes the human-readable description shown next to
    /// the command name in the fuzzy picker so `rep run` can
    /// build its labels without reaching into the struct's
    /// private fields.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// @why Exposes the opt-in export policy `rep init <shell>`
    /// reads to decide whether to emit a top-level shell function
    /// for this command so users get a direct shell command for
    /// their configured action without typing `rep run --cmd
    /// <name>` first.
    #[must_use]
    pub fn shell_alias_export(&self) -> ShellAliasExport {
        self.shell_alias
    }
}
