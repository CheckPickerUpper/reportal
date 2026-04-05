/// Validates the full RePortal installation and prints a diagnostic report.
///
/// Checks five areas:
/// 1. Config file existence and TOML parse correctness
/// 2. Integration script file exists and version matches the binary
/// 3. Shell integration is reachable (module on PowerShell, source line on Unix)
/// 4. Current session has loaded the integration (REPORTAL_LOADED env var)
/// 5. All registered repo paths exist on disk
///
/// Each failed check prints an actionable hint so the user knows
/// exactly what to run to fix the problem.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

use super::detected_shell::DetectedShell;
use super::integration_freshness::{check_integration_file_state, IntegrationFileState};

#[cfg(target_os = "windows")]
use super::detected_shell::powershell_module_directory;

/// Tracks pass/fail counts across all diagnostic checks.
struct DiagnosticSummary {
    passed: u32,
    failed: u32,
}

/// Construction, recording, and output for diagnostic pass/fail tallies.
impl DiagnosticSummary {
    fn new() -> Self {
        return Self {
            passed: 0,
            failed: 0,
        };
    }

    fn record_pass(&mut self, diagnostic_message: &str) {
        println!(
            "    {} {}",
            "✓".style(terminal_style::SUCCESS_STYLE),
            diagnostic_message,
        );
        self.passed += 1;
    }

    fn record_fail(&mut self, diagnostic_message: &str) {
        println!(
            "    {} {}",
            "✗".style(terminal_style::FAILURE_STYLE),
            diagnostic_message,
        );
        self.failed += 1;
    }

    fn print_hint(hint_message: &str) {
        println!(
            "      {} {}",
            "→".style(terminal_style::LABEL_STYLE),
            hint_message.style(terminal_style::PATH_STYLE),
        );
    }

    /// Checks that the config file exists and parses correctly.
    /// Returns the loaded config on success for use by later checks.
    fn check_config(&mut self) -> Option<ReportalConfig> {
        println!();
        println!(
            "  {}",
            "Config".style(terminal_style::EMPHASIS_STYLE),
        );

        let config_path = match ReportalConfig::config_file_path() {
            Ok(path) => path,
            Err(ref _path_error) => {
                self.record_fail("Could not determine config path");
                return None;
            }
        };

        match config_path.exists() {
            true => {
                self.record_pass(&format!("Config file exists at {}", config_path.display()));
            }
            false => {
                self.record_fail(&format!("Config not found at {}", config_path.display()));
                Self::print_hint("Run 'rep init' to create one");
                return None;
            }
        }

        match ReportalConfig::load_from_disk() {
            Ok(loaded_config) => {
                let repo_count = loaded_config.repos_with_aliases().len();
                self.record_pass(&format!("Config parses successfully ({repo_count} repos registered)"));
                return Some(loaded_config);
            }
            Err(ref parse_error) => {
                self.record_fail(&format!("Config failed to parse: {parse_error}"));
                Self::print_hint("Check ~/.reportal/config.toml for syntax errors");
                return None;
            }
        }
    }

    /// Checks the integration file, shell profile source line, and session state.
    fn check_shell_integration(&mut self) {
        println!();
        println!(
            "  {}",
            "Shell Integration".style(terminal_style::EMPHASIS_STYLE),
        );

        let detected_shell = DetectedShell::detect();

        let _profile_path = match detected_shell.profile_path() {
            Some(path) => {
                let path_string = format!("{}", path.display());
                let detected_label = match path_string.contains("PowerShell") {
                    true => "PowerShell",
                    false => match path_string.contains(".zshrc") {
                        true => "Zsh",
                        false => "Bash",
                    },
                };
                self.record_pass(&format!("Shell detected: {detected_label}"));
                path
            }
            None => {
                self.record_fail("Could not detect shell profile");
                Self::print_hint("Run 'rep init' to install shell integration");
                return;
            }
        };

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
    /// On PowerShell, verifies the module directory exists.
    /// On Unix, verifies the profile sources the integration file.
    fn check_integration_reachable(&mut self, detected_shell: &DetectedShell) {
        #[cfg(target_os = "windows")]
        {
            match detected_shell {
                DetectedShell::PowerShell(_) => {
                    match powershell_module_directory() {
                        Ok(module_path) => {
                            let module_file = module_path.join("RePortal.psm1");
                            match module_file.exists() {
                                true => {
                                    self.record_pass(&format!(
                                        "PowerShell module installed at {}",
                                        module_path.display(),
                                    ));
                                }
                                false => {
                                    self.record_fail(&format!(
                                        "PowerShell module not found at {}",
                                        module_path.display(),
                                    ));
                                    Self::print_hint("Run 'rep init' to install the module");
                                }
                            }
                        }
                        Err(ref _module_path_error) => {
                            self.record_fail("Could not determine PowerShell module directory");
                            Self::print_hint("Run 'rep init' to install shell integration");
                        }
                    }
                    return;
                }
                _ => {}
            }
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
                return line.contains(&integration_marker);
            });

            match profile_sources_integration {
                true => {
                    self.record_pass("Profile sources integration file");
                }
                false => {
                    self.record_fail("Profile does not source integration file");
                    Self::print_hint("Run 'rep init' to set it up");
                }
            }
        }
    }

    /// Checks whether the REPORTAL_LOADED env var is set, which proves
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
        println!();
        println!(
            "  {}",
            "Repo Paths".style(terminal_style::EMPHASIS_STYLE),
        );

        let all_repos = loaded_config.repos_with_aliases();

        match all_repos.is_empty() {
            true => {
                self.record_fail("No repos registered");
                Self::print_hint("Run 'rep add <path>' to register a repo");
            }
            false => {
                for (alias, entry) in &all_repos {
                    let resolved_path = entry.resolved_path();
                    match resolved_path.exists() {
                        true => {
                            self.record_pass(&format!(
                                "{}  {}",
                                alias.style(terminal_style::ALIAS_STYLE),
                                resolved_path
                                    .display()
                                    .to_string()
                                    .style(terminal_style::PATH_STYLE),
                            ));
                        }
                        false => {
                            self.record_fail(&format!(
                                "{}  {} (path does not exist)",
                                alias.style(terminal_style::ALIAS_STYLE),
                                resolved_path
                                    .display()
                                    .to_string()
                                    .style(terminal_style::PATH_STYLE),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Validates the full RePortal installation and prints a diagnostic report.
pub fn run_doctor() -> Result<(), ReportalError> {
    println!(
        "{}",
        "RePortal Doctor".style(terminal_style::EMPHASIS_STYLE),
    );

    let mut summary = DiagnosticSummary::new();

    let loaded_config = summary.check_config();
    summary.check_shell_integration();

    match loaded_config {
        Some(ref config) => summary.check_repo_paths(config),
        None => {}
    }

    println!();
    match summary.failed {
        0 => {
            terminal_style::print_success(&format!(
                "All {} checks passed",
                summary.passed,
            ));
        }
        _failure_count => {
            println!(
                "  {} {} passed, {} failed",
                "Summary:".style(terminal_style::EMPHASIS_STYLE),
                summary.passed.style(terminal_style::SUCCESS_STYLE),
                summary.failed.style(terminal_style::FAILURE_STYLE),
            );
        }
    }

    return Ok(());
}
