//! Interactively edits an existing repo's metadata in the config.

use crate::error::ReportalError;
use crate::reportal_config::{HexColor, RepoColor, ReportalConfig, TabTitle};
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Input};
use owo_colors::OwoColorize;

/// Whether the user provided a color or left it empty.
enum ColorEditResult {
    /// The user entered a valid hex color.
    Provided(HexColor),
    /// The user cleared the color (entered empty).
    Cleared,
    /// The user kept the existing color unchanged.
    Unchanged(HexColor),
}

/// Prompts for a hex color with the current value as default,
/// re-asking on invalid input until the user enters a valid
/// `#RRGGBB`, clears it, or keeps the default.
fn prompt_for_color_edit(prompt_theme: &ColorfulTheme, current_default: &str) -> Result<ColorEditResult, ReportalError> {
    loop {
        let color_input: String = Input::with_theme(prompt_theme)
            .with_prompt("Background color (#RRGGBB, empty = none)")
            .default(current_default.to_string())
            .interact_text()
            .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                reason: prompt_error.to_string(),
            })?;

        match color_input.is_empty() {
            true => return Ok(ColorEditResult::Cleared),
            false => match HexColor::parse(&color_input) {
                Ok(valid_color) => {
                    match color_input.eq(current_default) {
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

/// Interactively edits a registered repo's description, tags, title,
/// and color. Pre-fills each prompt with the current value so the user
/// can press enter to keep it or type a new value to change it.
pub fn run_edit(alias: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    let existing_repo = loaded_config.get_repo(alias)?;

    let current_description = existing_repo.description().to_string();
    let current_tags = existing_repo.tags().join(", ");
    let current_title = match existing_repo.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_string(),
        TabTitle::UseAlias => String::new(),
    };
    let current_color = match existing_repo.repo_color() {
        RepoColor::Themed(hex_color) => hex_color.raw_value().to_string(),
        RepoColor::ResetToDefault => String::new(),
    };

    println!();
    println!(
        "  {} {}",
        "Editing:".style(terminal_style::LABEL_STYLE),
        alias.style(terminal_style::ALIAS_STYLE),
    );
    println!(
        "  {} {}",
        "Path:".style(terminal_style::LABEL_STYLE),
        existing_repo.raw_path().style(terminal_style::PATH_STYLE),
    );
    println!();

    let prompt_theme = ColorfulTheme::default();

    let new_description: String = Input::with_theme(&prompt_theme)
        .with_prompt("Description")
        .default(current_description)
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let tags_input: String = Input::with_theme(&prompt_theme)
        .with_prompt("Tags (comma-separated)")
        .default(current_tags)
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let new_tags: Vec<String> = tags_input
        .split(',')
        .map(|tag_segment| tag_segment.trim().to_string())
        .filter(|trimmed_tag| !trimmed_tag.is_empty())
        .collect();

    let new_title: String = Input::with_theme(&prompt_theme)
        .with_prompt("Tab title (empty = use alias)")
        .default(current_title)
        .interact_text()
        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
            reason: prompt_error.to_string(),
        })?;

    let color_result = prompt_for_color_edit(&prompt_theme, &current_color)?;

    let resolved_title = match new_title.is_empty() {
        true => TabTitle::UseAlias,
        false => TabTitle::Custom(new_title),
    };

    let resolved_color = match color_result {
        ColorEditResult::Provided(hex_color) => RepoColor::Themed(hex_color),
        ColorEditResult::Unchanged(hex_color) => RepoColor::Themed(hex_color),
        ColorEditResult::Cleared => RepoColor::ResetToDefault,
    };

    loaded_config.update_repo_metadata(alias, new_description, new_tags, resolved_title, resolved_color)?;
    loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!("Updated '{alias}'"));

    return Ok(());
}
