//! CLI args for `rep init`.

use clap::{Args, ValueEnum};

/// Arguments for the `rep init` subcommand.
///
/// `rep init` prints shell integration code to stdout for the
/// requested shell. Users wire it into their rc file with a single
/// `eval "$(rep init zsh)"` line, matching the pattern used by
/// starship, zoxide, direnv, and mise. The binary never writes
/// integration files to disk and never prompts the user.
#[derive(Args)]
pub struct InitArgs {
    /// Shell to generate integration code for
    shell: InitShell,
}

/// Shells that `rep init` can generate integration code for.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum InitShell {
    /// Zsh (prints POSIX-compatible shell functions).
    Zsh,
    /// Bash (prints POSIX-compatible shell functions).
    Bash,
    /// `PowerShell` (prints `PowerShell` function definitions).
    Powershell,
}

/// Accessor for the parsed shell choice.
impl InitArgs {
    /// The shell whose integration code should be emitted.
    pub fn shell(&self) -> InitShell {
        self.shell
    }
}
