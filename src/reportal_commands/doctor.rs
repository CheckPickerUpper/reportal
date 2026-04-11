//! Validates the full `RePortal` installation and prints a diagnostic report.
//!
//! Checks five areas:
//! 1. Config file existence and TOML parse correctness
//! 2. Integration script file exists and version matches the binary
//! 3. Shell integration is reachable (module on `PowerShell`, source line on Unix)
//! 4. Current session has loaded the integration (`REPORTAL_LOADED` env var)
//! 5. All registered repo paths exist on disk
//!
//! Each failed check prints an actionable hint so the user knows
//! exactly what to run to fix the problem.

use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

use super::detected_shell::DetectedShell;
use super::integration_freshness::{check_integration_file_state, IntegrationFileState};

#[cfg(target_os = "windows")]
use super::detected_shell::powershell_module_directory;

/// Determines the shell name from its profile path string.
fn detect_shell_label(path_string: &str) -> &'static str {
    if path_string.contains("PowerShell") {
        "PowerShell"
    } else if path_string.contains(".zshrc") {
        "Zsh"
    } else {
        "Bash"
    }
}

/// Tracks pass/fail counts across all diagnostic checks.
struct DiagnosticSummary {
    passed: u32,
    failed: u32,
}

/// Construction, recording, and output for diagnostic pass/fail tallies.
impl DiagnosticSummary {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn record_pass(&mut self, diagnostic_message: &str) {
        terminal_style::write_stdout(&format!(
            "    {} {}\n",
            "✓".style(terminal_style::SUCCESS_STYLE),
            diagnostic_message,
        ));
        self.passed += 1;
    }

    fn record_fail(&mut self, diagnostic_message: &str) {
        terminal_style::write_stdout(&format!(
            "    {} {}\n",
            "✗".style(terminal_style::FAILURE_STYLE),
            diagnostic_message,
        ));
        self.failed += 1;
    }

    fn print_hint(hint_message: &str) {
        terminal_style::write_stdout(&format!(
            "      {} {}\n",
            "→".style(terminal_style::LABEL_STYLE),
            hint_message.style(terminal_style::PATH_STYLE),
        ));
    }

    /// Checks that the config file exists and parses correctly.
    /// Returns the loaded config on success for use by later checks.
    fn check_config(&mut self) -> Option<ReportalConfig> {
        terminal_style::write_stdout("\n");
        terminal_style::write_stdout(&format!(
            "  {}\n",
            "Config".style(terminal_style::EMPHASIS_STYLE),
        ));

        let config_path = match ReportalConfig::config_file_path() {
            Ok(path) => path,
            Err(ref _path_error) => {
                self.record_fail("Could not determine config path");
                return None;
            }
        };

        if config_path.exists() {
            self.record_pass(&format!("Config file exists at {}", config_path.display()));
        } else {
            self.record_fail(&format!("Config not found at {}", config_path.display()));
            Self::print_hint("Run 'rep init' to create one");
            return None;
        }

        match ReportalConfig::load_from_disk() {
            Ok(loaded_config) => {
                let repo_count = loaded_config.repos_with_aliases().len();
                self.record_pass(&format!("Config parses successfully ({repo_count} repos registered)"));
                Some(loaded_config)
            }
            Err(ref parse_error) => {
                self.record_fail(&format!("Config failed to parse: {parse_error}"));
                Self::print_hint("Check ~/.reportal/config.toml for syntax errors");
                None
            }
        }
    }

    /// Checks the integration file, shell profile source line, and session state.
    fn check_shell_integration(&mut self) {
        terminal_style::write_stdout("\n");
        terminal_style::write_stdout(&format!(
            "  {}\n",
            "Shell Integration".style(terminal_style::EMPHASIS_STYLE),
        ));

        let detected_shell = DetectedShell::detect();

        let Some(profile_path) = detected_shell.profile_path() else {
            self.record_fail("Could not detect shell profile");
            Self::print_hint("Run 'rep init' to install shell integration");
            return;
        };
        let detected_label = detect_shell_label(&format!("{}", profile_path.display()));
        self.record_pass(&format!("Shell detected: {detected_label}"));

        match check_integration_file_state() {
            IntegrationFileState::Current => {
                self.record_pass(&format!(
                    "Integration file up to date (v{})",
                    env!("CARGO_PKG_VERSION"),
                ));
            }
            IntegrationFileState::Outdated { ref file_version } => {
                self.record_fail(&format!(
                    "Integration file outdated (file: {file_version}, binary: v{})",
                    env!("CARGO_PKG_VERSION"),
                ));
                Self::print_hint("Run 'rep init' to update");
            }
            IntegrationFileState::Missing => {
                self.record_fail("Integration file missing");
                Self::print_hint("Run 'rep init' to create it");
            }
        }

        self.check_integration_reachable(&detected_shell);
        self.check_session_loaded();
    }

    /// Checks that the shell can actually load the integration.
    /// On `PowerShell`, verifies the module directory exists.
    /// On Unix, verifies the profile sources the integration file.
    fn check_integration_reachable(&mut self, detected_shell: &DetectedShell) {
        #[cfg(target_os = "windows")]
        if let DetectedShell::PowerShell(_) = detected_shell {
            self.check_powershell_module();
        }

        #[cfg(not(target_os = "windows"))]
        {
            let profile_path = match detected_shell.profile_path() {
                Some(path) => path,
                None => return,
            };

            let profile_content = match std::fs::read_to_string(profile_path) {
                Ok(content) => content,
                Err(ref _read_error) => {
                    self.record_fail(&format!(
                        "Could not read profile at {}",
                        profile_path.display(),
                    ));
                    return;
                }
            };

            let integration_marker =
                format!(".reportal{}integration", std::path::MAIN_SEPARATOR);
            let profile_sources_integration = profile_content.lines().any(|line| {
                line.contains(&integration_marker)
            });

            if profile_sources_integration {
                self.record_pass("Profile sources integration file");
            } else {
                self.record_fail("Profile does not source integration file");
                Self::print_hint("Run 'rep init' to set it up");
            }
        }
    }

    /// Verifies the `PowerShell` module directory contains `RePortal.psm1`.
    #[cfg(target_os = "windows")]
    fn check_powershell_module(&mut self) {
        let module_path = match powershell_module_directory() {
            Ok(path) => path,
            Err(ref _module_path_error) => {
                self.record_fail("Could not determine PowerShell module directory");
                Self::print_hint("Run 'rep init' to install shell integration");
                return;
            }
        };
        let module_file = module_path.join("RePortal.psm1");
        if module_file.exists() {
            self.record_pass(&format!(
                "PowerShell module installed at {}",
                module_path.display(),
            ));
        } else {
            self.record_fail(&format!(
                "PowerShell module not found at {}",
                module_path.display(),
            ));
            Self::print_hint("Run 'rep init' to install the module");
        }
    }

    /// Checks whether the `REPORTAL_LOADED` env var is set, which proves
    /// the shell integration was loaded in the current session.
    fn check_session_loaded(&mut self) {
        match std::env::var("REPORTAL_LOADED") {
            Ok(ref _marker_value) => {
                self.record_pass("Shell integration loaded in current session");
            }
            Err(ref _env_error) => {
                self.record_fail("Shell integration NOT loaded in current session");
                Self::print_hint("Open a new terminal tab to pick up changes");
            }
        }
    }

    /// Checks that all registered repo paths exist on disk.
    fn check_repo_paths(&mut self, loaded_config: &ReportalConfig) {
        terminal_style::write_stdout("\n");
        terminal_style::write_stdout(&format!(
            "  {}\n",
            "Repo Paths".style(terminal_style::EMPHASIS_STYLE),
        ));

        let all_repos = loaded_config.repos_with_aliases();

        if all_repos.is_empty() {
            self.record_fail("No repos registered");
            Self::print_hint("Run 'rep add <path>' to register a repo");
            return;
        }

        for (alias, entry) in &all_repos {
            self.check_single_repo(alias, entry);
        }
    }

    /// Validates that a single repo path exists and records the diagnostic.
    fn check_single_repo(&mut self, alias: &str, entry: &crate::reportal_config::RepoEntry) {
        let resolved_path = entry.resolved_path();
        let path_display = resolved_path.display().to_string();
        let styled_alias = alias.style(terminal_style::ALIAS_STYLE);
        let styled_path = path_display.style(terminal_style::PATH_STYLE);
        let diagnostic_label = format!("{styled_alias}  {styled_path}");
        if resolved_path.exists() {
            self.record_pass(&diagnostic_label);
        } else {
            self.record_fail(&format!("{diagnostic_label} (path does not exist)"));
        }
    }
}

/// Validates the full `RePortal` installation and prints a diagnostic report.
/// Checks config file parsing, shell integration, and repo path existence,
/// then prints a pass/fail summary to stdout.
pub fn run_doctor() {
    terminal_style::write_stdout(&format!(
        "{}\n",
        "RePortal Doctor".style(terminal_style::EMPHASIS_STYLE),
    ));

    let mut summary = DiagnosticSummary::new();

    let loaded_config = summary.check_config();
    summary.check_shell_integration();

    if let Some(ref config) = loaded_config { summary.check_repo_paths(config) }

    terminal_style::write_stdout("\n");
    match summary.failed {
        0 => {
            terminal_style::print_success(&format!(
                "All {} checks passed",
                summary.passed,
            ));
        }
        _failure_count => {
            terminal_style::write_stdout(&format!(
                "  {} {} passed, {} failed\n",
                "Summary:".style(terminal_style::EMPHASIS_STYLE),
                summary.passed.style(terminal_style::SUCCESS_STYLE),
                summary.failed.style(terminal_style::FAILURE_STYLE),
            ));
        }
    }

}
