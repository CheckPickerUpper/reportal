/// Creates the default RePortal config file and installs shell integration.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::Command;

/// The PowerShell code that gets appended to the user's profile.
const POWERSHELL_INTEGRATION: &str = r#"
# RePortal shell integration
function rj { Set-Location (rep jump) }
function ro { rep open }
"#;

/// The Bash/Zsh code that gets appended to the user's profile.
#[cfg(not(target_os = "windows"))]
const BASH_INTEGRATION: &str = r#"
# RePortal shell integration
rj() { cd "$(rep jump)"; }
ro() { rep open; }
"#;

/// Which shell the user is running, with its profile path.
enum DetectedShell {
    /// PowerShell (Windows or cross-platform).
    PowerShell(PathBuf),
    /// Bash with a known profile path.
    #[cfg(not(target_os = "windows"))]
    Bash(PathBuf),
    /// Zsh with a known profile path.
    #[cfg(not(target_os = "windows"))]
    Zsh(PathBuf),
    /// Could not determine the shell or profile path.
    Unknown,
}

/// Whether pwsh was able to report its profile path.
enum PowerShellDetection {
    /// Profile path was returned by pwsh.
    Detected(String),
    /// pwsh failed to run or returned empty output.
    Unavailable,
}

/// Whether the SHELL env var was readable.
#[cfg(not(target_os = "windows"))]
enum ShellEnvDetection {
    /// SHELL env var was read successfully.
    Detected(String),
    /// SHELL env var was not set.
    Unavailable,
}

/// Attempts to get the PowerShell profile path by running pwsh.
fn detect_powershell_profile() -> PowerShellDetection {
    let profile_output = Command::new("pwsh")
        .args(["-NoProfile", "-Command", "echo $PROFILE"])
        .output();

    match profile_output {
        Ok(output_bytes) => match output_bytes.status.success() {
            true => {
                let profile_path_string = String::from_utf8_lossy(&output_bytes.stdout).trim().to_string();
                match profile_path_string.is_empty() {
                    true => PowerShellDetection::Unavailable,
                    false => PowerShellDetection::Detected(profile_path_string),
                }
            }
            false => PowerShellDetection::Unavailable,
        },
        Err(pwsh_spawn_error) => {
            eprintln!("  pwsh not found: {pwsh_spawn_error}");
            PowerShellDetection::Unavailable
        }
    }
}

/// Reads the SHELL environment variable to determine the Unix shell.
#[cfg(not(target_os = "windows"))]
fn detect_unix_shell_env() -> ShellEnvDetection {
    match std::env::var("SHELL") {
        Ok(shell_path) => ShellEnvDetection::Detected(shell_path),
        Err(env_read_error) => {
            eprintln!("  SHELL env not set: {env_read_error}");
            ShellEnvDetection::Unavailable
        }
    }
}

/// Detects the current shell and its profile path.
fn detect_shell_profile() -> DetectedShell {
    #[cfg(target_os = "windows")]
    {
        match detect_powershell_profile() {
            PowerShellDetection::Detected(profile_path) => DetectedShell::PowerShell(PathBuf::from(profile_path)),
            PowerShellDetection::Unavailable => DetectedShell::Unknown,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        match detect_unix_shell_env() {
            ShellEnvDetection::Detected(shell_path) => {
                match dirs::home_dir() {
                    Some(home_path) => {
                        if shell_path.contains("zsh") {
                            DetectedShell::Zsh(home_path.join(".zshrc"))
                        } else {
                            DetectedShell::Bash(home_path.join(".bashrc"))
                        }
                    }
                    None => DetectedShell::Unknown,
                }
            }
            ShellEnvDetection::Unavailable => DetectedShell::Unknown,
        }
    }
}

/// Checks if a file already contains the RePortal integration marker.
fn profile_already_has_integration(profile_path: &PathBuf) -> Result<bool, ReportalError> {
    if !profile_path.exists() {
        return Ok(false);
    }
    let profile_content = std::fs::read_to_string(profile_path).map_err(|io_error| {
        ReportalError::ConfigIoFailure {
            reason: io_error.to_string(),
        }
    })?;
    Ok(profile_content.contains("RePortal shell integration"))
}

/// Methods for installing shell integration based on detected shell type.
impl DetectedShell {
    /// Returns the integration code for this shell type.
    fn integration_code(&self) -> &'static str {
        match self {
            DetectedShell::PowerShell(_) => POWERSHELL_INTEGRATION,
            #[cfg(not(target_os = "windows"))]
            DetectedShell::Bash(_) | DetectedShell::Zsh(_) => BASH_INTEGRATION,
            DetectedShell::Unknown => "",
        }
    }

    /// Returns the profile path if this shell was detected.
    fn profile_path(&self) -> Option<&PathBuf> {
        match self {
            DetectedShell::PowerShell(ref shell_profile_path) => Some(shell_profile_path),
            #[cfg(not(target_os = "windows"))]
            DetectedShell::Bash(ref shell_profile_path)
            | DetectedShell::Zsh(ref shell_profile_path) => Some(shell_profile_path),
            DetectedShell::Unknown => None,
        }
    }

    /// Appends shell integration code to this shell's profile file.
    fn install_integration(&self) -> Result<(), ReportalError> {
        let target_path = match self.profile_path() {
            Some(resolved_path) => resolved_path,
            None => return Ok(()),
        };

        let mut profile_content = match target_path.exists() {
            true => std::fs::read_to_string(target_path).map_err(|io_error| {
                ReportalError::ConfigIoFailure {
                    reason: io_error.to_string(),
                }
            })?,
            false => String::new(),
        };

        profile_content.push_str(self.integration_code());

        std::fs::write(target_path, profile_content).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;

        Ok(())
    }
}

/// Writes a default config to `~/.reportal/config.toml` if none exists,
/// then offers to install shell integration (`rj` and `ro` shortcuts).
pub fn run_init() -> Result<(), ReportalError> {
    let config_path = ReportalConfig::config_file_path()?;
    if config_path.exists() {
        println!("Config already exists at {}", config_path.display());
        println!("Use 'reportal add' to register repos.");
    } else {
        let default_config = ReportalConfig::create_default();
        default_config.save_to_disk()?;
        terminal_style::print_success(&format!("Created config at {}", config_path.display()));
    }

    println!();
    println!("  {} Shell shortcuts:", ">>".style(terminal_style::LABEL_STYLE));
    println!("     {} jump to a repo (cd)", "rj".style(terminal_style::ALIAS_STYLE));
    println!("     {} open a repo in your editor", "ro".style(terminal_style::ALIAS_STYLE));
    println!();

    let detected_shell = detect_shell_profile();

    match detected_shell.profile_path() {
        Some(profile_path) => {
            let already_installed = profile_already_has_integration(profile_path)?;

            match already_installed {
                true => {
                    terminal_style::print_success("Shell integration already installed.");
                }
                false => {
                    println!(
                        "  Profile: {}",
                        profile_path.display().to_string().style(terminal_style::PATH_STYLE),
                    );

                    let user_wants_install = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Install shell shortcuts (rj, ro)?")
                        .default(true)
                        .interact()
                        .map_err(|prompt_error| ReportalError::ConfigIoFailure {
                            reason: prompt_error.to_string(),
                        })?;

                    match user_wants_install {
                        true => {
                            detected_shell.install_integration()?;
                            terminal_style::print_success(&format!(
                                "Added rj/ro to {}. Restart your shell to activate.",
                                profile_path.display(),
                            ));
                        }
                        false => {
                            println!("  Skipped. You can add them manually later.");
                        }
                    }
                }
            }
        }
        None => {
            println!("  Could not detect your shell profile.");
            println!("  Add these manually:");
            println!();
            println!("  PowerShell:");
            println!("    function rj {{ Set-Location (rep jump) }}");
            println!("    function ro {{ rep open }}");
            println!();
            println!("  Bash/Zsh:");
            println!("    rj() {{ cd \"$(rep jump)\"; }}");
            println!("    ro() {{ rep open; }}");
        }
    }

    Ok(())
}
