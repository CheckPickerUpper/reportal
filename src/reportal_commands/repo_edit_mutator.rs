//! Field-edit state and methods for `rep edit`, grouped on a
//! context struct so the command file stays within its
//! free-function budget and each mutation helper takes exactly
//! one non-`self` argument.

use crate::error::ReportalError;
use crate::reportal_commands::prompts::{
    self, ColorEditPromptParameters, ColorEditResult, TextPromptParameters,
};
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{RepoColor, ReportalConfig, TabTitle};
use dialoguer::theme::ColorfulTheme;
use std::path::PathBuf;

/// Live edit-session state for `rep edit`. Owns the current
/// display strings for every editable field so the command loop
/// can rebuild menu labels without re-querying the config between
/// iterations, and exposes one mutation method per field so the
/// command loop's match arm is a one-liner per field. The struct
/// is constructed directly via struct literal from `repo_edit.rs`
/// using the `pub(super)` field visibility, because the project's
/// argument rules forbid a multi-parameter constructor and a
/// one-step struct literal is not a function call so it sidesteps
/// that rule correctly.
pub struct RepoEditFieldMutator<'config, 'alias, 'theme> {
    /// The loaded config being mutated in-place.
    pub(super) loaded_config: &'config mut ReportalConfig,
    /// The primary key of the repo currently being edited.
    pub(super) selected_alias: &'alias str,
    /// Prompt theme shared by every interactive field edit.
    pub(super) prompt_theme: &'theme ColorfulTheme,
    /// Current raw path string for menu display and prompts.
    pub(super) current_raw_path: String,
    /// Current description text for menu display and prompts.
    pub(super) current_description: String,
    /// Current tag list rendered as comma-separated display text.
    pub(super) current_tags_csv: String,
    /// Current tab title text (empty means fall back to alias).
    pub(super) current_title: String,
    /// Current hex color text (empty means reset-to-default).
    pub(super) current_color_hex: String,
}

/// Field-mutation and menu-label methods for `RepoEditFieldMutator`.
impl RepoEditFieldMutator<'_, '_, '_> {
    /// Builds the menu label list for the edit loop in the same
    /// order the match arms in `repo_edit.rs` dispatch on, so
    /// label-index drift between the menu render and the mutation
    /// dispatch is impossible — both sides are driven from this
    /// single ordered list.
    #[must_use]
    pub fn build_menu_labels(&self) -> Vec<String> {
        let path_label = format!("Path: {}", truncate_for_menu(&self.current_raw_path));
        let description_label = format!(
            "Description: {}",
            truncate_for_menu(&self.current_description)
        );
        let tags_label = if self.current_tags_csv.is_empty() {
            "Tags: (none)".to_owned()
        } else {
            format!("Tags: {}", truncate_for_menu(&self.current_tags_csv))
        };
        let title_label = if self.current_title.is_empty() {
            "Title: (use alias)".to_owned()
        } else {
            format!("Title: {}", self.current_title)
        };
        let color_label = if self.current_color_hex.is_empty() {
            "Color: (none)".to_owned()
        } else {
            format!("Color: {}", self.current_color_hex)
        };
        vec![
            path_label,
            description_label,
            tags_label,
            title_label,
            color_label,
            "Done".to_owned(),
        ]
    }

    /// Prompts for a new filesystem path, validates it exists,
    /// writes it to config, and regenerates every `.code-workspace`
    /// file that references the repo. The regeneration call is
    /// the Design B invariant in action: without it, a repo move
    /// leaves every workspace containing the repo pointing at the
    /// old location, and editor sessions open against a stale
    /// path. Regeneration runs after the config save so the
    /// `WorkspaceRegenerator` reads the post-mutation repo path,
    /// which is what the generated `folders` array must reflect.
    /// Errors: returns `ValidationFailure` if the prompt yields an
    /// empty or non-existent path, `RepoNotFound` if the alias has
    /// been removed from under the edit session, or any config /
    /// file I/O error from save and regenerate.
    pub fn apply_path_edit(&mut self) -> Result<(), ReportalError> {
        let new_raw_path = prompts::prompt_for_text(&TextPromptParameters {
            prompt_theme: self.prompt_theme,
            label: "Path",
            default_value: &self.current_raw_path,
        })?;
        if new_raw_path.trim().is_empty() {
            return Err(ReportalError::ValidationFailure {
                field: "path".to_owned(),
                reason: "must not be empty".to_owned(),
            });
        }
        let expanded_new_path = PathBuf::from(shellexpand::tilde(&new_raw_path).as_ref());
        if !expanded_new_path.exists() {
            return Err(ReportalError::ValidationFailure {
                field: "path".to_owned(),
                reason: format!("{} does not exist", expanded_new_path.display()),
            });
        }
        let repo_to_update = self.loaded_config.get_repo_mut(self.selected_alias)?;
        repo_to_update.set_raw_path(new_raw_path);
        self.loaded_config.save_to_disk()?;
        self.current_raw_path = self
            .loaded_config
            .get_repo(self.selected_alias)?
            .raw_path()
            .to_owned();
        self.regenerate_affected_workspaces()?;
        Ok(())
    }

    /// Prompts the user for a new human-readable description and
    /// writes it to the repo entry in config, then refreshes the
    /// cached display string so the menu labels rebuild against
    /// the post-save state on the next iteration. Errors: returns
    /// whichever prompt, repo-lookup, or config I/O error the
    /// underlying calls surface.
    pub fn apply_description_edit(&mut self) -> Result<(), ReportalError> {
        let new_description = prompts::prompt_for_text(&TextPromptParameters {
            prompt_theme: self.prompt_theme,
            label: "Description",
            default_value: &self.current_description,
        })?;
        let repo_to_update = self.loaded_config.get_repo_mut(self.selected_alias)?;
        repo_to_update.set_description(new_description);
        self.loaded_config.save_to_disk()?;
        self.current_description = self
            .loaded_config
            .get_repo(self.selected_alias)?
            .description()
            .to_owned();
        Ok(())
    }

    /// Prompts for a comma-separated tag list, parses it into an
    /// ordered `Vec<String>`, and replaces the repo's existing tag
    /// list wholesale. Replacing rather than merging is correct
    /// because it gives the user a single predictable editing
    /// surface — what you type is what you get — without a hidden
    /// merge rule. Errors: returns whichever prompt or config I/O
    /// error the underlying calls surface.
    pub fn apply_tags_edit(&mut self) -> Result<(), ReportalError> {
        let tags_input = prompts::prompt_for_text(&TextPromptParameters {
            prompt_theme: self.prompt_theme,
            label: "Tags (comma-separated)",
            default_value: &self.current_tags_csv,
        })?;
        let new_tags = prompts::parse_comma_separated_tags(&tags_input);
        let repo_to_update = self.loaded_config.get_repo_mut(self.selected_alias)?;
        repo_to_update.set_tags(new_tags);
        self.loaded_config.save_to_disk()?;
        self.current_tags_csv = self
            .loaded_config
            .get_repo(self.selected_alias)?
            .tags()
            .join(", ");
        Ok(())
    }

    /// Prompts for a new tab title, stores an empty entry as
    /// `TabTitle::UseAlias` (the "fall back to alias" variant) and
    /// a non-empty entry as `TabTitle::Custom`, then refreshes the
    /// cached display string. Errors: returns whichever prompt or
    /// config I/O error the underlying calls surface.
    pub fn apply_title_edit(&mut self) -> Result<(), ReportalError> {
        let new_title = prompts::prompt_for_text(&TextPromptParameters {
            prompt_theme: self.prompt_theme,
            label: "Tab title (empty = use alias)",
            default_value: &self.current_title,
        })?;
        let resolved_title = if new_title.is_empty() {
            TabTitle::UseAlias
        } else {
            TabTitle::Custom(new_title)
        };
        let repo_to_update = self.loaded_config.get_repo_mut(self.selected_alias)?;
        repo_to_update.set_tab_title(resolved_title);
        self.loaded_config.save_to_disk()?;
        self.current_title = match self
            .loaded_config
            .get_repo(self.selected_alias)?
            .tab_title()
        {
            TabTitle::Custom(custom_title) => custom_title.to_owned(),
            TabTitle::UseAlias => String::new(),
        };
        Ok(())
    }

    /// Prompts for a color edit via the shared color-edit prompt,
    /// maps `Provided`/`Unchanged` outcomes to `RepoColor::Themed`
    /// and `Cleared` to `RepoColor::ResetToDefault`, then writes
    /// the result to config. Errors: returns whichever prompt or
    /// config I/O error the underlying calls surface.
    pub fn apply_color_edit(&mut self) -> Result<(), ReportalError> {
        let color_edit_result = prompts::prompt_for_color_edit(&ColorEditPromptParameters {
            prompt_theme: self.prompt_theme,
            current_default: &self.current_color_hex,
        })?;
        let resolved_color = match color_edit_result {
            ColorEditResult::Provided(hex_color) | ColorEditResult::Unchanged(hex_color) => {
                RepoColor::Themed(hex_color)
            }
            ColorEditResult::Cleared => RepoColor::ResetToDefault,
        };
        let repo_to_update = self.loaded_config.get_repo_mut(self.selected_alias)?;
        repo_to_update.set_repo_color(resolved_color);
        self.loaded_config.save_to_disk()?;
        self.current_color_hex = match self
            .loaded_config
            .get_repo(self.selected_alias)?
            .repo_color()
        {
            RepoColor::Themed(hex_color) => hex_color.raw_value().to_owned(),
            RepoColor::ResetToDefault => String::new(),
        };
        Ok(())
    }

    /// Regenerates every `.code-workspace` file whose workspace
    /// contains the currently-edited repo. Called from
    /// `apply_path_edit` because a repo move is the only field
    /// mutation that changes the paths embedded inside
    /// `.code-workspace` files. Returning the regeneration error
    /// rather than swallowing it is correct because a failure
    /// after a successful config save leaves disk state
    /// inconsistent with config state, which the user must see.
    fn regenerate_affected_workspaces(&self) -> Result<(), ReportalError> {
        let affected_workspaces =
            self.loaded_config.workspaces_containing_repo(self.selected_alias);
        if affected_workspaces.is_empty() {
            return Ok(());
        }
        let regenerator = WorkspaceRegenerator::for_config(self.loaded_config);
        for (workspace_name, _containing_entry) in affected_workspaces {
            regenerator.regenerate_workspace_file(workspace_name)?;
        }
        Ok(())
    }
}

/// Truncates a string to a reasonable menu-display length,
/// returning the first 50 characters if the input exceeds that
/// limit so long paths or descriptions do not blow out the menu
/// row height of the interactive select widget.
fn truncate_for_menu(display_text: &str) -> &str {
    if display_text.len() > 50 {
        &display_text[..50]
    } else {
        display_text
    }
}
