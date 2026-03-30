//! Interactively edits an existing repo's metadata via a field menu.

use crate::error::ReportalError;
use crate::reportal_commands::prompts::{
    self, ColorEditPromptParams, ColorEditResult, TextPromptParams,
};
use crate::reportal_commands::repo_selection::{self, SelectedRepoParams};
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle, TagFilter};
use crate::terminal_style;
use dialoguer::Select;
use owo_colors::OwoColorize;

/// All parameters needed to run the edit command.
pub struct EditCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and edit this alias directly.
    pub direct_alias: &'a str,
}

/// Truncates a string to a reasonable menu-display length, returning
/// the first 50 characters if the input exceeds that limit.
fn truncate_for_menu(display_text: &str) -> &str {
    match display_text.len() > 50 {
        true => &display_text[..50],
        false => display_text,
    }
}

/// Fuzzy-selects a repo then presents a looping field menu for editing
/// individual fields. Each edit saves to disk immediately and refreshes
/// the menu labels. The user exits by choosing "Done" or pressing Escape.
pub fn run_edit(command_params: EditCommandParams<'_>) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;

    let selected_alias = {
        let selection = repo_selection::select_repo(SelectedRepoParams {
            loaded_config: &loaded_config,
            direct_alias: command_params.direct_alias,
            tag_filter: &command_params.tag_filter,
            prompt_label: "Edit repo",
        })?;
        selection.repo_alias().to_string()
    };

    let repo_path_display = loaded_config.get_repo(&selected_alias)?.raw_path().to_string();

    println!();
    println!(
        "  {} {}",
        "Editing:".style(terminal_style::LABEL_STYLE),
        selected_alias.style(terminal_style::ALIAS_STYLE),
    );
    println!(
        "  {} {}",
        "Path:".style(terminal_style::LABEL_STYLE),
        repo_path_display.style(terminal_style::PATH_STYLE),
    );

    let prompt_theme = terminal_style::reportal_prompt_theme();

    let repo_entry = loaded_config.get_repo(&selected_alias)?;
    let mut current_description = repo_entry.description().to_string();
    let mut current_tags_csv = repo_entry.tags().join(", ");
    let mut current_title = match repo_entry.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_string(),
        TabTitle::UseAlias => String::new(),
    };
    let mut current_color_hex = match repo_entry.repo_color() {
        RepoColor::Themed(hex_color) => hex_color.raw_value().to_string(),
        RepoColor::ResetToDefault => String::new(),
    };

    loop {
        println!();

        let description_label = format!("Description: {}", truncate_for_menu(&current_description));
        let tags_label = match current_tags_csv.is_empty() {
            true => "Tags: (none)".to_string(),
            false => format!("Tags: {}", truncate_for_menu(&current_tags_csv)),
        };
        let title_label = match current_title.is_empty() {
            true => "Title: (use alias)".to_string(),
            false => format!("Title: {}", current_title),
        };
        let color_label = match current_color_hex.is_empty() {
            true => "Color: (none)".to_string(),
            false => format!("Color: {}", current_color_hex),
        };
        let menu_labels = vec![description_label, tags_label, title_label, color_label, "Done".to_string()];

        let chosen_index = Select::with_theme(&prompt_theme)
            .with_prompt("Pick a field to edit")
            .items(&menu_labels)
            .default(0)
            .interact_opt()
            .map_err(|select_error| ReportalError::ConfigIoFailure {
                reason: select_error.to_string(),
            })?;

        match chosen_index {
            None => break,
            Some(4..) => break,
            Some(0) => {
                let new_description = prompts::prompt_for_text(TextPromptParams {
                    prompt_theme: &prompt_theme,
                    label: "Description",
                    default_value: &current_description,
                })?;
                let repo_to_update = loaded_config.get_repo_mut(&selected_alias)?;
                repo_to_update.set_description(new_description);
                loaded_config.save_to_disk()?;
                current_description = loaded_config.get_repo(&selected_alias)?.description().to_string();
            }
            Some(1) => {
                let tags_input = prompts::prompt_for_text(TextPromptParams {
                    prompt_theme: &prompt_theme,
                    label: "Tags (comma-separated)",
                    default_value: &current_tags_csv,
                })?;
                let new_tags = prompts::parse_comma_separated_tags(&tags_input);
                let repo_to_update = loaded_config.get_repo_mut(&selected_alias)?;
                repo_to_update.set_tags(new_tags);
                loaded_config.save_to_disk()?;
                current_tags_csv = loaded_config.get_repo(&selected_alias)?.tags().join(", ");
            }
            Some(2) => {
                let new_title = prompts::prompt_for_text(TextPromptParams {
                    prompt_theme: &prompt_theme,
                    label: "Tab title (empty = use alias)",
                    default_value: &current_title,
                })?;
                let resolved_title = match new_title.is_empty() {
                    true => TabTitle::UseAlias,
                    false => TabTitle::Custom(new_title),
                };
                let repo_to_update = loaded_config.get_repo_mut(&selected_alias)?;
                repo_to_update.set_tab_title(resolved_title);
                loaded_config.save_to_disk()?;
                current_title = match loaded_config.get_repo(&selected_alias)?.tab_title() {
                    TabTitle::Custom(custom_title) => custom_title.to_string(),
                    TabTitle::UseAlias => String::new(),
                };
            }
            Some(3) => {
                let color_edit_result = prompts::prompt_for_color_edit(ColorEditPromptParams {
                    prompt_theme: &prompt_theme,
                    current_default: &current_color_hex,
                })?;
                let resolved_color = match color_edit_result {
                    ColorEditResult::Provided(hex_color) => RepoColor::Themed(hex_color),
                    ColorEditResult::Unchanged(hex_color) => RepoColor::Themed(hex_color),
                    ColorEditResult::Cleared => RepoColor::ResetToDefault,
                };
                let repo_to_update = loaded_config.get_repo_mut(&selected_alias)?;
                repo_to_update.set_repo_color(resolved_color);
                loaded_config.save_to_disk()?;
                current_color_hex = match loaded_config.get_repo(&selected_alias)?.repo_color() {
                    RepoColor::Themed(hex_color) => hex_color.raw_value().to_string(),
                    RepoColor::ResetToDefault => String::new(),
                };
            }
        }

        terminal_style::print_success("Saved");
    }

    return Ok(());
}
