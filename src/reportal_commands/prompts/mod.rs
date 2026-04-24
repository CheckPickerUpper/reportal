//! Shared interactive prompt helpers for add and edit commands.
mod color_edit_prompt;
mod color_edit_result;
mod color_prompt;
mod text_prompt;

pub use color_edit_prompt::{prompt_for_color_edit, ColorEditPromptParameters};
pub use color_edit_result::ColorEditResult;
pub use color_prompt::{prompt_for_color, ColorPromptResult};
pub use text_prompt::{parse_comma_separated_tags, prompt_for_text, TextPromptParameters};
