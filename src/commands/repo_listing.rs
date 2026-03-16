/// Lists all registered repos with their status, description, and tags.

use crate::error::ReportalError;
use crate::reportal_config::{ReportalConfig, TagFilter};
use colored::Colorize;

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

    for (alias, repo) in &matching_repos {
        let directory_exists = repo.resolved_path().exists();

        let existence_label = match directory_exists {
            true => "ok".green().to_string(),
            false => "missing".red().to_string(),
        };

        println!("  {} [{}]", alias.bold(), existence_label);
        println!("    {}", repo.raw_path().dimmed());
        if !repo.description().is_empty() {
            println!("    {}", repo.description());
        }
        if !repo.tags().is_empty() {
            println!("    tags: {}", repo.tags().join(", ").dimmed());
        }
        println!();
    }

    println!("{} repos total", matching_repos.len());
    Ok(())
}
