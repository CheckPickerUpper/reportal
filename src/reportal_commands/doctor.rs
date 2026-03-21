/// Validates the full RePortal installation and prints a diagnostic report.
///
/// Checks four areas:
/// 1. Config file existence and TOML parse correctness
/// 2. Integration script file exists and version matches the binary
/// 3. Shell profile sources the integration file
/// 4. Current session has loaded the integration (REPORTAL_LOADED env var)
/// 5. All registered repo paths exist on disk
///
/// Each failed check prints an actionable hint so the user knows
/// exactly what to run to fix the problem.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

use super::initialization::{detect_shell_profile, integration_file_path};

/// Whether a single diagnostic check passed or failed.
enum CheckOutcome {
    /// The check passed without issues.
    Passed,
    /// The check detected a problem.
    Failed,
}

/// Tracks pass/fail counts across all diagnostic checks.
struct DiagnosticSummary {
    passed: u32,
    failed: u32,
}

/// Construction and recording for diagnostic pass/fail tallies.
impl DiagnosticSummary {
    fn new() -> Self {
        return Self {
            passed: 0,
            failed: 0,
        };
    }

    fn record_check(&mut self, check_outcome: CheckOutcome) {
        match check_outcome {
            CheckOutcome::Passed => self.passed += 1,
            CheckOutcome::Failed => self.failed += 1,
        }
    }
}

fn print_pass(diagnostic_message: &str) {
    println!(
        "    {} {}",
        "✓".style(terminal_style::SUCCESS_STYLE),
        diagnostic_message,
    );
}

fn print_fail(diagnostic_message: &str) {
    println!(
        "    {} {}",
        "✗".style(terminal_style::FAILURE_STYLE),
        diagnostic_message,
    );
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
fn check_config(summary: &mut DiagnosticSummary) -> Option<ReportalConfig> {
    println!();
    println!(
        "  {}",
        "Config".style(terminal_style::EMPHASIS_STYLE),
    );

    let config_path = match ReportalConfig::config_file_path() {
        Ok(path) => path,
        Err(_path_error) => {
            print_fail("Could not determine config path");
            summary.record_check(CheckOutcome::Failed);
            return None;
        }
    };

    match config_path.exists() {
        true => {
            print_pass(&format!("Config file exists at {}", config_path.display()));
            summary.record_check(CheckOutcome::Passed);
        }
        false => {
            print_fail(&format!("Config not found at {}", config_path.display()));
            print_hint("Run 'rep init' to create one");
            summary.record_check(CheckOutcome::Failed);
            return None;
        }
    }

    match ReportalConfig::load_from_disk() {
        Ok(loaded_config) => {
            let repo_count = loaded_config.repos_with_aliases().len();
            print_pass(&format!("Config parses successfully ({repo_count} repos registered)"));
            summary.record_check(CheckOutcome::Passed);
            return Some(loaded_config);
        }
        Err(parse_error) => {
            print_fail(&format!("Config failed to parse: {parse_error}"));
            print_hint("Check ~/.reportal/config.toml for syntax errors");
            summary.record_check(CheckOutcome::Failed);
            return None;
        }
    }
}

/// Whether the integration file version matches, is outdated, or is missing.
enum IntegrationFileStatus {
    /// File exists and version matches the running binary.
    Current,
    /// File exists but was written by a different version.
    Outdated { file_version: String },
    /// File does not exist at the expected path.
    Missing,
}

/// Reads the integration script file and compares its version stamp
/// against the running binary version.
fn check_integration_file_version() -> IntegrationFileStatus {
    let script_path = match integration_file_path() {
        Ok(path) => path,
        Err(_path_error) => return IntegrationFileStatus::Missing,
    };

    let file_content = match std::fs::read_to_string(&script_path) {
        Ok(content) => content,
        Err(_read_error) => return IntegrationFileStatus::Missing,
    };

    let binary_version = env!("CARGO_PKG_VERSION");

    let first_line = match file_content.lines().next() {
        Some(line) => line,
        None => return IntegrationFileStatus::Outdated {
            file_version: String::from("empty"),
        },
    };

    match first_line.contains(binary_version) {
        true => IntegrationFileStatus::Current,
        false => {
            let extracted_version = first_line
                .rsplit("— v")
                .next()
                .map(String::from)
                .unwrap_or_else(|| String::from("unknown"));
            IntegrationFileStatus::Outdated {
                file_version: extracted_version,
            }
        }
    }
}

/// Checks the integration file, shell profile source line, and session state.
fn check_shell_integration(summary: &mut DiagnosticSummary) {
    println!();
    println!(
        "  {}",
        "Shell Integration".style(terminal_style::EMPHASIS_STYLE),
    );

    let detected_shell = detect_shell_profile();

    let profile_path = match detected_shell.profile_path() {
        Some(path) => {
            let path_string = format!("{}", path.display());
            let detected_label = match path_string.contains("PowerShell") {
                true => "PowerShell",
                false => match path_string.contains(".zshrc") {
                    true => "Zsh",
                    false => "Bash",
                },
            };
            print_pass(&format!("Shell detected: {detected_label}"));
            summary.record_check(CheckOutcome::Passed);
            path
        }
        None => {
            print_fail("Could not detect shell profile");
            print_hint("Run 'rep init' to install shell integration");
            summary.record_check(CheckOutcome::Failed);
            return;
        }
    };

    match check_integration_file_version() {
        IntegrationFileStatus::Current => {
            print_pass(&format!("Integration file up to date (v{})", env!("CARGO_PKG_VERSION")));
            summary.record_check(CheckOutcome::Passed);
        }
        IntegrationFileStatus::Outdated { ref file_version } => {
            print_fail(&format!(
                "Integration file outdated (file: {file_version}, binary: v{})",
                env!("CARGO_PKG_VERSION"),
            ));
            print_hint("Run 'rep init' to update");
            summary.record_check(CheckOutcome::Failed);
        }
        IntegrationFileStatus::Missing => {
            print_fail("Integration file missing");
            print_hint("Run 'rep init' to create it");
            summary.record_check(CheckOutcome::Failed);
        }
    }

    let profile_content = match std::fs::read_to_string(profile_path) {
        Ok(content) => content,
        Err(_read_error) => {
            print_fail(&format!("Could not read profile at {}", profile_path.display()));
            summary.record_check(CheckOutcome::Failed);
            return;
        }
    };

    let integration_marker = format!(".reportal{}integration", std::path::MAIN_SEPARATOR);
    let profile_sources_integration = profile_content.lines().any(|line| {
        return line.contains(&integration_marker);
    });

    match profile_sources_integration {
        true => {
            print_pass("Profile sources integration file");
            summary.record_check(CheckOutcome::Passed);
        }
        false => {
            print_fail("Profile does not source integration file");
            print_hint("Run 'rep init' to set it up");
            summary.record_check(CheckOutcome::Failed);
        }
    }

    check_session_loaded(summary);
}

/// Checks whether the REPORTAL_LOADED env var is set, which proves
/// the shell profile was sourced in the current session. If the
/// integration file exists but this var is missing, rj and ro
/// will not work until the user opens a new terminal.
fn check_session_loaded(summary: &mut DiagnosticSummary) {
    match std::env::var("REPORTAL_LOADED") {
        Ok(_marker_value) => {
            print_pass("Shell integration loaded in current session");
            summary.record_check(CheckOutcome::Passed);
        }
        Err(_env_error) => {
            print_fail("Shell integration NOT loaded in current session");
            print_hint("Open a new terminal tab to pick up changes");
            summary.record_check(CheckOutcome::Failed);
        }
    }
}

/// Parameters for validating that registered repo paths exist on disk.
struct RepoPathCheckParams<'a> {
    /// The loaded config containing repo entries to validate.
    loaded_config: &'a ReportalConfig,
    /// Running tally of diagnostic outcomes.
    summary: &'a mut DiagnosticSummary,
}

/// Checks that all registered repo paths exist on disk.
fn check_repo_paths(path_check: RepoPathCheckParams<'_>) {
    println!();
    println!(
        "  {}",
        "Repo Paths".style(terminal_style::EMPHASIS_STYLE),
    );

    let all_repos = path_check.loaded_config.repos_with_aliases();

    match all_repos.is_empty() {
        true => {
            print_fail("No repos registered");
            print_hint("Run 'rep add <path>' to register a repo");
            path_check.summary.record_check(CheckOutcome::Failed);
        }
        false => {
            for (alias, entry) in &all_repos {
                let resolved_path = entry.resolved_path();
                match resolved_path.exists() {
                    true => {
                        print_pass(&format!(
                            "{}  {}",
                            alias.style(terminal_style::ALIAS_STYLE),
                            resolved_path.display().to_string().style(terminal_style::PATH_STYLE),
                        ));
                        path_check.summary.record_check(CheckOutcome::Passed);
                    }
                    false => {
                        print_fail(&format!(
                            "{}  {} (path does not exist)",
                            alias.style(terminal_style::ALIAS_STYLE),
                            resolved_path.display().to_string().style(terminal_style::PATH_STYLE),
                        ));
                        path_check.summary.record_check(CheckOutcome::Failed);
                    }
                }
            }
        }
    }
}

/// Validates the full RePortal installation and prints a diagnostic report.
///
/// Checks config file health, integration file version, shell profile
/// source line, session load state, and whether all registered repo
/// paths exist on disk. Each failed check prints an actionable hint.
/// Returns Ok(()) unconditionally — diagnostic failures are reported
/// to the user via stdout, not propagated as errors.
pub fn run_doctor() -> Result<(), ReportalError> {
    println!(
        "{}",
        "RePortal Doctor".style(terminal_style::EMPHASIS_STYLE),
    );

    let mut summary = DiagnosticSummary::new();

    let loaded_config = check_config(&mut summary);
    check_shell_integration(&mut summary);

    match loaded_config {
        Some(ref config) => check_repo_paths(RepoPathCheckParams {
            loaded_config: config,
            summary: &mut summary,
        }),
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
