/// Shared interactive prompt helpers for add and edit commands.

use crate::error::ReportalError;
use crate::reportal_config::HexColor;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Input};

/// Parameters for a text input prompt with a label and default value.
pub struct TextPromptParams<'a> {
    /// The dialoguer theme to style the prompt.
    pub prompt_theme: &'a ColorfulTheme,
    /// The label shown to the user (e.g. "Description").
    pub label: &'a str,
    /// The default value pre-filled in the prompt.
    pub default_value: String,
}

/// Prompts the user for a single line of text with a default value.
///
/// Wraps the dialoguer `Input` widget and maps IO errors into
/// `ReportalError::ConfigIoFailure` so callers can propagate with `?`.
pub fn prompt_for_text(text_prompt_params: TextPromptParams<'_>) -> Result<String, ReportalError> {
    let entered_text: String = Input::with_theme(text_prompt_params.prompt_theme)
        .with_prompt(text_prompt_params.label)
        .default(text_prompt_params.default_value)
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    return Ok(entered_text);
}

/// Splits a comma-separated string into trimmed, non-empty tag strings.
///
/// Empty segments (from double commas or trailing commas) are filtered out.
/// Returns an empty vec if the input is empty.
pub fn parse_comma_separated_tags(raw_input: &str) -> Vec<String> {
    return raw_input
        .split(',')
        .map(|tag_segment| tag_segment.trim().to_string())
        .filter(|trimmed_tag| !trimmed_tag.is_empty())
        .collect();
}

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

/// Whether the user changed, cleared, or kept the color during repo editing.
pub enum ColorEditResult {
    /// The user entered a new valid hex color.
    Provided(HexColor),
    /// The user cleared the color (entered empty).
    Cleared,
    /// The user kept the existing color unchanged.
    Unchanged(HexColor),
}

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
