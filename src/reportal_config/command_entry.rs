//! User-defined command entries stored in the `[commands.*]`
//! sections of `config.toml`.

use serde::{Deserialize, Serialize};

/// A registered command with its shell command string and description.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandEntry {
    /// The shell command to execute (e.g. "npm run dev", "cargo test").
    command: String,
    /// Human-readable description shown in the fuzzy picker.
    #[serde(default)]
    description: String,
}

/// Construction and accessors for command configuration.
impl CommandEntry {
    /// The shell command string to execute.
    pub fn shell_command(&self) -> &str {
        &self.command
    }

    /// Human-readable description of what this command does.
    pub fn description(&self) -> &str {
        &self.description
    }
}
