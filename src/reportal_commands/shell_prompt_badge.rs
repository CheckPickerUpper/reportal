//! Shell-integration snippet that prepends the workspace /
//! repository prompt badge to the user's existing `PS1` /
//! `PROMPT` / `prompt` function.
//!
//! Lives in a separate file from `shell_integration` so the
//! existing shell-integration script can stay untouched by the
//! badge feature — `run_initialize` concatenates the snippet
//! produced here onto the integration script for its shell, and
//! the two halves can evolve independently.
//!
//! # Coexistence with prompt frameworks
//!
//! Frameworks like powerlevel10k, starship, and oh-my-zsh themes
//! own `PROMPT` / `PS1` and rewrite it on every prompt redraw.
//! Two strategies together make rep's badge survive alongside
//! any framework, regardless of load order:
//!
//! 1. **Idempotent prepend.** A sentinel variable tracks the
//!    exact badge string we added last time; on each redraw we
//!    strip that sentinel from the current prompt (no-op if the
//!    framework already rewrote it) and prepend a freshly
//!    resolved badge.
//!
//! 2. **Run-last self-registration.** The hook moves itself to
//!    the end of the shell's prompt-hook chain every time it
//!    runs. For zsh that means re-appending to
//!    `precmd_functions`; for bash it means rewriting
//!    `PROMPT_COMMAND` so our entry is last. Whoever else
//!    registered a prompt hook still runs, but rep always runs
//!    *after* them, so the strip-and-prepend sees the
//!    framework's final prompt content and lays the badge on
//!    top.
//!
//! For `PowerShell`, we wrap the `prompt` function. Wrapping
//! composes: if a framework also wraps the function after us,
//! its wrapper calls into ours, which calls into the original.
//! No strip-and-prepend needed because each call computes fresh
//! output.

use crate::cli_args::InitializeShell;

/// @why Builds the badge-prepending snippet for the target
/// shell so `run_initialize`'s output wires an inline colored
/// badge into the prompt on every redraw and coexists with any
/// prompt framework — the emitted hook strips only bytes it
/// itself added last time and re-registers itself at the end
/// of the prompt-hook chain so the framework's redraw never
/// overwrites the badge on the next tick.
#[must_use]
pub fn prompt_badge_integration_snippet(target_shell: InitializeShell) -> String {
    let null_device = std::path::Path::new("/dev").join("null");
    match target_shell {
        InitializeShell::Bash => format!(
            r#"
_REPORTAL_LAST_BADGE_BASH=""
_reportal_prompt_badge_hook() {{
    local _b; _b=$(rep prompt --shell bash 2>{null_device})
    if [ -n "$_REPORTAL_LAST_BADGE_BASH" ] && [[ "$PS1" == "$_REPORTAL_LAST_BADGE_BASH"* ]]; then
        PS1="${{PS1#$_REPORTAL_LAST_BADGE_BASH}}"
    fi
    if [ -n "$_b" ]; then
        PS1="$_b $PS1"
        _REPORTAL_LAST_BADGE_BASH="$_b "
    else
        _REPORTAL_LAST_BADGE_BASH=""
    fi
    PROMPT_COMMAND="${{PROMPT_COMMAND//;_reportal_prompt_badge_hook/}}"
    PROMPT_COMMAND="${{PROMPT_COMMAND//_reportal_prompt_badge_hook;/}}"
    PROMPT_COMMAND="${{PROMPT_COMMAND//_reportal_prompt_badge_hook/}}"
    PROMPT_COMMAND="${{PROMPT_COMMAND:+$PROMPT_COMMAND;}}_reportal_prompt_badge_hook"
}}
PROMPT_COMMAND="${{PROMPT_COMMAND:+$PROMPT_COMMAND;}}_reportal_prompt_badge_hook"
"#,
            null_device = null_device.display(),
        ),
        InitializeShell::Zsh => format!(
            r#"
typeset -g _REPORTAL_LAST_BADGE_ZSH=""
_reportal_prompt_badge_hook() {{
    local _b
    _b=$(rep prompt --shell zsh 2>{null_device})
    if [ -n "$_REPORTAL_LAST_BADGE_ZSH" ] && [[ "$PROMPT" == "$_REPORTAL_LAST_BADGE_ZSH"* ]]; then
        PROMPT="${{PROMPT#$_REPORTAL_LAST_BADGE_ZSH}}"
    fi
    if [ -n "$_b" ]; then
        PROMPT="$_b $PROMPT"
        _REPORTAL_LAST_BADGE_ZSH="$_b "
    else
        _REPORTAL_LAST_BADGE_ZSH=""
    fi
    precmd_functions=(${{precmd_functions:#_reportal_prompt_badge_hook}} _reportal_prompt_badge_hook)
}}
autoload -Uz add-zsh-hook 2>{null_device} || true
if typeset -f add-zsh-hook >{null_device} 2>&1; then add-zsh-hook precmd _reportal_prompt_badge_hook; fi
"#,
            null_device = null_device.display(),
        ),
        InitializeShell::Powershell => String::from(
            r#"
if (-not (Get-Variable -Name '_ReportalOriginalBadgePrompt' -Scope Global -ErrorAction SilentlyContinue)) {
    $Global:_ReportalOriginalBadgePrompt = $function:global:prompt
}
function global:prompt {
    $existingPromptOutput = & $Global:_ReportalOriginalBadgePrompt
    $badgeOutput = rep prompt --shell powershell 2>$null
    if ($badgeOutput) { "$badgeOutput $existingPromptOutput" } else { $existingPromptOutput }
}
"#,
        ),
    }
}
