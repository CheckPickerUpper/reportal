//! Text input prompt with a label and default value.

use crate::error::ReportalError;
use dialoguer::{theme::ColorfulTheme, Input};

/// Parameters for a text input prompt with a label and default value.
pub struct TextPromptParams<'a> {
    /// The dialoguer theme to style the prompt.
    pub prompt_theme: &'a ColorfulTheme,
    /// The label shown to the user (e.g. "Description").
    pub label: &'a str,
    /// The default value pre-filled in the prompt (borrowed, copied internally).
    pub default_value: &'a str,
}

/// Prompts the user for a single line of text with a default value.
///
/// Wraps the dialoguer `Input` widget and maps IO errors into
/// `ReportalError::ConfigIoFailure` so callers can propagate with `?`.
pub fn prompt_for_text(text_prompt_params: TextPromptParams<'_>) -> Result<String, ReportalError> {
    let entered_text: String = Input::with_theme(text_prompt_params.prompt_theme)
        .with_prompt(text_prompt_params.label)
        .default(text_prompt_params.default_value.to_string())
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
