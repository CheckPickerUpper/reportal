//! Implements `rep init <shell>`.
//!
//! Prints shell integration code to stdout so the user can wire it
//! into their rc file with a single `eval "$(rep init zsh)"` line,
//! matching the pattern used by starship, zoxide, direnv, and mise.
//! No disk writes, no profile editing, no prompts — the shell code
//! is regenerated every session and therefore never goes stale.

use crate::cli_args::InitializeShell;

use super::shell_integration::{bash_integration_content, powershell_integration_content};
use super::shell_prompt_badge::prompt_badge_integration_snippet;

/// @why Emits the shell integration script — chrome hooks plus
/// the prompt-badge snippet — for the requested shell to stdout
/// so the user wires a single `eval "$(rep init <shell>)"` line
/// into their rc file and gets both the tab / window title
/// chrome and the inline PS1 badge without further manual setup.
pub fn run_initialize(shell: InitializeShell) {
    let base_integration_script = match shell {
        InitializeShell::Zsh | InitializeShell::Bash => bash_integration_content(),
        InitializeShell::Powershell => powershell_integration_content(),
    };
    let badge_snippet = prompt_badge_integration_snippet(shell);
    print!("{base_integration_script}{badge_snippet}");
}
