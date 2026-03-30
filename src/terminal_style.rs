/// Centralized color palette and styling for all RePortal terminal output.
///
/// Uses the Nightfall Candy palette from the Roblox Studio syntax theme.
/// All colors are applied via owo-colors for zero-allocation styling.

use owo_colors::{OwoColorize, Style};

/// Neutral gray for repos with no configured color.
pub const DEFAULT_SWATCH_STYLE: Style = Style::new().dimmed();

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

/// Builds a dialoguer `ColorfulTheme` tuned to the Nightfall Candy palette.
///
/// Match-highlighted characters in fuzzy select use accent blue with bold.
/// Non-matched items are dimmed so the highlighted item and matched
/// characters pop visually. The prompt prefix uses cyan to match LABEL_STYLE.
pub fn reportal_prompt_theme() -> dialoguer::theme::ColorfulTheme {
    use console::Style as ConsoleStyle;

    dialoguer::theme::ColorfulTheme {
        fuzzy_match_highlight_style: ConsoleStyle::new().for_stderr().color256(117).bold(),
        active_item_style: ConsoleStyle::new().for_stderr().color256(117),
        inactive_item_style: ConsoleStyle::new().for_stderr().dim(),
        prompt_prefix: console::style("?".to_string()).for_stderr().cyan(),
        prompt_style: ConsoleStyle::new().for_stderr().bold(),
        ..dialoguer::theme::ColorfulTheme::default()
    }
}

/// Builds an owo-colors `Style` for the swatch block (`██`) based on
/// the repo's configured color. Returns a truecolor foreground style
/// for themed repos, or the default gray for repos with no color.
pub fn swatch_style_for_repo_color(repo_color: &crate::reportal_config::RepoColor) -> Result<Style, crate::error::ReportalError> {
    match repo_color {
        crate::reportal_config::RepoColor::Themed(hex_color) => {
            let (red, green, blue) = hex_color.as_rgb_bytes()?;
            return Ok(Style::new().truecolor(red, green, blue));
        }
        crate::reportal_config::RepoColor::ResetToDefault => {
            return Ok(DEFAULT_SWATCH_STYLE);
        }
    }
}

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

/// Writes a raw string directly to the console handle (CONOUT$ on Windows,
/// /dev/tty on Unix). Silently skipped if the console can't be opened.
pub fn write_to_console(text: &str) {
    use std::io::Write;

    #[cfg(target_os = "windows")]
    let console_handle = std::fs::OpenOptions::new()
        .write(true)
        .open("CONOUT$");

    #[cfg(not(target_os = "windows"))]
    let console_handle = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty");

    match console_handle {
        Ok(mut console) => {
            let _write_result = console.write_all(text.as_bytes());
            let _flush_result = console.flush();
        }
        Err(_console_open_error) => {}
    }
}

/// Writes OSC sequences directly to the console, bypassing both stdout
/// and stderr. This is necessary because PowerShell captures stdout in
/// subshells and prompt functions, so escape sequences written to stdout
/// or stderr never reach the terminal. On Windows this opens CONOUT$;
/// on Unix it opens /dev/tty. If the console can't be opened (e.g. in
/// a headless context), the sequences are silently skipped.
pub fn emit_terminal_identity_to_console(identity: &TerminalIdentity) {
    use std::io::Write;

    #[cfg(target_os = "windows")]
    let console_handle = std::fs::OpenOptions::new()
        .write(true)
        .open("CONOUT$");

    #[cfg(not(target_os = "windows"))]
    let console_handle = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty");

    match console_handle {
        Ok(mut console) => {
            let title_sequence = osc_tab_title_sequence(identity.resolved_title());
            let color_sequence = match identity.tab_color_action() {
                TabColorAction::SetColor(osc_sequence) => osc_sequence.to_string(),
                TabColorAction::Reset => osc_reset_tab_color_sequence().to_string(),
            };
            let combined = format!("{title_sequence}{color_sequence}");
            let _write_result = console.write_all(combined.as_bytes());
            let _flush_result = console.flush();
        }
        Err(_console_open_error) => {}
    }
}
