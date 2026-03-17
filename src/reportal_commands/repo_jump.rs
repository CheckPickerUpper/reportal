/// Fuzzy-selects a repo and prints its path for shell `cd` integration.

use crate::error::ReportalError;
use crate::reportal_config::{PathVisibility, ReportalConfig, TagFilter};
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use owo_colors::OwoColorize;

/// All parameters needed to run the jump command.
pub struct JumpCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the fuzzy finder and jump directly.
    pub direct_alias: &'a str,
}

/// Prints the selected repo's resolved path to stdout; the shell
/// wrapper function (`rj`) reads this and runs `cd`.
///
/// If a direct alias is given, skips the fuzzy finder entirely.
/// The raw path always goes to stdout for the shell function;
/// an optional styled confirmation goes to stderr based on config.
pub fn run_jump(jump_params: JumpCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let resolved = match jump_params.direct_alias.is_empty() {
        false => {
            let found_repo = loaded_config.get_repo(jump_params.direct_alias)?;
            found_repo.resolved_path()
        }
        true => {
            let matching_repos = loaded_config.repos_matching_tag_filter(&jump_params.tag_filter);

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
                    return format!("{}{}", alias, description_suffix);
                })
                .collect();

            let selected_index = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Jump to repo")
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

    let formatted_path = loaded_config.path_display_format().format_path(&resolved);

    print!("{formatted_path}");

    match loaded_config.path_on_select() {
        PathVisibility::Show => {
            eprintln!(
                "  {} {}",
                ">>".style(terminal_style::SUCCESS_STYLE),
                formatted_path.style(terminal_style::PATH_STYLE),
            );
        }
        PathVisibility::Hide => {}
    }

    return Ok(());
}
