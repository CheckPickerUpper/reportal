//! Interactively edits an existing repo's metadata in the config.

use crate::error::ReportalError;
use crate::reportal_commands::prompts::{
    self, ColorEditResult, ColorEditPromptParams, TextPromptParams,
};
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle};
use crate::terminal_style;
use dialoguer::theme::ColorfulTheme;
use owo_colors::OwoColorize;

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

    let new_description = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Description",
        default_value: current_description,
    })?;

    let tags_input = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Tags (comma-separated)",
        default_value: current_tags,
    })?;

    let new_tags = prompts::parse_comma_separated_tags(&tags_input);

    let new_title = prompts::prompt_for_text(TextPromptParams {
        prompt_theme: &prompt_theme,
        label: "Tab title (empty = use alias)",
        default_value: current_title,
    })?;

    let color_edit_result = prompts::prompt_for_color_edit(ColorEditPromptParams {
        prompt_theme: &prompt_theme,
        current_default: &current_color,
    })?;

    let resolved_title = match new_title.is_empty() {
        true => TabTitle::UseAlias,
        false => TabTitle::Custom(new_title),
    };

    let resolved_color = match color_edit_result {
        ColorEditResult::Provided(hex_color) => RepoColor::Themed(hex_color),
        ColorEditResult::Unchanged(hex_color) => RepoColor::Themed(hex_color),
        ColorEditResult::Cleared => RepoColor::ResetToDefault,
    };

    let repo_to_update = loaded_config.get_repo_mut(alias)?;
    repo_to_update.set_description(new_description);
    repo_to_update.set_tags(new_tags);
    repo_to_update.set_tab_title(resolved_title);
    repo_to_update.set_repo_color(resolved_color);
    loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!("Updated '{alias}'"));

    return Ok(());
}
