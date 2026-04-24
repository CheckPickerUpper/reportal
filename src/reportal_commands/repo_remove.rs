//! Removes a registered repo from the `RePortal` config by alias.

use crate::error::ReportalError;
use crate::reportal_config::ReportalConfig;
use crate::terminal_style;

/// Unregisters a repo by its primary alias key. Does not delete
/// any files on disk. Refuses the operation with
/// [`ReportalError::RepoIsWorkspaceMember`] when the repo is still
/// listed as a member of one or more workspaces, because silently
/// stripping the repo from every containing workspace would destroy
/// user-declared membership and silently cascade into either an
/// empty-workspace invariant violation or a hidden workspace
/// delete. Forcing the user to either `rep workspace remove-repo`
/// or `rep workspace delete` first keeps destructive changes
/// explicit.
///
/// # Errors
///
/// Returns [`ReportalError::RepoNotFound`] if the alias is not a
/// registered primary key,
/// [`ReportalError::RepoIsWorkspaceMember`] if the repo is still a
/// member of any workspace, or the config I/O errors that the load
/// and save paths surface.
pub fn run_remove(repository_alias: &str) -> Result<(), ReportalError> {
    let mut loaded_config = ReportalConfig::load_or_initialize()?;

    let containing_workspaces = loaded_config.workspaces_containing_repo(repository_alias);
    if !containing_workspaces.is_empty() {
        let joined_workspace_names = containing_workspaces
            .iter()
            .map(|(workspace_name, _entry)| workspace_name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        return Err(ReportalError::RepoIsWorkspaceMember {
            alias: repository_alias.to_owned(),
            affected_workspaces: joined_workspace_names,
        });
    }

    let removed_entry = loaded_config.remove_repo(repository_alias)?;
    loaded_config.save_to_disk()?;
    terminal_style::print_success(&format!("Removed '{}' ({})", repository_alias, removed_entry.raw_path()));
    Ok(())
}
