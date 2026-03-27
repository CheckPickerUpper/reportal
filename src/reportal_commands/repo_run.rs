/// Fuzzy-selects a repo and runs a user-defined command in it.

use crate::error::ReportalError;
use crate::reportal_config::{CommandEntry, ReportalConfig, TagFilter};
use crate::terminal_style;
use crate::reportal_commands::repo_selection::{
    self, RepoSelectionParams, TerminalIdentityEmitParams,
};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::process::Command;

/// All parameters needed to run the run command.
pub struct RunCommandParams<'a> {
    /// Which repos to show in the fuzzy finder.
    pub tag_filter: TagFilter,
    /// If non-empty, skip the repo fuzzy finder and use this alias directly.
    pub direct_alias: &'a str,
    /// If non-empty, skip the command fuzzy finder and run this command directly.
    pub direct_command: &'a str,
}

/// A resolved command ready to execute: its name and the shell string.
struct ResolvedCommand {
    command_name: String,
    shell_command: String,
}

/// A merged command entry combining global definition with optional repo override.
struct MergedCommandEntry<'a> {
    name: &'a str,
    shell_command: &'a str,
    description: &'a str,
}

/// Parameters for resolving which command to run.
struct ResolveCommandParams<'a> {
    /// Global commands from config.
    global_commands: &'a BTreeMap<String, CommandEntry>,
    /// Per-repo commands (override globals with same name, or add repo-specific ones).
    repo_commands: &'a BTreeMap<String, CommandEntry>,
    /// If non-empty, skip the fuzzy finder and use this command directly.
    direct_command: &'a str,
}

/// Merges global commands with repo-level commands, then resolves
/// which command to run (direct or fuzzy-selected).
///
/// Repo-level commands override global commands with the same name.
fn resolve_command(resolve_params: ResolveCommandParams<'_>) -> Result<ResolvedCommand, ReportalError> {
    let mut merged: Vec<MergedCommandEntry<'_>> = Vec::new();

    for (name, entry) in resolve_params.global_commands {
        match resolve_params.repo_commands.get(name) {
            Some(repo_override) => {
                merged.push(MergedCommandEntry {
                    name,
                    shell_command: repo_override.shell_command(),
                    description: repo_override.description(),
                });
            }
            None => {
                merged.push(MergedCommandEntry {
                    name,
                    shell_command: entry.shell_command(),
                    description: entry.description(),
                });
            }
        }
    }

    for (name, entry) in resolve_params.repo_commands {
        let already_merged = resolve_params.global_commands.contains_key(name);
        match already_merged {
            true => {}
            false => {
                merged.push(MergedCommandEntry {
                    name,
                    shell_command: entry.shell_command(),
                    description: entry.description(),
                });
            }
        }
    }

    match merged.is_empty() {
        true => return Err(ReportalError::NoCommandsAvailable),
        false => {}
    }

    match resolve_params.direct_command.is_empty() {
        false => {
            let found = merged.iter().find(|entry| entry.name == resolve_params.direct_command);
            match found {
                Some(entry) => {
                    return Ok(ResolvedCommand {
                        command_name: entry.name.to_string(),
                        shell_command: entry.shell_command.to_string(),
                    });
                }
                None => {
                    return Err(ReportalError::CommandNotFound {
                        command_name: resolve_params.direct_command.to_string(),
                    });
                }
            }
        }
        true => {
            let display_labels: Vec<String> = merged
                .iter()
                .map(|entry| {
                    let mut label = entry.name.to_string();
                    match entry.description.is_empty() {
                        true => {
                            label.push_str(&format!(" — {}", entry.shell_command));
                        }
                        false => {
                            label.push_str(&format!(" — {}", entry.description));
                        }
                    }
                    return label;
                })
                .collect();

            let selected_index = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Run command")
                .items(&display_labels)
                .interact_opt()
                .map_err(|select_error| ReportalError::ConfigIoFailure {
                    reason: select_error.to_string(),
                })?;

            match selected_index {
                Some(chosen_index) => match merged.get(chosen_index) {
                    Some(entry) => {
                        return Ok(ResolvedCommand {
                            command_name: entry.name.to_string(),
                            shell_command: entry.shell_command.to_string(),
                        });
                    }
                    None => return Err(ReportalError::SelectionCancelled),
                },
                None => return Err(ReportalError::SelectionCancelled),
            }
        }
    }
}

/// Selects a repo, then selects a command, then runs it in the repo directory.
///
/// Global commands from `[commands.*]` are merged with per-repo commands
/// from `[repos.<alias>.commands]`. Repo-level commands override globals
/// with the same name. The command is spawned via the system shell with
/// inherited stdio for interactive passthrough.
pub fn run_run(run_params: RunCommandParams<'_>) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_from_disk()?;

    let selected = repo_selection::select_repo(RepoSelectionParams {
        loaded_config: &loaded_config,
        direct_alias: run_params.direct_alias,
        tag_filter: &run_params.tag_filter,
        prompt_label: "Run command in",
    })?;

    repo_selection::emit_repo_terminal_identity(TerminalIdentityEmitParams {
        selected_alias: selected.repo_alias(),
        selected_repo: selected.repo_config(),
        title_override: "",
    });

    let resolved_command = resolve_command(ResolveCommandParams {
        global_commands: loaded_config.global_commands(),
        repo_commands: selected.repo_config().repo_commands(),
        direct_command: run_params.direct_command,
    })?;

    let resolved_repo_path = selected.repo_config().resolved_path();

    terminal_style::print_success(&format!(
        "Running {} in {}",
        resolved_command.command_name.style(terminal_style::ALIAS_STYLE),
        loaded_config.path_display_format().format_path(&resolved_repo_path)
            .style(terminal_style::PATH_STYLE),
    ));

    #[cfg(target_os = "windows")]
    let mut spawned_process = Command::new("cmd")
        .args(["/c", &resolved_command.shell_command])
        .current_dir(&resolved_repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|spawn_error| ReportalError::CommandLaunchFailure {
            reason: format!("{}: {spawn_error}", resolved_command.shell_command),
        })?;

    #[cfg(not(target_os = "windows"))]
    let mut spawned_process = Command::new("sh")
        .args(["-c", &resolved_command.shell_command])
        .current_dir(&resolved_repo_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|spawn_error| ReportalError::CommandLaunchFailure {
            reason: format!("{}: {spawn_error}", resolved_command.shell_command),
        })?;

    let exit_status = spawned_process.wait().map_err(|wait_error| ReportalError::CommandLaunchFailure {
        reason: format!("process exited unexpectedly: {wait_error}"),
    })?;

    match exit_status.success() {
        true => {}
        false => {
            match exit_status.code() {
                Some(exit_code) => {
                    terminal_style::print_error(&format!(
                        "Command '{}' exited with code {}",
                        resolved_command.command_name, exit_code
                    ));
                }
                None => {
                    terminal_style::print_error(&format!(
                        "Command '{}' was terminated by signal",
                        resolved_command.command_name
                    ));
                }
            }
        }
    }

    return Ok(());
}
