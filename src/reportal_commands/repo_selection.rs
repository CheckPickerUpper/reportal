/// Shared repo selection and terminal identity helpers used across commands.
///
/// Eliminates the duplicated fuzzy-finder and OSC tab-theming blocks
/// that were copy-pasted across jump, open, ai, and web.

use crate::error::ReportalError;
use crate::reportal_config::{RepoColor, RepoEntry, ReportalConfig, TabTitle, TagFilter};
use crate::terminal_style::{self, TabColorAction, TerminalIdentity, TerminalIdentityParams};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

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

/// Parameters for selecting a repo via fuzzy finder or direct alias.
pub struct RepoSelectionParams<'a> {
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
/// with the given `prompt_label`. Each item shows the alias,
/// any configured aliases in parens, and the description.
pub fn select_repo<'a>(selection_params: RepoSelectionParams<'a>) -> Result<SelectedRepo<'a>, ReportalError> {
    match selection_params.direct_alias.is_empty() {
        false => {
            let found_repo = selection_params.loaded_config.get_repo(selection_params.direct_alias)?;
            return Ok(SelectedRepo {
                repo_alias: selection_params.direct_alias,
                repo_config: found_repo,
            });
        }
        true => {
            let matching_repos = selection_params.loaded_config.repos_matching_tag_filter(selection_params.tag_filter);
            if matching_repos.is_empty() {
                return Err(ReportalError::NoReposMatchFilter);
            }

            let display_labels: Vec<String> = matching_repos
                .iter()
                .map(|(alias, repo)| {
                    let mut label = alias.to_string();

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

                    return label;
                })
                .collect();

            let selected_index = FuzzySelect::with_theme(&ColorfulTheme::default())
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

/// Parameters for emitting terminal identity (tab title + color).
pub struct TerminalIdentityEmitParams<'a> {
    /// The selected repo's alias (used as fallback title).
    pub selected_alias: &'a str,
    /// The selected repo's config entry (provides title and color fields).
    pub selected_repo: &'a RepoEntry,
    /// If non-empty, overrides the repo's configured title for this session.
    pub title_override: &'a str,
}

/// Resolves a repo's tab title and color, then emits OSC sequences
/// directly to the console handle (CONOUT$ / /dev/tty).
///
/// Title precedence: `title_override` > repo's `title` field > alias.
/// Color: repo's `color` field if set, otherwise resets to terminal default.
pub fn emit_repo_terminal_identity(identity_params: TerminalIdentityEmitParams<'_>) {
    let resolved_title = match identity_params.title_override.is_empty() {
        false => identity_params.title_override.to_string(),
        true => match identity_params.selected_repo.tab_title() {
            TabTitle::Custom(custom_title) => custom_title.to_string(),
            TabTitle::UseAlias => identity_params.selected_alias.to_string(),
        },
    };

    let tab_color_action = match identity_params.selected_repo.repo_color() {
        RepoColor::Themed(hex_color) => {
            TabColorAction::SetColor(hex_color.as_osc_tab_color_sequence())
        }
        RepoColor::ResetToDefault => TabColorAction::Reset,
    };

    let identity = TerminalIdentity::new(TerminalIdentityParams {
        resolved_title,
        tab_color_action,
    });
    terminal_style::emit_terminal_identity_to_console(&identity);
}
