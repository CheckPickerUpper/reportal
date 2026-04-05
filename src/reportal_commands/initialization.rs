/// Creates the default RePortal config file and installs shell integration.
///
/// This is the entry point for `rep init`. It delegates to `DetectedShell`
/// for the actual installation mechanics — PowerShell gets a module,
/// Unix shells get a profile source line.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use owo_colors::OwoColorize;

use super::detected_shell::{integration_file_path, DetectedShell};

#[cfg(target_os = "windows")]
use super::detected_shell::powershell_module_directory;

/// Writes the integration file and installs the source line in the
/// shell profile. Displays what will be installed and asks for confirmation.
fn install_or_update_shell_integration() -> Result<(), ReportalError> {
    println!();
    println!("  {} Shell integration:", ">>".style(terminal_style::LABEL_STYLE));
    println!("     {} jump to a repo (cd)", "rj".style(terminal_style::ALIAS_STYLE));
    println!("     {} open a repo in your editor", "ro".style(terminal_style::ALIAS_STYLE));
    println!("     {} open a repo in the browser", "rw".style(terminal_style::ALIAS_STYLE));
    println!("     {} run a configured command in a repo", "rr".style(terminal_style::ALIAS_STYLE));
    println!("     {} per-repo tab title + background color on every prompt", "color".style(terminal_style::ALIAS_STYLE));
    println!();

    let detected_shell = DetectedShell::detect();

    match detected_shell.profile_path() {
        Some(profile_path) => {
            #[cfg(target_os = "windows")]
            let install_target_label = match powershell_module_directory() {
                Ok(ref module_path) => format!("Module: {}", module_path.display()),
                Err(ref _module_path_error) => format!("Profile: {}", profile_path.display()),
            };

            #[cfg(not(target_os = "windows"))]
            let install_target_label = format!("Profile: {}", profile_path.display());

            println!(
                "  {}",
                install_target_label.style(terminal_style::PATH_STYLE),
            );

            let user_wants_install = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Install/update shell integration?")
                .default(true)
                .interact()
                .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                    reason: prompt_error.to_string(),
                })?;

            match user_wants_install {
                true => {
                    detected_shell.install_integration()?;

                    #[cfg(target_os = "windows")]
                    {
                        let script_path = integration_file_path()?;
                        let module_path = powershell_module_directory()?;
                        terminal_style::print_success(&format!(
                            "Wrote {}",
                            script_path.display(),
                        ));
                        terminal_style::print_success(&format!(
                            "Installed PowerShell module to {}",
                            module_path.display(),
                        ));
                        terminal_style::print_success(
                            "Migrated from profile sourcing to module auto-import.",
                        );
                        terminal_style::print_success(
                            "Open a new terminal to activate. rj/ro/rw/rr now load even if your $PROFILE has errors.",
                        );
                    }

                    #[cfg(not(target_os = "windows"))]
                    {
                        let script_path = integration_file_path()?;
                        terminal_style::print_success(&format!(
                            "Wrote {}",
                            script_path.display(),
                        ));
                        terminal_style::print_success(&format!(
                            "Updated {}. Restart your shell to activate.",
                            profile_path.display(),
                        ));
                    }
                }
                false => {
                    println!("  Skipped.");
                }
            }
        }
        None => {
            println!("  Could not detect your shell profile.");
            println!("  Add this to your profile manually:");
            println!();
            match integration_file_path() {
                Ok(script_path) => {
                    println!("  PowerShell:  . \"{}\"", script_path.display());
                    println!("  Bash/Zsh:    source \"{}\"", script_path.display());
                }
                Err(ref _path_error) => {
                    println!("  Run 'rep init' again after fixing the config directory.");
                }
            }
        }
    }

    return Ok(());
}

/// Creates a default config at `~/.reportal/config.toml` if none exists,
/// then writes the shell integration file and source line.
/// Safe to re-run at any time; idempotent on both config and profile.
pub fn run_init() -> Result<(), ReportalError> {
    let config_path = ReportalConfig::config_file_path()?;
    match config_path.exists() {
        true => {
            println!("Config already exists at {}", config_path.display());
            println!("Use 'reportal add' to register repos.");
        }
        false => {
            let default_config = ReportalConfig::create_default();
            default_config.save_to_disk()?;
            terminal_style::print_success(&format!("Created config at {}", config_path.display()));
        }
    }

    install_or_update_shell_integration()
}
