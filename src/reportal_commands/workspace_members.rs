//! Add-repo and remove-repo subcommands for workspace membership.

use crate::cli_args::WorkspaceArgumentsMemberEditParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Appends a repo alias to the workspace's member list and
/// regenerates the `.code-workspace` file from the post-mutation
/// state.
///
/// Accepts either the canonical workspace key or any declared
/// workspace alias. Resolves to canonical first so downstream
/// regeneration uses the correct file-location key.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
/// name or alias does not exist, [`ReportalError::RepoNotFound`]
/// if the repo alias is not registered, or
/// [`ReportalError::WorkspaceAlreadyContainsRepo`] if the member is already
/// present in the workspace. Config I/O and file regeneration
/// errors bubble up from the underlying calls.
pub fn run_workspace_add_repo(
    member_edit: &WorkspaceArgumentsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_workspace_name =
        loaded_config.resolve_workspace_canonical_name(member_edit.workspace_name())?;

    loaded_config.add_workspace_repo(&canonical_workspace_name, member_edit.repository_alias())?;
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    regenerator.regenerate_workspace_file(&canonical_workspace_name)?;

    terminal_style::print_success(&format!(
        "Added {} to workspace {}",
        member_edit
            .repository_alias()
            .style(terminal_style::ALIAS_STYLE),
        canonical_workspace_name.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}

/// Drops a repo alias from the workspace's member list and
/// regenerates the `.code-workspace` file from the post-mutation
/// state.
///
/// Accepts either the canonical workspace key or any declared
/// workspace alias. Refuses to remove a repo that is not currently
/// a member so the caller gets a clear error instead of a silent
/// no-op, and refuses to leave the workspace empty because an
/// empty workspace has no meaning.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
/// name or alias does not exist, or a membership error when the
/// repo is absent or removal would leave the workspace empty.
pub fn run_workspace_remove_repo(
    member_edit: &WorkspaceArgumentsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_workspace_name =
        loaded_config.resolve_workspace_canonical_name(member_edit.workspace_name())?;
    loaded_config
        .remove_workspace_repo(&canonical_workspace_name, member_edit.repository_alias())?;
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    regenerator.regenerate_workspace_file(&canonical_workspace_name)?;

    terminal_style::print_success(&format!(
        "Removed {} from workspace {}",
        member_edit
            .repository_alias()
            .style(terminal_style::ALIAS_STYLE),
        canonical_workspace_name.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
