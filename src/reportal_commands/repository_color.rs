//! Emits OSC terminal personalization sequences for the current
//! working directory or a specified repository.
//!
//! Used as a shell prompt hook so terminals that open directly
//! into a repository or workspace (e.g. the VS Code integrated
//! terminal) get the right tab title and background color
//! without going through `rj` / `rjw`.
//!
//! When called without `--repo`, the CWD is resolved via
//! [`PromptIdentityResolver`], which prefers workspace
//! directories over repository paths so a user inside
//! `~/dev/workspaces/venoble/` identifies with the venoble
//! workspace's color rather than the first member's.

use crate::error::ReportalError;
use crate::reportal_config::{HexColor, ReportalConfig, TabTitle};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParameters};

use super::color_command_mode::ColorCommandMode;
use super::prompt_identity::PromptIdentityResolver;

/// Inputs the `rep color` subcommand needs to resolve which
/// terminal identity to paint.
///
/// Bundled on a named struct so future knobs (e.g. a `--title`
/// override) can be added without changing every call site,
/// matching the project-wide params-bundle convention used by
/// `JumpCommandParameters` and siblings.
pub struct ColorCommandParameters<'command_lifetime> {
    /// Alias to paint instead of the CWD-derived identity.
    ///
    /// An empty string means "use the CWD": the resolver then
    /// tries workspace directories first and falls back to
    /// registered repositories. A non-empty alias bypasses the
    /// resolver entirely and looks up the repository registry
    /// directly, which is the path used by the `rj` / `ro`
    /// shell helpers that already know which alias was selected.
    pub repository_alias: &'command_lifetime str,
    /// How the command was invoked, controlling stdout output
    /// and what happens when no identity matches.
    ///
    /// `PromptHook` prints the title to stdout (so shell
    /// integrations can set the OS window title via native
    /// APIs) and leaves the previous identity intact on
    /// no-match. `Explicit` stays silent on stdout and resets
    /// the tab strip to the terminal default on no-match.
    pub mode: ColorCommandMode,
}

/// Pre-OSC-emit view of a terminal identity: a title and an
/// accent color, or no accent for the default-reset case.
///
/// Bundled on a named struct so the two lookup paths (CWD match
/// and explicit repository alias) each return the same shape,
/// and the single OSC-emission block downstream handles both
/// uniformly. Private to this file because it only bridges the
/// pre-existing [`TabColorAction`] / [`TerminalIdentity`] pair.
struct ResolvedTerminalIdentity {
    /// The tab / window title after resolving title > alias.
    display_title: String,
    /// The accent color, or `None` when the identity should
    /// reset the tab strip to the terminal's default.
    accent_color: Option<HexColor>,
}

/// Construction and OSC emission for the pre-emit identity.
impl ResolvedTerminalIdentity {
    /// @why Builds the identity for the `--repo <alias>` path so
    /// the explicit-alias branch produces the same shape as the
    /// workspace / repository CWD resolver and a single emit
    /// block downstream handles both inputs without a second
    /// branching match.
    fn from_repository_alias(
        loaded_configuration: &ReportalConfig,
        requested_alias: &str,
    ) -> Result<Self, ReportalError> {
        let found_repository = loaded_configuration.get_repo(requested_alias)?;
        let display_title = match found_repository.tab_title() {
            TabTitle::Custom(custom_title) => custom_title.clone(),
            TabTitle::UseAlias => requested_alias.to_owned(),
        };
        let accent_color = found_repository.repo_color().themed_hex_color().cloned();
        Ok(Self {
            display_title,
            accent_color,
        })
    }

    /// @why Writes the OSC tab-title and tab-color sequences for
    /// this identity through the shared console writer so the
    /// shell prompt hook and the explicit-alias call share the
    /// same output path and cannot drift in what they send to
    /// the terminal.
    fn emit_to_console(&self, invocation_mode: &ColorCommandMode) {
        if matches!(invocation_mode, ColorCommandMode::PromptHook) {
            terminal_style::write_stdout(&self.display_title);
        }
        let tab_color_action = self.accent_color.as_ref().map_or(
            TabColorAction::Reset,
            |hex_color| TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence()),
        );
        let terminal_identity = TerminalIdentity::new(TerminalIdentityParameters {
            resolved_title: self.display_title.clone(),
            tab_color_action,
        });
        terminal_style::emit_terminal_identity_to_console(&terminal_identity);
    }
}

/// @why Emits the terminal's tab-strip color and title for the
/// CWD (workspace-first, repository-second) or for an explicit
/// `--repo <alias>` override, so a terminal that opened directly
/// inside a repository gets the right identity without going
/// through the `rj` / `rjw` shell helpers. A CWD that matches
/// neither a workspace nor a repository resets the tab strip
/// (explicit call) or leaves the previous identity intact
/// (prompt hook).
///
/// # Errors
///
/// Returns any error from configuration load, CWD read, or
/// repository lookup.
pub fn run_color(color_params: &ColorCommandParameters<'_>) -> Result<(), ReportalError> {
    let loaded_configuration = ReportalConfig::load_or_initialize()?;

    let resolved_identity = if color_params.repository_alias.is_empty() {
        let identity_resolver = PromptIdentityResolver::for_configuration(&loaded_configuration);
        identity_resolver
            .resolve_from_current_directory()?
            .map(|prompt_identity| ResolvedTerminalIdentity {
                display_title: prompt_identity.display_label,
                accent_color: prompt_identity.accent_color,
            })
    } else {
        Some(ResolvedTerminalIdentity::from_repository_alias(
            &loaded_configuration,
            color_params.repository_alias,
        )?)
    };

    match resolved_identity {
        Some(identity) => identity.emit_to_console(&color_params.mode),
        None => match color_params.mode {
            ColorCommandMode::Explicit => {
                terminal_style::write_to_console(terminal_style::osc_reset_tab_color_sequence());
            }
            ColorCommandMode::PromptHook => {}
        },
    }

    Ok(())
}
