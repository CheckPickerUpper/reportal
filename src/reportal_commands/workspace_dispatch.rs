//! Dispatch layer for `rep workspace` subcommands.
//!
//! Routes a parsed `WorkspaceArgs` to the matching handler in one
//! place so main.rs does not grow a secondary match on workspace
//! variants, and so adding a new workspace subcommand requires
//! exactly one new arm in exactly one file.

use crate::cli_args::{WorkspaceArgs, WorkspaceArgsSubcommand};
use crate::error::ReportalError;
use crate::reportal_commands::{
    run_workspace_add_repo, run_workspace_create, run_workspace_delete, run_workspace_jump,
    run_workspace_list, run_workspace_open, run_workspace_rebuild, run_workspace_remove_repo,
    run_workspace_show,
};

/// Dispatches a parsed `rep workspace` invocation to the matching
/// subcommand handler.
///
/// Taking ownership of `WorkspaceArgs` is required because every
/// handler pulls owned strings out via `into_parts` /
/// `into_workspace_name`, which consume the parsed args. Returning
/// the handler's `Result` directly lets main.rs treat workspace
/// errors the same as every other subcommand's errors.
///
/// # Errors
///
/// Returns whatever error the selected handler produces. See the
/// individual `run_workspace_*` functions for the per-action error
/// set.
pub fn dispatch_workspace_subcommand(
    parsed_workspace_args: WorkspaceArgs,
) -> Result<(), ReportalError> {
    match parsed_workspace_args.into_action() {
        WorkspaceArgsSubcommand::List => run_workspace_list(),
        WorkspaceArgsSubcommand::Show(name_only) => {
            run_workspace_show(&name_only.into_workspace_name())
        }
        WorkspaceArgsSubcommand::Create(create_args) => {
            run_workspace_create(&create_args.into_parts())
        }
        WorkspaceArgsSubcommand::Delete(delete_args) => {
            run_workspace_delete(&delete_args.into_parts())
        }
        WorkspaceArgsSubcommand::AddRepo(member_edit) => {
            run_workspace_add_repo(&member_edit.into_parts())
        }
        WorkspaceArgsSubcommand::RemoveRepo(member_edit) => {
            run_workspace_remove_repo(&member_edit.into_parts())
        }
        WorkspaceArgsSubcommand::Open(optional_name) => {
            run_workspace_open(&optional_name.into_optional_workspace_name())
        }
        WorkspaceArgsSubcommand::Jump(optional_name) => {
            run_workspace_jump(&optional_name.into_optional_workspace_name())
        }
        WorkspaceArgsSubcommand::Rebuild(name_only) => {
            run_workspace_rebuild(&name_only.into_workspace_name())
        }
    }
}
