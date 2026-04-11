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
    if display_text.len() > 50 {
        &display_text[..50]
    } else {
        display_text
    }
}

/// Prompts for a new tab title, saves to disk, and returns the updated value.
fn apply_title_edit(
    loaded_config: &mut ReportalConfig,
    selected_alias: &str,
    prompt_theme: &dialoguer::theme::ColorfulTheme,
    current_title: &str,
) -> Result<String, ReportalError> {
    let new_title = prompts::prompt_for_text(&TextPromptParams {
        prompt_theme,
        label: "Tab title (empty = use alias)",
        default_value: current_title,
    })?;
    let resolved_title = if new_title.is_empty() {
        TabTitle::UseAlias
    } else {
        TabTitle::Custom(new_title)
    };
    let repo_to_update = loaded_config.get_repo_mut(selected_alias)?;
    repo_to_update.set_tab_title(resolved_title);
    loaded_config.save_to_disk()?;
    let updated = match loaded_config.get_repo(selected_alias)?.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_owned(),
        TabTitle::UseAlias => String::new(),
    };
    Ok(updated)
}

/// Prompts for a color edit, saves to disk, and returns the updated hex string.
fn apply_color_edit(
    loaded_config: &mut ReportalConfig,
    selected_alias: &str,
    prompt_theme: &dialoguer::theme::ColorfulTheme,
    current_color_hex: &str,
) -> Result<String, ReportalError> {
    let color_edit_result = prompts::prompt_for_color_edit(&ColorEditPromptParams {
        prompt_theme,
        current_default: current_color_hex,
    })?;
    let resolved_color = match color_edit_result {
        ColorEditResult::Provided(hex_color) | ColorEditResult::Unchanged(hex_color) => RepoColor::Themed(hex_color),
        ColorEditResult::Cleared => RepoColor::ResetToDefault,
    };
    let repo_to_update = loaded_config.get_repo_mut(selected_alias)?;
    repo_to_update.set_repo_color(resolved_color);
    loaded_config.save_to_disk()?;
    let updated = match loaded_config.get_repo(selected_alias)?.repo_color() {
        RepoColor::Themed(hex_color) => hex_color.raw_value().to_owned(),
        RepoColor::ResetToDefault => String::new(),
    };
    Ok(updated)
}

/// Fuzzy-selects a repo then presents a looping field menu for editing
/// individual fields. Each edit saves to disk immediately and refreshes
/// the menu labels. The user exits by choosing "Done" or pressing Escape.
pub fn run_edit(command_params: &EditCommandParams<'_>) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;

    let selected_alias = {
        let selection_params = SelectedRepoParams {
            loaded_config: &loaded_config,
            direct_alias: command_params.direct_alias,
            tag_filter: &command_params.tag_filter,
            prompt_label: "Edit repo",
        };
        let selection = repo_selection::select_repo(&selection_params)?;
        selection.repo_alias().to_owned()
    };

    let repo_path_display = loaded_config.get_repo(&selected_alias)?.raw_path().to_owned();

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {} {}\n",
        "Editing:".style(terminal_style::LABEL_STYLE),
        selected_alias.style(terminal_style::ALIAS_STYLE),
    ));
    terminal_style::write_stdout(&format!(
        "  {} {}\n",
        "Path:".style(terminal_style::LABEL_STYLE),
        repo_path_display.style(terminal_style::PATH_STYLE),
    ));

    let prompt_theme = terminal_style::reportal_prompt_theme();

    let repo_entry = loaded_config.get_repo(&selected_alias)?;
    let mut current_description = repo_entry.description().to_owned();
    let mut current_tags_csv = repo_entry.tags().join(", ");
    let mut current_title = match repo_entry.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_owned(),
        TabTitle::UseAlias => String::new(),
    };
    let mut current_color_hex = match repo_entry.repo_color() {
        RepoColor::Themed(hex_color) => hex_color.raw_value().to_owned(),
        RepoColor::ResetToDefault => String::new(),
    };

    loop {
        terminal_style::write_stdout("\n");

        let description_label = format!("Description: {}", truncate_for_menu(&current_description));
        let tags_label = if current_tags_csv.is_empty() {
            "Tags: (none)".to_owned()
        } else {
            format!("Tags: {}", truncate_for_menu(&current_tags_csv))
        };
        let title_label = if current_title.is_empty() {
            "Title: (use alias)".to_owned()
        } else {
            format!("Title: {current_title}")
        };
        let color_label = if current_color_hex.is_empty() {
            "Color: (none)".to_owned()
        } else {
            format!("Color: {current_color_hex}")
        };
        let menu_labels = vec![description_label, tags_label, title_label, color_label, "Done".to_owned()];

        let chosen_index = Select::with_theme(&prompt_theme)
            .with_prompt("Pick a field to edit")
            .items(&menu_labels)
            .default(0)
            .interact_opt()
            .map_err(|select_error| ReportalError::ConfigIoFailure {
                reason: select_error.to_string(),
            })?;

        match chosen_index {
            None | Some(4..) => break,
            Some(0) => {
                let new_description = prompts::prompt_for_text(&TextPromptParams {
                    prompt_theme: &prompt_theme,
                    label: "Description",
                    default_value: &current_description,
                })?;
                let repo_to_update = loaded_config.get_repo_mut(&selected_alias)?;
                repo_to_update.set_description(new_description);
                loaded_config.save_to_disk()?;
                current_description.clear();
                current_description.push_str(loaded_config.get_repo(&selected_alias)?.description());
            }
            Some(1) => {
                let tags_input = prompts::prompt_for_text(&TextPromptParams {
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
                current_title = apply_title_edit(&mut loaded_config, &selected_alias, &prompt_theme, &current_title)?;
            }
            Some(3) => {
                current_color_hex = apply_color_edit(&mut loaded_config, &selected_alias, &prompt_theme, &current_color_hex)?;
            }
        }

        terminal_style::print_success("Saved");
    }

    Ok(())
}
