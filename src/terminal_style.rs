/// Centralized color palette and styling for all RePortal terminal output.
///
/// Uses the Nightfall Candy palette from the Roblox Studio syntax theme.
/// All colors are applied via owo-colors for zero-allocation styling.

use owo_colors::{OwoColorize, Style};

/// Blue used for repo aliases and labels.
pub const ALIAS_STYLE: Style = Style::new().blue().bold();

/// Dim gray for paths, secondary info.
pub const PATH_STYLE: Style = Style::new().dimmed();

/// Green for success indicators and existence checks.
pub const SUCCESS_STYLE: Style = Style::new().green();

/// Red for failure indicators and error messages.
pub const FAILURE_STYLE: Style = Style::new().red();

/// Yellow for warnings and tag display.
pub const TAG_STYLE: Style = Style::new().dimmed().italic();

/// Cyan for prompts and labels.
pub const LABEL_STYLE: Style = Style::new().cyan();

/// Bold white for emphasis.
pub const EMPHASIS_STYLE: Style = Style::new().bold();

/// Prints an error message with a red "Error:" prefix.
pub fn print_error(error_message: &str) {
    eprintln!("{} {}", "Error:".style(FAILURE_STYLE), error_message);
}

/// Prints a success message with a green prefix.
pub fn print_success(success_message: &str) {
    println!("{} {}", ">>".style(SUCCESS_STYLE), success_message);
}

/// Returns the OSC 2 escape sequence that sets the terminal tab title.
pub fn osc_tab_title_sequence(title_text: &str) -> String {
    return format!("\x1b]2;{title_text}\x07");
}

/// Returns the OSC 104;264 escape sequence that resets the Windows Terminal
/// tab color strip (FRAME_BACKGROUND) back to the profile default.
pub fn osc_reset_tab_color_sequence() -> &'static str {
    return "\x1b]104;264\x07";
}

/// What to do with the terminal tab color strip when jumping to a repo.
pub enum TabColorAction {
    /// Emit OSC 6 sequences to set the tab color to specific RGB values.
    SetColor(String),
    /// Emit OSC 6 reset to restore the terminal's default tab color.
    Reset,
}

/// The resolved tab title and tab color action for a single jump/open.
/// Constructed from a repo's config + any CLI overrides, then passed
/// to an emit function to write the OSC sequences.
pub struct TerminalIdentity {
    resolved_title: String,
    tab_color_action: TabColorAction,
}

/// Construction and accessors for terminal identity data.
impl TerminalIdentity {
    /// Builds a terminal identity from the resolved title string and
    /// tab color action. Called after the title fallback chain
    /// (flag > config > alias) has been resolved.
    pub fn new(identity_params: TerminalIdentityParams) -> Self {
        return Self {
            resolved_title: identity_params.resolved_title,
            tab_color_action: identity_params.tab_color_action,
        };
    }

    /// The final tab title after resolving the flag > config > alias chain.
    pub fn resolved_title(&self) -> &str {
        &self.resolved_title
    }

    /// Whether to set a specific tab color or reset to the terminal default.
    pub fn tab_color_action(&self) -> &TabColorAction {
        &self.tab_color_action
    }
}

/// Parameters for constructing a `TerminalIdentity`.
pub struct TerminalIdentityParams {
    /// The final tab title after resolving flag > config > alias fallback.
    pub resolved_title: String,
    /// Whether to set a tab color or reset to default.
    pub tab_color_action: TabColorAction,
}

/// Emits OSC sequences to stderr for tab title + tab color strip.
/// Used by jump/open where stdout carries the path for the shell function.
/// Skips emission if stderr is not a TTY.
pub fn emit_terminal_identity_to_stderr(identity: &TerminalIdentity) {
    if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        return;
    }
    eprint!("{}", osc_tab_title_sequence(identity.resolved_title()));
    match identity.tab_color_action() {
        TabColorAction::SetColor(osc_sequence) => eprint!("{osc_sequence}"),
        TabColorAction::Reset => eprint!("{}", osc_reset_tab_color_sequence()),
    }
}

/// Emits OSC sequences to stdout for tab title + tab color strip.
/// Used by the `color` subcommand where nothing captures stdout.
/// Skips emission if stdout is not a TTY.
pub fn emit_terminal_identity_to_stdout(identity: &TerminalIdentity) {
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return;
    }
    print!("{}", osc_tab_title_sequence(identity.resolved_title()));
    match identity.tab_color_action() {
        TabColorAction::SetColor(osc_sequence) => print!("{osc_sequence}"),
        TabColorAction::Reset => print!("{}", osc_reset_tab_color_sequence()),
    }
}
