/// Lists all registered repos with their status, description, and tags.

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
            TagFilter::ByTag(target_tag) => println!("No repos found with tag '{target_tag}'"),
            TagFilter::All => println!("No repos registered. Use 'reportal add <path>' to add one."),
        }
        return Ok(());
    }

    println!();
    println!(
        "  {}",
        "RePortal".style(terminal_style::EMPHASIS_STYLE)
    );
    println!();

    for (alias, repo) in &matching_repos {
        let directory_exists = repo.resolved_path().exists();

        let swatch_style = terminal_style::swatch_style_for_repo_color(repo.repo_color())?;
        let uppercase_alias = alias.to_uppercase();
        println!(
            "  {} {}",
            "██".style(swatch_style),
            uppercase_alias.style(terminal_style::ALIAS_STYLE),
        );

        println!(
            "     {} {}",
            "Path:".style(terminal_style::LABEL_STYLE),
            repo.raw_path().style(terminal_style::PATH_STYLE),
        );

        if !repo.description().is_empty() {
            println!(
                "     {} {}",
                "Desc:".style(terminal_style::LABEL_STYLE),
                repo.description(),
            );
        }

        if !repo.aliases().is_empty() {
            let formatted_aliases = repo.aliases().join(", ");
            println!(
                "     {} {}",
                "Aliases:".style(terminal_style::LABEL_STYLE),
                formatted_aliases.style(terminal_style::PATH_STYLE),
            );
        }

        if !repo.tags().is_empty() {
            let formatted_tags = repo.tags().join(", ");
            println!(
                "     {} {}",
                "Tags:".style(terminal_style::LABEL_STYLE),
                formatted_tags.style(terminal_style::TAG_STYLE),
            );
        }

        let found_label = match directory_exists {
            true => "yes".style(terminal_style::SUCCESS_STYLE).to_string(),
            false => "no".style(terminal_style::FAILURE_STYLE).to_string(),
        };
        println!(
            "     {} {}",
            "Found:".style(terminal_style::LABEL_STYLE),
            found_label,
        );

        println!();
    }

    println!(
        "  {} repos total",
        matching_repos.len().style(terminal_style::EMPHASIS_STYLE),
    );
    println!();
    Ok(())
}
