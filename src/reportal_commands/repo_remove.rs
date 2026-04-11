//! Removes a registered repo from the `RePortal` config by alias.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;

/// Unregisters a repo by its alias. Does not delete any files on disk.
///
/// Loads the config, removes the entry, and saves back to disk.
/// Prints confirmation with the path that was unregistered.
pub fn run_remove(repo_alias: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    let removed_entry = loaded_config.remove_repo(repo_alias)?;
    loaded_config.save_to_disk()?;
    terminal_style::print_success(&format!("Removed '{}' ({})", repo_alias, removed_entry.raw_path()));
    Ok(())
}
