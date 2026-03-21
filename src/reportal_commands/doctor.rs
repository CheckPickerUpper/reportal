/// Validates the full RePortal installation: config, shell integration, and repo paths.
///
/// Prints a structured diagnostic report with pass/fail indicators for:
/// 1. Config file existence and TOML parse correctness
/// 2. Shell profile integration (rj/ro functions, prompt hook)
/// 3. Registered repo paths existing on disk
///
/// Each failed check includes an actionable hint (e.g. "Run 'rep upgrade'").
/// Exits with a summary count so the user knows if anything needs attention.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

use super::initialization::detect_shell_profile;

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

/// Parameters for checking whether a shell function is present in the profile.
struct ShellFunctionCheckParams<'a> {
    /// The full text of the shell profile file.
    profile_content: &'a str,
    /// The function name to look for (e.g. "rj", "ro").
    function_name: &'a str,
    /// The command the function should invoke (e.g. "rep jump").
    expected_command: &'a str,
    /// Running tally of diagnostic outcomes.
    summary: &'a mut DiagnosticSummary,
}

/// Checks that a specific shell function (rj or ro) is defined in the profile.
fn check_shell_function(function_check: ShellFunctionCheckParams<'_>) {
    let has_function = function_check.profile_content.lines().any(|line| {
        let trimmed = line.trim();
        let is_powershell_function = trimmed.starts_with(&format!("function {}", function_check.function_name))
            && trimmed.contains(function_check.expected_command);
        let is_bash_function = trimmed.starts_with(&format!("{}()", function_check.function_name))
            && trimmed.contains(function_check.expected_command);
        return is_powershell_function || is_bash_function;
    });

    match has_function {
        true => {
            print_pass(&format!("{} function installed", function_check.function_name));
            function_check.summary.record_check(CheckOutcome::Passed);
        }
        false => {
            print_fail(&format!("{} function missing from shell profile", function_check.function_name));
            print_hint("Run 'rep upgrade' to reinstall shell integration");
            function_check.summary.record_check(CheckOutcome::Failed);
        }
    }
}

/// Parameters for checking whether the prompt hook is present in the profile.
struct PromptHookCheckParams<'a> {
    /// The full text of the shell profile file.
    profile_content: &'a str,
    /// Running tally of diagnostic outcomes.
    summary: &'a mut DiagnosticSummary,
}

/// Checks that the prompt hook (for automatic tab color) is in the profile.
fn check_prompt_hook(hook_check: PromptHookCheckParams<'_>) {
    let has_prompt_hook = hook_check.profile_content.lines().any(|line| {
        let trimmed = line.trim();
        return trimmed.contains("rep color") && (
            trimmed.starts_with("function prompt")
            || trimmed.starts_with("$_reportal_")
            || trimmed.starts_with("PROMPT_COMMAND")
        );
    });

    match has_prompt_hook {
        true => {
            print_pass("Prompt hook installed (auto tab color)");
            hook_check.summary.record_check(CheckOutcome::Passed);
        }
        false => {
            print_fail("Prompt hook missing (tab color won't auto-apply)");
            print_hint("Run 'rep upgrade' to reinstall shell integration");
            hook_check.summary.record_check(CheckOutcome::Failed);
        }
    }
}

/// Checks that the shell profile exists and contains RePortal integration,
/// then verifies the current session has actually loaded it.
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

    let profile_content = match std::fs::read_to_string(profile_path) {
        Ok(content) => content,
        Err(_read_error) => {
            print_fail(&format!("Could not read profile at {}", profile_path.display()));
            summary.record_check(CheckOutcome::Failed);
            return;
        }
    };

    check_shell_function(ShellFunctionCheckParams {
        profile_content: &profile_content,
        function_name: "rj",
        expected_command: "rep jump",
        summary,
    });
    check_shell_function(ShellFunctionCheckParams {
        profile_content: &profile_content,
        function_name: "ro",
        expected_command: "rep open",
        summary,
    });
    check_prompt_hook(PromptHookCheckParams {
        profile_content: &profile_content,
        summary,
    });
    check_session_loaded(summary);
}

/// Checks whether the REPORTAL_LOADED env var is set, which proves
/// the shell profile was sourced in the current session. If the profile
/// file has the integration but this var is missing, rj/ro won't work
/// until the user reloads.
fn check_session_loaded(summary: &mut DiagnosticSummary) {
    match std::env::var("REPORTAL_LOADED") {
        Ok(_marker_value) => {
            print_pass("Shell integration loaded in current session");
            summary.record_check(CheckOutcome::Passed);
        }
        Err(_env_error) => {
            print_fail("Shell integration NOT loaded in current session");
            print_hint("Run '. $PROFILE' (PowerShell) or 'source ~/.bashrc' to reload");
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
/// Checks config file health, shell integration presence, and whether all
/// registered repo paths exist on disk. Each failed check prints an actionable
/// hint. Returns Ok(()) unconditionally — diagnostic failures are reported
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
