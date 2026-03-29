//! Color input prompt for repo creation (new repos with no existing color).

use crate::error::ReportalError;
use crate::reportal_config::HexColor;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Input};

/// Whether the user provided a color or left it empty during repo creation.
pub enum ColorPromptResult {
    /// The user entered a valid hex color.
    Provided(HexColor),
    /// The user left the prompt empty (no color).
    Skipped,
}

/// Prompts for a hex color during repo creation, re-asking on invalid input
/// until the user either enters a valid `#RRGGBB` or leaves it empty to skip.
pub fn prompt_for_color(prompt_theme: &ColorfulTheme) -> Result<ColorPromptResult, ReportalError> {
    loop {
        let color_input: String = Input::with_theme(prompt_theme)
            .with_prompt("Background color (#RRGGBB, empty = none)")
            .default(String::new())
            .interact_text()
            .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                reason: prompt_error.to_string(),
            })?;

        match color_input.is_empty() {
            true => return Ok(ColorPromptResult::Skipped),
            false => match HexColor::parse(&color_input) {
                Ok(valid_color) => return Ok(ColorPromptResult::Provided(valid_color)),
                Err(color_error) => {
                    terminal_style::print_error(&color_error.to_string());
                }
            },
        }
    }
}
