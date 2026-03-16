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
