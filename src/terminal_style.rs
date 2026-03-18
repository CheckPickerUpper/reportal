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

/// Returns the OSC 111 escape sequence that resets the terminal background
/// to its configured default.
pub fn osc_reset_background_sequence() -> &'static str {
    return "\x1b]111\x07";
}

/// What to do with the terminal background when jumping to a repo.
pub enum BackgroundAction {
    /// Emit an OSC 11 sequence to set a specific background color.
    SetColor(String),
    /// Emit an OSC 111 sequence to reset to the terminal's default.
    Reset,
}

/// The resolved tab title and background action for a single jump/open.
/// Constructed from a repo's config + any CLI overrides, then passed
/// to `emit_terminal_identity` to write the OSC sequences.
pub struct TerminalIdentity {
    resolved_title: String,
    background_action: BackgroundAction,
}

/// Construction and accessors for terminal identity data.
impl TerminalIdentity {
    /// Builds a terminal identity from the resolved title string and
    /// background action. Called after the title fallback chain
    /// (flag > config > alias) has been resolved.
    pub fn new(identity_params: TerminalIdentityParams) -> Self {
        return Self {
            resolved_title: identity_params.resolved_title,
            background_action: identity_params.background_action,
        };
    }

    /// The final tab title after resolving the flag > config > alias chain.
    pub fn resolved_title(&self) -> &str {
        &self.resolved_title
    }

    /// Whether to set a specific color or reset to the terminal default.
    pub fn background_action(&self) -> &BackgroundAction {
        &self.background_action
    }
}

/// Parameters for constructing a `TerminalIdentity`.
pub struct TerminalIdentityParams {
    /// The final tab title after resolving flag > config > alias fallback.
    pub resolved_title: String,
    /// Whether to set a color or reset the background.
    pub background_action: BackgroundAction,
}

/// Emits the appropriate OSC sequences to stderr for a repo's terminal
/// personalization (tab title + background color). Skips emission if
/// stderr is not a TTY (e.g. when piped or redirected).
pub fn emit_terminal_identity(identity: &TerminalIdentity) {
    if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        return;
    }
    eprint!("{}", osc_tab_title_sequence(identity.resolved_title()));
    match identity.background_action() {
        BackgroundAction::SetColor(osc_sequence) => eprint!("{osc_sequence}"),
        BackgroundAction::Reset => eprint!("{}", osc_reset_background_sequence()),
    }
}
