//! Implements `rep prompt --shell <shell>`.
//!
//! Resolves the current working directory to its workspace or
//! repository identity (via `PromptIdentityResolver`) and prints
//! a single ANSI-SGR colored badge to stdout, wrapped with the
//! target shell's non-printing-escape markers so the sequence
//! can be inlined inside a `PS1` / `$PROMPT` / `prompt` function
//! without breaking cursor math on long command lines.
//!
//! When the CWD matches neither a workspace directory nor a
//! registered repository path, the command prints nothing — the
//! shell integration script is expected to treat empty output as
//! "no badge this prompt" and render its normal PS1 unchanged.

use crate::cli_args::PromptShell;
use crate::error::ReportalError;
use crate::reportal_commands::prompt_identity::{PromptIdentity, PromptIdentityResolver};
use crate::reportal_config::{HexColor, ReportalConfig};
use crate::terminal_style;

/// Non-printing-escape wrapper pair for a single shell, used to
/// fence SGR sequences inside a prompt string without letting the
/// shell count them as visible characters.
///
/// Storing the pair on a named struct (rather than a 2-element
/// tuple) keeps the opening/closing direction named at the call
/// site and lets the shell-specific picker and the wrapping
/// helper live as methods here, keeping this file's free
/// function count to exactly one (the public command entry).
struct NonPrintingEscapeWrapper {
    /// Sequence placed immediately before a non-printing byte run.
    opening_marker: &'static str,
    /// Sequence placed immediately after a non-printing byte run.
    closing_marker: &'static str,
}

/// Shell-aware construction and SGR wrapping for the
/// non-printing-escape marker pair.
impl NonPrintingEscapeWrapper {
    /// Picks the marker pair for the requested shell: bash uses
    /// `\[` / `\]`, zsh uses `%{` / `%}`, and `PowerShell`
    /// emits raw SGR because `PSReadLine` already handles cursor
    /// math inside a prompt function.
    fn for_shell(requested_shell: PromptShell) -> Self {
        match requested_shell {
            PromptShell::Bash => Self {
                opening_marker: "\\[",
                closing_marker: "\\]",
            },
            PromptShell::Zsh => Self {
                opening_marker: "%{",
                closing_marker: "%}",
            },
            PromptShell::Powershell => Self {
                opening_marker: "",
                closing_marker: "",
            },
        }
    }

    /// Renders a full colored badge: opening-wrap, SGR
    /// foreground, closing-wrap, then the label, then
    /// opening-wrap, SGR reset, closing-wrap. The label itself
    /// sits outside the wrapper so the shell counts its visible
    /// characters correctly.
    fn render_colored_label(
        &self,
        display_label: &str,
        accent_hex_color: &HexColor,
    ) -> Result<String, ReportalError> {
        let (red_channel, green_channel, blue_channel) = accent_hex_color.as_rgb_bytes()?;
        let sgr_foreground =
            format!("\x1b[38;2;{red_channel};{green_channel};{blue_channel}m");
        let sgr_reset = "\x1b[0m";
        Ok(format!(
            "{open}{sgr_foreground}{close}{display_label}{open}{sgr_reset}{close}",
            open = self.opening_marker,
            close = self.closing_marker,
        ))
    }

    /// Renders the badge for an identity whose accent color is
    /// either set or absent. When absent, the label is returned
    /// unwrapped so the shell's current prompt color shows
    /// through.
    fn render_identity(
        &self,
        resolved_identity: &PromptIdentity,
    ) -> Result<String, ReportalError> {
        let Some(hex_color) = resolved_identity.accent_color.as_ref() else {
            return Ok(resolved_identity.display_label.clone());
        };
        self.render_colored_label(&resolved_identity.display_label, hex_color)
    }
}

/// @why Prints a shell-wrapped ANSI-colored prompt badge for the
/// current working directory so `PS1` / `$PROMPT` / the pwsh
/// `prompt` function can inline the badge without the shell
/// miscounting escape bytes and corrupting the cursor on long
/// command lines. Silent when the CWD is outside every
/// registered workspace and repository.
///
/// # Errors
///
/// Returns any error surfaced by configuration load or by the
/// prompt identity resolver — both bubble up so a broken
/// configuration fails loudly on the first prompt redraw rather
/// than silently stripping the badge.
pub fn run_prompt(requested_shell: PromptShell) -> Result<(), ReportalError> {
    let loaded_configuration = ReportalConfig::load_or_initialize()?;
    let identity_resolver = PromptIdentityResolver::for_configuration(&loaded_configuration);
    let Some(resolved_identity) = identity_resolver.resolve_from_current_directory()? else {
        return Ok(());
    };
    let escape_wrapper = NonPrintingEscapeWrapper::for_shell(requested_shell);
    let rendered_badge = escape_wrapper.render_identity(&resolved_identity)?;
    terminal_style::write_stdout(&rendered_badge);
    Ok(())
}
