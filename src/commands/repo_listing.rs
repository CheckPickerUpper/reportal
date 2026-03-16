/// Lists all registered repos with their status, description, and tags.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Prints a formatted list of all repos matching the given tag filter.
///
/// Each repo shows its alias, whether the path exists on disk,
/// the raw path, description, and tags. Prints a total count at the end.
pub fn run_list(tag_filter: TagFilter) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;
    let matching_repos = loaded_config.repos_matching_tag_filter(&tag_filter);

    if matching_repos.is_empty() {
        match tag_filter {
            TagFilter::ByTag(target_tag) => println!("No repos found with tag '{target_tag}'"),
            TagFilter::All => println!("No repos registered. Use 'reportal add <path>' to add one."),
        }
        return Ok(());
    }

    println!();
    for (alias, repo) in &matching_repos {
        let directory_exists = repo.resolved_path().exists();

        let existence_label = match directory_exists {
            true => "ok".style(terminal_style::SUCCESS_STYLE).to_string(),
            false => "missing".style(terminal_style::FAILURE_STYLE).to_string(),
        };

        println!(
            "  {} {}",
            alias.style(terminal_style::ALIAS_STYLE),
            format!("[{}]", existence_label).dimmed(),
        );
        println!("    {}", repo.raw_path().style(terminal_style::PATH_STYLE));
        if !repo.description().is_empty() {
            println!("    {}", repo.description());
        }
        if !repo.tags().is_empty() {
            let formatted_tags = repo.tags().join(", ");
            println!("    {}", formatted_tags.style(terminal_style::TAG_STYLE));
        }
        println!();
    }

    println!(
        "  {} repos total",
        matching_repos.len().style(terminal_style::EMPHASIS_STYLE)
    );
    println!();
    Ok(())
}
