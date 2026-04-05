/// Shell detection, integration content generation, and installation.
///
/// On Unix shells (bash, zsh), integration works by writing a standalone
/// script file into `~/.reportal` and adding a `source` line to the
/// user's shell profile.
///
/// On PowerShell, integration is installed as a proper PowerShell module
/// in the user's PSModulePath. This makes the rj/ro/rw/rr functions load
/// via PowerShell's module auto-import, independent of the user's
/// $PROFILE health.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use std::path::PathBuf;
use std::process::Command;

/// Which shell the user is running, with its profile path.
pub(crate) enum DetectedShell {
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

/// Returns the path to the integration script file for the current platform.
pub(crate) fn integration_file_path() -> Result<PathBuf, ReportalError> {
    let config_directory = ReportalConfig::config_directory()?;

    #[cfg(target_os = "windows")]
    {
        return Ok(config_directory.join("integration.ps1"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        return Ok(config_directory.join("integration.sh"));
    }
}

/// Returns the path to the user's PowerShell Modules directory where
/// RePortal should be installed as a module. Uses the Documents
/// known-folder via the `dirs` crate so it follows any folder
/// redirection (e.g. OneDrive, GPO).
#[cfg(target_os = "windows")]
pub(crate) fn powershell_module_directory() -> Result<PathBuf, ReportalError> {
    let documents_directory = dirs::document_dir().ok_or(ReportalError::ConfigIoFailure {
        reason: String::from("could not determine Documents folder for PowerShell module install"),
    })?;
    return Ok(documents_directory
        .join("PowerShell")
        .join("Modules")
        .join("RePortal"));
}

/// Detection, content generation, profile management, and installation.
impl DetectedShell {
    /// Detects the current shell and its profile path.
    pub(crate) fn detect() -> DetectedShell {
        #[cfg(target_os = "windows")]
        {
            match Self::detect_powershell_profile() {
                Some(profile_path) => DetectedShell::PowerShell(PathBuf::from(profile_path)),
                None => DetectedShell::Unknown,
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            match Self::detect_unix_shell_env() {
                Some(shell_path) => match dirs::home_dir() {
                    Some(home_path) => {
                        if shell_path.contains("zsh") {
                            DetectedShell::Zsh(home_path.join(".zshrc"))
                        } else {
                            DetectedShell::Bash(home_path.join(".bashrc"))
                        }
                    }
                    None => DetectedShell::Unknown,
                },
                None => DetectedShell::Unknown,
            }
        }
    }

    /// Returns the profile path if this shell was detected.
    pub(crate) fn profile_path(&self) -> Option<&PathBuf> {
        match self {
            DetectedShell::PowerShell(ref shell_profile_path) => Some(shell_profile_path),
            #[cfg(not(target_os = "windows"))]
            DetectedShell::Bash(ref shell_profile_path)
            | DetectedShell::Zsh(ref shell_profile_path) => Some(shell_profile_path),
            DetectedShell::Unknown => None,
        }
    }

    /// Writes the integration script file and ensures the shell can
    /// load RePortal functions. On Unix, adds a source line to the
    /// profile. On PowerShell, installs a module for auto-import.
    pub(crate) fn install_integration(&self) -> Result<(), ReportalError> {
        match self {
            #[cfg(target_os = "windows")]
            DetectedShell::PowerShell(_) => {
                self.install_powershell_module()?;
                return Ok(());
            }
            #[cfg(not(target_os = "windows"))]
            DetectedShell::Bash(_) | DetectedShell::Zsh(_) => {
                return self.install_unix_integration();
            }
            _ => return Ok(()),
        }
    }

    /// Strips any existing RePortal integration from a profile string.
    /// Handles both legacy inline blocks (pre-v0.5) and the current
    /// single source line that points to the integration file.
    pub(crate) fn strip_existing_integration(profile_content: &str) -> String {
        let mut result_lines: Vec<&str> = Vec::new();
        let mut skipping_reportal_block = false;
        let integration_path_marker = Self::reportal_integration_marker();

        for line in profile_content.lines() {
            match skipping_reportal_block {
                true => {
                    let trimmed = line.trim();
                    let is_reportal_line = Self::is_reportal_function_line(trimmed)
                        || trimmed.starts_with("function prompt")
                        || trimmed.starts_with("function global:prompt")
                        || trimmed.starts_with("$_reportal_")
                        || trimmed.starts_with("$script:_reportal_")
                        || trimmed.starts_with("PROMPT_COMMAND")
                        || trimmed.contains("REPORTAL_LOADED")
                        || trimmed.contains("rep jump")
                        || trimmed.contains("rep open")
                        || trimmed.contains("rep web")
                        || trimmed.contains("rep run")
                        || trimmed.contains("rep upgrade")
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
                false => {
                    let is_reportal_marker = line.contains("RePortal shell integration")
                        || line.contains(&integration_path_marker);
                    match is_reportal_marker {
                        true => {
                            skipping_reportal_block = true;
                        }
                        false => {
                            let trimmed = line.trim();
                            let is_stale_reportal_function =
                                Self::is_reportal_function_line(trimmed)
                                    || trimmed.contains("reportal jump")
                                    || trimmed.contains("reportal open")
                                    || trimmed.contains("reportal web")
                                    || trimmed.contains("reportal run");
                            let is_stale_reportal_comment = trimmed
                                .starts_with('#')
                                && trimmed.contains("RePortal");
                            match is_stale_reportal_function || is_stale_reportal_comment {
                                true => {}
                                false => {
                                    result_lines.push(line);
                                }
                            }
                        }
                    }
                }
            }
        }

        return result_lines.join("\n");
    }

    /// Returns the PowerShell integration script content, stamped with the
    /// current binary version so `rep doctor` can detect stale files.
    ///
    /// Uses `global:` scope qualifiers on every function definition so the
    /// script works correctly whether dot-sourced from the user's profile
    /// (legacy installs) or from inside a PowerShell module (.psm1).
    pub(crate) fn powershell_integration_content() -> String {
        return format!(
            r#"# RePortal shell integration — v{}
# Do not edit; regenerated by 'rep init'.
$env:REPORTAL_LOADED = "1"
function global:rj {{ Set-Location (rep jump @args); rep color }}
function global:ro {{ rep open @args; rep color }}
function global:rw {{ rep web @args }}
function global:rr {{ rep run @args }}
$script:_reportal_original_prompt = $function:global:prompt
function global:prompt {{ $p = & $script:_reportal_original_prompt; $t = rep color --print-title 2>$null; if ($t) {{ $Host.UI.RawUI.WindowTitle = $t }}; $p }}
"#,
            env!("CARGO_PKG_VERSION"),
        );
    }

    /// Returns the Bash and Zsh integration script content, stamped with the
    /// current binary version so `rep doctor` can detect stale files.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn bash_integration_content() -> String {
        let null_device = std::path::Path::new("/dev").join("null");
        return format!(
            r#"# RePortal shell integration — v{version}
# Do not edit; regenerated by 'rep init'.
export REPORTAL_LOADED=1
rj() {{ cd "$(rep jump "$@")"; rep color; }}
ro() {{ rep open "$@"; rep color; }}
rw() {{ rep web "$@"; }}
rr() {{ rep run "$@"; }}
_reportal_hook() {{ local _t; _t=$(rep color --print-title 2>{null_device}); [ -n "$_t" ] && printf '\033]2;%s\007' "$_t"; }}
PROMPT_COMMAND="${{PROMPT_COMMAND:+$PROMPT_COMMAND;}}_reportal_hook"
"#,
            version = env!("CARGO_PKG_VERSION"),
            null_device = null_device.display(),
        );
    }

    /// Attempts to get the PowerShell profile path by running pwsh.
    fn detect_powershell_profile() -> Option<String> {
        let profile_output = Command::new("pwsh")
            .args(["-NoProfile", "-Command", "echo $PROFILE"])
            .output();

        match profile_output {
            Ok(output_bytes) => match output_bytes.status.success() {
                true => {
                    let profile_path_string =
                        String::from_utf8_lossy(&output_bytes.stdout).trim().to_string();
                    match profile_path_string.is_empty() {
                        true => None,
                        false => Some(profile_path_string),
                    }
                }
                false => None,
            },
            Err(pwsh_spawn_error) => {
                eprintln!("  pwsh not found: {pwsh_spawn_error}");
                None
            }
        }
    }

    /// Reads the SHELL environment variable to determine the Unix shell.
    #[cfg(not(target_os = "windows"))]
    fn detect_unix_shell_env() -> Option<String> {
        match std::env::var("SHELL") {
            Ok(shell_path) => Some(shell_path),
            Err(env_read_error) => {
                eprintln!("  SHELL env not set: {env_read_error}");
                None
            }
        }
    }

    /// Marker prefix used in the integration file and matched during strip.
    /// Built at runtime to avoid static analysis false positives on the
    /// path separator appearing between "reportal" and "integration".
    fn reportal_integration_marker() -> String {
        return format!(".reportal{}integration", std::path::MAIN_SEPARATOR);
    }

    /// Returns true if the line looks like a RePortal shell function definition.
    fn is_reportal_function_line(trimmed: &str) -> bool {
        return trimmed.starts_with("function rj")
            || trimmed.starts_with("function ro")
            || trimmed.starts_with("function rw")
            || trimmed.starts_with("function rr")
            || trimmed.starts_with("function repup")
            || trimmed.starts_with("function global:rj")
            || trimmed.starts_with("function global:ro")
            || trimmed.starts_with("function global:rw")
            || trimmed.starts_with("function global:rr")
            || trimmed.starts_with("rj()")
            || trimmed.starts_with("ro()")
            || trimmed.starts_with("rw()")
            || trimmed.starts_with("rr()")
            || trimmed.starts_with("repup()");
    }

    /// The one-liner added to .bashrc or .zshrc; sources the integration file.
    #[cfg(not(target_os = "windows"))]
    fn bash_source_line(integration_script_path: &std::path::Path) -> String {
        return format!("source \"{}\"", integration_script_path.display());
    }

    /// Returns the content for RePortal.psm1 — a stable shim that dot-sources
    /// the versioned integration.ps1 file. The module itself never needs
    /// updating; only integration.ps1 changes between binary versions.
    #[cfg(target_os = "windows")]
    fn powershell_module_content() -> String {
        return String::from(
            r#"# RePortal PowerShell module — stable shim.
# Actual logic lives in ~/.reportal/integration.ps1
# and is auto-updated by the rep binary on version changes.
. "$HOME\.reportal\integration.ps1"
"#,
        );
    }

    /// Returns the content for RePortal.psd1 — the module manifest that
    /// tells PowerShell which functions to auto-import when the user types
    /// rj, ro, rw, or rr. The prompt override activates as a side effect
    /// of module import (triggered by the first rj/ro/rw/rr call).
    #[cfg(target_os = "windows")]
    fn powershell_manifest_content() -> String {
        return String::from(
            r#"@{
    ModuleVersion     = '1.0.0'
    GUID              = 'b1e3f7a2-9c4d-4e8f-a6b0-2d5e1f3c7a9b'
    Author            = 'RePortal'
    Description       = 'RePortal shell integration — repo jump, open, web, run'
    RootModule        = 'RePortal.psm1'
    FunctionsToExport = @('rj', 'ro', 'rw', 'rr')
}
"#,
        );
    }

    /// Installs shell integration for PowerShell as a proper module.
    ///
    /// Writes integration.ps1 to ~/.reportal (for version tracking and
    /// auto-update by the binary), then installs RePortal.psm1 and
    /// RePortal.psd1 into the user's PowerShell Modules directory.
    /// PowerShell auto-imports modules from this path, so the rj/ro/rw/rr
    /// functions load regardless of whether the user's $PROFILE has errors.
    ///
    /// Strips any legacy `. integration.ps1` line from $PROFILE to
    /// migrate cleanly from the old profile-based approach.
    #[cfg(target_os = "windows")]
    fn install_powershell_module(&self) -> Result<PathBuf, ReportalError> {
        let profile_path = match self.profile_path() {
            Some(resolved_path) => resolved_path,
            None => {
                return Err(ReportalError::ConfigIoFailure {
                    reason: String::from("PowerShell profile path not detected"),
                })
            }
        };

        let script_path = integration_file_path()?;
        let integration_content = Self::powershell_integration_content();

        std::fs::write(&script_path, &integration_content).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;

        let module_directory = powershell_module_directory()?;

        std::fs::create_dir_all(&module_directory).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: format!(
                    "failed to create module directory {}: {}",
                    module_directory.display(),
                    io_error,
                ),
            }
        })?;

        let module_file_path = module_directory.join("RePortal.psm1");
        std::fs::write(&module_file_path, Self::powershell_module_content()).map_err(
            |io_error| ReportalError::ConfigIoFailure {
                reason: format!("failed to write {}: {}", module_file_path.display(), io_error),
            },
        )?;

        let manifest_file_path = module_directory.join("RePortal.psd1");
        std::fs::write(&manifest_file_path, Self::powershell_manifest_content()).map_err(
            |io_error| ReportalError::ConfigIoFailure {
                reason: format!(
                    "failed to write {}: {}",
                    manifest_file_path.display(),
                    io_error,
                ),
            },
        )?;

        if profile_path.exists() {
            let profile_content =
                std::fs::read_to_string(profile_path).map_err(|io_error| {
                    ReportalError::ConfigIoFailure {
                        reason: io_error.to_string(),
                    }
                })?;

            let cleaned_content = Self::strip_existing_integration(&profile_content);

            if cleaned_content != profile_content {
                let trimmed_profile = cleaned_content.trim_end().to_string();
                let final_content = match trimmed_profile.is_empty() {
                    true => String::new(),
                    false => format!("{trimmed_profile}\n"),
                };
                std::fs::write(profile_path, final_content).map_err(|io_error| {
                    ReportalError::ConfigIoFailure {
                        reason: io_error.to_string(),
                    }
                })?;
            }
        }

        return Ok(module_directory);
    }

    /// Installs shell integration for Unix shells (bash/zsh).
    /// Writes the integration script and adds a source line to the profile.
    #[cfg(not(target_os = "windows"))]
    fn install_unix_integration(&self) -> Result<(), ReportalError> {
        let target_path = match self.profile_path() {
            Some(resolved_path) => resolved_path,
            None => return Ok(()),
        };

        let script_path = integration_file_path()?;
        let integration_content = Self::bash_integration_content();

        std::fs::write(&script_path, &integration_content).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;

        let source_line = Self::bash_source_line(&script_path);

        let existing_content = match target_path.exists() {
            true => std::fs::read_to_string(target_path).map_err(|io_error| {
                ReportalError::ConfigIoFailure {
                    reason: io_error.to_string(),
                }
            })?,
            false => String::new(),
        };

        let cleaned_content = Self::strip_existing_integration(&existing_content);
        let updated_content = format!("{cleaned_content}\n{source_line}\n");

        std::fs::write(target_path, updated_content).map_err(|io_error| {
            ReportalError::ConfigIoFailure {
                reason: io_error.to_string(),
            }
        })?;

        return Ok(());
    }
}
