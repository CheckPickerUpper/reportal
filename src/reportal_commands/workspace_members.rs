//! Add-repo and remove-repo subcommands for workspace membership.

use crate::cli_args::WorkspaceArgsMemberEditParts;
use crate::error::ReportalError;
use crate::reportal_commands::workspace_operations::WorkspaceRegenerator;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;
use owo_colors::OwoColorize;

/// Appends a repo alias to the workspace's member list and
/// regenerates the `.code-workspace` file from the post-mutation
/// state.
///
/// Refuses to add a repo alias that is not registered so the
/// on-load validation pass cannot later reject a config that this
/// command produced. Refuses to add a duplicate so the membership
/// list remains a set in practice even though it is serialized as
/// an ordered `Vec` for sidebar-order control.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
/// name does not exist, [`ReportalError::RepoNotFound`] if the
/// repo alias is not registered, or
/// [`ReportalError::ValidationFailure`] if the member is already
/// present in the workspace. Config I/O and file regeneration
/// errors bubble up from the underlying calls.
pub fn run_workspace_add_repo(
    member_edit: &WorkspaceArgsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    loaded_config.get_repo(member_edit.repo_alias())?;

    let target_workspace = loaded_config.get_workspace_mut(member_edit.workspace_name())?;
    if target_workspace.contains_repo(member_edit.repo_alias()) {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "repo '{}' is already a member of workspace '{}'",
                member_edit.repo_alias(),
                member_edit.workspace_name(),
            ),
        });
    }
    let mut updated_member_aliases = target_workspace.repo_aliases().to_vec();
    updated_member_aliases.push(member_edit.repo_alias().to_owned());
    target_workspace.set_repo_aliases(updated_member_aliases);
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    regenerator.regenerate_workspace_file(member_edit.workspace_name())?;

    terminal_style::print_success(&format!(
        "Added {} to workspace {}",
        member_edit.repo_alias().style(terminal_style::ALIAS_STYLE),
        member_edit.workspace_name().style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}

/// Drops a repo alias from the workspace's member list and
/// regenerates the `.code-workspace` file from the post-mutation
/// state.
///
/// Refuses to remove a repo that is not currently a member so the
/// caller gets a clear error instead of a silent no-op, which
/// would mask a typo in the alias. Refuses to leave the workspace
/// empty because an empty workspace has no meaning and the
/// validation on the `WorkspaceRegistrationBuilder` rejects that
/// shape — the two rules must agree.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] if the workspace
/// name does not exist, or [`ReportalError::ValidationFailure`] if
/// the repo is not a current member or if removal would leave the
/// workspace with zero members. Config I/O and file regeneration
/// errors bubble up from the underlying calls.
pub fn run_workspace_remove_repo(
    member_edit: &WorkspaceArgsMemberEditParts,
) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_from_disk()?;
    let target_workspace = loaded_config.get_workspace_mut(member_edit.workspace_name())?;
    if !target_workspace.contains_repo(member_edit.repo_alias()) {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "repo '{}' is not a member of workspace '{}'",
                member_edit.repo_alias(),
                member_edit.workspace_name(),
            ),
        });
    }
    let remaining_member_aliases: Vec<String> = target_workspace
        .repo_aliases()
        .iter()
        .filter(|existing_alias| existing_alias.as_str() != member_edit.repo_alias())
        .cloned()
        .collect();
    if remaining_member_aliases.is_empty() {
        return Err(ReportalError::ValidationFailure {
            field: "workspace member".to_owned(),
            reason: format!(
                "removing '{}' would leave workspace '{}' with zero members; delete the workspace instead",
                member_edit.repo_alias(),
                member_edit.workspace_name(),
            ),
        });
    }
    target_workspace.set_repo_aliases(remaining_member_aliases);
    loaded_config.save_to_disk()?;

    let regenerator = WorkspaceRegenerator::for_config(&loaded_config);
    regenerator.regenerate_workspace_file(member_edit.workspace_name())?;

    terminal_style::print_success(&format!(
        "Removed {} from workspace {}",
        member_edit.repo_alias().style(terminal_style::ALIAS_STYLE),
        member_edit.workspace_name().style(terminal_style::ALIAS_STYLE),
    ));
    Ok(())
}
