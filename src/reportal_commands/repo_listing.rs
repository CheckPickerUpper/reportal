//! Lists all registered repos with their status, description, and tags.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Prints a formatted list of all repos matching the given tag filter.
///
/// Each repo shows its alias as a bold header, followed by labeled
/// path, description, tags, and whether the directory exists on disk.
pub fn run_list(tag_filter: TagFilter) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;
    let matching_repos = loaded_config.repos_matching_tag_filter(&tag_filter);

    if matching_repos.is_empty() {
        match tag_filter {
            TagFilter::ByTag(target_tag) => terminal_style::write_stdout(&format!("No repos found with tag '{target_tag}'\n")),
            TagFilter::All => terminal_style::write_stdout("No repos registered. Use 'reportal add <path>' to add one.\n"),
        }
        return Ok(());
    }

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {}\n",
        "RePortal".style(terminal_style::EMPHASIS_STYLE)
    ));
    terminal_style::write_stdout("\n");

    for (alias, repo) in &matching_repos {
        let directory_exists = repo.resolved_path().exists();

        let swatch_style = terminal_style::swatch_style_for_repo_color(repo.repo_color())?;
        let uppercase_alias = alias.to_uppercase();
        terminal_style::write_stdout(&format!(
            "  {} {}\n",
            "██".style(swatch_style),
            uppercase_alias.style(terminal_style::ALIAS_STYLE),
        ));

        terminal_style::write_stdout(&format!(
            "     {} {}\n",
            "Path:".style(terminal_style::LABEL_STYLE),
            repo.raw_path().style(terminal_style::PATH_STYLE),
        ));

        if !repo.description().is_empty() {
            terminal_style::write_stdout(&format!(
                "     {} {}\n",
                "Desc:".style(terminal_style::LABEL_STYLE),
                repo.description(),
            ));
        }

        if !repo.aliases().is_empty() {
            let formatted_aliases = repo.aliases().join(", ");
            terminal_style::write_stdout(&format!(
                "     {} {}\n",
                "Aliases:".style(terminal_style::LABEL_STYLE),
                formatted_aliases.style(terminal_style::PATH_STYLE),
            ));
        }

        if !repo.tags().is_empty() {
            let formatted_tags = repo.tags().join(", ");
            terminal_style::write_stdout(&format!(
                "     {} {}\n",
                "Tags:".style(terminal_style::LABEL_STYLE),
                formatted_tags.style(terminal_style::TAG_STYLE),
            ));
        }

        let found_label = if directory_exists { "yes".style(terminal_style::SUCCESS_STYLE).to_string() } else { "no".style(terminal_style::FAILURE_STYLE).to_string() };
        terminal_style::write_stdout(&format!(
            "     {} {}\n",
            "Found:".style(terminal_style::LABEL_STYLE),
            found_label,
        ));

        terminal_style::write_stdout("\n");
    }

    terminal_style::write_stdout(&format!(
        "  {} repos total\n",
        matching_repos.len().style(terminal_style::EMPHASIS_STYLE),
    ));
    terminal_style::write_stdout("\n");
    Ok(())
}
