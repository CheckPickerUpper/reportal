//! Shows a single workspace's details, resolved member paths, and
//! the on-disk location of its `.code-workspace` file.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{HasAliases, ReportalConfig, WorkspaceMember};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Prints the named workspace's description, member aliases with
/// their resolved absolute paths, and the `.code-workspace` file
/// location that `rep workspace open` would launch.
///
/// Accepts either the canonical workspace key or any declared
/// alias: the first step resolves to the canonical name so every
/// downstream call (regenerator, header display) uses the same key
/// and derived values like the default file path stay consistent.
///
/// Regenerates the `.code-workspace` file before printing so the
/// shown file is guaranteed to match the current config state.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the name has no
/// matching entry or alias, [`ReportalError::RepoNotFound`] if any
/// member alias does not resolve, or the config / file I/O errors
/// that the load and regeneration paths surface.
pub fn run_workspace_show(alias_or_canonical: &str) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_name = loaded_config.resolve_workspace_canonical_name(alias_or_canonical)?;
    let target_workspace = loaded_config.get_workspace(&canonical_name)?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_file_path = regenerator.regenerate_workspace_file(&canonical_name)?;

    terminal_style::write_stdout("\n");
    terminal_style::write_stdout(&format!(
        "  {}\n",
        canonical_name.to_uppercase().style(terminal_style::ALIAS_STYLE),
    ));

    if !target_workspace.description().is_empty() {
        terminal_style::write_stdout(&format!(
            "     {} {}\n",
            "Desc:".style(terminal_style::LABEL_STYLE),
            target_workspace.description(),
        ));
    }

    if !target_workspace.aliases().is_empty() {
        terminal_style::write_stdout(&format!(
            "     {} {}\n",
            "Aliases:".style(terminal_style::LABEL_STYLE),
            target_workspace.aliases().join(", ").style(terminal_style::ALIAS_STYLE),
        ));
    }

    terminal_style::write_stdout(&format!(
        "     {} {}\n",
        "File:".style(terminal_style::LABEL_STYLE),
        workspace_file_path.display().to_string().style(terminal_style::PATH_STYLE),
    ));

    terminal_style::write_stdout(&format!(
        "     {}\n",
        "Members:".style(terminal_style::LABEL_STYLE),
    ));
    for member in target_workspace.members() {
        match member {
            WorkspaceMember::RegisteredRepo(member_alias) => {
                let member_repo = loaded_config.get_repo(member_alias)?;
                let resolved_member_path = member_repo.resolved_path();
                terminal_style::write_stdout(&format!(
                    "       {} {} {}\n",
                    "-".style(terminal_style::LABEL_STYLE),
                    member_alias.style(terminal_style::ALIAS_STYLE),
                    resolved_member_path
                        .display()
                        .to_string()
                        .style(terminal_style::PATH_STYLE),
                ));
            }
            WorkspaceMember::InlinePath { path } => {
                let expanded = shellexpand::tilde(path);
                terminal_style::write_stdout(&format!(
                    "       {} {} {}\n",
                    "-".style(terminal_style::LABEL_STYLE),
                    "(inline)".style(terminal_style::LABEL_STYLE),
                    expanded.as_ref().style(terminal_style::PATH_STYLE),
                ));
            }
        }
    }

    terminal_style::write_stdout("\n");
    Ok(())
}
