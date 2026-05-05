//! Implements `rep init <shell>`.
//!
//! Prints shell integration code to stdout so the user can wire it
//! into their rc file with a single `eval "$(rep init zsh)"` line,
//! matching the pattern used by starship, zoxide, direnv, and mise.
//! No disk writes, no profile editing, no prompts — the shell code
//! is regenerated every session and therefore never goes stale.

use crate::cli_args::InitializeShell;
use crate::reportal_config::ReportalConfig;

use super::shell_alias_emit::{
    self, ShellAliasEmissionParameters,
};
use super::shell_integration::{bash_integration_content, powershell_integration_content};
use super::shell_prompt_badge::prompt_badge_integration_snippet;

/// @why Emits the shell integration script — chrome hooks plus
/// the prompt-badge snippet plus the per-config alias block — for
/// the requested shell to stdout so the user wires a single
/// `eval "$(rep init <shell>)"` line into their rc file and gets
/// the tab / window title chrome, the inline PS1 badge, and any
/// opted-in shell aliases without further manual setup.
pub fn run_initialize(shell: InitializeShell) {
    let base_integration_script = match shell {
        InitializeShell::Zsh | InitializeShell::Bash => bash_integration_content(),
        InitializeShell::Powershell => powershell_integration_content(),
    };
    let badge_snippet = prompt_badge_integration_snippet(shell);
    let alias_snippet = build_alias_snippet_or_empty_on_failure(shell);
    print!("{base_integration_script}{badge_snippet}{alias_snippet}");
}

/// Renders the per-config alias block, returning the empty string
/// if the configuration cannot be loaded. Shell init runs from
/// the user's rc file on every new shell, so a config-load
/// failure here must never break the shell startup — the base
/// integration still emits and the alias block is silently
/// omitted.
fn build_alias_snippet_or_empty_on_failure(shell: InitializeShell) -> String {
    match ReportalConfig::load_or_initialize() {
        Ok(loaded_configuration) => {
            shell_alias_emit::shell_alias_export_snippet(
                &ShellAliasEmissionParameters {
                    target_shell: shell,
                    configuration: &loaded_configuration,
                },
            )
        }
        Err(_load_error_swallowed_to_keep_shell_init_safe) => String::new(),
    }
}
