//! Lists every registered VSCode/Cursor `.code-workspace` entry.
//!
//! Reads the config, then prints each workspace's name, description,
//! and member repo aliases in declared order so the user can see at
//! a glance which workspaces exist and what each one groups together.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{HasAliases, ReportalConfig};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Prints every registered workspace with its description and member repos.
///
/// Iterates `workspaces_with_names` so output order matches the
/// deterministic `BTreeMap` iteration, which means two invocations
/// against the same config produce byte-identical output — required
/// for the user to rely on list output as a source of truth.
///
/// # Errors
///
/// Returns [`ReportalError::ConfigParseFailure`] /
/// [`ReportalError::ConfigIoFailure`] if the config cannot be
/// loaded, or [`ReportalError::WorkspaceHasDanglingRepo`] if the
/// on-load validation pass finds a workspace pointing at a
/// non-registered repo.
pub fn run_workspace_list() -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let registered_workspaces = loaded_config.workspaces_with_names();

    if registered_workspaces.is_empty() {
        terminal_style::write_stdout(
            "No workspaces registered. Use 'rep workspace create' to add one.\n",
        );
        return Ok(());
    }

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {}\n",
        "Workspaces".style(terminal_style::EMPHASIS_STYLE)
    ));
    terminal_style::write_stdout("\n");

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    for (workspace_name, workspace_entry) in &registered_workspaces {
        let uppercase_name = workspace_name.to_uppercase();
        terminal_style::write_stdout(&format!(
            "  {}\n",
            uppercase_name.style(terminal_style::ALIAS_STYLE),
        ));

        if !workspace_entry.description().is_empty() {
            terminal_style::write_stdout(&format!(
                "     {} {}\n",
                "Desc:".style(terminal_style::LABEL_STYLE),
                workspace_entry.description(),
            ));
        }

        if !workspace_entry.aliases().is_empty() {
            terminal_style::write_stdout(&format!(
                "     {} {}\n",
                "Aliases:".style(terminal_style::LABEL_STYLE),
                workspace_entry.aliases().join(", ").style(terminal_style::ALIAS_STYLE),
            ));
        }

        if let Ok(workspace_directory) = regenerator.resolve_workspace_directory(workspace_name) {
            terminal_style::write_stdout(&format!(
                "     {} {}\n",
                "Dir:".style(terminal_style::LABEL_STYLE),
                workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
            ));
        }

        let formatted_member_list = workspace_entry.repo_aliases().join(", ");
        terminal_style::write_stdout(&format!(
            "     {} {}\n",
            "Repos:".style(terminal_style::LABEL_STYLE),
            formatted_member_list.style(terminal_style::PATH_STYLE),
        ));

        terminal_style::write_stdout("\n");
    }

    terminal_style::write_stdout(&format!(
        "  {} workspaces total\n\n",
        registered_workspaces.len().style(terminal_style::EMPHASIS_STYLE),
    ));
    Ok(())
}
