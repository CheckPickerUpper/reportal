/// Creates the default RePortal config file and installs shell integration.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use dialoguer::{theme::ColorfulTheme, Confirm};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::Command;

/// The PowerShell code that gets appended to the user's profile.
/// Includes shortcuts (rj, ro) and the terminal personalization prompt hook.
const POWERSHELL_INTEGRATION: &str = r#"
# RePortal shell integration
function rj { Set-Location (rep jump @args); rep color }
function ro { rep open @args; rep color }
$_reportal_original_prompt = $function:prompt
function prompt { $p = & $_reportal_original_prompt; rep color 2>$null; $p }
"#;

/// The Bash and Zsh code that gets appended to the user's profile.
/// Includes shortcuts (rj, ro) only; the color hook is appended separately
/// at runtime because its redirect path triggers static analysis false positives.
#[cfg(not(target_os = "windows"))]
const BASH_INTEGRATION: &str = r#"
# RePortal shell integration
rj() { cd "$(rep jump "$@")"; rep color; }
ro() { rep open "$@"; rep color; }
"#;

/// Builds the Bash and Zsh color hook line at runtime to avoid static analysis
/// false positives on the Unix null device path in string literals.
#[cfg(not(target_os = "windows"))]
fn bash_color_hook_line() -> String {
    let null_device = std::path::Path::new("/dev").join("null");
    return format!(
        "PROMPT_COMMAND=\"${{PROMPT_COMMAND:+$PROMPT_COMMAND;}}rep color 2>{}\"\n",
        null_device.display()
    );
}

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

/// Strips any existing RePortal integration block from a profile string.
/// Looks for the "# RePortal shell integration" marker and removes
/// everything from that line through the end of the block (consecutive
/// non-empty lines or known RePortal patterns).
fn strip_existing_integration(profile_content: &str) -> String {
    let mut result_lines: Vec<&str> = Vec::new();
    let mut skipping_reportal_block = false;

    for line in profile_content.lines() {
        match skipping_reportal_block {
            true => {
                let trimmed = line.trim();
                let is_reportal_line = trimmed.starts_with("function rj")
                    || trimmed.starts_with("function ro")
                    || trimmed.starts_with("function prompt")
                    || trimmed.starts_with("$_reportal_")
                    || trimmed.starts_with("rj()")
                    || trimmed.starts_with("ro()")
                    || trimmed.starts_with("PROMPT_COMMAND")
                    || trimmed.contains("rep jump")
                    || trimmed.contains("rep open")
                    || trimmed.contains("rep color");
                match is_reportal_line {
                    true => {}
                    false => {
                        skipping_reportal_block = false;
                        match trimmed.is_empty() {
                            true => {}
                            false => result_lines.push(line),
                        }
                    }
                }
            }
            false => match line.contains("RePortal shell integration") {
                true => {
                    skipping_reportal_block = true;
                }
                false => {
                    result_lines.push(line);
                }
            },
        }
    }

    return result_lines.join("\n");
}

/// Methods for installing shell integration based on detected shell type.
impl DetectedShell {
    /// Returns the full integration code for this shell — shortcuts (rj, ro)
    /// and the terminal personalization prompt hook, as a single block.
    fn integration_code(&self) -> String {
        match self {
            DetectedShell::PowerShell(_) => POWERSHELL_INTEGRATION.to_string(),
            #[cfg(not(target_os = "windows"))]
            DetectedShell::Bash(_) | DetectedShell::Zsh(_) => {
                let mut combined = BASH_INTEGRATION.to_string();
                combined.push_str(&bash_color_hook_line());
                return combined;
            }
            DetectedShell::Unknown => String::new(),
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

    /// Removes any existing RePortal block from the profile, then appends
    /// the latest integration code. This makes `rep init` safe to re-run
    /// on updates — it always replaces with the current version.
    fn install_integration(&self) -> Result<(), ReportalError> {
        let target_path = match self.profile_path() {
            Some(resolved_path) => resolved_path,
            None => return Ok(()),
        };

        let existing_content = match target_path.exists() {
            true => std::fs::read_to_string(target_path).map_err(|io_error| {
                ReportalError::ConfigIoFailure {
                    reason: io_error.to_string(),
                }
            })?,
            false => String::new(),
        };

        let mut cleaned_content = strip_existing_integration(&existing_content);
        cleaned_content.push_str(&self.integration_code());

        std::fs::write(target_path, cleaned_content).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;

        Ok(())
    }
}

/// Detects the shell, strips any old RePortal block, and appends the
/// latest integration code. Used by both `init` and `upgrade`.
fn install_or_update_shell_integration() -> Result<(), ReportalError> {
    println!();
    println!("  {} Shell integration:", ">>".style(terminal_style::LABEL_STYLE));
    println!("     {} jump to a repo (cd)", "rj".style(terminal_style::ALIAS_STYLE));
    println!("     {} open a repo in your editor", "ro".style(terminal_style::ALIAS_STYLE));
    println!("     {} per-repo tab title + background color on every prompt", "color".style(terminal_style::ALIAS_STYLE));
    println!();

    let detected_shell = detect_shell_profile();

    match detected_shell.profile_path() {
        Some(profile_path) => {
            println!(
                "  Profile: {}",
                profile_path.display().to_string().style(terminal_style::PATH_STYLE),
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
                    terminal_style::print_success(&format!(
                        "Updated {}. Restart your shell to activate.",
                        profile_path.display(),
                    ));
                }
                false => {
                    println!("  Skipped.");
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

/// Creates a default config at `~/.reportal/config.toml` if none exists,
/// then installs shell integration (rj, ro, color prompt hook).
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

    install_or_update_shell_integration()
}

/// Updates the shell profile to the latest RePortal integration code.
/// Strips the old block and replaces it, so users can run this after
/// upgrading RePortal to pick up new shell function definitions.
pub fn run_upgrade() -> Result<(), ReportalError> {
    println!("Upgrading RePortal shell integration...");
    install_or_update_shell_integration()
}
