//! Color edit prompt for modifying an existing repo's terminal background color.

use crate::error::ReportalError;
use crate::reportal_config::HexColor;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Input};

use super::color_edit_result::ColorEditResult;

/// Parameters for the color edit prompt that needs the current value as default.
pub struct ColorEditPromptParams<'a> {
    /// The dialoguer theme to style the prompt.
    pub prompt_theme: &'a ColorfulTheme,
    /// The current color value as a raw hex string (empty if no color set).
    pub current_default: &'a str,
}

/// Prompts for a hex color during repo editing, with the current value
/// as default. Re-asks on invalid input until the user enters a valid
/// `#RRGGBB`, clears it, or keeps the default.
pub fn prompt_for_color_edit(color_edit_params: &ColorEditPromptParams<'_>) -> Result<ColorEditResult, ReportalError> {
    loop {
        let color_input: String = Input::with_theme(color_edit_params.prompt_theme)
            .with_prompt("Background color (#RRGGBB, empty = none)")
            .default(color_edit_params.current_default.to_owned())
            .interact_text()
            .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                reason: prompt_error.to_string(),
            })?;

        if color_input.is_empty() {
            return Ok(ColorEditResult::Cleared);
        }
        let valid_color = match HexColor::parse(&color_input) {
            Ok(parsed) => parsed,
            Err(color_error) => {
                terminal_style::print_error(&color_error.to_string());
                continue;
            }
        };
        if color_input == color_edit_params.current_default {
            return Ok(ColorEditResult::Unchanged(valid_color));
        }
        return Ok(ColorEditResult::Provided(valid_color));
    }
}
