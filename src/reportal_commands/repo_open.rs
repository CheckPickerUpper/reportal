/// Fuzzy-selects a repo and opens it in the configured editor.

use crate::error::ReportalError;
use crate::reportal_config::{PathVisibility, RepoColor, ReportalConfig, TabTitle, TagFilter};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use owo_colors::OwoColorize;
use std::process::Command;

/// All parameters needed to run the open command.
pub struct OpenCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and open this alias directly.
    pub direct_alias: &'a str,
    /// If non-empty, use this editor instead of the configured default.
    pub editor_override: &'a str,
    /// If non-empty, override the tab title for this session.
    pub title_override: &'a str,
}

/// Opens a repo in the configured editor (default: cursor).
///
/// If `direct_alias` is provided, opens that repo directly without
/// prompting. Otherwise, presents a fuzzy finder for interactive selection.
/// The editor is launched by `cd`-ing into the repo directory first,
/// then running `<editor> .` so the editor opens the folder correctly.
/// Also emits OSC escape sequences for tab title and background color.
pub fn run_open(open_params: OpenCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let (selected_alias, selected_repo): (&str, &crate::reportal_config::RepoEntry) =
        match open_params.direct_alias.is_empty() {
            false => {
                let found_repo = loaded_config.get_repo(open_params.direct_alias)?;
                (open_params.direct_alias, found_repo)
            }
            true => {
                let matching_repos =
                    loaded_config.repos_matching_tag_filter(&open_params.tag_filter);
                if matching_repos.is_empty() {
                    return Err(ReportalError::NoReposMatchFilter);
                }

                let display_labels: Vec<String> = matching_repos
                    .iter()
                    .map(|(alias, repo)| {
                        let description_suffix = match repo.description().is_empty() {
                            true => String::new(),
                            false => format!(" - {}", repo.description()),
                        };
                        format!("{}{}", alias, description_suffix)
                    })
                    .collect();

                let selected_index = FuzzySelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Open in editor")
                    .items(&display_labels)
                    .interact_opt()
                    .map_err(|select_error| ReportalError::ConfigIoFailure {
                        reason: select_error.to_string(),
                    })?;

                match selected_index {
                    Some(chosen_index) => match matching_repos.get(chosen_index) {
                        Some((chosen_alias, chosen_repo)) => (chosen_alias.as_str(), *chosen_repo),
                        None => return Err(ReportalError::SelectionCancelled),
                    },
                    None => return Err(ReportalError::SelectionCancelled),
                }
            }
        };

    let resolved_title = match open_params.title_override.is_empty() {
        false => open_params.title_override.to_string(),
        true => match selected_repo.tab_title() {
            TabTitle::Custom(custom_title) => custom_title.to_string(),
            TabTitle::UseAlias => selected_alias.to_string(),
        },
    };

    let tab_color_action = match selected_repo.repo_color() {
        RepoColor::Themed(hex_color) => {
            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
        }
        RepoColor::ResetToDefault => TabColorAction::Reset,
    };

    let identity = TerminalIdentity::new(TerminalIdentityParams {
        resolved_title,
        tab_color_action,
    });
    terminal_style::emit_terminal_identity_to_stderr(&identity);

    let resolved_repo_path = selected_repo.resolved_path();

    let editor_command = match open_params.editor_override.is_empty() {
        true => loaded_config.default_editor(),
        false => open_params.editor_override,
    };

    #[cfg(target_os = "windows")]
    let spawn_result = Command::new("cmd")
        .args(["/c", editor_command, "."])
        .current_dir(&resolved_repo_path)
        .spawn();

    #[cfg(not(target_os = "windows"))]
    let spawn_result = Command::new(editor_command)
        .arg(".")
        .current_dir(&resolved_repo_path)
        .spawn();

    spawn_result.map_err(|spawn_error| ReportalError::EditorLaunchFailure {
        reason: spawn_error.to_string(),
    })?;

    match loaded_config.path_on_select() {
        PathVisibility::Show => {
            let formatted_path =
                loaded_config.path_display_format().format_path(&resolved_repo_path);
            terminal_style::print_success(&format!(
                "Opened {} in {}",
                formatted_path.style(terminal_style::PATH_STYLE),
                editor_command.style(terminal_style::ALIAS_STYLE)
            ));
        }
        PathVisibility::Hide => {}
    }

    return Ok(());
}
