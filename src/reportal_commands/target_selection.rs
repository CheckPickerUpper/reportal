//! Unified fuzzy-select helper for `rep jump` / `rep open` that
//! includes both registered repos AND registered workspaces in
//! one list.
//!
//! Needed because the user's mental model for `rj` / `ro` is "go
//! somewhere" — and workspaces are legitimate targets. A repo-only
//! fuzzy list forces them to remember which entities are repos vs
//! workspaces before picking a command, which defeats the "fuzzy
//! finder to the rescue" ergonomic.

use crate::error::ReportalError;
use crate::reportal_config::{HasAliases, ReportalConfig, TagFilter};
use crate::terminal_style;
use dialoguer::FuzzySelect;
use owo_colors::OwoColorize;

/// Classified fuzzy-select outcome for the `rj` / `ro` entry
/// points.
///
/// Carries just the canonical alias/name; callers re-resolve the
/// full entry from the config as needed. Avoids tangling the
/// lifetime of the selection result with the borrowed config
/// snapshot taken at the selection moment.
pub enum SelectedTarget {
    /// The user picked a registered repo; payload is the repo's
    /// canonical alias (BTreeMap key).
    Repo(String),
    /// The user picked a registered workspace; payload is the
    /// workspace's canonical name.
    Workspace(String),
}

/// Inputs needed to present the unified fuzzy finder.
///
/// Takes the tag filter so the repo rows still honor `--tag`.
/// Workspace rows are included only when the tag filter is `All`,
/// because workspaces do not carry tags and slipping them into a
/// tag-restricted list would imply they match the filter.
pub struct SelectedTargetParameters<'borrow> {
    /// The loaded config to draw rows from.
    pub loaded_config: &'borrow ReportalConfig,
    /// Which repos to show in the finder; workspaces are shown
    /// only when this is `TagFilter::All`.
    pub tag_filter: &'borrow TagFilter,
    /// The prompt shown above the fuzzy finder.
    pub prompt_label: &'borrow str,
}

/// Presents the unified fuzzy finder and returns the user's
/// selection as a [`SelectedTarget`].
///
/// Rows:
/// - repos render with their color swatch `██` and any aliases,
///   description, and tags — same format as the repo-only finder.
/// - workspaces render with `▣▣` and a trailing `[workspace]`
///   suffix so the two kinds are visually distinct in the list.
///
/// The repo list precedes the workspace list because repos are
/// the common case; the selected-index arithmetic uses the split
/// count to dispatch to the correct variant.
///
/// # Errors
///
/// Returns [`ReportalError::NoReposMatchFilter`] when the tag
/// filter leaves zero repos AND the workspace namespace is also
/// empty; [`ReportalError::SelectionCancelled`] if the user
/// escapes the prompt; [`ReportalError::ConfigIoFailure`] for
/// fuzzy-finder I/O errors.
pub fn select_target(
    params: &SelectedTargetParameters<'_>,
) -> Result<SelectedTarget, ReportalError> {
    let matching_repos = params.loaded_config.repos_matching_tag_filter(params.tag_filter);
    let workspace_rows_included = match params.tag_filter {
        TagFilter::All => params.loaded_config.workspaces_with_names(),
        TagFilter::ByTag(_restricted_tag) => Vec::new(),
    };

    if matching_repos.is_empty() && workspace_rows_included.is_empty() {
        return Err(ReportalError::NoReposMatchFilter);
    }

    let mut display_labels: Vec<String> = Vec::with_capacity(
        matching_repos.len() + workspace_rows_included.len(),
    );

    for (repository_alias, repo_entry) in &matching_repos {
        let swatch_style =
            match terminal_style::swatch_style_for_repo_color(repo_entry.repo_color()) {
                Ok(resolved_style) => resolved_style,
                Err(_color_error) => terminal_style::DEFAULT_SWATCH_STYLE,
            };
        let swatch = "██".style(swatch_style);
        let mut label = format!("{swatch} {repository_alias}");
        if !repo_entry.aliases().is_empty() {
            let aliases_joined = repo_entry.aliases().join(", ");
            label = format!("{label} ({aliases_joined})");
        }
        if !repo_entry.description().is_empty() {
            label = format!("{label} — {}", repo_entry.description());
        }
        if !repo_entry.tags().is_empty() {
            let tags_joined = repo_entry.tags().join(", ");
            label = format!("{label} [{tags_joined}]");
        }
        display_labels.push(label);
    }

    for (workspace_name, workspace_entry) in &workspace_rows_included {
        let uppercase_name = workspace_name.to_uppercase();
        let mut label = format!("▣▣ {uppercase_name}");
        if !workspace_entry.aliases().is_empty() {
            let aliases_joined = workspace_entry.aliases().join(", ");
            label = format!("{label} ({aliases_joined})");
        }
        if !workspace_entry.description().is_empty() {
            label = format!("{label} — {}", workspace_entry.description());
        }
        display_labels.push(format!("{label} [workspace]"));
    }

    let prompt_theme = terminal_style::reportal_prompt_theme();
    let selected_index = FuzzySelect::with_theme(&prompt_theme)
        .with_prompt(params.prompt_label)
        .items(&display_labels)
        .highlight_matches(false)
        .interact_opt()
        .map_err(|select_error| ReportalError::ConfigIoFailure {
            reason: select_error.to_string(),
        })?;

    let chosen_index = match selected_index {
        Some(index) => index,
        None => return Err(ReportalError::SelectionCancelled),
    };

    let repo_count = matching_repos.len();
    if chosen_index < repo_count {
        match matching_repos.get(chosen_index) {
            Some((chosen_repo_alias, _chosen_repo_entry)) => {
                Ok(SelectedTarget::Repo((*chosen_repo_alias).clone()))
            }
            None => Err(ReportalError::SelectionCancelled),
        }
    } else {
        let workspace_offset = chosen_index - repo_count;
        match workspace_rows_included.get(workspace_offset) {
            Some((chosen_workspace_name, _chosen_entry)) => {
                Ok(SelectedTarget::Workspace((*chosen_workspace_name).clone()))
            }
            None => Err(ReportalError::SelectionCancelled),
        }
    }
}
