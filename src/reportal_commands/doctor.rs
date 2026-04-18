//! Validates the full `RePortal` installation and prints a diagnostic report.
//!
//! Checks three areas:
//! 1. Config file existence and TOML parse correctness
//! 2. Current session has loaded the shell integration (`REPORTAL_LOADED` env var)
//! 3. All registered repo paths exist on disk
//!
//! Each failed check prints an actionable hint so the user knows
//! exactly what to run to fix the problem.

use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

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

        match ReportalConfig::load_or_initialize() {
            Ok(loaded_config) => {
                let repo_count = loaded_config.repos_with_aliases().len();
                self.record_pass(&format!(
                    "Config loaded from {} ({repo_count} repos registered)",
                    config_path.display(),
                ));
                Some(loaded_config)
            }
            Err(ref parse_error) => {
                self.record_fail(&format!("Config failed to load: {parse_error}"));
                Self::print_hint("Check ~/.reportal/config.toml for syntax errors");
                None
            }
        }
    }

    /// Checks the session state — whether the shell integration was
    /// loaded in the current process tree via `eval "$(rep init ...)"`.
    fn check_shell_integration(&mut self) {
        terminal_style::write_stdout("\n");
        terminal_style::write_stdout(&format!(
            "  {}\n",
            "Shell Integration".style(terminal_style::EMPHASIS_STYLE),
        ));

        match std::env::var("REPORTAL_LOADED") {
            Ok(ref _marker_value) => {
                self.record_pass("Shell integration loaded in current session");
            }
            Err(ref _env_error) => {
                self.record_fail("Shell integration NOT loaded in current session");
                Self::print_hint(
                    "Add eval \"$(rep init zsh)\" (or bash/powershell) to your shell rc file",
                );
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
/// Checks config file parsing, shell integration session state, and repo
/// path existence, then prints a pass/fail summary to stdout.
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
