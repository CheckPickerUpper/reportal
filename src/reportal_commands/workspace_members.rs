//! Add-repo and remove-repo subcommands for workspace membership.

use crate::cli_args::WorkspaceArgumentsMemberEditParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::{ReportalConfig, WorkspaceMember, WorkspaceMemberAliasLookup};
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
    member_edit: &WorkspaceArgumentsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_repository_name =
        loaded_config.resolve_repo_canonical_name(member_edit.repository_alias())?;
    let canonical_workspace_name =
        loaded_config.resolve_workspace_canonical_name(member_edit.workspace_name())?;

    let already_member = loaded_config
        .workspaces_containing_repo(&canonical_repository_name)
        .iter()
        .any(|(workspace_name, _workspace)| workspace_name.as_str() == canonical_workspace_name);
    if already_member {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "repo '{}' is already a member of workspace '{canonical_workspace_name}'",
                member_edit.repository_alias(),
            ),
        });
    }

    let target_workspace = loaded_config.get_workspace_mut(&canonical_workspace_name)?;
    let mut updated_members = target_workspace.members().to_vec();
    updated_members.push(WorkspaceMember::RegisteredRepo(canonical_repository_name));
    target_workspace.set_members(updated_members);
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
/// name or alias does not exist, or
/// [`ReportalError::ValidationFailure`] if the repo is not a
/// current member or if removal would leave the workspace with
/// zero members. Config I/O and file regeneration errors bubble up
/// from the underlying calls.
pub fn run_workspace_remove_repo(
    member_edit: &WorkspaceArgumentsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_repository_name =
        loaded_config.resolve_repo_canonical_name(member_edit.repository_alias())?;
    let canonical_workspace_name =
        loaded_config.resolve_workspace_canonical_name(member_edit.workspace_name())?;

    let current_members = loaded_config
        .get_workspace(&canonical_workspace_name)?
        .members()
        .to_vec();
    let workspace_contains_repo = loaded_config
        .workspaces_containing_repo(&canonical_repository_name)
        .iter()
        .any(|(workspace_name, _workspace)| workspace_name.as_str() == canonical_workspace_name);
    if !workspace_contains_repo {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "repo '{}' is not a member of workspace '{canonical_workspace_name}'",
                member_edit.repository_alias(),
            ),
        });
    }
    let remaining_members: Vec<WorkspaceMember> = current_members
        .into_iter()
        .filter(|member| {
            !workspace_member_targets_repo(&loaded_config, member, &canonical_repository_name)
        })
        .collect();
    if remaining_members.is_empty() {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "removing '{}' would leave workspace '{canonical_workspace_name}' with zero members; delete the workspace instead",
                member_edit.repository_alias(),
            ),
        });
    }
    let target_workspace = loaded_config.get_workspace_mut(&canonical_workspace_name)?;
    target_workspace.set_members(remaining_members);
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

/// Returns whether a workspace member refers to a given canonical
/// repository key, resolving both canonical names and declared
/// repository aliases. Invalid references are retained by removal
/// operations rather than being silently discarded.
fn workspace_member_targets_repo(
    loaded_config: &ReportalConfig,
    member: &WorkspaceMember,
    canonical_repository_name: &str,
) -> bool {
    match member.registered_repo_alias() {
        WorkspaceMemberAliasLookup::Matches(member_alias) => {
            match loaded_config.resolve_repo_canonical_name(member_alias) {
                Ok(member_canonical_name) => member_canonical_name == canonical_repository_name,
                Err(_not_found) => false,
            }
        }
        WorkspaceMemberAliasLookup::NotARepoReference => false,
    }
}
