//! Implements `rep init <shell>`.
//!
//! Prints shell integration code to stdout so the user can wire it
//! into their rc file with a single `eval "$(rep init zsh)"` line,
//! matching the pattern used by starship, zoxide, direnv, and mise.
//! No disk writes, no profile editing, no prompts — the shell code
//! is regenerated every session and therefore never goes stale.

use crate::cli_args::InitShell;

use super::shell_integration::{bash_integration_content, powershell_integration_content};

/// Emits the shell integration script for the requested shell to
/// stdout and returns. All integration content is generated in-memory
/// from the running binary's version, which guarantees that a binary
/// update takes effect in the next shell session with no manual step.
pub fn run_init(shell: InitShell) {
    let integration_script = match shell {
        InitShell::Zsh | InitShell::Bash => bash_integration_content(),
        InitShell::Powershell => powershell_integration_content(),
    };
    print!("{integration_script}");
}
