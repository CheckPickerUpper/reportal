/// Shared repo fuzzy-selection helper used across jump, open, ai, edit, and web.

use crate::error::ReportalError;
use crate::reportal_config::{RepoEntry, ReportalConfig, TagFilter};
use crate::terminal_style;
use dialoguer::FuzzySelect;
use owo_colors::OwoColorize;

/// A repo that was resolved either by direct alias lookup or fuzzy selection.
/// Holds borrowed references into the loaded config so callers can read
/// the alias and entry without cloning.
pub struct SelectedRepo<'a> {
    repo_alias: &'a str,
    repo_config: &'a RepoEntry,
}

/// Accessors for the selected repo's alias and config entry.
impl<'a> SelectedRepo<'a> {
    /// The canonical alias used to identify this repo in the config
    /// (either the direct alias passed in, or the one chosen from the fuzzy finder).
    pub fn repo_alias(&self) -> &'a str {
        self.repo_alias
    }

    /// The full config entry for this repo, including path, tags,
    /// description, color, and title.
    pub fn repo_config(&self) -> &'a RepoEntry {
        self.repo_config
    }
}

/// All inputs needed to resolve a repo — either by direct alias lookup
/// or interactive fuzzy selection with tag filtering and a labeled prompt.
pub struct SelectedRepoParams<'a> {
    /// The loaded config to search within.
    pub loaded_config: &'a ReportalConfig,
    /// If non-empty, skip the fuzzy finder and look up this alias directly.
    pub direct_alias: &'a str,
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: &'a TagFilter,
    /// The prompt shown in the fuzzy finder (e.g. "Jump to repo").
    pub prompt_label: &'a str,
}

/// Resolves a repo by direct alias or interactive fuzzy selection.
///
/// If `direct_alias` is non-empty, looks it up directly in the config.
/// Otherwise, presents a fuzzy finder filtered by `tag_filter`
/// with the given `prompt_label`. Repos are sorted by their first tag
/// so same-tag repos cluster visually. Each item shows a color swatch,
/// the alias, aliases, description, and tags.
pub fn select_repo<'a>(selection_params: SelectedRepoParams<'a>) -> Result<SelectedRepo<'a>, ReportalError> {
    match selection_params.direct_alias.is_empty() {
        false => {
            let found_repo = selection_params.loaded_config.get_repo(selection_params.direct_alias)?;
            return Ok(SelectedRepo {
                repo_alias: selection_params.direct_alias,
                repo_config: found_repo,
            });
        }
        true => {
            let mut matching_repos = selection_params.loaded_config.repos_matching_tag_filter(selection_params.tag_filter);
            if matching_repos.is_empty() {
                return Err(ReportalError::NoReposMatchFilter);
            }

            matching_repos.sort_by(|(alias_a, repo_a), (alias_b, repo_b)| {
                let first_tag_a = repo_a.tags().first().map(String::as_str);
                let first_tag_b = repo_b.tags().first().map(String::as_str);
                first_tag_a.cmp(&first_tag_b).then(alias_a.cmp(alias_b))
            });

            let display_labels: Vec<String> = matching_repos
                .iter()
                .map(|(alias, repo)| {
                    let swatch_style = match terminal_style::swatch_style_for_repo_color(repo.repo_color()) {
                        Ok(resolved_style) => resolved_style,
                        Err(_color_error) => terminal_style::DEFAULT_SWATCH_STYLE,
                    };
                    let swatch = "██".style(swatch_style);

                    let mut label = format!("{swatch} {alias}");

                    match repo.aliases().is_empty() {
                        true => {}
                        false => {
                            let aliases_joined = repo.aliases().join(", ");
                            label.push_str(&format!(" ({aliases_joined})"));
                        }
                    }

                    match repo.description().is_empty() {
                        true => {}
                        false => {
                            label.push_str(&format!(" — {}", repo.description()));
                        }
                    }

                    match repo.tags().is_empty() {
                        true => {}
                        false => {
                            let tags_display = repo.tags().join(", ");
                            label.push_str(&format!(" [{}]", tags_display));
                        }
                    }

                    return label;
                })
                .collect();

            let prompt_theme = terminal_style::reportal_prompt_theme();
            let selected_index = FuzzySelect::with_theme(&prompt_theme)
                .with_prompt(selection_params.prompt_label)
                .items(&display_labels)
                .interact_opt()
                .map_err(|select_error| ReportalError::ConfigIoFailure {
                    reason: select_error.to_string(),
                })?;

            match selected_index {
                Some(chosen_index) => match matching_repos.get(chosen_index) {
                    Some((chosen_alias, chosen_repo)) => {
                        return Ok(SelectedRepo {
                            repo_alias: chosen_alias.as_str(),
                            repo_config: *chosen_repo,
                        });
                    }
                    None => return Err(ReportalError::SelectionCancelled),
                },
                None => return Err(ReportalError::SelectionCancelled),
            }
        }
    }
}
