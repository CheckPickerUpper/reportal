//! Add-repo and remove-repo subcommands for workspace membership.

use crate::cli_args::WorkspaceArgsMemberEditParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{
    ReportalConfig, WorkspaceMember, WorkspaceMemberAliasLookup,
};
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
/// [`ReportalError::ValidationFailure`] if the member is already
/// present in the workspace. Config I/O and file regeneration
/// errors bubble up from the underlying calls.
pub fn run_workspace_add_repo(
    member_edit: &WorkspaceArgsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    loaded_config.get_repo(member_edit.repo_alias())?;
    let canonical_workspace_name =
        loaded_config.resolve_workspace_canonical_name(member_edit.workspace_name())?;

    let target_workspace = loaded_config.get_workspace_mut(&canonical_workspace_name)?;
    if target_workspace.contains_repo(member_edit.repo_alias()) {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "repo '{}' is already a member of workspace '{canonical_workspace_name}'",
                member_edit.repo_alias(),
            ),
        });
    }
    let mut updated_members = target_workspace.members().to_vec();
    updated_members.push(WorkspaceMember::RegisteredRepo(
        member_edit.repo_alias().to_owned(),
    ));
    target_workspace.set_members(updated_members);
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    regenerator.regenerate_workspace_file(&canonical_workspace_name)?;

    terminal_style::print_success(&format!(
        "Added {} to workspace {}",
        member_edit.repo_alias().style(terminal_style::ALIAS_STYLE),
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
/// name or alias does not exist, or
/// [`ReportalError::ValidationFailure`] if the repo is not a
/// current member or if removal would leave the workspace with
/// zero members. Config I/O and file regeneration errors bubble up
/// from the underlying calls.
pub fn run_workspace_remove_repo(
    member_edit: &WorkspaceArgsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    let canonical_workspace_name =
        loaded_config.resolve_workspace_canonical_name(member_edit.workspace_name())?;
    let target_workspace = loaded_config.get_workspace_mut(&canonical_workspace_name)?;
    if !target_workspace.contains_repo(member_edit.repo_alias()) {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "repo '{}' is not a member of workspace '{canonical_workspace_name}'",
                member_edit.repo_alias(),
            ),
        });
    }
    let remaining_members: Vec<WorkspaceMember> = target_workspace
        .members()
        .iter()
        .filter(|member| match member.registered_repo_alias() {
            WorkspaceMemberAliasLookup::Matches(existing_alias) => {
                existing_alias != member_edit.repo_alias()
            }
            WorkspaceMemberAliasLookup::NotARepoReference => true,
        })
        .cloned()
        .collect();
    if remaining_members.is_empty() {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "removing '{}' would leave workspace '{canonical_workspace_name}' with zero members; delete the workspace instead",
                member_edit.repo_alias(),
            ),
        });
    }
    target_workspace.set_members(remaining_members);
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    regenerator.regenerate_workspace_file(&canonical_workspace_name)?;

    terminal_style::print_success(&format!(
        "Removed {} from workspace {}",
        member_edit.repo_alias().style(terminal_style::ALIAS_STYLE),
        canonical_workspace_name.style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
