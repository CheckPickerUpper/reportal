/// Fuzzy-selects a repo and opens it in the configured editor.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
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
}

/// Opens a repo in the configured editor (default: cursor).
///
/// If `direct_alias` is provided, opens that repo directly without
/// prompting. Otherwise, presents a fuzzy finder for interactive selection.
/// The editor is launched by `cd`-ing into the repo directory first,
/// then running `<editor> .` so the editor opens the folder correctly.
pub fn run_open(open_params: OpenCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let resolved_repo_path = match open_params.direct_alias.is_empty() {
        false => {
            let found_repo = loaded_config.get_repo(open_params.direct_alias)?;
            found_repo.resolved_path()
        }
        true => {
            let matching_repos = loaded_config.repos_matching_tag_filter(&open_params.tag_filter);
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
                    Some((_, chosen_repo)) => chosen_repo.resolved_path(),
                    None => return Err(ReportalError::SelectionCancelled),
                },
                None => return Err(ReportalError::SelectionCancelled),
            }
        }
    };

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

    terminal_style::print_success(
        &format!("Opened {} in {}", resolved_repo_path.display(), editor_command.style(terminal_style::ALIAS_STYLE)),
    );
    Ok(())
}
