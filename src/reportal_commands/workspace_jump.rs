//! Prints a workspace's `.code-workspace` file parent directory
//! for shell `cd` integration.

use crate::error::ReportalError;
use crate::reportal_commands::direct_alias_router::DirectAliasRouter;
use crate::reportal_commands::workspace_selection;
use crate::reportal_config::ReportalConfig;

/// Resolves the named workspace (by canonical key or alias) and
/// prints the `.code-workspace` file's parent directory to stdout
/// so the `rjw` shell wrapper can cd there.
///
/// When the alias string is empty, presents a workspace fuzzy
/// finder so `rjw` with no arguments is a useful entry point
/// instead of an error. This is the workspace-only counterpart to
/// `rep jump`; it never falls through to the repo registry.
/// Separating the two entry points keeps `rjw` from accidentally
/// cd'ing into a repo when the user meant to target a workspace
/// by the same short name.
///
/// # Errors
///
/// Returns [`ReportalError::WorkspaceNotFound`] when a non-empty
/// name or alias matches no registered workspace,
/// [`ReportalError::NoWorkspacesConfigured`] when the fuzzy
/// finder would have no items,
/// [`ReportalError::SelectionCancelled`] if the user escapes the
/// prompt, or [`ReportalError::ConfigIoFailure`] if the default
/// workspace file location needs the home directory and it cannot
/// be resolved.
pub fn run_workspace_jump(alias_or_canonical: &str) -> Result<(), ReportalError> {
    let loaded_config = ReportalConfig::load_or_initialize()?;
    let canonical_workspace_name = if alias_or_canonical.is_empty() {
        workspace_selection::select_workspace(&loaded_config)?
    } else {
        loaded_config.resolve_workspace_canonical_name(alias_or_canonical)?
    };
    let router = DirectAliasRouter::for_config(&loaded_config);
    router.jump_to_workspace_parent(&canonical_workspace_name)
}
