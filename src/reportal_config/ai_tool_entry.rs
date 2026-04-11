//! AI coding CLI tool registry entries stored in the `[ai_tools.*]`
//! sections of `config.toml`.

use serde::{Deserialize, Serialize};

/// A registered AI coding CLI tool with its launch command and arguments.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AiToolEntry {
    /// The executable command to run (e.g. "claude", "codex").
    command: String,
    /// Additional arguments passed to the command on every launch.
    #[serde(default)]
    args: Vec<String>,
}

/// Construction and accessors for AI tool configuration.
impl AiToolEntry {
    /// Registers an AI tool with the given executable and no default arguments.
    /// Use this for tools like `claude` or `codex` that need no extra flags.
    /// Additional args can be added per-tool in the TOML config's `args` array.
    pub fn with_executable(executable_command: String) -> Self {
        Self {
            command: executable_command,
            args: Vec::new(),
        }
    }


    /// The executable name used to spawn the AI CLI process.
    pub fn cli_command(&self) -> &str {
        &self.command
    }

    /// Extra arguments appended after the executable on every launch.
    pub fn launch_args(&self) -> &[String] {
        &self.args
    }
}
