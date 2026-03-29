//! Interactive metadata collection and repo registration flow.

use crate::error::ReportalError;
use crate::reportal_commands::prompts::{
    self, ColorPromptResult, TextPromptParams,
};
use crate::reportal_config::{RepoRegistrationBuilder, ReportalConfig};
use crate::terminal_style;
use dialoguer::theme::ColorfulTheme;
use owo_colors::OwoColorize;

/// All data needed to run the interactive metadata collection and registration.
pub struct RegistrationContext<'a> {
    /// Mutable reference to the loaded config for adding the repo.
    pub(super) loaded_config: &'a mut ReportalConfig,
    /// The filesystem path where the repo lives.
    pub(super) filesystem_path: &'a str,
    /// A suggested alias to pre-fill the prompt.
    pub(super) suggested_alias: String,
    /// A detected or known remote URL to pre-fill the prompt.
    pub(super) detected_remote: String,
}

/// Prompts the user for metadata then registers the repo in config.
impl<'a> RegistrationContext<'a> {
    /// Walks the user through alias, description, tags, remote, title,
    /// and color prompts, shows a confirmation summary, then saves
    /// the new repo entry to disk.
    pub fn collect_metadata_and_register(self) -> Result<(), ReportalError> {
        let prompt_theme = ColorfulTheme::default();

        let repo_alias = prompts::prompt_for_text(TextPromptParams {
            prompt_theme: &prompt_theme,
            label: "Alias",
            default_value: &self.suggested_alias,
        })?;

        let repo_description = prompts::prompt_for_text(TextPromptParams {
            prompt_theme: &prompt_theme,
            label: "Description",
            default_value: "",
        })?;

        let tags_input = prompts::prompt_for_text(TextPromptParams {
            prompt_theme: &prompt_theme,
            label: "Tags (comma-separated)",
            default_value: "",
        })?;

        let parsed_tags = prompts::parse_comma_separated_tags(&tags_input);

        let repo_remote = prompts::prompt_for_text(TextPromptParams {
            prompt_theme: &prompt_theme,
            label: "Remote URL",
            default_value: &self.detected_remote,
        })?;

        let tab_title = prompts::prompt_for_text(TextPromptParams {
            prompt_theme: &prompt_theme,
            label: "Tab title (empty = use alias)",
            default_value: "",
        })?;

        let repo_color = prompts::prompt_for_color(&prompt_theme)?;

        println!();
        println!("  {} {}", "Alias:".style(terminal_style::LABEL_STYLE), repo_alias.style(terminal_style::ALIAS_STYLE));
        println!("  {} {}", "Path:".style(terminal_style::LABEL_STYLE), self.filesystem_path.style(terminal_style::PATH_STYLE));
        if !repo_description.is_empty() {
            println!("  {} {}", "Desc:".style(terminal_style::LABEL_STYLE), repo_description);
        }
        if !parsed_tags.is_empty() {
            println!("  {} {}", "Tags:".style(terminal_style::LABEL_STYLE), parsed_tags.join(", ").style(terminal_style::TAG_STYLE));
        }
        if !repo_remote.is_empty() {
            println!("  {} {}", "Remote:".style(terminal_style::LABEL_STYLE), repo_remote.style(terminal_style::PATH_STYLE));
        }
        if !tab_title.is_empty() {
            println!("  {} {}", "Title:".style(terminal_style::LABEL_STYLE), tab_title.style(terminal_style::ALIAS_STYLE));
        }
        match &repo_color {
            ColorPromptResult::Provided(hex_color) => {
                println!("  {} {}", "Color:".style(terminal_style::LABEL_STYLE), hex_color.raw_value());
            }
            ColorPromptResult::Skipped => {}
        }
        println!();

        let display_alias = repo_alias.as_str().to_string();

        let mut builder = RepoRegistrationBuilder::start(repo_alias)
            .repo_path(self.filesystem_path.to_string())
            .repo_description(repo_description)
            .repo_tags(parsed_tags)
            .repo_remote(repo_remote);

        if !tab_title.is_empty() {
            builder = builder.repo_title(tab_title);
        }
        match repo_color {
            ColorPromptResult::Provided(hex_color) => {
                builder = builder.repo_color(hex_color);
            }
            ColorPromptResult::Skipped => {}
        }

        let validated_registration = builder.build()?;

        self.loaded_config.add_repo(validated_registration)?;
        self.loaded_config.save_to_disk()?;

        terminal_style::print_success(&format!("Registered '{}'", display_alias));
        Ok(())
    }
}
