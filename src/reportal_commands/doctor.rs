//! Validates the full `RePortal` installation and prints a diagnostic report.
//!
//! Checks three areas:
//! 1. Config file existence and TOML parse correctness
//! 2. Current session has loaded the shell integration (`REPORTAL_LOADED` env var)
//! 3. All registered repo paths exist on disk
//!
//! Each failed check prints an actionable hint so the user knows
//! exactly what to run to fix the problem.

use crate::reportal_config::{HasAliases, ReportalConfig, ShellAliasExport};
use crate::system_executable_lookup::SystemExecutableLookupOutcome;
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

        for (registered_alias, registered_repository_entry) in &all_repos {
            self.check_single_repository(&DiagnosticSummaryRepositoryEntryParameters {
                registered_alias,
                registered_repository_entry,
            });
        }
    }

    /// Validates that a single repository path exists and records the diagnostic.
    fn check_single_repository(
        &mut self,
        parameters: &DiagnosticSummaryRepositoryEntryParameters<'_>,
    ) {
        let resolved_path =
            parameters.registered_repository_entry.resolved_path();
        let path_display = resolved_path.display().to_string();
        let styled_alias = parameters
            .registered_alias
            .style(terminal_style::ALIAS_STYLE);
        let styled_path = path_display.style(terminal_style::PATH_STYLE);
        let diagnostic_label = format!("{styled_alias}  {styled_path}");
        if resolved_path.exists() {
            self.record_pass(&diagnostic_label);
        } else {
            self.record_fail(&format!("{diagnostic_label} (path does not exist)"));
        }
    }
}

/// Parameters for one repository-path diagnostic. Named with the
/// primary type's prefix so the type-per-file guard treats it as
/// a companion of `DiagnosticSummary`.
struct DiagnosticSummaryRepositoryEntryParameters<'entry> {
    /// The repository's canonical alias key as registered in
    /// configuration.
    registered_alias: &'entry str,
    /// The repository entry whose path is being checked.
    registered_repository_entry: &'entry crate::reportal_config::RepoEntry,
}

/// Parameters for one shell-alias shadow probe, kept as a named
/// struct so the probe helper has one self argument and one
/// params argument instead of two unlabelled positional `&str`
/// args. Named with the primary type's prefix so the
/// type-per-file guard treats it as a companion of
/// `DiagnosticSummary`.
struct DiagnosticSummaryShadowProbeParameters<'probe> {
    /// The name (canonical key or declared alias) that
    /// `rep init <shell>` would emit as a top-level shell
    /// function for this opted-in entry.
    candidate_emitted_name: &'probe str,
    /// Human-readable label classifying which kind of name this
    /// is (`repository canonical`, `repository alias`,
    /// `workspace canonical`, `workspace alias`, `command`).
    emitted_kind_label: &'probe str,
}

/// Shell-alias emission diagnostics for `DiagnosticSummary`,
/// kept in their own impl block so the addition does not
/// cascade into the surrounding methods.
impl DiagnosticSummary {
    /// Walks every opted-in repository, workspace, and command
    /// entry and flags any name (canonical key or declared alias)
    /// that resolves to an existing executable on the user's
    /// `PATH`. Such a name silently shadows the system command
    /// once `rep init <shell>` emits it as a top-level shell
    /// function, so doctor reports it here even though
    /// configuration-load tolerates legacy entries that pre-date
    /// the system-shadow validation.
    fn check_shell_alias_emission_health(
        &mut self,
        loaded_configuration: &ReportalConfig,
    ) {
        terminal_style::write_stdout("\n");
        terminal_style::write_stdout(&format!(
            "  {}\n",
            "Shell Alias Shadows".style(terminal_style::EMPHASIS_STYLE),
        ));

        let mut probed_any_opted_in_name = false;
        for (repository_canonical_key, repository_entry) in
            loaded_configuration.repos_with_aliases()
        {
            match repository_entry.shell_alias_export() {
                ShellAliasExport::Disabled => continue,
                ShellAliasExport::Enabled => {
                    probed_any_opted_in_name = true;
                    self.probe_one_emitted_name(
                        &DiagnosticSummaryShadowProbeParameters {
                            candidate_emitted_name: repository_canonical_key,
                            emitted_kind_label: "repository canonical",
                        },
                    );
                    for declared_alias in repository_entry.aliases() {
                        self.probe_one_emitted_name(
                            &DiagnosticSummaryShadowProbeParameters {
                                candidate_emitted_name: declared_alias,
                                emitted_kind_label: "repository alias",
                            },
                        );
                    }
                }
            }
        }
        for (workspace_canonical_name, workspace_entry) in
            loaded_configuration.workspaces_with_names()
        {
            match workspace_entry.shell_alias_export() {
                ShellAliasExport::Disabled => continue,
                ShellAliasExport::Enabled => {
                    probed_any_opted_in_name = true;
                    self.probe_one_emitted_name(
                        &DiagnosticSummaryShadowProbeParameters {
                            candidate_emitted_name: workspace_canonical_name,
                            emitted_kind_label: "workspace canonical",
                        },
                    );
                    for declared_alias in workspace_entry.aliases() {
                        self.probe_one_emitted_name(
                            &DiagnosticSummaryShadowProbeParameters {
                                candidate_emitted_name: declared_alias,
                                emitted_kind_label: "workspace alias",
                            },
                        );
                    }
                }
            }
        }
        for (command_key, command_entry) in loaded_configuration.global_commands() {
            match command_entry.shell_alias_export() {
                ShellAliasExport::Disabled => continue,
                ShellAliasExport::Enabled => {
                    probed_any_opted_in_name = true;
                    self.probe_one_emitted_name(
                        &DiagnosticSummaryShadowProbeParameters {
                            candidate_emitted_name: command_key,
                            emitted_kind_label: "command",
                        },
                    );
                }
            }
        }
        if !probed_any_opted_in_name {
            self.record_pass("No entries opted into shell-alias export");
        }
    }

    /// Probes a single emitted-name candidate against `PATH` and
    /// records pass / fail. Used by
    /// `check_shell_alias_emission_health` for every opted-in
    /// canonical key and declared alias.
    fn probe_one_emitted_name(
        &mut self,
        probe_parameters: &DiagnosticSummaryShadowProbeParameters<'_>,
    ) {
        let styled_name = probe_parameters
            .candidate_emitted_name
            .style(terminal_style::ALIAS_STYLE);
        match SystemExecutableLookupOutcome::for_candidate_name(
            probe_parameters.candidate_emitted_name,
        ) {
            SystemExecutableLookupOutcome::NotFound => {
                self.record_pass(&format!(
                    "{label}  {styled_name}",
                    label = probe_parameters.emitted_kind_label,
                ));
            }
            SystemExecutableLookupOutcome::ShadowsExisting {
                existing_executable,
            } => {
                let existing_executable_display = existing_executable.display().to_string();
                let styled_existing_executable_path =
                    existing_executable_display.style(terminal_style::PATH_STYLE);
                self.record_fail(&format!(
                    "{label}  {styled_name} shadows {styled_existing_executable_path}",
                    label = probe_parameters.emitted_kind_label,
                ));
                Self::print_hint(
                    "Pick a different alias, or remove `shell_alias = true` to keep this entry as a `rj <alias>` target only",
                );
            }
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

    if let Some(ref config) = loaded_config {
        summary.check_repo_paths(config);
        summary.check_shell_alias_emission_health(config);
    }

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
