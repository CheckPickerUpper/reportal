//! Creates the default `RePortal` config file and installs shell integration.
//!
//! This is the entry point for `rep init`. It delegates to `DetectedShell`
//! for the actual installation mechanics — `PowerShell` gets a module,
//! Unix shells get a profile source line.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use owo_colors::OwoColorize;

use super::detected_shell::{integration_file_path, DetectedShell};

#[cfg(target_os = "windows")]
use super::detected_shell::powershell_module_directory;

/// Prints success messages after shell integration installation.
#[cfg(target_os = "windows")]
fn print_post_install_messages(_profile_path: &std::path::Path) -> Result<(), ReportalError> {
    let script_path = integration_file_path()?;
    let module_path = powershell_module_directory()?;
    terminal_style::print_success(&format!("Wrote {}", script_path.display()));
    terminal_style::print_success(&format!("Installed PowerShell module to {}", module_path.display()));
    terminal_style::print_success("Migrated from profile sourcing to module auto-import.");
    terminal_style::print_success("Open a new terminal to activate. rj/ro/rw/rr now load even if your $PROFILE has errors.");
    Ok(())
}

/// Prints success messages after shell integration installation.
#[cfg(not(target_os = "windows"))]
fn print_post_install_messages(profile_path: &std::path::Path) -> Result<(), ReportalError> {
    let script_path = integration_file_path()?;
    terminal_style::print_success(&format!("Wrote {}", script_path.display()));
    terminal_style::print_success(&format!("Updated {}. Restart your shell to activate.", profile_path.display()));
    Ok(())
}

/// Writes the integration file and installs the source line in the
/// shell profile. Displays what will be installed and asks for confirmation.
fn install_or_update_shell_integration() -> Result<(), ReportalError> {
    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!("  {} Shell integration:\n", ">>".style(terminal_style::LABEL_STYLE)));
    terminal_style::write_stdout(&format!("     {} jump to a repo (cd)\n", "rj".style(terminal_style::ALIAS_STYLE)));
    terminal_style::write_stdout(&format!("     {} open a repo in your editor\n", "ro".style(terminal_style::ALIAS_STYLE)));
    terminal_style::write_stdout(&format!("     {} open a repo in the browser\n", "rw".style(terminal_style::ALIAS_STYLE)));
    terminal_style::write_stdout(&format!("     {} run a configured command in a repo\n", "rr".style(terminal_style::ALIAS_STYLE)));
    terminal_style::write_stdout(&format!("     {} per-repo tab title + background color on every prompt\n", "color".style(terminal_style::ALIAS_STYLE)));
    terminal_style::write_stdout("\n");

    let detected_shell = DetectedShell::detect();

    if let Some(profile_path) = detected_shell.profile_path() {
        #[cfg(target_os = "windows")]
        let install_target_label = match powershell_module_directory() {
            Ok(ref module_path) => format!("Module: {}", module_path.display()),
            Err(ref _module_path_error) => format!("Profile: {}", profile_path.display()),
        };

        #[cfg(not(target_os = "windows"))]
        let install_target_label = format!("Profile: {}", profile_path.display());

        terminal_style::write_stdout(&format!(
            "  {}\n",
            install_target_label.style(terminal_style::PATH_STYLE),
        ));

        let user_wants_install = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Install/update shell integration?")
            .default(true)
            .interact()
            .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                reason: prompt_error.to_string(),
            })?;

        if user_wants_install {
            detected_shell.install_integration()?;
            print_post_install_messages(profile_path)?;
        } else {
            terminal_style::write_stdout("  Skipped.\n");
        }
    } else {
        terminal_style::write_stdout("  Could not detect your shell profile.\n");
        terminal_style::write_stdout("  Add this to your profile manually:\n");
        terminal_style::write_stdout("\n");
        match integration_file_path() {
            Ok(script_path) => {
                terminal_style::write_stdout(&format!("  PowerShell:  . \"{}\"\n", script_path.display()));
                terminal_style::write_stdout(&format!("  Bash/Zsh:    source \"{}\"\n", script_path.display()));
            }
            Err(ref _path_error) => {
                terminal_style::write_stdout("  Run 'rep init' again after fixing the config directory.\n");
            }
        }
    }

    Ok(())
}

/// Creates a default config at `~/.reportal/config.toml` if none exists,
/// then writes the shell integration file and source line.
/// Safe to re-run at any time; idempotent on both config and profile.
pub fn run_init() -> Result<(), ReportalError> {
    let config_path = ReportalConfig::config_file_path()?;
    if config_path.exists() {
        terminal_style::write_stdout(&format!("Config already exists at {}\n", config_path.display()));
        terminal_style::write_stdout("Use 'reportal add' to register repos.\n");
    } else {
        let default_config = ReportalConfig::create_default();
        default_config.save_to_disk()?;
        terminal_style::print_success(&format!("Created config at {}", config_path.display()));
    }

    install_or_update_shell_integration()
}
