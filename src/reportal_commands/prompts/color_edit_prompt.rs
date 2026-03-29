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
pub fn prompt_for_color_edit(color_edit_params: ColorEditPromptParams<'_>) -> Result<ColorEditResult, ReportalError> {
    loop {
        let color_input: String = Input::with_theme(color_edit_params.prompt_theme)
            .with_prompt("Background color (#RRGGBB, empty = none)")
            .default(color_edit_params.current_default.to_string())
            .interact_text()
            .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                reason: prompt_error.to_string(),
            })?;

        match color_input.is_empty() {
            true => return Ok(ColorEditResult::Cleared),
            false => match HexColor::parse(&color_input) {
                Ok(valid_color) => {
                    match color_input.eq(color_edit_params.current_default) {
                        true => return Ok(ColorEditResult::Unchanged(valid_color)),
                        false => return Ok(ColorEditResult::Provided(valid_color)),
                    }
                }
                Err(color_error) => {
                    terminal_style::print_error(&color_error.to_string());
                }
            },
        }
    }
}
