//! Interactively edits an existing repo's metadata via a field menu.

use crate::error::ReportalError;
use crate::reportal_commands::repo_edit_mutator::RepoEditFieldMutator;
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

/// Fuzzy-selects a repo then presents a looping field menu for editing
/// individual fields. Each edit saves to disk immediately and refreshes
/// the menu labels. The user exits by choosing "Done" or pressing Escape.
///
/// The `Path` menu item delegates to
/// `RepoEditFieldMutator::apply_path_edit`, which regenerates every
/// `.code-workspace` file that references the edited repo so
/// Design B's invariant — moving a repo updates every workspace
/// containing it — holds.
///
/// # Errors
///
/// Returns whatever the underlying selection, prompt, config I/O,
/// validation, or workspace regeneration call surfaces.
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

    let initial_repo = loaded_config.get_repo(&selected_alias)?;
    let initial_raw_path = initial_repo.raw_path().to_owned();
    let initial_description = initial_repo.description().to_owned();
    let initial_tags_csv = initial_repo.tags().join(", ");
    let initial_title = match initial_repo.tab_title() {
        TabTitle::Custom(custom_title) => custom_title.to_owned(),
        TabTitle::UseAlias => String::new(),
    };
    let initial_color_hex = match initial_repo.repo_color() {
        RepoColor::Themed(hex_color) => hex_color.raw_value().to_owned(),
        RepoColor::ResetToDefault => String::new(),
    };

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {} {}\n",
        "Editing:".style(terminal_style::LABEL_STYLE),
        selected_alias.style(terminal_style::ALIAS_STYLE),
    ));
    terminal_style::write_stdout(&format!(
        "  {} {}\n",
        "Path:".style(terminal_style::LABEL_STYLE),
        initial_raw_path.style(terminal_style::PATH_STYLE),
    ));

    let prompt_theme = terminal_style::reportal_prompt_theme();
    let mut field_mutator = RepoEditFieldMutator {
        loaded_config: &mut loaded_config,
        selected_alias: &selected_alias,
        prompt_theme: &prompt_theme,
        current_raw_path: initial_raw_path,
        current_description: initial_description,
        current_tags_csv: initial_tags_csv,
        current_title: initial_title,
        current_color_hex: initial_color_hex,
    };

    loop {
        terminal_style::write_stdout("\n");
        let menu_labels = field_mutator.build_menu_labels();
        let chosen_index = Select::with_theme(&prompt_theme)
            .with_prompt("Pick a field to edit")
            .items(&menu_labels)
            .default(0)
            .interact_opt()
            .map_err(|select_error| ReportalError::ConfigIoFailure {
                reason: select_error.to_string(),
            })?;
        match chosen_index {
            None | Some(5..) => break,
            Some(0) => field_mutator.apply_path_edit()?,
            Some(1) => field_mutator.apply_description_edit()?,
            Some(2) => field_mutator.apply_tags_edit()?,
            Some(3) => field_mutator.apply_title_edit()?,
            Some(4) => field_mutator.apply_color_edit()?,
        }
        terminal_style::print_success("Saved");
    }

    Ok(())
}
