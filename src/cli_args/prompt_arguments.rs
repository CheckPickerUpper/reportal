//! CLI arguments for `rep prompt`.

use clap::{Args, ValueEnum};

/// Arguments for the `rep prompt` subcommand.
///
/// `rep prompt` emits a single colored badge line to stdout,
/// wrapped with the non-printing-escape markers of the target
/// shell so the sequence can be inlined inside a PS1 / prompt
/// function without breaking cursor math on long command lines.
///
/// The shell choice is required (no inference from the
/// environment) so the output contract is unambiguous: the same
/// binary running inside the same terminal still produces bash-
/// compatible output when `--shell bash` is passed and zsh-
/// compatible output when `--shell zsh` is passed.
#[derive(Args)]
pub struct PromptArguments {
    /// Shell whose non-printing-escape wrappers the badge is formatted for
    #[arg(long, value_enum)]
    shell: PromptShell,
}

/// Shells for which `rep prompt` can emit a wrapped badge.
///
/// A separate enum from `InitializeShell` because the set of shells
/// that accept a prompt-badge wrapper is not necessarily the
/// same as the set for which `rep init` generates integration
/// code; keeping the two enums independent means a future shell
/// can be added on one axis without the other.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PromptShell {
    /// Bash — wraps SGR with the `\[` / `\]` non-printing markers.
    Bash,
    /// Zsh — wraps SGR with the `%{` / `%}` non-printing markers.
    Zsh,
    /// `PowerShell` — emits raw SGR; `PSReadLine` handles cursor math.
    Powershell,
}

/// Consuming conversion that splits into domain-layer parts.
impl PromptArguments {
    /// @why Hands the parsed shell choice to the command layer
    /// so the command never imports clap types and the clap
    /// wrapper can be swapped without touching the emit logic.
    #[must_use]
    pub fn into_shell(self) -> PromptShell {
        self.shell
    }
}
