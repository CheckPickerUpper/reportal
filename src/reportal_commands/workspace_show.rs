//! Shows a single workspace's details, resolved member paths, and
//! the on-disk location of its workspace directory.

use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{HasAliases, ReportalConfig, WorkspaceMember};
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Prints the named workspace's description, member aliases with
/// their resolved absolute paths, the materialized workspace
/// directory path, and the `.code-workspace` file path inside it.
///
/// For each member, shows whether the on-disk symlink / junction
/// exists so the user can spot a missing link and re-run
/// `rep workspace rebuild`. Does NOT auto-rebuild here; `show` is
/// read-only, and writing through a diagnostic command would make
/// it harder to notice that the layout is stale.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the name has no
/// matching entry or alias, [`ReportalError::RepoNotFound`] if any
/// member alias does not resolve, or the config / file I/O errors
/// that the load path surfaces.
pub fn run_workspace_show(alias_or_canonical: &str) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_name = loaded_config.resolve_workspace_canonical_name(alias_or_canonical)?;
    let target_workspace = loaded_config.get_workspace(&canonical_name)?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    let workspace_directory = regenerator.resolve_workspace_directory(&canonical_name)?;
    let workspace_file_path = regenerator.resolve_workspace_file_path(&canonical_name)?;

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
        "Dir:".style(terminal_style::LABEL_STYLE),
        workspace_directory.display().to_string().style(terminal_style::PATH_STYLE),
    ));
    if !workspace_directory.exists() {
        terminal_style::write_stdout(&format!(
            "          {} directory has not been materialized yet — run `rep workspace rebuild {canonical_name}`\n",
            "note:".style(terminal_style::LABEL_STYLE),
        ));
    }
    if target_workspace.is_legacy_file_path() {
        terminal_style::write_stdout(&format!(
            "          {} config still stores a pre-v0.15.2 `.code-workspace` path — run `rep workspace rebuild {canonical_name}` to migrate\n",
            "note:".style(terminal_style::LABEL_STYLE),
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
                let link_path = workspace_directory.join(member_alias);
                let link_status_text = link_existence_marker(&link_path);
                terminal_style::write_stdout(&format!(
                    "       {} {} {} {}\n",
                    "-".style(terminal_style::LABEL_STYLE),
                    member_alias.style(terminal_style::ALIAS_STYLE),
                    resolved_member_path
                        .display()
                        .to_string()
                        .style(terminal_style::PATH_STYLE),
                    link_status_text,
                ));
            }
            WorkspaceMember::InlinePath { path } => {
                let expanded = shellexpand::tilde(path);
                let inline_target = std::path::PathBuf::from(expanded.as_ref());
                let link_name = inline_target
                    .file_name()
                    .map_or_else(|| "inline".to_owned(), |os| os.to_string_lossy().into_owned());
                let link_path = workspace_directory.join(&link_name);
                let link_status_text = link_existence_marker(&link_path);
                terminal_style::write_stdout(&format!(
                    "       {} {} {} {}\n",
                    "-".style(terminal_style::LABEL_STYLE),
                    "(inline)".style(terminal_style::LABEL_STYLE),
                    expanded.as_ref().style(terminal_style::PATH_STYLE),
                    link_status_text,
                ));
            }
        }
    }

    terminal_style::write_stdout("\n");
    Ok(())
}

/// Returns a short styled marker describing whether the given link
/// path exists on disk (present) or is missing (link expected but
/// not found).
fn link_existence_marker(link_path: &std::path::Path) -> String {
    if std::fs::symlink_metadata(link_path).is_ok() {
        format!("[{}]", "linked".style(terminal_style::SUCCESS_STYLE))
    } else {
        format!("[{}]", "missing".style(terminal_style::FAILURE_STYLE))
    }
}
