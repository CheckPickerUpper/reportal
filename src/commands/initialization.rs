/// Creates the default RePortal config file for first-time setup.

use crate::reportal_config::ReportalConfig;
use crate::error::ReportalError;

/// Writes a default config to `~/.reportal/config.toml` if none exists.
///
/// Prints the config path on success, or informs the user if one
/// already exists and suggests using `reportal add` instead.
pub fn run_init() -> Result<(), ReportalError> {
    let config_path = ReportalConfig::config_file_path()?;
    if config_path.exists() {
        println!("Config already exists at {}", config_path.display());
        println!("Use 'reportal add' to register repos.");
        return Ok(());
    }

    let default_config = ReportalConfig::create_default();
    default_config.save_to_disk()?;
    println!("Created config at {}", config_path.display());
    println!("Use 'reportal add <path>' to register your repos.");
    Ok(())
}
