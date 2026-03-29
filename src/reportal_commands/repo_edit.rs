//! Interactively edits an existing repo's metadata in the config.

use crate::error::ReportalError;
use crate::reportal_commands::prompts::{
    self, ColorEditResult, ColorEditPromptParams, TextPromptParams,
};
use crate::reportal_commands::repo_selection::{self, RepoSelectionParams};
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle, TagFilter};
use crate::terminal_style;
use dialoguer::theme::ColorfulTheme;
use owo_colors::OwoColorize;

/// All parameters needed to run the edit command.
pub struct EditCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and edit this alias directly.
    pub direct_alias: &'a str,
}

/// Interactively edits a registered repo's description, tags, title,
/// and color. Resolves the target repo via fuzzy selection or direct
/// alias lookup, pre-fills each prompt with the current value so the
/// user can press enter to keep it or type a new value to change it.
pub fn run_edit(command_params: EditCommandParams<'_>) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;

    let (selected_alias, current_description, current_tags, current_title, current_color, current_repo_path) = {
        let selection = repo_selection::select_repo(RepoSelectionParams {
            loaded_config: &loaded_config,
            direct_alias: command_params.direct_alias,
            tag_filter: &command_params.tag_filter,
            prompt_label: "Edit repo",
        })?;

        let alias = selection.repo_alias().to_string();
        let description = selection.repo_config().description().to_string();
        let tags = selection.repo_config().tags().join(", ");
        let title = match selection.repo_config().tab_title() {
            TabTitle::Custom(custom_title) => custom_title.to_string(),
            TabTitle::UseAlias => String::new(),
        };
        let color = match selection.repo_config().repo_color() {
            RepoColor::Themed(hex_color) => hex_color.raw_value().to_string(),
            RepoColor::ResetToDefault => String::new(),
        };
        let repo_path = selection.repo_config().raw_path().to_string();

        (alias, description, tags, title, color, repo_path)
    };

    println!();
    println!(
        "  {} {}",
        "Editing:".style(terminal_style::LABEL_STYLE),
        selected_alias.style(terminal_style::ALIAS_STYLE),
    );
    println!(
        "  {} {}",
        "Path:".style(terminal_style::LABEL_STYLE),
        current_repo_path.style(terminal_style::PATH_STYLE),
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

    let repo_to_update = loaded_config.get_repo_mut(&selected_alias)?;
    repo_to_update.set_description(new_description);
    repo_to_update.set_tags(new_tags);
    repo_to_update.set_tab_title(resolved_title);
    repo_to_update.set_repo_color(resolved_color);
    loaded_config.save_to_disk()?;

    terminal_style::print_success(&format!("Updated '{selected_alias}'"));

    return Ok(());
}
