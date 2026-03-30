/// Emits OSC terminal identity sequences (tab title + color) for a selected repo.

use crate::reportal_config::{RepoColor, RepoEntry, TabTitle};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};

/// Parameters for emitting terminal identity (tab title + color).
pub struct TerminalIdentityEmitParams<'a> {
    /// The selected repo's alias (used as fallback title).
    pub selected_alias: &'a str,
    /// The selected repo's config entry (provides title and color fields).
    pub selected_repo: &'a RepoEntry,
    /// If non-empty, overrides the repo's configured title for this session.
    pub title_override: &'a str,
}

/// Resolves a repo's tab title and color, then emits OSC sequences
/// directly to the console handle (CONOUT$ / /dev/tty).
///
/// Title precedence: `title_override` > repo's `title` field > alias.
/// Color: repo's `color` field if set, otherwise resets to terminal default.
pub fn emit_repo_terminal_identity(identity_params: TerminalIdentityEmitParams<'_>) {
    let resolved_title = match identity_params.title_override.is_empty() {
        false => identity_params.title_override.to_string(),
        true => match identity_params.selected_repo.tab_title() {
            TabTitle::Custom(custom_title) => custom_title.to_string(),
            TabTitle::UseAlias => identity_params.selected_alias.to_string(),
        },
    };

    let tab_color_action = match identity_params.selected_repo.repo_color() {
        RepoColor::Themed(hex_color) => {
            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
        }
        RepoColor::ResetToDefault => TabColorAction::Reset,
    };

    let identity = TerminalIdentity::new(TerminalIdentityParams {
        resolved_title,
        tab_color_action,
    });
    terminal_style::emit_terminal_identity_to_console(&identity);
}
